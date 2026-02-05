# Type Conversions

Fabrique models map Rust types to database columns. While primitive types like
`String`, `i32`, and `Uuid` map directly, you often need custom types — enums,
newtypes, or domain-specific wrappers — that don't have a direct database
representation.

## The Problem

Consider an order status. In Rust, you want type safety:

```rust
pub enum OrderStatus {
    Pending,
    Shipped,
    Delivered,
}
# fn main() {}
```

But databases don't have Rust enums. You need to store this as a `String` or
`i32` column. Type conversions bridge this gap.

## The `as` Attribute

The `#[fabrique(as = "DatabaseType")]` attribute tells Fabrique how to store a field:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug)]
# pub enum OrderStatus { Pending, Shipped, Delivered }
# impl From<OrderStatus> for String {
#     fn from(s: OrderStatus) -> String {
#         match s {
#             OrderStatus::Pending => "pending",
#             OrderStatus::Shipped => "shipped",
#             OrderStatus::Delivered => "delivered",
#         }.to_string()
#     }
# }
# impl TryFrom<String> for OrderStatus {
#     type Error = String;
#     fn try_from(s: String) -> Result<Self, Self::Error> {
#         match s.as_str() {
#             "pending" => Ok(OrderStatus::Pending),
#             "shipped" => Ok(OrderStatus::Shipped),
#             "delivered" => Ok(OrderStatus::Delivered),
#             _ => Err(format!("unknown status: {}", s)),
#         }
#     }
# }
#[derive(Model)]
pub struct Order {
    pub id: Uuid,

    #[fabrique(as = "String")]
    pub status: OrderStatus,  // Stored as TEXT in database
}
# fn main() {}
```

Fabrique will:

- Convert `OrderStatus` → `String` when writing to the database
- Convert `String` → `OrderStatus` when reading from the database

## Trait Requirements

For conversions to work, your type must implement two traits:

| Direction | Trait                             | Purpose                      |
| --------- | --------------------------------- | ---------------------------- |
| Reading   | `TryFrom<DatabaseType>`           | Database → Rust (may fail)   |
| Writing   | `From<YourType> for DatabaseType` | Rust → Database (infallible) |

Reading uses `TryFrom` because database values might be invalid (e.g., an unknown
status string). Writing uses `From` because your Rust type should always produce
a valid database value.

Note: Implementing `From<T> for U` automatically provides `TryInto<U> for T`,
which Fabrique uses internally.

## Common Use Cases

**Enums as strings** — Store enum variants as human-readable strings. Good for
debugging and when values might be queried directly in SQL.

**Enums as integers** — Store enum variants as integers for compact storage.
Useful when storage size matters or when integrating with systems that expect
numeric codes.

**Newtypes** — Wrap primitive types for type safety (e.g., `CustomerId(Uuid)` vs
`ProductId(Uuid)`). Prevents accidentally passing the wrong ID type to a
function.

## Query Behavior

When a field uses `as`, query parameters use the database type:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
# }
# #[derive(Factory, Model)]
# pub struct Order {
#     id: Uuid,
#     status: String,
#     #[fabrique(belongs_to = "User")]
#     user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# Order::factory().status("pending".to_string()).for_user(user.clone()).create(&pool).await?;
# Order::factory().status("shipped".to_string()).for_user(user).create(&pool).await?;
// Use String (database type), not OrderStatus
let pending_orders = Order::query()
    .select_as::<Order, _>()
    .r#where(Order::STATUS, "=", "pending".to_string())
    .get(&pool)
    .await?;

// Only the pending order is returned
assert_eq!(pending_orders.len(), 1);
assert_eq!(pending_orders[0].status, "pending");
# Ok(())
# }
```

The generated column constant (`Order::STATUS`) is typed as the database type.

## Error Handling

When conversion fails during a read, Fabrique returns `Error::Conversion` with
detailed context: which field failed, what value was encountered, and the error
message from your `TryFrom` implementation. See
[Error Handling](error-handling.md) for details.

## Summary

- Use `#[fabrique(as = "Type")]` to map custom types to database columns
- Implement `TryFrom<DatabaseType>` for reading (fallible)
- Implement `From<YourType> for DatabaseType` for writing (infallible)
- Query parameters use the database type
