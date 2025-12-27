//! # Factories
//!
//! Factories generate test data by creating model instances with sensible
//! defaults. They provide a fluent builder API for overriding specific fields
//! while keeping the rest at their default values.
//!
//! ## Creating Models
//!
//! Use `Model::factory()` to get a factory, configure it with setter methods,
//! then call `create()` to persist the instance:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//! #
//! #[derive(Factory, Model)]
//! pub struct Anvil {
//!     id: Uuid,
//!     material: String,
//!     weight: i32,
//! }
//!
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! // Create with defaults
//! let anvil = Anvil::factory().create(&connection).await?;
//!
//! // Override specific fields
//! let heavy_anvil = Anvil::factory()
//!     .material("Iron".to_owned())
//!     .weight(500)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Belongs To Relationships
//!
//! Use `for_<relation>()` methods to link a factory to a parent model.
//! The method accepts either a factory (creates a new parent) or an existing
//! model instance (uses its primary key):
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
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
//!     #[fabrique(belongs_to = "User")]
//!     user_id: Uuid,
//! }
//!
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! // Creates a new User, then creates an Anvil linked to it
//! Anvil::factory()
//!     .for_user(User::factory())
//!     .create(&connection)
//!     .await?;
//!
//! // Or use an existing user
//! let user = User::factory().create(&connection).await?;
//! Anvil::factory()
//!     .for_user(user)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Has Many Relationships
//!
//! Use `has_<relation>()` methods to create child models after the parent.
//! Specify a child factory and the number of instances to create:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//!
//! #[derive(Factory, Model)]
//! pub struct Customer {
//!     id: Uuid,
//!     #[fabrique(foreign_key = Order::CUSTOMER_ID)]
//!     orders: HasMany<Order>,
//! }
//!
//! #[derive(Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     customer_id: Uuid,
//! }
//!
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! // Creates a Customer, then creates 3 Orders linked to it
//! Customer::factory()
//!     .has_orders(Order::factory(), 3)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::factory::*;
pub use fabrique_derive::Factory;
