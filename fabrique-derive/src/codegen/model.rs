use crate::Analysis;
use crate::analysis::ast::FieldKind;
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

                #fn_primary_key_columns

                #fn_uses_soft_delete
            }

            #lazy_loading_methods
        }
    }

    fn generate_ty_primary_key(&self) -> TokenStream {
        let primary_keys: Vec<_> = self
            .analysis
            .column_fields()
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
            .column_fields()
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
            .column_fields()
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
            .column_fields()
            .map(|field| field.column.as_str())
            .collect();

        quote! {
            fn columns() -> &'static [&'static str] {
                &[#(#columns),*]
            }
        }
    }

    fn generate_fn_primary_key_columns(&self) -> TokenStream {
        let pk_columns: Vec<_> = self
            .analysis
            .column_fields()
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
            .column_fields()
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

        let methods = self.analysis.has_many_fields().map(|field| {
            let method_name = &field.ident;
            let FieldKind::HasMany(ref target_type) = field.kind else {
                unreachable!("has_many_fields() only returns HasMany fields")
            };
            let foreign_key = field.foreign_key.as_ref().expect("HasMany requires foreign_key");

            quote! {
                pub fn #method_name(&self) -> ::fabrique::model::QueryBuilder<#target_type, ::fabrique::sql::Filtered<::fabrique::sql::Selected>> {
                    let pk = <Self as ::fabrique::Model>::primary_key(self);
                    <#target_type as ::fabrique::Query>::query()
                        .select()
                        .r#where(#foreign_key, "=", pk)
                }
            }
        });

        quote! {
            impl #base_struct_ident {
                #(#methods)*
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
        // Arrange
        let input = parse_quote! {
            struct Customer {
                id: String,

                #[fabrique(foreign_key = Order::CUSTOMER_ID)]
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = ModelCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - verify the lazy loading method is generated
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

                    fn primary_key_columns() -> &'static [&'static str] {
                        &["id"]
                    }
                }

                impl Customer {
                    pub fn orders(&self) -> ::fabrique::model::QueryBuilder<Order, ::fabrique::sql::Filtered<::fabrique::sql::Selected>> {
                        let pk = <Self as ::fabrique::Model>::primary_key(self);
                        <Order as ::fabrique::Query>::query()
                            .select()
                            .r#where(Order::CUSTOMER_ID, "=", pk)
                    }
                }
            }
            .to_string()
        );
    }
}
