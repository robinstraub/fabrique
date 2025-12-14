use crate::database::Database;
use crate::query_builder::QueryBuilder;

/// Model metadata and identity
pub trait Model: Database {
    /// Primary key type (single value or tuple for composite keys)
    type PrimaryKey: Send;

    /// Returns the primary key value of this model instance
    fn primary_key(&self) -> Self::PrimaryKey;

    /// Returns the table name for this model
    fn table_name() -> &'static str;

    /// Returns whether this model uses soft delete
    fn uses_soft_delete() -> bool;
}

/// Query building and retrieval operations
pub trait Query: Model {
    type QueryBuilder: QueryBuilder;

    /// Creates a new query builder for this model
    fn query() -> Self::QueryBuilder;

    /// Retrieves all instances of this model from the database
    fn all(
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Vec<Self>, Self::Error>> + Send;
}

/// Create operations
pub trait Persist: Model {
    /// Creates and persists this model instance
    fn create(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

/// Delete operations
pub trait Delete: Model {
    /// Deletes this model instance
    ///
    /// If the model uses soft delete, this will perform a soft delete.
    /// Otherwise, it will permanently delete the record.
    fn delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Destroys a model by its primary key
    ///
    /// If the model uses soft delete, this will perform a soft destroy.
    /// Otherwise, it will permanently delete the record.
    fn destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Soft delete operations
pub trait SoftDelete: Model {
    /// Soft deletes this model instance
    fn soft_delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Soft destroys a model by its primary key
    fn soft_destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Restores a soft-deleted model instance
    fn restore(
        &self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Checks if this model instance is soft-deleted
    fn trashed(
        &self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
}

/// Hard delete operations for soft-deletable models
pub trait HardDelete: Model {
    /// Permanently deletes this model instance (bypassing soft delete)
    fn hard_delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Permanently destroys a model by its primary key
    fn hard_destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
