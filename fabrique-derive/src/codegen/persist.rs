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

        quote! {
            impl ::fabrique::Persist for #base_struct_ident {
                #fn_create

                #fn_save
            }
        }
    }

    /// Generates the `create()` method using the Layer 2 query builder.
    fn generate_fn_create(&self) -> TokenStream {
        // Generate .set() calls for each column field
        let set_calls = self.analysis.column_fields.iter().map(|field| {
            let const_name = &field.const_column_name;
            let ident = &field.ident;
            let field_name = ident.to_string();
            match &field.r#as {
                Some(db_ty) => {
                    let rust_ty = &field.ty;
                    quote! {
                        .set(Self::#const_name, {
                            let value_str = format!("{:?}", &self.#ident);
                            <#db_ty>::try_from(self.#ident).map_err(|e| {
                                ::fabrique::Error::Conversion {
                                    field: #field_name.to_string(),
                                    from: stringify!(#rust_ty),
                                    to: stringify!(#db_ty),
                                    value: value_str,
                                    reason: e.to_string(),
                                    direction: ::fabrique::error::ConversionDirection::ToDb,
                                }
                            })?
                        })
                    }
                }
                None => quote! { .set(Self::#const_name, self.#ident) },
            }
        });

        #[cfg(any(feature = "postgres", feature = "sqlite"))]
        return quote! {
            fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    <Self as ::fabrique::Query>::query()
                        .insert()
                        #(#set_calls)*
                        .returning()
                        .first_or_fail(&mut *conn)
                        .await
                        .map_err(Into::into)
                }
            }
        };

        #[cfg(feature = "mysql")]
        return quote! {
            fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let pk = self.primary_key();
                    <Self as ::fabrique::Query>::query()
                        .insert()
                        #(#set_calls)*
                        .execute(&mut *conn)
                        .await?;
                    <Self as ::fabrique::Query>::find(&mut *conn, pk)
                        .await
                        .map_err(Into::into)
                }
            }
        };
    }

    /// Generates the `save()` method using the Layer 2 query builder.
    fn generate_fn_save(&self) -> TokenStream {
        // Generate .set() calls for each column field
        let set_calls = self.analysis.column_fields.iter().map(|field| {
            let const_name = &field.const_column_name;
            let ident = &field.ident;
            let field_name = ident.to_string();
            match &field.r#as {
                Some(db_ty) => {
                    let rust_ty = &field.ty;
                    quote! {
                        .set(Self::#const_name, {
                            let value_str = format!("{:?}", &self.#ident);
                            <#db_ty>::try_from(self.#ident).map_err(|e| {
                                ::fabrique::Error::Conversion {
                                    field: #field_name.to_string(),
                                    from: stringify!(#rust_ty),
                                    to: stringify!(#db_ty),
                                    value: value_str,
                                    reason: e.to_string(),
                                    direction: ::fabrique::error::ConversionDirection::ToDb,
                                }
                            })?
                        })
                    }
                }
                None => quote! { .set(Self::#const_name, self.#ident) },
            }
        });

        #[cfg(any(feature = "postgres", feature = "sqlite"))]
        return quote! {
            fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    <Self as ::fabrique::Query>::query()
                        .insert()
                        #(#set_calls)*
                        .on_conflict()
                        .do_update()
                        .returning()
                        .first_or_fail(&mut *conn)
                        .await
                        .map_err(Into::into)
                }
            }
        };

        #[cfg(feature = "mysql")]
        return quote! {
            fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
            {
                async move {
                    let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                    let pk = self.primary_key();
                    <Self as ::fabrique::Query>::query()
                        .insert()
                        #(#set_calls)*
                        .on_conflict()
                        .do_update()
                        .execute(&mut *conn)
                        .await?;
                    <Self as ::fabrique::Query>::find(&mut *conn, pk)
                        .await
                        .map_err(Into::into)
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    #[test]
    fn test_generate_persist_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = PersistCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert — PostgreSQL and SQLite use RETURNING clause
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Persist for Anvil {
                    fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            <Self as ::fabrique::Query>::query()
                                .insert()
                                .set(Self::ID, self.id)
                                .returning()
                                .first_or_fail(&mut *conn)
                                .await
                                .map_err(Into::into)
                        }
                    }

                    fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            <Self as ::fabrique::Query>::query()
                                .insert()
                                .set(Self::ID, self.id)
                                .on_conflict()
                                .do_update()
                                .returning()
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

    #[cfg(feature = "mysql")]
    #[test]
    fn test_generate_persist_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = PersistCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert — MySQL lacks RETURNING, so we INSERT then SELECT
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Persist for Anvil {
                    fn create<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let pk = self.primary_key();
                            <Self as ::fabrique::Query>::query()
                                .insert()
                                .set(Self::ID, self.id)
                                .execute(&mut *conn)
                                .await?;
                            <Self as ::fabrique::Query>::find(&mut *conn, pk)
                                .await
                                .map_err(Into::into)
                        }
                    }

                    fn save<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = Self::Database> + Send + 'e,
                    {
                        async move {
                            let mut conn = executor.acquire().await.map_err(|e| ::fabrique::Error::from(e))?;
                            let pk = self.primary_key();
                            <Self as ::fabrique::Query>::query()
                                .insert()
                                .set(Self::ID, self.id)
                                .on_conflict()
                                .do_update()
                                .execute(&mut *conn)
                                .await?;
                            <Self as ::fabrique::Query>::find(&mut *conn, pk)
                                .await
                                .map_err(Into::into)
                        }
                    }
                }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
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

        // Assert - should generate TryFrom conversion code
        let result_str = result.to_string();
        assert!(
            result_str.contains("try_from"),
            "should use try_from for type conversion"
        );
        assert!(
            result_str.contains("Error :: Conversion"),
            "should handle conversion errors"
        );
    }
}
