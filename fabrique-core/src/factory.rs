use crate::{database::DatabaseAware, model::Model, relation::BelongsTo};
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

/// Trait for setting a foreign key on a factory that references a parent model.
///
/// This trait is automatically implemented by the derive macro for factories
/// whose model has a `#[fabrique(belongs_to = "Parent")]` field. It enables
/// parent factories to set the foreign key on child factories when creating
/// `HasMany` relationships.
///
/// The optional `Alias` parameter disambiguates multiple relationships
/// to the same parent type (see `alias` attribute).
///
/// # Example
///
/// ```rust
/// # use fabrique::prelude::*;
/// // OrderFactory implements SetForeignKey<Customer> because Order belongs_to Customer.
/// // CustomerFactory uses this when creating child orders:
/// //
/// // <OrderFactory as SetForeignKey<Customer>>::set_foreign_key(factory, customer_pk)
/// ```
pub trait SetForeignKey<Parent: Model, Alias = ()>: Factory
where
    Self::Model: BelongsTo<Parent, Alias>,
{
    /// Sets the foreign key field that references the parent model.
    fn set_foreign_key(self, parent_key: Parent::PrimaryKey) -> Self;
}
