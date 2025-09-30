use quote::ToTokens;
use syn::{
    Data, DataStruct, DeriveInput, Expr, ExprLit, Field, Fields, FieldsNamed, Ident, Lit, LitStr,
    Meta, MetaNameValue, Token, punctuated::Punctuated, spanned::Spanned, token::Comma,
};

use crate::error::Error;

/// Analyzes a derive input to extract factory-related information.
///
/// Only supports structs with named fields.
pub struct FactoryAnalysis {
    input: DeriveInput,
}

/// The arguments supported by a factory relation.
pub struct FactoryRelationArgs {
    /// The relation type.
    ty: LitStr,
    /// The field to extract from the relation.
    extract: Option<String>,
}

impl FactoryAnalysis {
    /// Creates a new analysis from a derive input.
    pub fn from(input: DeriveInput) -> Self {
        Self { input }
    }

    /// Performs the analysis and returns the output.
    pub fn analyze(self) -> Result<FactoryAnalysisOutput, Error> {
        Ok(FactoryAnalysisOutput {
            base_struct_ident: self.input.ident.clone(),
            fields: self.fields()?.clone(),
            relations: self.relations()?,
        })
    }

    /// Returns the fields of a named struct.
    ///
    /// # Errors
    ///
    /// Returns an error for enums, unions, unit structs, or tuple structs.
    fn fields(&self) -> Result<&Punctuated<Field, Comma>, Error> {
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

    /// Extracts factory relations from field attributes.
    ///
    /// Relations allow linking factories together for creating instances with
    /// related dependencies, enabling cleaner bootstrapping of complex object graphs.
    fn relations(&self) -> Result<Vec<Relation>, Error> {
        self.fields()?
            .iter()
            .filter_map(|field| Self::parse_relation_from_field(field).transpose())
            .collect()
    }

    /// Parse a relation from a field
    fn parse_relation_from_field(field: &syn::Field) -> Result<Option<Relation>, Error> {
        for attr in &field.attrs {
            if !attr.path().is_ident("factory") {
                continue;
            }

            if let Meta::List(ref list) = attr.meta {
                let args = list
                    .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
                    .ok();

                if let Some(args) = args {
                    let args = Self::parse_relation_args(args)?;

                    if let Some(args) = args {
                        return Ok(Some(Relation::new(field, args)?));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Parse relation arguments
    fn parse_relation_args(
        args: Punctuated<MetaNameValue, Token![,]>,
    ) -> Result<Option<FactoryRelationArgs>, Error> {
        let mut ty = None;
        let mut extract = None;

        for arg in args {
            if let Some(value) = Self::extract_meta_value_literal(&arg, "relation")? {
                ty = Some(value)
            } else if let Some(value) = Self::extract_meta_value_literal(&arg, "extract")? {
                extract = Some(value.value())
            }
        }

        Ok(ty.map(|ty| FactoryRelationArgs { ty, extract }))
    }

    /// Extracts a literal from a MetaNameValue if the path matches.
    fn extract_meta_value_literal(
        name_value: &MetaNameValue,
        path: &str,
    ) -> Result<Option<LitStr>, Error> {
        if name_value.path.is_ident(path) {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(ref lit_str),
                ..
            }) = name_value.value
            {
                return Ok(Some(lit_str.clone()));
            } else {
                let value = name_value.value.to_token_stream().to_string();
                return Err(Error::UnparsableLiteral(value));
            }
        }

        Ok(None)
    }
}

/// Output of factory analysis containing extracted fields and relations.
#[derive(Debug)]
pub struct FactoryAnalysisOutput {
    /// The identifier of the original struct
    pub base_struct_ident: Ident,
    /// All named fields from the struct
    pub fields: Punctuated<Field, Comma>,
    /// Extracted factory relations from field attributes
    pub relations: Vec<Relation>,
}

/// Represents a factory relation extracted from struct field attributes.
#[derive(Debug)]
pub struct Relation {
    pub field: Field,
    /// The identifier for the factory field (e.g., `anvil_factory`)
    pub ident: Ident,
    /// The type of the related factory (e.g., `AnvilFactory`)
    pub ty: Ident,
    /// The base name of the relation (e.g., `anvil`)
    pub name: String,
    /// The extracted field from the relation (e.g., `id`)
    pub extract: Option<Ident>,
}

impl Relation {
    /// Creates a new relation from a field and its factory type.
    ///
    /// Automatically derives the relation name by stripping the extracted field suffix
    /// from the field name if present.
    pub fn new(
        field: &Field,
        FactoryRelationArgs { ty, extract }: FactoryRelationArgs,
    ) -> Result<Self, Error> {
        let field = field.clone();

        let field_name = field
            .ident
            .as_ref()
            .ok_or(Error::UnsupportedDataStructureTupleStruct)?
            .to_string();

        let name = if let Some(ref extract) = extract {
            field_name
                .strip_suffix(&format!("_{extract}"))
                .unwrap_or(&field_name)
                .to_owned()
        } else {
            field_name
        };

        let ident = Ident::new(&format!("{}_factory", &name), field.span());

        let ty = syn::parse_str(&ty.value()).map_err(|_| Error::UnparsableType(ty.value()))?;
        let extract = if let Some(extract) = extract {
            Some(syn::parse_str(&extract).map_err(|_| Error::UnparsableLiteral(extract))?)
        } else {
            None
        };

        Ok(Self {
            field,
            ident,
            name,
            ty,
            extract,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_analyze() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                weight: u32,
                #[factory(relation = "HammerFactory")]
                hammer: Hammer,
            }
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_fails_explicitly_on_fields() {
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
    fn test_analyze_fails_explicitly_on_relations() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(relation=true)]
                weight: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnparsableLiteral("true".to_owned())
        );
    }

    #[test]
    fn test_the_relations_method_handles_none() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {}
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_the_relations_method_handles_some() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(relation = "HammerFactory")]
                hammer: Hammer,
            }
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_the_relations_method_handles_extracted_field() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(relation = "HammerFactory", extract = "id")]
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].extract.is_some());
        let extract = result[0].extract.as_ref().unwrap();
        assert_eq!(extract.to_string(), "id");
        assert_eq!(result[0].name, "hammer");
    }

    #[test]
    fn test_the_relations_method_fails_explicitly_on_invalid_fields() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil;
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnsupportedDataStructureUnitStruct
        );
    }

    #[test]
    fn test_the_relations_method_fails_explicitly_on_invalid_type() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(relation = 1)]
                hammer: Hammer,
            }
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnparsableLiteral("1".to_owned())
        );
    }

    #[test]
    fn test_the_relations_method_handles_different_annotations() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(other = "foo")]
                hammer: Hammer,

                #[foo]
                density: u32,

                #[factory(unamed)]
                weight: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_the_relations_method_fails_explicitly_on_invalid_annotation() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[factory(relation = 1)]
                hammer: Hammer,
            }
        });

        // Act the call to the analyze method
        let result = analysis.relations();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnparsableLiteral("1".to_owned())
        );
    }

    #[test]
    fn test_a_relation_can_be_created() {
        // Arrange the relation
        let factory = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                weight: u32,
            }
        });
        let field = &factory.fields().unwrap()[0];

        // Act the relation instantiation
        let result = Relation::new(
            field,
            FactoryRelationArgs {
                ty: syn::parse_str("\"u32\"").unwrap(),
                extract: None,
            },
        );

        // Assert the result
        assert!(result.is_ok());
    }

    #[test]
    fn test_a_relation_creation_fails_explicitly_on_invalid_ty() {
        // Arrange the relation
        let factory = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                weight: u32,
            }
        });
        let field = &factory.fields().unwrap()[0];

        // Act the relation instantiation
        let literal = LitStr::new("Not A Valid Type", field.span());
        let result = Relation::new(
            field,
            FactoryRelationArgs {
                ty: literal,
                extract: None,
            },
        );

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnparsableType("Not A Valid Type".to_owned())
        );
    }

    #[test]
    fn test_a_relation_creation_fails_explicitly_on_unit_field() {
        // Arrange the relation
        let field: Field = parse_quote! {
            u32
        };

        // Act the relation instantiation
        let literal = LitStr::new("Not A Valid Type", field.span());
        let result = Relation::new(
            &field,
            FactoryRelationArgs {
                ty: literal,
                extract: None,
            },
        );

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnsupportedDataStructureTupleStruct,
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
