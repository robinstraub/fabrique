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
        let ty_soft_delete_column = self.generate_ty_soft_delete_column();
        let fn_primary_key = self.generate_fn_primary_key();
        let fn_table_name = self.generate_fn_table_name();
        let fn_columns = self.generate_fn_columns();
        let fn_uses_soft_delete = self.generate_fn_soft_delete_column();

        quote! {
            impl ::fabrique::Model for #base_struct_ident {
                type PrimaryKey = #ty_primary_key;
                type SoftDeleteColumn = #ty_soft_delete_column;

                #fn_primary_key

                #fn_table_name

                #fn_columns

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

    fn generate_ty_soft_delete_column(&self) -> TokenStream {
        match self.analysis.fields.iter().find(|field| field.soft_delete) {
            Some(field) => {
                let ty = &field.column_type;
                quote! { #ty }
            }
            None => quote! { ::fabrique::Nil },
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

    fn generate_fn_columns(&self) -> TokenStream {
        let columns: Vec<_> = self
            .analysis
            .fields
            .iter()
            .map(|field| field.column.as_str())
            .collect();

        quote! {
            fn columns() -> &'static [&'static str] {
                &[#(#columns),*]
            }
        }
    }

    fn generate_fn_soft_delete_column(&self) -> TokenStream {
        let soft_delete = self
            .analysis
            .fields
            .iter()
            .find(|field| field.soft_delete)
            .map(|field| {
                let const_column_name = &field.const_column_name;
                quote! { Some(Self::#const_column_name) }
            });

        match soft_delete {
            Some(soft_delete) => quote! {
                fn soft_delete_column() -> Option<Self::SoftDeleteColumn> {
                    #soft_delete
                }
            },
            None => quote! {},
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
                    type SoftDeleteColumn = ::fabrique::Nil;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn columns() ->  &'static [&'static str] {
                        &["id"]
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
                    type SoftDeleteColumn = AnvilDeletedAtColumn;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn columns() ->  &'static [&'static str] {
                        &["id", "deleted_at"]
                    }

                    fn soft_delete_column() -> Option<Self::SoftDeleteColumn> {
                        Some(Self::DELETED_AT)
                    }
                }
            }
            .to_string()
        );
    }
}
