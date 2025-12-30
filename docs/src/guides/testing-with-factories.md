# Testing with Factories

Factories make it easy to set up test data without writing verbose setup code.
This guide shows common patterns for using factories in tests.

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
# pub struct Product {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
#     price_cents: i32,
# }
#
# async fn test_heavy_products(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Only set the attributes you care about
let product = Product::factory()
    .price_cents(150)
    .create(&pool)
    .await?;

assert!(product.price_cents > 100);
# Ok(())
# }
# fn main() {}
```

## Testing with Relations

When testing models with relationships, factories handle the foreign keys
automatically:

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
#     #[fabrique(belongs_to = "Customer")]
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
#     #[fabrique(belongs_to = "Customer")]
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
# pub struct Product {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     price_cents: i32,
# }
#
# async fn test_bulk_creation(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
for i in 0..5 {
    Product::factory()
        .price_cents(i * 10)
        .create(&pool)
        .await?;
}
# Ok(())
# }
# fn main() {}
```

## Generating Realistic Data

By default, factories generate random values for each field. For more realistic
test data, use the `faker` attribute with expressions from the
[fake](https://crates.io/crates/fake) crate:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use fabrique::fake::faker::name::en::Name;
# use fabrique::fake::faker::internet::en::SafeEmail;
# use fabrique::fake::faker::phone_number::en::PhoneNumber;
# use uuid::Uuid;
#
#[derive(Factory, Model)]
pub struct Customer {
    #[fabrique(primary_key)]
    id: Uuid,

    #[fabrique(faker = "Name()")]
    name: String,

    #[fabrique(faker = "SafeEmail()")]
    email: String,

    #[fabrique(faker = "PhoneNumber()")]
    phone: String,
}
# fn main() {}
```

Common faker expressions:

| Expression | Example Output |
|------------|----------------|
| `Name()` | "John Smith" |
| `SafeEmail()` | "john.smith@example.com" |
| `PhoneNumber()` | "+1 555-123-4567" |
| `CompanyName()` | "Acme Industries" |
| `CityName()` | "Springfield" |
| `(1..100)` | Random integer 1-99 |
| `(100..10000)` | Random integer for cents |

Import fakers from `fabrique::fake::faker`:

```rust,no_run
# extern crate fabrique;
use fabrique::fake::faker::company::en::CompanyName;
use fabrique::fake::faker::address::en::CityName;
use fabrique::fake::faker::lorem::en::Sentence;
# fn main() {}
```

Each factory call generates fresh random values, so you get unique data across
tests without hardcoding strings.
