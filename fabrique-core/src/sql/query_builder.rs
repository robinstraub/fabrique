//! # SQL Query Builder (Layer 1)
//!
//! Low-level SQL query builder using the **typestate pattern** for compile-time
//! query validation.
//!
//! This is the foundation layer of Fabrique's 2-layer query builder
//! architecture. It handles raw SQL construction and is designed to work with
//! any sqlx backend. For a detailed explanation of the architecture, see the
//! [Query Builder Internals](https://fabrique.rs/internals/query-builder.html) documentation.
//!
//! # Database Compatibility
//!
//! > **Note:** While the architecture is designed to be database-agnostic, the
//! > current
//! > implementation generates **PostgreSQL-specific SQL** in several places
//! > (e.g.,
//! > `RETURNING` clause, `ON CONFLICT` syntax). Support for other databases
//! > (MySQL,
//! > SQLite) requires specializing these implementations with alternative
//! > syntax.
//! > See [#69](https://github.com/robinstraub/fabrique/issues/69) for progress.
//!
//! # Typestate Pattern
//!
//! This module implements a **state machine** through a **typestate pattern**
//! where:
//! - Each state is a **zero-sized marker type** (e.g., [`Initial`],
//!   [`Selected`], [`Filtered<S>`])
//! - Method signatures encode **valid transitions** as return types
//! - **Invalid transitions are compile errors**, not runtime errors
//! - The builder is **consumed** by each method, preventing reuse after
//!   transition
//!
//! ## Example: Valid Transition Chain
//!
//! ```rust,no_run
//! # use fabrique_core::sql::QueryBuilder;
//! # use sqlx::PgPool;
//! # async fn example(pool: &PgPool) -> Result<(), sqlx::Error> {
//! let products: Vec<(i32, String)> = QueryBuilder::table("products")  // Initial
//!     .select(&["id", "name"])     // → Selected
//!     .r#where("price", ">", 100)  // → Filtered<Selected>
//!     .order_by("name", "asc")     // → Ordered
//!     .limit(10)                   // → Limited
//!     .get(pool).await?;           // → Executed
//! # Ok(())
//! # }
//! ```
//!
//! # State Transitions
//!
//! See the documentation of [`QueryBuilder`] for the complete state diagram.
//!
//! # Usage
//!
//! This layer is typically **not used directly**. Use
//! [`crate::model::QueryBuilder`] for the type-safe API that validates columns
//! against model definitions.
//!
//! Direct usage is appropriate for:
//! - Custom queries that don't map to a model
//! - Raw SQL escape hatches
//! - Testing the SQL layer in isolation
//!
//! # Module Organization
//!
//! This file declares all macros first (shared infrastructure for generating
//! methods across multiple states), then defines each state in sequence. For
//! each state, you will find its marker type definition, followed by its impl
//! blocks and any relevant macro invocations. States are ordered following the
//! logical sequence of query construction.

use std::marker::PhantomData;

use crate::sql::operators::{Direction, Operator};
use sqlx::{Database, Executor, FromRow, IntoArguments};

// ############################################################################
// MACRO DEFINITIONS
// ############################################################################
//
// Shared infrastructure for generating methods across multiple states.
// These macros reduce boilerplate by implementing common patterns (where,
// order_by, limit, get, first, etc.) for multiple builder states at once.

/// Implements the `join` method for a given input state.
///
/// Syntax: `impl_join!(InputState => OutputState)`
///
/// Generates `join()` method on `QueryBuilder<DB, InputState>` that
/// transitions to `QueryBuilder<DB, OutputState>`.
macro_rules! impl_join {
    ($input:ty => $output:ty) => {
        impl<DB: Database> QueryBuilder<DB, $input> {
            /// Adds an INNER JOIN clause to the query.
            ///
            /// Transitions to [`Joined`] state. Use additional `join()` calls to chain
            /// multiple joins.
            pub fn join(
                mut self,
                table: &str,
                left_column: &str,
                right_column: &str,
            ) -> QueryBuilder<DB, $output> {
                self.inner.push(" JOIN ");
                self.inner.push(table);
                self.inner.push(" ON ");
                self.inner.push(left_column);
                self.inner.push(" = ");
                self.inner.push(right_column);

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Joined {
                        _marker: PhantomData,
                    },
                }
            }
        }
    };
}

