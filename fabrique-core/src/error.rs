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
