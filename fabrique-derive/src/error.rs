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

    #[error("Invalid faker expression: {0}")]
    InvalidFakerExpression(String),

    #[error("`alias` can only be used on `belongs_to` fields")]
    AliasWithoutRelation,

    #[error("Duplicate alias `{0}`: each alias name must be unique within a model")]
    DuplicateAlias(String),

    #[error("#[fabrique::test] expects `async fn test<DB: Dialect>(pool: Pool<DB>)`")]
    InvalidTestSignature,

    #[error("#[fabrique::test] requires at least one backend feature (sqlite, postgres, mysql)")]
    NoBackendFeature,

    #[error(
        "#[fabrique::test] requires exactly one backend feature, but multiple are active. Run tests separately per backend (e.g. `cargo test --features sqlite,testing`)"
    )]
    MultipleBackendFeatures,
}

impl From<Error> for syn::Error {
    fn from(error: Error) -> Self {
        syn::Error::new(error.span, error.kind.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_error_converts_to_syn_error() {
        // Arrange a crate error
        let error = Error::new(Span::call_site(), ErrorKind::MissingPrimaryKey);

        // Act the conversion
        let error: syn::Error = error.into();

        // Assert the syn error serialized value
        assert_eq!(
            error.to_string(),
            "Missing primary key, either add an id column or mark an existing column as primary"
        );
    }
}
