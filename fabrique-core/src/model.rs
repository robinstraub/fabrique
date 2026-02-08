pub mod column_set;
pub mod join;
mod query_builder;

use crate::{
    database::{Column, DatabaseAware},
    sql::{Inserting, Updating},
};
pub use column_set::ColumnSet;
pub use join::{Joined, RootModel};
pub use query_builder::{Building, Initial, QueryBuilder};

/// Model metadata and identity
pub trait Model: DatabaseAware {
    /// Primary key type (single value or tuple for composite keys)
    type PrimaryKey: Send;

    /// Soft delete column type (unit type if soft deletes are not enabled).
    type SoftDeleteColumn: Column<Self> + Send;

    /// Returns the primary key value of this model instance
    fn primary_key(&self) -> Self::PrimaryKey;

    /// Returns the table name for this model
    fn table_name() -> &'static str;

    /// Returns the column names for this model (unqualified)
    fn columns() -> &'static [&'static str];

    /// Returns the column names qualified with table name (e.g., "products.id")
    fn qualified_columns() -> &'static [&'static str];

    /// Returns the primary key column names for this model
    fn primary_key_columns() -> &'static [&'static str];

    /// Returns the soft delete column if this model uses soft deletes.
    fn soft_delete_column() -> Option<Self::SoftDeleteColumn>
    where
        Self: Sized,
    {
        None
    }
}

/// Query building and retrieval operations
pub trait Query: Model + Send + Unpin
where
    Self::Database: sqlx::Database,
    Self::Error: From<sqlx::Error>,
    <Self::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Self::Database>,
    for<'r> Self: sqlx::FromRow<'r, <Self::Database as sqlx::Database>::Row>,
{
    /// Creates a new SELECT query builder for this model.
    fn query() -> QueryBuilder<Initial, Joined<Self, ()>> {
        QueryBuilder::new()
    }

    /// Creates a new INSERT query builder for this model.
    ///
    /// Returns a builder in the `Inserting` state, ready for `.set()` calls.
    fn insert() -> QueryBuilder<Building<Self::Database, Inserting>, Joined<Self, ()>> {
        Self::query().insert()
    }

    /// Creates a new UPDATE query builder for this model.
    ///
    /// Returns a builder in the `Updating` state, ready for `.set()` calls.
    fn update() -> QueryBuilder<Building<Self::Database, Updating>, Joined<Self, ()>> {
        Self::query().update()
    }

    /// Finds a model instance by its primary key.
    ///
    /// Returns an error if no record is found.
    fn find<'e, A>(
        executor: A,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Retrieves all instances of this model from the database.
    fn all<'e, E>(executor: E) -> impl Future<Output = Result<Vec<Self>, Self::Error>> + Send + 'e
    where
        E: sqlx::Executor<'e, Database = Self::Database> + 'e,
        for<'q> <Self::SoftDeleteColumn as Column<Self>>::Type:
            sqlx::Encode<'q, Self::Database> + sqlx::Type<Self::Database>,
    {
        async move {
            match Self::soft_delete_column() {
                Some(column) => Self::query().where_null(column).get(executor).await,
                None => Self::query().get(executor).await,
            }
        }
    }
}

/// Create and update operations
pub trait Persist: Model {
    /// Creates and persists this model instance
    fn create<'e, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Saves this model instance to the database.
    ///
    /// Performs an UPSERT: inserts the record if it doesn't exist,
    /// or updates it if there's a conflict on the primary key.
    fn save<'e, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;
}

/// Delete operations
pub trait Delete: Model {
    /// Deletes this model instance
    ///
    /// If the model uses soft delete, this will perform a soft delete.
    /// Otherwise, it will permanently delete the record.
    fn delete<'e, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Destroys a model by its primary key
    ///
    /// If the model uses soft delete, this will perform a soft destroy.
    /// Otherwise, it will permanently delete the record.
    fn destroy<'e, A>(
        executor: A,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;
}

/// Soft delete operations
pub trait SoftDelete: Model {
    /// Soft deletes this model instance
    fn soft_delete<'e, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Soft destroys a model by its primary key
    fn soft_destroy<'e, A>(
        executor: A,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Restores a soft-deleted model instance
    fn restore<'e, A>(
        &self,
        executor: A,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Checks if this model instance is soft-deleted
    fn trashed<'e, A>(
        &self,
        executor: A,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;
}

/// Hard delete operations for soft-deletable models
pub trait HardDelete: Model {
    /// Permanently deletes this model instance (bypassing soft delete)
    fn hard_delete<'e, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;

    /// Permanently destroys a model by its primary key
    fn hard_destroy<'e, A>(
        executor: A,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'e
    where
        A: sqlx::Acquire<'e, Database = Self::Database> + Send + 'e;
}
