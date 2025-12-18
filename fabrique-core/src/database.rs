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

/// A zero-sized type used as a placeholder column for models without soft
/// deletes. This type is never actually instantiated or used, but exists to
/// satisfy the type system's trait bounds.
#[derive(Debug, Clone, Copy)]
pub struct Nil;

/// Implement Column for Nil to serve as a placeholder for models without soft
/// deletes. This allows the type system to work uniformly. Actually attempting
/// to use this column will panic.
impl<M> Column<M> for Nil {
    type Type = Nil;

    fn name(&self) -> &'static str {
        panic!("Attempted to use Nil as a column")
    }
}

// Implement sqlx traits for Nil to satisfy trait bounds
// These implementations will never actually be called since models without
// soft deletes don't use the soft delete code path
impl<DB: sqlx::Database> sqlx::Type<DB> for Nil {
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        panic!("Nil should never be used as a column type")
    }
}

impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for Nil {
    fn encode_by_ref(
        &self,
        _buf: &mut <DB as sqlx::Database>::ArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        panic!("Nil should never be encoded")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Attempted to use Nil as a column")]
    fn test_nil_column_name_panics() {
        let nil = Nil;
        let _ = Column::<()>::name(&nil);
    }

    #[test]
    #[should_panic(expected = "Nil should never be used as a column type")]
    fn test_nil_type_info_panics() {
        let _ = <Nil as sqlx::Type<sqlx::Postgres>>::type_info();
    }
}
