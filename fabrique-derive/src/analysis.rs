use quote::ToTokens;
use syn::{
    Data, DataStruct, DeriveInput, Expr, ExprAssign, ExprLit, Field, Fields, FieldsNamed, Ident,
    Lit, LitBool, Meta, MetaNameValue, Token, punctuated::Punctuated, spanned::Spanned,
};

use crate::error::Error;

/// Analyzes a derive input to extract factory-related information.
///
/// Only supports structs with named fields.
pub struct FactoryAnalysis {
    input: DeriveInput,
}

#[derive(Default, Debug)]
pub struct FabriqueFieldAttributes {
    primary_key: bool,
    relation: Option<Ident>,
    referenced_key: Option<Ident>,
}

impl FabriqueFieldAttributes {
    pub fn from_field(field: &Field) -> Result<FabriqueFieldAttributes, Error> {
        let mut result = Self::default();

        for attribute in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("fabrique"))
        {
            match attribute.meta {
                Meta::NameValue(ref name_value) => {
                    result.parse_name_value(name_value)?;
                }
                Meta::List(ref list) => {
                    match list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
                        Ok(exprs) => {
                            for expr in exprs {
                                match expr {
                                    Expr::Assign(ExprAssign {
                                        left,
                                        eq_token,
                                        right,
                                        ..
                                    }) => match *left {
                                        Expr::Path(ref path) => {
                                            result.parse_name_value(&MetaNameValue {
                                                path: path.path.clone(),
                                                eq_token,
                                                value: *right,
                                            })?;
                                        }
                                        _ => {
                                            return Err(Error::UnparsableAttribute(
                                                attribute.to_token_stream().to_string(),
                                            ));
                                        }
                                    },
                                    Expr::Path(ref path) => {
                                        result.parse_name_value(&MetaNameValue {
                                            path: path.path.clone(),
                                            eq_token: Token![=](path.span()),
                                            value: Expr::Lit(ExprLit {
                                                lit: Lit::Bool(LitBool {
                                                    value: true,
                                                    span: path.span(),
                                                }),
                                                attrs: vec![],
                                            }),
                                        })?;
                                    }
                                    _ => {
                                        return Err(Error::UnparsableAttribute(
                                            attribute.to_token_stream().to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            return Err(Error::UnparsableAttribute(
                                attribute.to_token_stream().to_string(),
                            ));
                        }
                    }
                }
                Meta::Path(ref path) => {
                    println!("{path:?}");
                    return Err(Error::UnparsableAttribute(
                        attribute.to_token_stream().to_string(),
                    ));
                }
            }
        }

        Ok(result)
    }

    fn parse_name_value(&mut self, name_value: &MetaNameValue) -> Result<(), Error> {
        if name_value.path.is_ident("primary_key") {
            match name_value.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Bool(LitBool { value: true, .. }),
                    ..
                }) => self.primary_key = true,

                Expr::Lit(ExprLit {
                    lit: Lit::Str(ref str),
                    ..
                }) => {
                    if let Ok(primary_key) = str.value().parse::<bool>() {
                        self.primary_key = primary_key;
                    } else {
                        return Err(Error::UnparsableLiteral(str.value()));
                    }
                }
                _ => {
                    return Err(Error::UnparsableLiteral(
                        name_value.value.to_token_stream().to_string(),
                    ));
                }
            }
        } else if name_value.path.is_ident("relation") {
            match name_value.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(ref str),
                    ..
                }) => {
                    let mut ident = syn::parse_str::<Ident>(&str.value())
                        .map_err(|_| Error::UnparsableType(str.value()))?;
                    ident.set_span(str.span());
                    self.relation = Some(ident);
                }
                Expr::Path(ref path) => self.relation = path.path.get_ident().cloned(),
                _ => {
                    return Err(Error::UnparsableType(
                        name_value.value.to_token_stream().to_string(),
                    ));
                }
            }
        } else if name_value.path.is_ident("referenced_key") {
            match name_value.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(ref str),
                    ..
                }) => {
                    self.referenced_key = Some(Ident::new(&str.value(), str.span()));
                }
                Expr::Path(ref path) => self.referenced_key = path.path.get_ident().cloned(),
                _ => {
                    return Err(Error::UnparsableType(
                        name_value.value.to_token_stream().to_string(),
                    ));
                }
            }
        } else {
            return Err(Error::UnknownAttribute(
                name_value
                    .path
                    .get_ident()
                    .map(|ident| ident.to_string())
                    .unwrap_or("".to_string()),
            ));
        }

        Ok(())
    }
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
            fields: self.fields()?,
        })
    }

    /// Returns the fields of a named struct.
    ///
    /// # Errors
    ///
    /// Returns an error for enums, unions, unit structs, or tuple structs.
    fn fields(&self) -> Result<Vec<FactoryFieldAnalysisOutput>, Error> {
        let fields = match &self.input.data {
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
        }?;

        fields
            .into_iter()
            .map(|field| -> Result<FactoryFieldAnalysisOutput, Error> {
                let attributes = FabriqueFieldAttributes::from_field(field)?;

                Ok(FactoryFieldAnalysisOutput {
                    field: field.clone(),
                    primary_key: attributes.primary_key,
                    relation: Relation::new(field, attributes)?,
                })
            })
            .collect::<Result<Vec<FactoryFieldAnalysisOutput>, Error>>()
    }
}

