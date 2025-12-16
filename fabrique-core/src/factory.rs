use sqlx::Database;

use crate::{database::DatabaseAware, model::Model};
use std::{future::Future, pin::Pin};

/// Trait representing a factory that can create model instances.
///
/// Factories provide a fluent interface for building and persisting model
/// instances with optional field values and relation support.
///
/// Factory methods that produce async behavior (like `for`) are designed to
/// be chained synchronously, with their async execution deferred to a single
/// async block in the `create()` method. This enables a clean builder pattern
/// while maintaining async capabilities.
pub trait Factory: Sized {
    /// The model type this factory produces
    type Model: Model;

    fn create<E>(
        self,
        executor: E,
    ) -> impl Future<Output = Result<Self::Model, <Self::Model as DatabaseAware>::Error>> + Send
    where
        E: for<'e> sqlx::Executor<'e, Database = <Self::Model as DatabaseAware>::Database>;
}

/// Type alias for the future returned by `into_key`.
pub type IntoKeyFuture<M> = Pin<
    Box<dyn Future<Output = Result<<M as Model>::PrimaryKey, <M as DatabaseAware>::Error>> + Send>,
>;

/// Trait for types that can establish factory belongs-to relationships.
///
/// Model instances automatically implement this trait via a blanket
/// implementation that extracts their primary key. Factory instances receive an
/// implementation from the derive macro that creates the model first, then
/// extracts its key.
pub trait ForRelation<M: Model>: Send
where
    M::PrimaryKey: Send,
    M::Database: sqlx::Database,
{
    /// Extracts or creates the relation key value.
    ///
    /// For model instances, extracts the primary key. For factory instances,
    /// creates the model and extracts its primary key.
    ///
    /// Returns a pinned boxed future to enable object-safe trait usage and
    /// deferred execution. This allows relation setup to remain synchronous
    /// while deferring the actual async work to the `create()` method.
    fn into_key<E>(self: Box<Self>, executor: E) -> IntoKeyFuture<M>
    where
        E: for<'e> sqlx::Executor<'e, Database = <M as DatabaseAware>::Database>;
}

/// Blanket implementation for model instances.
///
/// This adapter implementation allows passing model instances directly to
/// `for_[relation]` methods by extracting their primary key.
impl<M: Model + Send + 'static> ForRelation<M> for M
where
    M::Database: sqlx::Database,
{
    fn into_key<E>(self: Box<Self>, _executor: E) -> IntoKeyFuture<M>
    where
        E: for<'e> sqlx::Executor<'e, Database = <M as DatabaseAware>::Database>,
    {
        Box::pin(async move { Ok(self.primary_key()) })
    }
}
