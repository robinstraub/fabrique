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
        let fn_save = self.generate_fn_save();

        // Per-column-field Encode/Type bounds (using `as` type when present)
        let field_bounds = self.analysis.column_fields.iter().map(|f| {
            let db_ty = f.r#as.as_ref().unwrap_or(&f.ty);
            quote! {
                for<'q> #db_ty: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>
            }
        });

        quote! {
            impl<DB: ::fabrique::Dialect> ::fabrique::Persist<DB> for #base_struct_ident
            where
                for<'r> #base_struct_ident: ::sqlx::FromRow<'r, <DB as ::sqlx::Database>::Row>,
                #(#field_bounds,)*
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB> + ::sqlx::Executor<'c, Database = DB>,
                <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
            {
                #fn_create

                #fn_save
            }
        }
    }

    /// Generates the `create()` method using execute + find (universal).
    fn generate_fn_create(&self) -> TokenStream {
        let set_calls = self.analysis.column_fields.iter().map(|field| {
            let const_name = &field.const_column_name;
            let ident = &field.ident;
            quote! { .set(Self::#const_name, self.#ident) }
        });

        quote! {
            fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let pk = <Self as ::fabrique::Model>::primary_key(&self);
                    <Self as ::fabrique::Query<DB>>::insert()
                        #(#set_calls)*
                        .execute(&mut *conn)
                        .await?;
                    <Self as ::fabrique::Query<DB>>::find(&mut *conn, pk)
                        .await
                        .map_err(Into::into)
                }
            }
        }
    }

    /// Generates the `save()` method using execute + find (universal).
    fn generate_fn_save(&self) -> TokenStream {
        let set_calls = self.analysis.column_fields.iter().map(|field| {
            let const_name = &field.const_column_name;
            let ident = &field.ident;
            quote! { .set(Self::#const_name, self.#ident) }
        });

        quote! {
            fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, ::fabrique::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let pk = <Self as ::fabrique::Model>::primary_key(&self);
                    <Self as ::fabrique::Query<DB>>::insert()
                        #(#set_calls)*
                        .on_conflict()
                        .do_update()
                        .execute(&mut *conn)
                        .await?;
                    <Self as ::fabrique::Query<DB>>::find(&mut *conn, pk)
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
                impl<DB: ::fabrique::Dialect> ::fabrique::Persist<DB> for Anvil
                where
                    for<'r> Anvil: ::sqlx::FromRow<'r, <DB as ::sqlx::Database>::Row>,
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB> + ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                {
                    fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let pk = <Self as ::fabrique::Model>::primary_key(&self);
                            <Self as ::fabrique::Query<DB>>::insert()
                                .set(Self::ID, self.id)
                                .execute(&mut *conn)
                                .await?;
                            <Self as ::fabrique::Query<DB>>::find(&mut *conn, pk)
                                .await
                                .map_err(Into::into)
                        }
                    }

                    fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let pk = <Self as ::fabrique::Model>::primary_key(&self);
                            <Self as ::fabrique::Query<DB>>::insert()
                                .set(Self::ID, self.id)
                                .on_conflict()
                                .do_update()
                                .execute(&mut *conn)
                                .await?;
                            <Self as ::fabrique::Query<DB>>::find(&mut *conn, pk)
                                .await
                                .map_err(Into::into)
                        }
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_persist_trait_with_type_conversion() {
        // Arrange - field with #[fabrique(as = "String")]
        let input = parse_quote! {
            struct Anvil {
                id: String,
                #[fabrique(as = "String")]
                status: Status,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = PersistCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - conversion is now handled by the query builder's .set(),
        // so persist codegen should NOT contain try_from or Error::Conversion
        let result_str = result.to_string();
        assert!(
            !result_str.contains("try_from"),
            "should not contain try_from — conversion is handled by .set(). Got:\n{result_str}"
        );
        assert!(
            !result_str.contains("Conversion"),
            "should not contain Conversion error handling. Got:\n{result_str}"
        );
    }
}
