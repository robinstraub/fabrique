use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Query trait implementation.
pub struct QueryCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> QueryCodegen<'a> {
    /// Creates a new code generator for Query trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `Query` trait implementation.
    ///
    /// The Query trait now provides default implementations for all methods,
    /// so we just need to implement the trait marker.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        quote! {
            impl ::fabrique::Query for #base_struct_ident {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_query_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = QueryCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        // The Query trait now provides default implementations,
        // so we just generate an empty impl block
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Query for Anvil {}
            }
            .to_string()
        );
    }
}
