use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::analysis::{FactoryAnalysis, FactoryAnalysisOutput};

/// Code generator for factory struct implementations.
pub struct FactoryCodegen {
    analysis: FactoryAnalysisOutput,
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
        let factory_method_new = self.generate_factory_method_new();
        quote! {
            impl #base_struct_ident {
                pub fn factory() -> #factory_ident {
                    #factory_ident::new()
                }
            }

            pub struct #factory_ident {
                #(#factory_fields,)*

            }

            impl #factory_ident {
                #factory_method_new
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

    /// Generates the factory identifier with "Factory" suffix.
    fn generate_factory_ident(&self) -> Ident {
        let factory_name = format!("{}Factory", &self.input.ident);
        Ident::new(&factory_name, self.input.ident.span())
    }

    /// Generates the `new()` method for the factory struct.
    fn generate_factory_method_new(&self) -> TokenStream {
        let initialized_fields = self.analysis.fields.clone().into_iter().map(|field| {
            let name = &field.ident;
            quote! {
                #name: None
            }
        });

        quote! {
            pub fn new() -> Self {
                Self {
                    #(#initialized_fields,)*
                }
            }
        }
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
                    hardness: std::option::Option<u32>,
                    weight: std::option::Option<u32>,

                }

                impl AnvilFactory {
                    pub fn new() -> Self {
                        Self {
                            hardness: None,
                            weight: None,
                        }
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
}
