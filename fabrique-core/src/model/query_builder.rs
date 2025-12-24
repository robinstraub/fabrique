use crate::database::Column;
use crate::model::Model;
use crate::sql::operators::{Direction, Operator};
use crate::sql::{
    Conflicted, Filtered, Initial, Inserted, Inserting, Limited, Offsetted, Ordered,
    QueryBuilder as SqlQueryBuilder, Returned, Selected, Updated, Updating, Upserted,
};

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
            impl<M> QueryBuilder<M, $state>
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
            impl<M> QueryBuilder<M, $state>
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
            inner: self.inner.select(M::columns()),
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
impl_where!(Selected, Updated);
impl_order_by!(Selected, Filtered<Selected>);
impl_limit!(Selected, Filtered<Selected>, Ordered);
impl_get!(Selected, Filtered<Selected>, Ordered, Limited, Offsetted);
impl_first!(Selected, Filtered<Selected>, Ordered);
impl_first_or_fail!(Selected, Filtered<Selected>, Ordered);
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
{
    /// Executes the query and returns all resulting rows.
    pub async fn get<'e, E>(self, executor: E) -> Result<Vec<M>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.get(executor).await
    }

    /// Executes the query and returns the first resulting row.
    pub async fn first<'e, E>(self, executor: E) -> Result<Option<M>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.first(executor).await
    }

    /// Executes the query and returns the first resulting row, or fails.
    pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<M, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = M::Database>,
        M: for<'r> sqlx::FromRow<'r, <M::Database as sqlx::Database>::Row> + Send + Unpin,
        <M::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<M::Database>,
    {
        self.inner.first_or_fail(executor).await
    }
}

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
        material: String,
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
            &["id", "material", "name", "weight"]
        }

        fn primary_key_columns() -> &'static [&'static str] {
            &["id"]
        }
    }

    /// Type-safe column marker for the `id` column.
    struct IdColumn;

    impl Column<Anvil> for IdColumn {
        type Type = Uuid;

        fn name(&self) -> &'static str {
            "id"
        }
    }

    /// Type-safe column marker for the `material` column.
    struct MaterialColumn;

    impl Column<Anvil> for MaterialColumn {
        type Type = String;

        fn name(&self) -> &'static str {
            "material"
        }
    }

    /// Type-safe column marker for the `name` column.
    struct NameColumn;

    impl Column<Anvil> for NameColumn {
        type Type = String;

        fn name(&self) -> &'static str {
            "name"
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

    #[test]
    fn test_primary_key() {
        let anvil = Anvil::default();
        assert_eq!(anvil.primary_key(), Uuid::default());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = QueryBuilder::<Anvil>::default()
            .select()
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
        let result: Result<Vec<Anvil>, sqlx::Error> = QueryBuilder::<Anvil>::default()
            .select()
            // Call `where_null` on `Selected`, transitioning to `Filtered`
            .where_null(WeightColumn)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_not_null(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = QueryBuilder::<Anvil>::default()
            .select()
            // Call `where_null` on `Selected`, transitioning to `Filtered`
            .where_not_null(WeightColumn)
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
            QueryBuilder::<Anvil>::default()
                .select()
                .order_by("weight", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `order_by` can be called on `Filtered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
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
            QueryBuilder::<Anvil>::default()
                .select()
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Filtered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Ordered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_offset(connection: Pool<Postgres>) {
        let result: Result<Vec<Anvil>, sqlx::Error> = QueryBuilder::<Anvil>::default()
            .select()
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
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Filtered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Ordered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Limited`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Offsetted`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .limit(10)
                .offset(20)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Returned`
        assert!(
            QueryBuilder::<Anvil>::default()
                .update()
                .set(WeightColumn, 0)
                .r#where(WeightColumn, ">=", 0)
                .returning(&[WeightColumn.name()])
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first(connection: Pool<Postgres>) {
        // `first` can be called on `Selected`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .first(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Filtered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .first(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Ordered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
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
            QueryBuilder::<Anvil>::default()
                .select()
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Filtered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Ordered`
        assert!(
            QueryBuilder::<Anvil>::default()
                .select()
                .r#where(WeightColumn, ">=", 0)
                .order_by("weight", Direction::Asc)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update(connection: Pool<Postgres>) {
        // Arrange a row in the database.
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("anvils")
            .insert()
            .set("id", id)
            .set("material", "Iron")
            .set("name", "Original")
            .set("weight", 100i16)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the update.
        let result: Anvil = QueryBuilder::<Anvil>::default()
            .update()
            .set(NameColumn, "Updated".to_string())
            .set(WeightColumn, 200i16)
            .r#where(IdColumn, "=", id)
            .returning(&["id", "material", "name", "weight"])
            .first_or_fail(&connection)
            .await
            .unwrap();

        // Assert the row is updated.
        assert_eq!(result.name, "Updated");
        assert_eq!(result.weight, 200);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert(connection: Pool<Postgres>) {
        // Arrange a new anvil.
        let id = Uuid::new_v4();

        // Act the insert.
        let result: Result<Anvil, _> = QueryBuilder::<Anvil>::default()
            .insert()
            .set(IdColumn, id)
            .set(MaterialColumn, "Iron".to_string())
            .set(NameColumn, "Test".to_string())
            .set(WeightColumn, 100i16)
            .returning()
            .first_or_fail(&connection)
            .await;

        // Assert the row is inserted.
        assert!(result.is_ok());
        let anvil = result.unwrap();
        assert_eq!(anvil.id, id);
        assert_eq!(anvil.material, "Iron");
        assert_eq!(anvil.name, "Test");
        assert_eq!(anvil.weight, 100);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert_do_nothing_on_conflict(connection: Pool<Postgres>) {
        // Arrange a row in the database.
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("anvils")
            .insert()
            .set("id", id)
            .set("material", "Iron")
            .set("name", "Existing")
            .set("weight", 100i16)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the insert with DO NOTHING on conflict.
        let result: Result<Option<Anvil>, _> = QueryBuilder::<Anvil>::default()
            .insert()
            .set(IdColumn, id)
            .set(MaterialColumn, "Steel".to_string())
            .set(NameColumn, "Ignored".to_string())
            .set(WeightColumn, 999i16)
            .on_conflict()
            .do_nothing()
            .returning()
            .first(&connection)
            .await;

        // Assert the call succeeded but no row was returned.
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert_do_update_on_conflict(connection: Pool<Postgres>) {
        // Arrange a row in the database.
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("anvils")
            .insert()
            .set("id", id)
            .set("material", "Iron")
            .set("name", "Existing")
            .set("weight", 100i16)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the insert with DO UPDATE on conflict.
        let result: Result<Anvil, _> = QueryBuilder::<Anvil>::default()
            .insert()
            .set(IdColumn, id)
            .set(MaterialColumn, "Steel".to_string())
            .set(NameColumn, "Updated".to_string())
            .set(WeightColumn, 200i16)
            .on_conflict()
            .do_update()
            .returning()
            .first_or_fail(&connection)
            .await;

        // Assert the row is updated.
        assert!(result.is_ok());
        let anvil = result.unwrap();
        assert_eq!(anvil.id, id);
        assert_eq!(anvil.material, "Steel");
        assert_eq!(anvil.name, "Updated");
        assert_eq!(anvil.weight, 200);
    }
}
