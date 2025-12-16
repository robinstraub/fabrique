use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Persist trait implementation.
pub struct PersistCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> PersistCodegen<'a> {
    /// Creates a new code generator for Persist trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `Persist` trait implementation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let fn_create = self.generate_fn_create();

        quote! {
            impl ::fabrique::Persist for #base_struct_ident {
                #fn_create
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
            match &field.r#as {
                Some(ty) => quote! { #ty::from(self.#ident) },
                None => quote! { self.#ident },
            }
        });

        quote! {
            fn create<E>(self, executor: E) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send
            where
                E: for<'e> ::sqlx::Executor<'e, Database = Self::Database>,
            {
                async move {
                    ::sqlx::query_as::<_, Self>(#query)
                        #(.bind(#field_bindings))*
                        .fetch_one(executor)
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
    fn test_generate_persist_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = PersistCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Persist for Anvil {
                    async fn create(self, connection: &Self::Database) -> Result<Self, Self::Error> {
                        sqlx::query_as::<_, Self>("INSERT INTO anvils (id) VALUES ($1) RETURNING id")
                            .bind(self.id)
                            .fetch_one(connection)
                            .await
                    }
                }
            }
            .to_string()
        );
    }
}
