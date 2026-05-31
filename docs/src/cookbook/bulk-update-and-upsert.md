# Efficiently Reprice a Catalog with Bulk Updates

You have a catalog of products and need to reprice them — apply
a discount, adjust margins, or import a supplier price list.
Doing this one record at a time means N queries for N products.
The [query builder](../concepts/query-builder.md#writing-data)
lets you do it in one.

## Apply a Discount in One Query

Mark all expensive products as discounted. A single UPDATE
touches every matching row:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# Product::factory()
#     .price_cents(15000)
#     .in_stock(true)
#     .create(&pool)
#     .await?;
// Mark all products over $100 as out of stock
Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .execute(&pool)
    .await?;
# let products = Product::query()
#     .r#where(Product::PRICE_CENTS, ">", 10000)
#     .get(&pool)
#     .await?;
# assert!(products.iter().all(|p| !p.in_stock));
# Ok(())
# }
```

Chain multiple `.set()` calls to update several columns at once:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# Product::factory()
#     .name("Anvil 3000".to_string())
#     .price_cents(100)
#     .in_stock(false)
#     .create(&pool)
#     .await?;
Product::update()
    .set(Product::PRICE_CENTS, 4999)
    .set(Product::IN_STOCK, true)
    .r#where(Product::NAME, "=", "Anvil 3000")
    .execute(&pool)
    .await?;
# let product = Product::query()
#     .r#where(Product::NAME, "=", "Anvil 3000")
#     .first_or_fail(&pool)
#     .await?;
# assert_eq!(product.price_cents, 4999);
# assert!(product.in_stock);
# Ok(())
# }
```

## Get the Updated Records Back

Add `.returning()` to retrieve the affected rows without a
separate SELECT:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# Product::factory()
#     .price_cents(15000)
#     .in_stock(true)
#     .create(&pool)
#     .await?;
# Product::factory()
#     .price_cents(20000)
#     .in_stock(true)
#     .create(&pool)
#     .await?;
let updated: Vec<Product> = Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .returning()
    .get(&pool)
    .await?;

println!("Repriced {} products", updated.len());
# assert_eq!(updated.len(), 2);
# assert!(updated.iter().all(|p| !p.in_stock));
# Ok(())
# }
```

## Import a Price List (Upsert)

When importing external data, some products may already exist.
`on_conflict().do_update()` turns the INSERT into an upsert —
inserting new rows and updating existing ones in a single
statement:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# let product = Product::factory()
#     .name("Old Name".to_string())
#     .create(&pool)
#     .await?;
# let id = product.id;
// Insert or update if the product already exists
let saved: Product = Product::insert()
    .set(Product::ID, id)
    .set(Product::NAME, "Anvil 3000")
    .set(Product::PRICE_CENTS, 5000)
    .set(Product::IN_STOCK, true)
    .on_conflict()
    .do_update()
    .returning()
    .first_or_fail(&pool)
    .await?;
# assert_eq!(saved.name, "Anvil 3000");
# assert_eq!(saved.price_cents, 5000);
# Ok(())
# }
```

`.do_update()` updates all non-primary-key columns with the
values from the INSERT — the SQL equivalent of
`SET col = EXCLUDED.col` for each column.

## Bulk Upsert a Collection

When you have a `Vec` of models to upsert — e.g. syncing an
external data source — use `.upsert()` to insert or update
them all in a single statement:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# let products = vec![
#     Product::factory::<Sqlite>().name("Anvil 3000".to_string()).make(),
#     Product::factory::<Sqlite>().name("Rocket Skates".to_string()).make(),
# ];
// Insert all products, or update on ID conflict
products.upsert(&pool, Product::ID).await?;
# Ok(())
# }
```

The second argument specifies the conflict target — the column
(or columns) that identify a unique row. All other columns are
updated when a conflict is detected.

For a composite unique key, pass a tuple:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
# async fn example(
#     products: Vec<Product>,
#     pool: Pool<Sqlite>,
# ) -> Result<(), fabrique::Error> {
products
    .upsert(&pool, (Product::NAME, Product::PRICE_CENTS))
    .await?;
# Ok(())
# }
```

> **Note:** The conflict target columns must have a UNIQUE
> constraint or be the primary key in your database schema.

If you only want to skip duplicates without updating, use
`.do_nothing()`:

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
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
# let id = Uuid::new_v4();
// Insert if not exists, silently skip if exists
Product::insert()
    .set(Product::ID, id)
    .set(Product::NAME, "Anvil 3000")
    .set(Product::PRICE_CENTS, 4999)
    .set(Product::IN_STOCK, true)
    .on_conflict()
    .do_nothing()
    .execute(&pool)
    .await?;
# let found = Product::find(&pool, id).await?;
# assert_eq!(found.name, "Anvil 3000");
# Ok(())
# }
```
