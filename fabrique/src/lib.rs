//! # Fabrique
//!
//! SQL-first, type-safe, ergonomic database toolkit for Rust.
//!
//! Fabrique provides a fluent API for defining models, querying
//! data, and generating test fixtures — built on SQL semantics
//! rather than hiding them.
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
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! // Define a model
//! #[derive(Clone, Model, Factory)]
//! pub struct Product {
//!     id: uuid::Uuid,
//!     name: String,
//!     price_cents: i32,
//! }
//!
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! // Query the database
//! let expensive_products: Vec<Product> = Product::query()
//!     .r#where(Product::PRICE_CENTS, ">=", 1000)
//!     .get(&pool)
//!     .await?;
//!
//! // Create a new record
//! let product = Product {
//!     id: uuid::Uuid::new_v4(),
//!     name: "Anvil 3000".to_string(),
//!     price_cents: 9999,
//! };
//! product.create(&pool).await?;
//!
//! // Generate test data with factories
//! let test_product = Product::factory().create(&pool).await?;
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
//! fabrique = { version = "0.2", features = ["sqlite"] }
//!
//! [dev-dependencies]
//! fabrique = { version = "0.2", features = ["testing"] }
//! ```
//!
//! Then define your models and start querying. See the [`model`] module for a
//! comprehensive guide on model conventions and usage
pub use database::*;
pub use dialect::*;
pub use error::*;
#[cfg(feature = "testing")]
pub use factory::*;
pub use model::*;
pub use relation::*;

#[cfg(feature = "testing")]
pub use fake;

/// Generates a seeded value for factory fields.
///
/// When the `testing` feature is enabled, this generates random test data
/// using the `fake` crate.
///
/// This function is used internally by the `#[derive(Factory)]` macro and
/// should not typically be called directly.
#[cfg(feature = "testing")]
pub fn seeded_value<T>() -> T
where
    T: fake::Dummy<fake::Faker>,
{
    use fake::Fake;
    fake::Faker.fake()
}

pub mod database;
pub mod dialect;
pub mod error;
#[cfg(feature = "testing")]
pub mod factory;
pub mod model;
pub mod prelude;
pub mod relation;
pub mod sql;
#[cfg(feature = "testing")]
#[doc(hidden)]
pub use fabrique_core::__private;

#[cfg(feature = "testing")]
pub use fabrique_derive::doctest;
#[cfg(feature = "testing")]
pub use fabrique_derive::test;