/// Implements the `where` methods for a given input state.
///
/// Syntax:
/// - `impl_where!(State)` - transitions to `Filtered<State>`
/// - `impl_where!(InputState => OutputState)` - transitions to
///   `Filtered<OutputState>`
///
/// Generates `r#where()`, `where_null()`, and `where_not_null()` methods.
macro_rules! impl_where {
    // Entry point: simple state (input = output)
    ($state:ty) => {
        impl_where!($state => $state);
    };
    // Entry point: explicit input => output
    ($input:ty => $output:ty) => {
        impl<DB: Database> QueryBuilder<DB, $input> {
            /// Adds a WHERE clause to the query.
            ///
            /// Transitions to [`Filtered`] state. Use additional `r#where()` calls to add
            /// AND conditions.
            pub fn r#where<'a, T, O>(
                mut self,
                column: &str,
                operator: O,
                value: T,
            ) -> QueryBuilder<DB, Filtered<$output>>
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

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Filtered { _marker: PhantomData },
                }
            }

            /// Adds a WHERE IS NULL clause to the query.
            ///
            /// Transitions to [`Filtered`] state.
            pub fn where_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<$output>> {
                self.inner.push(" WHERE ");
                self.inner.push(column);
                self.inner.push(" IS NULL");

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Filtered { _marker: PhantomData },
                }
            }

            /// Adds a WHERE IS NOT NULL clause to the query.
            ///
            /// Transitions to [`Filtered`] state.
            pub fn where_not_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<$output>> {
                self.inner.push(" WHERE ");
                self.inner.push(column);
                self.inner.push(" IS NOT NULL");

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Filtered { _marker: PhantomData },
                }
            }
        }
    };
}

/// Implements the `returning` method for a given input state.
///
/// Syntax: `impl_returning!(State)` - transitions to [`Returned`]
macro_rules! impl_returning {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Specifies the columns to return after the statement.
            ///
            /// Generates `RETURNING col1, col2, ...`. Transitions to [`Returned`]
            /// state.
            pub fn returning(mut self, columns: &[&str]) -> QueryBuilder<DB, Returned> {
                self.inner.push(" RETURNING ");
                self.inner.push(columns.join(", "));

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Returned,
                }
            }
        }
    };
}

/// Implements the `order_by` method for a given input state.
///
/// Syntax: `impl_order_by!(State)` - transitions to [`Ordered`]
macro_rules! impl_order_by {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Adds an `ORDER BY` clause to the query.
            ///
            /// Transitions to [`Ordered`] state.
            pub fn order_by(
                mut self,
                column: &str,
                direction: impl Into<Direction>,
            ) -> QueryBuilder<DB, Ordered> {
                let direction: Direction = direction.into();
                self.inner.push(" ORDER BY ");
                self.inner.push(column);
                self.inner.push(" ");
                self.inner.push(direction.as_str());

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Ordered,
                }
            }
        }
    };
}

/// Implements the `limit` method for a given input state.
///
/// Syntax: `impl_limit!(State)` - transitions to [`Limited`]
macro_rules! impl_limit {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Adds a `LIMIT` clause to the query.
            ///
            /// Transitions to [`Limited`] state.
            pub fn limit<'a>(mut self, count: i64) -> QueryBuilder<DB, Limited>
            where
                i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
            {
                self.inner.push(" LIMIT ");
                self.inner.push_bind(count);

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Limited,
                }
            }
        }
    };
}

/// Implements the `get` method for a given state.
///
/// Syntax: `impl_get!(State)` - adds execution capability (no state transition)
macro_rules! impl_get {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Executes the query and returns all matching rows.
            ///
            /// Supports both connection pools and transactions via the `Executor`
            /// trait.
            pub async fn get<'e, T, E>(mut self, executor: E) -> Result<Vec<T>, sqlx::Error>
            where
                E: Executor<'e, Database = DB>,
                T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                <DB as Database>::Arguments: IntoArguments<DB>,
            {
                self.inner.build_query_as::<T>().fetch_all(executor).await
            }
        }
    };
}

