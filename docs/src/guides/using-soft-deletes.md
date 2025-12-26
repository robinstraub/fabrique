# Using Soft Deletes

Soft deletes allow you to mark records as deleted without actually removing them from the database. This is useful for auditing, recovery, or when you need to preserve referential integrity.

## Enabling Soft Deletes

To enable soft deletes, add a field annotated with `#[fabrique(soft_delete)]`. The field type must be an optional datetime:

```rust,ignore
# use fabrique::prelude::*;
# use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Factory, Model)]
pub struct Anvil {
    id: Uuid,

    #[fabrique(soft_delete)]
    deleted_at: Option<DateTime<Utc>>,
}
```

## Deleting Records

When you call `delete` on a model with soft deletes enabled, the `deleted_at` column is set to the current timestamp instead of removing the record:

```rust,ignore
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.delete(&pool).await?;
// Record still exists with deleted_at set
# Ok(())
# }
```

Soft-deleted records are automatically excluded from query results.

## Checking if a Record is Deleted

Use the `trashed` method to check if a model has been soft deleted:

```rust,ignore
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
if anvil.trashed(&pool).await? {
    println!("This anvil has been deleted");
}
# Ok(())
# }
```

## Restoring Deleted Records

To restore a soft-deleted record, use the `restore` method:

```rust,ignore
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.restore(&pool).await?;
// deleted_at is now null, record appears in queries again
# Ok(())
# }
```

## Permanently Deleting Records

To permanently remove a soft-deleted record from the database, use `hard_delete`:

```rust,ignore
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
# use chrono::{DateTime, Utc};
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# async fn example(pool: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
anvil.hard_delete(&pool).await?;
// Record is permanently removed
# Ok(())
# }
```
