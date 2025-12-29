use crate::database::{self, Column};
use crate::model::Model;
use crate::model::join::{Contains, Joined};
use crate::relation::{BelongsTo, Joinable};
use crate::sql::operators::{self, Direction};
use crate::sql::{
    Conflicted, Filtered, Initial, Inserted, Inserting, Joined as SqlJoined, Limited, Offsetted,
    Ordered, QueryBuilder as SqlQueryBuilder, Returned, Selected, Updated, Updating, Upserted,
};
use std::marker::PhantomData;

/// Implements the `join` and `join_through` methods for base states and their
/// joined variants.
///
/// For each base state, generates:
/// - `Base` → `Joined<Base>` (initial join)
/// - `Joined<Base>` → `Joined<Base>` (chained joins)
///
/// - `join::<J>()`: Bidirectional join via `Joinable<M>` trait
/// - `join_through::<Pivot, Target>()`: Many-to-many join via pivot table
macro_rules! impl_join {
    // Shared body - takes input state and output base
    (@body $state:ty, $base:ty) => {
        impl<M, Joins> QueryBuilder<M, $state, Joins>
        where
            M: Model,
            M::Database: sqlx::Database,
        {
            /// Adds an INNER JOIN clause to the query.
            ///
            /// The ON clause is automatically inferred from the `Joinable<M>` trait.
            /// Works bidirectionally - both parent→child and child→parent joins.
            ///
            /// Transitions to [`Joined`] state.
            pub fn join<J>(self) -> QueryBuilder<M, SqlJoined<$base>, Joined<J, Joins>>
            where
                J: Model<Database = M::Database> + Joinable<M>,
            {
                QueryBuilder {
                    inner: self.inner.join(
                        J::table_name(),
                        <J as Joinable<M>>::left_column().qualified_name(),
                        <J as Joinable<M>>::right_column().qualified_name(),
                    ),
                    _joins: PhantomData,
                }
            }

            /// Adds a many-to-many JOIN through a pivot table.
            ///
            /// Joins `Pivot` to `M`, then `Target` to `Pivot`.
            ///
            /// Transitions to [`Joined`] state.
            pub fn join_through<Pivot, Target>(self) -> QueryBuilder<M, SqlJoined<$base>, Joined<Target, Joins>>
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
                        ),
                    _joins: PhantomData,
                }
            }
        }
    };
    // Entry: for each base, generate both impls
    ($($base:ty),+ $(,)?) => {
        $(
            impl_join!(@body $base, $base);
            impl_join!(@body SqlJoined<$base>, $base);
        )+
    };
}

/// Implements the `where` method for states that start a WHERE clause.
///
/// Supports two syntaxes:
/// - `impl_where!(State)` - transitions to `Filtered<State>`
/// - `impl_where!(InputState => OutputState)` - transitions to
///   `Filtered<OutputState>`
macro_rules! impl_where {
    // Handle input => output syntax
    (@impl $input:ty => $output:ty) => {
        impl<M, Joins> QueryBuilder<M, $input, Joins>
        where
            M: Model,
            M::Database: sqlx::Database,
        {
            /// Adds a WHERE clause to the query.
            ///
            /// Requires a type-safe column from the model to ensure compile-time safety.
            /// Transitions to [`Filtered`] state.
            pub fn r#where<Column, Operator, JoinedModel, Index>(
                self,
                column: Column,
                operator: Operator,
                value: Column::Type
            ) -> QueryBuilder<M, Filtered<$output>, Joins>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Index>,
                for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                Operator: Into<operators::Operator>,
            {
                QueryBuilder {
                    inner: self.inner.r#where(column.qualified_name(), operator, value),
                    _joins: self._joins,
                }
            }

            /// Adds a WHERE IS NULL clause to the query.
            ///
            /// Transitions to [`Filtered`] state.
            pub fn where_null<Column, JoinedModel, Index>(
                self,
                column: Column
            ) -> QueryBuilder<M, Filtered<$output>, Joins>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Index>,
                for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
            {
                QueryBuilder {
                    inner: self.inner.where_null(column.qualified_name()),
                    _joins: self._joins,
                }
            }

            /// Adds a WHERE IS NOT NULL clause to the query.
            ///
            /// Transitions to [`Filtered`] state.
            pub fn where_not_null<Column, JoinedModel, Index>(
                self,
                column: Column
            ) -> QueryBuilder<M, Filtered<$output>, Joins>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Index>,
                for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
            {
                QueryBuilder {
                    inner: self.inner.where_not_null(column.qualified_name()),
                    _joins: self._joins,
                }
            }
        }
    };
    // Entry point: simple state (input = output)
    ($state:ty) => {
        impl_where!(@impl $state => $state);
    };
    // Entry point: explicit input => output
    ($input:ty => $output:ty) => {
        impl_where!(@impl $input => $output);
    };
    // Entry point: multiple states
    ($($state:ty),+ $(,)?) => {
        $(
            impl_where!($state);
        )+
    };
}

