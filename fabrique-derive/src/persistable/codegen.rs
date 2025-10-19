use crate::{analysis::Analysis, error::Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// Code generator for persistable trait implementation.
pub struct PersistableCodegen<'a> {
    /// Analysis output containing fields and relations
    analysis: Analysis<'a>,
}

impl<'a> PersistableCodegen<'a> {
    /// Creates a code generator from the given derive input.
    pub fn from(input: &'a DeriveInput) -> Result<Self, Error> {
        let analysis = Analysis::from(input)?;

        Ok(Self { analysis })
    }

    pub fn generate(self) -> Result<TokenStream, Error> {
        let base_struct_ident = &self.analysis.ident;
        let fn_all = self.generate_fn_all();
        let fn_create = self.generate_fn_create();

        let generated = quote! {
            impl fabrique::Persistable for #base_struct_ident {
                type Connection = sqlx::Pool<sqlx::Postgres>;
                type Error = sqlx::Error;

                #fn_create
                #fn_all
            }
        };

        Ok(generated)
    }

    /// Generates the `all()` associated function.
    fn generate_fn_all(&self) -> TokenStream {
        // Compute the sql column names for the query
        let column_names = self
            .analysis
            .fields
            .iter()
            .filter_map(|field| field.ident.as_ref())
            .map(|ident| ident.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        let query = format!("SELECT {} FROM {}", column_names, self.analysis.table_name);

        quote! {
            async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                sqlx::query_as!(Self, #query).fetch_all(connection).await
            }
        }
    }

    /// Generates the `create()` method.
    fn generate_fn_create(&self) -> TokenStream {
        quote! {
            async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                todo!()
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
        let codegen = PersistableCodegen::from(&input).unwrap();

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                impl fabrique::Persistable for Anvil {
                    type Connection = sqlx::Pool<sqlx::Postgres>;
                    type Error = sqlx::Error;

                    async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                        todo!()
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as!(Self, "SELECT id FROM anvils").fetch_all(connection).await
                    }
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_fn_all() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let codegen = PersistableCodegen::from(&input).unwrap();

        // Act the call to the generate method
        let result = codegen.generate_fn_all();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                    sqlx::query_as!(Self, "SELECT id FROM anvils").fetch_all(connection).await
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_generate_fn_create() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {} };
        let codegen = PersistableCodegen::from(&input).unwrap();

        // Act the call to the generate method
        let result = codegen.generate_fn_create();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                    todo!()
                }
            }
            .to_string()
        )
    }
}
