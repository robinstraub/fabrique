use std::{future::Future, pin::Pin};

use crate::Model;

/// Trait representing a factory that can create model instances.
///
/// Factories provide a fluent interface for building and persisting model
/// instances with optional field values and relation support.
///
/// Factory methods that produce async behavior (like `for`) are designed to
/// be chained synchronously, with their async execution deferred to a single
/// async block in the `create()` method. This enables a clean builder pattern
/// while maintaining async capabilities.
pub trait Factory {
    /// The model type this factory produces
    type Model: Model;
}

/// Type alias for the future returned by `into_key`.
pub type IntoKeyFuture<'a, M> = Pin<
    Box<dyn Future<Output = Result<<M as Model>::PrimaryKey, <M as Model>::Error>> + Send + 'a>,
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
{
    /// Extracts or creates the relation key value.
    ///
    /// For model instances, extracts the primary key. For factory instances,
    /// creates the model and extracts its primary key.
    ///
    /// Returns a pinned boxed future to enable object-safe trait usage and
    /// deferred execution. This allows relation setup to remain synchronous
    /// while deferring the actual async work to the `create()` method.
    fn into_key<'a>(self: Box<Self>, connection: &'a M::Connection) -> IntoKeyFuture<'a, M>;
}

/// Blanket implementation for model instances.
///
/// This adapter implementation allows passing model instances directly to
/// `for_[relation]` methods by extracting their primary key.
impl<M: Model + Send + 'static> ForRelation<M> for M {
    fn into_key<'a>(self: Box<Self>, _connection: &'a M::Connection) -> IntoKeyFuture<'a, M> {
        Box::pin(async move { Ok(self.primary_key()) })
    }
}