/// Implements chained `where` methods for already-filtered states.
///
/// Returns the same `Filtered<$state>` type (no nesting).
/// Uses `Contains` trait for compile-time validation of joined models.
macro_rules! impl_chain_where {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M, Joins> QueryBuilder<M, Filtered<$state>, Joins>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds an additional WHERE clause (AND) to the query.
                ///
                /// The column must belong to a model that is joined in this query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn r#where<Column, Operator, JoinedModel, Index>(
                    self,
                    column: Column,
                    operator: Operator,
                    value: Column::Type,
                ) -> QueryBuilder<M, Filtered<$state>, Joins>
                where
                    Column: database::Column<JoinedModel>,
                    Joins: Contains<JoinedModel, Index>,
                    for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                    Operator: Into<operators::Operator>,
                {
                    QueryBuilder {
                        inner: self.inner.r#where(column.qualified_name(), operator, value),
                        _joins: self._joins,
                    }
                }

                /// Adds an additional WHERE IS NULL clause to the query.
                ///
                /// The column must belong to a model that is joined in this query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn where_null<Column, JoinedModel, Index>(
                    self,
                    column: Column,
                ) -> QueryBuilder<M, Filtered<$state>, Joins>
                where
                    Column: database::Column<JoinedModel>,
                    Joins: Contains<JoinedModel, Index>,
                    for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_null(column.qualified_name()),
                        _joins: self._joins,
                    }
                }

                /// Adds an additional WHERE IS NOT NULL clause to the query.
                ///
                /// The column must belong to a model that is joined in this query.
                ///
                /// Remains in [`Filtered`] state.
                pub fn where_not_null<Column, JoinedModel, Index>(
                    self,
                    column: Column,
                ) -> QueryBuilder<M, Filtered<$state>, Joins>
                where
                    Column: database::Column<JoinedModel>,
                    Joins: Contains<JoinedModel, Index>,
                    for<'q> Column::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.where_not_null(column.qualified_name()),
                        _joins: self._joins,
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
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
                ) -> QueryBuilder<M, Ordered, Joins> {
                    QueryBuilder {
                        inner: self.inner.order_by(column, direction),
                        _joins: self._joins,
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
                pub fn limit<'a>(self, count: i64) -> QueryBuilder<M, Limited, Joins>
                where
                    i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
                {
                    QueryBuilder {
                        inner: self.inner.limit(count),
                        _joins: self._joins,
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
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
            impl<M, Joins> QueryBuilder<M, $state, Joins>
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

pub struct QueryBuilder<M, S = Initial, Joins = Joined<M, ()>>
where
    M: Model,
    M::Database: sqlx::Database,
{
    inner: SqlQueryBuilder<M::Database, S>,
    _joins: PhantomData<Joins>,
}

impl<M> Default for QueryBuilder<M, Initial>
where
    M: Model,
    M::Database: sqlx::Database,
{
    fn default() -> Self {
        let inner = SqlQueryBuilder::<M::Database, Initial>::table(M::table_name());
        Self {
            inner,
            _joins: PhantomData,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Initial, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Starts a SELECT query for this model.
    ///
    /// Transitions to [`Selected`] state.
    pub fn select(self) -> QueryBuilder<M, Selected, Joins> {
        QueryBuilder {
            inner: self.inner.select(M::qualified_columns()),
            _joins: self._joins,
        }
    }

    /// Starts an UPDATE query for this model.
    ///
    /// Transitions to [`Updating`] state.
    pub fn update(self) -> QueryBuilder<M, Updating, Joins> {
        QueryBuilder {
            inner: self.inner.update(),
            _joins: self._joins,
        }
    }

    /// Starts an INSERT query for this model.
    ///
    /// Transitions to [`Inserting`] state.
    pub fn insert(self) -> QueryBuilder<M, Inserting, Joins> {
        QueryBuilder {
            inner: self.inner.insert(),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Limited, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Remains in [`Limited`] state, allowing execution.
    pub fn offset<'a>(self, count: i64) -> QueryBuilder<M, Offsetted, Joins>
    where
        i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.offset(count),
            _joins: self._joins,
        }
    }
}

/// Implements the `returning` method for multiple builder states.
///
/// Always transitions to [`Returned`] state.
macro_rules! impl_returning {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M, Joins> QueryBuilder<M, $state, Joins>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Specifies the columns to return after the statement.
                ///
                /// Generates `RETURNING col1, col2, ...`.
                /// Transitions to [`Returned`] state.
                pub fn returning(self, columns: &[&str]) -> QueryBuilder<M, Returned, Joins> {
                    QueryBuilder {
                        inner: self.inner.returning(columns),
                        _joins: self._joins,
                    }
                }
            }
        )+
    };
}

// Use macros to implement query execution methods across multiple states
impl_join!(Selected);
impl_where!(Selected, Updated);
impl_where!(SqlJoined<Selected> => Selected);
impl_where!(SqlJoined<Updated> => Updated);
impl_chain_where!(Selected, Updated);
impl_order_by!(
    Selected,
    Filtered<Selected>,
    SqlJoined<Selected>,
    Filtered<SqlJoined<Selected>>
);
impl_limit!(
    Selected,
    Filtered<Selected>,
    Ordered,
    SqlJoined<Selected>,
    Filtered<SqlJoined<Selected>>
);
impl_get!(
    Selected,
    Filtered<Selected>,
    Ordered,
    Limited,
    Offsetted,
    SqlJoined<Selected>,
    Filtered<SqlJoined<Selected>>
);
impl_first!(
    Selected,
    Filtered<Selected>,
    Ordered,
    SqlJoined<Selected>,
    Filtered<SqlJoined<Selected>>
);
impl_first_or_fail!(
    Selected,
    Filtered<Selected>,
    Ordered,
    SqlJoined<Selected>,
    Filtered<SqlJoined<Selected>>
);
impl_returning!(Updated, Filtered<Updated>);
impl_execute!(Filtered<Updated>, Inserted<M::Database>, Upserted);

impl<M, Joins> QueryBuilder<M, Updating, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets a column value for the UPDATE statement.
    ///
    /// Transitions to [`Updated`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Updated, Joins>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Updated, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets an additional column value for the UPDATE statement.
    ///
    /// Remains in [`Updated`] state.
    pub fn set<'a, C>(self, column: C, value: C::Type) -> QueryBuilder<M, Updated, Joins>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Inserting, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets a column value for the INSERT statement.
    ///
    /// Transitions to [`Inserted`] state.
    pub fn set<'a, C>(
        self,
        column: C,
        value: C::Type,
    ) -> QueryBuilder<M, Inserted<M::Database>, Joins>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database> + Send + 'static,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Inserted<M::Database>, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Sets an additional column value for the INSERT statement.
    ///
    /// Remains in [`Inserted`] state.
    pub fn set<'a, C>(
        self,
        column: C,
        value: C::Type,
    ) -> QueryBuilder<M, Inserted<M::Database>, Joins>
    where
        C: Column<M>,
        C::Type: 'a + sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database> + Send + 'static,
    {
        QueryBuilder {
            inner: self.inner.set(column.name(), value),
            _joins: self._joins,
        }
    }

    /// Specifies the conflict target for an UPSERT operation.
    ///
    /// Uses the model's primary key columns as the conflict target.
    /// Transitions to [`Conflicted`] state.
    pub fn on_conflict(self) -> QueryBuilder<M, Conflicted, Joins> {
        QueryBuilder {
            inner: self.inner.on_conflict(M::primary_key_columns()),
            _joins: self._joins,
        }
    }

    /// Specifies the columns to return after the INSERT.
    ///
    /// Uses all model columns. Transitions to [`Returned`] state.
    pub fn returning(self) -> QueryBuilder<M, Returned, Joins> {
        QueryBuilder {
            inner: self.inner.returning(M::columns()),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Conflicted, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Specifies that conflicting rows should be updated.
    ///
    /// Updates all non-primary-key columns with `col = EXCLUDED.col`.
    /// Transitions to [`Upserted`] state.
    pub fn do_update(self) -> QueryBuilder<M, Upserted, Joins> {
        let pk_columns = M::primary_key_columns();
        let update_columns: Vec<&str> = M::columns()
            .iter()
            .copied()
            .filter(|c| !pk_columns.contains(c))
            .collect();

        QueryBuilder {
            inner: self.inner.do_update(&update_columns),
            _joins: self._joins,
        }
    }

    /// Specifies that conflicting rows should be ignored.
    ///
    /// Generates `DO NOTHING`. Transitions to [`Upserted`] state.
    pub fn do_nothing(self) -> QueryBuilder<M, Upserted, Joins> {
        QueryBuilder {
            inner: self.inner.do_nothing(),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Upserted, Joins>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Specifies the columns to return after the UPSERT.
    ///
    /// Uses all model columns. Transitions to [`Returned`] state.
    pub fn returning(self) -> QueryBuilder<M, Returned, Joins> {
        QueryBuilder {
            inner: self.inner.returning(M::columns()),
            _joins: self._joins,
        }
    }
}

impl<M, Joins> QueryBuilder<M, Returned, Joins>
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
