# Relations

At its core, SQL has a single relationship mechanism: the
**foreign key**. Fabrique models this directly — a `belongs_to`
attribute on a foreign key column connects two models, and the
compiler handles the rest (bidirectional joins, type validation).

On top of this foundation, aliases provide the ergonomics
you'd expect from a traditional ORM — without multiplying
relationship types.

## Declaring a Relationship

A relationship starts with a foreign key. Annotate the field with
`belongs_to` to tell Fabrique which model it references:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct User { id: Uuid }
#[derive(Model)]
pub struct Order {
    id: Uuid,

    #[fabrique(belongs_to = "User")]
    customer_id: Uuid,
}
# fn main() {}
```

Here, `Order` holds the foreign key (`customer_id`) that references
`User`'s primary key (`id`). From this single declaration, Fabrique
generates:

- `BelongsTo<User>` for `Order` — exposes the foreign key column
  for queries and factories
- `Joinable<User>` for `Order` — enables
  `Order::query().join::<User>()`
- `Joinable<Order>` for `User` — enables
  `User::query().join::<Order>()`

Joins work in both directions regardless of which model holds the
foreign key.

### Loading Related Records

From this single `belongs_to` declaration, Fabrique also generates
an `orders()` method on `User` that returns a
[query builder](query-builder.md) filtering orders by the user's
primary key — no declaration needed on the parent side:

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
# pub struct Order {
#     id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     user_id: Uuid,
#     status: String,
# }
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
let orders = user.orders().get(&pool).await?;
# Ok(())
# }
```

## Aliases

When a model has multiple foreign keys to the same parent, there
is an ambiguity: Fabrique cannot determine which foreign key to
use for joins. The `alias` attribute disambiguates each
reference:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct User { id: Uuid, name: String, email: String }
#[derive(Model)]
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

Each `alias` generates a marker type (unit struct) implementing
the `Alias` trait. From the example above, Fabrique generates:

- `struct Sender` and `struct Recipient`
- `BelongsTo<User, Sender>` and `BelongsTo<User, Recipient>`
  for `Message`
- Bidirectional `Joinable` impls for each alias

Loading methods become generic over the alias, so the parent can
specify which foreign key to use:

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
# let user = User::factory().create(&pool).await?;
let sent = user.messages::<Sender>().get(&pool).await?;
let received = user.messages::<Recipient>().get(&pool).await?;
# Ok(())
# }
```

In queries, `join_as` joins the same model multiple times under
different aliases. `where_on` and `order_by_on` qualify columns
through a specific alias:

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
# async fn main(
#     pool: Pool<Backend>,
# ) -> Result<(), fabrique::Error> {
let messages = Message::query()
    .join_as::<User, Sender>()
    .join_as::<User, Recipient>()
    .where_on::<Sender, _, _, _, _>(
        User::NAME, "=", "Alice".to_string(),
    )
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
JOIN users AS recipient
  ON recipient.id = messages.recipient_id
WHERE sender.name = $1
```

See [Handling Multiple belongs_to Relationships](
../cookbook/handle-ambiguous-relations.md) for a complete walkthrough
covering factories, lazy loading, and ordering.

---

Next: [Query Builder](query-builder.md) — building and executing
queries.
