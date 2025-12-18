use crate::database::Column;
use crate::model::Model;
use crate::sql::builder::{Builder as SqlBuilder, Initial, Offsetted};
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
    pub fn offset<'a>(self, count: i64) -> Builder<M, Offsetted>
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
impl_get!(Selected, Filtered, Ordered, Limited, Offsetted);
impl_first!(Selected, Filtered, Ordered);
impl_first_or_fail!(Selected, Filtered, Ordered);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{DatabaseAware, Nil};
    use sqlx::{FromRow, Pool, Postgres};
    use uuid::Uuid;

    /// Test model implementing required traits for query builder tests.
    #[derive(Debug, Default, PartialEq, FromRow)]
    struct Anvil {
        id: Uuid,
        name: String,
        weight: i16,
    }

    impl DatabaseAware for Anvil {
        type Database = Postgres;
        type Error = sqlx::Error;
    }

    impl Model for Anvil {
        type PrimaryKey = Uuid;
        type SoftDeleteColumn = Nil;

        fn primary_key(&self) -> Self::PrimaryKey {
            self.id
        }

        fn table_name() -> &'static str {
            "anvils"
        }

        fn columns() -> &'static [&'static str] {
            &["id", "name", "weight"]
        }
    }

    /// Type-safe column marker for the `weight` column.
    struct WeightColumn;

    impl Column<Anvil> for WeightColumn {
        type Type = i16;

        fn name(&self) -> &'static str {
            "weight"
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = Builder::<Anvil>::default()
            // Call `where` on `Selected`, transitioning to `Filtered`
            .r#where(WeightColumn, ">=", 10)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_null(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = Builder::<Anvil>::default()
            // Call `where_null` on `Selected`, transitioning to `Filtered`
            .r#where_null(WeightColumn)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_order_by(connection: Pool<Postgres>) {
        // `order_by` can be called on `Selected`
        assert!(
            Builder::<Anvil>::default()
                .order_by("weight", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `order_by` can be called on `Filtered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_limit(connection: Pool<Postgres>) {
        // `limit` can be called on `Selected`
        assert!(
            Builder::<Anvil>::default()
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Filtered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Ordered`
        assert!(
            Builder::<Anvil>::default()
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_offset(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = Builder::<Anvil>::default()
            // Call `limit` on `Selected`, transitioning to `Limited`
            .limit(10)
            // Call `offset` on `Limited`, transitioning to `Offsetted`
            .offset(20)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_get(connection: Pool<Postgres>) {
        // `get` can be called on `Selected`
        assert!(Builder::<Anvil>::default().get(&connection).await.is_ok());

        // `get` can be called on `Filtered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Ordered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Limited`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Offsetted`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .limit(10)
                .offset(20)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first(connection: Pool<Postgres>) {
        // `first` can be called on `Selected`
        assert!(Builder::<Anvil>::default().first(&connection).await.is_ok());

        // `first` can be called on `Filtered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .first(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Ordered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .first(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first_or_fail(connection: Pool<Postgres>) {
        // `first_or_fail` can be called on `Selected`
        assert!(
            Builder::<Anvil>::default()
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Filtered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Ordered`
        assert!(
            Builder::<Anvil>::default()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );
    }
}
