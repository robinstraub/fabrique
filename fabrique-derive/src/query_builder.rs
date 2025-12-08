use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::analysis::Analysis;

/// Code generator for the QueryBuilder companion struct.
pub struct QueryBuilderCodegen<'a> {
    pub analysis: &'a Analysis<'a>,
    /// The generated query builder identifier (e.g., "AnvilQueryBuilder").
    pub query_builder_ident: Ident,
}

impl<'a> QueryBuilderCodegen<'a> {
    /// Creates a new code generator for QueryBuilder implementation.
    pub fn new(analysis: &'a Analysis) -> Self {
        let query_builder_ident = Self::generate_ident_query_builder(analysis.ident);

        Self {
            analysis,
            query_builder_ident,
        }
    }

    /// Generates the QueryBuilder companion struct.
    ///
    /// This method generates:
    /// - QueryBuilder struct definition
    /// - `new()` constructor method
    /// - `QueryBuilder` trait implementation with `where()` and `fetch_all()` methods
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let ident = &self.query_builder_ident;
        let struct_query_builder = self.generate_struct_query_builder();
        let fn_new = self.generate_fn_new();
        let fn_where = self.generate_fn_where();
        let fetch_all = self.generate_fetch_all();

        quote! {
            #struct_query_builder

            impl #ident {
                #fn_new
            }

            impl ::fabrique::QueryBuilder for #ident {
                type Model = #base_struct_ident;
                type Executor = ::sqlx::Pool<::sqlx::Postgres>;
                type Error = ::sqlx::Error;

                #fn_where

                #fetch_all
            }
        }
    }

    /// Generates the `where` method implementation.
    fn generate_fn_where(&self) -> TokenStream {
        quote! {
            fn r#where<T, O>(mut self, column: ::fabrique::ColumnMarker<T>, operator: O, value: T) -> Self
            where
                T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>,
                O: Into<::fabrique::Operator>
            {
                let operator: ::fabrique::Operator = operator.into();

                if !self.has_where {
                    self.builder.push(" WHERE ");
                    self.has_where = true;
                } else {
                    self.builder.push(" AND ");
                }

                self.builder.push(column.name);
                self.builder.push(" ");
                self.builder.push(operator.as_str());
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

    /// Generates the `fetch_all()` function.
    fn generate_fetch_all(&self) -> TokenStream {
        quote! {
            async fn fetch_all(mut self, executor: Self::Executor) -> Result<Vec<Self::Model>, Self::Error> {
                self.builder
                    .build_query_as::<Self::Model>()
                    .fetch_all(&executor)
                    .await
            }
        }
    }

    /// Generates the `new()` function.
    fn generate_fn_new(&self) -> TokenStream {
        let base_query = &self.analysis.base_select_query;

        quote! {
            pub fn new() -> Self {
                let builder = ::sqlx::query_builder::QueryBuilder::new(#base_query);
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
                pub struct AnvilQueryBuilder {
                    builder: ::sqlx::query_builder::QueryBuilder<'static, ::sqlx::Postgres>,
                    has_where: bool,
                }

                impl AnvilQueryBuilder {
                    pub fn new() -> Self {
                        let builder = ::sqlx::query_builder::QueryBuilder::new("SELECT id FROM anvils");
                        Self {
                            builder,
                            has_where: false,
                        }
                    }
                }

                impl ::fabrique::QueryBuilder for AnvilQueryBuilder {
                    type Model = Anvil;
                    type Executor = ::sqlx::Pool<::sqlx::Postgres>;
                    type Error = ::sqlx::Error;

                    fn r#where<T, O>(mut self, column: ::fabrique::ColumnMarker<T>, operator: O, value: T) -> Self
                    where
                        T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>,
                        O: Into<::fabrique::Operator>
                    {
                        let operator: ::fabrique::Operator = operator.into();

                        if !self.has_where {
                            self.builder.push(" WHERE ");
                            self.has_where = true;
                        } else {
                            self.builder.push(" AND ");
                        }

                        self.builder.push(column.name);
                        self.builder.push(" ");
                        self.builder.push(operator.as_str());
                        self.builder.push(" ");
                        self.builder.push_bind(value);

                        self
                    }

                    async fn fetch_all(
                        mut self,
                        executor: Self::Executor
                    ) -> Result<Vec<Self::Model>, Self::Error> {
                        self.builder
                            .build_query_as::<Self::Model>()
                            .fetch_all(&executor)
                            .await
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
                fn r#where<T, O>(mut self, column: ::fabrique::ColumnMarker<T>, operator: O, value: T) -> Self
                where
                    T: 'static + for<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> + ::sqlx::Type<::sqlx::Postgres>,
                    O: Into<::fabrique::Operator>
                {
                    let operator: ::fabrique::Operator = operator.into();

                    if !self.has_where {
                        self.builder.push(" WHERE ");
                        self.has_where = true;
                    } else {
                        self.builder.push(" AND ");
                    }

                    self.builder.push(column.name);
                    self.builder.push(" ");
                    self.builder.push(operator.as_str());
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
