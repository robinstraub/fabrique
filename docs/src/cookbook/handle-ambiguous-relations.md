# Handling Multiple belongs_to Relationships

When a model references the same parent model multiple times, you need to
disambiguate the relationships using aliases. This guide shows how to set this
up correctly.

## Define the Child Model with Aliases

Use the `alias` attribute to give each relationship a unique name:

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

    #[fabrique(belongs_to = "User", alias = "Sender")]
    sender_id: Uuid,

    #[fabrique(belongs_to = "User", alias = "Recipient")]
    recipient_id: Uuid,
}
# fn main() {}
```

This generates:

- `struct Sender` and `struct Recipient` as alias marker types
- `impl BelongsTo<User, Sender> for Message`
- `impl BelongsTo<User, Recipient> for Message`
- `impl Joinable<User, Sender> for Message` (and reverse)
- `impl Joinable<User, Recipient> for Message` (and reverse)

No declaration is needed on the parent model — Fabrique infers
everything from the child's `belongs_to` declarations.

## Named Joins

With aliases, you can join the same model multiple times using `join_as`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String
# }
#
# #[derive(Factory, Model)]
# pub struct Message {
#     id: Uuid,
#     content: String,
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     recipient_id: Uuid,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let messages = Message::query()
    .join_as::<User, Sender>()
    .join_as::<User, Recipient>()
    .get(&pool)
    .await?;
# Ok(())
# }
```

This generates:

```sql
SELECT messages.*
FROM messages
JOIN users AS sender ON sender.id = messages.sender_id
JOIN users AS recipient ON recipient.id = messages.recipient_id
```

### Filtering on Named Joins

Use `where_on` to filter on a specific alias:

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
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     recipient_id: Uuid,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Find messages sent by Alice
let messages = Message::query()
    .join_as::<User, Sender>()
    .where_on::<Sender, _, _, _, _>(User::NAME, "=", "Alice".to_string())
    .get(&pool)
    .await?;
# Ok(())
# }
```

### Ordering by Named Joins

Use `order_by_on` to sort by a column from a specific alias:

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
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     recipient_id: Uuid,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Order messages by sender name
let messages = Message::query()
    .join_as::<User, Sender>()
    .order_by_on::<Sender, _, _, _>(User::NAME, "ASC")
    .get(&pool)
    .await?;
# Ok(())
# }
```

## Using Factories

Aliases generate `for_<alias>` methods on the factory:

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
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
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

let message = Message::factory()
    .content("Hello Bob!".to_string())
    .for_sender(&alice)
    .for_recipient(bob)
    .create(&pool)
    .await?;
# Ok(())
# }
```

## Loading Related Records

With aliases, the loading method is generic — pass the alias type
to specify which foreign key to follow:

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
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     recipient_id: Uuid,
# }
# #[derive(Clone, Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
// Get messages sent by this user
let sent = user.messages::<_, Sender>().get(&pool).await?;

// Get messages received by this user
let received = user.messages::<_, Recipient>().get(&pool).await?;
# Ok(())
# }
```
