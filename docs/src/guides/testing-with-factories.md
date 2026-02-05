# Testing with Factories

Factories make it easy to set up test data without writing verbose setup code.
This guide shows common patterns for using factories in tests.

## Basic Test Setup

Create models with only the attributes relevant to your test:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Only set the attributes you care about
let product = Product::factory()
    .price_cents(150)
    .create(&pool)
    .await?;

assert!(product.price_cents > 100);
# Ok(())
# }
```

## Testing with Relations

When testing models with relationships, factories handle the foreign keys automatically:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct User { id: Uuid, name: String, email: String }
#
# #[derive(Factory, Model)]
# pub struct Order {
#     id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     user_id: Uuid,
#     status: String,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Create a user with specific attributes
let user = User::factory()
    .name("Acme Corp".to_string())
    .create(&pool)
    .await?;

// Create orders for that user
let order1 = Order::factory()
    .for_user(user.clone())
    .status("pending".to_string())
    .create(&pool)
    .await?;

let order2 = Order::factory()
    .for_user(user.clone())
    .status("shipped".to_string())
    .create(&pool)
    .await?;

// Both orders belong to the same user
assert_eq!(order1.user_id, user.id);
assert_eq!(order2.user_id, user.id);
# Ok(())
# }
```

## Inline Relation Creation

For simpler tests, create related models inline:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct User { id: Uuid, name: String, email: String }
#
# #[derive(Factory, Model)]
# pub struct Order {
#     id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     user_id: Uuid,
#     status: String,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// The user is created automatically
let order = Order::factory()
    .for_user(User::factory())
    .create(&pool)
    .await?;
# // The order has a valid user_id
# assert!(!order.user_id.is_nil());
# Ok(())
# }
```

## Multiple Records

Create multiple records by calling `create` in a loop or using iterators:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
for i in 0..5 {
    Product::factory()
        .price_cents(i * 10)
        .create(&pool)
        .await?;
}
# let products = Product::all(&pool).await?;
# assert_eq!(products.len(), 5);
# Ok(())
# }
```

## Generating Realistic Data

By default, factories generate random values for each field. For more realistic
test data, use the `faker` attribute with expressions from the
[fake](https://crates.io/crates/fake) crate:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use fabrique::fake::faker::name::en::Name;
# use fabrique::fake::faker::internet::en::SafeEmail;
# use uuid::Uuid;
#
#[derive(Factory, Model)]
#[fabrique(table = "users")]
pub struct User {
    id: Uuid,

    #[fabrique(faker = "Name()")]
    name: String,

    #[fabrique(faker = "SafeEmail()")]
    email: String,
}
# fn main() {}
```

Common faker expressions:

| Expression      | Example Output                   |
| --------------- | -------------------------------- |
| `Name()`        | "John Smith"                     |
| `SafeEmail()`   | "<john.smith@example.com>"       |
| `PhoneNumber()` | "+1 555-123-4567"                |
| `CompanyName()` | "Acme Industries"                |
| `CityName()`    | "Springfield"                    |
| `(1..100)`      | Random integer 1-99              |
| `(100..10000)`  | Random integer for cents         |

Import fakers from `fabrique::fake::faker`:

```rust
# extern crate fabrique;
use fabrique::fake::faker::company::en::CompanyName;
use fabrique::fake::faker::address::en::CityName;
use fabrique::fake::faker::lorem::en::Sentence;
# fn main() {}
```

Each factory call generates fresh random values, so you get unique data across
tests without hardcoding strings.
