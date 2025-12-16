use crate::analysis::Analysis;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub struct FactoryRelation<'a> {
    /// The attribute base ident.
    base_ident: &'a Ident,

    /// The relation factory field to append to the factory struct.
    factory_field: Ident,

    /// The relation name.
    name: &'a String,

    /// The related model type.
    referenced_type: &'a Ident,
}

/// Code generator for factory struct implementations.
pub struct FactoryCodegen<'a> {
    /// Analysis output containing fields and relations
    analysis: &'a Analysis<'a>,
    // /// Original derive input for span information
    // input: DeriveInput,
    ident: Ident,
}

impl<'a> FactoryCodegen<'a> {
    /// Creates a code generator from the given derive input.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        let ident = Ident::new(
            &format!("{}Factory", &analysis.ident),
            analysis.ident.span(),
        );
        Self { analysis, ident }
    }

    /// Generates the complete factory implementation as a token stream.
    pub fn generate_factory(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let factory_ident = &self.ident;
        let factory_fields = self.generate_factory_fields();
        let factory_method_new = self.generate_factory_method_new();
        let factory_method_fields = self.generate_factory_method_fields();
        let factory_methods_for_relation = self.generate_factory_methods_for_relation();
        let factory_relation_fields = self.generate_factory_relation_fields();
        let factory_trait_impl = self.generate_impl_factory_trait();
        let for_relation_factory_impl = self.generate_impl_for_relation_factory();

        quote! {
            impl #base_struct_ident {
                pub fn factory() -> #factory_ident {
                    #factory_ident::new()
                }
            }

            pub struct #factory_ident {
                #(#factory_fields,)*
                #(#factory_relation_fields,)*
            }

            #factory_trait_impl

            #for_relation_factory_impl

            impl #factory_ident {
                #factory_method_new

                #(#factory_method_fields)*

                #(#factory_methods_for_relation)*
            }
        }
    }

    fn relations(&self) -> impl Iterator<Item = FactoryRelation<'a>> {
        self.analysis.relations().map(|(field, relation)| {
            let factory_field = Ident::new(&format!("{}_relation", &relation.name), field.span);

            FactoryRelation {
                base_ident: &field.ident,
                factory_field,
                name: &relation.name,
                referenced_type: &relation.referenced_type,
            }
        })
    }

    /// Generates field definitions for the factory struct.
    ///
    /// Transforms each field into an Option so users can either set specific
    /// values or let the factory generate defaults when building the final
    /// struct.
    fn generate_factory_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;
            quote! {
                #name: std::option::Option<#ty>
            }
        })
    }

    /// Generates factory relation fields for linked factory dependencies.
    fn generate_factory_relation_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.relations().map(|relation| {
            let factory_field = relation.factory_field;
            let referenced_type = relation.referenced_type;

            quote! {
                #factory_field: std::option::Option<Box<dyn fabrique::ForRelation<#referenced_type>>>
            }
        })
    }

    /// Generates the `create()` method for the factory struct.
    ///
    /// This method handles both relation creation and object persistence:
    /// 1. Creates any related objects first (via factory relations)
    /// 2. Creates the main object with all field values
    /// 3. Persists the object using the Persistable trait
    fn generate_factory_method_create(&self) -> TokenStream {
        // Generate relation creation code - related objects are created first
        // to establish the dependency graph before creating the main object
        let relations_create = self.relations().map(|relation| {
            let field = &relation.base_ident;
            let factory_field = relation.factory_field;

            quote! {
                if let Some(relation) = self.#factory_field {
                    let key = relation.into_key(&executor).await?;
                    self.#field = Some(key);
                }
            }
        });

        // Generate struct field initialization - use provided values or defaults
        let struct_ident = &self.analysis.ident;
        let struct_fields = self.analysis.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote! {
                #name: self.#name.unwrap_or(<#ty as Default>::default())
            }
        });

        quote! {
            fn create<E>(
                mut self,
                executor: E,
            ) -> impl ::std::future::Future<Output = Result<#struct_ident, <#struct_ident as fabrique::DatabaseAware>::Error>> + Send
            where
                E: for<'e> ::sqlx::Executor<'e, Database = <#struct_ident as fabrique::DatabaseAware>::Database>,
            {
                async move {
                    #(#relations_create)*

                    let instance = #struct_ident {
                        #(#struct_fields,)*
                    };

                    <#struct_ident as fabrique::Persist>::create(instance, executor).await
                }
            }
        }
    }

    /// Generates the `new()` method for the factory struct.
    fn generate_factory_method_new(&self) -> TokenStream {
        let initialized_fields = self.analysis.fields.iter().map(|field| {
            let name = &field.ident;
            quote! {
                #name: None
            }
        });

        let initialized_relation_fields = self.relations().map(|relation| {
            let name = &relation.factory_field;
            quote! {
                #name: None
            }
        });

        quote! {
            pub fn new() -> Self {
                Self {
                    #(#initialized_fields,)*
                    #(#initialized_relation_fields,)*
                }
            }
        }
    }

    /// Generates setter methods for each field in the factory struct.
    ///
    /// Each setter method takes a value and stores it in the factory's optional
    /// field, enabling a fluent builder pattern for constructing objects.
    fn generate_factory_method_fields(&self) -> impl Iterator<Item = TokenStream> {
        self.analysis.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote! {
                pub fn #name(mut self, #name: #ty) -> Self {
                    self.#name = Some(#name);
                    self
                }
            }
        })
    }

    /// Generates the `for_[relation]` methods for the factory struct.
    ///
    /// These methods accept either a model instance or a factory instance
    /// and store them for deferred execution during `create()`.
    fn generate_factory_methods_for_relation(&self) -> impl Iterator<Item = TokenStream> {
        self.relations().map(|relation| {
            let referenced_type = relation.referenced_type;
            let method_name =
                Ident::new(&format!("for_{}", &relation.name), referenced_type.span());
            let field_ident = &relation.factory_field;

            quote! {
                pub fn #method_name<R>(mut self, input: R) -> Self
                where R: fabrique::ForRelation<#referenced_type> + 'static
                {
                    self.#field_ident = Some(Box::new(input));
                    self
                }
            }
        })
    }

    /// Generates the Factory trait implementation.
    fn generate_impl_factory_trait(&self) -> TokenStream {
        let factory_ident = &self.ident;
        let model_ident = &self.analysis.ident;
        let factory_method_create = self.generate_factory_method_create();

        quote! {
            impl fabrique::Factory for #factory_ident {
                type Model = #model_ident;

                #factory_method_create
            }
        }
    }

    /// Generates ForRelation implementation for the Factory type.
    fn generate_impl_for_relation_factory(&self) -> TokenStream {
        let factory_ident = &self.ident;
        let model_ident = &self.analysis.ident;

        quote! {
            impl fabrique::ForRelation<#model_ident> for #factory_ident
            where
                Self: 'static,
            {
                fn into_key<E>(
                    self: Box<Self>,
                    executor: E,
                ) -> fabrique::IntoKeyFuture<#model_ident>
                where
                    E: for<'e> ::sqlx::Executor<'e, Database = <#model_ident as fabrique::DatabaseAware>::Database>,
                {
                    Box::pin(async move {
                        let instance = <Self as fabrique::Factory>::create(*self, executor).await?;
                        Ok(<#model_ident as fabrique::Model>::primary_key(&instance))
                    })
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
    fn test_factory_codegen_from_fails_on_invalid_input() {
        // Arrange an enum (which is not supported)
        let input = parse_quote! { enum Anvil {} };
        let analysis = Analysis::from(&input);

        // Assert that it returns an error
        assert!(analysis.is_err());
    }

    #[test]
    fn test_generate_factory() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                #[fabrique(primary_key)]
                id: u32,
                #[fabrique(relation = "Hammer")]
                hammer_id: u32,
                hardness: u32,
                weight: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act the call to the factory ident method
        let generated = codegen.generate_factory();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                impl Anvil {
                    pub fn factory() -> AnvilFactory {
                        AnvilFactory::new()
                    }
                }
                pub struct AnvilFactory {
                    id: std::option::Option<u32>,
                    hammer_id: std::option::Option<u32>,
                    hardness: std::option::Option<u32>,
                    weight: std::option::Option<u32>,

                    hammer_relation: std::option::Option<Box<dyn fabrique::ForRelation<Hammer>>>,
                }

                impl fabrique::Factory for AnvilFactory {
                    type Model = Anvil;

                    fn create<E>(
                        mut self,
                        executor: E,
                    ) -> impl ::std::future::Future<Output = Result<Anvil, <Anvil as fabrique::DatabaseAware>::Error>> + Send
                    where
                        E: for<'e> ::sqlx::Executor<'e, Database = <Anvil as fabrique::DatabaseAware>::Database>,
                    {
                        async move {
                            if let Some(relation) = self.hammer_relation {
                                let key = relation.into_key(&executor).await?;
                                self.hammer_id = Some(key);
                            }

                            let instance = Anvil {
                                id: self.id.unwrap_or(<u32 as Default>::default()),
                                hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                                hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                                weight: self.weight.unwrap_or(<u32 as Default>::default()),
                            };
                            <Anvil as fabrique::Persist>::create(instance, executor).await
                        }
                    }
                }

                impl fabrique::ForRelation<Anvil> for AnvilFactory
                where
                    Self: 'static,
                {
                    fn into_key<E>(
                        self: Box<Self>,
                        executor: E,
                    ) -> fabrique::IntoKeyFuture<Anvil>
                    where
                        E: for<'e> ::sqlx::Executor<'e, Database = <Anvil as fabrique::DatabaseAware>::Database>,
                    {
                        Box::pin(async move {
                            let instance = <Self as fabrique::Factory>::create(*self, executor).await?;
                            Ok(<Anvil as fabrique::Model>::primary_key(&instance))
                        })
                    }
                }

                impl AnvilFactory {
                    pub fn new() -> Self {
                        Self {
                            id: None,
                            hammer_id: None,
                            hardness: None,
                            weight: None,
                            hammer_relation: None,
                        }
                    }

                    pub fn id(mut self, id: u32) -> Self {
                        self.id = Some(id);
                        self
                    }

                    pub fn hammer_id(mut self, hammer_id: u32) -> Self {
                        self.hammer_id = Some(hammer_id);
                        self
                    }

                    pub fn hardness(mut self, hardness: u32) -> Self {
                        self.hardness = Some(hardness);
                        self
                    }

                    pub fn weight(mut self, weight: u32) -> Self {
                        self.weight = Some(weight);
                        self
                    }

                    pub fn for_hammer<R>(mut self, input: R) -> Self
                    where R: fabrique::ForRelation<Hammer> + 'static
                    {
                        self.hammer_relation = Some(Box::new(input));
                        self
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_fields() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: u32,
                weight: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act the call to the codegen fields method
        let generated: Vec<TokenStream> = codegen.generate_factory_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! { id: std::option::Option<u32> }.to_string()
        );
        assert_eq!(
            generated[1].to_string(),
            quote! { weight: std::option::Option<u32> }.to_string()
        );
    }

    #[test]
    fn test_generate_factory_relation_fields() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Dynamite {
                id: u32,
                #[fabrique(relation = "Explosive")]
                explosive_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act the call to the codegen fields method
        let generated: Vec<TokenStream> = codegen.generate_factory_relation_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                explosive_relation: std::option::Option<Box<dyn fabrique::ForRelation<Explosive>>>
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_create() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: u32,
                #[fabrique(relation = "Hammer")]
                hammer_id: u32,
                hardness: u32,
                weight: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let factory = FactoryCodegen::new(&analysis);

        // Act the call to the factory ident method
        let generated = factory.generate_factory_method_create();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                fn create<E>(
                    mut self,
                    executor: E,
                ) -> impl ::std::future::Future<Output = Result<Anvil, <Anvil as fabrique::DatabaseAware>::Error>> + Send
                where
                    E: for<'e> ::sqlx::Executor<'e, Database = <Anvil as fabrique::DatabaseAware>::Database>,
                {
                    async move {
                        if let Some(relation) = self.hammer_relation {
                            let key = relation.into_key(&executor).await?;
                            self.hammer_id = Some(key);
                        }

                        let instance = Anvil {
                            id: self.id.unwrap_or(<u32 as Default>::default()),
                            hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                            hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                            weight: self.weight.unwrap_or(<u32 as Default>::default()),
                        };
                        <Anvil as fabrique::Persist>::create(instance, executor).await
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_new() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: u32,
                hardness: u32,
                weight: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let factory = FactoryCodegen::new(&analysis);

        // Act the call to the factory ident method
        let generated = factory.generate_factory_method_new();

        // Assert the result
        assert_eq!(
            generated.to_string(),
            quote! {
                pub fn new() -> Self {
                    Self {
                        id: None,
                        hardness: None,
                        weight: None,
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_fields() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: u32,
                hardness: u32,
                weight: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let factory = FactoryCodegen::new(&analysis);

        // Act the call to the generate_factory_method_fields method
        let generated: Vec<TokenStream> = factory.generate_factory_method_fields().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn id(mut self, id: u32) -> Self {
                    self.id = Some(id);
                    self
                }
            }
            .to_string()
        );
        assert_eq!(
            generated[1].to_string(),
            quote! {
                pub fn hardness(mut self, hardness: u32) -> Self {
                    self.hardness = Some(hardness);
                    self
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_methods_for_relation() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Dynamite {
                id: u32,
                #[fabrique(relation = "Explosive")]
                explosive_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let factory = FactoryCodegen::new(&analysis);

        // Act the call to the generate_factory_method_fields method
        let generated: Vec<TokenStream> = factory.generate_factory_methods_for_relation().collect();

        // Assert the result
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn for_explosive<R>(mut self, input: R) -> Self
                where R: fabrique::ForRelation<Explosive> + 'static
                {
                    self.explosive_relation = Some(Box::new(input));
                    self
                }
            }
            .to_string()
        );
    }
}
