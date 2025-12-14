use std::marker::PhantomData;

/// Database configuration and types
pub trait Database: Sized {
    /// The connection type (e.g., PgPool, MySqlPool)
    type Connection: Clone + Sync;

    /// The error type for database operations
    type Error;
}

/// Marker type that associates a column name with its value type.
///
/// This type is used to provide compile-time type safety when building queries,
/// ensuring that the value passed to a where clause matches the expected type
/// for that column. Column markers are typically generated as constants by the
/// derive macro.
pub struct ColumnMarker<T> {
    /// The name of the database column
    pub name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> ColumnMarker<T> {
    /// Creates a new column marker with the specified name.
    ///
    /// This is a const function to allow generating column constants at compile
    /// time.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }
}
