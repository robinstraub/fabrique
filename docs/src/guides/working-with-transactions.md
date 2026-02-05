# Working with Transactions

Fabrique works seamlessly with SQLx transactions. All database operations accept
both connection pools and transactions.

## Basic Transaction Usage

Use `pool.begin()` to start a transaction, then pass it to Fabrique methods:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let mut tx = pool.begin().await?;

// All operations use the transaction
let product = Product {
    id: Uuid::new_v4(),
    name: "Anvil".to_string(),
    price_cents: 4999,
    in_stock: true,
};
product.create(&mut tx).await?;

// Commit the transaction
tx.commit().await?;
# Ok(())
# }
```

## Automatic Rollback

If a transaction is dropped without calling `commit()`, it automatically rolls back:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let mut tx = pool.begin().await?;

let product = Product {
    id: Uuid::new_v4(),
    name: "Rocket".to_string(),
    price_cents: 9999,
    in_stock: true,
};
product.create(&mut tx).await?;

// Simulating an error before commit
let result = Product::query()
    .select()
    .r#where(Product::ID, "=", Uuid::nil())
    .first_or_fail(&mut *tx)
    .await;

// Query fails — transaction rolls back when dropped without commit
assert!(result.is_err());
# Ok(())
# }
```

## Queries in Transactions

All query methods work with transactions:

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
# Product::factory().id(id).in_stock(true).price_cents(5000).create(&pool).await?;
let mut tx = pool.begin().await?;

// Query builder with filter
let products = Product::query()
    .select()
    .r#where(Product::IN_STOCK, "=", true)
    .get(&mut *tx)
    .await?;

// Update builder
Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 1000)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
# let updated = Product::find(&pool, id).await?;
# assert!(!updated.in_stock);
# Ok(())
# }
```

## Writing Generic Functions

You can write functions that accept both pools and transactions using SQLx's
`Acquire` trait:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn create_product<'a, A>(
    db: A,
    name: String,
    price_cents: i32,
) -> Result<Product, fabrique::Error>
where
    A: sqlx::Acquire<'a, Database = Backend> + Send + 'a,
{
    let product = Product {
        id: Uuid::new_v4(),
        name,
        price_cents,
        in_stock: true,
    };
    product.create(db).await
}
# fn main() {}
```

## Summary

- Start transactions with `pool.begin().await?`
- Pass `&mut tx` for persistence methods (`create`, `save`)
- Pass `&mut *tx` for query methods (`get`, `first`, `execute`)
- Call `tx.commit().await?` to persist changes
- Transactions auto-rollback on drop if not committed
