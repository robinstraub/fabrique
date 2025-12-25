//! Error types for Fabrique database operations.
//!
//! This module provides a database-agnostic error type that doesn't expose
//! implementation details of the underlying database driver.

use std::fmt;

/// The direction of a type conversion that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionDirection {
    /// Converting a Rust value to a database value (during create/save).
    ToDb,
    /// Converting a database value to a Rust value (during read).
    FromDb,
}

impl fmt::Display for ConversionDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionDirection::ToDb => write!(f, "to database"),
            ConversionDirection::FromDb => write!(f, "from database"),
        }
    }
}

/// Errors that can occur during Fabrique database operations.
///
/// This error type is intentionally agnostic of the underlying database driver,
/// providing a stable API that doesn't expose implementation details.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested entity was not found in the database.
    #[error("entity not found")]
    NotFound,

    /// Failed to convert a value between Rust and database types.
    #[error("failed to convert field '{field}' from {from} to {to} {direction}: {reason} (value: {value})")]
    Conversion {
        /// The name of the field that failed to convert.
        field: String,
        /// The source type name.
        from: &'static str,
        /// The target type name.
        to: &'static str,
        /// String representation of the value that failed to convert.
        value: String,
        /// The reason for the conversion failure.
        reason: String,
        /// The direction of the conversion.
        direction: ConversionDirection,
    },

    /// Other database errors (connection, query syntax, constraints, etc.).
    ///
    /// This variant may be split into more specific variants in future versions.
    #[error("database error: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Error::NotFound,
            sqlx::Error::Decode(boxed) => {
                // Try to downcast to our Error type (used for conversion errors in FromRow)
                match boxed.downcast::<Error>() {
                    Ok(our_error) => *our_error,
                    Err(other) => Error::Other(other),
                }
            }
            other => Error::Other(Box::new(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_direction_display() {
        assert_eq!(ConversionDirection::ToDb.to_string(), "to database");
        assert_eq!(ConversionDirection::FromDb.to_string(), "from database");
    }

    #[test]
    fn error_display_not_found() {
        assert_eq!(Error::NotFound.to_string(), "entity not found");
    }

    #[test]
    fn error_display_conversion() {
        let error = Error::Conversion {
            field: "material".into(),
            from: "String",
            to: "Material",
            value: "INVALID".into(),
            reason: "unknown variant".into(),
            direction: ConversionDirection::ToDb,
        };
        assert_eq!(
            error.to_string(),
            "failed to convert field 'material' from String to Material to database: unknown variant (value: INVALID)"
        );
    }

    #[test]
    fn error_display_other() {
        let error = Error::Other(Box::new(std::io::Error::other("test")));
        assert_eq!(error.to_string(), "database error: test");
    }

    #[test]
    fn from_sqlx_row_not_found() {
        let error: Error = sqlx::Error::RowNotFound.into();
        assert!(matches!(error, Error::NotFound));
    }

    #[test]
    fn from_sqlx_decode_with_fabrique_error() {
        let fabrique_error = Error::Conversion {
            field: "f".into(),
            from: "A",
            to: "B",
            value: "v".into(),
            reason: "r".into(),
            direction: ConversionDirection::FromDb,
        };
        let sqlx_error = sqlx::Error::Decode(Box::new(fabrique_error));

        let error: Error = sqlx_error.into();

        assert!(matches!(error, Error::Conversion { field, .. } if field == "f"));
    }

    #[test]
    fn from_sqlx_decode_with_other_error() {
        let other_error = std::io::Error::other("decode failed");
        let sqlx_error = sqlx::Error::Decode(Box::new(other_error));

        let error: Error = sqlx_error.into();

        assert!(matches!(error, Error::Other(_)));
    }

    #[test]
    fn from_sqlx_other_error() {
        let sqlx_error = sqlx::Error::WorkerCrashed;

        let error: Error = sqlx_error.into();

        assert!(matches!(error, Error::Other(_)));
    }
}
