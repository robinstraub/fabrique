# Error Handling

Fabrique provides a database-agnostic error type that covers the most common failure
cases: missing records, type conversion failures, and other database errors.

## The Error Type

All Fabrique operations return `Result<T, fabrique::Error>`. The error type has
three variants:

```rust,ignore
pub enum Error {
    /// The requested entity was not found in the database.
    NotFound,

    /// Failed to convert a value between Rust and database types.
    Conversion {
        field: String,
        from: &'static str,
        to: &'static str,
        value: String,
        reason: String,
        direction: ConversionDirection,
    },

    /// Other database errors (connection, query syntax, constraints, etc.).
    Other(Box<dyn std::error::Error + Send + Sync>),
}
```

## NotFound

Returned when a query expects a record but none exists:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product { pub id: Uuid, pub name: String }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let result = Product::find(&pool, Uuid::nil()).await;

match &result {
    Ok(product) => println!("Found: {}", product.name),
    Err(fabrique::Error::NotFound) => println!("Product not found"),
    Err(e) => println!("Other error: {}", e),
}

// A nil UUID doesn't exist in the database
assert!(matches!(result, Err(fabrique::Error::NotFound)));
# Ok(())
# }
```

Methods that return `NotFound`:

- `find(pool, id)` — when no record matches the primary key
- `first_or_fail(pool)` — when the query returns no results

Methods that return `Option` instead of `NotFound`:

- `first(pool)` — returns `Ok(None)` if no results

## Conversion

Returned when converting between Rust and database types fails. This typically
happens with custom types using the [`as` attribute](type-conversions.md):

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug)]
# pub enum Status { Active, Inactive }
# impl TryFrom<String> for Status {
#     type Error = String;
#     fn try_from(s: String) -> Result<Self, Self::Error> {
#         match s.as_str() {
#             "active" => Ok(Self::Active),
#             "inactive" => Ok(Self::Inactive),
#             _ => Err(format!("Unknown status: {}", s)),
#         }
#     }
# }
# impl From<Status> for String {
#     fn from(s: Status) -> String { "active".to_string() }
# }
#
# #[derive(Clone, Debug, Model)]
# pub struct Account {
#     pub id: Uuid,
#     #[fabrique(as = "String")]
#     pub status: Status
# }
#
# async fn example(pool: &Pool<Backend>) -> Result<(), fabrique::Error> {
let result = Account::find(pool, Uuid::nil()).await;

match result {
    Ok(account) => println!("Found account"),
    Err(fabrique::Error::Conversion {
        field, from, to, value, reason, direction
    }) => {
        println!(
            "Conversion error on field '{}': {} -> {} failed for value '{}': {}",
            field, from, to, value, reason
        );
    }
    Err(e) => println!("Other error: {}", e),
}
# Ok(())
# }
# fn main() {}
```

The `Conversion` error includes:

| Field       | Description                                      |
| ----------- | ------------------------------------------------ |
| `field`     | The struct field name that failed                |
| `from`      | The source type name                             |
| `to`        | The target type name                             |
| `value`     | String representation of the failing value       |
| `reason`    | Error message from your `TryFrom` implementation |
| `direction` | `FromDb` (reading) or `ToDb` (writing)           |

### Conversion Direction

```rust,ignore
pub enum ConversionDirection {
    /// Converting a database value to a Rust value (during reads)
    FromDb,
    /// Converting a Rust value to a database value (during writes)
    ToDb,
}
```

## Other

Wraps all other database errors — connection failures, constraint violations,
query syntax errors, etc.:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# // Create a user first to cause a duplicate key violation
# let existing = User::factory().create(&pool).await?;
// Insert a duplicate primary key to trigger a constraint violation
let duplicate = User {
    id: existing.id,
    name: "Other".to_string(),
    email: "other@example.com".to_string(),
};
let result = duplicate.create(&pool).await;

match &result {
    Ok(_) => println!("Created user"),
    Err(fabrique::Error::Other(e)) => {
        // Access the underlying sqlx error if needed
        println!("Database error: {}", e);
    }
    Err(e) => println!("Error: {}", e),
}

// Duplicate primary key causes an Other error
assert!(matches!(result, Err(fabrique::Error::Other(_))));
# Ok(())
# }
```

## Summary

| Variant      | When it occurs                                  |
| ------------ | ----------------------------------------------- |
| `NotFound`   | Record doesn't exist (`find`, `first_or_fail`)  |
| `Conversion` | Type conversion failed (custom types with `as`) |
| `Other`      | Connection, constraint, or query errors         |
