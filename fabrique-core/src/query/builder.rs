use crate::database::Column;
use crate::model::Model;
use crate::sql::builder::{Builder as SqlBuilder, Initial};
use crate::sql::builder::{Filtered, Limited, Ordered, Selected};
use crate::sql::operators::{Direction, Operator};

/// Implements the `order_by` method for multiple builder states.
///
/// Always transitions to [`Ordered`] state.
macro_rules! impl_order_by {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> Builder<M, $state>
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
                ) -> Builder<M, Ordered> {
                    Builder {
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
            impl<M> Builder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Adds a `LIMIT` clause to the query.
                ///
                /// Transitions to [`Limited`] state, allowing `OFFSET` or execution.
                pub fn limit<'a>(self, count: i64) -> Builder<M, Limited>
                where
                    i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
                {
                    Builder {
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
            impl<M> Builder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Executes the query and returns all matching rows.
                pub async fn get<'e, E>(self, executor: E) -> Result<Vec<M>, sqlx::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.get(executor).await
                }
            }
        )+
    };
}

/// Implements the `first` method for multiple builder states.
macro_rules! impl_first {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> Builder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Retrieves the first row from the query result.
                ///
                /// Returns `None` if no rows match the query.
                pub async fn first<'e, E>(self, executor: E) -> Result<Option<M>, sqlx::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.first(executor).await
                }
            }
        )+
    };
}

/// Implements the `first_or_fail` method for multiple builder states.
macro_rules! impl_first_or_fail {
    ($($state:ty),+ $(,)?) => {
        $(
            impl<M> Builder<M, $state>
            where
                M: Model,
                M::Database: sqlx::Database,
            {
                /// Retrieves the first row from the query result, or fails if none exists.
                ///
                /// Returns an error if no rows match the query.
                pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<M, sqlx::Error>
                where
                    E: sqlx::Executor<'e, Database = M::Database>,
                    M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
                    <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
                {
                    self.inner.first_or_fail(executor).await
                }
            }
        )+
    };
}

pub struct Builder<M, S = Selected>
where
    M: Model,
    M::Database: sqlx::Database,
{
    inner: SqlBuilder<M::Database, S>,
}

impl<M> Default for Builder<M, Selected>
where
    M: Model,
    M::Database: sqlx::Database,
{
    fn default() -> Self {
        let inner = SqlBuilder::<M::Database, Initial>::table(M::table_name()).select(M::columns());
        Self { inner }
    }
}

impl<M> Builder<M, Selected>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Adds a WHERE clause to the query.
    ///
    /// Requires a type-safe column from the model (e.g., `Anvil::WEIGHT`) to
    /// ensure compile-time safety. The value type must match the column's type.
    ///
    /// Transitions to `Filtered` state.
    pub fn r#where<C, O>(self, column: C, operator: O, value: C::Type) -> Builder<M, Filtered>
    where
        C: Column<M>,
        for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
        O: Into<Operator>,
    {
        Builder {
            inner: self.inner.r#where(column.name(), operator, value),
        }
    }

    /// Adds a WHERE IS NULL clause to the query.
    ///
    /// Transitions to `Filtered` state.
    pub fn r#where_null<C>(self, column: C) -> Builder<M, Filtered>
    where
        C: Column<M>,
        for<'q> C::Type: sqlx::Encode<'q, M::Database> + sqlx::Type<M::Database>,
    {
        Builder {
            inner: self.inner.where_null(column.name()),
        }
    }
}

impl<M> Builder<M, Limited>
where
    M: Model,
    M::Database: sqlx::Database,
{
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Remains in [`Limited`] state, allowing execution.
    pub fn offset<'a>(self, count: i64) -> Builder<M, Limited>
    where
        i64: sqlx::Encode<'a, M::Database> + sqlx::Type<M::Database>,
    {
        Builder {
            inner: self.inner.offset(count),
        }
    }
}

// Use macros to implement query execution methods across multiple states
impl_order_by!(Selected, Filtered);
impl_limit!(Selected, Filtered, Ordered);
impl_get!(Selected, Filtered, Ordered, Limited);
impl_first!(Selected, Filtered, Ordered, Limited);
impl_first_or_fail!(Selected, Filtered, Ordered, Limited);