/// Output of factory analysis containing extracted fields and relations.
#[derive(Debug)]
pub struct FactoryAnalysisOutput {
    /// The identifier of the original struct
    pub base_struct_ident: Ident,
    /// All named fields from the struct
    pub fields: Vec<FactoryFieldAnalysisOutput>,
}

impl FactoryAnalysisOutput {
    pub fn relations(&self) -> impl Iterator<Item = (&Field, &Relation)> {
        self.fields.iter().filter_map(|field| {
            field
                .relation
                .as_ref()
                .map(|relation| (&field.field, relation))
        })
    }
}

#[derive(Debug, Clone)]
pub struct FactoryFieldAnalysisOutput {
    pub field: Field,
    #[allow(dead_code)]
    pub primary_key: bool,
    pub relation: Option<Relation>,
}

/// Represents a factory relation extracted from struct field attributes.
#[derive(Debug, Clone)]
pub struct Relation {
    /// The identifier for the factory field (e.g., `anvil_factory`)
    pub factory_field: Ident,
    /// The type of the referenced object (e.g., `Anvil`)
    pub referenced_type: Ident,
    /// The field of the referenced object referenced by this relation (e.g. `id`)
    pub referenced_key: Ident,
    /// The base name of the relation (e.g., `anvil`)
    pub name: String,
}

impl Relation {
    /// Creates a new relation from a field and its factory type.
    ///
    /// Automatically derives the relation name by stripping the `_id` suffix
    /// from the field name if present.
    pub fn new(field: &Field, attributes: FabriqueFieldAttributes) -> Result<Option<Self>, Error> {
        if attributes.relation.is_none() {
            return Ok(None);
        }

        let referenced_type = attributes.relation.unwrap();

        let field = field.clone();

        let field_name = field
            .ident
            .as_ref()
            .ok_or(Error::UnsupportedDataStructureTupleStruct)?
            .to_string();

        let referenced_key = attributes
            .referenced_key
            .or(field_name
                .rsplit_once("_")
                .map(|(_, suffix)| Ident::new(suffix, field.span())))
            .ok_or(Error::MissingReferencedKey(field_name.clone()))?;

        let name = field_name
            .strip_suffix(&format!("_{}", referenced_key))
            .unwrap_or(&field_name)
            .to_owned();

        let ident = Ident::new(&format!("{}_factory", &name), field.span());

        Ok(Some(Self {
            factory_field: ident,
            referenced_type,
            referenced_key,
            name,
        }))
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
                #[fabrique(primary_key)]
                id: u32,
                weight: u32,
                #[fabrique(relation = "Hammer")]
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.base_struct_ident.to_string(), "Anvil");
        assert_eq!(result.fields.len(), 3);

