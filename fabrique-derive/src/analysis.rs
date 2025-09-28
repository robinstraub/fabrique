use syn::{
    Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident, punctuated::Punctuated,
    token::Comma,
};

use crate::error::Error;

/// Analyzes a derive input to extract factory-related information.
///
/// Only supports structs with named fields.
pub struct FactoryAnalysis {
    input: DeriveInput,
}

impl FactoryAnalysis {
    /// Creates a new analysis from a derive input.
    pub fn from(input: DeriveInput) -> Self {
        Self { input }
    }

    /// Returns the fields of a named struct.
    ///
    /// # Errors
    ///
    /// Returns an error for enums, unions, unit structs, or tuple structs.
    pub fn fields(&self) -> Result<&Punctuated<Field, Comma>, Error> {
        match &self.input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(FieldsNamed { named, .. }),
                ..
            }) => Ok(named),
            Data::Struct(DataStruct {
                fields: Fields::Unit,
                ..
            }) => Err(Error::UnsupportedDataStructureUnitStruct),
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(_),
                ..
            }) => Err(Error::UnsupportedDataStructureTupleStruct),
            Data::Enum(_) => Err(Error::UnsupportedDataStructureEnum),
            Data::Union(_) => Err(Error::UnsupportedDataStructureUnion),
        }
    }

    /// Performs the analysis and returns the output.
    pub fn analyze(self) -> Result<FactoryAnalysisOutput, Error> {
        let output = FactoryAnalysisOutput {
            base_struct_ident: self.input.ident.clone(),
            fields: self.fields()?.clone(),
        };

        Ok(output)
    }
}

/// Output of factory analysis containing extracted fields.
#[derive(Debug)]
pub struct FactoryAnalysisOutput {
    pub base_struct_ident: Ident,
    pub fields: Punctuated<Field, Comma>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_analyze_fails_explicitly() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil;
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnsupportedDataStructureUnitStruct
        );
    }

    #[test]
    fn test_deriving_an_enum_fails_explicitly() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            enum Anvil {}
        });

        // Act the call to the fields method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::UnsupportedDataStructureEnum);
    }

    #[test]
    fn test_deriving_a_tuple_struct_fails_explicitly() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil(u32, u32);
        });

        // Act the call to the fields method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnsupportedDataStructureTupleStruct,
        );
    }

    #[test]
    fn test_deriving_a_unit_struct_fails_explicitly() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil;
        });

        // Act the call to the fields method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnsupportedDataStructureUnitStruct,
        );
    }

    #[test]
    fn test_deriving_a_union_fails_explicitly() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            union Anvil {}
        });

        // Act the call to the fields method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::UnsupportedDataStructureUnion);
    }
}
