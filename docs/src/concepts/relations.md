# Relations

At its core, SQL has a single relationship mechanism: the
**foreign key**. Fabrique models this directly — a `belongs_to`
attribute on a foreign key column connects two models, and the
compiler handles the rest (bidirectional joins, type validation).

On top of this foundation, a few annotations provide the ergonomics
you'd expect from a traditional ORM — inverse declarations,
many-to-many through tables — without multiplying relationship
types.

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

### The Inverse

The parent side can declare a `HasMany<T>` field to get a
convenience method for loading related records. This field is not
stored in the database — it's a marker:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct Order {
#     id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     customer_id: Uuid,
# }
#[derive(Model)]
pub struct User {
    id: Uuid,
    name: String,

    orders: HasMany<Order>,
}
# fn main() {}
```

This generates an `orders()` method on `User` that returns a
[query builder](query-builder.md) filtering orders by the user's
primary key. Fabrique resolves the foreign key column by looking
at the `BelongsTo<User>` trait on `Order`.

## Through Tables

When two models are related through an intermediate table, use the
`through` attribute on `HasMany`. The join model must have
`belongs_to` relationships to both sides:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Anvil {
    id: Uuid,
    name: String,
}

/// The join model must belong to both sides
#[derive(Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    order_id: Uuid,

    #[fabrique(primary_key, belongs_to = "Anvil")]
    anvil_id: Uuid,

    quantity: i32,
}

#[derive(Model)]
pub struct Order {
    id: Uuid,

    #[fabrique(through = "OrderLine")]
    anvils: HasMany<Anvil>,
}
# fn main() {}
```

This generates an `anvils()` method on `Order` that joins through
`OrderLine` to fetch related `Anvil` records.

## Aliases

When a model has multiple foreign keys to the same parent, there
is an ambiguity: Fabrique cannot determine which foreign key to
use for joins or `HasMany` resolution. The `alias` attribute
disambiguates each reference:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct User { id: Uuid, name: String }
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

On the parent side, `HasMany` references the alias to resolve
which foreign key to use:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct Message {
#     id: Uuid,
#     content: String,
#     #[fabrique(belongs_to = "User", alias = "Sender")]
#     sender_id: Uuid,
#     #[fabrique(belongs_to = "User", alias = "Recipient")]
#     recipient_id: Uuid,
# }
#[derive(Model)]
pub struct User {
    id: Uuid,
    name: String,

    #[fabrique(alias = "Sender")]
    sent_messages: HasMany<Message>,

    #[fabrique(alias = "Recipient")]
    received_messages: HasMany<Message>,
}
# fn main() {}
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
# pub struct User { id: Uuid, name: String }
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
