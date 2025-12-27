//! # Fabrique
//!
//! Fluent ORM for Rust with type-safe queries and integrated testing tools.
//!
//! Fabrique is an object-relational mapper (ORM) that makes it enjoyable to
//! interact with your database. It provides a fluent, type-safe API for
//! defining models, querying data, and generating test fixtures.
//!
//! ## Core Features
//!
//! ### Models
//!
//! Define database models using the `#[derive(Model)]` macro. Models
//! automatically map to database tables and provide methods for CRUD
//! operations, soft deletes, and more. See the [`model`] module for detailed
//! documentation.
//!
//! ### Query Builder
//!
//! Build type-safe database queries with a fluent API. The query builder
//! provides methods for filtering, ordering, limiting results, and more. See
//! the [`query_builder`] module for details.
//!
//! ### Factories
//!
//! Generate test data easily with the `#[derive(Factory)]` macro. Factories
//! help you create model instances for testing without manually specifying
//! every field. See the [`factory`] module for more information.
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use fabrique::prelude::*;
//! use sqlx::PgPool;
//!
//! // Define a model
//! #[derive(Model, Factory)]
//! pub struct Anvil {
//!     id: uuid::Uuid,
//!     weight: i32,
//!     material: String,
//! }
//!
//! # async fn example(db: &PgPool) -> Result<(), fabrique::Error> {
//! // Query the database
//! let heavy_anvils: Vec<Anvil> = Anvil::query()
//!     .select()
//!     .r#where(Anvil::WEIGHT, ">=", 100)
//!     .get(db)
//!     .await?;
//!
//! // Create a new record
//! let anvil = Anvil {
//!     id: uuid::Uuid::new_v4(),
//!     weight: 150,
//!     material: "iron".to_string(),
//! };
//! anvil.create(db).await?;
//!
//! // Generate test data with factories
//! let test_anvil = Anvil::factory().create(db).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Getting Started
//!
//! To use Fabrique in your project, add it to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! fabrique = "0.1"
//! sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
//! ```
//!
//! Then define your models and start querying. See the [`model`] module for a
//! comprehensive guide on model conventions and usage
pub use database::*;
pub use error::*;
pub use factory::*;
pub use model::*;
pub use relation::*;

pub mod database;
pub mod error;
pub mod factory;
pub mod model;
pub mod prelude;
pub mod relation;
pub mod sql;
