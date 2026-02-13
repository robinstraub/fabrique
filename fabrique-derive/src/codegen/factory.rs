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
        let relation_enums = self.generate_relation_enums();
        let relation_from_impls = self.generate_relation_from_impls();
        let set_foreign_key_impls = self.generate_set_foreign_key_impls();
        let clone_fields = self.generate_clone_fields();

        quote! {
            impl #base_struct_ident {
                pub fn factory<DB: ::fabrique::Dialect>() -> #factory_ident<DB> {
                    #factory_ident::new()
                }
            }

            #(#relation_enums)*

            #(#relation_from_impls)*

            pub struct #factory_ident<DB: ::fabrique::Dialect> {
                #(#factory_fields,)*
                #(#factory_relation_fields,)*
                children: Vec<std::sync::Arc<dyn fabrique::DeferredFactory<
                    <#base_struct_ident as fabrique::Model>::PrimaryKey,
                    DB,
                >>>,
            }

            impl<DB: ::fabrique::Dialect> Clone for #factory_ident<DB> {
                fn clone(&self) -> Self {
                    Self {
                        #(#clone_fields,)*
                        children: self.children.clone(),
                    }
                }
            }

            #factory_trait_impl

            #(#set_foreign_key_impls)*

            impl<DB: ::fabrique::Dialect> #factory_ident<DB> {
                #factory_method_new

                #(#factory_method_fields)*

                #(#factory_methods_for_relation)*
            }
        }
    }

    /// Returns belongs_to fields for factory generation.
    ///
    /// Uses alias_snake as the relation name when present, otherwise uses the
    /// parent type name. Ambiguous relations (multiple non-aliased belongs_to
    /// to the same parent) are rejected at parse time, so we don't need to
    /// filter here.
    fn belongs_to_fields(&self) -> impl Iterator<Item = FactoryRelation<'a>> {
        self.analysis.belongs_to().map(|(field, relation)| {
            let name = relation.alias_snake.as_ref().unwrap_or(&relation.name);
            let factory_field = Ident::new(&format!("{}_relation", name), field.span);

            FactoryRelation {
                base_ident: &field.ident,
                factory_field,
                name,
                referenced_type: &relation.referenced_type,
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
    fn generate_factory_relation_fields(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let struct_name = self.analysis.ident.to_string();
        self.belongs_to_fields().map(move |relation| {
            let factory_field = relation.factory_field;
            let enum_ident = Self::relation_enum_ident(&struct_name, relation.name);

            quote! {
                #factory_field: std::option::Option<#enum_ident<DB>>
            }
        })
    }

    /// Generates relation enum types.
    ///
    /// Each enum stores either a primary key (extracted from a model instance)
    /// or a factory to create a new instance. This design avoids requiring
    /// Clone on model types.
    fn generate_relation_enums(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let struct_name = self.analysis.ident.to_string();
        self.belongs_to_fields().map(move |relation| {
            let enum_ident = Self::relation_enum_ident(&struct_name, relation.name);
            let referenced_type = relation.referenced_type;
            let factory_type = Ident::new(
                &format!("{}Factory", referenced_type),
                referenced_type.span(),
            );

            quote! {
                pub enum #enum_ident<DB: ::fabrique::Dialect> {
                    PrimaryKey(<#referenced_type as fabrique::Model>::PrimaryKey),
                    Factory(#factory_type<DB>),
                }

                impl<DB: ::fabrique::Dialect> Clone for #enum_ident<DB> {
                    fn clone(&self) -> Self {
                        match self {
                            Self::PrimaryKey(pk) => Self::PrimaryKey(pk.clone()),
                            Self::Factory(f) => Self::Factory(f.clone()),
                        }
                    }
                }
            }
        })
    }

    /// Generates From implementations for relation enums.
    fn generate_relation_from_impls(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let struct_name = self.analysis.ident.to_string();
        self.belongs_to_fields().flat_map(move |relation| {
            let enum_ident = Self::relation_enum_ident(&struct_name, relation.name);
            let referenced_type = relation.referenced_type;
            let factory_type = Ident::new(
                &format!("{}Factory", referenced_type),
                referenced_type.span(),
            );

            vec![
                quote! {
                    impl<DB: ::fabrique::Dialect> From<#referenced_type> for #enum_ident<DB> {
                        fn from(model: #referenced_type) -> Self {
                            #enum_ident::PrimaryKey(fabrique::Model::primary_key(&model))
                        }
                    }
                },
                quote! {
                    impl<DB: ::fabrique::Dialect> From<&#referenced_type> for #enum_ident<DB> {
                        fn from(model: &#referenced_type) -> Self {
                            #enum_ident::PrimaryKey(fabrique::Model::primary_key(model))
                        }
                    }
                },
                quote! {
                    impl<DB: ::fabrique::Dialect> From<#factory_type<DB>> for #enum_ident<DB> {
                        fn from(factory: #factory_type<DB>) -> Self {
                            #enum_ident::Factory(factory)
                        }
                    }
                },
            ]
        })
    }

    /// Helper to generate relation enum identifier.
    ///
    /// Generates `{StructName}{RelationName}Relation` to avoid name collisions
    /// when multiple models in the same module have belongs_to to the same
    /// parent.
    fn relation_enum_ident(struct_name: &str, relation_name: &str) -> Ident {
        let capitalized_relation = relation_name
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect::<String>();
        let enum_name = format!("{}{}Relation", struct_name, capitalized_relation);
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
        let struct_name = struct_ident.to_string();
        let belongs_to_create = self.belongs_to_fields().map(|relation| {
            let field = &relation.base_ident;
            let factory_field = relation.factory_field;
            let enum_ident = Self::relation_enum_ident(&struct_name, relation.name);
            let factory_type = Ident::new(
                &format!("{}Factory", relation.referenced_type),
                relation.referenced_type.span(),
            );

            quote! {
                {
                    let key = match self.#factory_field.take() {
                        Some(#enum_ident::PrimaryKey(pk)) => pk,
                        Some(#enum_ident::Factory(factory)) => {
                            let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                            fabrique::Model::primary_key(&instance)
                        }
                        None => match self.#field.take() {
                            Some(pk) => pk,
                            None => {
                                let instance = fabrique::Factory::create(#factory_type::new(), &mut *conn).await?;
                                fabrique::Model::primary_key(&instance)
                            }
                        },
                    };
                    self.#field = Some(key);
                }
            }
        });

        // Step 3: Create the main instance
        // Check if any field has a custom faker expression (requires Fake trait import)
        let has_custom_faker = self
            .analysis
            .column_fields
            .iter()
            .any(|f| f.faker.is_some());

        // Only import Fake trait when custom faker expressions are used
        // seeded_value() handles the import internally for auto-generated values
        let fake_import = if has_custom_faker {
            quote! { use ::fabrique::fake::Fake; }
        } else {
            quote! {}
        };

        let column_fields = self.analysis.column_fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            match &field.faker {
                // Custom faker expression provided - requires fake feature
                Some(faker_expr) => quote! {
                    #name: self.#name.unwrap_or_else(|| #faker_expr.fake())
                },
                // Default: use seeded_value (works with or without fake feature)
                None => quote! {
                    #name: self.#name.unwrap_or_else(::fabrique::seeded_value::<#ty>)
                },
            }
        });

        quote! {
            fn create<'a, A>(
                mut self,
                executor: A,
            ) -> impl ::std::future::Future<Output = Result<#struct_ident, fabrique::Error>> + Send + 'a
            where
                A: ::sqlx::Acquire<'a, Database = DB> + Send + 'a,
            {
                ::std::boxed::Box::pin(async move {
                    #fake_import

                    let mut conn = executor.acquire().await
                        .map_err(fabrique::Error::from)?;

                    #(#belongs_to_create)*

                    let instance = #struct_ident {
                        #(#column_fields,)*
                    };
                    let instance = <#struct_ident as fabrique::Persist<DB>>::create(instance, &mut *conn).await?;
                    let pk = <#struct_ident as fabrique::Model>::primary_key(&instance);

                    for child in &self.children {
                        child.create(pk.clone(), &mut *conn).await?;
                    }

                    Ok(instance)
                })
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

        let initialized_relation_fields = self.belongs_to_fields().map(|relation| {
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
                    children: Vec::new(),
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
    fn generate_factory_methods_for_relation(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let struct_name = self.analysis.ident.to_string();
        self.belongs_to_fields().map(move |relation| {
            let referenced_type = relation.referenced_type;
            let method_name =
                Ident::new(&format!("for_{}", &relation.name), referenced_type.span());
            let field_ident = &relation.factory_field;
            let enum_ident = Self::relation_enum_ident(&struct_name, relation.name);

            quote! {
                pub fn #method_name(mut self, input: impl Into<#enum_ident<DB>>) -> Self
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

        // Each belongs_to parent needs its factory to be available
        let parent_factory_bounds = self.belongs_to_fields().map(|relation| {
            let parent_type = relation.referenced_type;
            let parent_factory = Ident::new(&format!("{}Factory", parent_type), parent_type.span());
            quote! { #parent_factory<DB>: fabrique::Factory<DB, Model = #parent_type> }
        });

        quote! {
            impl<DB: ::fabrique::Dialect> fabrique::Factory<DB> for #factory_ident<DB>
            where
                #model_ident: ::fabrique::Persist<DB>,
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                #(#parent_factory_bounds,)*
            {
                type Model = #model_ident;

                #factory_method_create
            }
        }
    }

    /// Generates `impl SetForeignKey<Parent, Alias>` for each belongs_to
    /// relation.
    ///
    /// This allows parent factories to set the FK on child factories when
    /// creating HasMany relationships. Uses the alias when present for
    /// disambiguation.
    fn generate_set_foreign_key_impls(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let factory_ident = &self.ident;

        self.analysis.belongs_to().map(move |(field, relation)| {
            let parent_type = &relation.referenced_type;
            let fk_field = &field.ident;

            let trait_params = match &relation.alias {
                Some(alias) => quote! { #parent_type, #alias },
                None => quote! { #parent_type },
            };

            quote! {
                impl<DB: ::fabrique::Dialect> fabrique::SetForeignKey<#trait_params> for #factory_ident<DB> {
                    fn set_foreign_key(self, parent_key: <#parent_type as fabrique::Model>::PrimaryKey) -> Self {
                        self.#fk_field(parent_key)
                    }
                }
            }
        })
    }

    /// Generates clone expressions for each field (column + relation fields).
    fn generate_clone_fields(&self) -> Vec<TokenStream> {
        let mut fields: Vec<TokenStream> = self
            .analysis
            .column_fields
            .iter()
            .map(|field| {
                let name = &field.ident;
                quote! { #name: self.#name.clone() }
            })
            .collect();

        for relation in self.belongs_to_fields() {
            let name = relation.factory_field;
            fields.push(quote! { #name: self.#name.clone() });
        }

        fields
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
                #[fabrique(belongs_to = "Hammer")]
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
                    pub fn factory<DB: ::fabrique::Dialect>() -> AnvilFactory<DB> {
                        AnvilFactory::new()
                    }
                }

                pub enum AnvilHammerRelation<DB: ::fabrique::Dialect> {
                    PrimaryKey(<Hammer as fabrique::Model>::PrimaryKey),
                    Factory(HammerFactory<DB>),
                }

                impl<DB: ::fabrique::Dialect> Clone for AnvilHammerRelation<DB> {
                    fn clone(&self) -> Self {
                        match self {
                            Self::PrimaryKey(pk) => Self::PrimaryKey(pk.clone()),
                            Self::Factory(f) => Self::Factory(f.clone()),
                        }
                    }
                }

                impl<DB: ::fabrique::Dialect> From<Hammer> for AnvilHammerRelation<DB> {
                    fn from(model: Hammer) -> Self {
                        AnvilHammerRelation::PrimaryKey(fabrique::Model::primary_key(&model))
                    }
                }

                impl<DB: ::fabrique::Dialect> From<&Hammer> for AnvilHammerRelation<DB> {
                    fn from(model: &Hammer) -> Self {
                        AnvilHammerRelation::PrimaryKey(fabrique::Model::primary_key(model))
                    }
                }

                impl<DB: ::fabrique::Dialect> From<HammerFactory<DB>> for AnvilHammerRelation<DB> {
                    fn from(factory: HammerFactory<DB>) -> Self {
                        AnvilHammerRelation::Factory(factory)
                    }
                }

                pub struct AnvilFactory<DB: ::fabrique::Dialect> {
                    id: std::option::Option<u32>,
                    hammer_id: std::option::Option<u32>,
                    hardness: std::option::Option<u32>,
                    weight: std::option::Option<u32>,
                    hammer_relation: std::option::Option<AnvilHammerRelation<DB>>,
                    children: Vec<std::sync::Arc<dyn fabrique::DeferredFactory<
                        <Anvil as fabrique::Model>::PrimaryKey,
                        DB,
                    >>>,
                }

                impl<DB: ::fabrique::Dialect> Clone for AnvilFactory<DB> {
                    fn clone(&self) -> Self {
                        Self {
                            id: self.id.clone(),
                            hammer_id: self.hammer_id.clone(),
                            hardness: self.hardness.clone(),
                            weight: self.weight.clone(),
                            hammer_relation: self.hammer_relation.clone(),
                            children: self.children.clone(),
                        }
                    }
                }

                impl<DB: ::fabrique::Dialect> fabrique::Factory<DB> for AnvilFactory<DB>
                where
                    Anvil: ::fabrique::Persist<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Acquire<'c, Database = DB>,
                    HammerFactory<DB>: fabrique::Factory<DB, Model = Hammer>,
                {
                    type Model = Anvil;

                    fn create<'a, A>(
                        mut self,
                        executor: A,
                    ) -> impl ::std::future::Future<Output = Result<Anvil, fabrique::Error>> + Send + 'a
                    where
                        A: ::sqlx::Acquire<'a, Database = DB> + Send + 'a,
                    {
                        ::std::boxed::Box::pin(async move {
                            let mut conn = executor.acquire().await
                                .map_err(fabrique::Error::from)?;

                            {
                                let key = match self.hammer_relation.take() {
                                    Some(AnvilHammerRelation::PrimaryKey(pk)) => pk,
                                    Some(AnvilHammerRelation::Factory(factory)) => {
                                        let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                                        fabrique::Model::primary_key(&instance)
                                    }
                                    None => match self.hammer_id.take() {
                                        Some(pk) => pk,
                                        None => {
                                            let instance = fabrique::Factory::create(HammerFactory::new(), &mut *conn).await?;
                                            fabrique::Model::primary_key(&instance)
                                        }
                                    },
                                };
                                self.hammer_id = Some(key);
                            }

                            let instance = Anvil {
                                id: self.id.unwrap_or_else(::fabrique::seeded_value::<u32>),
                                hammer_id: self.hammer_id.unwrap_or_else(::fabrique::seeded_value::<u32>),
                                hardness: self.hardness.unwrap_or_else(::fabrique::seeded_value::<u32>),
                                weight: self.weight.unwrap_or_else(::fabrique::seeded_value::<u32>),
                            };

                            let instance = <Anvil as fabrique::Persist<DB>>::create(instance, &mut *conn).await?;
                            let pk = <Anvil as fabrique::Model>::primary_key(&instance);

                            for child in &self.children {
                                child.create(pk.clone(), &mut *conn).await?;
                            }

                            Ok(instance)
                        })
                    }
                }

                impl<DB: ::fabrique::Dialect> fabrique::SetForeignKey<Hammer> for AnvilFactory<DB> {
                    fn set_foreign_key(self, parent_key: <Hammer as fabrique::Model>::PrimaryKey) -> Self {
                        self.hammer_id(parent_key)
                    }
                }

                impl<DB: ::fabrique::Dialect> AnvilFactory<DB> {
                    pub fn new() -> Self {
                        Self {
                            id: None,
                            hammer_id: None,
                            hardness: None,
                            weight: None,
                            hammer_relation: None,
                            children: Vec::new(),
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

                    pub fn for_hammer(mut self, input: impl Into<AnvilHammerRelation<DB>>) -> Self {
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
                #[fabrique(belongs_to = "Explosive")]
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
                explosive_relation: std::option::Option<DynamiteExplosiveRelation<DB>>
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
                #[fabrique(belongs_to = "Hammer")]
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
                ) -> impl ::std::future::Future<Output = Result<Anvil, fabrique::Error>> + Send + 'a
                where
                    A: ::sqlx::Acquire<'a, Database = DB> + Send + 'a,
                {
                    ::std::boxed::Box::pin(async move {
                        let mut conn = executor.acquire().await
                            .map_err(fabrique::Error::from)?;

                        {
                            let key = match self.hammer_relation.take() {
                                Some(AnvilHammerRelation::PrimaryKey(pk)) => pk,
                                Some(AnvilHammerRelation::Factory(factory)) => {
                                    let instance = fabrique::Factory::create(factory, &mut *conn).await?;
                                    fabrique::Model::primary_key(&instance)
                                }
                                None => match self.hammer_id.take() {
                                    Some(pk) => pk,
                                    None => {
                                        let instance = fabrique::Factory::create(HammerFactory::new(), &mut *conn).await?;
                                        fabrique::Model::primary_key(&instance)
                                    }
                                },
                            };
                            self.hammer_id = Some(key);
                        }

                        let instance = Anvil {
                            id: self.id.unwrap_or_else(::fabrique::seeded_value::<u32>),
                            hammer_id: self.hammer_id.unwrap_or_else(::fabrique::seeded_value::<u32>),
                            hardness: self.hardness.unwrap_or_else(::fabrique::seeded_value::<u32>),
                            weight: self.weight.unwrap_or_else(::fabrique::seeded_value::<u32>),
                        };

                        let instance = <Anvil as fabrique::Persist<DB>>::create(instance, &mut *conn).await?;
                        let pk = <Anvil as fabrique::Model>::primary_key(&instance);

                        for child in &self.children {
                            child.create(pk.clone(), &mut *conn).await?;
                        }

                        Ok(instance)
                    })
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
                        children: Vec::new(),
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
                #[fabrique(belongs_to = "Explosive")]
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
                pub fn for_explosive(mut self, input: impl Into<DynamiteExplosiveRelation<DB>>) -> Self {
                    self.explosive_relation = Some(input.into());
                    self
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_set_foreign_key_impls() {
        // Arrange - Order belongs_to Customer
        let input = parse_quote! {
            struct Order {
                id: u32,
                #[fabrique(belongs_to = "Customer")]
                customer_id: u32
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        // Act
        let generated: Vec<TokenStream> = codegen.generate_set_foreign_key_impls().collect();

        // Assert
        assert_eq!(
            generated[0].to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> fabrique::SetForeignKey<Customer> for OrderFactory<DB> {
                    fn set_foreign_key(self, parent_key: <Customer as fabrique::Model>::PrimaryKey) -> Self {
                        self.customer_id(parent_key)
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_aliased_belongs_to_generates_set_foreign_key_with_alias() {
        let input = parse_quote! {
            struct Message {
                id: u32,
                #[fabrique(belongs_to = "User", alias = "Sender")]
                sender_id: u32,
                #[fabrique(belongs_to = "User", alias = "Recipient")]
                recipient_id: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        let generated: Vec<TokenStream> = codegen.generate_set_foreign_key_impls().collect();

        assert_eq!(generated.len(), 2);
        assert_eq!(
            generated[0].to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> fabrique::SetForeignKey<User, Sender> for MessageFactory<DB> {
                    fn set_foreign_key(self, parent_key: <User as fabrique::Model>::PrimaryKey) -> Self {
                        self.sender_id(parent_key)
                    }
                }
            }
            .to_string()
        );
        assert_eq!(
            generated[1].to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> fabrique::SetForeignKey<User, Recipient> for MessageFactory<DB> {
                    fn set_foreign_key(self, parent_key: <User as fabrique::Model>::PrimaryKey) -> Self {
                        self.recipient_id(parent_key)
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_aliased_belongs_to_generates_for_alias_methods() {
        let input = parse_quote! {
            struct Message {
                id: u32,
                #[fabrique(belongs_to = "User", alias = "Sender")]
                sender_id: u32,
                #[fabrique(belongs_to = "User", alias = "Recipient")]
                recipient_id: u32,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FactoryCodegen::new(&analysis);

        let generated: Vec<TokenStream> = codegen.generate_factory_methods_for_relation().collect();

        assert_eq!(generated.len(), 2);
        assert_eq!(
            generated[0].to_string(),
            quote! {
                pub fn for_sender(mut self, input: impl Into<MessageSenderRelation<DB>>) -> Self {
                    self.sender_relation = Some(input.into());
                    self
                }
            }
            .to_string()
        );
        assert_eq!(
            generated[1].to_string(),
            quote! {
                pub fn for_recipient(mut self, input: impl Into<MessageRecipientRelation<DB>>) -> Self {
                    self.recipient_relation = Some(input.into());
                    self
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_factory_method_create_with_custom_faker() {
        // Arrange a struct with a custom faker expression
        let input = parse_quote! {
            struct User {
                id: u32,
                #[fabrique(faker = "Name()")]
                name: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let factory = FactoryCodegen::new(&analysis);

        // Act
        let generated = factory.generate_factory_method_create().to_string();

        // Assert - should contain Fake import and custom faker expression
        assert!(
            generated.contains("use :: fabrique :: fake :: Fake"),
            "Should import Fake trait when custom faker is used. Generated: {}",
            generated
        );
        assert!(
            generated.contains("Name () . fake ()"),
            "Should use custom faker expression. Generated: {}",
            generated
        );
        // id should still use seeded_value
        assert!(
            generated.contains("seeded_value :: < u32 >"),
            "Fields without faker should use seeded_value. Generated: {}",
            generated
        );
    }
}
