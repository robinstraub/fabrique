use crate::{database::DatabaseAware, model::Model};
use std::future::Future;

/// Factory pattern for creating model instances with test data.
///
/// Factories provide a fluent builder for constructing and persisting
/// model instances with optional field overrides and relation support.
///
/// Factories must implement `Clone` to allow configuration reuse.
/// A configured factory can be cloned and used multiple times,
/// for example when creating several related instances.
pub trait Factory: Clone + Sized {
    /// The model type this factory produces.
    type Model: Model;

    /// Builds and persists the model instance.
    fn create<'a, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<Self::Model, <Self::Model as DatabaseAware>::Error>> + Send + 'a
    where
        A: sqlx::Acquire<'a, Database = <Self::Model as DatabaseAware>::Database> + Send + 'a;
}
