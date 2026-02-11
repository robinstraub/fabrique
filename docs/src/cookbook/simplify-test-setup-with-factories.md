# Simplify Complex Test Setup with Factories

Testing a feature that spans users, orders, order lines, and
products requires creating several related records. Without
factories, each test starts with a wall of manual inserts.
Factories let you express the same setup in a few chained
calls.

For the full factory API, see
[Factories](../concepts/factories.md). For writing tests with
`#[fabrique::test]`, see
[Testing with Fabrique](../tutorials/testing-with-fabrique.md).

## Seed a Full Order in One Chain

An order needs a user, products, and the join records linking
them. With factories, the entire graph is built in one chain
— missing parents are auto-created:

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
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
# #[derive(Clone, Factory, Model)]
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
let order = Order::factory()
    .has_order_lines(OrderLine::factory(), 3)
    .create(&pool)
    .await?;
# Ok(())
# }
```

This single chain creates 1 user, 1 order, 3 order lines,
and 3 products — 8 records total. The factory resolves the
foreign keys automatically.

## Share a Parent Across Children

Three orders for the same customer. Pass a factory instance
to each `for_user` — the first `.create()` persists it, the
next ones reuse the same record:

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
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Factory, Model)]
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
    .name("Wile E. Coyote".to_string())
    .create(&pool)
    .await?;

let pending = Order::factory()
    .for_user(&user)
    .status("pending".to_string())
    .create(&pool)
    .await?;

let shipped = Order::factory()
    .for_user(&user)
    .status("shipped".to_string())
    .create(&pool)
    .await?;

assert_eq!(pending.user_id, shipped.user_id);
# Ok(())
# }
```

`for_user` accepts both a model instance and a factory. When
you pass an already-created instance, no extra INSERT happens.

## Override Only What Matters

Testing a status filter? Only set `status` — let the factory
generate everything else:

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
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Arrange — only the status matters for this test
Order::factory()
    .status("shipped".to_string())
    .create(&pool)
    .await?;

Order::factory()
    .status("pending".to_string())
    .create(&pool)
    .await?;

// Act
let pending: Vec<Order> = Order::query()
    .r#where(Order::STATUS, "=", "pending")
    .get(&pool)
    .await?;

// Assert
assert_eq!(pending.len(), 1);
assert_eq!(pending[0].status, "pending");
# Ok(())
# }
```

The test reads at a glance: two orders with different statuses,
query for one. No noise from IDs, names, or emails.

## Handle Ambiguous Relations

When a model has multiple foreign keys to the same parent
(e.g. `Message` with `sender_id` and `recipient_id`), use
aliases to disambiguate. Fabrique then generates a dedicated
`for_<alias>()` method for each relationship.

Here, Alice and Bob each send 100 messages to each other —
200 messages total, seeded in a loop:

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
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Message {
#     pub id: Uuid,
#     pub content: String,
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     pub sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     pub recipient_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let alice = User::factory()
    .name("Alice".to_string())
    .create(&pool)
    .await?;
let bob = User::factory()
    .name("Bob".to_string())
    .create(&pool)
    .await?;

// Alice sends 100 messages to Bob
for _ in 0..100 {
    Message::factory()
        .for_sender(&alice)
        .for_recipient(&bob)
        .create(&pool)
        .await?;
}

// Bob sends 100 messages to Alice
for _ in 0..100 {
    Message::factory()
        .for_sender(&bob)
        .for_recipient(&alice)
        .create(&pool)
        .await?;
}
# Ok(())
# }
```

For more details on aliases, see
[Handle Ambiguous Relations](handle-ambiguous-relations.md).