/// Implements the `first` method for a given state.
///
/// Syntax: `impl_first!(State)` - adds execution capability (no state
/// transition)
macro_rules! impl_first {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Retrieves the first row from the query result.
            ///
            /// Returns `None` if no rows match. Automatically adds `LIMIT 1`.
            pub async fn first<'e, T, E>(mut self, executor: E) -> Result<Option<T>, sqlx::Error>
            where
                E: Executor<'e, Database = DB>,
                T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                <DB as Database>::Arguments: IntoArguments<DB>,
            {
                self.inner.push(" LIMIT 1");
                self.inner
                    .build_query_as::<T>()
                    .fetch_optional(executor)
                    .await
            }
        }
    };
}

/// Implements the `first_or_fail` method for a given state.
///
/// Syntax: `impl_first_or_fail!(State)` - adds execution capability (no state
/// transition)
macro_rules! impl_first_or_fail {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Retrieves the first row from the query result, or fails if none exists.
            ///
            /// Returns an error if no rows match. Automatically adds `LIMIT 1`.
            pub async fn first_or_fail<'e, T, E>(mut self, executor: E) -> Result<T, sqlx::Error>
            where
                E: Executor<'e, Database = DB>,
                T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                <DB as Database>::Arguments: IntoArguments<DB>,
            {
                self.inner.push(" LIMIT 1");
                self.inner.build_query_as::<T>().fetch_one(executor).await
            }
        }
    };
}

/// Implements the `execute` method for a given state.
///
/// Syntax: `impl_execute!(State)` - adds execution capability (no state
/// transition)
macro_rules! impl_execute {
    ($state:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Executes the statement without returning any rows.
            pub async fn execute<'e, E>(mut self, executor: E) -> Result<(), sqlx::Error>
            where
                E: Executor<'e, Database = DB>,
                <DB as Database>::Arguments: IntoArguments<DB>,
            {
                self.inner.build().execute(executor).await.map(|_| ())
            }
        }
    };
}

// ############################################################################
// STATES
// ############################################################################
//
// Each state is a zero-sized marker type that encodes the current position
// in the query building process. The QueryBuilder struct is generic over
// its state, and method availability is controlled by impl blocks or
// macro invocations on specific state types.

// ============================================================================
// QueryBuilder
// ============================================================================

/// Type-safe SQL query builder using the typestate pattern.
///
/// Enforces correct SQL clause ordering at compile time by consuming itself
/// with each method call and transitioning to a new state. State transitions
/// are one-way, preventing invalid SQL clause sequences.
///
/// The builder wraps [`sqlx::QueryBuilder`] for safe parameterized query
/// construction, supporting any sqlx database backend (Postgres, MySQL, SQLite,
/// etc.).
///
/// # State Transitions
///
/// ```text
///                                  ┌───────────┐
///                                  │  Initial  │
///                                  └─────┬─────┘
///           ┌────────────────────────────┼──────────────────────────────────┐
///           │ insert()                   │ select()                         │ update()
///           ▼                            │                                  ▼
///      ┌──────────┐                      │                             ┌──────────┐
///      │Inserting │                      │                             │ Updating │
///      └────┬─────┘                      │                             └────┬─────┘
///           │ set()                      │                                  │ set()
///           ▼                            ▼                                  ▼
///      ┌──────────┐◀─┐              ┌──────────┐                       ┌──────────┐◀─┐
///      │ Inserted │  │ set()        │ Selected │                       │ Updated  │  │ set()
///      └────┬─────┘──┘              └────┬─────┘                       └────┬─────┘──┘
///           │                            │ join()                           │ join()
///           │                            ▼                                  │
///           │                   ┌──────────────────┐ ◀─┐           ┌─────────────────┐ ◀─┐
///           │                   │ Joined<Selected> │   │ join()    │ Joined<Updated> │   │ join()
///           │                   └────────┬─────────┘ ──┘           └────────┬────────┘ ──┘
///           │                            │ where()                          │ where()
///           │                            ▼                                  ▼
///           │                  ┌───────────────────┐◀─┐            ┌──────────────────┐◀─┐
///           │                  │Filtered<Selected> │  │ where()    │ Filtered<Updated>│  │ where()
///           │                  └─────────┬─────────┘──┘            └────────┬─────────┘──┘
///           │ on_conflict()              │ order_by()                       │ returning()
///           ▼                            ▼                                  │
///      ┌──────────┐                 ┌──────────┐                            │
///      │Conflicted│                 │ Ordered  │                            │
///      └────┬─────┘                 └────┬─────┘                            │
///           │                            │ limit()                          │
///           │ do_update()                ▼                                  │
///           │ do_nothing()          ┌──────────┐                            │
///           ▼                       │ Limited  │                            │
///      ┌──────────┐                 └────┬─────┘                            │
///      │ Upserted │                      │ offset()                         │
///      └────┬─────┘                      ▼                                  │
///           │ returning()           ┌──────────┐                            │
///           │                       │ Offsetted│                            │
///           │                       └────┬─────┘                            │
///           ▼                            │                                  │
///      ┌──────────┐◀─────────────────────│──────────────────────────────────┘
///      │ Returned │                      │
///      └────┬─────┘                      │
///           │                            │
///           ▼                            ▼
/// ┌─────────────────────────────────────────────────────────────────────────────────────┐
/// │                                     Executed                                        │
/// └─────────────────────────────────────────────────────────────────────────────────────┘
/// ```
pub struct QueryBuilder<DB: Database, S = Initial> {
    /// Underlying sqlx query builder, used for safe parameter binding and
    /// SQL string construction. This handles input sanitization.
    inner: sqlx::QueryBuilder<DB>,
    /// The table name specified at construction. Used when generating the
    /// opening clause (`SELECT ... FROM`, `INSERT INTO`, `UPDATE`).
    table: String,
    /// Current state marker. Most states are zero-sized types (ZST), but some
    /// like [`Inserted`] carry data needed for deferred SQL generation.
    state: S,
}

