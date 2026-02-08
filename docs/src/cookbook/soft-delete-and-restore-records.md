# Keep Order History Intact with Soft Deletes

A product is discontinued, but existing orders reference it.
A hard DELETE would either violate foreign key constraints or
cascade-delete order history. Soft deletes solve this: the
record stays in the database with a timestamp marking when it
was archived.

For the full list of model methods, see
[Models > Convenience Methods](../concepts/models.md#convenience-methods).

## Enable Soft Deletes on a Model

Add a nullable datetime field annotated with
`#[fabrique(soft_delete)]`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# extern crate chrono;
use fabrique::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Factory, Model)]
pub struct Product {
    id: Uuid,
    name: String,
    price_cents: i32,
    in_stock: bool,

    #[fabrique(soft_delete)]
    deleted_at: Option<DateTime<Utc>>,
}
# fn main() {}
```

This single annotation changes the behavior of `.delete()` and
`.destroy()` — they now set `deleted_at` instead of issuing a
`DELETE` statement.

## Archive a Product

Call `.delete()` on the instance. The row — and all its order
line references — stays intact:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# extern crate chrono;
# use fabrique::prelude::*;
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
#     #[fabrique(soft_delete)]
#     pub deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let anvil = Product::factory()
    .name("Anvil 3000".to_string())
    .create(&pool)
    .await?;
let id = anvil.id;

// Archive it — sets deleted_at to now()
anvil.delete(&pool).await?;

// Convenience methods like all() skip soft-deleted records
let active = Product::all(&pool).await?;
assert!(active.iter().all(|p| p.id != id));

// find() still returns it — useful for admin views
let archived = Product::find(&pool, id).await?;
assert!(archived.trashed(&pool).await?);
# Ok(())
# }
```

> **Note:** The query builder does not add hidden WHERE
> clauses — `Product::query().get()` returns all records,
> including soft-deleted ones. This is by design: implicit
> filtering would conflict with explicit `where_not_null` /
> `where_null` clauses and add runtime cost. Convenience
> methods like `all()` handle the filtering for you; for
> custom queries, add
> `.where_null(Product::DELETED_AT)` explicitly.

## Check and Restore

The supplier restocks the Anvil 3000. Use `.restore()` to
clear `deleted_at` and bring the product back:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# extern crate chrono;
# use fabrique::prelude::*;
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
#     #[fabrique(soft_delete)]
#     pub deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().create(&pool).await?;
# let id = product.id;
# product.delete(&pool).await?;
let archived = Product::find(&pool, id).await?;
archived.restore(&pool).await?;

// Back in active results
let product = Product::find(&pool, id).await?;
assert!(!product.trashed(&pool).await?);
# Ok(())
# }
```

## Archive by ID Without Fetching

In a REST endpoint you typically have the ID but not the full
model. `Product::destroy()` soft-deletes by primary key
without a prior SELECT:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# extern crate chrono;
# use fabrique::prelude::*;
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
#     #[fabrique(soft_delete)]
#     pub deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().create(&pool).await?;
# let id = product.id;
// DELETE /products/:id handler
Product::destroy(&pool, id).await?;
# let product = Product::find(&pool, id).await?;
# assert!(product.trashed(&pool).await?);
# Ok(())
# }
```

## Permanently Remove When Needed

For GDPR compliance or data purges, `hard_delete()` issues a
real `DELETE` — bypassing the soft delete mechanism:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# extern crate chrono;
# use fabrique::prelude::*;
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
#     #[fabrique(soft_delete)]
#     pub deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().create(&pool).await?;
# let id = product.id;
// Permanently remove the record
product.hard_delete(&pool).await?;

// Or by ID, without fetching first:
// Product::hard_destroy(&pool, id).await?;
# Ok(())
# }
```
