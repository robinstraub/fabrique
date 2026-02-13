use crate::{dialect::Dialect, model::Model};
use std::future::Future;
use std::pin::Pin;

use crate::error::Error;

/// Factory pattern for creating model instances with test data.
///
/// Factories provide a fluent builder for constructing and persisting
/// model instances with optional field overrides and relation support.
///
/// Factories must implement `Clone` to allow configuration reuse.
/// A configured factory can be cloned and used multiple times,
/// for example when creating several related instances.
pub trait Factory<DB: Dialect>: Clone + Sized {
    /// The model type this factory produces.
    type Model: Model;

    /// Builds and persists the model instance.
    fn create<'a, A>(
        self,
        executor: A,
    ) -> impl Future<Output = Result<Self::Model, Error>> + Send + 'a
    where
        A: sqlx::Acquire<'a, Database = DB> + Send + 'a;
}

/// Trait for setting a foreign key on a factory that references a parent model.
///
/// This trait is automatically implemented by the derive macro for factories
/// whose model has a `#[fabrique(belongs_to = "Parent")]` field. It enables
/// parent factories to set the foreign key on child factories when creating
/// one-to-many relationships.
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
pub trait SetForeignKey<Parent: Model, Alias = ()>: Sized {
    /// Sets the foreign key field that references the parent model.
    fn set_foreign_key(self, parent_key: Parent::PrimaryKey) -> Self;
}

/// A factory whose execution is deferred until a parent is created.
///
/// When a parent factory has `has_` methods (e.g., `has_orders`), each call
/// pushes an `Arc<dyn DeferredFactory>` into the parent's children list.
/// After the parent instance is persisted, each deferred factory is invoked
/// with the parent's primary key to create the child instances.
pub trait DeferredFactory<PK, DB: Dialect>: Send + Sync {
    /// Creates child instances using the given parent primary key.
    fn create<'a>(
        &'a self,
        parent_pk: PK,
        conn: &'a mut <DB as sqlx::Database>::Connection,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
}
