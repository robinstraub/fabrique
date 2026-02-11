use std::collections::BTreeMap;

use crate::Analysis;
use heck::ToSnakeCase;
use pluralizer::pluralize;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub struct HasManyCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> HasManyCodegen<'a> {
    /// Creates a new code generator for `HasMany` methods.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    pub fn generate(self) -> TokenStream {
        let child_struct = &self.analysis.ident;

        // Group belongs_to relations by parent type.
        // If any relation to a parent has an alias, generate a generic method;
        // otherwise generate a simple method.
        let mut by_parent: BTreeMap<String, (&Ident, bool)> = BTreeMap::new();
        for (_, relation) in self.analysis.belongs_to() {
            let entry = by_parent
                .entry(relation.referenced_type.to_string())
                .or_insert((&relation.referenced_type, false));
            if relation.alias.is_some() {
                entry.1 = true;
            }
        }

        let method_name = pluralize(&child_struct.to_string().to_snake_case(), 2, false);
        let method_name = Ident::new(&method_name, child_struct.span());

        let methods = by_parent.into_values().map(|(parent_struct, has_alias)| {
            if has_alias {
                quote! {
                    impl #parent_struct {
                        pub fn #method_name<Alias>(
                            &self,
                        ) -> ::fabrique::model::QueryBuilder<
                            ::fabrique::model::Building<
                                <#child_struct as ::fabrique::database::DatabaseAware>::Database,
                                ::fabrique::sql::Filtered<::fabrique::sql::Selected>,
                            >,
                            ::fabrique::model::join::Joined<(#child_struct, ()), ()>,
                            #child_struct,
                        >
                        where
                            #child_struct: ::fabrique::BelongsTo<Self, Alias>,
                            <#child_struct as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn: ::fabrique::database::Column<#child_struct>,
                            for<'q> <<#child_struct as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn as ::fabrique::database::Column<
                                #child_struct,
                            >>::DbType: ::sqlx::Encode<'q, <#child_struct as ::fabrique::database::DatabaseAware>::Database>
                                + ::sqlx::Type<<#child_struct as ::fabrique::database::DatabaseAware>::Database>,
                            <Self as ::fabrique::Model>::PrimaryKey: Into<
                                <<#child_struct as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn as ::fabrique::database::Column<
                                    #child_struct,
                                >>::Type,
                            >,
                        {
                            let pk = <Self as ::fabrique::Model>::primary_key(self);
                            let fk = <#child_struct as ::fabrique::BelongsTo<Self, Alias>>::foreign_key_column();
                            <#child_struct as ::fabrique::Query>::query()
                                .select_as::<#child_struct, _>()
                                .r#where(fk, "=", pk)
                        }
                    }
                }
            } else {
                quote! {
                    impl #parent_struct {
                        pub fn #method_name(&self) -> ::fabrique::model::QueryBuilder<
                            ::fabrique::model::Building<
                                <#child_struct as ::fabrique::database::DatabaseAware>::Database,
                                ::fabrique::sql::Filtered<::fabrique::sql::Selected>
                            >,
                            ::fabrique::model::join::Joined<(#child_struct, ()), ()>,
                            #child_struct
                        > {
                            let pk = <Self as ::fabrique::Model>::primary_key(self);
                            let fk = <#child_struct as ::fabrique::BelongsTo<Self>>::foreign_key_column();
                            <#child_struct as ::fabrique::Query>::query()
                                .select_as::<#child_struct, _>()
                                .r#where(fk, "=", pk)
                        }
                    }
                }
            }
        });

        quote! {
            #(#methods)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_has_many_methods() {
        // Arrange a codegen
        let input = parse_quote! {
            struct Message {
                id: Uuid,

                #[fabrique(belongs_to = "User", alias = "Sender")]
                pub sender_id: Uuid,

                #[fabrique(belongs_to = "User", alias = "Recipient")]
                pub recipient_id: Uuid,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HasManyCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl User {
                    pub fn messages<Alias>(
                        &self,
                    ) -> ::fabrique::model::QueryBuilder<
                        ::fabrique::model::Building<
                            <Message as ::fabrique::database::DatabaseAware>::Database,
                            ::fabrique::sql::Filtered<::fabrique::sql::Selected>,
                        >,
                        ::fabrique::model::join::Joined<(Message, ()), ()>,
                        Message,
                    >
                    where
                        Message: ::fabrique::BelongsTo<Self, Alias>,
                        <Message as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn: ::fabrique::database::Column<Message>,
                        for<'q> <<Message as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn as ::fabrique::database::Column<
                            Message,
                        >>::DbType: ::sqlx::Encode<'q, <Message as ::fabrique::database::DatabaseAware>::Database>
                            + ::sqlx::Type<<Message as ::fabrique::database::DatabaseAware>::Database>,
                        <Self as ::fabrique::Model>::PrimaryKey: Into<
                            <<Message as ::fabrique::BelongsTo<Self, Alias>>::ForeignKeyColumn as ::fabrique::database::Column<
                                Message,
                            >>::Type,
                        >,
                    {
                        let pk = <Self as ::fabrique::Model>::primary_key(self);
                        let fk = <Message as ::fabrique::BelongsTo<Self, Alias>>::foreign_key_column();
                        <Message as ::fabrique::Query>::query()
                            .select_as::<Message, _>()
                            .r#where(fk, "=", pk)
                    }
                }
            }.to_string(),
        )
    }

    #[test]
    fn test_generate_has_many_method_without_alias() {
        // Arrange
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User")]
                user_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HasManyCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl User {
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
            .to_string(),
        )
    }
}
