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

        // PK-field Encode/Type bounds
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
            impl<DB: ::fabrique::Dialect> ::fabrique::HardDelete<DB> for #base_struct_ident
            where
                #(#pk_bounds,)*
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
            {
                #fn_hard_destroy
                #fn_hard_delete
            }
        }
    }

    fn generate_fn_hard_delete(&self) -> TokenStream {
        let table_name = &self.analysis.model.table_name;

        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let pk_col_names: Vec<_> = primary_key.iter().map(|f| f.ident.to_string()).collect();
        let bindings = primary_key
            .iter()
            .map(|ColumnField { ident, .. }| quote! { self.#ident });

        quote! {
            fn hard_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let clause = [#(#pk_col_names),*]
                        .iter()
                        .enumerate()
                        .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    let query = format!(
                        "DELETE FROM {} WHERE {}",
                        #table_name,
                        clause
                    );
                    ::sqlx::query(::sqlx::AssertSqlSafe(query))#(.bind(#bindings))*.execute(&mut *conn).await?;
                    Ok(())
                }
            }
        }
    }

    fn generate_fn_hard_destroy(&self) -> TokenStream {
        let table_name = &self.analysis.model.table_name;

        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let pk_col_names: Vec<_> = primary_key.iter().map(|f| f.ident.to_string()).collect();

        let binds = match primary_key.as_slice() {
            [_] => quote! { .bind(id) },
            composite => {
                let indices = (0..composite.len()).map(syn::Index::from);
                quote! { #(.bind(id.#indices))* }
            }
        };

        quote! {
            fn hard_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let clause = [#(#pk_col_names),*]
                        .iter()
                        .enumerate()
                        .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    let query = format!(
                        "DELETE FROM {} WHERE {}",
                        #table_name,
                        clause
                    );
                    ::sqlx::query(::sqlx::AssertSqlSafe(query))
                        #binds
                        .execute(&mut *conn)
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
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HardDeleteCodegen::new(&analysis);
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::HardDelete<DB> for Anvil
                where
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                {
                    fn hard_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "DELETE FROM {} WHERE {}",
                                "anvils",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query))
                                .bind(id)
                                .execute(&mut *conn)
                                .await?;
                            Ok(())
                        }
                    }

                    fn hard_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "DELETE FROM {} WHERE {}",
                                "anvils",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(self.id).execute(&mut *conn).await?;
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
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::HardDelete<DB> for Anvil
                where
                    for<'q> uuid::Uuid: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'q> uuid::Uuid: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                {
                    fn hard_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["user_id", "organization_id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "DELETE FROM {} WHERE {}",
                                "anvils",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query))
                                .bind(id.0)
                                .bind(id.1)
                                .execute(&mut *conn)
                                .await?;
                            Ok(())
                        }
                    }

                    fn hard_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["user_id", "organization_id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "DELETE FROM {} WHERE {}",
                                "anvils",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(self.user_id).bind(self.organization_id).execute(&mut *conn).await?;
                            Ok(())
                        }
                    }
                }
            }
            .to_string()
        );
    }
}
