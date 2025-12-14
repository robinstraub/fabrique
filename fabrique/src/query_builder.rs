//! # Database: Query Builder
//!
//! ## Introduction
//!
//! Fabrique's database query builder provides a convenient, fluent interface to
//! creating and running database queries. It can be used to perform most
//! database operations in your application and works with all of sqlx's
//! supported database systems (PostgreSQL, MySQL, SQLite, and MSSQL).
//!
//! The Fabrique query builder uses sqlx's parameterized queries to protect your
//! application against SQL injection attacks. There is no need to clean or
//! sanitize strings passed to the query builder as query parameters.
//!
//! ## Basic Where Clauses
//!
//! ### Where Clauses
//!
//! You may use the query builder's where method to add
//! [where][QueryBuilder::where] clauses to the query. The most basic call to
//! the where method requires three arguments. The first argument is the name of
//! the column. The second argument is an operator, which can be any of the
//! database's supported operators. The third argument is the value to compare
//! against the column's value.
//!
//! For example, the following query retrieves users where the value of the
//! votes column is equal to 100 and the value of the age column is greater than
//! 35:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::PgPool;
//!
//! # #[derive(Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     weight: i32,
//! # }
//! #
//! # async fn example(connection: PgPool) -> Result<(), sqlx::Error> {
//! let anvils = Anvil::query()
//!     .r#where(Anvil::WEIGHT, ">=", 42)
//!     .r#where(Anvil::WEIGHT, "<=", 99)
//!     .fetch_all(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! As previously mentioned, you may use any operator that is supported by your
//! database system:
//!
//!```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::PgPool;
//!
//! # #[derive(Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     name: String,
//! #     weight: i32,
//! # }
//! #
//! # async fn example(connection: PgPool) -> Result<(), sqlx::Error> {
//! Anvil::query().r#where(Anvil::WEIGHT, "=", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::WEIGHT, "!=", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::WEIGHT, "<", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::WEIGHT, "<=", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::WEIGHT, ">", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::WEIGHT, ">=", 42).fetch_all(&connection).await?;
//! Anvil::query().r#where(Anvil::NAME, "LIKE", "%anvil%".to_owned()).fetch_all(&connection).await?;
//! // --snip--
//! #     Ok(())
//! # }
//! ```
pub use fabrique_core::query_builder::*;
