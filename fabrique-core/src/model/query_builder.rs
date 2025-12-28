use crate::database::Column;
use crate::model::Model;
use crate::relation::BelongsTo;
use crate::sql::operators::{Direction, Operator};
use crate::sql::{
    Conflicted, Filtered, Initial, Inserted, Inserting, Joined, Limited, Offsetted, Ordered,
    QueryBuilder as SqlQueryBuilder, Returned, Selected, Updated, Updating, Upserted,
};

/// Implements the `join` and `join_through` methods for base states and their
/// joined variants.
///
/// For each base state, generates:
/// - `Base` → `Joined<Base>` (initial join)
/// - `Joined<Base>` → `Joined<Base>` (chained joins)
///
/// - `join::<J>()`: Simple join, ON clause links J to M via `BelongsTo<M>`
/// - `join_through::<Pivot, Target>()`: Many-to-many join via pivot table
macro_rules! impl_join {
    // Shared body - takes input state and output base
    (@body $state:ty, $base:ty) => {
        impl<M> QueryBuilder<M, $state>
        where
            M: Model,
            M::Database: sqlx::Database,
        {
            /// Adds an INNER JOIN clause to the query.
            ///
            /// The ON clause is automatically inferred from the `BelongsTo<M>` trait
            /// implementation on the join model `J`.
            ///
            /// Transitions to [`Joined`] state.
            pub fn join<J>(self) -> QueryBuilder<M, Joined<$base>>
            where
                J: Model<Database = M::Database> + BelongsTo<M>,
            {
                QueryBuilder {
                    inner: self.inner.join(
                        J::table_name(),
                        J::foreign_key_column().qualified_name(),
                        &format!("{}.{}", M::table_name(), M::primary_key_columns()[0]),
                    ),
                }
            }

            /// Adds a many-to-many JOIN through a pivot table.
            ///
            /// Joins `Pivot` to `M`, then `Target` to `Pivot`.
            ///
            /// Transitions to [`Joined`] state.
            pub fn join_through<Pivot, Target>(self) -> QueryBuilder<M, Joined<$base>>
            where
                Pivot: Model<Database = M::Database>,
                Pivot: BelongsTo<M>,
                Pivot: BelongsTo<Target>,
                Target: Model<Database = M::Database>,
            {
                QueryBuilder {
                    inner: self.inner
                        .join(
                            Pivot::table_name(),
                            <Pivot as BelongsTo<M>>::foreign_key_column().qualified_name(),
                            &format!("{}.{}", M::table_name(), M::primary_key_columns()[0]),
                        )
                        .join(
                            Target::table_name(),
                            <Pivot as BelongsTo<Target>>::foreign_key_column().qualified_name(),
                            &format!("{}.{}", Target::table_name(), Target::primary_key_columns()[0]),
                        )
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
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds a WHERE clause to the query.
                ///
                /// Requires a type-safe column from the model to ensure compile-time safety.
                /// Transitions to [`Filtered`] state.
                pub fn r#where<C, O>(self, column: C, operator: O, value: C::Type) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                    O: Into<Operator>,
                {
                    QueryBuilder {
                        inner: self.inner.r#where(column.name(), operator, value),
                    }
                }

                /// Adds a WHERE IS NULL clause to the query.
                ///
                /// Transitions to [`Filtered`] state.
                pub fn where_null<C>(self, column: C) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_null(column.name()),
                    }
                }

                /// Adds a WHERE IS NOT NULL clause to the query.
                ///
                /// Transitions to [`Filtered`] state.
                pub fn where_not_null<C>(self, column: C) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_null(column.name()),
                    }
                }
            }
        )+
    };
}

