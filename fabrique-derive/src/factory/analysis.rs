use darling::FromField;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident, spanned::Spanned};

use crate::error::Error;

/// Analyzes a derive input to extract factory-related information.
///
/// Only supports structs with named fields.
pub struct FactoryAnalysis {
    input: DeriveInput,
}

#[derive(FromField, Debug, Default, Clone)]
#[darling(attributes(fabrique))]
pub struct FabriqueFieldAttributes {
    #[darling(default)]
    primary_key: bool,

    #[darling(default)]
    relation: Option<Ident>,

    #[darling(default)]
    referenced_key: Option<Ident>,
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
    /// Automatically derives the relation name by stripping the `referenced_key` suffix
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
                #[fabrique(relation = "Hammer", referenced_key = "id")]
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

        assert!(
            result
                .fields
                .iter()
                .find(|field| field.field.ident.as_ref().unwrap() == "id")
                .map(|field| {
                    assert!(field.primary_key);
                    assert!(field.relation.is_none());

                    true
                })
                .unwrap_or(false)
        );

        assert!(
            result
                .fields
                .iter()
                .find(|field| field.field.ident.as_ref().unwrap() == "weight")
                .map(|field| {
                    assert!(!field.primary_key);
                    assert!(field.relation.is_none());

                    true
                })
                .unwrap_or(false)
        );

        assert!(
            result
                .fields
                .iter()
                .find(|field| field.field.ident.as_ref().unwrap() == "hammer_id")
                .map(|field| {
                    assert!(!field.primary_key);
                    assert!(field.relation.is_some());
                    let relation = field.relation.as_ref().unwrap();
                    assert_eq!(relation.factory_field.to_string(), "hammer_factory");
                    assert_eq!(relation.referenced_type.to_string(), "Hammer");
                    assert_eq!(relation.referenced_key.to_string(), "id");
                    assert_eq!(relation.name, "hammer");

                    true
                })
                .unwrap_or(false)
        );
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
                #[fabrique(relation = "Hammer", referenced_key = "id")]
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
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::MissingReferencedKey(rel) if rel == "hammer_id"
        ));
    }

    #[test]
    fn test_the_fields_handles_implicit_referenced_key() {
        // Arrange the analysis
        let analysis = FactoryAnalysis::from(parse_quote! {
            struct Anvil {
                #[fabrique(relation = "Hammer", referenced_key = "id")]
                hammer_id: u32,
            }
        });

        // Act the call to the analyze method
        let result = analysis.fields();

        // Assert the result
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(
            result
                .iter()
                .find(|field| field.field.ident.as_ref().unwrap() == "hammer_id")
                .map(|field| {
                    assert!(field.relation.is_some());
                    let relation = field.relation.as_ref().unwrap();
                    assert_eq!(relation.referenced_key.to_string(), "id");
                    assert_eq!(relation.name, "hammer");

                    true
                })
                .unwrap_or(false)
        );
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
        assert!(
            result
                .iter()
                .find(|field| field.field.ident.as_ref().unwrap() == "hammer")
                .map(|field| {
                    assert!(field.relation.is_some());
                    let relation = field.relation.as_ref().unwrap();
                    assert_eq!(relation.referenced_key.to_string(), "id");
                    assert_eq!(relation.name, "hammer");

                    true
                })
                .unwrap_or(false)
        );
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
            #[fabrique(relation = "Not A Valid Type", referenced_key = "id")]
            hammer_id: u32
        };

        // Act the field parsing
        let result = FabriqueFieldAttributes::from_field(&field);

        // Assert the result
        assert!(result.is_err());
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
    }
}
