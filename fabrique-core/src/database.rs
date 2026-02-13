#[cfg(any(feature = "postgres", feature = "sqlite", feature = "mysql"))]
pub use sqlx::Pool;

#[cfg(feature = "mysql")]
pub use sqlx::MySql;
#[cfg(feature = "postgres")]
pub use sqlx::Postgres;
#[cfg(feature = "sqlite")]
pub use sqlx::Sqlite;

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

    /// The database type used for SQL encoding.
    ///
    /// Equal to [`Type`](Self::Type) for plain fields, or the `as` type for
    /// fields with `#[fabrique(as = "...")]` type conversion.
    type DbType;

    /// The unqualified column name (e.g., "name").
    ///
    /// Use this for INSERT and UPDATE SET clauses.
    const NAME: &'static str;

    /// The qualified column name (e.g., "products.name").
    ///
    /// Use this for SELECT, WHERE, and JOIN clauses where table qualification
    /// is needed to avoid ambiguity.
    const QUALIFIED_NAME: &'static str;

    /// Converts a Rust value to its database representation.
    fn into_db(value: Self::Type) -> Self::DbType;

    /// Returns the unqualified column name.
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// Returns the qualified column name.
    fn qualified_name(&self) -> &'static str {
        Self::QUALIFIED_NAME
    }
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
    type DbType = Nil;

    const NAME: &'static str = panic!("Attempted to use Nil as a column");
    const QUALIFIED_NAME: &'static str = panic!("Attempted to use Nil as a column");

    fn into_db(value: Nil) -> Nil {
        value
    }

    fn name(&self) -> &'static str {
        panic!("Attempted to use Nil as a column")
    }

    fn qualified_name(&self) -> &'static str {
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
    #[should_panic(expected = "Attempted to use Nil as a column")]
    fn test_nil_column_qualified_name_panics() {
        let nil = Nil;
        let _ = Column::<()>::qualified_name(&nil);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    #[should_panic(expected = "Nil should never be used as a column type")]
    fn test_nil_type_info_panics() {
        let _ = <Nil as sqlx::Type<sqlx::Sqlite>>::type_info();
    }

    #[cfg(feature = "sqlite")]
    #[test]
    #[should_panic(expected = "Nil should never be encoded")]
    fn test_nil_encode_panics() {
        use sqlx::Encode;
        let nil = Nil;
        use std::mem::MaybeUninit;
        let mut storage: MaybeUninit<<sqlx::Sqlite as sqlx::Database>::ArgumentBuffer> =
            MaybeUninit::uninit();
        let buf_ref = unsafe { storage.assume_init_mut() };
        let _ = <Nil as Encode<sqlx::Sqlite>>::encode_by_ref(&nil, buf_ref);
    }

    #[test]
    fn test_nil_into_db_returns_nil() {
        let result = <Nil as Column<()>>::into_db(Nil);
        assert!(matches!(result, Nil));
    }
}
