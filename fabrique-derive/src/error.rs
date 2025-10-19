use proc_macro2::Span;

/// Errors that can occur during factory derivation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    UnparsableAttribute(#[from] darling::Error),

    #[error("Expected a literal str, got {0:?}")]
    UnparsableLiteral(String),

    #[error("Could not parse literal to an ident: {0}")]
    UnparsableType(String),

    #[error("Factory can only be derived from named structs, enum given")]
    UnsupportedDataStructureEnum,

    #[error("Factory can only be derived from named structs, tuple struct given")]
    UnsupportedDataStructureTupleStruct,

    #[error("Factory can only be derived from named structs, union given")]
    UnsupportedDataStructureUnion,

    #[error("Factory can only be derived from named structs, unit struct given")]
    UnsupportedDataStructureUnitStruct,

    #[error("Missing `referenced_key` attribute for relation {0}")]
    MissingReferencedKey(String),

    #[error("{0}")]
    Darling(#[from] darling::Error),
}

impl From<Error> for syn::Error {
    fn from(value: Error) -> Self {
        syn::Error::new(Span::call_site(), value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_converts_to_syn_error() {
        // Arrange an error
        let error = Error::UnsupportedDataStructureEnum;

        // Act - convert to syn::Error
        let syn_error: syn::Error = error.into();

        // Assert the error message
        assert_eq!(
            syn_error.to_string(),
            "Factory can only be derived from named structs, enum given"
        );
    }

    #[test]
    fn test_unparsable_literal_error_converts_to_syn_error() {
        // Arrange an error
        let error = Error::UnparsableLiteral("true".to_string());

        // Act - convert to syn::Error
        let syn_error: syn::Error = error.into();

        // Assert the error message contains the value
        assert!(syn_error.to_string().contains("true"));
    }

    #[test]
    fn test_unparsable_type_error_converts_to_syn_error() {
        // Arrange an error
        let error = Error::UnparsableType("Not A Type".to_string());

        // Act - convert to syn::Error
        let syn_error: syn::Error = error.into();

        // Assert the error message contains the type
        assert!(syn_error.to_string().contains("Not A Type"));
    }
}
