//! # Fabrique: Factories
//!
//! ## Introduction
//!
//! ## Factory Relationships
//!
//! ### Belongs To Relationships
//!
//! Now that we have explored how to build [has many][Factory::has]
//! relationships using factories, let's explore the inverse of the
//! relationship. The [for][Factory::for] method may be used to define the
//! parent model that factory created models belong to. For example, we can
//! create an `Order` model instance that belong to a factory generated user:
//!
//!```rust,no_run
//! # use fabrique::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//!
//! # #[derive(Factory, Persistable)]
//! # pub struct User {
//! #     id: Uuid,
//! # }
//! #
//! #[derive(Factory, Persistable)]
//! pub struct Anvil {
//!     id: Uuid,
//!
//!     #[fabrique(relation="User")]
//!     user_id: Uuid,
//! }
//!
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! Anvil::factory()
//!     .for_user(User::factory())
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! If you already have a parent model instance that should be associated with
//! the models you are creating, you may pass the model instance to the
//! [for][Factory::for] method:
//!
//!```rust,no_run
//! # use fabrique::{Factory, Persistable};
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//!
//! # #[derive(Factory, Persistable)]
//! # pub struct User {
//! #     id: Uuid,
//! # }
//!
//! #[derive(Factory, Persistable)]
//! pub struct Anvil {
//!     id: Uuid,
//!
//!     #[fabrique(relation="User")]
//!     user_id: Uuid,
//! }
//!
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! let user = User::factory().create(&connection).await?;
//! Anvil::factory()
//!     .for_user(user)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::factory::*;
pub use fabrique_derive::Factory;
