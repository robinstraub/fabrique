//! Type-safe SQL query builder with compile-time state enforcement.
//!
//! This module implements a query builder using the typestate pattern to
//! enforce correct SQL clause ordering at compile time. The builder consumes
//! itself with each method call, transitioning to a new state. State
//! transitions are one-way, preventing invalid SQL clause sequences.
//!
//! # State Transition Matrix
//!
//! The table shows which methods are available on each builder state.
//! Checkmarks indicate available methods. Methods transition to new states
//! (shown in return types), and these transitions cannot be reversed. Optional
//! clauses can be skipped by calling methods that jump to later states.
//!
//! ```text
//! State      │ select │ where │ order_by │ limit │ offset │ fetch_*
//! ───────────┼────────┼───────┼──────────┼───────┼────────┼────────
//! Initial    │   ✓    │       │          │   ✓   │        │
//! Selected   │        │   ✓   │    ✓     │   ✓   │        │   ✓
//! Filtered   │        │   ✓   │    ✓     │   ✓   │        │   ✓
//! Ordered    │        │       │          │   ✓   │        │   ✓
//! Limited    │        │       │          │       │   ✓    │   ✓
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabrique::sql::Builder;
//! use fabrique::query_builder::Direction;
//! use sqlx::Postgres;
//!
//! // Full query with all clauses
//! let query = Builder::<Postgres>::table("anvils")
//!     .select(&["id", "name", "weight"])
//!     .where("weight", ">=", 100)
//!     .order_by("name", Direction::Asc)
//!     .limit(10)
//!     .fetch_all(&pool)
//!     .await?;
//!
//! // With transactions
//! let mut tx = pool.begin().await?;
//!
//! Builder::<Postgres>::table("anvils")
//!     .select(&["id"])
//!     .limit(10)
//!     .fetch_all(&mut tx)
//!     .await?;
//!
//! tx.commit().await?;
//! ```

use crate::query_builder::{Direction, Operator};
use sqlx::{Database, Executor, FromRow, IntoArguments};
use std::marker::PhantomData;

/// Implements the `order_by` method for multiple builder states.
///
/// Always transitions to [`Ordered`] state.
macro_rules! impl_order_by {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<'a, DB: Database> Builder<'a, DB, $state> {
                /// Adds an `ORDER BY` clause to the query.
                ///
                /// Transitions to [`Ordered`] state, allowing `LIMIT` or execution.
                pub fn order_by(
                    mut self,
                    column: &str,
                    direction: impl Into<Direction>,
                ) -> Builder<'a, DB, Ordered> {
                    let direction: Direction = direction.into();
                    self.inner.push(" ORDER BY ");
                    self.inner.push(column);
                    self.inner.push(" ");
                    self.inner.push(direction.as_str());

                    Builder {
                        inner: self.inner,
                        table: self.table,
                        _state: PhantomData,
                    }
                }
            }
        )+
    };
}

/// Implements the `limit` method for multiple builder states.
///
/// Always transitions to [`Limited`] state.
macro_rules! impl_limit {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<'a, DB: Database> Builder<'a, DB, $state> {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
                pub fn limit(mut self, count: i64) -> Builder<'a, DB, Limited>
                where
                    i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
                {
                    self.inner.push(" LIMIT ");
                    self.inner.push_bind(count);

                    Builder {
                        inner: self.inner,
                        table: self.table,
                        _state: PhantomData,
                    }
                }
            }
        )+
    };
}

/// Implements the `fetch_all` method for multiple builder states.
macro_rules! impl_fetch_all {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<'a, DB: Database> Builder<'a, DB, $state> {
                /// Executes the query and returns all matching rows.
                ///
                /// Supports both connection pools and transactions via the `Executor`
                /// trait.
                pub async fn fetch_all<'s, T, E>(&'s mut self, executor: E) -> Result<Vec<T>, sqlx::Error>
                where
                    E: Executor<'a, Database = DB>,
                    T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                    for<'q> <DB as Database>::Arguments<'q>: IntoArguments<'q, DB>,
                    's: 'a,
                {
                    self.inner.build_query_as::<T>().fetch_all(executor).await
                }
            }
        )+
    };
}

