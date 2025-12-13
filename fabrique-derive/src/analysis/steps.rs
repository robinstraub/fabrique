//! Analysis builder using typestate pattern for procedural macro input
//! validation.
//!
//! # Workflow
//!
//! ```mermaid
//! graph TD
//!     Input -->|parse_struct| ParsedStruct
//!     ParsedStruct -->|parse_fields| ParsedFields
//!     ParsedFields -->|build| Analysis
//! ```

use darling::{FromDeriveInput, FromField};
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed, Ident, spanned::Spanned};

use crate::{
    analysis::{
        Analysis,
        ast::{Model, ModelAttrs, ModelField, ModelFieldAttrs},
    },
    error::{Error, ErrorKind},
};

/// Entry point for analysis. Wraps the derive input.
pub struct Input<'a> {
    input: &'a DeriveInput,
}

/// Validated struct data.
pub struct ParsedStruct<'a> {
    data: &'a DataStruct,
    ident: &'a Ident,
    model: Model,
}

/// Validated and parsed fields.
pub struct ParsedFields<'a> {
    fields: Vec<ModelField>,
    ident: &'a Ident,
    model: Model,
}

impl<'a> Input<'a> {
    /// Creates a new input from a derive input.
    pub fn new(input: &'a DeriveInput) -> Self {
        Self { input }
    }

    /// Validates that the input is a struct.
    pub fn parse_struct(self) -> Result<ParsedStruct<'a>, Error> {
        let data = match &self.input.data {
            Data::Struct(data) => data,
            Data::Enum(_) => {
                return Err(Error::new(
                    self.input.ident.span(),
                    ErrorKind::UnsupportedDataStructureEnum,
                ));
            }
            Data::Union(_) => {
                return Err(Error::new(
                    self.input.ident.span(),
                    ErrorKind::UnsupportedDataStructureUnion,
                ));
            }
        };

        let attrs = ModelAttrs::from_derive_input(self.input)
            .map_err(|e| Error::from_darling(e, self.input.span()))?;
        let model = Model::new(&self.input.ident, attrs);

        Ok(ParsedStruct::new(self, data, model))
    }
}

impl<'a> ParsedStruct<'a> {
    pub fn new(previous_step: Input<'a>, data: &'a DataStruct, model: Model) -> Self {
        Self {
            data,
            ident: &previous_step.input.ident,
            model,
        }
    }

    /// Parses and validates named fields.
    pub fn parse_fields(self) -> Result<ParsedFields<'a>, Error> {
        // Extract fields from the structure
        let fields = match &self.data.fields {
            Fields::Named(FieldsNamed { named, .. }) => named,
            Fields::Unit => {
                return Err(Error::new(
                    self.ident.span(),
                    ErrorKind::UnsupportedDataStructureUnitStruct,
                ));
            }
            Fields::Unnamed(fields) => {
                return Err(Error::new(
                    fields.span(),
                    ErrorKind::UnsupportedDataStructureTupleStruct,
                ));
            }
        };

        // Transform `syn::Field` into `ast::ModelField`
        let mut fields = fields
            .iter()
            .map(|field| {
                let attrs = ModelFieldAttrs::from_field(field)
                    .map_err(|e| Error::from_darling(e, field.span()))?;
                ModelField::try_from(attrs)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Ensure manual primary keys are defined or attempt to infer auto primary keys
        if !fields.iter().any(|field| field.primary_key) {
            match fields.iter().position(|field| field.ident == "id") {
                Some(index) => fields[index].primary_key = true,
                None => Err(Error::new(self.ident.span(), ErrorKind::MissingPrimaryKey))?,
            }
        }

        Ok(ParsedFields::new(self, fields))
    }
}

impl<'a> ParsedFields<'a> {
    pub fn new(previous_step: ParsedStruct<'a>, fields: Vec<ModelField>) -> Self {
        Self {
            ident: previous_step.ident,
            fields,
            model: previous_step.model,
        }
    }

    /// Builds the final analysis.
    pub fn build(self) -> Result<Analysis<'a>, Error> {
        let returning = self
            .fields
            .iter()
            .map(|fields| fields.column.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        Ok(Analysis {
            returning,
            fields: self.fields,
            ident: self.ident,
            model: self.model,
        })
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
        let analysis = Input::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsing_an_enum_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { enum Anvil {} };
        let analysis = Input::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_union_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { union Anvil {} };
        let analysis = Input::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_named_struct_works() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil { id: u32 } };
        let analysis = Input::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct().unwrap().parse_fields();

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsing_a_unit_struct_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil; };
        let analysis = Input::new(&input);

        // Act the call to the fields method
        let result = analysis.parse_struct().unwrap().parse_fields();

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_parsing_a_tuple_struct_fails_explicitly() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil(u32, u32); };
        let analysis = Input::new(&input);

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
    fn test_analysis_fails_explicitly_on_missing_primary_key() {
        // Arrange the analysis
        let input = parse_quote! { struct Anvil { name: String }  };
        let analysis = Analysis::from(&input);

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
        assert_eq!(analysis.model.table_name, "anvils");
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
        assert_eq!(analysis.model.table_name, "custom_anvils");
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
