# Persisting Data

This guide covers inserting, updating, and deleting records in the database.

## Inserting Records

To insert a new record, instantiate a model and call the `save` method:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let product = Product {
    id: Uuid::nil(),
    name: "Anvil 3000".to_string(),
};

product.save(&pool).await?;
# Ok(())
# }
# fn main() {}
```

The `save` method performs an UPSERT: it inserts if the record is new, or updates if a record with the same primary key already exists.

> **Tip:** For more control over upsert behavior (e.g., specifying conflict columns or choosing which fields to update), see [Advanced Querying](advanced-querying.md#on-conflict-upsert).

Alternatively, use `create` to insert a new record. This method fails if a record with the same primary key already exists:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>, product: Product) -> Result<(), fabrique::Error> {
product.create(&pool).await?;
# Ok(())
# }
# fn main() {}
```

## Updating Records

To update a model, retrieve it, modify its attributes, and call `save`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>, mut product: Product) -> Result<(), fabrique::Error> {
product.name = "Super Anvil 3000".to_string();
product.save(&pool).await?;
# Ok(())
# }
# fn main() {}
```

## Mass Updates

Update multiple records matching a query using the `update` builder:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
#     price_cents: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
Product::update()
    .set(Product::PRICE_CENTS, 100)
    .r#where(Product::PRICE_CENTS, "<", 50)
    .execute(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Deleting Records

To delete a model, call the `delete` method:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>, product: Product) -> Result<(), fabrique::Error> {
product.delete(&pool).await?;
# Ok(())
# }
# fn main() {}
```

If you know the primary key, delete without retrieving the model first using `destroy`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>, id: Uuid) -> Result<(), fabrique::Error> {
Product::destroy(&pool, id).await?;
# Ok(())
# }
# fn main() {}
```

> **Note:** If you need to keep deleted records for auditing or recovery, see [Using Soft Deletes](using-soft-deletes.md).
