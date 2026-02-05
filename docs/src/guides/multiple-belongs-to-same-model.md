# Handling Multiple belongs_to Relationships

When a model references the same parent model multiple times, you need to tell
Fabrique which foreign key to use for each relationship. This guide shows how to
set this up correctly.

## Define the Child Model

Start with the model that has multiple references to the same parent. Each
foreign key gets its own `belongs_to` attribute:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Factory, Model)]
# pub struct User { id: Uuid, name: String, email: String }
#[derive(Factory, Model)]
pub struct Message {
    id: Uuid,
    content: String,

    #[fabrique(belongs_to = "User")]
    sender_id: Uuid,

    #[fabrique(belongs_to = "User")]
    recipient_id: Uuid,
}
# fn main() {}
```

## Define the Parent Model with Explicit Foreign Keys

On the parent model, each `HasMany` relationship must specify which foreign key
it uses:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Factory, Model)]
# pub struct Message {
#     id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     recipient_id: Uuid,
# }
#[derive(Clone, Factory, Model)]
pub struct User {
    id: Uuid,
    name: String,

    #[fabrique(foreign_key = "sender_id")]
    sent_messages: HasMany<Message>,

    #[fabrique(foreign_key = "recipient_id")]
    received_messages: HasMany<Message>,
}
# fn main() {}
```

## Using Lazy Loading

With explicit foreign keys, lazy loading methods work as expected:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Factory, Model)]
# pub struct Message {
#     id: Uuid,
#     content: String,
#     #[fabrique(belongs_to = "User")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     recipient_id: Uuid,
# }
# #[derive(Clone, Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
#     #[fabrique(foreign_key = "sender_id")]
#     sent_messages: HasMany<Message>,
#     #[fabrique(foreign_key = "recipient_id")]
#     received_messages: HasMany<Message>,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
// Get messages sent by this user
let sent = user.sent_messages().get(&pool).await?;

// Get messages received by this user
let received = user.received_messages().get(&pool).await?;
# Ok(())
# }
```

## Using Factories

Use the field setter methods directly on the child factory:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Factory, Model)]
# pub struct User { id: Uuid, name: String, email: String }
# #[derive(Factory, Model)]
# pub struct Message {
#     id: Uuid,
#     content: String,
#     #[fabrique(belongs_to = "User")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     recipient_id: Uuid,
# }
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

// Set both foreign keys explicitly
let message = Message::factory()
    .content("Hello!".to_string())
    .sender_id(alice.id)
    .recipient_id(bob.id)
    .create(&pool)
    .await?;

// The message references both users
assert_eq!(message.sender_id, alice.id);
assert_eq!(message.recipient_id, bob.id);
# Ok(())
# }
```

## Why No for_user Method?

When a model has a single `belongs_to` to `User`, Fabrique generates a
`for_user()` method on the factory. With multiple references to `User`, this
method is not generated because it would be ambiguous.

Instead, use the direct setter methods (`sender_id()`, `recipient_id()`) as
shown above.
