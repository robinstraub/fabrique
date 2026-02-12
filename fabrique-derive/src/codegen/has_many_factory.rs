use std::collections::BTreeMap;

use crate::Analysis;
use heck::ToSnakeCase;
use pluralizer::pluralize;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Code generator for `has_` factory methods from the child's `belongs_to`.
///
/// For each `belongs_to` relation, generates:
/// - A `DeferredFactory` implementation struct
/// - A `has_<children>` inherent method on the parent factory
///
/// This is the factory counterpart of `HasManyCodegen` (which generates
/// loading methods). Both use the same BTreeMap grouping by parent type.
pub struct HasManyFactoryCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> HasManyFactoryCodegen<'a> {
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    pub fn generate(self) -> TokenStream {
        let child_struct = &self.analysis.ident;
        let child_factory = Ident::new(&format!("{}Factory", child_struct), child_struct.span());

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
        let method_name = Ident::new(&format!("has_{}", method_name), child_struct.span());

        let impls = by_parent.into_values().map(|(parent_struct, has_alias)| {
            let parent_factory =
                Ident::new(&format!("{}Factory", parent_struct), parent_struct.span());
            let pending_struct = Ident::new(
                &format!("__Deferred{}For{}", child_struct, parent_struct),
                child_struct.span(),
            );

            if has_alias {
                Self::generate_aliased(
                    child_struct,
                    &child_factory,
                    parent_struct,
                    &parent_factory,
                    &pending_struct,
                    &method_name,
                )
            } else {
                Self::generate_simple(
                    &child_factory,
                    parent_struct,
                    &parent_factory,
                    &pending_struct,
                    &method_name,
                )
            }
        });

        quote! { #(#impls)* }
    }

    fn generate_simple(
        child_factory: &Ident,
        parent_struct: &Ident,
        parent_factory: &Ident,
        pending_struct: &Ident,
        method_name: &Ident,
    ) -> TokenStream {
        quote! {
            struct #pending_struct<DB: ::fabrique::Dialect> {
                factory: #child_factory<DB>,
                count: usize,
            }

            impl<DB: ::fabrique::Dialect> Clone for #pending_struct<DB> {
                fn clone(&self) -> Self {
                    Self { factory: self.factory.clone(), count: self.count }
                }
            }

            impl<DB: ::fabrique::Dialect> fabrique::DeferredFactory<
                <#parent_struct as fabrique::Model>::PrimaryKey,
                DB,
            > for #pending_struct<DB>
            where
                #child_factory<DB>: fabrique::SetForeignKey<#parent_struct> + fabrique::Factory<DB>,
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
            {
                fn create<'a>(
                    &'a self,
                    parent_pk: <#parent_struct as fabrique::Model>::PrimaryKey,
                    conn: &'a mut <DB as ::sqlx::Database>::Connection,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        for _ in 0..self.count {
                            let child_factory = <#child_factory<DB> as fabrique::SetForeignKey<#parent_struct>>::set_foreign_key(
                                self.factory.clone(),
                                parent_pk.clone(),
                            );
                            fabrique::Factory::create(child_factory, &mut *conn).await?;
                        }
                        Ok(())
                    })
                }
            }

            impl<DB: ::fabrique::Dialect> #parent_factory<DB> {
                pub fn #method_name(mut self, factory: #child_factory<DB>, count: usize) -> Self
                where
                    #child_factory<DB>: fabrique::SetForeignKey<#parent_struct> + fabrique::Factory<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                {
                    self.children.push(::std::sync::Arc::new(#pending_struct { factory, count }));
                    self
                }
            }
        }
    }

    fn generate_aliased(
        child_struct: &Ident,
        child_factory: &Ident,
        parent_struct: &Ident,
        parent_factory: &Ident,
        pending_struct: &Ident,
        method_name: &Ident,
    ) -> TokenStream {
        quote! {
            struct #pending_struct<DB: ::fabrique::Dialect, Alias> {
                factory: #child_factory<DB>,
                count: usize,
                _alias: ::std::marker::PhantomData<Alias>,
            }

            impl<DB: ::fabrique::Dialect, Alias> Clone for #pending_struct<DB, Alias> {
                fn clone(&self) -> Self {
                    Self { factory: self.factory.clone(), count: self.count, _alias: ::std::marker::PhantomData }
                }
            }

            impl<DB: ::fabrique::Dialect, Alias> fabrique::DeferredFactory<
                <#parent_struct as fabrique::Model>::PrimaryKey,
                DB,
            > for #pending_struct<DB, Alias>
            where
                #child_struct: fabrique::BelongsTo<#parent_struct, Alias>,
                #child_factory<DB>: fabrique::SetForeignKey<#parent_struct, Alias> + fabrique::Factory<DB>,
                Alias: Send + Sync + Clone + 'static,
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
            {
                fn create<'a>(
                    &'a self,
                    parent_pk: <#parent_struct as fabrique::Model>::PrimaryKey,
                    conn: &'a mut <DB as ::sqlx::Database>::Connection,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        for _ in 0..self.count {
                            let child_factory = <#child_factory<DB> as fabrique::SetForeignKey<#parent_struct, Alias>>::set_foreign_key(
                                self.factory.clone(),
                                parent_pk.clone(),
                            );
                            fabrique::Factory::create(child_factory, &mut *conn).await?;
                        }
                        Ok(())
                    })
                }
            }

            impl<DB: ::fabrique::Dialect> #parent_factory<DB> {
                pub fn #method_name<Alias>(mut self, factory: #child_factory<DB>, count: usize) -> Self
                where
                    #child_struct: fabrique::BelongsTo<#parent_struct, Alias>,
                    #child_factory<DB>: fabrique::SetForeignKey<#parent_struct, Alias> + fabrique::Factory<DB>,
                    Alias: Send + Sync + Clone + 'static,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                {
                    self.children.push(::std::sync::Arc::new(#pending_struct {
                        factory,
                        count,
                        _alias: ::std::marker::PhantomData,
                    }));
                    self
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
    fn test_generate_deferred_factory_simple() {
        // Arrange — Order belongs_to User (no alias)
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User")]
                user_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HasManyFactoryCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                struct __DeferredOrderForUser<DB: ::fabrique::Dialect> {
                    factory: OrderFactory<DB>,
                    count: usize,
                }

                impl<DB: ::fabrique::Dialect> Clone for __DeferredOrderForUser<DB> {
                    fn clone(&self) -> Self {
                        Self { factory: self.factory.clone(), count: self.count }
                    }
                }

                impl<DB: ::fabrique::Dialect> fabrique::DeferredFactory<
                    <User as fabrique::Model>::PrimaryKey,
                    DB,
                > for __DeferredOrderForUser<DB>
                where
                    OrderFactory<DB>: fabrique::SetForeignKey<User> + fabrique::Factory<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                {
                    fn create<'a>(
                        &'a self,
                        parent_pk: <User as fabrique::Model>::PrimaryKey,
                        conn: &'a mut <DB as ::sqlx::Database>::Connection,
                    ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                        Box::pin(async move {
                            for _ in 0..self.count {
                                let child_factory = <OrderFactory<DB> as fabrique::SetForeignKey<User>>::set_foreign_key(
                                    self.factory.clone(),
                                    parent_pk.clone(),
                                );
                                fabrique::Factory::create(child_factory, &mut *conn).await?;
                            }
                            Ok(())
                        })
                    }
                }

                impl<DB: ::fabrique::Dialect> UserFactory<DB> {
                    pub fn has_orders(mut self, factory: OrderFactory<DB>, count: usize) -> Self
                    where
                        OrderFactory<DB>: fabrique::SetForeignKey<User> + fabrique::Factory<DB>,
                        for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                    {
                        self.children.push(::std::sync::Arc::new(__DeferredOrderForUser { factory, count }));
                        self
                    }
                }
            }
            .to_string(),
        );
    }

    #[test]
    fn test_generate_deferred_factory_aliased() {
        // Arrange — Message belongs_to User via Sender and Recipient
        let input = parse_quote! {
            struct Message {
                id: String,
                #[fabrique(belongs_to = "User", alias = "Sender")]
                sender_id: String,
                #[fabrique(belongs_to = "User", alias = "Recipient")]
                recipient_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = HasManyFactoryCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert — generic method with Alias parameter
        assert_eq!(
            result.to_string(),
            quote! {
                struct __DeferredMessageForUser<DB: ::fabrique::Dialect, Alias> {
                    factory: MessageFactory<DB>,
                    count: usize,
                    _alias: ::std::marker::PhantomData<Alias>,
                }

                impl<DB: ::fabrique::Dialect, Alias> Clone for __DeferredMessageForUser<DB, Alias> {
                    fn clone(&self) -> Self {
                        Self { factory: self.factory.clone(), count: self.count, _alias: ::std::marker::PhantomData }
                    }
                }

                impl<DB: ::fabrique::Dialect, Alias> fabrique::DeferredFactory<
                    <User as fabrique::Model>::PrimaryKey,
                    DB,
                > for __DeferredMessageForUser<DB, Alias>
                where
                    Message: fabrique::BelongsTo<User, Alias>,
                    MessageFactory<DB>: fabrique::SetForeignKey<User, Alias> + fabrique::Factory<DB>,
                    Alias: Send + Sync + Clone + 'static,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                {
                    fn create<'a>(
                        &'a self,
                        parent_pk: <User as fabrique::Model>::PrimaryKey,
                        conn: &'a mut <DB as ::sqlx::Database>::Connection,
                    ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                        Box::pin(async move {
                            for _ in 0..self.count {
                                let child_factory = <MessageFactory<DB> as fabrique::SetForeignKey<User, Alias>>::set_foreign_key(
                                    self.factory.clone(),
                                    parent_pk.clone(),
                                );
                                fabrique::Factory::create(child_factory, &mut *conn).await?;
                            }
                            Ok(())
                        })
                    }
                }

                impl<DB: ::fabrique::Dialect> UserFactory<DB> {
                    pub fn has_messages<Alias>(mut self, factory: MessageFactory<DB>, count: usize) -> Self
                    where
                        Message: fabrique::BelongsTo<User, Alias>,
                        MessageFactory<DB>: fabrique::SetForeignKey<User, Alias> + fabrique::Factory<DB>,
                        Alias: Send + Sync + Clone + 'static,
                        for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                    {
                        self.children.push(::std::sync::Arc::new(__DeferredMessageForUser {
                            factory,
                            count,
                            _alias: ::std::marker::PhantomData,
                        }));
                        self
                    }
                }
            }
            .to_string(),
        );
    }
}
