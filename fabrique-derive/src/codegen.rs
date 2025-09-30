use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::analysis::{FactoryAnalysis, FactoryAnalysisOutput};

/// Code generator for factory struct implementations.
pub struct FactoryCodegen {
    /// Analysis output containing fields and relations
    analysis: FactoryAnalysisOutput,
    /// Original derive input for span information
    input: DeriveInput,
}

impl FactoryCodegen {
    /// Creates a code generator from the given derive input.
    pub fn from(input: DeriveInput) -> Self {
        let output = FactoryAnalysis::from(input.clone()).analyze().unwrap();
        Self {
            analysis: output,
            input,
        }
    }

    /// Generates the complete factory implementation as a token stream.
    pub fn generate_factory(self) -> TokenStream {
        let base_struct_ident = &self.analysis.base_struct_ident;
        let factory_ident = self.generate_factory_ident();
        let factory_fields = self.generate_factory_fields();
        let factory_method_create = self.generate_factory_method_create();
        let factory_method_new = self.generate_factory_method_new();
        let factory_method_fields = self.generate_factory_method_fields();
        let factory_methods_for_relation = self.generate_factory_methods_for_relation();
        let factory_relation_fields = self.generate_factory_relation_fields();

        quote! {
            impl #base_struct_ident {
                pub fn factory() -> #factory_ident {
                    #factory_ident::new()
                }
            }

