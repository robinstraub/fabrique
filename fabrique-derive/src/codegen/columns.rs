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
        let types = self.analysis.fields.iter().map(|field| {
            let type_name = self.column_type_name(&field.ident);

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
        let impls = self.analysis.fields.iter().map(|field| {
            let type_name = self.column_type_name(&field.ident);
            let field_type = &field.ty;
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
        let constants = self.analysis.fields.iter().map(|field| {
            let const_name =
                syn::Ident::new(&field.ident.to_string().to_uppercase(), field.ident.span());
            let type_name = self.column_type_name(&field.ident);

            quote! {
                pub const #const_name: #type_name = #type_name;
            }
        });

        quote! {
            #(#constants)*
        }
    }

    fn column_type_name(&self, field_ident: &syn::Ident) -> syn::Ident {
        let struct_name = self.analysis.ident.to_string();
        let field_name = field_ident.to_string();

        // Convert snake_case to PascalCase
        let pascal_case_field = field_name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();

        let type_name = format!("{}{}Column", struct_name, pascal_case_field);
        syn::Ident::new(&type_name, field_ident.span())
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
