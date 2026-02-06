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
//!
//! // Define a model
//! #[derive(Model, Factory)]
//! pub struct Product {
//!     id: uuid::Uuid,
//!     name: String,
//!     price_cents: i32,
//! }
//!
//! # async fn example(db: &Pool<Backend>) -> Result<(), fabrique::Error> {
//! // Query the database
//! let expensive_products: Vec<Product> = Product::query()
//!     .r#where(Product::PRICE_CENTS, ">=", 1000)
//!     .get(db)
//!     .await?;
//!
//! // Create a new record
//! let product = Product {
//!     id: uuid::Uuid::new_v4(),
//!     name: "Anvil 3000".to_string(),
//!     price_cents: 9999,
//! };
//! product.create(db).await?;
//!
//! // Generate test data with factories
//! let test_product = Product::factory().create(db).await?;
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

#[cfg(feature = "fake")]
pub use fake;

/// Generates a seeded value for factory fields.
///
/// When the `fake` feature is enabled, this generates random test data using
/// the `fake` crate. When disabled, it falls back to `Default::default()`.
///
/// This function is used internally by the `#[derive(Factory)]` macro and
/// should not typically be called directly.
#[cfg(feature = "fake")]
pub fn seeded_value<T>() -> T
where
    T: fake::Dummy<fake::Faker>,
{
    use fake::Fake;
    fake::Faker.fake()
}

/// Generates a seeded value for factory fields.
///
/// When the `fake` feature is disabled, this falls back to
/// `Default::default()`.
#[cfg(not(feature = "fake"))]
pub fn seeded_value<T>() -> T
where
    T: Default,
{
    T::default()
}

pub mod database;
pub mod error;
pub mod factory;
pub mod model;
pub mod prelude;
pub mod relation;
pub mod sql;

#[cfg(feature = "sqlite")]
#[doc(hidden)]
pub use fabrique_core::__private;

pub use fabrique_derive::doctest;
pub use fabrique_derive::test;
