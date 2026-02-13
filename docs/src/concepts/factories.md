# Factories

Factories generate model instances for testing. Instead of manually
specifying every attribute, factories provide sensible defaults and
let you override only what matters for your specific test case.

> **Note:** Factories require the `testing` feature flag.
> Add `fabrique` with `features = ["testing"]` to your
> `[dev-dependencies]` to use them.

## The Builder Pattern

Each model with `#[derive(Factory)]` gets a builder struct that
mirrors its fields. Call `Model::factory()` to get a builder, set
any fields you care about, then call `create()` to persist:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let product = Product::factory()
    .name("Anvil 3000".to_string())  // Override name
    .create(&pool)                    // id and price_cents: defaults
    .await?;

assert_eq!(product.name, "Anvil 3000");
# Ok(())
# }
```

Fields you don't set are filled with generated values automatically.

## Random Value Generation

By default, factories generate random values for all fields using
the [fake](https://crates.io/crates/fake) crate. Each factory
instance gets unique data without additional configuration.

For more realistic data, use the `faker` attribute to specify a
generator:

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

    #[fabrique(faker = "(18..65)")]
    age: i32,
}
# fn main() {}
```

The expression is evaluated each time, ensuring unique values
across instances. See the
[fake](https://docs.rs/fake) crate documentation for the full
list of available generators.

## Relation Support

Factories understand `belongs_to` relationships. When a
factory creates a record with a foreign key, it automatically
creates the parent record if none is specified:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Model, Factory)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// A User is auto-created — no manual setup needed
let order = Order::factory()
    .create(&pool)
    .await?;

assert_ne!(order.user_id, Uuid::nil());
# Ok(())
# }
```

> **Cyclic relation graphs:** If your relation graph contains
> a cycle, you might wonder whether auto-creation recurses
> infinitely. It doesn't — SQL already requires at least one
> foreign key in the cycle to be nullable, and factories skip
> optional foreign keys (`Option<Uuid>`), which breaks the
> chain.

Use `for_<relation>()` to take control — pass a **factory**
to customize the parent, or a **model instance** to reuse an
existing one:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Model, Factory)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Pass a factory: creates a new user with specific attributes
let order = Order::factory()
    .for_user(User::factory().name("Wile E.".to_string()))
    .create(&pool)
    .await?;

// Pass a model: reuses an existing user
let user = User::factory().create(&pool).await?;
let order = Order::factory()
    .for_user(user)
    .create(&pool)
    .await?;
# Ok(())
# }
```

This distinction matters when creating multiple records.
Passing a factory creates a **new** parent each time; passing
an instance shares the **same** parent:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Model, Factory)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// A conversation between two specific people
let wile = User::factory()
    .name("Wile E.".to_string())
    .create(&pool)
    .await?;
let runner = User::factory()
    .name("Road Runner".to_string())
    .create(&pool)
    .await?;

// All orders share the same user — no duplication
for _ in 0..5 {
    Order::factory()
        .for_user(&wile)
        .create(&pool)
        .await?;
}
# Ok(())
# }
```

`has_<relation>()` works in the other direction — creating
children for a parent:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Model, Factory)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let user = User::factory()
    .has_orders(Order::factory(), 3)
    .create(&pool)
    .await?;

let orders = user.orders::<_>().get(&pool).await?;
assert_eq!(orders.len(), 3);
# Ok(())
# }
```

Both combine for complex object graphs in a single call.
Missing parents are auto-created at every level of the chain:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Model, Factory)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[derive(Clone, Model, Factory)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[derive(Clone, Model, Factory)]
# #[fabrique(table = "order_lines")]
# pub struct OrderLine {
#     #[fabrique(primary_key, belongs_to = "Order")]
#     pub order_id: Uuid,
#     #[fabrique(primary_key, belongs_to = "Product")]
#     pub product_id: Uuid,
#     pub quantity: i32,
#     pub unit_price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// 1 order, 3 order lines, each with its own product
let order = Order::factory()
    .has_order_lines(OrderLine::factory(), 3)
    .create(&pool)
    .await?;
# Ok(())
# }
```

The factory creates records in dependency order: a user and
products are auto-created, then order lines with the correct
foreign keys, then the order — all from a single chain.

---

Next: [Database](database.md) — backends, connections, and error
handling.
