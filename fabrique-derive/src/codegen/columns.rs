use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for column marker constants.
pub struct ColumnsCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> ColumnsCodegen<'a> {
    /// Creates a new code generator for column marker constants.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates column marker constants for type-safe query building.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let constants = self.generate_column_constants();

        quote! {
            impl #base_struct_ident {
                #constants
            }
        }
    }

    fn generate_column_constants(&self) -> TokenStream {
        let constants = self
            .analysis
            .fields
            .iter()
            .map(|field| {
                let field_ident = &field.ident;
                let field_type = &field.ty;
                let const_name = syn::Ident::new(
                    &field_ident.to_string().to_uppercase(),
                    field_ident.span(),
                );
                let column_name = field_ident.to_string();

                Some(quote! {
                    pub const #const_name: ::fabrique::ColumnMarker<#field_type> = ::fabrique::ColumnMarker::new(#column_name);
                })
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
    fn test_generate_column_constants() {
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
                impl Anvil {
                    pub const ID: ::fabrique::ColumnMarker<String> = ::fabrique::ColumnMarker::new("id");
                    pub const NAME: ::fabrique::ColumnMarker<String> = ::fabrique::ColumnMarker::new("name");
                }
            }
            .to_string()
        );
    }
}
