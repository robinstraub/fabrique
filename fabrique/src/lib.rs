//! Fluent ORM for Rust with type-safe queries and integrated testing tools.
//!
//! Fabrique provides an ergonomic interface for database interactions, combining
//! expressive query building with model-driven persistence and factory-based test data generation.
//!
//! # A tour of Fabrique
//!
//! Fabrique centers around three concepts: models derive [`Persistable`] to interact with the database,
//! use the [`QueryBuilder`] for type-safe queries, and leverage [`Factory`] to generate test data.
//!
//! ## Persistable
//!
//! Derive `Persistable` on your models to enable database operations. The macro generates methods
//! for creating records, querying all instances, and building queries with column constants.
//!
//! ## QueryBuilder
//!
//! Build queries using generated column constants that enforce type safety. The query builder
//! prevents passing values of the wrong type to WHERE clauses at compile time.
//!
//! ## Factory
//!
//! Generate test data with the factory pattern. Derive `Factory` to create builder instances
//! with optional fields, making test setup concise and readable.
//!

pub use fabrique_core::QueryBuilder;
pub use fabrique_core::{ColumnMarker, Operator, Persistable};
pub use fabrique_derive::Factory;
pub use fabrique_derive::Persistable;
