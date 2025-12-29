//! Relationship types for model associations.
//!
//! This module provides marker types and traits for defining relationships
//! between models.

use std::marker::PhantomData;

use crate::database::Column;
use crate::model::Model;

/// Trait for models that belong to a parent model.
///
/// This trait is automatically implemented by the derive macro for fields
/// annotated with `#[fabrique(belongs_to = "ParentModel")]`. It provides
/// the foreign key column, enabling automatic relationship inference
/// for `HasMany<T>` fields.
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
///     orders: HasMany<Order>,  // Infers FK via <Order as BelongsTo<User>>
/// }
/// ```
pub trait BelongsTo<Parent: Model>: Model {
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
pub trait Joinable<J: Model>: Model {
    /// The column type from `Self` used in the join.
    type LeftColumn: Column<Self>;

    /// The column type from `J` used in the join.
    type RightColumn: Column<J>;

    /// Returns the column from `Self` for the join ON clause.
    fn left_column() -> Self::LeftColumn;

    /// Returns the column from `J` for the join ON clause.
    fn right_column() -> Self::RightColumn;
}

/// Marker type for one-to-many relationships.
///
/// `HasMany<T>` is a zero-sized type (ZST) that represents a one-to-many
/// relationship where the current model is the "parent" and `T` is the
/// "child" model type.
///
/// This type is NOT stored in the database. It's purely a compile-time
/// marker used by the derive macros to generate navigation methods and
/// factory helpers.
#[derive(Debug, PartialEq, Eq)]
pub struct HasMany<T> {
    _marker: PhantomData<T>,
}

// Manual implementations to avoid requiring T: Clone/Copy
impl<T> Clone for HasMany<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for HasMany<T> {}

impl<T> Default for HasMany<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T> HasMany<T> {
    /// Creates a new HasMany marker.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_many_default() {
        let _: HasMany<String> = HasMany::default();
    }

    #[test]
    fn test_has_many_new() {
        let _: HasMany<String> = HasMany::new();
    }

    #[test]
    fn test_has_many_is_zst() {
        assert_eq!(std::mem::size_of::<HasMany<String>>(), 0);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_has_many_clone() {
        let original: HasMany<String> = HasMany::new();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_has_many_copy() {
        let original: HasMany<String> = HasMany::new();
        let copied = original; // Copy semantics
        assert_eq!(original, copied); // original still usable after copy
    }
}
