use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::analysis::Analysis;

/// Code generator for the QueryBuilder companion struct.
pub struct QueryBuilderCodegen {
    /// The generated query builder identifier (e.g., "AnvilQueryBuilder").
    pub query_builder_ident: Ident,
}

impl QueryBuilderCodegen {
    pub fn new(analysis: &Analysis) -> Self {
        let query_builder_ident = Self::generate_ident_query_builder(analysis.ident);

        Self {
            query_builder_ident,
        }
    }

    /// Generates the QueryBuilder companion struct.
    pub fn generate(self) -> TokenStream {
        let ident = &self.query_builder_ident;
        let struct_query_builder = self.generate_struct_query_builder();
        let fn_new = self.generate_fn_new();
        let fn_where = self.generate_fn_where();

        quote! {
            #[cfg(feature = "sqlx")]
            #struct_query_builder

            #[cfg(feature = "sqlx")]
            impl #ident {
                #fn_new
            }

            #[cfg(feature = "sqlx")]
            impl ::fabrique::QueryBuilder for #ident {
                #fn_where
            }
        }
    }

    /// Generates the `where` method implementation.
    fn generate_fn_where(&self) -> TokenStream {
        quote! {
            fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
            where
                T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
            {
                if !self.has_where {
                    self.builder.push(" WHERE ");
                    self.has_where = true;
                } else {
                    self.builder.push(" AND ");
                }

                self.builder.push(column.name);
                self.builder.push(" ");
                self.builder.push(operator);
                self.builder.push(" ");
                self.builder.push_bind(value);

                self
            }
        }
    }

    /// Generates the query builder identifier with "QueryBuilder" suffix.
    fn generate_ident_query_builder(base_ident: &Ident) -> Ident {
        let query_builder_name = format!("{}QueryBuilder", base_ident);
        Ident::new(&query_builder_name, base_ident.span())
    }

    /// Generates the query builder struct definition.
    fn generate_struct_query_builder(&self) -> TokenStream {
        let ident = &self.query_builder_ident;
        quote! {
            pub struct #ident {
                builder: ::sqlx::query_builder::QueryBuilder<'static, ::sqlx::Postgres>,
                has_where: bool,
            }
        }
    }

    /// Generates the `new()` function.
    fn generate_fn_new(&self) -> TokenStream {
        quote! {
            pub fn new() -> Self {
                let builder = ::sqlx::query_builder::QueryBuilder::new("");
                Self {
                    builder,
                    has_where: false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = QueryBuilderCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                #[cfg(feature = "sqlx")]
                pub struct AnvilQueryBuilder {
                    builder: ::sqlx::query_builder::QueryBuilder<'static, ::sqlx::Postgres>,
                    has_where: bool,
                }

                #[cfg(feature = "sqlx")]
                impl AnvilQueryBuilder {
                    pub fn new() -> Self {
                        let builder = ::sqlx::query_builder::QueryBuilder::new("");
                        Self {
                            builder,
                            has_where: false,
                        }
                    }
                }

                #[cfg(feature = "sqlx")]
                impl ::fabrique::QueryBuilder for AnvilQueryBuilder {
                    fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
                    where
                        T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
                    {
                        if !self.has_where {
                            self.builder.push(" WHERE ");
                            self.has_where = true;
                        } else {
                            self.builder.push(" AND ");
                        }

                        self.builder.push(column.name);
                        self.builder.push(" ");
                        self.builder.push(operator);
                        self.builder.push(" ");
                        self.builder.push_bind(value);

                        self
                    }
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_fn_where() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {} };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = QueryBuilderCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate_fn_where();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
                where
                    T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
                {
                    if !self.has_where {
                        self.builder.push(" WHERE ");
                        self.has_where = true;
                    } else {
                        self.builder.push(" AND ");
                    }

                    self.builder.push(column.name);
                    self.builder.push(" ");
                    self.builder.push(operator);
                    self.builder.push(" ");
                    self.builder.push_bind(value);

                    self
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_ident_query_builder() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {} };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = QueryBuilderCodegen::new(&analysis);

        // Act - access the precomputed identifier
        let generated = &codegen.query_builder_ident;

        // Assert the result
        assert_eq!(generated, "AnvilQueryBuilder");
    }

    #[test]
    fn test_generate_struct_query_builder() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {} };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = QueryBuilderCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate_struct_query_builder();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                pub struct AnvilQueryBuilder {
                    builder: ::sqlx::query_builder::QueryBuilder<'static, ::sqlx::Postgres>,
                    has_where: bool,
                }
            }
            .to_string()
        )
    }
}
