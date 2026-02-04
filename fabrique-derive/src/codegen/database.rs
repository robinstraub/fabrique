use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for DatabaseAware trait implementation.
pub struct DatabaseAwareCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> DatabaseAwareCodegen<'a> {
    /// Creates a new code generator for DatabaseAware trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `DatabaseAware` trait implementation.
    ///
    /// Uses the `Backend` type alias from `fabrique-core`, which resolves to
    /// the concrete database type based on the active feature flag.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        quote! {
            impl ::fabrique::DatabaseAware for #base_struct_ident {
                type Database = ::fabrique::Backend;
                type Error = ::fabrique::Error;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_database_aware() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DatabaseAwareCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::DatabaseAware for Anvil {
                    type Database = ::fabrique::Backend;
                    type Error = ::fabrique::Error;
                }
            }
            .to_string()
        );
    }
}
