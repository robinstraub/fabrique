use crate::Analysis;
use crate::analysis::ast::ColumnField;
use proc_macro2::TokenStream;
use quote::quote;

pub struct HardDeleteCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> HardDeleteCodegen<'a> {
    /// Creates a new code generator for the `fabrique::HardDelete` trait
    /// implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `fabrique::HardDelete` trait implementation as a token
    /// stream.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let fn_hard_destroy = self.generate_fn_hard_destroy();
        let fn_hard_delete = self.generate_fn_hard_delete();

        quote! {
            impl ::fabrique::HardDelete for #base_struct_ident {
                #fn_hard_destroy
                #fn_hard_delete
            }
        }
    }

    fn generate_fn_hard_delete(&self) -> TokenStream {
        let primary_key = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key);

        let clause = primary_key
            .clone()
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field.ident, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");

        let query = format!(
            "DELETE FROM {} WHERE {}",
            self.analysis.model.table_name, clause
        );

        let bindings = primary_key.map(|ColumnField { ident, .. }| quote! { self.#ident });

        quote! {
            fn hard_delete<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
            where
                E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
            {
                async move {
                    ::sqlx::query(#query)#(.bind(#bindings))*.execute(executor).await?;
                    Ok(())
                }
            }
        }
    }

    fn generate_fn_hard_destroy(&self) -> TokenStream {
        let primary_key: Vec<_> = self
            .analysis
            .column_fields
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
            "DELETE FROM {} WHERE {}",
            self.analysis.model.table_name, clause
        );

        let binds = match primary_key.as_slice() {
            [_] => quote! { .bind(id) },
            composite => {
                let indices = (0..composite.len()).map(syn::Index::from);
                quote! { #(.bind(id.#indices))* }
            }
        };

        quote! {
            fn hard_destroy<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
            where
                E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
            {
                async move {
                    ::sqlx::query(#query)
                        #binds
                        .execute(executor)
                        .await?;
                    Ok(())
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
    fn test_a_basic_struct_derive_delete() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HardDeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::HardDelete for Anvil {
                    fn hard_destroy<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            ::sqlx::query("DELETE FROM anvils WHERE id = $1").bind(id).execute(executor).await?;
                            Ok(())
                        }
                    }

                    fn hard_delete<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            ::sqlx::query("DELETE FROM anvils WHERE id = $1").bind(self.id).execute(executor).await?;
                            Ok(())
                        }
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_composite_keys() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                #[fabrique(primary_key)]
                user_id: uuid::Uuid,

                #[fabrique(primary_key)]
                organization_id: uuid::Uuid
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HardDeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::HardDelete for Anvil {
                    fn hard_destroy<'e, E>(executor: E, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            ::sqlx::query("DELETE FROM anvils WHERE user_id = $1 AND organization_id = $2").bind(id.0).bind(id.1).execute(executor).await?;
                            Ok(())
                        }
                    }

                    fn hard_delete<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<(), Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            ::sqlx::query("DELETE FROM anvils WHERE user_id = $1 AND organization_id = $2").bind(self.user_id).bind(self.organization_id).execute(executor).await?;
                            Ok(())
                        }
                    }
                }
            }
            .to_string()
        );
    }
}
