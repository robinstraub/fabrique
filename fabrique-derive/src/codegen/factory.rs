use crate::analysis::Analysis;
use crate::analysis::ast::HasManyField;
use heck::ToSnakeCase;
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
        let has_many_fields = self.generate_has_many_fields();
        let has_many_methods = self.generate_has_many_methods();
        let factory_trait_impl = self.generate_impl_factory_trait();
        let relation_enums = self.generate_relation_enums();
        let relation_from_impls = self.generate_relation_from_impls();

        quote! {
            impl #base_struct_ident {
                pub fn factory() -> #factory_ident {
                    #factory_ident::new()
                }
            }

            #(#relation_enums)*

            #(#relation_from_impls)*

            #[derive(Clone)]
            pub struct #factory_ident {
                #(#factory_fields,)*
                #(#factory_relation_fields,)*
                #(#has_many_fields,)*
            }

            #factory_trait_impl

            impl #factory_ident {
                #factory_method_new

                #(#factory_method_fields)*

                #(#factory_methods_for_relation)*

                #(#has_many_methods)*
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

    /// Returns HasMany fields from the analysis.
    fn has_many_fields(&self) -> impl Iterator<Item = &'a HasManyField> {
        self.analysis.has_many_fields.iter()
    }

    /// Generates pending HasMany fields for the factory struct.
    fn generate_has_many_fields(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.has_many_fields().map(|field| {
            let field_name = Ident::new(&format!("pending_{}", field.ident), field.ident.span());
            let target_factory = Ident::new(
                &format!("{}Factory", field.target_type),
                field.target_type.span(),
            );

            quote! {
                #field_name: Vec<(#target_factory, usize)>
            }
        })
    }

    /// Generates `has_<relation>` setter methods for HasMany fields.
    fn generate_has_many_methods(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.has_many_fields().map(|field| {
            let method_name = Ident::new(&format!("has_{}", field.ident), field.ident.span());
            let pending_field = Ident::new(&format!("pending_{}", field.ident), field.ident.span());
            let target_factory = Ident::new(
                &format!("{}Factory", field.target_type),
                field.target_type.span(),
            );

            quote! {
                pub fn #method_name(mut self, factory: #target_factory, count: usize) -> Self {
                    self.#pending_field.push((factory, count));
                    self
                }
            }
        })
    }

    /// Generates field definitions for the factory struct.
    ///
    /// Transforms each field into an Option so users can either set specific
    /// values or let the factory generate defaults when building the final
    /// struct.
    fn generate_factory_fields(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.analysis.column_fields.iter().map(|field| {
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
            let enum_ident = Self::relation_enum_ident(relation.name);

            quote! {
                #factory_field: std::option::Option<#enum_ident>
            }
        })
    }

    /// Generates relation enum types.
    ///
    /// Each enum stores either a primary key (extracted from a model instance)
    /// or a factory to create a new instance. This design avoids requiring
    /// Clone on model types.
    fn generate_relation_enums(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.relations().map(|relation| {
            let enum_ident = Self::relation_enum_ident(relation.name);
            let referenced_type = relation.referenced_type;
            let factory_type = Ident::new(
                &format!("{}Factory", referenced_type),
                referenced_type.span(),
            );

            quote! {
                #[derive(Clone)]
                pub enum #enum_ident {
                    PrimaryKey(<#referenced_type as fabrique::Model>::PrimaryKey),
                    Factory(#factory_type),
                }
            }
        })
    }

    /// Generates From implementations for relation enums.
    fn generate_relation_from_impls(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.relations().flat_map(|relation| {
            let enum_ident = Self::relation_enum_ident(relation.name);
            let referenced_type = relation.referenced_type;
            let factory_type = Ident::new(
                &format!("{}Factory", referenced_type),
                referenced_type.span(),
            );

            vec![
                quote! {
                    impl From<#referenced_type> for #enum_ident {
                        fn from(model: #referenced_type) -> Self {
                            #enum_ident::PrimaryKey(fabrique::Model::primary_key(&model))
                        }
                    }
                },
                quote! {
                    impl From<#factory_type> for #enum_ident {
                        fn from(factory: #factory_type) -> Self {
                            #enum_ident::Factory(factory)
                        }
                    }
                },
            ]
        })
    }

    /// Helper to generate relation enum identifier.
    fn relation_enum_ident(relation_name: &str) -> Ident {
        let enum_name = format!(
            "{}Relation",
            relation_name
                .chars()
                .enumerate()
                .map(|(i, c)| if i == 0 {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                })
                .collect::<String>()
        );
        Ident::new(&enum_name, proc_macro2::Span::call_site())
    }

    /// Generates the `create()` method for the factory struct.
    ///
    /// The method follows this sequence:
    /// 1. Acquire connection
    /// 2. Create belongs_to relations (if any)
    /// 3. Create the main instance
    /// 4. Create has_many children (if any)
    /// 5. Return the instance
    fn generate_factory_method_create(&self) -> TokenStream {
        let struct_ident = &self.analysis.ident;

        // Step 1: Acquire connection is inline in the quote! below

        // Step 2: Create belongs_to relations
        let belongs_to_create = self.relations().map(|relation| {
            let field = &relation.base_ident;
            let factory_field = relation.factory_field;
            let enum_ident = Self::relation_enum_ident(relation.name);

            quote! {
                if let Some(relation) = self.#factory_field.take() {
                    let key = match relation {
                        #enum_ident::PrimaryKey(pk) => pk,
                        #enum_ident::Factory(factory) => {
                            let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                            fabrique::Model::primary_key(&instance)
                        }
                    };
                    self.#field = Some(key);
                }
            }
        });

        // Step 3: Create the main instance
        let column_fields = self.analysis.column_fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;
            quote! {
                #name: self.#name.unwrap_or(<#ty as Default>::default())
            }
        });

        let has_many_init = self.has_many_fields().map(|field| {
            let name = &field.ident;
            quote! {
                #name: ::fabrique::HasMany::default()
            }
        });

        let struct_fields = column_fields.chain(has_many_init);

        // Step 4: Create has_many children
        let has_many_create = self.has_many_fields().map(|field| {
            let pending_field = Ident::new(&format!("pending_{}", field.ident), field.ident.span());

            // Extract the FK field method name from the path (e.g., Order::CUSTOMER_ID ->
            // customer_id)
            let fk_method = field
                .foreign_key
                .segments
                .last()
                .map(|seg| {
                    let name = seg.ident.to_string().to_snake_case();
                    Ident::new(&name, seg.ident.span())
                })
                .expect("foreign_key path must have at least one segment");

            quote! {
                for (factory, count) in self.#pending_field {
                    for _ in 0..count {
                        let child_factory = factory.clone().#fk_method(pk.clone());
                        fabrique::Factory::create(child_factory, &mut *conn).await?;
                    }
                }
            }
        });

        // Step 5: Return the instance is inline in the quote! below

        quote! {
            fn create<'a, A>(
                mut self,
                executor: A,
            ) -> impl ::std::future::Future<Output = Result<#struct_ident, <#struct_ident as fabrique::DatabaseAware>::Error>> + Send + 'a
            where
                A: ::sqlx::Acquire<'a, Database = <#struct_ident as fabrique::DatabaseAware>::Database> + Send + 'a,
            {
                async move {
                    // Step 1: Acquire connection
                    let mut conn = executor.acquire().await
                        .map_err(|e| <#struct_ident as fabrique::DatabaseAware>::Error::from(e))?;

                    // Step 2: Create belongs_to relations
                    #(#belongs_to_create)*

                    // Step 3: Create the main instance
                    let instance = #struct_ident {
                        #(#struct_fields,)*
                    };
                    let instance = <#struct_ident as fabrique::Persist>::create(instance, &mut *conn).await?;
                    let pk = <#struct_ident as fabrique::Model>::primary_key(&instance);

                    // Step 4: Create has_many children
                    #(#has_many_create)*

                    // Step 5: Return the instance
                    Ok(instance)
                }
            }
        }
    }

    /// Generates the `new()` method for the factory struct.
    fn generate_factory_method_new(&self) -> TokenStream {
        let initialized_fields = self.analysis.column_fields.iter().map(|field| {
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

        let initialized_has_many_fields = self.has_many_fields().map(|field| {
            let name = Ident::new(&format!("pending_{}", field.ident), field.ident.span());
            quote! {
                #name: Vec::new()
            }
        });

        quote! {
            pub fn new() -> Self {
                Self {
                    #(#initialized_fields,)*
                    #(#initialized_relation_fields,)*
                    #(#initialized_has_many_fields,)*
                }
            }
        }
    }

    /// Generates setter methods for each field in the factory struct.
    ///
    /// Each setter method takes a value and stores it in the factory's optional
    /// field, enabling a fluent builder pattern for constructing objects.
    fn generate_factory_method_fields(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.analysis.column_fields.iter().map(|field| {
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
            let enum_ident = Self::relation_enum_ident(relation.name);

            quote! {
                pub fn #method_name(mut self, input: impl Into<#enum_ident>) -> Self
                {
                    self.#field_ident = Some(input.into());
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

                #[derive(Clone)]
                pub enum HammerRelation {
                    PrimaryKey(<Hammer as fabrique::Model>::PrimaryKey),
                    Factory(HammerFactory),
                }

                impl From<Hammer> for HammerRelation {
                    fn from(model: Hammer) -> Self {
                        HammerRelation::PrimaryKey(fabrique::Model::primary_key(&model))
                    }
                }

                impl From<HammerFactory> for HammerRelation {
                    fn from(factory: HammerFactory) -> Self {
                        HammerRelation::Factory(factory)
                    }
                }

                #[derive(Clone)]
                pub struct AnvilFactory {
                    id: std::option::Option<u32>,
                    hammer_id: std::option::Option<u32>,
                    hardness: std::option::Option<u32>,
                    weight: std::option::Option<u32>,
                    hammer_relation: std::option::Option<HammerRelation>,
                }

                impl fabrique::Factory for AnvilFactory {
                    type Model = Anvil;

                    fn create<'a, A>(
                        mut self,
                        executor: A,
                    ) -> impl ::std::future::Future<Output = Result<Anvil, <Anvil as fabrique::DatabaseAware>::Error>> + Send + 'a
                    where
                        A: ::sqlx::Acquire<'a, Database = <Anvil as fabrique::DatabaseAware>::Database> + Send + 'a,
                    {
                        async move {
                            let mut conn = executor.acquire().await
                                .map_err(|e| <Anvil as fabrique::DatabaseAware>::Error::from(e))?;

                            if let Some(relation) = self.hammer_relation.take() {
                                let key = match relation {
                                    HammerRelation::PrimaryKey(pk) => pk,
                                    HammerRelation::Factory(factory) => {
                                        let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                                        fabrique::Model::primary_key(&instance)
                                    }
                                };
                                self.hammer_id = Some(key);
                            }

                            let instance = Anvil {
                                id: self.id.unwrap_or(<u32 as Default>::default()),
                                hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                                hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                                weight: self.weight.unwrap_or(<u32 as Default>::default()),
                            };

                            let instance = <Anvil as fabrique::Persist>::create(instance, &mut *conn).await?;
                            let pk = <Anvil as fabrique::Model>::primary_key(&instance);

                            Ok(instance)
                        }
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

                    pub fn for_hammer(mut self, input: impl Into<HammerRelation>) -> Self {
                        self.hammer_relation = Some(input.into());
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
                explosive_relation: std::option::Option<ExplosiveRelation>
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
                fn create<'a, A>(
                    mut self,
                    executor: A,
                ) -> impl ::std::future::Future<Output = Result<Anvil, <Anvil as fabrique::DatabaseAware>::Error>> + Send + 'a
                where
                    A: ::sqlx::Acquire<'a, Database = <Anvil as fabrique::DatabaseAware>::Database> + Send + 'a,
                {
                    async move {
                        let mut conn = executor.acquire().await
                            .map_err(|e| <Anvil as fabrique::DatabaseAware>::Error::from(e))?;

                        if let Some(relation) = self.hammer_relation.take() {
                            let key = match relation {
                                HammerRelation::PrimaryKey(pk) => pk,
                                HammerRelation::Factory(factory) => {
                                    let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                                    fabrique::Model::primary_key(&instance)
                                }
                            };
                            self.hammer_id = Some(key);
                        }

                        let instance = Anvil {
                            id: self.id.unwrap_or(<u32 as Default>::default()),
                            hammer_id: self.hammer_id.unwrap_or(<u32 as Default>::default()),
                            hardness: self.hardness.unwrap_or(<u32 as Default>::default()),
                            weight: self.weight.unwrap_or(<u32 as Default>::default()),
                        };

                        let instance = <Anvil as fabrique::Persist>::create(instance, &mut *conn).await?;
                        let pk = <Anvil as fabrique::Model>::primary_key(&instance);

                        Ok(instance)
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
                pub fn for_explosive(mut self, input: impl Into<ExplosiveRelation>) -> Self {
                    self.explosive_relation = Some(input.into());
                    self
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_with_has_many() {
        // Arrange
        let input = parse_quote! {
            struct Customer {
                id: u32,

                #[fabrique(foreign_key = Order::CUSTOMER_ID)]
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act
        let generated = codegen.generate_factory();

        // Assert - verify the factory includes HasMany handling
        assert_eq!(
            generated.to_string(),
            quote! {
                impl Customer {
                    pub fn factory() -> CustomerFactory {
                        CustomerFactory::new()
                    }
                }

                #[derive(Clone)]
                pub struct CustomerFactory {
                    id: std::option::Option<u32>,
                    pending_orders: Vec<(OrderFactory, usize)>,
                }

                impl fabrique::Factory for CustomerFactory {
                    type Model = Customer;

                    fn create<'a, A>(
                        mut self,
                        executor: A,
                    ) -> impl ::std::future::Future<Output = Result<Customer, <Customer as fabrique::DatabaseAware>::Error>> + Send + 'a
                    where
                        A: ::sqlx::Acquire<'a, Database = <Customer as fabrique::DatabaseAware>::Database> + Send + 'a,
                    {
                        async move {
                            let mut conn = executor.acquire().await
                                .map_err(|e| <Customer as fabrique::DatabaseAware>::Error::from(e))?;

                            let instance = Customer {
                                id: self.id.unwrap_or(<u32 as Default>::default()),
                                orders: ::fabrique::HasMany::default(),
                            };

                            let instance = <Customer as fabrique::Persist>::create(instance, &mut *conn).await?;
                            let pk = <Customer as fabrique::Model>::primary_key(&instance);

                            for (factory, count) in self.pending_orders {
                                for _ in 0..count {
                                    let child_factory = factory.clone().customer_id(pk.clone());
                                    fabrique::Factory::create(child_factory, &mut *conn).await?;
                                }
                            }

                            Ok(instance)
                        }
                    }
                }

                impl CustomerFactory {
                    pub fn new() -> Self {
                        Self {
                            id: None,
                            pending_orders: Vec::new(),
                        }
                    }

                    pub fn id(mut self, id: u32) -> Self {
                        self.id = Some(id);
                        self
                    }

                    pub fn has_orders(mut self, factory: OrderFactory, count: usize) -> Self {
                        self.pending_orders.push((factory, count));
                        self
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_has_many_fields() {
        // Arrange
        let input = parse_quote! {
            struct Customer {
                id: u32,

                #[fabrique(foreign_key = Order::CUSTOMER_ID)]
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act
        let generated: Vec<TokenStream> = codegen.generate_has_many_fields().collect();

        // Assert
        assert_eq!(
            generated[0].to_string(),
            quote! { pending_orders: Vec<(OrderFactory, usize)> }.to_string()
        );
    }

    #[test]
    fn test_generate_has_many_methods() {
        // Arrange
        let input = parse_quote! {
            struct Customer {
                id: u32,

                #[fabrique(foreign_key = Order::CUSTOMER_ID)]
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act
        let generated: Vec<TokenStream> = codegen.generate_has_many_methods().collect();

        // Assert
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn has_orders(mut self, factory: OrderFactory, count: usize) -> Self {
                    self.pending_orders.push((factory, count));
                    self
                }
            }
            .to_string()
        );
    }
}
