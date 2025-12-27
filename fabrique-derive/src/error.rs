use proc_macro2::Span;
use thiserror::Error as ThisError;

/// Error with span information for precise diagnostics.
#[derive(Debug)]
pub struct Error {
    /// The span where the error occurred
    pub span: Span,
    /// The kind of error
    pub kind: ErrorKind,
}

impl Error {
    /// Create a new error with a span and kind
    pub fn new(span: Span, kind: ErrorKind) -> Self {
        Self { span, kind }
    }

    /// Create an error from a darling error with the given span
    pub fn from_darling(error: darling::Error, span: Span) -> Self {
        Self {
            span,
            kind: ErrorKind::UnparsableAttribute(error.to_string()),
        }
    }
}

/// The kind of error that occurred during factory derivation.
#[derive(Debug, ThisError)]
pub enum ErrorKind {
    #[error("{0}")]
    UnparsableAttribute(String),

    #[error("Factory can only be derived from named structs, enum given")]
    UnsupportedDataStructureEnum,

    #[error("Factory can only be derived from named structs, tuple struct given")]
    UnsupportedDataStructureTupleStruct,

    #[error("Factory can only be derived from named structs, union given")]
    UnsupportedDataStructureUnion,

    #[error("Factory can only be derived from named structs, unit struct given")]
    UnsupportedDataStructureUnitStruct,

    #[error("Missing primary key, either add an id column or mark an existing column as primary")]
    MissingPrimaryKey,

    #[error(
        "HasMany fields require a foreign_key attribute, e.g. #[fabrique(foreign_key = Order::CUSTOMER_ID)]"
    )]
    MissingForeignKeyAttribute,
}

impl From<Error> for syn::Error {
    fn from(error: Error) -> Self {
        syn::Error::new(error.span, error.kind.to_string())
    }
}