// ============================================================================
// Initial
// ============================================================================

/// Entry point for query building.
///
/// In this state, the table name is captured but no SQL is generated yet.
/// This deferred generation is intentional: the opening clause differs based
/// on the operation (`SELECT ... FROM`, `INSERT INTO`, `UPDATE ... SET`),
/// so we wait until the operation is known.
///
/// From here, transition to:
/// - [`Selected`] via [`Initial::select`]
/// - [`Inserting`] via [`Initial::insert`]
/// - [`Updating`] via [`Initial::update`]
pub struct Initial;

// ============================================================================
// Selected
// ============================================================================

/// State after `SELECT columns FROM table` has been generated.
///
/// From here, transition to:
/// - [`Joined`] via [`QueryBuilder::join`]
/// - [`Filtered`] via [`QueryBuilder::r#where`]
/// - [`Ordered`] via [`QueryBuilder::order_by`]
/// - [`Limited`] via [`QueryBuilder::limit`]
/// - Or execute directly with [`QueryBuilder::get`], [`QueryBuilder::first`]
pub struct Selected;

// Transitions from Selected
impl_join!(Selected => Joined<Selected>);
impl_where!(Selected);
impl_order_by!(Selected);
impl_limit!(Selected);

// Execution from Selected
impl_get!(Selected);
impl_first!(Selected);
impl_first_or_fail!(Selected);

// ============================================================================
// Joined
// ============================================================================

/// State after one or more `JOIN` clauses have been added.
///
/// Generic over `Source` to track which base state we joined from (e.g.,
/// `Joined<Selected>` or `Joined<Updated>`). This controls which methods
/// remain available after joining.
pub struct Joined<Source = Selected> {
    _marker: PhantomData<Source>,
}

// Transitions from Joined<Selected>
impl_join!(Joined<Selected> => Joined<Selected>);
impl_where!(Joined<Selected> => Selected);
impl_order_by!(Joined<Selected>);
impl_limit!(Joined<Selected>);

// Execution from Joined<Selected>
impl_get!(Joined<Selected>);
impl_first!(Joined<Selected>);
impl_first_or_fail!(Joined<Selected>);

// Transitions from Joined<Updated>
impl_join!(Joined<Updated> => Joined<Updated>);
impl_where!(Joined<Updated> => Updated);

// ============================================================================
// Filtered
// ============================================================================

/// State after one or more `WHERE` clauses have been added.
///
/// Generic over `Source` to control which states are reachable:
/// - `Filtered<Selected>`: can transition to [`Ordered`], [`Limited`],
///   [`Offsetted`]
/// - `Filtered<Updated>`: can transition to [`Returned`]
pub struct Filtered<Source = Selected> {
    _marker: PhantomData<Source>,
}

// Transitions from Filtered
impl_order_by!(Filtered<Selected>);
impl_limit!(Filtered<Selected>);
impl_returning!(Filtered<Updated>);

