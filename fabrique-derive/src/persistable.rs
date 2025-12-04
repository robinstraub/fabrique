use crate::{analysis::Analysis, query_builder::QueryBuilderCodegen};
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for persistable trait implementation.
pub struct PersistableCodegen<'a> {
    /// Analysis output containing fields and relations.
    analysis: &'a Analysis<'a>,

    query_builder: &'a QueryBuilderCodegen<'a>,
}

impl<'a> PersistableCodegen<'a> {
    /// Creates a new code generator for Persistable trait implementation.
    pub fn new(analysis: &'a Analysis<'a>, query_builder: &'a QueryBuilderCodegen) -> Self {
        Self {
            analysis,
            query_builder,
        }
    }

    /// Generates the complete Persistable trait implementation as a token stream.
    ///
    /// This method generates:
    /// - `FromRow` trait implementation for database row mapping
    /// - `Persistable` trait implementation
    /// - Column marker constants for type-safe query building
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let query_builder_ident = &self.query_builder.query_builder_ident;
        let fn_all = self.generate_fn_all();
        let fn_create = self.generate_fn_create();
        let fn_query = self.generate_fn_query(query_builder_ident);
        let column_constants = self.generate_column_constants();

        let generated = quote! {

            impl ::fabrique::Persistable for #base_struct_ident {
                type Connection = sqlx::Pool<sqlx::Postgres>;
                type Error = sqlx::Error;

                type QueryBuilder = #query_builder_ident;

                #fn_create
                #fn_all

                #fn_query
            }

            impl #base_struct_ident {
                #column_constants
            }
        };

        generated
    }

    /// Generates the `all()` associated function.
    fn generate_fn_all(&self) -> TokenStream {
        let query = &self.analysis.base_select_query;

        quote! {
            async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                sqlx::query_as!(Self, #query).fetch_all(connection).await
            }
        }
    }

    /// Generates the `create()` method.
    fn generate_fn_create(&self) -> TokenStream {
        // Get field identifiers and names

        let columns = self
            .analysis
            .fields
            .iter()
            .map(|fields| fields.ident.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        // Generate placeholders ($1, $2, $3, ...)
        let placeholders = (1..=self.analysis.fields.len())
            .map(|i| format!("${}", i))
            .collect::<Vec<String>>()
            .join(", ");

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
            self.analysis.model.table_name, columns, placeholders, self.analysis.returning,
        );

        // Generate field bindings (self.field1, self.field2, ...)
        let field_bindings = self.analysis.fields.iter().map(|field| {
            let ident = &field.ident;
            quote! { self.#ident }
        });

        quote! {
            async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                sqlx::query_as!(Self, #query, #(#field_bindings),*)
                    .fetch_one(connection)
                    .await
            }
        }
    }

    /// Generates column constants for type-safe query building.
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

    /// Generates the `query()` function.
    fn generate_fn_query(&self, query_builder_ident: &syn::Ident) -> TokenStream {
        quote! {
            fn query() -> Self::QueryBuilder {
                #query_builder_ident::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_builder::QueryBuilderCodegen;
    use syn::parse_quote;

    #[test]
    fn test_generate() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Persistable for Anvil {
                    type Connection = sqlx::Pool<sqlx::Postgres>;
                    type Error = sqlx::Error;

                    type QueryBuilder = AnvilQueryBuilder;

                    async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                        sqlx::query_as!(Self, "INSERT INTO anvils (id) VALUES ($1) RETURNING id as \"id: String\"", self.id)
                            .fetch_one(connection)
                            .await
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as!(Self, "SELECT id as \"id: String\" FROM anvils").fetch_all(connection).await
                    }

                    fn query() -> Self::QueryBuilder {
                        AnvilQueryBuilder::new()
                    }
                }

                impl Anvil {
                    pub const ID: ::fabrique::ColumnMarker<String> = ::fabrique::ColumnMarker::new("id");
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_fn_all() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate_fn_all();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                    sqlx::query_as!(Self, "SELECT id as \"id: String\" FROM anvils").fetch_all(connection).await
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_fn_create() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate_fn_create();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                    sqlx::query_as!(Self, "INSERT INTO anvils (id) VALUES ($1) RETURNING id as \"id: String\"", self.id)
                        .fetch_one(connection)
                        .await
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_codegen_fail_explicitly() {
        // Arrange the codegen
        let input = parse_quote! { enum Anvil {} };

        // Act the call to the codegen
        let result = Analysis::from(&input);

        // Assert the result
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_column_constants() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate_column_constants();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                pub const ID: ::fabrique::ColumnMarker<String> = ::fabrique::ColumnMarker::new("id");
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_column_constants_multiple_fields() {
        // Arrange the codegen with multiple fields to cover all iterator branches
        let input = parse_quote! {
            struct Anvil {
                id: String,
                name: String,
                weight: i32
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate_column_constants();

        // Assert the result contains all three constants
        let result_str = result.to_string();
        assert!(result_str.contains("pub const ID"));
        assert!(result_str.contains("pub const NAME"));
        assert!(result_str.contains("pub const WEIGHT"));
    }
}
