# Advanced Querying

This guide covers advanced query features: bulk updates, upserts with
`ON CONFLICT`, and the `RETURNING` clause.

## Bulk Updates

Update multiple records matching a condition with the update builder:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(15000).in_stock(true).create(&pool).await?;
// Mark all expensive products as out of stock
Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .execute(&pool)
    .await?;
# let products = Product::query()
#     .select()
#     .r#where(Product::PRICE_CENTS, ">", 10000)
#     .get(&pool)
#     .await?;
# assert!(products.iter().all(|p| !p.in_stock));
# Ok(())
# }
```

Chain multiple `.set()` calls to update several columns:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().name("Anvil".to_string()).price_cents(100).in_stock(false).create(&pool).await?;
Product::update()
    .set(Product::PRICE_CENTS, 9999)
    .set(Product::IN_STOCK, true)
    .r#where(Product::NAME, "=", "Anvil".to_string())
    .execute(&pool)
    .await?;
# let product = Product::query().select().r#where(Product::NAME, "=", "Anvil".to_string()).first_or_fail(&pool).await?;
# assert_eq!(product.price_cents, 9999);
# assert!(product.in_stock);
# Ok(())
# }
```

## RETURNING Clause

Get back the updated rows with `.returning()`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(15000).in_stock(true).create(&pool).await?;
# Product::factory().price_cents(20000).in_stock(true).create(&pool).await?;
let updated: Vec<Product> = Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .returning()
    .get(&pool)
    .await?;

println!("Updated {} products", updated.len());
# assert_eq!(updated.len(), 2);
# assert!(updated.iter().all(|p| !p.in_stock));
# Ok(())
# }
```

## ON CONFLICT (Upsert)

Handle conflicts on insert with `.on_conflict()`. This is useful for upsert operations.

### Do Nothing

Silently ignore conflicts:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let id = Uuid::new_v4();
// Insert if not exists, do nothing if exists
Product::query()
    .insert()
    .set(Product::ID, id)
    .set(Product::NAME, "Anvil".to_string())
    .set(Product::PRICE_CENTS, 4999)
    .set(Product::IN_STOCK, true)
    .on_conflict()
    .do_nothing()
    .execute(&pool)
    .await?;
# let found = Product::find(&pool, id).await?;
# assert_eq!(found.name, "Anvil");
# Ok(())
# }
```

### Do Update

Update the existing row on conflict:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().name("Old Name".to_string()).create(&pool).await?;
# let id = product.id;
// Insert or update if exists
let saved: Product = Product::query()
    .insert()
    .set(Product::ID, id)
    .set(Product::NAME, "New Name".to_string())
    .set(Product::PRICE_CENTS, 5000)
    .set(Product::IN_STOCK, true)
    .on_conflict()
    .do_update()
    .returning()
    .first_or_fail(&pool)
    .await?;
# // Product was updated with new values
# assert_eq!(saved.name, "New Name");
# assert_eq!(saved.price_cents, 5000);
# Ok(())
# }
```

The `.do_update()` method updates all non-primary-key columns with `col = EXCLUDED.col`.

## Summary

| Operation        | Method                                           |
| ---------------- | ------------------------------------------------ |
| Bulk update      | `Model::update().set().r#where().execute()`      |
| Get updated rows | `.returning().get()`                             |
| Insert or ignore | `.insert().set(...).on_conflict().do_nothing()`  |
| Insert or update | `.insert().set(...).on_conflict().do_update()`   |
