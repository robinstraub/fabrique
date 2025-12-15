use crate::database::{ColumnMarker, Database};

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
    ///
    /// This implementation accepts panics for invalid input as the expected use
    /// case involves developer-written compile-time string literals that
    /// will be validated by tests. Result-based error handling would
    /// require propagating errors through the builder's state machine,
    /// adding significant complexity.
    ///
    /// If runtime validation of user input becomes necessary, `TryFrom` can be
    /// added alongside this implementation without breaking the public API,
    /// with errors surfacing at query execution (`.await`).
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

/// Sort direction for `ORDER BY` clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Ascending order (smallest to largest)
    Asc,
    /// Descending order (largest to smallest)
    Desc,
}

impl Direction {
    /// Returns the SQL keyword for this direction.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Direction::Asc => "ASC",
            Direction::Desc => "DESC",
        }
    }
}

impl From<&'static str> for Direction {
    /// Converts a string literal to a direction.
    ///
    /// # Panics
    ///
    /// Panics if the string is not a recognized direction.
    ///
    /// This implementation accepts panics for invalid input as the expected use
    /// case involves developer-written compile-time string literals that
    /// will be validated by tests. Result-based error handling would
    /// require propagating errors through the builder's state machine,
    /// adding significant complexity.
    ///
    /// If runtime validation of user input becomes necessary, `TryFrom` can be
    /// added alongside this implementation without breaking the public API,
    /// with errors surfacing at query execution (`.await`).
    fn from(s: &'static str) -> Self {
        match s.to_uppercase().as_str() {
            "ASC" => Direction::Asc,
            "DESC" => Direction::Desc,
            _ => panic!(
                "Unknown direction: '{}'. Valid directions are: ASC, DESC",
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
pub trait QueryBuilder: Database {
    /// The model type that this query builder queries
    type Model;

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

    #[test]
    fn test_direction_as_str() {
        assert_eq!(Direction::Asc.as_str(), "ASC");
        assert_eq!(Direction::Desc.as_str(), "DESC");
    }

    #[test]
    fn test_direction_from_str() {
        assert_eq!(Direction::from("ASC"), Direction::Asc);
        assert_eq!(Direction::from("asc"), Direction::Asc);
        assert_eq!(Direction::from("DESC"), Direction::Desc);
        assert_eq!(Direction::from("desc"), Direction::Desc);
    }

    #[test]
    #[should_panic(expected = "Unknown direction")]
    fn test_direction_from_str_invalid() {
        let _ = Direction::from("INVALID");
    }
}
