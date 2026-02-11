# Testing with Fabrique

The previous tutorials built service functions for an
e-commerce app. This tutorial shows how to test them — with
isolated databases, generated test data, and relations.

## Prerequisites

- Completed [Getting Started](getting-started.md) and
  [Building an E-commerce App](building-an-ecommerce-app.md)
- The models and service functions from those tutorials

## Setup

Factories and `#[fabrique::test]` require the `testing` feature.
Add it to your `[dev-dependencies]`:

```toml
[dev-dependencies]
fabrique = { version = "0.2", features = ["testing"] }
```

Cargo unifies features during `cargo test`, so both your
production and test features are available in test builds.

## Your First Database Test

Each test needs a fresh database with tables ready. The
`#[fabrique::test]` macro handles this — it wraps
[`#[sqlx::test]`](https://docs.rs/sqlx/latest/sqlx/attr.test.html)
and automatically selects the migration directory for the
active backend:

```rust,ignore
use fabrique::prelude::*;

#[fabrique::test]
async fn test_name(pool: Pool<Backend>) {
    // pool is a fresh, migrated database
    // unique to this test — no interference
}
```

The `pool` parameter is an isolated database created for this
test only. When the test ends, the database is dropped.

Let's test `list_available_products` from the Getting Started
tutorial. First, add `Factory` to the derive list if you
haven't already:

```rust,ignore
#[derive(Clone, Debug, Factory, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}
```

Now write the test using the **Arrange / Act / Assert**
pattern:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# pub async fn list_available_products(
#     pool: &Pool<Backend>,
# ) -> Result<Vec<Product>, fabrique::Error> {
#     Product::query()
#         .r#where(Product::IN_STOCK, "=", true)
#         .get(pool)
#         .await
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Arrange — two in stock, one out of stock
Product::factory()
    .in_stock(true)
    .create(&pool).await?;
Product::factory()
    .in_stock(true)
    .create(&pool).await?;
Product::factory()
    .in_stock(false)
    .create(&pool).await?;

// Act
let available = list_available_products(&pool).await?;

// Assert
assert_eq!(available.len(), 2);
# Ok(())
# }
```

Two things to notice:

- We only set `in_stock` — the field our test cares about.
  `id`, `name`, and `price_cents` are generated automatically.
- Each test gets its own database — no cleanup needed, no
  interference between tests.

## Focusing on What Matters

Compare the factory approach to manual construction:

```rust,ignore
// Manual — every field, even irrelevant ones
let product = Product {
    id: Uuid::new_v4(),
    name: "Irrelevant".to_string(),
    price_cents: 0,
    in_stock: true,
};
product.create(&pool).await.unwrap();

// Factory — only what matters
Product::factory()
    .in_stock(true)
    .create(&pool).await.unwrap();
```

With a 4-field struct the difference is modest. With 15 fields,
it becomes significant — and the test's intent is immediately
clear: this test is about stock availability, nothing else.

## Testing Relations

Let's test `get_user_pending_orders` from the E-commerce
tutorial. We need a user with orders in different statuses.

### Sharing a Parent

Create a user first, then create orders that reference it:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let user = User::factory().create(&pool).await?;

Order::factory()
    .for_user(user.clone())
    .status("pending".to_string())
    .create(&pool).await?;
Order::factory()
    .for_user(user.clone())
    .status("shipped".to_string())
    .create(&pool).await?;

// Both orders belong to the same user
let orders = user.orders().get(&pool).await?;
assert_eq!(orders.len(), 2);
# Ok(())
# }
```

Both orders share the same user because we pass the
**instance**. Passing `User::factory()` instead would create
a **new** user for each order — useful when you need distinct
parents.

### Creating from the Parent Side

`has_orders` creates children in a single call:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let user = User::factory()
    .has_orders(
        Order::factory().status("pending".to_string()),
        3,
    )
    .create(&pool).await?;

let orders = user.orders().get(&pool).await?;
assert_eq!(orders.len(), 3);
assert!(orders.iter().all(|o| o.status == "pending"));
# Ok(())
# }
```

### A Complete Test

Putting it together — test that `get_user_pending_orders`
returns only pending orders:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
#
# pub async fn get_user_pending_orders(
#     pool: &Pool<Backend>,
#     user_id: Uuid,
# ) -> Result<Vec<Order>, fabrique::Error> {
#     let user: User = User::query()
#         .r#where(User::ID, "=", user_id)
#         .first_or_fail(pool)
#         .await?;
#     user.orders()
#         .r#where(Order::STATUS, "=", "pending")
#         .get(pool)
#         .await
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Arrange — a user with mixed order statuses
let user = User::factory()
    .has_orders(
        Order::factory().status("pending".to_string()),
        2,
    )
    .has_orders(
        Order::factory().status("shipped".to_string()),
        1,
    )
    .create(&pool).await?;

// Act
let pending = get_user_pending_orders(&pool, user.id)
    .await?;

// Assert
assert_eq!(pending.len(), 2);
assert!(pending.iter().all(|o| o.status == "pending"));
# Ok(())
# }
```

The test reads like a specification: given a user with 2
pending and 1 shipped order, `get_user_pending_orders` returns
exactly the 2 pending ones.

## Generating Realistic Data

By default, factories fill fields with random values. For more
realistic data, the `faker` attribute specifies a generator
from the [fake](https://docs.rs/fake) crate:

```rust
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
    name: String,

    #[fabrique(faker = "SafeEmail()")]
    email: String,
}
# fn main() {}
```

Common expressions:

| Expression      | Example Output           |
| --------------- | ------------------------ |
| `Name()`        | "John Smith"             |
| `SafeEmail()`   | "<john@example.com>"     |
| `CompanyName()` | "Acme Industries"        |
| `CityName()`    | "Springfield"            |
| `(1..100)`      | Random integer 1–99      |
| `(100..10000)`  | Random integer for cents |

Import generators from `fabrique::fake::faker`:

```rust
# extern crate fabrique;
use fabrique::fake::faker::company::en::CompanyName;
use fabrique::fake::faker::address::en::CityName;
use fabrique::fake::faker::lorem::en::Sentence;
# fn main() {}
```

Each factory call generates fresh values — records are unique
without hardcoded strings. See the
[fake crate documentation](https://docs.rs/fake) for the full
list of generators.

## Summary

You've learned how to test a Fabrique application:

1. **`#[fabrique::test]`** gives each test an isolated,
   migrated database
2. **Factories** generate test data — set only the fields that
   matter for your assertion
3. **`for_<relation>`** and **`has_<relation>`** set up related
   records without manual foreign key wiring
4. **`faker`** produces realistic data when random values
   aren't enough

## Next Steps

- Explore the [Factories](../concepts/factories.md) concept
  for the full factory API
- Read about
  [Transactions](../concepts/database.md#connections) to test
  transactional code
- See the [Query Builder](../concepts/query-builder.md) for
  more complex test assertions
