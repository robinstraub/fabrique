use std::marker::PhantomData;

use crate::sql::operators::{Direction, Operator};
use sqlx::{Database, Executor, FromRow, IntoArguments};

/// Implements the `join` method for base states and their joined variants.
///
/// For each base state, generates:
/// - `Base` → `Joined<Base>` (initial join)
/// - `Joined<Base>` → `Joined<Base>` (chained joins)
macro_rules! impl_join {
    // Shared body - takes input state and output base
    (@body $state:ty, $base:ty) => {
        impl<DB: Database> QueryBuilder<DB, $state> {
            /// Adds an INNER JOIN clause to the query.
            ///
            /// Transitions to [`Joined`] state. Use additional `join()` calls to chain
            /// multiple joins.
            pub fn join(
                mut self,
                table: &str,
                left_column: &str,
                right_column: &str,
            ) -> QueryBuilder<DB, Joined<$base>> {
                self.inner.push(" JOIN ");
                self.inner.push(table);
                self.inner.push(" ON ");
                self.inner.push(left_column);
                self.inner.push(" = ");
                self.inner.push(right_column);

                QueryBuilder {
                    inner: self.inner,
                    table: self.table,
                    state: Joined { _marker: PhantomData },
                }
            }
        }
    };
    // Entry: for each base, generate both impls
    ($($base:ty),+ $(,)?) => {
        $(
            impl_join!(@body $base, $base);
            impl_join!(@body Joined<$base>, $base);
        )+
    };
}

/// Implements the `where` method for states that start a WHERE clause.
///
/// Pushes " WHERE " and transitions to [`Filtered<$state>`].
macro_rules! impl_where {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Adds a WHERE clause to the query.
                ///
                /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
                /// AND conditions.
                pub fn r#where<'a, T, O>(
                    mut self,
                    column: &str,
                    operator: O,
                    value: T,
                ) -> QueryBuilder<DB, Filtered<$state>>
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

                /// Adds a WHERE NULL clause to the query.
                ///
                /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
                /// AND conditions.
                pub fn where_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<$state>> {
                    self.inner.push(" WHERE ");
                    self.inner.push(column);
                    self.inner.push(" IS NULL");

                    QueryBuilder {
                        inner: self.inner,
                        table: self.table,
                        state: Filtered { _marker: PhantomData },
                    }
                }

                /// Adds a WHERE NOT NULL clause to the query.
                ///
                /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
                /// AND conditions.
                pub fn where_not_null(mut self, column: &str) -> QueryBuilder<DB, Filtered<$state>> {
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
        )+
    };
}

/// Implements the `returning` method for multiple builder states.
///
/// Always transitions to [`Returned`] state.
macro_rules! impl_returning {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Specifies the columns to return after the statement.
                ///
                /// Generates `RETURNING col1, col2, ...`.
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
        )+
    };
}

/// Implements the `order_by` method for multiple builder states.
///
/// Always transitions to [`Ordered`] state.
macro_rules! impl_order_by {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Adds an `ORDER BY` clause to the query.
                ///
                /// Transitions to [`Ordered`] state, allowing `LIMIT` or execution.
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
        )+
    };
}

/// Implements the `limit` method for multiple builder states.
///
/// Always transitions to [`Limited`] state.
macro_rules! impl_limit {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
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
        )+
    };
}

/// Implements the `get` method for multiple builder states.
macro_rules! impl_get {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Executes the query and returns all matching rows.
                ///
                /// Supports both connection pools and transactions via the `Executor`
                /// trait.
                pub async fn get<'e, T, E>(mut self, executor: E) -> Result<Vec<T>, sqlx::Error>
                where
                    E:  Executor<'e, Database = DB>,
                    T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                    <DB as Database>::Arguments: IntoArguments<DB>,
                {
                    self.inner.build_query_as::<T>().fetch_all(executor).await
                }
            }
        )+
    };
}

/// Implements the `first` method for multiple builder states.
macro_rules! impl_first {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Retrieves the first row from the query result.
                ///
                /// Returns `None` if no rows match the query. Automatically adds
                /// `LIMIT 1` to the query.
                pub async fn first<'e, T, E>(mut self, executor: E) -> Result<Option<T>, sqlx::Error>
                where
                    E: Executor<'e, Database = DB>,
                    T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
                    <DB as Database>::Arguments: IntoArguments<DB>,
                {
                    self.inner.push(" LIMIT 1");
                    self.inner.build_query_as::<T>().fetch_optional(executor).await
                }
            }
        )+
    };
}