// Execution from Filtered
impl_get!(Filtered<Selected>);
impl_first!(Filtered<Selected>);
impl_first_or_fail!(Filtered<Selected>);
impl_execute!(Filtered<Updated>);

// ============================================================================
// Ordered
// ============================================================================

/// State after an `ORDER BY` clause has been added.
///
/// Can transition to [`Limited`] or execute directly.
pub struct Ordered;

// Transitions from Ordered
impl_limit!(Ordered);

// Execution from Ordered
impl_get!(Ordered);
impl_first!(Ordered);
impl_first_or_fail!(Ordered);

// ============================================================================
// Limited
// ============================================================================

/// State after a `LIMIT` clause has been added.
///
/// Can transition to [`Offsetted`] or execute directly.
pub struct Limited;

// Execution from Limited
impl_get!(Limited);

// ============================================================================
// Offsetted
// ============================================================================

/// State after an `OFFSET` clause has been added.
///
/// Leaf state in the [`Selected`] flow - can only execute from here.
pub struct Offsetted;

// Execution from Offsetted
impl_get!(Offsetted);

// ============================================================================
// Inserting
// ============================================================================

/// State after `INSERT INTO table` - waiting for at least one column.
///
/// Must transition to [`Inserted`] by calling `set()` at least once.
pub struct Inserting;

// ============================================================================
// Inserted
// ============================================================================

/// Type alias for boxed bind functions used to defer value binding.
type BindFn<DB> = Box<dyn FnOnce(&mut sqlx::QueryBuilder<DB>) + Send>;

/// State after one or more columns have been set for INSERT.
///
/// Unlike most states, this is **not** a ZST: it accumulates column names and
/// bound values until the statement is finalized. SQL generation is deferred
/// because the full column list must be known before generating the query.
///
/// Can transition to:
/// - [`Conflicted`] via `on_conflict()`
/// - [`Returned`] via `returning()`
/// - Or execute directly
pub struct Inserted<DB: Database> {
    /// Column names accumulated via `set()` calls.
    columns: Vec<String>,
    /// Closures that bind values, executed when finalizing the statement.
    bind_fns: Vec<BindFn<DB>>,
}

// ============================================================================
// Updating
// ============================================================================

/// State after `UPDATE table` - waiting for at least one column.
///
/// Must transition to [`Updated`] by calling `set()` at least once.
pub struct Updating;

// ============================================================================
// Updated
// ============================================================================

/// State after one or more columns have been set for UPDATE.
///
/// Can call `set()` again to remain in this state, or transition to:
/// - [`Joined`] via `join()`
/// - [`Filtered`] via `r#where()`
pub struct Updated;

// Transitions from Updated
impl_join!(Updated => Joined<Updated>);
impl_where!(Updated);
impl_returning!(Updated);

// Execution from Updated
impl_execute!(Updated);

// ============================================================================
// Conflicted
// ============================================================================

/// State after `ON CONFLICT (columns)` has been added to an INSERT.
///
/// Must transition to [`Upserted`] via either:
/// - `do_update()` - update conflicting rows
/// - `do_nothing()` - ignore conflicts
pub struct Conflicted;

// ============================================================================
// Upserted
// ============================================================================

/// State after `DO UPDATE` or `DO NOTHING` has been added.
///
/// Can transition to [`Returned`] via `returning()` or execute directly.
pub struct Upserted;

// Transitions from Upserted
impl_returning!(Upserted);

// Execution from Upserted
impl_execute!(Upserted);

// ============================================================================
// Returned
// ============================================================================

/// State after a `RETURNING` clause has been added.
///
/// Terminal state - can only execute from here. Execution returns the
/// specified columns from affected rows.
pub struct Returned;

impl<DB: Database> QueryBuilder<DB, Initial> {
    /// Creates a new query builder for the specified table.
    pub fn table(table: impl Into<String>) -> Self {
        Self {
            inner: sqlx::QueryBuilder::new(""),
            table: table.into(),
            state: Initial,
        }
    }

