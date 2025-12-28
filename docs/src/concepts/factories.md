# Factories

Factories provide a convenient way to generate model instances for testing and database seeding. Instead of manually specifying every attribute, factories generate sensible defaults and let you override only what matters for your specific test case.

## The Builder Pattern

Each model with `#[derive(Factory)]` gets a builder struct that mirrors its fields. Call `Model::factory()` to get a builder, set any fields you care about, then call `create()` to persist:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let product = Product::factory()
    .name("Anvil 3000".to_string())  // Override name
    .create(&pool)                    // id and price_cents use defaults
    .await?;
# Ok(())
# }
# fn main() {}
```

Fields you don't set are filled with generated values automatically.

## Random Value Generation

By default, factories generate random values for all fields using the [fake](https://crates.io/crates/fake) crate. This means each factory instance gets unique data without additional configuration.

For more realistic data, use the `faker` attribute to specify a custom faker expression:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use fabrique::fake::faker::name::en::Name;
# use fabrique::fake::faker::internet::en::SafeEmail;
# use uuid::Uuid;
#[derive(Model, Factory)]
pub struct User {
    id: Uuid,

    #[fabrique(faker = "Name()")]
    name: String,                       // "John Smith"

    #[fabrique(faker = "SafeEmail()")]
    email: String,                      // "john.smith@example.com"

    #[fabrique(faker = "(18..65)")]
    age: i32,                           // Random integer between 18 and 65
}
# fn main() {}
```

The faker expression is evaluated each time, ensuring unique values across instances.

To disable random generation and use `Default::default()` instead, disable the `fake` feature:

```toml
[dependencies]
fabrique = { version = "0.1", default-features = false }
```

## Relation Support

Factories understand model relationships and provide methods to create related records:

- **`for_<relation>()`** - Set a belongs-to relationship (accepts a model instance or another factory)
- **`has_<relation>(factory, count)`** - Create child records for has-many relationships

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Customer {
#     id: Uuid,
#     name: String,
#     orders: HasMany<Order>,
# }
#
# #[derive(Model, Factory)]
# pub struct Order {
#     id: Uuid,
#     #[fabrique(belongs_to = "Customer")]
#     customer_id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create a customer with 3 orders in one call
let customer = Customer::factory()
    .has_orders(Order::factory(), 3)
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

The factory creates the parent first, then creates children with the correct foreign key values.

## See Also

- [Testing with Factories](../guides/testing-with-factories.md) - Common testing patterns
- [Relations](relations.md) - How to define model relationships
