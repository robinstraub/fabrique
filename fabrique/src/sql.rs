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
//! You may use the [table][QueryBuilder::table] method provided by the
//! [QueryBuilder] struct to begin a query. The [table][QueryBuilder::table]
//! method returns a fluent query builder instance for the given table, allowing
//! you to chain more constraints onto the query and then finally retrieve the
//! results of the query using the [get][QueryBuilder::get] method:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::sql::QueryBuilder;
//! # use fabrique::prelude::*;
//! #
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! let rows: Vec<(String, String, i32)> = QueryBuilder::table("products")
//!     .select(&["id", "name", "price_cents"])
//!     .get(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! The [get][QueryBuilder::get] method returns a `Vec` of database rows.
//! You may access each column's value by using the [Row::get] method, which
//! requires you to specify the expected type for each column:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::sql::QueryBuilder;
//! # use fabrique::prelude::*;
//! #
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! let rows: Vec<(String, String, i32)> = QueryBuilder::table("products")
//!     .select(&["id", "name", "price_cents"])
//!     .get(&pool)
//!     .await?;
//!
//! for (id, name, price_cents) in rows {
//!     println!("Product {} costs {} cents", name, price_cents);
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! ### Retrieving a Single Row From a Table
//!
//! If you just need to retrieve a single row from a database table, you may use
//! the [QueryBuilder][QueryBuilder] [first][QueryBuilder::first] method. This
//! method will return a single [Row][sqlx::Row] object:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::sql::QueryBuilder;
//! # use fabrique::prelude::*;
//! #
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! let row: Option<(String, String, i32)> = QueryBuilder::table("products")
//!     .select(&["id", "name", "price_cents"])
//!     .first(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! If you would like to retrieve a single row from a database table, but throw
//! an error if no matching row is found, you may use the
//! [first_or_fail][QueryBuilder::first_or_fail] method:
//!
//! ```rust
//! # extern crate fabrique;
//! # extern crate sqlx;
//! # extern crate tokio;
//! # extern crate uuid;
//! # use fabrique::sql::QueryBuilder;
//! # use fabrique::prelude::*;
//! #
//! # #[fabrique::doctest]
//! # async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
//! # sqlx::query("INSERT INTO products (id, name, price_cents) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'Widget', 1200)")
//! #     .execute(&pool).await?;
//! let row: (String, String, i32) = QueryBuilder::table("products")
//!     .select(&["id", "name", "price_cents"])
//!     .first_or_fail(&pool)
//!     .await?;
//! #     Ok(())
//! # }
//! ```

pub use fabrique_core::sql::*;
