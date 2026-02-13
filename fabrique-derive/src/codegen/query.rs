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

        // Per-PK-field Encode/Type bounds
        let pk_bounds = self
            .analysis
            .column_fields
            .iter()
            .filter(|f| f.primary_key)
            .map(|f| {
                let db_ty = f.r#as.as_ref().unwrap_or(&f.ty);
                quote! {
                    for<'q> #db_ty: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>
                }
            });

        quote! {
            impl<DB: ::fabrique::Dialect> ::fabrique::Query<DB> for #base_struct_ident
            where
                for<'r> #base_struct_ident: ::sqlx::FromRow<'r, <DB as ::sqlx::Database>::Row>,
                #(#pk_bounds,)*
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
            {
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
                impl<DB: ::fabrique::Dialect> ::fabrique::Query<DB> for Anvil
                where
                    for<'r> Anvil: ::sqlx::FromRow<'r, <DB as ::sqlx::Database>::Row>,
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                {
                    fn find<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<Self, ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            Self::query()
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
