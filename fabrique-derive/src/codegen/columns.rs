use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for column types and constants.
pub struct ColumnsCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> ColumnsCodegen<'a> {
    /// Creates a new code generator for column types and constants.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates ZST column types, Column trait implementations, and column
    /// constants for type-safe query building.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let column_types = self.generate_column_types();
        let column_impls = self.generate_column_impls();
        let constants = self.generate_column_constants();

        quote! {
            #column_types
            #column_impls
            impl #base_struct_ident {
                #constants
            }
        }
    }

    fn generate_column_types(&self) -> TokenStream {
        let types = self.analysis.column_fields.iter().map(|field| {
            let type_name = &field.column_type;

            quote! {
                pub struct #type_name;
            }
        });

        quote! {
            #(#types)*
        }
    }

    fn generate_column_impls(&self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let impls = self.analysis.column_fields.iter().map(|field| {
            let type_name = &field.column_type;
            let field_type = match &field.r#as {
                Some(as_type) => quote! { #as_type },
                None => {
                    let ty = &field.ty;
                    quote! { #ty }
                }
            };
            let column_name = field.ident.to_string();

            quote! {
                impl ::fabrique::Column<#base_struct_ident> for #type_name {
                    type Type = #field_type;

                    fn name(&self) -> &'static str {
                        #column_name
                    }
                }
            }
        });

        quote! {
            #(#impls)*
        }
    }

    fn generate_column_constants(&self) -> TokenStream {
        let constants = self.analysis.column_fields.iter().map(|field| {
            let const_name = &field.const_column_name;
            let type_name = &field.column_type;

            quote! {
                pub const #const_name: #type_name = #type_name;
            }
        });

        quote! {
            #(#constants)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_columns() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String, name: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ColumnsCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                pub struct AnvilIdColumn;
                pub struct AnvilNameColumn;

                impl ::fabrique::Column<Anvil> for AnvilIdColumn {
                    type Type = String;

                    fn name(&self) -> &'static str {
                        "id"
                    }
                }
                impl ::fabrique::Column<Anvil> for AnvilNameColumn {
                    type Type = String;

                    fn name(&self) -> &'static str {
                        "name"
                    }
                }

                impl Anvil {
                    pub const ID: AnvilIdColumn = AnvilIdColumn;
                    pub const NAME: AnvilNameColumn = AnvilNameColumn;
                }
            }
            .to_string()
        );
    }
}
