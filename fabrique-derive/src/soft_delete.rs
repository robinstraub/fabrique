use crate::{Analysis, analysis::ast::ModelField};
use proc_macro2::TokenStream;
use quote::quote;

pub struct SoftDeleteCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> SoftDeleteCodegen<'a> {
    /// Creates a new code generator for the `fabrique::SoftDelete` trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the SoftDelete trait implementation as a token stream.
    pub fn generate(self) -> Option<TokenStream> {
        let soft_delete_field = self
            .analysis
            .fields
            .iter()
            .find(|field| field.soft_delete)?;

        let base_struct_ident = &self.analysis.ident;
        let fn_force_delete = &self.generate_fn_force_delete(soft_delete_field);
        let fn_soft_destroy = &self.generate_fn_soft_destroy(soft_delete_field);
        let fn_soft_delete = &self.generate_fn_soft_delete(soft_delete_field);
        let fn_restore = &self.generate_fn_restore(soft_delete_field);
        let fn_trashed = &self.generate_fn_trashed(soft_delete_field);

        Some(quote! {
            impl ::fabrique::SoftDelete for #base_struct_ident {
                #fn_force_delete
                #fn_soft_destroy
                #fn_soft_delete
                #fn_restore
                #fn_trashed
            }
        })
    }

    fn generate_fn_force_delete(&self, _soft_delete_field: &ModelField) -> TokenStream {
        quote! {
            async fn force_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                <Self as ::fabrique::HardDelete>::hard_delete(self, connection).await
            }
        }
    }

    fn generate_fn_soft_delete(&self, soft_delete_field: &ModelField) -> TokenStream {
        let primary_key = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key);

        let clause = primary_key
            .clone()
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field.ident, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");

        let query = format!(
            "UPDATE {} SET {} = now() WHERE {}",
            self.analysis.model.table_name, soft_delete_field.ident, clause
        );

        let bindings = primary_key.map(|ModelField { ident, .. }| quote! { self.#ident });

        quote! {
            async fn soft_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                sqlx::query(#query)#(.bind(#bindings))*.execute(connection).await?;

                Ok(())
            }
        }
    }

    fn generate_fn_soft_destroy(&self, soft_delete_field: &ModelField) -> TokenStream {
        let primary_key: Vec<_> = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let clause = primary_key
            .iter()
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field.ident, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");

        let query = format!(
            "UPDATE {} SET {} = now() WHERE {}",
            self.analysis.model.table_name, soft_delete_field.ident, clause
        );

        let binds = match primary_key.as_slice() {
            [ModelField { ident, .. }] => quote! { .bind(#ident) },
            composite => {
                let indices = (0..composite.len()).map(syn::Index::from);
                quote! { #(.bind(id.#indices))* }
            }
        };

        quote! {
            async fn soft_destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                sqlx::query(#query)
                    #binds
                    .execute(connection)
                    .await?;
                Ok(())
            }
        }
    }

    fn generate_fn_restore(&self, soft_delete_field: &ModelField) -> TokenStream {
        let primary_key = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key);

        let clause = primary_key
            .clone()
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field.ident, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");

        let query = format!(
            "UPDATE {} SET {} = NULL WHERE {}",
            self.analysis.model.table_name, soft_delete_field.ident, clause
        );

        let bindings = primary_key.map(|ModelField { ident, .. }| quote! { self.#ident });

        quote! {
            async fn restore(&self, connection: &Self::Connection) -> Result<(), Self::Error> {
                sqlx::query(#query)#(.bind(#bindings))*.execute(connection).await?;

                Ok(())
            }
        }
    }

    fn generate_fn_trashed(&self, soft_delete_field: &ModelField) -> TokenStream {
        let primary_key = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key);

        let clause = primary_key
            .clone()
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field.ident, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");

        let query = format!(
            "SELECT {} IS NOT NULL FROM {} WHERE {}",
            soft_delete_field.ident, self.analysis.model.table_name, clause
        );

        let bindings = primary_key.map(|ModelField { ident, .. }| quote! { self.#ident });

        quote! {
            async fn trashed(&self, connection: &Self::Connection) -> Result<bool, Self::Error> {
                sqlx::query_scalar(#query)#(.bind(#bindings))*.fetch_one(connection).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_a_basic_struct_derive_none() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = SoftDeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert!(result.is_none());
    }

    #[test]
    fn test_a_struct_with_soft_delete_derive_soft_delete() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime::<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = SoftDeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                impl ::fabrique::SoftDelete for Anvil {
                    async fn force_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        <Self as ::fabrique::HardDelete>::hard_delete(self, connection).await
                    }

                    async fn soft_destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = now() WHERE id = $1").bind(id).execute(connection).await?;
                        Ok(())
                    }

                    async fn soft_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = now() WHERE id = $1").bind(self.id).execute(connection).await?;
                        Ok(())
                    }

                    async fn restore(&self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = NULL WHERE id = $1").bind(self.id).execute(connection).await?;
                        Ok(())
                    }

                    async fn trashed(&self, connection: &Self::Connection) -> Result<bool, Self::Error> {
                        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM anvils WHERE id = $1").bind(self.id).fetch_one(connection).await
                    }
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_composite_keys() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                #[fabrique(primary_key)]
                user_id: uuid::Uuid,

                #[fabrique(primary_key)]
                organization_id: uuid::Uuid,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime::<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = SoftDeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                impl ::fabrique::SoftDelete for Anvil {
                    async fn force_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        <Self as ::fabrique::HardDelete>::hard_delete(self, connection).await
                    }

                    async fn soft_destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = now() WHERE user_id = $1 AND organization_id = $2").bind(id.0).bind(id.1).execute(connection).await?;
                        Ok(())
                    }

                    async fn soft_delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = now() WHERE user_id = $1 AND organization_id = $2").bind(self.user_id).bind(self.organization_id).execute(connection).await?;
                        Ok(())
                    }

                    async fn restore(&self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("UPDATE anvils SET deleted_at = NULL WHERE user_id = $1 AND organization_id = $2").bind(self.user_id).bind(self.organization_id).execute(connection).await?;
                        Ok(())
                    }

                    async fn trashed(&self, connection: &Self::Connection) -> Result<bool, Self::Error> {
                        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM anvils WHERE user_id = $1 AND organization_id = $2").bind(self.user_id).bind(self.organization_id).fetch_one(connection).await
                    }
                }
            }
            .to_string()
        )
    }
}