/// Implements chained `where` methods for already-filtered states.
///
/// Returns the same `Filtered<$state>` type (no nesting).
macro_rules! impl_chain_where {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> QueryBuilder<M, Filtered<$state>>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds an additional WHERE clause (AND) to the query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn r#where<C, O>(self, column: C, operator: O, value: C::Type) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                    O: Into<Operator>,
                {
                    QueryBuilder {
                        inner: self.inner.r#where(column.name(), operator, value),
                    }
                }

                /// Adds an additional WHERE IS NULL clause to the query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn where_null<C>(self, column: C) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_null(column.name()),
                    }
                }

                /// Adds an additional WHERE IS NOT NULL clause to the query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn where_not_null<C>(self, column: C) -> QueryBuilder<M, Filtered<$state>>
                where
                    C: Column<M>,
                    for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_not_null(column.name()),
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
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds an `ORDER BY` clause to the query.
                ///
                /// Transitions to [`Ordered`] state, allowing `LIMIT` or execution.
                pub fn order_by(
                    self,
                    column: &str,
                    direction: impl Into<Direction>,
                ) -> QueryBuilder<M, Ordered> {
                    QueryBuilder {
                        inner: self.inner.order_by(column, direction),
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
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
                pub fn limit<'a>(self, count: i64) -> QueryBuilder<M, Limited>
                where
                    i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.limit(count),
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
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
                M::Error: From<sqlx::Error>,
            {
                /// Executes the query and returns all matching rows.
                pub async fn get<'e, E>(self, executor: E) -> Result<Vec<M>, M::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.get(executor).await.map_err(Into::into)
                }
            }
        )+
    };
}

/// Implements the `first` method for multiple builder states.
macro_rules! impl_first {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
                M::Error: From<sqlx::Error>,
            {
                /// Retrieves the first row from the query result.
                ///
                /// Returns `None` if no rows match the query.
                pub async fn first<'e, E>(self, executor: E) -> Result<Option<M>, M::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.first(executor).await.map_err(Into::into)
                }
            }
        )+
    };
}

/// Implements the `first_or_fail` method for multiple builder states.
macro_rules! impl_first_or_fail {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
                M::Error: From<sqlx::Error>,
            {
                /// Retrieves the first row from the query result, or fails if none exists.
                ///
                /// Returns an error if no rows match the query.
                pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<M, M::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.first_or_fail(executor).await.map_err(Into::into)
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
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Executes the statement without returning any rows.
                pub async fn execute<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.execute(executor).await
                }
            }
        )+
    };
}

pub struct QueryBuilder<M, S = Initial>
where
    M: Model,
    M::Database: sqlx::Database,
{
    inner: SqlQueryBuilder<M::Database, S>,
}

impl<M> Default for QueryBuilder<M, Initial>
where
    M: Model,
    M::Database: sqlx::Database,
{
    fn default() -> Self {
        let inner = SqlQueryBuilder::<M::Database, Initial>::table(M::table_name());
        Self { inner }
    }
}

impl<M> QueryBuilder<M, Initial>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Starts a SELECT query for this model.
    ///
    /// Transitions to [`Selected`] state.
    pub fn select(self) -> QueryBuilder<M, Selected> {
        QueryBuilder {
            inner: self.inner.select(M::qualified_columns()),
        }
    }

    /// Starts an UPDATE query for this model.
    ///
    /// Transitions to [`Updating`] state.
    pub fn update(self) -> QueryBuilder<M, Updating> {
        QueryBuilder {
            inner: self.inner.update(),
        }
    }

    /// Starts an INSERT query for this model.
    ///
    /// Transitions to [`Inserting`] state.
    pub fn insert(self) -> QueryBuilder<M, Inserting> {
        QueryBuilder {
            inner: self.inner.insert(),
        }
    }
}

impl<M> QueryBuilder<M, Limited>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Remains in [`Limited`] state, allowing execution.
    pub fn offset<'a>(self, count: i64) -> QueryBuilder<M, Offsetted>
    where
        i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.offset(count),
        }
    }
}

/// Implements the `returning` method for multiple builder states.
///
/// Always transitions to [`Returned`] state.
macro_rules! impl_returning {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> QueryBuilder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Specifies the columns to return after the statement.
                ///
                /// Generates `RETURNING col1, col2, ...`.
                /// Transitions to [`Returned`] state.
                pub fn returning(self, columns: &[&str]) -> QueryBuilder<M, Returned> {
                    QueryBuilder {
                        inner: self.inner.returning(columns),
                    }
                }
            }
        )+
    };
}

