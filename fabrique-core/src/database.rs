/// Database awareness for models.
///
/// This trait marks types that are aware of and connected to a database,
/// providing the database type and error handling. Models extend this trait
/// to add persistence operations.
pub trait DatabaseAware: Sized {
    /// The database connection type.
    type Database;

    /// The error type returned by database operations.
    type Error;
}

/// Represents a type-safe column from a specific model.
///
/// This trait is implemented by zero-sized types generated for each model
/// column, providing compile-time guarantees that columns belong to the correct
/// model and carry their value type information.
///
/// The derive macro generates one implementation per column, ensuring type
/// safety when building queries.
pub trait Column<M>: Sized {
    /// The Rust type of values stored in this column.
    type Type;

    /// Returns the database column name.
    fn name(&self) -> &'static str;
}

/// Implement Column for () to serve as a "null" column placeholder.
/// This allows the type system to work uniformly, though actually using it
/// will panic.
impl<M> Column<M> for () {
    type Type = ();

    fn name(&self) -> &'static str {
        panic!("Attempted to use () as a column")
    }
}
