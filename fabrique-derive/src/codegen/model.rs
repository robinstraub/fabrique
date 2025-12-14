use crate::analysis::{Analysis, ast::ModelField};
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Model trait implementation.
pub struct ModelCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> ModelCodegen<'a> {
    /// Creates a new code generator for Model trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `Model` trait implementation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let ty_primary_key = self.generate_ty_primary_key();
        let fn_primary_key = self.generate_fn_primary_key();
        let fn_table_name = self.generate_fn_table_name();
        let fn_uses_soft_delete = self.generate_fn_uses_soft_delete();

        quote! {
            impl ::fabrique::Model for #base_struct_ident {
                type PrimaryKey = #ty_primary_key;

                #fn_primary_key

                #fn_table_name

                #fn_uses_soft_delete
            }
        }
    }

    fn generate_ty_primary_key(&self) -> TokenStream {
        let primary_keys: Vec<&ModelField> = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        match primary_keys.as_slice() {
            [simple] => {
                let ty = &simple.ty;
                quote! { #ty }
            }
            composite => {
                let tys = composite.iter().map(|field| &field.ty);
                quote! { (#(#tys),*) }
            }
        }
    }

    fn generate_fn_primary_key(&self) -> TokenStream {
        let primary_keys: Vec<&ModelField> = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        match primary_keys.as_slice() {
            [simple] => {
                let ident = &simple.ident;
                quote! {
                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.#ident.clone()
                    }
                }
            }
            composite => {
                let idents = composite.iter().map(|field| {
                    let ident = &field.ident;
                    quote! { self.#ident.clone() }
                });
                quote! {
                    fn primary_key(&self) -> Self::PrimaryKey {
                        (#(#idents),*)
                    }
                }
            }
        }
    }

    fn generate_fn_table_name(&self) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        quote! {
            fn table_name() -> &'static str {
                #table_name
            }
        }
    }

    fn generate_fn_uses_soft_delete(&self) -> TokenStream {
        let has_soft_delete = self.analysis.fields.iter().any(|field| field.soft_delete);

        quote! {
            fn uses_soft_delete() -> bool {
                #has_soft_delete
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_model_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Anvil {
                    type PrimaryKey = String;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn uses_soft_delete() -> bool {
                        false
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_model_trait_with_soft_delete() {
        // Arrange
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Anvil {
                    type PrimaryKey = String;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn uses_soft_delete() -> bool {
                        true
                    }
                }
            }
            .to_string()
        );
    }
}
