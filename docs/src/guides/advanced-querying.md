# Advanced Querying

This guide covers advanced query features: bulk updates, upserts with `ON CONFLICT`, and the `RETURNING` clause.

## Bulk Updates

Update multiple records matching a condition with the update builder:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
# async fn example(pool: &Pool<Postgres>) -> Result<(), fabrique::Error> {
// Mark all expensive products as out of stock
Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .execute(pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

Chain multiple `.set()` calls to update several columns:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
# async fn example(pool: &Pool<Postgres>) -> Result<(), fabrique::Error> {
Product::update()
    .set(Product::PRICE_CENTS, 9999)
    .set(Product::IN_STOCK, true)
    .r#where(Product::NAME, "=", "Anvil".to_string())
    .execute(pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## RETURNING Clause

Get back the updated rows with `.returning()`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
# async fn example(pool: &Pool<Postgres>) -> Result<(), fabrique::Error> {
let updated: Vec<Product> = Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .returning(Product::columns())
    .get(pool)
    .await?;

println!("Updated {} products", updated.len());
# Ok(())
# }
# fn main() {}
```

## ON CONFLICT (Upsert)

Handle conflicts on insert with `.on_conflict()`. This is useful for upsert operations.

### Do Nothing

Silently ignore conflicts:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
# async fn example(pool: &Pool<Postgres>, product: Product) -> Result<(), fabrique::Error> {
// Insert if not exists, do nothing if exists
Product::query()
    .insert()
    .set(Product::ID, product.id)
    .set(Product::NAME, product.name)
    .set(Product::PRICE_CENTS, product.price_cents)
    .set(Product::IN_STOCK, product.in_stock)
    .on_conflict()
    .do_nothing()
    .execute(pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

### Do Update

Update the existing row on conflict:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
# async fn example(pool: &Pool<Postgres>, product: Product) -> Result<(), fabrique::Error> {
// Insert or update if exists
let saved: Product = Product::query()
    .insert()
    .set(Product::ID, product.id)
    .set(Product::NAME, product.name)
    .set(Product::PRICE_CENTS, product.price_cents)
    .set(Product::IN_STOCK, product.in_stock)
    .on_conflict()
    .do_update()
    .returning()
    .first_or_fail(pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

The `.do_update()` method updates all non-primary-key columns with `col = EXCLUDED.col`.

## Summary

| Operation | Method |
|-----------|--------|
| Bulk update | `Model::update().set().r#where().execute()` |
| Get updated rows | `.returning(Model::columns()).get()` |
| Insert or ignore | `Model::query().insert().set(...).on_conflict().do_nothing()` |
| Insert or update | `Model::query().insert().set(...).on_conflict().do_update()` |
