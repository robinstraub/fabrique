use crate::Analysis;
use crate::analysis::ast::ColumnField;
use proc_macro2::TokenStream;
use quote::quote;

pub struct SoftDeleteCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> SoftDeleteCodegen<'a> {
    /// Creates a new code generator for the `fabrique::SoftDelete` trait
    /// implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the SoftDelete trait implementation as a token stream.
    pub fn generate(self) -> Option<TokenStream> {
        let soft_delete_field = self
            .analysis
            .column_fields
            .iter()
            .find(|field| field.soft_delete)?;

        let base_struct_ident = &self.analysis.ident;
        let fn_soft_destroy = &self.generate_fn_soft_destroy(soft_delete_field);
        let fn_soft_delete = &self.generate_fn_soft_delete(soft_delete_field);
        let fn_restore = &self.generate_fn_restore(soft_delete_field);
        let fn_trashed = &self.generate_fn_trashed(soft_delete_field);

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

        Some(quote! {
            impl<DB: ::fabrique::Dialect> ::fabrique::SoftDelete<DB> for #base_struct_ident
            where
                #(#pk_bounds,)*
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                usize: ::sqlx::ColumnIndex<<DB as ::sqlx::Database>::Row>,
                for<'d> bool: ::sqlx::decode::Decode<'d, DB> + ::sqlx::Type<DB>,
            {
                #fn_soft_delete
                #fn_soft_destroy
                #fn_restore
                #fn_trashed
            }
        })
    }

    fn generate_fn_soft_delete(&self, soft_delete_field: &ColumnField) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        let soft_delete_col = soft_delete_field.ident.to_string();

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
            fn soft_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
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
                        "UPDATE {} SET {} = {} WHERE {}",
                        #table_name,
                        #soft_delete_col,
                        <DB as ::fabrique::Dialect>::now(),
                        clause
                    );
                    ::sqlx::query(::sqlx::AssertSqlSafe(query))#(.bind(#bindings))*.execute(&mut *conn).await?;
                    Ok(())
                }
            }
        }
    }

    fn generate_fn_soft_destroy(&self, soft_delete_field: &ColumnField) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        let soft_delete_col = soft_delete_field.ident.to_string();

        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let pk_col_names: Vec<_> = primary_key.iter().map(|f| f.ident.to_string()).collect();

        let binds = match primary_key.as_slice() {
            [ColumnField { ident, .. }] => quote! { .bind(#ident) },
            composite => {
                let indices = (0..composite.len()).map(syn::Index::from);
                quote! { #(.bind(id.#indices))* }
            }
        };

        quote! {
            fn soft_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
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
                        "UPDATE {} SET {} = {} WHERE {}",
                        #table_name,
                        #soft_delete_col,
                        <DB as ::fabrique::Dialect>::now(),
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

    fn generate_fn_restore(&self, soft_delete_field: &ColumnField) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        let soft_delete_col = soft_delete_field.ident.to_string();

        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let pk_col_names: Vec<_> = primary_key.iter().map(|f| f.ident.to_string()).collect();

        let pk_captures = primary_key.iter().map(|field| {
            let ident = &field.ident;
            quote! { let #ident = self.#ident.clone(); }
        });

        let bindings = primary_key.iter().map(|field| {
            let ident = &field.ident;
            quote! { #ident }
        });

        quote! {
            fn restore<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                #(#pk_captures)*
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let clause = [#(#pk_col_names),*]
                        .iter()
                        .enumerate()
                        .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    let query = format!(
                        "UPDATE {} SET {} = NULL WHERE {}",
                        #table_name,
                        #soft_delete_col,
                        clause
                    );
                    ::sqlx::query(::sqlx::AssertSqlSafe(query))#(.bind(#bindings))*.execute(&mut *conn).await?;
                    Ok(())
                }
            }
        }
    }

    fn generate_fn_trashed(&self, soft_delete_field: &ColumnField) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        let soft_delete_col = soft_delete_field.ident.to_string();

        let primary_key: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        let pk_col_names: Vec<_> = primary_key.iter().map(|f| f.ident.to_string()).collect();

        let pk_captures = primary_key.iter().map(|field| {
            let ident = &field.ident;
            quote! { let #ident = self.#ident.clone(); }
        });

        let bindings = primary_key.iter().map(|field| {
            let ident = &field.ident;
            quote! { #ident }
        });

        quote! {
            fn trashed<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<bool, ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                #(#pk_captures)*
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let clause = [#(#pk_col_names),*]
                        .iter()
                        .enumerate()
                        .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    let query = format!(
                        "SELECT {} IS NOT NULL FROM {} WHERE {}",
                        #soft_delete_col,
                        #table_name,
                        clause
                    );
                    ::sqlx::query_scalar(::sqlx::AssertSqlSafe(query))#(.bind(#bindings))*.fetch_one(&mut *conn).await.map_err(Into::into)
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
    fn test_a_basic_struct_derive_none() {
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = SoftDeleteCodegen::new(&analysis);
        let result = codegen.generate();
        assert!(result.is_none());
    }

    #[test]
    fn test_a_struct_with_soft_delete_derive_soft_delete() {
        let input = parse_quote! {
            struct Anvil {
                id: String,
                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime::<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = SoftDeleteCodegen::new(&analysis);
        let result = codegen.generate();

        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::SoftDelete<DB> for Anvil
                where
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                    usize: ::sqlx::ColumnIndex<<DB as ::sqlx::Database>::Row>,
                    for<'d> bool: ::sqlx::decode::Decode<'d, DB> + ::sqlx::Type<DB>,
                {
                    fn soft_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
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
                                "UPDATE {} SET {} = {} WHERE {}",
                                "anvils",
                                "deleted_at",
                                <DB as ::fabrique::Dialect>::now(),
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(self.id).execute(&mut *conn).await?;
                            Ok(())
                        }
                    }
                    fn soft_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
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
                                "UPDATE {} SET {} = {} WHERE {}",
                                "anvils",
                                "deleted_at",
                                <DB as ::fabrique::Dialect>::now(),
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query))
                                .bind(id)
                                .execute(&mut *conn)
                                .await?;
                            Ok(())
                        }
                    }
                    fn restore<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        let id = self.id.clone();
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "UPDATE {} SET {} = NULL WHERE {}",
                                "anvils",
                                "deleted_at",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(id).execute(&mut *conn).await?;
                            Ok(())
                        }
                    }
                    fn trashed<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<bool, ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        let id = self.id.clone();
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "SELECT {} IS NOT NULL FROM {} WHERE {}",
                                "deleted_at",
                                "anvils",
                                clause
                            );
                            ::sqlx::query_scalar(::sqlx::AssertSqlSafe(query)).bind(id).fetch_one(&mut *conn).await.map_err(Into::into)
                        }
                    }
                }
            }.to_string()
        )
    }

    #[test]
    fn test_composite_keys() {
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
        let result = codegen.generate();

        assert_eq!(
            result.unwrap().to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::SoftDelete<DB> for Anvil
                where
                    for<'q> uuid::Uuid: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'q> uuid::Uuid: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                    usize: ::sqlx::ColumnIndex<<DB as ::sqlx::Database>::Row>,
                    for<'d> bool: ::sqlx::decode::Decode<'d, DB> + ::sqlx::Type<DB>,
                {
                    fn soft_delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
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
                                "UPDATE {} SET {} = {} WHERE {}",
                                "anvils",
                                "deleted_at",
                                <DB as ::fabrique::Dialect>::now(),
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(self.user_id).bind(self.organization_id).execute(&mut *conn).await?;
                            Ok(())
                        }
                    }
                    fn soft_destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
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
                                "UPDATE {} SET {} = {} WHERE {}",
                                "anvils",
                                "deleted_at",
                                <DB as ::fabrique::Dialect>::now(),
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
                    fn restore<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        let user_id = self.user_id.clone();
                        let organization_id = self.organization_id.clone();
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["user_id", "organization_id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "UPDATE {} SET {} = NULL WHERE {}",
                                "anvils",
                                "deleted_at",
                                clause
                            );
                            ::sqlx::query(::sqlx::AssertSqlSafe(query)).bind(user_id).bind(organization_id).execute(&mut *conn).await?;
                            Ok(())
                        }
                    }
                    fn trashed<'e, A>(&self, executor: A) -> impl ::std::future::Future<Output = Result<bool, ::fabrique::Error>> + Send + 'e
                    where A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        let user_id = self.user_id.clone();
                        let organization_id = self.organization_id.clone();
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let clause = ["user_id", "organization_id"]
                                .iter()
                                .enumerate()
                                .map(|(i, col)| format!("{} = {}", col, <DB as ::fabrique::Dialect>::placeholder(i + 1)))
                                .collect::<Vec<_>>()
                                .join(" AND ");
                            let query = format!(
                                "SELECT {} IS NOT NULL FROM {} WHERE {}",
                                "deleted_at",
                                "anvils",
                                clause
                            );
                            ::sqlx::query_scalar(::sqlx::AssertSqlSafe(query)).bind(user_id).bind(organization_id).fetch_one(&mut *conn).await.map_err(Into::into)
                        }
                    }
                }
            }.to_string()
        )
    }
}
