use std::marker::PhantomData;

pub mod factory;

pub trait Model: Sized {
    /// The connection type used for database operations (e.g., database
    /// connection pool)
    type Connection: Clone + Sync;

    /// The error type returned by persistence operations
    type Error;

    /// The primary key type, which is either concrete value or a tuple for
    /// composite primary keys.
    type PrimaryKey: Send;

    /// Returns the primary key value of this model instance.
    fn primary_key(&self) -> Self::PrimaryKey;
}

/// Trait for objects that can be persisted to a database or storage backend.
///
/// This trait enables factories to create and persist objects using the
/// `create()` method. The trait is designed to be flexible, allowing different
/// connection types and error handling strategies based on your specific
/// database or persistence layer.
///
/// # Example
///
/// ```rust
/// use fabrique::Persistable;
/// use uuid::Uuid;
///
/// #[derive(Persistable)]
/// pub struct Anvil {
///     pub id: Uuid,
///     pub name: String,
///     pub weight: i16,
/// }
/// ```
pub trait Persistable: Model {
    /// The query builder type for constructing database queries
    type QueryBuilder: QueryBuilder;

    /// Creates a new query builder for this model.
    ///
    /// This method returns a query builder that allows chaining where clauses
    /// to construct type-safe database queries with compile-time column
    /// validation.
    fn query() -> Self::QueryBuilder;

    /// Creates and persists this model using the provided connection.
    ///
    /// This method should handle the actual database insertion or persistence
    /// logic and return the created object with any auto-generated fields
    /// (like IDs) populated.
    fn create(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;

    /// Destroy the model for the given ID.
    fn destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Deletes this model using the provided connection.
    fn delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Retrieves all instances of this model from the persistence layer
    ///
    /// This method should handle querying the persistence layer for all records
    /// and return them as a vector of model instances.
    fn all(
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Vec<Self>, Self::Error>> + Send;
}

pub trait HardDelete: Model {
    /// Permanently destroy the model with the given ID
    fn hard_destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Permanently delete this model instance
    fn hard_delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait SoftDelete: Model {
    /// Permanently delete this model instance, bypassing soft delete
    fn force_delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Soft destroy the model with the given ID
    fn soft_destroy(
        connection: &Self::Connection,
        id: Self::PrimaryKey,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Soft delete this model instance
    fn soft_delete(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Restore a soft-deleted model instance
    fn restore(
        &self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Determine if the model instance has been soft-deleted
    fn trashed(
        &self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
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

/// SQL comparison operators for query building.
///
/// This enum provides type-safe SQL operators that can be used in WHERE
/// clauses. It supports conversion from string literals for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// Equal to (=)
    Eq,
    /// Not equal to (!=)
    NotEq,
    /// Less than (<)
    Lt,
    /// Less than or equal to (<=)
    Lte,
    /// Greater than (>)
    Gt,
    /// Greater than or equal to (>=)
    Gte,
    /// LIKE pattern matching
    Like,
    /// NOT LIKE pattern matching
    NotLike,
}

impl Operator {
    /// Converts the operator to its SQL string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::NotEq => "!=",
            Operator::Lt => "<",
            Operator::Lte => "<=",
            Operator::Gt => ">",
            Operator::Gte => ">=",
            Operator::Like => "LIKE",
            Operator::NotLike => "NOT LIKE",
        }
    }
}

impl From<&'static str> for Operator {
    /// Converts a string literal to an operator.
    ///
    /// # Panics
    ///
    /// Panics if the string is not a recognized operator.
    fn from(s: &'static str) -> Self {
        match s {
            "=" => Operator::Eq,
            "!=" | "<>" => Operator::NotEq,
            "<" => Operator::Lt,
            "<=" => Operator::Lte,
            ">" => Operator::Gt,
            ">=" => Operator::Gte,
            "LIKE" | "like" => Operator::Like,
            "NOT LIKE" | "not like" => Operator::NotLike,
            _ => panic!(
                "Unknown operator: '{}'. Valid operators are: =, !=, <>, <, <=, >, >=, LIKE, NOT LIKE",
                s
            ),
        }
    }
}

/// Trait for building type-safe database queries.
///
/// Query builders enable constructing SQL queries with compile-time safety by
/// requiring column names to be static strings. This prevents dynamic column
/// name injection while providing a fluent, chainable API for building complex
/// queries.
pub trait QueryBuilder {
    /// The model type that this query builder queries
    type Model;

    /// The database executor type used to run queries
    type Connection;

    /// The error type returned from query operations
    type Error;

    /// Adds a WHERE clause to the query.
    ///
    /// This method appends a condition to the query using the specified column,
    /// operator, and value. Multiple where clauses can be chained together
    /// to build complex queries.
    ///
    /// The operator can be either an `Operator` enum variant or a string
    /// literal that will be converted to an operator at runtime.
    fn r#where<T, O>(self, column: ColumnMarker<T>, operator: O, value: T) -> Self
    where
        T: 'static + for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
        O: Into<Operator>;

    /// Executes the query and returns all matching rows.
    ///
    /// This method finalizes the query and executes it against the database,
    /// returning a vector of model instances that match the query criteria.
    fn fetch_all(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Vec<Self::Model>, Self::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_marker_new() {
        // Test ColumnMarker::new() in a runtime context to ensure coverage
        let column: ColumnMarker<String> = ColumnMarker::new("test_column");
        assert_eq!(column.name, "test_column");
    }

    #[test]
    fn test_operator_as_str() {
        assert_eq!(Operator::Eq.as_str(), "=");
        assert_eq!(Operator::NotEq.as_str(), "!=");
        assert_eq!(Operator::Lt.as_str(), "<");
        assert_eq!(Operator::Lte.as_str(), "<=");
        assert_eq!(Operator::Gt.as_str(), ">");
        assert_eq!(Operator::Gte.as_str(), ">=");
        assert_eq!(Operator::Like.as_str(), "LIKE");
        assert_eq!(Operator::NotLike.as_str(), "NOT LIKE");
    }

    #[test]
    fn test_operator_from_str() {
        assert_eq!(Operator::from("="), Operator::Eq);
        assert_eq!(Operator::from("!="), Operator::NotEq);
        assert_eq!(Operator::from("<>"), Operator::NotEq);
        assert_eq!(Operator::from("<"), Operator::Lt);
        assert_eq!(Operator::from("<="), Operator::Lte);
        assert_eq!(Operator::from(">"), Operator::Gt);
        assert_eq!(Operator::from(">="), Operator::Gte);
        assert_eq!(Operator::from("LIKE"), Operator::Like);
        assert_eq!(Operator::from("like"), Operator::Like);
        assert_eq!(Operator::from("NOT LIKE"), Operator::NotLike);
        assert_eq!(Operator::from("not like"), Operator::NotLike);
    }

    #[test]
    #[should_panic(expected = "Unknown operator")]
    fn test_operator_from_str_invalid() {
        let _ = Operator::from("INVALID");
    }
}
