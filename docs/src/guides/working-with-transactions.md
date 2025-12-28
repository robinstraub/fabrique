# Working with Transactions

Fabrique works seamlessly with SQLx transactions. All database operations accept any type implementing `sqlx::Executor`, which includes both connection pools and transactions.

## Basic Transaction Usage

Use `pool.begin()` to start a transaction, then pass it to Fabrique methods:

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
let mut tx = pool.begin().await?;

// All operations use the transaction
let product = Product {
    id: Uuid::nil(), // In production, use Uuid::new_v4()
    name: "Anvil".to_string(),
    price_cents: 4999,
    in_stock: true,
};
product.create(&mut *tx).await?;

// Commit the transaction
tx.commit().await?;
# Ok(())
# }
# fn main() {}
```

Note the `&mut *tx` syntax — this dereferences the transaction to satisfy the `Executor` trait bounds.

## Automatic Rollback

If a transaction is dropped without calling `commit()`, it automatically rolls back:

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
# #[derive(Clone, Debug, Model)]
# pub struct Order { pub id: Uuid, pub user_id: Uuid, pub status: String }
#
# async fn example(pool: &Pool<Postgres>) -> Result<(), fabrique::Error> {
let mut tx = pool.begin().await?;

let product = Product {
    id: Uuid::nil(), // In production, use Uuid::new_v4()
    name: "Rocket".to_string(),
    price_cents: 9999,
    in_stock: true,
};
product.create(&mut *tx).await?;

// Simulating an error before commit — no order exists with nil UUID
let _order: Order = Order::query()
    .select()
    .r#where(Order::ID, "=", Uuid::nil())
    .first_or_fail(&mut *tx)
    .await?; // Returns error

tx.commit().await?; // Never reached — transaction rolls back on drop
# Ok(())
# }
# fn main() {}
```

## Queries in Transactions

All query methods work with transactions:

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
let mut tx = pool.begin().await?;

// Find by primary key
let product: Product = Product::query()
    .select()
    .r#where(Product::ID, "=", Uuid::nil())
    .first_or_fail(&mut *tx)
    .await?;

// Query builder with filter
let products = Product::query()
    .select()
    .r#where(Product::IN_STOCK, "=", true)
    .get(&mut *tx)
    .await?;

// Update builder
Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 10000)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
# Ok(())
# }
# fn main() {}
```

## The Executor Pattern

Fabrique methods are generic over any `sqlx::Executor`. This means you can write functions that work with both pools and transactions:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres, PgExecutor};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
pub async fn create_product<'e, E>(
    executor: E,
    name: String,
    price_cents: i32,
) -> Result<Product, fabrique::Error>
where
    E: PgExecutor<'e> + 'e,
{
    let product = Product {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        name,
        price_cents,
        in_stock: true,
    };
    product.create(executor).await
}

# async fn example(pool: &Pool<Postgres>) -> Result<(), fabrique::Error> {
// Works with a pool
let p1 = create_product(&*pool, "Anvil".to_string(), 4999).await?;

// Works with a transaction
let mut tx = pool.begin().await?;
let p2 = create_product(&mut *tx, "Rocket".to_string(), 9999).await?;
tx.commit().await?;
# Ok(())
# }
# fn main() {}
```

## Summary

- Start transactions with `pool.begin().await?`
- Pass `&mut *tx` to Fabrique methods
- Call `tx.commit().await?` to persist changes
- Transactions auto-rollback on drop if not committed
- All Fabrique methods accept any `sqlx::Executor`
