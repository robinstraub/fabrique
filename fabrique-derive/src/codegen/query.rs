use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Query trait implementation.
pub struct QueryCodegen<'a> {
    analysis: &'a Analysis<'a>,
    query_builder_ident: &'a syn::Ident,
}

impl<'a> QueryCodegen<'a> {
    /// Creates a new code generator for Query trait implementation.
    pub fn new(analysis: &'a Analysis<'a>, query_builder_ident: &'a syn::Ident) -> Self {
        Self {
            analysis,
            query_builder_ident,
        }
    }

    /// Generates the `Query` trait implementation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let query_builder_ident = self.query_builder_ident;
        let fn_query = self.generate_fn_query();
        let fn_all = self.generate_fn_all();

        quote! {
            impl ::fabrique::Query for #base_struct_ident {
                type QueryBuilder = #query_builder_ident;

                #fn_query

                #fn_all
            }
        }
    }

    /// Generates the `query()` function.
    fn generate_fn_query(&self) -> TokenStream {
        let query_builder_ident = self.query_builder_ident;
        quote! {
            fn query() -> Self::QueryBuilder {
                #query_builder_ident::new()
            }
        }
    }

    /// Generates the `all()` associated function.
    fn generate_fn_all(&self) -> TokenStream {
        let soft_delete = self.analysis.fields.iter().find(|field| field.soft_delete);

        let returning = self
            .analysis
            .fields
            .iter()
            .map(|fields| fields.column.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        let base_select_query = format!(
            "SELECT {} FROM {}",
            &returning, &self.analysis.model.table_name
        );

        let query = match soft_delete {
            Some(field) => format!("{} WHERE {} IS NULL", base_select_query, field.ident),
            None => base_select_query,
        };

        quote! {
            async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                sqlx::query_as::<_, Self>(#query).fetch_all(connection).await
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
        let query_builder_ident = syn::Ident::new("AnvilQueryBuilder", proc_macro2::Span::call_site());
        let codegen = QueryCodegen::new(&analysis, &query_builder_ident);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Query for Anvil {
                    type QueryBuilder = AnvilQueryBuilder;

                    fn query() -> Self::QueryBuilder {
                        AnvilQueryBuilder::new()
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as::<_, Self>("SELECT id FROM anvils").fetch_all(connection).await
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_query_trait_with_soft_delete() {
        // Arrange
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_ident = syn::Ident::new("AnvilQueryBuilder", proc_macro2::Span::call_site());
        let codegen = QueryCodegen::new(&analysis, &query_builder_ident);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Query for Anvil {
                    type QueryBuilder = AnvilQueryBuilder;

                    fn query() -> Self::QueryBuilder {
                        AnvilQueryBuilder::new()
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as::<_, Self>("SELECT id, deleted_at FROM anvils WHERE deleted_at IS NULL").fetch_all(connection).await
                    }
                }
            }
            .to_string()
        );
    }
}
