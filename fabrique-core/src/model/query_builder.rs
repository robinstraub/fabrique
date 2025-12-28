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
    struct Product {
        id: Uuid,
        name: String,
        price_cents: i32,
        in_stock: bool,
    }

    impl DatabaseAware for Product {
        type Database = Postgres;
        type Error = sqlx::Error;
    }

    impl Model for Product {
        type PrimaryKey = Uuid;
        type SoftDeleteColumn = Nil;

        fn primary_key(&self) -> Self::PrimaryKey {
            self.id
        }

        fn table_name() -> &'static str {
            "products"
        }

        fn columns() -> &'static [&'static str] {
            &["id", "name", "price_cents", "in_stock"]
        }

        fn qualified_columns() -> &'static [&'static str] {
            &[
                "products.id",
                "products.name",
                "products.price_cents",
                "products.in_stock",
            ]
        }

        fn primary_key_columns() -> &'static [&'static str] {
            &["id"]
        }
    }

    /// Type-safe column marker for the `id` column.
    struct IdColumn;

    impl Column<Product> for IdColumn {
        type Type = Uuid;

        fn name(&self) -> &'static str {
            "id"
        }

        fn qualified_name(&self) -> &'static str {
            "products.id"
        }
    }

    /// Type-safe column marker for the `name` column.
    struct NameColumn;

    impl Column<Product> for NameColumn {
        type Type = String;

        fn name(&self) -> &'static str {
            "name"
        }

        fn qualified_name(&self) -> &'static str {
            "products.name"
        }
    }

    /// Type-safe column marker for the `price_cents` column.
    struct PriceCentsColumn;

    impl Column<Product> for PriceCentsColumn {
        type Type = i32;

        fn name(&self) -> &'static str {
            "price_cents"
        }

        fn qualified_name(&self) -> &'static str {
            "products.price_cents"
        }
    }

    /// Type-safe column marker for the `in_stock` column.
    struct InStockColumn;

    impl Column<Product> for InStockColumn {
        type Type = bool;

        fn name(&self) -> &'static str {
            "in_stock"
        }

        fn qualified_name(&self) -> &'static str {
            "products.in_stock"
        }
    }

    #[test]
    fn test_primary_key() {
        let product = Product::default();
        assert_eq!(product.primary_key(), Uuid::default());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where(connection: Pool<Postgres>) {
        let result: Result<Vec<Product>, sqlx::Error> = QueryBuilder::<Product>::default()
            .select()
            // Call `where` on `Selected`, transitioning to `Filtered`
            .r#where(PriceCentsColumn, ">=", 10)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_null(connection: Pool<Postgres>) {
        let result: Result<Vec<Product>, sqlx::Error> = QueryBuilder::<Product>::default()
            .select()
            // Call `where_null` on `Selected`, transitioning to `Filtered`
            .where_null(PriceCentsColumn)
            // Ensure the generated query can be executed
            .get(&connection)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_where_not_null(connection: Pool<Postgres>) {
        let result: Result<Vec<Product>, sqlx::Error> = QueryBuilder::<Product>::default()
            .select()
            // Call `where_null` on `Selected`, transitioning to `Filtered`
            .where_not_null(PriceCentsColumn)
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
            QueryBuilder::<Product>::default()
                .select()
                .order_by("price_cents", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `order_by` can be called on `Filtered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_limit(connection: Pool<Postgres>) {
        // `limit` can be called on `Selected`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Filtered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `limit` can be called on `Ordered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_offset(connection: Pool<Postgres>) {
        let result: Result<Vec<Product>, sqlx::Error> = QueryBuilder::<Product>::default()
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
            QueryBuilder::<Product>::default()
                .select()
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Filtered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Ordered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Limited`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Offsetted`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .limit(10)
                .offset(20)
                .get(&connection)
                .await
                .is_ok()
        );

        // `get` can be called on `Returned`
        assert!(
            QueryBuilder::<Product>::default()
                .update()
                .set(PriceCentsColumn, 0)
                .r#where(PriceCentsColumn, ">=", 0)
                .returning(&[PriceCentsColumn.name()])
                .get(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first(connection: Pool<Postgres>) {
        // `first` can be called on `Selected`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .first(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Filtered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .first(&connection)
                .await
                .is_ok()
        );

        // `first` can be called on `Ordered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .first(&connection)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_first_or_fail(connection: Pool<Postgres>) {
        // `first_or_fail` can be called on `Selected`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Filtered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );

        // `first_or_fail` can be called on `Ordered`
        assert!(
            QueryBuilder::<Product>::default()
                .select()
                .r#where(PriceCentsColumn, ">=", 0)
                .order_by("price_cents", Direction::Asc)
                .first_or_fail(&connection)
                .await
                .is_err() // No rows exist, so this should fail
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_update(connection: Pool<Postgres>) {
        // Arrange a row in the database.
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Original")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the update.
        let result: Product = QueryBuilder::<Product>::default()
            .update()
            .set(NameColumn, "Updated".to_string())
            .set(PriceCentsColumn, 200i32)
            .r#where(IdColumn, "=", id)
            .returning(&["id", "in_stock", "name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();

        // Assert the row is updated.
        assert_eq!(result.name, "Updated");
        assert_eq!(result.price_cents, 200);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert(connection: Pool<Postgres>) {
        // Arrange a new product.
        let id = Uuid::new_v4();

        // Act the insert.
        let result: Result<Product, _> = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, true)
            .set(NameColumn, "Test".to_string())
            .set(PriceCentsColumn, 100i32)
            .returning()
            .first_or_fail(&connection)
            .await;

        // Assert the row is inserted.
        assert!(result.is_ok());
        let product = result.unwrap();
        assert_eq!(product.id, id);
        assert!(product.in_stock);
        assert_eq!(product.name, "Test");
        assert_eq!(product.price_cents, 100);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert_do_nothing_on_conflict(connection: Pool<Postgres>) {
        // Arrange a row in the database.
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Existing")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the insert with DO NOTHING on conflict.
        let result: Result<Option<Product>, _> = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, true)
            .set(NameColumn, "Ignored".to_string())
            .set(PriceCentsColumn, 999i32)
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
        SqlQueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Existing")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        // Act the insert with DO UPDATE on conflict.
        let result: Result<Product, _> = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, true)
            .set(NameColumn, "Updated".to_string())
            .set(PriceCentsColumn, 200i32)
            .on_conflict()
            .do_update()
            .returning()
            .first_or_fail(&connection)
            .await;

        // Assert the row is updated.
        assert!(result.is_ok());
        let product = result.unwrap();
        assert_eq!(product.id, id);
        assert!(product.in_stock);
        assert_eq!(product.name, "Updated");
        assert_eq!(product.price_cents, 200);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_returning(connection: Pool<Postgres>) {
        // Setup: insert a row
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("products")
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
        let result: Product = QueryBuilder::<Product>::default()
            .update()
            .set(NameColumn, "Updated".to_string())
            .returning(&["id", "in_stock", "name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.name, "Updated");

        // `returning` can be called on `Filtered<Updated>`
        let result: Product = QueryBuilder::<Product>::default()
            .update()
            .set(NameColumn, "Filtered Update".to_string())
            .r#where(IdColumn, "=", id)
            .returning(&["id", "in_stock", "name", "price_cents"])
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.name, "Filtered Update");

        // `returning` can be called on `Inserted`
        let new_id = Uuid::new_v4();
        let result: Product = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, new_id)
            .set(InStockColumn, true)
            .set(NameColumn, "New Product".to_string())
            .set(PriceCentsColumn, 150i32)
            .returning()
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.id, new_id);
        assert_eq!(result.name, "New Product");

        // `returning` can be called on `Upserted` (DO UPDATE)
        let result: Product = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, false)
            .set(NameColumn, "Upserted Name".to_string())
            .set(PriceCentsColumn, 200i32)
            .on_conflict()
            .do_update()
            .returning()
            .first_or_fail(&connection)
            .await
            .unwrap();
        assert_eq!(result.name, "Upserted Name");
        assert_eq!(result.price_cents, 200);

        // `returning` can be called on `Upserted` (DO NOTHING)
        let result: Option<Product> = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, false)
            .set(NameColumn, "Ignored".to_string())
            .set(PriceCentsColumn, 999i32)
            .on_conflict()
            .do_nothing()
            .returning()
            .first(&connection)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_execute(connection: Pool<Postgres>) {
        // `execute` can be called on `Filtered<Updated>`
        let id = Uuid::new_v4();
        SqlQueryBuilder::table("products")
            .insert()
            .set("id", id)
            .set("in_stock", true)
            .set("name", "Original")
            .set("price_cents", 100i32)
            .returning(&["id"])
            .first_or_fail::<(Uuid,), _>(&connection)
            .await
            .unwrap();

        let result = QueryBuilder::<Product>::default()
            .update()
            .set(NameColumn, "Updated".to_string())
            .r#where(IdColumn, "=", id)
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Inserted`
        let result = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, Uuid::new_v4())
            .set(InStockColumn, true)
            .set(NameColumn, "New Product".to_string())
            .set(PriceCentsColumn, 150i32)
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Upserted` (DO UPDATE)
        let result = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, false)
            .set(NameColumn, "Upserted".to_string())
            .set(PriceCentsColumn, 200i32)
            .on_conflict()
            .do_update()
            .execute(&connection)
            .await;
        assert!(result.is_ok());

        // `execute` can be called on `Upserted` (DO NOTHING)
        let result = QueryBuilder::<Product>::default()
            .insert()
            .set(IdColumn, id)
            .set(InStockColumn, false)
            .set(NameColumn, "Ignored".to_string())
            .set(PriceCentsColumn, 999i32)
            .on_conflict()
            .do_nothing()
            .execute(&connection)
            .await;
        assert!(result.is_ok());
    }
}
