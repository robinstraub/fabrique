use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

pub struct FindCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> FindCodegen<'a> {
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `find` method implementation using the query builder.
    pub fn generate(self) -> TokenStream {
        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        // Generate .r#where() chains for each primary key column
        let where_clauses = match primary_key.as_slice() {
            [single] => {
                let const_name = &single.const_column_name;
                quote! { .r#where(Self::#const_name, "=", id) }
            }
            composite => {
                let clauses = composite.iter().enumerate().map(|(i, field)| {
                    let const_name = &field.const_column_name;
                    let idx = syn::Index::from(i);
                    quote! { .r#where(Self::#const_name, "=", id.#idx) }
                });
                quote! { #(#clauses)* }
            }
        };

        quote! {
            fn find<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
            {
                async move {
                    Self::query()
                        .select()
                        #where_clauses
                        .first_or_fail(executor)
                        .await
                        .map_err(Into::into)
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
    fn test_find_with_single_primary_key() {
        let input = parse_quote! { struct Anvil { id: String, name: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FindCodegen::new(&analysis);

        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                fn find<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    async move {
                        Self::query()
                            .select()
                            .r#where(Self::ID, "=", id)
                            .first_or_fail(executor)
                            .await
                            .map_err(Into::into)
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_find_with_composite_primary_key() {
        let input = parse_quote! {
            struct OrderLine {
                #[fabrique(primary_key)]
                order_id: uuid::Uuid,

                #[fabrique(primary_key)]
                product_id: uuid::Uuid,

                quantity: i32
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FindCodegen::new(&analysis);

        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                fn find<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    async move {
                        Self::query()
                            .select()
                            .r#where(Self::ORDER_ID, "=", id.0)
                            .r#where(Self::PRODUCT_ID, "=", id.1)
                            .first_or_fail(executor)
                            .await
                            .map_err(Into::into)
                    }
                }
            }
            .to_string()
        );
    }
}
