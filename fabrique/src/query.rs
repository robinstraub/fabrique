//! # Database: Query Builder
//!
//! ## Introduction
//!
//! Fabrique's database query builder provides a convenient, fluent interface to
//! creating and running database queries. It can be used to perform most
//! database operations in your application and works perfectly with all of
//! Fabrique's supported database systems.
//!
//! The Fabrique query builder uses the [sqlx::QueryBuilder] parameter binding
//! to protect your application against SQL injection attacks. There is no need
//! to clean or sanitize strings passed to the query builder as query bindings.
//!
//! ## Running Database Queries
//!
//! ### Retrieving All Rows From a Table
//!
//! You may use the [table][Builder::table] method provided by the [Builder]
//! struct to begin a query. The [table][Builder::table] method returns a fluent
//! query builder instance for the given table, allowing you to chain more
//! constraints onto the query and then finally retrieve the results of the
//! query using the [get][Builder::get] method:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), sqlx::Error> {
//! let rows: Vec<(uuid::Uuid, String, i16)> = Builder::table("anvils")
//!     .get(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! The [get][Builder::get] method returns a `Vec` of database rows.
//! You may access each column's value by using the [Row::get] method, which
//! requires you to specify the expected type for each column:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), sqlx::Error> {
//! # let rows: Vec<(uuid::Uuid, String, i16)> = Builder::table("anvils")
//! #     .get(&connection)
//! #     .await?;
//! #
//! for (id, name, weight) in rows {
//!     println!("Anvil {} weighs {}", name, weight);
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! ### Retrieving a Single Row From a Table
//!
//! If you just need to retrieve a single row from a database table, you may use
//! the [Builder][Builder] [first][Builder::first] method. This method will
//! return a single [Row][sqlx::Row] object:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), sqlx::Error> {
//! let row: Option<(uuid::Uuid, String, i16)> = Builder::table("anvils")
//!     .first(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! If you would like to retrieve a single row from a database table, but throw
//! an error if no matching row is found, you may use the
//! [first_or_fail][Builder::first_or_fail] method:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), sqlx::Error> {
//! let row: (uuid::Uuid, String, i16) = Builder::table("anvils")
//!     .first_or_fail(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::sql::builder::*;
