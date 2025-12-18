use crate::sql::operators::{Direction, Operator};
use sqlx::{Database, Executor, FromRow, IntoArguments};
use std::marker::PhantomData;

/// Implements the `order_by` method for multiple builder states.
///
/// Always transitions to [`Ordered`] state.
macro_rules! impl_order_by {
    ($($state:ty),+ $(,)?) => {
        $(
            impl< DB: Database> Builder<DB, $state> {
                /// Adds an `ORDER BY` clause to the query.
                ///
                /// Transitions to [`Ordered`] state, allowing `LIMIT` or execution.
                pub fn order_by(
                    mut self,
                    column: &str,
                    direction: impl Into<Direction>,
                ) -> Builder< DB, Ordered> {
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
            impl<DB: Database> Builder< DB, $state> {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
                pub fn limit<'a>(mut self, count: i64) -> Builder< DB, Limited>
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

/// Implements the `get` method for multiple builder states.
macro_rules! impl_get {
    ($($state:ty),+ $(,)?) => {
        $(
            impl< DB: Database> Builder< DB, $state> {
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
            impl<DB: Database> Builder<DB, $state> {
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
            impl<DB: Database> Builder<DB, $state> {
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
/// # State Transition Matrix
///
/// The table shows which methods are available on each builder state.
/// Checkmarks indicate available methods. Methods transition to new states
/// (shown in return types), and these transitions cannot be reversed. Optional
/// clauses can be skipped by calling methods that jump to later states.
///
/// ```text
/// State      │ select │ where │ order_by │ limit │ offset │ get │ first
/// ───────────┼────────┼───────┼──────────┼───────┼────────┼─────┼──────
/// Initial    │   ✓    │       │          │       │        │  ✓  │   ✓
/// Selected   │        │   ✓   │    ✓     │   ✓   │        │  ✓  │   ✓
/// Filtered   │        │   ✓   │    ✓     │   ✓   │        │  ✓  │   ✓
/// Ordered    │        │       │          │   ✓   │        │  ✓  │   ✓
/// Limited    │        │       │          │       │   ✓    │  ✓  │
/// Offsetted  │        │       │          │       │        │  ✓  │
/// ```
pub struct Builder<DB: Database, S = Initial> {
    inner: sqlx::QueryBuilder<DB>,
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

/// 0ffsetted state - OFFSET added, can execute.
pub struct Offsetted;

impl<DB: Database> Builder<DB, Initial> {
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
    pub fn select(mut self, columns: &[&str]) -> Builder<DB, Selected> {
        let columns = columns.join(", ");
        let query = format!("SELECT {} FROM {}", columns, &self.table);
        self.inner.push(query);

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
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
}

impl<DB: Database> Builder<DB, Selected> {
    /// Adds a WHERE clause to the query.
    ///
    /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
    /// AND conditions.
    pub fn r#where<'a, T, O>(mut self, column: &str, operator: O, value: T) -> Builder<DB, Filtered>
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

    /// Adds a WHERE NULL clause to the query.
    ///
    /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
    /// AND conditions.
    pub fn where_null(mut self, column: &str) -> Builder<DB, Filtered> {
        self.inner.push(" WHERE ");
        self.inner.push(column);
        self.inner.push(" IS NULL");

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }

    /// Adds a WHERE NOT NULL clause to the query.
    ///
    /// Transitions to [`Filtered`] state. Use additional `where()` calls to add
    /// AND conditions.
    pub fn where_not_null(mut self, column: &str) -> Builder<DB, Filtered> {
        self.inner.push(" WHERE ");
        self.inner.push(column);
        self.inner.push(" IS NOT NULL");

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

impl<DB: Database> Builder<DB, Filtered> {
    /// Adds an additional WHERE clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// allowing additional conditions, ORDER BY, LIMIT, or execution.
    pub fn r#where<'a, T, O>(mut self, column: &str, operator: O, value: T) -> Builder<DB, Filtered>
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

    /// Adds an additional WHERE NULL clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// allowing additional conditions, ORDER BY, LIMIT, or execution.
    pub fn where_null(mut self, column: &str) -> Builder<DB, Filtered> {
        self.inner.push(" AND ");
        self.inner.push(column);
        self.inner.push(" IS NULL");

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }

    /// Adds an additional WHERE NOT NULL clause using AND.
    ///
    /// Chains multiple conditions together. Remains in the [`Filtered`] state,
    /// allowing additional conditions, ORDER BY, LIMIT, or execution.
    pub fn where_not_null(mut self, column: &str) -> Builder<DB, Filtered> {
        self.inner.push(" AND ");
        self.inner.push(column);
        self.inner.push(" IS NOT NULL");

        Builder {
            inner: self.inner,
            table: self.table,
            _state: PhantomData,
        }
    }
}

impl<DB: Database> Builder<DB, Ordered> {}

impl<DB: Database> Builder<DB, Limited> {
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Remains in [`Limited`] state, allowing execution.
    pub fn offset<'a>(mut self, count: i64) -> Builder<DB, Offsetted>
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
impl_get!(Selected, Filtered, Ordered, Limited, Offsetted);
impl_first!(Selected, Filtered, Ordered);
impl_first_or_fail!(Selected, Filtered, Ordered);

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i16)>, sqlx::Error> = Builder::table("anvils")
            .select(&["id", "name", "weight"])
            // Call where on `Selected`, transitionning to `Filtered`
            .r#where("weight", ">=", 10)
            // Call where on `Filtered`, allowing chains
            .r#where("weight", ">=", 10)
            .r#where("weight", ">=", 10)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_null(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i16)>, sqlx::Error> = Builder::table("anvils")
            .select(&["id", "name", "weight"])
            // Call `where_null` on `Selected`, transitionning to `Filtered`
            .r#where_null("weight")
            // Call `where_null` on `Filtered`, allowing chains
            .r#where_null("weight")
            .r#where_null("weight")
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_not_null(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i16)>, sqlx::Error> = Builder::table("anvils")
            .select(&["id", "name", "weight"])
            // Call `where_not_null` on `Selected`, transitionning to `Filtered`
            .r#where_not_null("weight")
            // Call `where_null` on `Filtered`, allowing chains
            .r#where_not_null("weight")
            .r#where_not_null("weight")
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
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .order_by("weight", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `order_by` can be called on `Filtered`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .where_not_null("weight")
                .order_by("weight", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_limit(connection: Pool<Postgres>) {
        // `limit` can be called on `Selected`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Filtered`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .where_not_null("weight")
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Ordered`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_offset(connection: Pool<Postgres>) {
        let result: Result<Vec<(Uuid, String, i16)>, sqlx::Error> = Builder::table("anvils")
            .select(&["id", "name", "weight"])
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
            Builder::<Postgres, _>::table("anvils")
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Selected`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Filtered`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Ordered`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .order_by("weight", Direction::Asc)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Limited`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Offsetted`;
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .order_by("weight", Direction::Asc)
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
            Builder::<Postgres, _>::table("anvils")
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Selected`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Filtered`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Ordered`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .order_by("weight", Direction::Asc)
                .first::<(), _>(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first_or_fail(connection: Pool<Postgres>) {
        // `first_or_fail` can be called on `Initial`
        assert!(
            Builder::<Postgres, _>::table("anvils")
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Selected`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Filtered`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Ordered`
        assert!(
            Builder::table("anvils")
                .select(&["id", "name", "weight"])
                .r#where("weight", ">=", 0)
                .order_by("weight", Direction::Asc)
                .first_or_fail::<(), _>(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );
    }
}