/// Implements the `first_or_fail` method for multiple builder states.
macro_rules! impl_first_or_fail {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Retrieves the first row from the query result, or fails if none exists.
                ///
                /// Returns an error if no rows match the query. Automatically adds
                /// `LIMIT 1` to the query.
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
        )+
    };
}

/// Implements the `execute` method for multiple builder states.
///
/// Executes the query without returning rows.
macro_rules! impl_execute {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<DB: Database> QueryBuilder<DB, $state> {
                /// Executes the statement without returning any rows.
                pub async fn execute<'e, E>(mut self, executor: E) -> Result<(), sqlx::Error>
                where
                    E: Executor<'e, Database = DB>,
                    <DB as Database>::Arguments: IntoArguments<DB>,
                {
                    self.inner
                        .build()
                        .execute(executor)
                        .await
                        .map(|_| ())
                }
            }
        )+
    };
}

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
    inner: sqlx::QueryBuilder<DB>,
    table: String,
    state: S,
}

/// Initial state - table specified, ready for query construction.
pub struct Initial;

/// Selected state - SELECT clause added.
pub struct Selected;

/// Joined state - JOIN clause(s) added.
///
/// Generic over the source state to enable chained joins and control
/// which methods are available after joining.
pub struct Joined<Source = Selected> {
    _marker: PhantomData<Source>,
}

/// Filtered state - WHERE clause(s) added.
///
/// Generic over the source state to control which methods are available:
/// - `Filtered<Selected>`: Allows ORDER BY, LIMIT, OFFSET (valid for SELECT)
/// - `Filtered<Updated>`: Only allows RETURNING (UPDATE doesn't support ORDER
///   BY/LIMIT)
pub struct Filtered<Source = Selected> {
    _marker: PhantomData<Source>,
}

/// Ordered state - ORDER BY added.
pub struct Ordered;

/// Limited state - LIMIT added.
pub struct Limited;

/// Offsetted state - OFFSET added.
pub struct Offsetted;

/// Inserting state - INSERT INTO added, waiting for at least one `.set()`.
pub struct Inserting;

/// Type alias for boxed bind functions used in INSERT statements.
type BindFn<DB> = Box<dyn FnOnce(&mut sqlx::QueryBuilder<DB>) + Send>;

/// Inserted state - at least one column set for INSERT.
///
/// Accumulates columns and their bound values until the INSERT statement
/// is finalized via `.on_conflict()`, `.returning()`, or execution.
pub struct Inserted<DB: Database> {
    columns: Vec<String>,
    bind_fns: Vec<BindFn<DB>>,
}

/// Updating state - UPDATE added, waiting for at least one `.set()`.
pub struct Updating;

/// Updated state - at least one column set for UPDATE.
pub struct Updated;

/// Upserted state - DO UPDATE clause added to an INSERT ON CONFLICT.
pub struct Upserted;

/// Conflicted state - ON CONFLICT added.
pub struct Conflicted;

