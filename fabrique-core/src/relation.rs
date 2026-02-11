//! Relationship types for model associations.
//!
//! This module provides marker types and traits for defining relationships
//! between models.

use crate::database::Column;
use crate::model::Model;

/// Trait for models that belong to a parent model.
///
/// This trait is automatically implemented by the derive macro for fields
/// annotated with `#[fabrique(belongs_to = "ParentModel")]`. It provides
/// the foreign key column for relationship inference.
///
/// The optional `Alias` parameter disambiguates multiple relationships
/// to the same parent type (see `alias` attribute).
///
/// # Example
///
/// ```rust
/// # use fabrique::prelude::*;
/// # use uuid::Uuid;
/// #[derive(Model)]
/// struct Order {
///     id: Uuid,
///     #[fabrique(belongs_to = "User")]
///     user_id: Uuid,  // Generates: impl BelongsTo<User> for Order
/// }
///
/// #[derive(Model)]
/// struct User {
///     id: Uuid,
/// }
/// ```
pub trait BelongsTo<Parent: Model, Alias = ()>: Model {
    /// The type-safe column type for the foreign key.
    type ForeignKeyColumn: Column<Self>;

    /// Returns the foreign key column that references the parent model.
    fn foreign_key_column() -> Self::ForeignKeyColumn;
}

/// Trait for models that can be joined with another model.
///
/// Enables bidirectional joins between related models. When a `belongs_to`
/// relationship is defined, the derive macro generates `Joinable`
/// implementations in both directions.
///
/// The optional `Alias` parameter disambiguates multiple joins
/// to the same model type (see `alias` attribute).
pub trait Joinable<J: Model, Alias = ()>: Model {
    /// The column type from `Self` used in the join.
    type LeftColumn: Column<Self>;

    /// The column type from `J` used in the join.
    type RightColumn: Column<J>;

    /// Returns the column from `Self` for the join ON clause.
    fn left_column() -> Self::LeftColumn;

    /// Returns the column from `J` for the join ON clause.
    fn right_column() -> Self::RightColumn;
}

/// Trait for alias pseudo-models that reference a real model.
///
/// When a `belongs_to` field has an `alias` attribute, the derive macro
/// generates a pseudo-model (e.g., `Seller`) that implements this trait to link
/// back to the real model (e.g., `User`).
///
/// This enables factories to create the correct record type while using
/// the alias for type-safe disambiguation.
pub trait Alias {
    /// The real model that this alias represents.
    type Target: Model;

    /// The SQL alias name (e.g., "sender").
    const NAME: &'static str;
}
