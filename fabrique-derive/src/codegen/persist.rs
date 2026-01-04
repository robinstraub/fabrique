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

    /// Generates the `create()` method.
    fn generate_fn_create(&self) -> TokenStream {
        // Get field identifiers and names (column fields only)
        let columns = self
            .analysis
            .column_fields
            .iter()
            .map(|field| field.ident.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        // Generate placeholders ($1, $2, $3, ...)
        let placeholders = (1..=self.analysis.column_fields.len())
            .map(|i| format!("${}", i))
            .collect::<Vec<String>>()
            .join(", ");

        // Generate RETURNING clause
        let returning = self
            .analysis
            .column_fields
            .iter()
            .map(|field| field.column.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
            self.analysis.model.table_name, columns, placeholders, returning,
        );

        // Generate field bindings (self.field1, self.field2, ...)
        let field_bindings = self.analysis.column_fields.iter().map(|field| {
            let ident = &field.ident;
            let field_name = ident.to_string();
            match &field.r#as {
                Some(db_ty) => {
                    let rust_ty = &field.ty;
                    quote! {{
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
                    }}
                }
                None => quote! { self.#ident },
            }
        });

        quote! {
            fn create<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
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

        quote! {
            fn save<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
            where
                E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
            {
                async move {
                    <Self as ::fabrique::Query>::query()
                        .insert()
                        #(#set_calls)*
                        .on_conflict()
                        .do_update()
                        .returning()
                        .first_or_fail(executor)
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
                    fn create<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            ::sqlx::query_as::<_, Self>("INSERT INTO anvils (id) VALUES ($1) RETURNING id")
                                .bind(self.id)
                                .fetch_one(executor)
                                .await
                                .map_err(Into::into)
                        }
                    }

                    fn save<'e, E>(self, executor: E) -> impl ::std::future::Future<Output = Result<Self, Self::Error>> + Send + 'e
                    where
                        E: ::sqlx::Executor<'e, Database = Self::Database> + 'e,
                    {
                        async move {
                            <Self as ::fabrique::Query>::query()
                                .insert()
                                .set(Self::ID, self.id)
                                .on_conflict()
                                .do_update()
                                .returning()
                                .first_or_fail(executor)
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
