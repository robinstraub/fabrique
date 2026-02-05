# Persisting Data

This guide covers inserting, updating, and deleting records in the database.

## Inserting Records

To insert a new record, instantiate a model and call the `save` method:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let product = Product {
    id: Uuid::new_v4(),
    name: "Anvil 3000".to_string(),
    price_cents: 4999,
    in_stock: true,
};

let saved = product.save(&pool).await?;

// The product is persisted with the specified values
assert_eq!(saved.name, "Anvil 3000");
# Ok(())
# }
```

The `save` method performs an UPSERT: it inserts if the record is new, or
updates if a record with the same primary key already exists.

> **Tip:** For more control over upsert behavior (e.g., specifying conflict
> columns or choosing which fields to update), see
> [Advanced Querying](advanced-querying.md#on-conflict-upsert).

Alternatively, use `create` to insert a new record. This method fails if a
record with the same primary key already exists:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let product = Product {
    id: Uuid::new_v4(),
    name: "Rocket".to_string(),
    price_cents: 9999,
    in_stock: true,
};

let created = product.create(&pool).await?;

// The product is created with the specified values
assert_eq!(created.name, "Rocket");
# Ok(())
# }
```

## Updating Records

To update a model, retrieve it, modify its attributes, and call `save`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let mut product = Product::factory().name("Anvil 3000".to_string()).create(&pool).await?;
product.name = "Super Anvil 3000".to_string();
let updated = product.save(&pool).await?;

// The product name is updated
assert_eq!(updated.name, "Super Anvil 3000");
# Ok(())
# }
```

## Mass Updates

Update multiple records matching a query using the `update` builder:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# // Create products with low prices
# Product::factory().price_cents(30).create(&pool).await?;
# Product::factory().price_cents(40).create(&pool).await?;
Product::update()
    .set(Product::PRICE_CENTS, 100)
    .r#where(Product::PRICE_CENTS, "<", 50)
    .execute(&pool)
    .await?;

// Both products now have price 100
let products = Product::all(&pool).await?;
assert!(products.iter().all(|p| p.price_cents == 100));
# Ok(())
# }
```

## Deleting Records

To delete a model, call the `delete` method:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().create(&pool).await?;
# let id = product.id;
product.delete(&pool).await?;

// The product no longer exists
let result = Product::find(&pool, id).await;
assert!(matches!(result, Err(fabrique::Error::NotFound)));
# Ok(())
# }
```

If you know the primary key, delete without retrieving the model first using `destroy`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, PartialEq, Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
#     price_cents: i32,
#     in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let product = Product::factory().create(&pool).await?;
# let id = product.id;
Product::destroy(&pool, id).await?;

// The product no longer exists
let result = Product::find(&pool, id).await;
assert!(matches!(result, Err(fabrique::Error::NotFound)));
# Ok(())
# }
```

> **Note:** If you need to keep deleted records for auditing or recovery, see
> [Using Soft Deletes](using-soft-deletes.md).
