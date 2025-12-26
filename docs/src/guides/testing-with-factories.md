# Testing with Factories

Factories make it easy to set up test data without writing verbose setup code. This guide shows common patterns for using factories in tests.

## Basic Test Setup

Create models with only the attributes relevant to your test:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
#     weight: i32,
# }
#
# async fn test_heavy_anvils(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Only set the attributes you care about
let anvil = Anvil::factory()
    .weight(150)
    .create(&pool)
    .await?;

assert!(anvil.weight > 100);
# Ok(())
# }
# fn main() {}
```

## Testing with Relations

When testing models with relationships, factories handle the foreign keys automatically:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Customer {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
# }
#
# #[derive(Factory, Model)]
# pub struct Order {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     #[fabrique(relation = "Customer")]
#     customer_id: Uuid,
#     total: i32,
# }
#
# async fn test_customer_orders(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create a customer with specific attributes
let customer = Customer::factory()
    .name("Acme Corp".to_string())
    .create(&pool)
    .await?;

// Create orders for that customer
let order1 = Order::factory()
    .for_customer(customer.clone())
    .total(100)
    .create(&pool)
    .await?;

let order2 = Order::factory()
    .for_customer(customer)
    .total(200)
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Inline Relation Creation

For simpler tests, create related models inline:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Customer {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
# }
#
# #[derive(Factory, Model)]
# pub struct Order {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     #[fabrique(relation = "Customer")]
#     customer_id: Uuid,
# }
#
# async fn test_order_creation(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// The customer is created automatically
let order = Order::factory()
    .for_customer(Customer::factory())
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Multiple Records

Create multiple records by calling `create` in a loop or using iterators:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     weight: i32,
# }
#
# async fn test_bulk_creation(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
for i in 0..5 {
    Anvil::factory()
        .weight(i * 10)
        .create(&pool)
        .await?;
}
# Ok(())
# }
# fn main() {}
```
