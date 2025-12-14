//! # Fabrique: Factories
//!
//! ## Introduction
//!
//! ## Creating Models Using Factories
//!
//! ### Instantiating Models
//!
//! #### Overriding Attributes
//!
//! If you would like to override some of the default values of your models, you
//! may use the generated setter methods on the factory. Only the specified
//! attributes will be replaced while the rest of the attributes remain set to
//! their default values as specified by the factory:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//! #
//! #[derive(Factory, Model)]
//! pub struct Anvil {
//!     id: Uuid,
//!     material: String,
//!     weight: i32,
//! }
//!
//! # async fn example(connection: &PgPool, anvil: Anvil) -> Result<(), sqlx::Error> {
//! Anvil::factory()
//!     .material("Iron".to_owned())
//!     .weight(42)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//!
//! ## Factory Relationships
//!
//! ### Belongs To Relationships
//!
//! Now that we have explored how to build [has many][Factory::has]
//! relationships using factories, let's explore the inverse of the
//! relationship. The `for_[relation]` methods may be used to define the
//! parent model that factory created models belong to. For example, we can
//! create an `Anvil` model instance that belongs to a factory generated user:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//!
//! # #[derive(Factory, Model)]
//! # pub struct User {
//! #     id: Uuid,
//! # }
//! #
//! #[derive(Factory, Model)]
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
//! `for_[relation]` method:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::PgPool;
//! # use uuid::Uuid;
//!
//! # #[derive(Factory, Model)]
//! # pub struct User {
//! #     id: Uuid,
//! # }
//!
//! #[derive(Factory, Model)]
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