    /// Specifies the columns to select.
    ///
    /// Transitions to [`Selected`] state, allowing WHERE, ORDER BY, LIMIT, or
    /// execution.
    pub fn select(mut self, columns: &[&str]) -> QueryBuilder<DB, Selected> {
        let columns = columns.join(", ");
        let query = format!("SELECT {} FROM {}", columns, &self.table);
        self.inner.push(query);

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Selected,
        }
    }

    /// Starts an INSERT statement.
    ///
    /// Transitions to [`Inserting`] state, requiring at least one `.set()`
    /// call.
    pub fn insert(self) -> QueryBuilder<DB, Inserting> {
        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Inserting,
        }
    }

    /// Starts an UPDATE statement.
    ///
    /// Transitions to [`Updating`] state, requiring at least one `.set()` call.
    pub fn update(mut self) -> QueryBuilder<DB, Updating> {
        self.inner.push("UPDATE ");
        self.inner.push(&self.table);

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Updating,
        }
    }
}

impl<DB: Database, Source> QueryBuilder<DB, Filtered<Source>> {
    /// Adds an additional WHERE clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// preserving the source type.
    pub fn r#where<'a, T, O>(
        mut self,
        column: &str,
        operator: O,
        value: T,
    ) -> QueryBuilder<DB, Filtered<Source>>
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

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Filtered {
                _marker: PhantomData,
            },
        }
    }

    /// Adds an additional WHERE NULL clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// preserving the source type.
    pub fn where_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<Source>> {
        self.inner.push(" AND ");
        self.inner.push(column);
        self.inner.push(" IS NULL");

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Filtered {
                _marker: PhantomData,
            },
        }
    }

    /// Adds an additional WHERE NOT NULL clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// preserving the source type.
    pub fn where_not_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<Source>> {
        self.inner.push(" AND ");
        self.inner.push(column);
        self.inner.push(" IS NOT NULL");

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Filtered {
                _marker: PhantomData,
            },
        }
    }
}

impl<DB: Database> QueryBuilder<DB, Inserting> {
    /// Sets a column value for the INSERT statement.
    ///
    /// Transitions to [`Inserted`] state. Call multiple times to set additional
    /// columns.
    pub fn set<'a, T>(self, column: &str, value: T) -> QueryBuilder<DB, Inserted<DB>>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB> + Send + 'static,
    {
        let mut columns = Vec::new();
        let mut bind_fns: Vec<BindFn<DB>> = Vec::new();

        columns.push(column.to_string());
        bind_fns.push(Box::new(move |builder: &mut sqlx::QueryBuilder<DB>| {
            builder.push_bind(value);
        }));

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Inserted { columns, bind_fns },
        }
    }
}

impl<DB: Database> QueryBuilder<DB, Inserted<DB>> {
    /// Sets an additional column value for the INSERT statement.
    ///
    /// Remains in [`Inserted`] state. Call multiple times to set additional
    /// columns.
    pub fn set<'a, T>(mut self, column: &str, value: T) -> QueryBuilder<DB, Inserted<DB>>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB> + Send + 'static,
    {
        self.state.columns.push(column.to_string());
        self.state
            .bind_fns
            .push(Box::new(move |builder: &mut sqlx::QueryBuilder<DB>| {
                builder.push_bind(value);
            }));

        self
    }

    /// Specifies the conflict target columns for an UPSERT operation.
    ///
    /// Flushes the accumulated INSERT data and transitions to [`Conflicted`]
    /// state.
    pub fn on_conflict(mut self, columns: &[&str]) -> QueryBuilder<DB, Conflicted> {
        self.inner.push("INSERT INTO ");
        self.inner.push(&self.table);
        self.inner.push(" (");
        self.inner.push(self.state.columns.join(", "));
        self.inner.push(") VALUES (");

        let mut first = true;
        for bind_fn in self.state.bind_fns {
            if !first {
                self.inner.push(", ");
            }
            first = false;
            bind_fn(&mut self.inner);
        }

        self.inner.push(") ON CONFLICT (");
        self.inner.push(columns.join(", "));
        self.inner.push(")");

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Conflicted,
        }
    }

    /// Specifies the columns to return after the INSERT.
    ///
    /// Flushes the accumulated INSERT data and generates `RETURNING col1, col2,
    /// ...`.
    pub fn returning(mut self, columns: &[&str]) -> QueryBuilder<DB, Returned> {
        self.inner.push("INSERT INTO ");
        self.inner.push(&self.table);
        self.inner.push(" (");
        self.inner.push(self.state.columns.join(", "));
        self.inner.push(") VALUES (");

        let mut first = true;
        for bind_fn in self.state.bind_fns {
            if !first {
                self.inner.push(", ");
            }
            first = false;
            bind_fn(&mut self.inner);
        }

        self.inner.push(") RETURNING ");
        self.inner.push(columns.join(", "));

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Returned,
        }
    }

    /// Executes the INSERT statement without returning any rows.
    ///
    /// Flushes the accumulated INSERT data and executes.
    pub async fn execute<'e, E>(mut self, executor: E) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        self.inner.push("INSERT INTO ");
        self.inner.push(&self.table);
        self.inner.push(" (");
        self.inner.push(self.state.columns.join(", "));
        self.inner.push(") VALUES (");

        let mut first = true;
        for bind_fn in self.state.bind_fns {
            if !first {
                self.inner.push(", ");
            }
            first = false;
            bind_fn(&mut self.inner);
        }

        self.inner.push(")");

        self.inner.build().execute(executor).await.map(|_| ())
    }
}

