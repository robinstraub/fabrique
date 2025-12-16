use crate::{database::DatabaseAware, query::builder::Builder};

/// Model metadata and identity
pub trait Model: DatabaseAware {
    /// Primary key type (single value or tuple for composite keys)
    type PrimaryKey: Send;

    /// Returns the primary key value of this model instance
    fn primary_key(&self) -> Self::PrimaryKey;

    /// Returns the table name for this model
    fn table_name() -> &'static str;

    /// Returns the column names for this model
    fn columns() -> &'static [&'static str];

    /// Returns whether this model uses soft delete
    fn uses_soft_delete() -> bool;
}

/// Query building and retrieval operations
pub trait Query: Model + Send + Unpin
where
    Self::Database: sqlx::Database,
    Self::Error: From<sqlx::Error>,
    <Self::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Self::Database>,
    for<'r> Self: sqlx::FromRow<'r, <Self::Database as sqlx::Database>::Row>,
{
    /// Creates a new query builder for this model.
    fn query() -> Builder<Self> {
        Builder::default()
    }

    /// Retrieves all instances of this model from the database
    fn all<E>(executor: E) -> impl Future<Output = Result<Vec<Self>, Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>,
    {
        async {
            Builder::default()
                .fetch_all(executor)
                .await
                .map_err(Into::into)
        }
    }
}

/// Create operations
pub trait Persist: Model {
    /// Creates and persists this model instance
    fn create<E>(self, executor: E) -> impl Future<Output = Result<Self, Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;
}

/// Delete operations
pub trait Delete: Model {
    /// Deletes this model instance
    ///
    /// If the model uses soft delete, this will perform a soft delete.
    /// Otherwise, it will permanently delete the record.
    fn delete<E>(self, executor: E) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;

    /// Destroys a model by its primary key
    ///
    /// If the model uses soft delete, this will perform a soft destroy.
    /// Otherwise, it will permanently delete the record.
    fn destroy<E>(
        executor: E,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;
}

/// Soft delete operations
pub trait SoftDelete: Model {
    /// Soft deletes this model instance
    fn soft_delete<E>(
        self,
        executor: E,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;

    /// Soft destroys a model by its primary key
    fn soft_destroy<E>(
        executor: E,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;

    /// Restores a soft-deleted model instance
    fn restore<E>(&self, executor: E) -> impl Future<Output = Result<(), Self::Error>>
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;

    /// Checks if this model instance is soft-deleted
    fn trashed<E>(&self, executor: E) -> impl Future<Output = Result<bool, Self::Error>>
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;
}

/// Hard delete operations for soft-deletable models
pub trait HardDelete: Model {
    /// Permanently deletes this model instance (bypassing soft delete)
    fn hard_delete<E>(
        self,
        executor: E,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;

    /// Permanently destroys a model by its primary key
    fn hard_destroy<E>(
        executor: E,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = Self::Database>;
}
