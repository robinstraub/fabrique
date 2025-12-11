use crate::{
    analysis::{Analysis, ast::ModelField},
    query_builder::QueryBuilderCodegen,
};
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for persistable trait implementation.
pub struct PersistableCodegen<'a> {
    /// Analysis output containing fields and relations.
    analysis: &'a Analysis<'a>,

    query_builder: &'a QueryBuilderCodegen<'a>,
}

impl<'a> PersistableCodegen<'a> {
    /// Creates a new code generator for Persistable trait implementation.
    pub fn new(analysis: &'a Analysis<'a>, query_builder: &'a QueryBuilderCodegen) -> Self {
        Self {
            analysis,
            query_builder,
        }
    }

    /// Generates the complete Persistable trait implementation as a token stream.
    ///
    /// This method generates:
    /// - `FromRow` trait implementation for database row mapping
    /// - `Persistable` trait implementation
    /// - Column marker constants for type-safe query building
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let query_builder_ident = &self.query_builder.query_builder_ident;
        let fn_all = self.generate_fn_all();
        let fn_create = self.generate_fn_create();
        let fn_delete = self.generate_fn_delete();
        let fn_destroy = self.generate_fn_destroy();
        let fn_query = self.generate_fn_query(query_builder_ident);
        let ty_primary_key = self.generate_ty_primary_key();
        let column_constants = self.generate_column_constants();
        let from_row_impl = self.generate_impl_from_row();

        let generated = quote! {
            #from_row_impl

            impl ::fabrique::Persistable for #base_struct_ident {
                type Connection = sqlx::Pool<sqlx::Postgres>;

                type Error = sqlx::Error;

                type PrimaryKey = #ty_primary_key;

                type QueryBuilder = #query_builder_ident;

                #fn_create

                #fn_destroy

                #fn_delete

                #fn_all

                #fn_query
            }