impl<DB: Database> QueryBuilder<DB, Conflicted> {
    /// Specifies that conflicting rows should be updated.
    ///
    /// Generates `DO UPDATE SET col = EXCLUDED.col` for each specified column.
    /// Transitions to [`Upserted`] state, requiring `returning()` before
    /// execution.
    pub fn do_update(mut self, columns: &[&str]) -> QueryBuilder<DB, Upserted> {
        let set_clause = columns
            .iter()
            .map(|col| format!("{} = EXCLUDED.{}", col, col))
            .collect::<Vec<_>>()
            .join(", ");

        self.inner.push(" DO UPDATE SET ");
        self.inner.push(set_clause);

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Upserted,
        }
    }

    /// Specifies that conflicting rows should be ignored.
    ///
    /// Generates `DO NOTHING`. Transitions to [`Upserted`] state.
    pub fn do_nothing(mut self) -> QueryBuilder<DB, Upserted> {
        self.inner.push(" DO NOTHING");

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Upserted,
        }
    }
}

impl<DB: Database> QueryBuilder<DB, Updating> {
    /// Sets a column value for the UPDATE statement.
    ///
    /// Transitions to [`Updated`] state. Call multiple times to set additional
    /// columns.
    pub fn set<'a, T>(mut self, column: &str, value: T) -> QueryBuilder<DB, Updated>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    {
        self.inner.push(" SET ");
        self.inner.push(column);
        self.inner.push(" = ");
        self.inner.push_bind(value);

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Updated,
        }
    }
}

impl<DB: Database> QueryBuilder<DB, Updated> {
    /// Sets an additional column value for the UPDATE statement.
    ///
    /// Remains in [`Updated`] state. Call multiple times to set additional
    /// columns.
    pub fn set<'a, T>(mut self, column: &str, value: T) -> QueryBuilder<DB, Updated>
    where
        T: 'a + sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    {
        self.inner.push(", ");
        self.inner.push(column);
        self.inner.push(" = ");
        self.inner.push_bind(value);

        self
    }
}

impl<DB: Database> QueryBuilder<DB, Returned> {
    /// Executes the query and returns all resulting rows.
    pub async fn get<'e, T, E>(mut self, executor: E) -> Result<Vec<T>, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        self.inner.build_query_as::<T>().fetch_all(executor).await
    }

    /// Executes the query and returns the first resulting row.
    pub async fn first<'e, T, E>(mut self, executor: E) -> Result<Option<T>, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        self.inner
            .build_query_as::<T>()
            .fetch_optional(executor)
            .await
    }

    /// Executes the query and returns the first resulting row, or fails.
    pub async fn first_or_fail<'e, T, E>(mut self, executor: E) -> Result<T, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        self.inner.build_query_as::<T>().fetch_one(executor).await
    }
}

impl<DB: Database> QueryBuilder<DB, Limited> {
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Transitions to [`Offsetted`] state, allowing execution.
    pub fn offset<'a>(mut self, count: i64) -> QueryBuilder<DB, Offsetted>
    where
        i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    {
        self.inner.push(" OFFSET ");
        self.inner.push_bind(count);

        QueryBuilder {
            inner: self.inner,
            table: self.table,
            state: Offsetted,
        }
    }
}