/// Returned state - RETURNING clause added.
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

    /// Executes `SELECT * FROM {table}` and returns all matching rows.
    ///
    /// Automatically selects all columns from the table.
    pub async fn get<'e, T, E>(mut self, executor: E) -> Result<Vec<T>, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        let query = format!("SELECT * FROM {}", &self.table);
        self.inner.push(query);
        self.inner.build_query_as::<T>().fetch_all(executor).await
    }

    /// Executes `SELECT * FROM {table} LIMIT 1` and returns the first row.
    ///
    /// Returns `None` if no rows exist. Automatically selects all columns.
    pub async fn first<'e, T, E>(mut self, executor: E) -> Result<Option<T>, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        let query = format!("SELECT * FROM {}", &self.table);
        self.inner.push(query);
        self.inner.push(" LIMIT 1");
        self.inner
            .build_query_as::<T>()
            .fetch_optional(executor)
            .await
    }

    /// Executes `SELECT * FROM {table} LIMIT 1` and returns the first row, or
    /// fails.
    ///
    /// Returns an error if no rows exist. Automatically selects all columns.
    pub async fn first_or_fail<'e, T, E>(mut self, executor: E) -> Result<T, sqlx::Error>
    where
        E: Executor<'e, Database = DB>,
        T: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
        <DB as Database>::Arguments: IntoArguments<DB>,
    {
        let query = format!("SELECT * FROM {}", &self.table);
        self.inner.push(query);
        self.inner.push(" LIMIT 1");
        self.inner.build_query_as::<T>().fetch_one(executor).await
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

impl<DB: Database> QueryBuilder<DB, Ordered> {}

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

impl<DB: Database> QueryBuilder<DB, Upserted> {
    /// Specifies the columns to return after the UPSERT.
    ///
    /// Generates `RETURNING col1, col2, ...`.
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

impl<DB: Database> QueryBuilder<DB, Filtered<Updated>> {
    /// Specifies the columns to return after the UPDATE statement.
    ///
    /// Generates `RETURNING col1, col2, ...`.
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

// Use macros to implement common methods across multiple states
impl_join!(Selected, Updated);
impl_where!(Selected, Updated, Joined<Selected>);
impl_order_by!(
    Selected,
    Filtered<Selected>,
    Joined<Selected>,
    Filtered<Joined<Selected>>
);
impl_limit!(
    Selected,
    Filtered<Selected>,
    Ordered,
    Joined<Selected>,
    Filtered<Joined<Selected>>
);
impl_returning!(Updated);
impl_get!(
    Selected,
    Filtered<Selected>,
    Ordered,
    Limited,
    Offsetted,
    Joined<Selected>,
    Filtered<Joined<Selected>>
);
impl_first!(
    Selected,
    Filtered<Selected>,
    Ordered,
    Joined<Selected>,
    Filtered<Joined<Selected>>
);
impl_first_or_fail!(
    Selected,
    Filtered<Selected>,
    Ordered,
    Joined<Selected>,
    Filtered<Joined<Selected>>
);
impl_execute!(Filtered<Updated>, Upserted);

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i32)>, sqlx::Error> = QueryBuilder::table("products")
            .select(&["id", "name", "price_cents"])
            // Call where on `Selected`, transitionning to `Filtered`
            .r#where("price_cents", ">=", 10)
            // Call where on `Filtered`, allowing chains
            .r#where("price_cents", ">=", 10)
            .r#where("price_cents", ">=", 10)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_null(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i32)>, sqlx::Error> = QueryBuilder::table("products")
            .select(&["id", "name", "price_cents"])
            // Call `where_null` on `Selected`, transitionning to `Filtered`
            .r#where_null("price_cents")
            // Call `where_null` on `Filtered`, allowing chains
            .r#where_null("price_cents")
            .r#where_null("price_cents")
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_not_null(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i32)>, sqlx::Error> = QueryBuilder::table("products")
            .select(&["id", "name", "price_cents"])
            // Call `where_not_null` on `Selected`, transitionning to `Filtered`
            .r#where_not_null("price_cents")
            // Call `where_not_null` on `Filtered`, allowing chains
            .r#where_not_null("price_cents")
            .r#where_not_null("price_cents")
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_order_by(connection: Pool<Postgres>) {
        // `order_by` can be called on `Selected`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .order_by("price_cents", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `order_by` can be called on `Filtered`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .where_not_null("price_cents")
                .order_by("price_cents", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_limit(connection: Pool<Postgres>) {
        // `limit` can be called on `Selected`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Filtered`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .where_not_null("price_cents")
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Ordered`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_offset(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i32)>, sqlx::Error> = QueryBuilder::table("products")
            .select(&["id", "name", "price_cents"])
            // Call `limit` on `Selected`, transitionning to `Limited`
            .limit(10)
            // Call `offset` on `Limited`, transitionning to `Offsetted`
            .offset(20)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get(connection: Pool<Postgres>) {
        // `get` can be called on `Initial`
        assert!(
            QueryBuilder::<Postgres, _>::table("products")
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Selected`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Filtered`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Ordered`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Limited`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Offsetted`;
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .offset(20)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first(connection: Pool<Postgres>) {
        // `first` can be called on `Initial`
        assert!(
            QueryBuilder::<Postgres, _>::table("products")
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Selected`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Filtered`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Ordered`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first_or_fail(connection: Pool<Postgres>) {
        // `first_or_fail` can be called on `Initial`
        assert!(
            QueryBuilder::<Postgres, _>::table("products")
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Selected`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Filtered`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Ordered`
        assert!(
            QueryBuilder::table("products")
                .select(&["id", "name", "price_cents"])
                .r#where("price_cents", ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert(connection: Pool<Postgres>) {
        // INSERT ... RETURNING (simple insert)
        let result: (Uuid, String, i32) = QueryBuilder::table("products")
            .insert()
            .set("id", Uuid::new_v4())
            .set("in_stock", true)
            .set("name", "Anvil")
            .set("price_cents", 100i32)
            .returning(&["id", "name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.1, "Anvil");

        // INSERT ... ON CONFLICT ... DO UPDATE
        let id = result.0;
        let result: (String, i32) = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Updated")
            .set("price_cents", 200i32)
            .on_conflict(&["id"])
            .do_update(&["name", "price_cents"])
            .returning(&["name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, "Updated");
        assert_eq!(result.1, 200);

        // INSERT ... ON CONFLICT ... DO NOTHING
        let result: Option<(Uuid,)> = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Ignored")
            .set("price_cents", 300i32)
            .on_conflict(&["id"])
            .do_nothing()
            .returning(&["id"])
            .first(&connection)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update(connection: Pool<Postgres>) {
        // Setup: insert a row
        let id = Uuid::new_v4();
        QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Original")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // UPDATE ... SET ... WHERE ... RETURNING
        let result: (String, i32) = QueryBuilder::table("products")
            .update()
            .set("name", "Updated")
            .set("price_cents", 200i32)
            .r#where("id", "=", id)
            .returning(&["name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, "Updated");
        assert_eq!(result.1, 200);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_returning(connection: Pool<Postgres>) {
        // Setup: insert a row
        let id = Uuid::new_v4();
        QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Original")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // `returning` can be called on `Updated`
        let result: (String,) = QueryBuilder::table("products")
            .update()
            .set("name", "Updated")
            .returning(&["name"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, "Updated");

        // `returning` can be called on `Filtered<Updated>`
        let result: (String,) = QueryBuilder::table("products")
            .update()
            .set("name", "Filtered Update")
            .r#where("id", "=", id)
            .returning(&["name"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, "Filtered Update");

        // `returning` can be called on `Inserted`
        let new_id = Uuid::new_v4();
        let result: (Uuid, String) = QueryBuilder::table("products")
            .insert()
            .set("id", new_id)
            .set("in_stock", true)
            .set("name", "New Anvil")
            .set("price_cents", 150i32)
            .returning(&["id", "name"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, new_id);
        assert_eq!(result.1, "New Anvil");

        // `returning` can be called on `Upserted` (DO UPDATE)
        let result: (String, i32) = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", false)
            .set("name", "Upserted Name")
            .set("price_cents", 200i32)
            .on_conflict(&["id"])
            .do_update(&["in_stock", "name", "price_cents"])
            .returning(&["name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.0, "Upserted Name");
        assert_eq!(result.1, 200);

        // `returning` can be called on `Upserted` (DO NOTHING)
        let result: Option<(Uuid,)> = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", false)
            .set("name", "Ignored")
            .set("price_cents", 999i32)
            .on_conflict(&["id"])
            .do_nothing()
            .returning(&["id"])
            .first(&connection)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_execute(connection: Pool<Postgres>) {
        // `execute` can be called on `Filtered<Updated>`
        let id = Uuid::new_v4();
        QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Original")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        let result = QueryBuilder::table("products")
            .update()
            .set("name", "Updated")
            .r#where("id", "=", id)
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Inserted`
        let result = QueryBuilder::table("products")
            .insert()
            .set("id", Uuid::new_v4())
            .set("in_stock", true)
            .set("name", "New Anvil")
            .set("price_cents", 150i32)
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Upserted` (DO UPDATE)
        let result = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", false)
            .set("name", "Upserted")
            .set("price_cents", 200i32)
            .on_conflict(&["id"])
            .do_update(&["in_stock", "name", "price_cents"])
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Upserted` (DO NOTHING)
        let result = QueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", false)
            .set("name", "Ignored")
            .set("price_cents", 999i32)
            .on_conflict(&["id"])
            .do_nothing()
            .execute(&connection)
            .await;
        assert!(result.is_ok());
    }
}
