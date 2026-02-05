use crate::Analysis;
use crate::codegen::FindCodegen;
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
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let fn_find = FindCodegen::new(self.analysis).generate();

        quote! {
            impl ::fabrique::Query for #base_struct_ident {
                #fn_find
            }
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
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Query for Anvil {
                    fn find<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            Self::query()
                                .select()
                                .r#where(Self::ID, "=", id)
                                .first_or_fail(&mut *conn)
                                .await
                                .map_err(Into::into)
                        }
                    }
                }
            }
            .to_string()
        );
    }
}
