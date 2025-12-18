use crate::{database::DatabaseAware, model::Model};
use std::future::Future;

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

    fn create<'a, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<Self::Model, <Self::Model as DatabaseAware>::Error>> + Send + 'a
    where
        A: sqlx::Acquire<'a, Database = <Self::Model as DatabaseAware>::Database> + Send + 'a;
}
