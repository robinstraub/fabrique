use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for Model trait implementation.
pub struct ModelCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> ModelCodegen<'a> {
    /// Creates a new code generator for Model trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `Model` trait implementation and lazy loading methods.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let ty_primary_key = self.generate_ty_primary_key();
        let ty_soft_delete_column = self.generate_ty_soft_delete_column();
        let fn_primary_key = self.generate_fn_primary_key();
        let fn_table_name = self.generate_fn_table_name();
        let fn_columns = self.generate_fn_columns();
        let fn_qualified_columns = self.generate_fn_qualified_columns();
        let fn_primary_key_columns = self.generate_fn_primary_key_columns();
        let fn_uses_soft_delete = self.generate_fn_soft_delete_column();
        let lazy_loading_methods = self.generate_lazy_loading_methods();

        quote! {
            impl ::fabrique::Model for #base_struct_ident {
                type PrimaryKey = #ty_primary_key;
                type SoftDeleteColumn = #ty_soft_delete_column;

                #fn_primary_key

                #fn_table_name

                #fn_columns

                #fn_qualified_columns

                #fn_primary_key_columns

                #fn_uses_soft_delete
            }

            #lazy_loading_methods
        }
    }

    fn generate_ty_primary_key(&self) -> TokenStream {
        let primary_keys: Vec<_> = self
            .analysis
            .column_fields
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

    fn generate_ty_soft_delete_column(&self) -> TokenStream {
        match self
            .analysis
            .column_fields
            .iter()
            .find(|field| field.soft_delete)
        {
            Some(field) => {
                let ty = &field.column_type;
                quote! { #ty }
            }
            None => quote! { ::fabrique::Nil },
        }
    }

    fn generate_fn_primary_key(&self) -> TokenStream {
        let primary_keys: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .collect();

        match primary_keys.as_slice() {
            [simple] => {
                let ident = &simple.ident;
                quote! {
                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.#ident.clone()
                    }
                }
            }
            composite => {
                let idents = composite.iter().map(|field| {
                    let ident = &field.ident;
                    quote! { self.#ident.clone() }
                });
                quote! {
                    fn primary_key(&self) -> Self::PrimaryKey {
                        (#(#idents),*)
                    }
                }
            }
        }
    }

    fn generate_fn_table_name(&self) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        quote! {
            fn table_name() -> &'static str {
                #table_name
            }
        }
    }

    fn generate_fn_columns(&self) -> TokenStream {
        let columns: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .map(|field| field.column.as_str())
            .collect();

        quote! {
            fn columns() -> &'static [&'static str] {
                &[#(#columns),*]
            }
        }
    }

    fn generate_fn_qualified_columns(&self) -> TokenStream {
        let table_name = &self.analysis.model.table_name;
        let qualified_columns: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .map(|field| format!("{}.{}", table_name, field.column))
            .collect();

        quote! {
            fn qualified_columns() -> &'static [&'static str] {
                &[#(#qualified_columns),*]
            }
        }
    }

    fn generate_fn_primary_key_columns(&self) -> TokenStream {
        let pk_columns: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|field| field.primary_key)
            .map(|field| field.column.as_str())
            .collect();

        quote! {
            fn primary_key_columns() -> &'static [&'static str] {
                &[#(#pk_columns),*]
            }
        }
    }

    fn generate_fn_soft_delete_column(&self) -> TokenStream {
        let soft_delete = self
            .analysis
            .column_fields
            .iter()
            .find(|field| field.soft_delete)
            .map(|field| {
                let const_column_name = &field.const_column_name;
                quote! { Some(Self::#const_column_name) }
            });

        match soft_delete {
            Some(soft_delete) => quote! {
                fn soft_delete_column() -> Option<Self::SoftDeleteColumn> {
                    #soft_delete
                }
            },
            None => quote! {},
        }
    }

    fn generate_lazy_loading_methods(&self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        // One-to-many lazy loading (without through).
        let one_to_many_methods = self.analysis.one_to_many_fields().map(|field| {
            let method_name = &field.ident;
            let target_type = &field.target_type;

            // Use BelongsTo<Self, Alias> when alias is specified for disambiguation,
            // otherwise use BelongsTo<Self> (only valid for unique relations).
            let fk_expr = match &field.alias {
                Some(alias) => {
                    quote! { <#target_type as ::fabrique::BelongsTo<Self, #alias>>::foreign_key_column() }
                }
                None => {
                    quote! { <#target_type as ::fabrique::BelongsTo<Self>>::foreign_key_column() }
                }
            };

            quote! {
                pub fn #method_name(&self) -> ::fabrique::model::QueryBuilder<
                    ::fabrique::model::Building<
                        <#target_type as ::fabrique::database::DatabaseAware>::Database,
                        ::fabrique::sql::Filtered<::fabrique::sql::Selected>
                    >,
                    ::fabrique::model::join::Joined<(#target_type, ()), ()>,
                    #target_type
                > {
                    let pk = <Self as ::fabrique::Model>::primary_key(self);
                    let fk = #fk_expr;
                    <#target_type as ::fabrique::Query>::query()
                        .select_as::<#target_type, _>()
                        .r#where(fk, "=", pk)
                }
            }
        });

        // Many-to-many lazy loading (with through).
        let pk_fields: Vec<_> = self
            .analysis
            .column_fields
            .iter()
            .filter(|f| f.primary_key)
            .collect();

        let where_clauses: Vec<_> = pk_fields
            .iter()
            .map(|pk_field| {
                let const_name = &pk_field.const_column_name;
                let field_ident = &pk_field.ident;
                quote! {
                    .r#where(Self::#const_name, "=", self.#field_ident)
                }
            })
            .collect();

        let many_to_many_methods = self.analysis.many_to_many_fields().map(|field| {
            let method_name = &field.ident;
            let target_type = &field.target_type;
            let join_type = field.through.as_ref().unwrap();

            quote! {
                pub fn #method_name(&self) -> ::fabrique::model::QueryBuilder<
                    ::fabrique::model::Building<
                        <#target_type as ::fabrique::database::DatabaseAware>::Database,
                        ::fabrique::sql::Filtered<::fabrique::sql::Selected>
                    >,
                    ::fabrique::model::join::Joined<(#base_struct_ident, ()), ::fabrique::model::join::Joined<(#join_type, ()), ::fabrique::model::join::Joined<(#target_type, ()), ()>>>,
                    #target_type
                > {
                    <#target_type as ::fabrique::Query>::query()
                        .join::<#join_type>()
                        .join_through::<#base_struct_ident, #join_type, _>()
                        .select_as::<#target_type, _>()
                        #(#where_clauses)*
                }
            }
        });

        quote! {
            impl #base_struct_ident {
                #(#one_to_many_methods)*
                #(#many_to_many_methods)*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_model_trait() {
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Anvil {
                    type PrimaryKey = String;
                    type SoftDeleteColumn = ::fabrique::Nil;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn columns() ->  &'static [&'static str] {
                        &["id"]
                    }

                    fn qualified_columns() -> &'static [&'static str] {
                        &["anvils.id"]
                    }

                    fn primary_key_columns() -> &'static [&'static str] {
                        &["id"]
                    }
                }

                impl Anvil {}
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_model_trait_with_soft_delete() {
        // Arrange
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Anvil {
                    type PrimaryKey = String;
                    type SoftDeleteColumn = AnvilDeletedAtColumn;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "anvils"
                    }

                    fn columns() ->  &'static [&'static str] {
                        &["id", "deleted_at"]
                    }

                    fn qualified_columns() -> &'static [&'static str] {
                        &["anvils.id", "anvils.deleted_at"]
                    }

                    fn primary_key_columns() -> &'static [&'static str] {
                        &["id"]
                    }

                    fn soft_delete_column() -> Option<Self::SoftDeleteColumn> {
                        Some(Self::DELETED_AT)
                    }
                }

                impl Anvil {}
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_lazy_loading_methods() {
        // Arrange - HasMany infers FK via BelongsTo trait
        let input = parse_quote! {
            struct Customer {
                id: String,
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - verify the lazy loading method uses BelongsTo trait
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Customer {
                    type PrimaryKey = String;
                    type SoftDeleteColumn = ::fabrique::Nil;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "customers"
                    }

                    fn columns() -> &'static [&'static str] {
                        &["id"]
                    }

                    fn qualified_columns() -> &'static [&'static str] {
                        &["customers.id"]
                    }

                    fn primary_key_columns() -> &'static [&'static str] {
                        &["id"]
                    }
                }

                impl Customer {
                    pub fn orders(&self) -> ::fabrique::model::QueryBuilder<
                        ::fabrique::model::Building<
                            <Order as ::fabrique::database::DatabaseAware>::Database,
                            ::fabrique::sql::Filtered<::fabrique::sql::Selected>
                        >,
                        ::fabrique::model::join::Joined<(Order, ()), ()>,
                        Order
                    > {
                        let pk = <Self as ::fabrique::Model>::primary_key(self);
                        let fk = <Order as ::fabrique::BelongsTo<Self>>::foreign_key_column();
                        <Order as ::fabrique::Query>::query()
                            .select_as::<Order, _>()
                            .r#where(fk, "=", pk)
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_lazy_loading_with_alias() {
        // Arrange - HasMany with alias for disambiguation
        let input = parse_quote! {
            struct User {
                id: String,
                #[fabrique(alias = "Sender")]
                sent_messages: HasMany<Message>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate().to_string();

        // Assert - verify the lazy loading method uses BelongsTo<Self, Alias>
        assert!(
            result.contains(
                &quote::quote! {
                    <Message as ::fabrique::BelongsTo<Self, Sender>>::foreign_key_column()
                }
                .to_string()
            ),
            "Should use BelongsTo with alias. Generated: {}",
            result
        );
    }

    #[test]
    fn test_many_to_many_lazy_loading() {
        // Arrange - Order has many Products through OrderLine
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(through = "OrderLine")]
                products: HasMany<Product>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - verify the lazy loading method uses join_through
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Model for Order {
                    type PrimaryKey = String;
                    type SoftDeleteColumn = ::fabrique::Nil;

                    fn primary_key(&self) -> Self::PrimaryKey {
                        self.id.clone()
                    }

                    fn table_name() -> &'static str {
                        "orders"
                    }

                    fn columns() -> &'static [&'static str] {
                        &["id"]
                    }

                    fn qualified_columns() -> &'static [&'static str] {
                        &["orders.id"]
                    }

                    fn primary_key_columns() -> &'static [&'static str] {
                        &["id"]
                    }
                }

                impl Order {
                    pub fn products(&self) -> ::fabrique::model::QueryBuilder<
                        ::fabrique::model::Building<
                            <Product as ::fabrique::database::DatabaseAware>::Database,
                            ::fabrique::sql::Filtered<::fabrique::sql::Selected>
                        >,
                        ::fabrique::model::join::Joined<(Order, ()), ::fabrique::model::join::Joined<(OrderLine, ()), ::fabrique::model::join::Joined<(Product, ()), ()>>>,
                        Product
                    > {
                        <Product as ::fabrique::Query>::query()
                            .join::<OrderLine>()
                            .join_through::<Order, OrderLine, _>()
                            .select_as::<Product, _>()
                            .r#where(Self::ID, "=", self.id)
                    }
                }
            }
            .to_string()
        );
    }
}
