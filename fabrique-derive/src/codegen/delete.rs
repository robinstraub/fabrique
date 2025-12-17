use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

pub struct DeleteCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> DeleteCodegen<'a> {
    /// Creates a new code generator for the `fabrique::Delete` trait
    /// implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `fabrique::Delete` trait implementation as a token stream.
    ///
    /// The implementation delegates to either `SoftDelete` or `HardDelete`
    /// based on whether the model has a soft delete field.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let fn_delete = self.generate_fn_delete();
        let fn_destroy = self.generate_fn_destroy();

        quote! {
            impl ::fabrique::Delete for #base_struct_ident {
                #fn_delete
                #fn_destroy
            }
        }
    }

    fn generate_fn_delete(&self) -> TokenStream {
        let soft_delete = self.analysis.fields.iter().find(|field| field.soft_delete);

        match soft_delete {
            Some(_) => quote! {
                fn delete<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    <Self as ::fabrique::SoftDelete>::soft_delete(self, executor)
                }
            },
            None => quote! {
                fn delete<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    <Self as ::fabrique::HardDelete>::hard_delete(self, executor)
                }
            },
        }
    }

    fn generate_fn_destroy(&self) -> TokenStream {
        let soft_delete = self.analysis.fields.iter().find(|field| field.soft_delete);

        match soft_delete {
            Some(_) => quote! {
                fn destroy<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    <Self as ::fabrique::SoftDelete>::soft_destroy(executor, id)
                }
            },
            None => quote! {
                fn destroy<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                where
                    E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                {
                    <Self as ::fabrique::HardDelete>::hard_destroy(executor, id)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_basic_struct_delegates_to_hard_delete() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Delete for Anvil {
                    async fn delete(self, connection: &Self::Database) -> Result<(), Self::Error> {
                        <Self as ::fabrique::HardDelete>::hard_delete(self, connection).await
                    }

                    async fn destroy(connection: &Self::Database, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        <Self as ::fabrique::HardDelete>::hard_destroy(connection, id).await
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_soft_delete_struct_delegates_to_soft_delete() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Delete for Anvil {
                    async fn delete(self, connection: &Self::Database) -> Result<(), Self::Error> {
                        <Self as ::fabrique::SoftDelete>::soft_delete(self, connection).await
                    }

                    async fn destroy(connection: &Self::Database, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        <Self as ::fabrique::SoftDelete>::soft_destroy(connection, id).await
                    }
                }
            }
            .to_string()
        );
    }
}
