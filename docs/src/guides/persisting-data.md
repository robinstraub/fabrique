# Persisting Data

This guide covers inserting, updating, and deleting records in the database.

## Inserting Records

To insert a new record, instantiate a model and call the `save` method:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvil = Anvil {
    id: Uuid::new_v4(),
    name: "Heavy Duty".to_string(),
};

anvil.save(&pool).await?;
# Ok(())
# }
```

The `save` method performs an UPSERT: it inserts if the record is new, or updates if a record with the same primary key already exists.

Alternatively, use `create` to insert a new record. This method fails if a record with the same primary key already exists:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.create(&pool).await?;
# Ok(())
# }
```

## Updating Records

To update a model, retrieve it, modify its attributes, and call `save`:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     name: String,
# }
#
# async fn example(pool: Pool<Postgres>, mut anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.name = "Super Heavy Duty".to_string();
anvil.save(&pool).await?;
# Ok(())
# }
```

## Mass Updates

Update multiple records matching a query using the `update` builder:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     weight: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
Anvil::update()
    .set(Anvil::WEIGHT, 100)
    .r#where(Anvil::WEIGHT, "<", 50)
    .execute(&pool)
    .await?;
# Ok(())
# }
```

## Deleting Records

To delete a model, call the `delete` method:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.delete(&pool).await?;
# Ok(())
# }
```

If you know the primary key, delete without retrieving the model first using `destroy`:

```rust,no_run
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>, id: Uuid) -> Result<(), fabrique::Error> {
Anvil::destroy(&pool, id).await?;
# Ok(())
# }
```

> **Note:** If you need to keep deleted records for auditing or recovery, see [Using Soft Deletes](using-soft-deletes.md).
