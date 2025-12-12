//! Fluent ORM for Rust with type-safe queries and integrated testing tools.
//!
//! Fabrique provides an ergonomic interface for database interactions, combining
//! expressive query building with model-driven persistence and factory-based test data generation.
//!
//! # Inserting and Updating Models
//!
//! ## Inserts
//!
//! To insert a new record into the database, you should instantiate a new model
//! instance and set attributes on the model. Then, call the `create` method on
//! the model instance:
//!
//! ```rust,no_run
//! # use fabrique_core::Persistable;
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! #
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! anvil.create(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! # Deleting Models
//!
//! To delete a model, you may call the `delete` method on the model instance:
//!
//! ```rust,no_run
//! # use fabrique_core::Persistable;
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! #
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! anvil.delete(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Deleting an Existing Model by its Primary Key
//!
//! In the example above, we are retrieving the model from the database before calling
//! the delete method. However, if you know the primary key of the model, you may delete
//! the model without explicitly retrieving it by calling the destroy method.
//!
//! ```rust,no_run
//! # use fabrique_core::Persistable;
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! #
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
//! Anvil::destroy(&connection, id).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Soft Deleting
//!
//! In addition to actually removing records from your database, Fabrique can also "soft
//! delete" models. When models are soft deleted, they are not actually removed from your
//! database. Instead, a soft delete attribute is set on the model indicating the date and
//! time at which the model was "deleted". To enable soft deletes for a model, annotate
//! an attribute with the `#[fabrique(soft_delete)]` annotation. The attribute type must
//! be an optional date, such as `Option<chrono::DateTime<chrono::Utc>>`.
//!
//!```rust,no_run
//! # use fabrique_core::Persistable;
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! use chrono::{DateTime, Utc};
//!
//! #[derive(Factory, Persistable)]
//! pub struct Anvil {
//!     id: uuid::Uuid,
//!
//!     #[fabrique(soft_delete)]
//!     deleted_at: Option<DateTime<Utc>>
//! }
//! ```
//!
//! Now, when you call the delete method on the model, the deleted_at column will be set
//! to the current date and time. However, the model's database record will be left in the
//! table. When querying a model that uses soft deletes, the soft deleted models will
//! automatically be excluded from all query results.
//!
//! To determine if a given model instance has been soft deleted, you may use the
//! `trashed` method:
//!
//!```rust,no_run
//! # use fabrique_core::{Persistable, SoftDelete};
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! if anvil.trashed(&connection).await? {
//!     // --snip--
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! ### Restoring Soft Deleted Models
//!
//! Sometimes you may wish to "un-delete" a soft deleted model. To restore a soft deleted
//! model, you may call the `restore` method on a model instance. The restore method will
//! set the model's deleted_at column to null:
//!
//!```rust,no_run
//! # use fabrique_core::{Persistable, SoftDelete};
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! anvil.restore(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Permanently Deleting Models
//!
//! Sometimes you may need to truly remove a model from your database. You may use the
//! `hard_delete` method to permanently remove a soft deleted model from the database
//! table:
//!
//!```rust,no_run
//! # use fabrique_core::{HardDelete, Persistable};
//! # use fabrique_derive::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Persistable)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! anvil.hard_delete(&connection).await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::QueryBuilder;
pub use fabrique_core::{ColumnMarker, HardDelete, Model, Operator, Persistable, SoftDelete};
pub use fabrique_derive::Factory;
pub use fabrique_derive::Persistable;
