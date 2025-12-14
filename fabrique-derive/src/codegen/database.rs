use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Database trait implementation.
pub struct DatabaseCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> DatabaseCodegen<'a> {
    /// Creates a new code generator for Database trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `Database` trait implementation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        quote! {
            impl ::fabrique::Database for #base_struct_ident {
                type Connection = sqlx::Pool<sqlx::Postgres>;
                type Error = sqlx::Error;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_database() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DatabaseCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Database for Anvil {
                    type Connection = sqlx::Pool<sqlx::Postgres>;
                    type Error = sqlx::Error;
                }
            }
            .to_string()
        );
    }
}
