use crate::database::Column;
use crate::model::Model;
use crate::sql::builder::{Builder as SqlBuilder, Initial};
use crate::sql::builder::{Filtered, Selected};
use crate::sql::operators::Operator;

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
        Self {
            inner: SqlBuilder::<M::Database, Initial>::table(M::table_name()).select(M::columns()),
        }
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
}

// Use macros to implement query execution methods across multiple states
impl_get!(Selected, Filtered);
impl_first!(Selected, Filtered);
impl_first_or_fail!(Selected, Filtered);
