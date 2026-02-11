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
            #[derive(Clone)]
            struct #pending_struct {
                factory: #child_factory,
                count: usize,
            }

            impl fabrique::DeferredFactory<
                <#parent_struct as fabrique::Model>::PrimaryKey,
                <#parent_struct as fabrique::database::DatabaseAware>::Database,
            > for #pending_struct {
                fn create<'a>(
                    &'a self,
                    parent_pk: <#parent_struct as fabrique::Model>::PrimaryKey,
                    conn: &'a mut <<#parent_struct as fabrique::database::DatabaseAware>::Database as ::sqlx::Database>::Connection,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        for _ in 0..self.count {
                            let child_factory = <#child_factory as fabrique::SetForeignKey<#parent_struct>>::set_foreign_key(
                                self.factory.clone(),
                                parent_pk.clone(),
                            );
                            fabrique::Factory::create(child_factory, &mut *conn).await?;
                        }
                        Ok(())
                    })
                }
            }

            impl #parent_factory {
                pub fn #method_name(mut self, factory: #child_factory, count: usize) -> Self {
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
            #[derive(Clone)]
            struct #pending_struct<Alias> {
                factory: #child_factory,
                count: usize,
                _alias: ::std::marker::PhantomData<Alias>,
            }

            impl<Alias> fabrique::DeferredFactory<
                <#parent_struct as fabrique::Model>::PrimaryKey,
                <#parent_struct as fabrique::database::DatabaseAware>::Database,
            > for #pending_struct<Alias>
            where
                #child_struct: fabrique::BelongsTo<#parent_struct, Alias>,
                #child_factory: fabrique::SetForeignKey<#parent_struct, Alias>,
                Alias: Send + Sync + Clone + 'static,
            {
                fn create<'a>(
                    &'a self,
                    parent_pk: <#parent_struct as fabrique::Model>::PrimaryKey,
                    conn: &'a mut <<#parent_struct as fabrique::database::DatabaseAware>::Database as ::sqlx::Database>::Connection,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        for _ in 0..self.count {
                            let child_factory = <#child_factory as fabrique::SetForeignKey<#parent_struct, Alias>>::set_foreign_key(
                                self.factory.clone(),
                                parent_pk.clone(),
                            );
                            fabrique::Factory::create(child_factory, &mut *conn).await?;
                        }
                        Ok(())
                    })
                }
            }

            impl #parent_factory {
                pub fn #method_name<Alias>(mut self, factory: #child_factory, count: usize) -> Self
                where
                    #child_struct: fabrique::BelongsTo<#parent_struct, Alias>,
                    #child_factory: fabrique::SetForeignKey<#parent_struct, Alias>,
                    Alias: Send + Sync + Clone + 'static,
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
                #[derive(Clone)]
                struct __DeferredOrderForUser {
                    factory: OrderFactory,
                    count: usize,
                }

                impl fabrique::DeferredFactory<
                    <User as fabrique::Model>::PrimaryKey,
                    <User as fabrique::database::DatabaseAware>::Database,
                > for __DeferredOrderForUser {
                    fn create<'a>(
                        &'a self,
                        parent_pk: <User as fabrique::Model>::PrimaryKey,
                        conn: &'a mut <<User as fabrique::database::DatabaseAware>::Database as ::sqlx::Database>::Connection,
                    ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                        Box::pin(async move {
                            for _ in 0..self.count {
                                let child_factory = <OrderFactory as fabrique::SetForeignKey<User>>::set_foreign_key(
                                    self.factory.clone(),
                                    parent_pk.clone(),
                                );
                                fabrique::Factory::create(child_factory, &mut *conn).await?;
                            }
                            Ok(())
                        })
                    }
                }

                impl UserFactory {
                    pub fn has_orders(mut self, factory: OrderFactory, count: usize) -> Self {
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
                #[derive(Clone)]
                struct __DeferredMessageForUser<Alias> {
                    factory: MessageFactory,
                    count: usize,
                    _alias: ::std::marker::PhantomData<Alias>,
                }

                impl<Alias> fabrique::DeferredFactory<
                    <User as fabrique::Model>::PrimaryKey,
                    <User as fabrique::database::DatabaseAware>::Database,
                > for __DeferredMessageForUser<Alias>
                where
                    Message: fabrique::BelongsTo<User, Alias>,
                    MessageFactory: fabrique::SetForeignKey<User, Alias>,
                    Alias: Send + Sync + Clone + 'static,
                {
                    fn create<'a>(
                        &'a self,
                        parent_pk: <User as fabrique::Model>::PrimaryKey,
                        conn: &'a mut <<User as fabrique::database::DatabaseAware>::Database as ::sqlx::Database>::Connection,
                    ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), fabrique::Error>> + Send + 'a>> {
                        Box::pin(async move {
                            for _ in 0..self.count {
                                let child_factory = <MessageFactory as fabrique::SetForeignKey<User, Alias>>::set_foreign_key(
                                    self.factory.clone(),
                                    parent_pk.clone(),
                                );
                                fabrique::Factory::create(child_factory, &mut *conn).await?;
                            }
                            Ok(())
                        })
                    }
                }

                impl UserFactory {
                    pub fn has_messages<Alias>(mut self, factory: MessageFactory, count: usize) -> Self
                    where
                        Message: fabrique::BelongsTo<User, Alias>,
                        MessageFactory: fabrique::SetForeignKey<User, Alias>,
                        Alias: Send + Sync + Clone + 'static,
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
