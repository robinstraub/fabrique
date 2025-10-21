use crate::error::Error;
use darling::FromDeriveInput;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident};

/// Initial builder state for derive input analysis.
pub struct AnalysisBuilder<'a> {
    input: &'a DeriveInput,
}

/// Analysis state containing validated struct data.
#[derive(Debug)]
pub struct ValidatedStruct<'a> {
    pub ident: &'a Ident,
    pub input: &'a DeriveInput,
    data: &'a DataStruct,
}

/// Analysis state containing validated named fields.
#[derive(Debug)]
pub struct ParsedFields<'a> {
    fields: &'a Punctuated<Field, Comma>,
    ident: &'a Ident,
    input: &'a DeriveInput,
}

/// Completed analysis containing parsed input and validated metadata.
#[derive(Debug)]
pub struct Analysis<'a> {
    /// Named fields of the analyzed struct.
    pub fields: &'a Punctuated<Field, Comma>,

    /// Identifier of the analyzed struct.
    #[allow(dead_code)]
    pub ident: &'a Ident,

    /// The table name for this model.
    #[allow(dead_code)]
    pub table_name: String,
}

#[derive(FromDeriveInput)]
#[darling(attributes(fabrique))]
pub struct FabriqueAttrs {
    /// The table name for this model
    #[darling(default)]
    pub table: Option<String>,
}

impl<'a> AnalysisBuilder<'a> {
    /// Constructs a new analysis builder from the given derive input.
    pub fn new(input: &'a DeriveInput) -> Self {
        Self { input }
    }

    /// Validates that the input is a struct and transitions to the next state.
    pub fn parse_struct(self) -> Result<ValidatedStruct<'a>, Error> {
        let data = match &self.input.data {
            Data::Struct(data) => Ok(data),
            Data::Enum(_) => Err(Error::UnsupportedDataStructureEnum),
            Data::Union(_) => Err(Error::UnsupportedDataStructureUnion),
        }?;

        Ok(ValidatedStruct::new(self.input, data))
    }
}

impl<'a> ValidatedStruct<'a> {
    /// Constructs a new ValidatedStruct struct.
    pub fn new(input: &'a DeriveInput, data: &'a DataStruct) -> Self {
        let ident = &input.ident;
        Self { ident, input, data }
    }

    /// Validates that input struct is composed of named fields and transistions to the next state.
    pub fn parse_fields(self) -> Result<ParsedFields<'a>, Error> {
        let fields = match &self.data.fields {
            Fields::Named(FieldsNamed { named, .. }) => Ok(named),
            Fields::Unit => Err(Error::UnsupportedDataStructureUnitStruct),
            Fields::Unnamed(_) => Err(Error::UnsupportedDataStructureTupleStruct),
        }?;

        Ok(ParsedFields::new(self, fields))
    }
}

impl<'a> ParsedFields<'a> {
    /// Constructs a new ParsedFields struct.
    pub fn new(prev: ValidatedStruct<'a>, fields: &'a Punctuated<Field, Comma>) -> Self {
        Self {
            ident: prev.ident,
            input: prev.input,
            fields,
        }
    }

    /// Transistions to the next state.
    pub fn validate(self) -> Result<Analysis<'a>, Error> {
        let table_name = FabriqueAttrs::from_derive_input(self.input)
            .map_err(Error::UnparsableAttribute)?
            .table
            .unwrap_or_else(|| format!("{}s", self.ident.to_string().to_lowercase()));

        let analysis = Analysis::new(self.fields, self.ident, table_name);

        Ok(analysis)
    }
}

impl<'a> Analysis<'a> {
    /// Constructs a new analysis.
    pub fn new(fields: &'a Punctuated<Field, Comma>, ident: &'a Ident, table_name: String) -> Self {
        Self {
            fields,
            ident,
            table_name,
        }
    }

    /// Performs complete analysis of the derive input.
    pub fn from(input: &'a DeriveInput) -> Result<Self, Error> {
        let analysis = AnalysisBuilder::new(input)
            .parse_struct()?
            .parse_fields()?
            .validate()?;

        Ok(analysis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parsing_a_struct_works() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil {} };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsing_an_enum_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { enum Anvil {} };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_union_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { union Anvil {} };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_named_struct_works() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil {} };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct().unwrap().parse_fields();
        println!("analysis: {:?}", &result);

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsing_a_unit_struct_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil; };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct().unwrap().parse_fields();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_tuple_struct_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil(u32, u32); };
        let analysis = AnalysisBuilder::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct().unwrap().parse_fields();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_analysis_fails_explicitly_on_invalid_struct() {
        // Arrange the analysis
        let input = parse_quote! { enum Anvil {} };
        let analysis = Analysis::from(&input);

        // Assert the result
        assert!(analysis.is_err());
    }

    #[test]
    fn test_analysis_fails_explicitly_on_invalid_fields() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil(u32, u32); };
        let analysis = Analysis::from(&input);

        // Assert the result
        assert!(analysis.is_err());
    }

    #[test]
    fn test_validate_with_default_table_name() {
        // Arrange the analysis without a custom table name
        let input = parse_quote! {
            struct Anvil {
                id: u32,
            }
        };

        // Act the call to the Analysis::from method
        let result = Analysis::from(&input);

        // Assert the result is ok and has the default table name
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.table_name, "anvils");
    }

    #[test]
    fn test_validate_with_custom_table_name() {
        // Arrange the analysis with a custom table name
        let input = parse_quote! {
            #[fabrique(table = "custom_anvils")]
            struct Anvil {
                id: u32,
            }
        };

        // Act the call to the Analysis::from method
        let result = Analysis::from(&input);

        // Assert the result is ok and has the custom table name
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.table_name, "custom_anvils");
    }

    #[test]
    fn test_validate_with_unknown_attribute_fails() {
        // Arrange the analysis with an unknown attribute field
        let input = parse_quote! {
            #[fabrique(unknown_field = "value")]
            struct Anvil {
                id: u32,
            }
        };

        // Act the call to the Analysis::from method
        let result = Analysis::from(&input);

        // Assert the result is an error from darling (unknown field)
        assert!(result.is_err());
    }
}