            pub struct #factory_ident {
                #(#factory_fields,)*
                #(#factory_relation_fields,)*
            }

            impl #factory_ident {
                #factory_method_new

                #factory_method_create

                #(#factory_method_fields)*

                #(#factory_methods_for_relation)*
            }
        }
    }

    /// Generates field definitions for the factory struct.
    ///
    /// Transforms each field into an Option so users can either set specific values
    /// or let the factory generate defaults when building the final struct.
    fn generate_factory_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.fields.clone().into_iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;
            quote! {
                #name: std::option::Option<#ty>
            }
        })
    }

    /// Generates factory relation fields for linked factory dependencies.
    fn generate_factory_relation_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.relations.iter().map(|relation| {
            let ident = &relation.ident;
            let ty = &relation.ty;
            quote! {
                #ident: std::option::Option<Box<dyn FnOnce(#ty) -> #ty + Send>>

            }
        })
    }

    /// Generates the factory identifier with "Factory" suffix.
    fn generate_factory_ident(&self) -> Ident {
        let factory_name = format!("{}Factory", &self.input.ident);
        Ident::new(&factory_name, self.input.ident.span())
    }

    /// Generates the `create()` method for the factory struct.
    ///
    /// This method handles both relation creation and object persistence:
    /// 1. Creates any related objects first (via factory relations)
    /// 2. Creates the main object with all field values
    /// 3. Persists the object using the Persistable trait
    fn generate_factory_method_create(&self) -> TokenStream {
        // Generate relation creation code - related objects are created first
        // to establish the dependency graph before creating the main object
        let relations_create = self.analysis.relations.iter().map(|relation| {
            let field = &relation.field.ident;
            let ident = &relation.ident;
            let ty = &relation.ty;

            quote! {
                if let Some(callback) = self.#ident {
                    let instance = callback(#ty::new()).create(connection).await?;
                    self.#field = Some(instance.id);
                }
            }
        });

        // Generate struct field initialization - use provided values or defaults
        let struct_ident = &self.analysis.base_struct_ident;
        let struct_fields = self.analysis.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote! {
                #name: self.#name.unwrap_or(<#ty as Default>::default())
            }
        });

        quote! {
            pub async fn create(mut self, connection: &<#struct_ident as fabrique::Persistable>::Connection) -> Result<#struct_ident, <#struct_ident as fabrique::Persistable>::Error>
            {
                #(#relations_create)*

                let instance = #struct_ident {
                    #(#struct_fields,)*
                };

                instance.create(connection).await
            }
        }
    }

    /// Generates the `new()` method for the factory struct.
    fn generate_factory_method_new(&self) -> TokenStream {
        let initialized_fields = self.analysis.fields.clone().into_iter().map(|field| {
            let name = &field.ident;
            quote! {
                #name: None
            }
        });

        let initialized_relation_fields = self.analysis.relations.iter().map(|relation| {
            let name = &relation.ident;
            quote! {
                #name: None
            }
        });

        quote! {
            pub fn new() -> Self {
                Self {
                    #(#initialized_fields,)*
                    #(#initialized_relation_fields,)*
                }
            }
        }
    }

    fn generate_factory_method_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.fields.clone().into_iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote! {
                pub fn #name(mut self, #name: #ty) -> Self {
                    self.#name = Some(#name);
                    self
                }
            }
        })
    }

    /// Generates the `for_[relation]` methods for the factory struct.
    ///
    /// These methods allow buffering the creation of related factory instances,
    /// which are then executed when building the final object.
    fn generate_factory_methods_for_relation(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.relations.iter().map(|relation| {
            let ty = &relation.ty;
            let method_name = Ident::new(&format!("for_{}", &relation.name), relation.ident.span());
            let field_ident = &relation.ident;
            quote! {
                pub fn #method_name<F>(mut self, callback: F) -> Self
                where F: FnOnce(#ty) -> #ty + Send + 'static
                {
                    self.#field_ident = Some(Box::new(callback));
                    self
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_generate_factory() {
        // Arrange the codegen
        let codegen = FactoryCodegen::from(parse_quote! {
            struct Anvil {
                #[factory(relation = "HammerFactory")]
                hammer_id: u32,
                hardness: u32,
                weight: u32,
            }
        });

        // Act the call to the factory ident method
        let generated = codegen.generate_factory();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                impl Anvil {
                    pub fn factory() -> AnvilFactory {
                        AnvilFactory::new()
                    }
                }
                pub struct AnvilFactory {
                    hammer_id: std::option::Option<u32>,
                    hardness: std::option::Option<u32>,
                    weight: std::option::Option<u32>,

                    hammer_factory: std::option::Option<Box<dyn FnOnce(HammerFactory) -> HammerFactory + Send>>,
                }

                impl AnvilFactory {
                    pub fn new() -> Self {
                        Self {
                            hammer_id: None,
                            hardness: None,
                            weight: None,
                            hammer_factory: None,
                        }
                    }

                    pub async fn create(mut self, connection: &<Anvil as fabrique::Persistable>::Connection) -> Result<Anvil, <Anvil as fabrique::Persistable>::Error> {
                        if let Some(callback) = self.hammer_factory {
                            let instance = callback(HammerFactory::new()).create(connection).await?;
                            self.hammer_id = Some(instance.id);
                        }

                        let instance = Anvil {
                            hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                            hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                            weight: self.weight.unwrap_or(<u32 as Default>::default()),
                        };
                        instance.create(connection).await
                    }

                    pub fn hammer_id(mut self, hammer_id: u32) -> Self {
                        self.hammer_id = Some(hammer_id);
                        self
                    }

                    pub fn hardness(mut self, hardness: u32) -> Self {
                        self.hardness = Some(hardness);
                        self
                    }

                    pub fn weight(mut self, weight: u32) -> Self {
                        self.weight = Some(weight);
                        self
                    }

                    pub fn for_hammer<F>(mut self, callback: F) -> Self
                    where F: FnOnce(HammerFactory) -> HammerFactory + Send + 'static
                    {
                        self.hammer_factory = Some(Box::new(callback));
                        self
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_fields() {
        // Arrange the codegen
        let codegen = FactoryCodegen::from(parse_quote! {
            struct Anvil {
                weight: u32,
            }
        });

        // Act the call to the codegen fields method
        let generated: Vec<TokenStream> = codegen.generate_factory_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! { weight: std::option::Option<u32> }.to_string()
        );
    }

    #[test]
    fn test_generate_factory_relation_fields() {
        // Arrange the codegen
        let codegen = FactoryCodegen::from(parse_quote! {
            struct Dynamite {
                #[factory(relation = "ExplosiveFactory")]
                explosive_id: String,
            }
        });

        // Act the call to the codegen fields method
        let generated: Vec<TokenStream> = codegen.generate_factory_relation_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                explosive_factory: std::option::Option<Box<dyn FnOnce(ExplosiveFactory) -> ExplosiveFactory + Send>>
            }.to_string()
        );
    }

    #[test]
    fn test_generate_factory_ident() {
        // Arrange the codegen
        let factory = FactoryCodegen::from(parse_quote! {
            struct Anvil {}
        });

        // Act the call to the factory ident method
        let generated = factory.generate_factory_ident();

        // Assert the result
        assert_eq!(&generated, "AnvilFactory");
    }

    #[test]
    fn test_generate_factory_method_create() {
        // Arrange the codegen
        let factory = FactoryCodegen::from(parse_quote! {
            struct Anvil {
                #[factory(relation = "HammerFactory")]
                hammer_id: u32,
                hardness: u32,
                weight: u32,
            }
        });

        // Act the call to the factory ident method
        let generated = factory.generate_factory_method_create();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                pub async fn create(mut self, connection: &<Anvil as fabrique::Persistable>::Connection) -> Result<Anvil, <Anvil as fabrique::Persistable>::Error> {
                    if let Some(callback) = self.hammer_factory {
                        let instance = callback(HammerFactory::new()).create(connection).await?;
                        self.hammer_id = Some(instance.id);
                    }

                    let instance = Anvil {
                        hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                        hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                        weight: self.weight.unwrap_or(<u32 as Default>::default()),
                    };
                    instance.create(connection).await
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_new() {
        // Arrange the codegen
        let factory = FactoryCodegen::from(parse_quote! {
            struct Anvil {
                hardness: u32,
                weight: u32,
            }
        });

        // Act the call to the factory ident method
        let generated = factory.generate_factory_method_new();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                pub fn new() -> Self {
                    Self {
                        hardness: None,
                        weight: None,
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_fields() {
        // Arrange the codegen
        let factory = FactoryCodegen::from(parse_quote! {
            struct Anvil {
                hardness: u32,
                weight: u32,
            }
        });

        // Act the call to the generate_factory_method_fields method
        let generated: Vec<TokenStream> = factory.generate_factory_method_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn hardness(mut self, hardness: u32) -> Self {
                    self.hardness = Some(hardness);
                    self
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_methods_for_relation() {
        // Arrange the codegen
        let factory = FactoryCodegen::from(parse_quote! {
            struct Dynamite {
                #[factory(relation = "ExplosiveFactory")]
                explosive_id: String,
            }
        });

        // Act the call to the generate_factory_method_fields method
        let generated: Vec<TokenStream> = factory.generate_factory_methods_for_relation().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn for_explosive<F>(mut self, callback: F) -> Self
                where F: FnOnce(ExplosiveFactory) -> ExplosiveFactory + Send + 'static
                {
                    self.explosive_factory = Some(Box::new(callback));
                    self
                }
            }
            .to_string()
        );
    }
}