            impl #base_struct_ident {
                #column_constants
            }
        };

        generated
    }

    /// Generates the `all()` associated function.
    fn generate_fn_all(&self) -> TokenStream {
        let query = &self.analysis.base_select_query;

        quote! {
            async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                sqlx::query_as::<_, Self>(#query).fetch_all(connection).await
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
            match &field._as {
                Some(ty) => quote! { #ty::from(self.#ident) },
                None => quote! { self.#ident },
            }
        });

        quote! {
            async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                sqlx::query_as::<_, Self>(#query)
                    #(.bind(#field_bindings))*
                    .fetch_one(connection)
                    .await
            }
        }
    }

    fn generate_fn_destroy(&self) -> TokenStream {
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
            "DELETE FROM {} WHERE {}",
            self.analysis.model.table_name, clause
        );

        let binds = match primary_key.as_slice() {
            [ModelField { ident, .. }] => quote! { .bind(#ident) },
            composite => {
                let indices = (0..composite.len()).map(syn::Index::from);
                quote! { #(.bind(id.#indices))* }
            }
        };

        quote! {
            async fn destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                sqlx::query(#query)
                    #binds
                    .execute(connection)
                    .await?;
                Ok(())
            }
        }
    }

    fn generate_fn_delete(&self) -> TokenStream {
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
            "DELETE FROM {} WHERE {}",
            self.analysis.model.table_name, clause
        );

        let bindings = primary_key.map(|ModelField { ident, .. }| quote! { self.#ident });

        quote! {
            async fn delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                sqlx::query(#query)#(.bind(#bindings))*.execute(connection).await?;

                Ok(())
            }
        }
    }

    /// Generates column constants for type-safe query building.
    fn generate_column_constants(&self) -> TokenStream {
        let constants = self
            .analysis
            .fields
            .iter()
            .map(|field| {
                let field_ident = &field.ident;
                let field_type = &field.ty;
                let const_name = syn::Ident::new(
                    &field_ident.to_string().to_uppercase(),
                    field_ident.span(),
                );
                let column_name = field_ident.to_string();

                Some(quote! {
                    pub const #const_name: ::fabrique::ColumnMarker<#field_type> = ::fabrique::ColumnMarker::new(#column_name);
                })
            });

        quote! {
            #(#constants)*
        }
    }

    /// Generates the `query()` function.
    fn generate_fn_query(&self, query_builder_ident: &syn::Ident) -> TokenStream {
        quote! {
            fn query() -> Self::QueryBuilder {
                #query_builder_ident::new()
            }
        }
    }

    /// Generates the `FromRow` trait implementation.
    ///
    /// This implementation handles automatic type conversions for fields with the `as` attribute.
    fn generate_impl_from_row(&self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        // Generate field assignments
        let field_assignments = self.analysis.fields.iter().map(|field| {
            let field_ident = &field.ident;
            let column_name = field.ident.to_string();

            match &field._as {
                Some(intermediate_ty) => {
                    // Field has `as` attribute, need to convert from intermediate type using TryFrom
                    quote! {
                        #field_ident: row.try_get::<#intermediate_ty, _>(#column_name)?
                            .try_into()
                            .map_err(|e| ::sqlx::Error::Decode(Box::new(e)))?
                    }
                }
                None => {
                    // No conversion needed, read directly
                    quote! {
                        #field_ident: row.try_get(#column_name)?
                    }
                }
            }
        });

        quote! {
            impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for #base_struct_ident {
                fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
                    use ::sqlx::Row;
                    Ok(Self {
                        #(#field_assignments),*
                    })
                }
            }
        }
    }

    fn generate_ty_primary_key(&self) -> TokenStream {
        // todo: use the primary_keys attr from analysis: we're insterting the 'id' when no pk.
        // todo: actually we should mutate the id column to mark it as primary
        let primary_keys: Vec<&ModelField> = self
            .analysis
            .fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        match primary_keys.as_slice() {
            [simple] => {
                let ty = &simple.ty;
                quote! { #ty }
            }
            composite => {
                let tys = composite.iter().map(|field| &field.ty);
                quote! { (#(#tys),*) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_builder::QueryBuilderCodegen;
    use syn::parse_quote;

    #[test]
    fn test_a_basic_struct_can_derive_persistable() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for Anvil {
                    fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
                        use ::sqlx::Row;
                        Ok(Self {
                            id: row.try_get("id")?
                        })
                    }
                }

                impl ::fabrique::Persistable for Anvil {
                    type Connection = sqlx::Pool<sqlx::Postgres>;
                    type Error = sqlx::Error;
                    type PrimaryKey = String;
                    type QueryBuilder = AnvilQueryBuilder;

                    async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                        sqlx::query_as::<_, Self>("INSERT INTO anvils (id) VALUES ($1) RETURNING id")
                            .bind(self.id)
                            .fetch_one(connection)
                            .await
                    }

                    async fn destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        sqlx::query("DELETE FROM anvils WHERE id = $1").bind(id).execute(connection).await?;
                        Ok(())
                    }

                    async fn delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("DELETE FROM anvils WHERE id = $1").bind(self.id).execute(connection).await?;
                        Ok(())
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as::<_, Self>("SELECT id FROM anvils").fetch_all(connection).await
                    }

                    fn query() -> Self::QueryBuilder {
                        AnvilQueryBuilder::new()
                    }
                }

                impl Anvil {
                    pub const ID: ::fabrique::ColumnMarker<String> = ::fabrique::ColumnMarker::new("id");
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_composite_keys() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil {
            #[fabrique(primary_key)]
            user_id: uuid::Uuid,

            #[fabrique(primary_key)]
            organization_id: uuid::Uuid,
        } };
        let analysis = Analysis::from(&input).unwrap();
        let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
        let codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for Anvil {
                    fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
                        use ::sqlx::Row;
                        Ok(Self {
                            user_id: row.try_get("user_id")?,
                            organization_id: row.try_get("organization_id")?
                        })
                    }
                }

                impl ::fabrique::Persistable for Anvil {
                    type Connection = sqlx::Pool<sqlx::Postgres>;
                    type Error = sqlx::Error;
                    type PrimaryKey = (uuid::Uuid, uuid::Uuid);
                    type QueryBuilder = AnvilQueryBuilder;

                    async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
                        sqlx::query_as::<_, Self>("INSERT INTO anvils (user_id, organization_id) VALUES ($1, $2) RETURNING user_id, organization_id")
                            .bind(self.user_id)
                            .bind(self.organization_id)
                            .fetch_one(connection)
                            .await
                    }

                    async fn destroy(connection: &Self::Connection, id: Self::PrimaryKey) -> Result<(), Self::Error> {
                        sqlx::query("DELETE FROM anvils WHERE user_id = $1 AND organization_id = $2").bind(id.0).bind(id.1).execute(connection).await?;
                        Ok(())
                    }

                    async fn delete(self, connection: &Self::Connection) -> Result<(), Self::Error> {
                        sqlx::query("DELETE FROM anvils WHERE user_id = $1 AND organization_id = $2").bind(self.user_id).bind(self.organization_id).execute(connection).await?;
                        Ok(())
                    }

                    async fn all(connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
                        sqlx::query_as::<_, Self>("SELECT user_id, organization_id FROM anvils").fetch_all(connection).await
                    }

                    fn query() -> Self::QueryBuilder {
                        AnvilQueryBuilder::new()
                    }
                }

                impl Anvil {
                    pub const USER_ID: ::fabrique::ColumnMarker<uuid::Uuid> = ::fabrique::ColumnMarker::new("user_id");
                    pub const ORGANIZATION_ID: ::fabrique::ColumnMarker<uuid::Uuid> = ::fabrique::ColumnMarker::new("organization_id");
                }
            }
            .to_string()
        )
    }

    #[test]
    fn test_codegen_fail_explicitly() {
        // Arrange the codegen
        let input = parse_quote! { enum Anvil {} };

        // Act the call to the codegen
        let result = Analysis::from(&input);

        // Assert the result
        assert!(result.is_err());
    }
}