        assert!(result.fields.iter().any(|field| {
            if field.field.ident.as_ref().unwrap() != "id" {
                return false;
            }

            assert!(field.primary_key);
            assert!(field.relation.is_none());

            true
        }));

        assert!(result.fields.iter().any(|field| {
            if field.field.ident.as_ref().unwrap() != "weight" {
                return false;
            }

            assert!(!field.primary_key);
            assert!(field.relation.is_none());

            true
        }));

        assert!(result.fields.iter().any(|field| {
            if field.field.ident.as_ref().unwrap() != "hammer_id" {
                return false;
            }

            assert!(!field.primary_key);
            assert!(field.relation.is_some());
            let relation = field.relation.as_ref().unwrap();
            assert_eq!(relation.factory_field.to_string(), "hammer_factory");
            assert_eq!(relation.referenced_type.to_string(), "Hammer");
            assert_eq!(relation.referenced_key.to_string(), "id");
            assert_eq!(relation.name, "hammer");

            true
        }));
    }

    #[test]
    fn test_analyze_fails_explicitly_on_unknown_attribute() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(unknown = true)]
                weight: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnknownAttribute("unknown".to_owned())
        );
    }

    #[test]
    fn test_analyze_fails_explicitly_on_invalid_relation_attribute() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation=true)]
                weight: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.analyze();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::UnparsableType("true".to_owned())
        );
    }

    #[test]
    fn test_the_fields_method_handles_no_relations() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {}
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_the_fields_method_handles_some_relations() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation = "Hammer")]
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_the_fields_method_fails_explicitely_on_no_referenced_key() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation = "Hammer")]
                hammer: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::MissingReferencedKey("hammer".to_owned())
        );
    }

    #[test]
    fn test_the_fields_handles_implicit_referenced_key() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation = "Hammer")]
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.iter().any(|field| {
            if field.field.ident.as_ref().unwrap() != "hammer_id" {
                return false;
            }

            assert!(field.relation.is_some());
            let relation = field.relation.as_ref().unwrap();
            assert_eq!(relation.referenced_key.to_string(), "id");
            assert_eq!(relation.name, "hammer");

            true
        }));
    }

    #[test]
    fn test_the_fields_handles_explicit_referenced_key() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation = "Hammer", referenced_key = "id")]
                hammer: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.iter().any(|field| {
            if field.field.ident.as_ref().unwrap() != "hammer" {
                return false;
            }

            assert!(field.relation.is_some());
            let relation = field.relation.as_ref().unwrap();
            assert_eq!(relation.referenced_key.to_string(), "id");
            assert_eq!(relation.name, "hammer");

            true
        }));
    }

    #[test]
    fn test_the_fields_method_handles_different_annotations() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[foo]
                density: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .iter()
                .filter(|field| field.relation.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn test_a_relation_can_be_created() {
        // Arrange the relation
        let factory = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                hammer_id: u32,
            }
        });
        let field = &factory.fields().unwrap()[0];

        // Act the relation instantiation
        let result = Relation::new(
            &field.field,
            FabriqueFieldAttributes {
                relation: Some(Ident::new("Hammer", field.field.span())),
                referenced_key: Some(Ident::new("id", field.field.span())),
                ..Default::default()
            },
        );

        // Assert the result
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_field_attribute_parsing_fails_explicitly_on_invalid_referenced_type() {
        // Arrange the field
        let field = parse_quote! {
            #[fabrique(relation = "Not A Valid Type")]
            hammer_id: u32
        };

        // Act the field parsing
        let result = FabriqueFieldAttributes::from_field(&field);

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
        let result = Relation::new(
            &field,
            FabriqueFieldAttributes {
                relation: Some(Ident::new("Hammer", field.span())),
                referenced_key: Some(Ident::new("id", field.span())),
                ..Default::default()
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
