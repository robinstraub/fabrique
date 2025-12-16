use crate::database::Column;
use crate::model::Model;
use crate::sql::builder::{Builder as SqlBuilder, Initial};
use crate::sql::builder::{Filtered, Selected};
use crate::sql::operators::Operator;

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

    pub async fn fetch_all<'s, E>(self, executor: E) -> Result<Vec<M>, sqlx::Error>
    where
        E: for<'e> sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.fetch_all(executor).await
    }
}

impl<M> Builder<M, Filtered>
where
    M: Model,
    M::Database: sqlx::Database,
{
    pub async fn fetch_all<'s, E>(self, executor: E) -> Result<Vec<M>, sqlx::Error>
    where
        E: for<'e> sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.fetch_all(executor).await
    }
}
