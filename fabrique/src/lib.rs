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
pub use fabrique_core::QueryBuilder;
pub use fabrique_core::{ColumnMarker, Operator, Persistable};
pub use fabrique_derive::Factory;
pub use fabrique_derive::Persistable;
