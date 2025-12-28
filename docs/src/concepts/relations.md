# Relations

Fabrique supports two types of relationships between models:

- **Belongs to**: A model references another model via a foreign key (e.g., an order belongs to a user)
- **Has many**: A model has multiple related records in another table (e.g., a user has many orders)

These relationships enable lazy loading of related records and automatic foreign key handling in [factories](factories.md).

## Belongs To

A belongs-to relationship indicates that a model holds a foreign key to another model. When you mark a field with `belongs_to`, Fabrique learns which model is being referenced. Combined with the referenced model's primary key (defined via the [Model](models.md) derive), Fabrique can establish the link between the two tables.

```rust,no_run
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

Here, `Order` holds the foreign key (`customer_id`) that references `User`'s primary key (`id`). Fabrique generates a `BelongsTo<User>` trait for `Order`, which exposes the foreign key column for use in queries and [factories](factories.md).

## Has Many

A has-many relationship is declared on the parent side to indicate it has multiple related records. The `HasMany<T>` field is not stored in the database; it's a marker that tells Fabrique to generate a lazy loading method.

```rust,no_run
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

This generates an `orders()` method on `User` that returns a [query builder](queries.md) filtering orders by the user's primary key. Fabrique resolves the foreign key column by looking at the `BelongsTo<User>` trait on `Order`.

## Many-to-Many Relationships

For many-to-many relationships, use the `through` attribute to specify a join model:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct Anvil { id: Uuid }
# #[derive(Model)]
# pub struct OrderLine { id: Uuid, order_id: Uuid, anvil_id: Uuid }
#[derive(Model)]
pub struct Order {
    id: Uuid,

    #[fabrique(through = "OrderLine")]
    anvils: HasMany<Anvil>,
}
# fn main() {}
```

## Multiple Relationships to the Same Model

When a model has multiple foreign keys pointing to the same parent type, Fabrique requires explicit disambiguation using the `foreign_key` attribute.

Consider a `Message` model with two references to `User`:

```rust,ignore
#[derive(Model)]
pub struct Message {
    id: Uuid,

    #[fabrique(belongs_to = "User")]
    sender_id: Uuid,

    #[fabrique(belongs_to = "User")]
    recipient_id: Uuid,
}
```

Since both fields reference `User`, Fabrique cannot determine which foreign key to use for a `HasMany<Message>` relationship. On the parent model, you must specify which foreign key each has-many relationship uses:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Model)]
# pub struct Message { id: Uuid, sender_id: Uuid, recipient_id: Uuid }
#[derive(Model)]
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

This pattern is common for models like:

| Model | Foreign Keys | Parent |
|-------|--------------|--------|
| `Message` | `sender_id`, `recipient_id` | `User` |
| `Transfer` | `from_account_id`, `to_account_id` | `Account` |
| `Flight` | `departure_airport_id`, `arrival_airport_id` | `Airport` |

For a step-by-step guide, see [Handling Multiple belongs_to Relationships](../guides/multiple-belongs-to-same-model.md).
