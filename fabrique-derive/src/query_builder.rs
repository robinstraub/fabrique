use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::{analysis::Analysis, error::Error};

/// Code generator for the QueryBuilder companion struct.
pub struct QueryBuilderCodegen<'a> {
    /// Analysis output containing fields and relations.
    analysis: Analysis<'a>,
    /// The generated query builder identifier (e.g., "AnvilQueryBuilder").
    pub query_builder_ident: Ident,
}

impl<'a> QueryBuilderCodegen<'a> {
    /// Creates a code generator from the given derive input.
    pub fn from(input: &'a DeriveInput) -> Result<Self, Error> {
        let analysis = Analysis::from(input)?;
        let query_builder_ident = Self::generate_ident_query_builder(&analysis.ident);

        Ok(Self {
            analysis,
            query_builder_ident,
        })
    }

    /// Generates the QueryBuilder companion struct.
    pub fn generate(self) -> Result<TokenStream, Error> {
        let ident = &self.query_builder_ident;
        let struct_query_builder = self.generate_struct_query_builder();
        let fn_where = self.generate_fn_where();

        let generated = quote! {
            #struct_query_builder

            impl ::fabrique::QueryBuilder for #ident {
                #fn_where
            }
        };

        Ok(generated)
    }

    /// Generates the `where` method implementation.
    fn generate_fn_where(&self) -> TokenStream {
        quote! {
            fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
            where
                T: for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let codegen = QueryBuilderCodegen::from(&input).unwrap();

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                pub struct AnvilQueryBuilder {
                    builder: ::sqlx::query_builder::QueryBuilder<'static, ::sqlx::Postgres>,
                    has_where: bool,
                }

                impl ::fabrique::QueryBuilder for AnvilQueryBuilder {
                    fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
                    where
                        T: for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
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
        let codegen = QueryBuilderCodegen::from(&input).unwrap();

        // Act the call to the generate method
        let result = codegen.generate_fn_where();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                fn r#where<T>(mut self, column: ::fabrique::ColumnMarker<T>, operator: &'static str, value: T) -> Self
                where
                    T: for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>
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
        let codegen = QueryBuilderCodegen::from(&input).unwrap();

        // Act - access the precomputed identifier
        let generated = &codegen.query_builder_ident;

        // Assert the result
        assert_eq!(generated, "AnvilQueryBuilder");
    }

    #[test]
    fn test_generate_struct_query_builder() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {} };
        let codegen = QueryBuilderCodegen::from(&input).unwrap();

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
