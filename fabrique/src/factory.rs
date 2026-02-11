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
//! # use sqlx::Pool;
//! # use uuid::Uuid;
//! #
//! #[derive(Factory, Model)]
//! pub struct Product {
//!     id: Uuid,
//!     name: String,
//!     price_cents: i32,
//! }
//!
//! # async fn example(connection: Pool<Backend>) -> Result<(), fabrique::Error> {
//! // Create with defaults
//! let product = Product::factory().create(&connection).await?;
//!
//! // Override specific fields
//! let expensive_product = Product::factory()
//!     .name("Anvil 3000".to_owned())
//!     .price_cents(9999)
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
//! # use sqlx::Pool;
//! # use uuid::Uuid;
//!
//! # #[derive(Factory, Model)]
//! # pub struct User {
//! #     id: Uuid,
//! # }
//! #
//! #[derive(Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     #[fabrique(belongs_to = "User")]
//!     user_id: Uuid,
//! }
//!
//! # async fn example(connection: Pool<Backend>) -> Result<(), fabrique::Error> {
//! // Creates a new User, then creates an Order linked to it
//! Order::factory()
//!     .for_user(User::factory())
//!     .create(&connection)
//!     .await?;
//!
//! // Or use an existing user
//! let user = User::factory().create(&connection).await?;
//! Order::factory()
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
//! These methods are automatically generated from the child's `belongs_to`
//! declaration — no `HasMany<T>` field is needed on the parent.
//! Specify a child factory and the number of instances to create:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::Pool;
//! # use uuid::Uuid;
//!
//! #[derive(Factory, Model)]
//! pub struct Customer {
//!     id: Uuid,
//! }
//!
//! #[derive(Default, Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//!     #[fabrique(belongs_to = "Customer")]
//!     customer_id: Uuid,
//! }
//!
//! # async fn example(connection: Pool<Backend>) -> Result<(), fabrique::Error> {
//! // Creates a Customer, then creates 3 Orders linked to it
//! Customer::factory()
//!     .has_orders(Order::factory(), 3)
//!     .create(&connection)
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
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::Pool;
//! # use uuid::Uuid;
//!
//! #[derive(Factory, Model)]
//! pub struct Order {
//!     id: Uuid,
//! }
//!
//! #[derive(Default, Factory, Model)]
//! pub struct Product {
//!     id: Uuid,
//! }
//!
//! #[derive(Default, Factory, Model)]
//! #[fabrique(table = "order_lines")]
//! pub struct OrderLine {
//!     #[fabrique(primary_key, belongs_to = "Order")]
//!     order_id: Uuid,
//!     #[fabrique(primary_key, belongs_to = "Product")]
//!     product_id: Uuid,
//! }
//!
//! # async fn example(connection: Pool<Backend>) -> Result<(), fabrique::Error> {
//! // Creates an Order, then 2 OrderLines (each auto-creating a Product)
//! Order::factory()
//!     .has_order_lines(OrderLine::factory(), 2)
//!     .create(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::factory::*;
pub use fabrique_derive::Factory;