// Use macros to implement query execution methods across multiple states
impl_join!(Selected);
impl_where!(Selected, Updated, Joined<Selected>);
impl_chain_where!(Selected, Updated);
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
impl_returning!(Updated, Filtered<Updated>);
impl_execute!(Filtered<Updated>, Inserted<M::Database>, Upserted);

impl<M> QueryBuilder<M, Updating>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets a column value for the UPDATE statement.
    ///
    /// Transitions to [`Updated`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Updated>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
        }
    }
}

impl<M> QueryBuilder<M, Updated>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets an additional column value for the UPDATE statement.
    ///
    /// Remains in [`Updated`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Updated>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
        }
    }
}

impl<M> QueryBuilder<M, Inserting>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets a column value for the INSERT statement.
    ///
    /// Transitions to [`Inserted`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Inserted<M::Database>>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database> + Send + 'static,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
        }
    }
}

impl<M> QueryBuilder<M, Inserted<M::Database>>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets an additional column value for the INSERT statement.
    ///
    /// Remains in [`Inserted`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Inserted<M::Database>>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database> + Send + 'static,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
        }
    }

    /// Specifies the conflict target for an UPSERT operation.
    ///
    /// Uses the model's primary key columns as the conflict target.
    /// Transitions to [`Conflicted`] state.
    pub fn on_conflict(self) -> QueryBuilder<M, Conflicted> {
        QueryBuilder {
            inner: self.inner.on_conflict(M::primary_key_columns()),
        }
    }

    /// Specifies the columns to return after the INSERT.
    ///
    /// Uses all model columns. Transitions to [`Returned`] state.
    pub fn returning(self) -> QueryBuilder<M, Returned> {
        QueryBuilder {
            inner: self.inner.returning(M::columns()),
        }
    }
}

impl<M> QueryBuilder<M, Conflicted>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Specifies that conflicting rows should be updated.
    ///
    /// Updates all non-primary-key columns with `col = EXCLUDED.col`.
    /// Transitions to [`Upserted`] state.
    pub fn do_update(self) -> QueryBuilder<M, Upserted> {
        let pk_columns = M::primary_key_columns();
        let update_columns: Vec<&str> = M::columns()
            .iter()
            .copied()
            .filter(|c| !pk_columns.contains(c))
            .collect();

        QueryBuilder {
            inner: self.inner.do_update(&update_columns),
        }
    }

    /// Specifies that conflicting rows should be ignored.
    ///
    /// Generates `DO NOTHING`. Transitions to [`Upserted`] state.
    pub fn do_nothing(self) -> QueryBuilder<M, Upserted> {
        QueryBuilder {
            inner: self.inner.do_nothing(),
        }
    }
}

impl<M> QueryBuilder<M, Upserted>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Specifies the columns to return after the UPSERT.
    ///
    /// Uses all model columns. Transitions to [`Returned`] state.
    pub fn returning(self) -> QueryBuilder<M, Returned> {
        QueryBuilder {
            inner: self.inner.returning(M::columns()),
        }
    }
}

impl<M> QueryBuilder<M, Returned>
where
    M: Model,
    M::Database: sqlx::Database,
    M::Error: From<sqlx::Error>,
{
    /// Executes the query and returns all resulting rows.
    pub async fn get<'e, E>(self, executor: E) -> Result<Vec<M>, M::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.get(executor).await.map_err(Into::into)
    }

    /// Executes the query and returns the first resulting row.
    pub async fn first<'e, E>(self, executor: E) -> Result<Option<M>, M::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.first(executor).await.map_err(Into::into)
    }

    /// Executes the query and returns the first resulting row, or fails.
    pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<M, M::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.first_or_fail(executor).await.map_err(Into::into)
    }
}
