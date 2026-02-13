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
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//! #
//! #[derive(Clone, Factory, Model)]
//! pub struct Product {
//!     id: Uuid,
//!     name: String,
//!     price_cents: i32,
//! }
//!
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! // Create with defaults
//! let product = Product::factory().create(&pool).await?;
//!
//! // Override specific fields
//! let expensive_product = Product::factory()
//!     .name("Anvil 3000".to_owned())
//!     .price_cents(9999)
//!     .create(&pool)
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
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//! #
//! # #[derive(Clone, Factory, Model)]
//! # pub struct User {
//! #     id: Uuid,
//! #     name: String,
//! #     email: String,
//! # }
//! #
//! #[derive(Clone, Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     #[fabrique(belongs_to = "User")]
//!     user_id: Uuid,
//! }
//!
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! // Creates a new User, then creates an Order linked to it
//! Order::factory()
//!     .for_user(User::factory())
//!     .create(&pool)
//!     .await?;
//!
//! // Or use an existing user
//! let user = User::factory().create(&pool).await?;
//! Order::factory()
//!     .for_user(user)
//!     .create(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Has Many Relationships
//!
//! Use `has_<relation>()` methods to create child models after the parent.
//! These methods are automatically generated from the child's `belongs_to`
//! declaration — no `HasMany<T>` field is needed on the parent.
//! Specify a child factory and the number of instances to create:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! #[derive(Clone, Factory, Model)]
//! pub struct User {
//!     id: Uuid,
//!     name: String,
//!     email: String,
//! }
//!
//! #[derive(Clone, Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     #[fabrique(belongs_to = "User")]
//!     user_id: Uuid,
//! }
//!
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! // Creates a User, then creates 3 Orders linked to it
//! User::factory()
//!     .has_orders(Order::factory(), 3)
//!     .create(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Many-to-Many Relationships
//!
//! For many-to-many relationships, create instances through the join model
//! using `has_<join_model>()`:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! # #[derive(Clone, Factory, Model)]
//! # pub struct User {
//! #     id: Uuid,
//! #     name: String,
//! #     email: String,
//! # }
//! #
//! #[derive(Clone, Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     #[fabrique(belongs_to = "User")]
//!     user_id: Uuid,
//! }
//!
//! #[derive(Clone, Factory, Model)]
//! pub struct Product {
//!     id: Uuid,
//!     name: String,
//!     price_cents: i32,
//! }
//!
//! #[derive(Clone, Factory, Model)]
//! #[fabrique(table = "order_lines")]
//! pub struct OrderLine {
//!     #[fabrique(primary_key, belongs_to = "Order")]
//!     order_id: Uuid,
//!     #[fabrique(primary_key, belongs_to = "Product")]
//!     product_id: Uuid,
//!     quantity: i32,
//!     unit_price_cents: i32,
//! }
//!
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! // Creates an Order, then 2 OrderLines (each auto-creating a Product)
//! Order::factory()
//!     .has_order_lines(OrderLine::factory(), 2)
//!     .create(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::factory::*;
pub use fabrique_derive::Factory;
