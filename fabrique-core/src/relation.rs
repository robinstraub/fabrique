//! Relationship types for model associations.
//!
//! This module provides marker types for defining relationships between models.

use std::marker::PhantomData;

/// Marker type for one-to-many relationships.
///
/// `HasMany<T>` is a zero-sized type (ZST) that represents a one-to-many
/// relationship where the current model is the "parent" and `T` is the
/// "child" model type.
///
/// This type is NOT stored in the database. It's purely a compile-time
/// marker used by the derive macros to generate navigation methods and
/// factory helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HasMany<T> {
    _marker: PhantomData<T>,
}

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
    fn test_has_many_clone() {
        let original: HasMany<String> = HasMany::new();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
