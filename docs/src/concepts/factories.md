# Factories

Factories provide a convenient way to generate model instances for testing and database seeding. Instead of manually specifying each attribute, factories let you define defaults and override only what you need.

## Defining a Factory

To define a factory, derive the `Factory` macro alongside `Model`:

```rust,no_run
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model, Factory)]
pub struct Anvil {
    #[fabrique(primary_key)]
    id: Uuid,
    name: String,
    weight: i32,
}
```

## Creating Instances

Use the `factory()` method to get a factory builder, then call `create()` to persist to the database:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Anvil {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
#     weight: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvil = Anvil::factory()
    .name("Heavy Duty".to_string())
    .weight(100)
    .create(&pool)
    .await?;
# Ok(())
# }
```

## Relations

Factories support creating related models. When a model has a foreign key, use the `for_<relation>` method to link it:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Customer {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
# }
#
# #[derive(Model, Factory)]
# pub struct Order {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     #[fabrique(relation = "Customer")]
#     customer_id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create an order with an existing customer
let customer = Customer::factory().create(&pool).await?;
let order = Order::factory()
    .for_customer(customer)
    .create(&pool)
    .await?;

// Or create the related customer inline
let order = Order::factory()
    .for_customer(Customer::factory().name("Acme Corp".to_string()))
    .create(&pool)
    .await?;
# Ok(())
# }
```

The `for_customer` method accepts either a `Customer` instance or a `CustomerFactory`, giving you flexibility in how you set up test data.
