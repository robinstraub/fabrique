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
disambiguates by generating a pseudo-Model for each reference:

> **Note:** Aliases are being implemented in
> [#75](https://github.com/robinstraub/fabrique/issues/75).
> <!-- TODO: remove this note once #75 is merged -->

```rust,ignore
#[derive(Model)]
pub struct Order {
    id: Uuid,

    #[fabrique(belongs_to = User, alias = Seller)]
    seller_id: Uuid,

    #[fabrique(belongs_to = User, alias = Buyer)]
    buyer_id: Uuid,

    amount: i32,
}
```

`alias = Seller` generates a `Seller` pseudo-Model that implements
`Model` with `table_name() = "users AS seller"`. On the parent
side, `HasMany` references the alias directly:

```rust,ignore
#[derive(Model)]
pub struct User {
    id: Uuid,
    name: String,

    sold_orders: HasMany<Seller>,
    bought_orders: HasMany<Buyer>,
}
```

In queries, aliases work like any other model — join them, filter
on them, select from them. The `where_on` method qualifies a
column through an alias:

```rust,ignore
let orders = Order::query()
    .join::<Seller>()
    .join::<Buyer>()
    .where_on::<Seller>(User::NAME, "=", "Wile E.".to_string())
    .where_on::<Buyer>(User::NAME, "=", "Road Runner".to_string())
    .get(&pool)
    .await?;
```

This generates:

```sql
SELECT orders.*
FROM orders
JOIN users AS seller ON seller.id = orders.seller_id
JOIN users AS buyer ON buyer.id = orders.buyer_id
WHERE seller.name = $1 AND buyer.name = $2
```

---

Next: [Query Builder](query-builder.md) — building and executing
queries.
