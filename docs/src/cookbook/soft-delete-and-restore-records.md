# Using Soft Deletes

Soft deletes allow you to mark records as deleted without actually removing them
from the database. This is useful for auditing, recovery, or when you need to
preserve referential integrity.

## Enabling Soft Deletes

To enable soft deletes, add a field annotated with `#[fabrique(soft_delete)]`.
The field type must be an optional datetime:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# extern crate chrono;
use fabrique::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Factory, Model)]
pub struct User {
    id: Uuid,

    #[fabrique(soft_delete)]
    deleted_at: Option<DateTime<Utc>>,
}
# fn main() {}
```

## Deleting Records

When you call `delete` on a model with soft deletes enabled, the `deleted_at`
column is set to the current timestamp instead of removing the record:

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
# #[derive(Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Create a user into the database
let user = User::factory().create(&pool).await?;

user.delete(&pool).await?;
# Ok(())
# }
```

Soft-deleted records are automatically excluded from query results.

## Checking if a Record is Deleted

Use the `trashed` method to check if a model has been soft deleted:

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
# #[derive(Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# let id = user.id;
# user.delete(&pool).await?;
// Retrieve a soft-deleted user by its id
let user = User::find(&pool, id).await?;
assert!(user.trashed(&pool).await?);
# Ok(())
# }
```

## Restoring Deleted Records

To restore a soft-deleted record, use the `restore` method:

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
# #[derive(Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# let id = user.id;
# user.delete(&pool).await?;
// Retrieve a soft-deleted user by its id
let user = User::find(&pool, id).await?;
user.restore(&pool).await?;

// deleted_at is now null, record appears in queries again
let user = User::find(&pool, id).await?;
assert!(!user.trashed(&pool).await?);
# Ok(())
# }
```

## Permanently Deleting Records

To permanently remove a soft-deleted record from the database, use `hard_delete`:

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
# #[derive(Factory, Model)]
# pub struct User {
#     id: Uuid,
#     name: String,
#     email: String,
#     #[fabrique(soft_delete)]
#     deleted_at: Option<DateTime<Utc>>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Create a user into the database
let user = User::factory().create(&pool).await?;

// Permanently remove the record
user.hard_delete(&pool).await?;
# Ok(())
# }
```