/// Type-safe SQL query builder using the typestate pattern.
///
/// Enforces correct SQL clause ordering at compile time by transitioning
/// through states: [`Initial`] → [`Selected`] → [`Filtered`] → [`Ordered`] →
/// [`Limited`]
///
/// Wraps `sqlx::QueryBuilder` for safe parameterized query construction.
///
/// Generic over database type `DB`, supporting any sqlx database backend
/// (Postgres, MySQL, SQLite, etc.).
pub struct Builder<'a, DB: Database, S = Initial> {
    inner: sqlx::QueryBuilder<'a, DB>,
    table: String,
    _state: PhantomData<(S, DB)>,
}

/// Initial state - table specified, ready for query construction.
pub struct Initial;

/// Selected state - SELECT clause added, can filter, order, limit, or execute.
pub struct Selected;

/// Filtered state - WHERE clause(s) added, can add more filters, order, limit,
/// or execute.
pub struct Filtered;

/// Ordered state - ORDER BY added, can limit or execute.
pub struct Ordered;

/// Limited state - LIMIT added, can offset or execute.
pub struct Limited;

impl<'a, DB: Database> Builder<'a, DB, Initial> {
    /// Creates a new query builder for the specified table.
    pub fn table(table: impl Into<String>) -> Self {
        Self {
            inner: sqlx::QueryBuilder::new(""),
            table: table.into(),
            _state: PhantomData,
        }
    }

    /// Specifies the columns to select.
    ///
    /// Transitions to [`Selected`] state, allowing WHERE, ORDER BY, LIMIT, or
    /// execution.
    pub fn select(mut self, columns: &[&str]) -> Builder<'a, DB, Selected> {
        let columns = columns.join(", ");
        let query = format!("SELECT {} FROM {}", columns, &self.table);
        self.inner.push(query);

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

impl<'a, DB: Database> Builder<'a, DB, Selected> {
    /// Adds a WHERE clause to the query.
    ///
    /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
    /// AND conditions.
    pub fn r#where<T, O>(mut self, column: &str, operator: O, value: T) -> Builder<'a, DB, Filtered>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        O: Into<Operator>,
    {
        self.inner.push(" WHERE ");
        self.inner.push(column);
        self.inner.push(" ");
        self.inner.push(operator.into().as_str());
        self.inner.push(" ");
        self.inner.push_bind(value);

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

impl<'a, DB: Database> Builder<'a, DB, Filtered> {
    /// Adds an additional WHERE clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// allowing additional conditions, ORDER BY, LIMIT, or execution.
    pub fn r#where<T, O>(mut self, column: &str, operator: O, value: T) -> Builder<'a, DB, Filtered>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB>,
        O: Into<Operator>,
    {
        self.inner.push(" AND ");
        self.inner.push(column);
        self.inner.push(" ");
        self.inner.push(operator.into().as_str());
        self.inner.push(" ");
        self.inner.push_bind(value);

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

impl<'a, DB: Database> Builder<'a, DB, Ordered> {}

impl<'a, DB: Database> Builder<'a, DB, Limited> {
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Remains in [`Limited`] state, allowing execution.
    pub fn offset(mut self, count: i64) -> Builder<'a, DB, Limited>
    where
        i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    {
        self.inner.push(" OFFSET ");
        self.inner.push_bind(count);

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

// Use macros to implement common methods across multiple states
impl_order_by!(Selected, Filtered);
impl_limit!(Selected, Filtered, Ordered);
impl_fetch_all!(Selected, Filtered, Ordered, Limited);

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    #[sqlx::test(migrations = "../migrations")]
    async fn test(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i16)>, sqlx::Error> = Builder::table("anvils")
            .select(&["id", "name", "weight"])
            .r#where("weight", ">=", 10)
            .r#where("weight", "<=", 99)
            .order_by("weight", "ASC")
            .limit(10)
            .fetch_all(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }
}
