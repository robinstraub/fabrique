//! Procedural macro for generating factory structs.
//!
//! This crate provides the `#[derive(Factory)]` macro to automatically generate
//! factory structs for your data types. Each field in the original struct becomes
//! an `Option<T>` field in the factory, allowing selective value setting.

use crate::factory::FactoryCodegen;
use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Error};

mod analysis;
mod error;
mod factory;

#[cfg(feature = "sqlx")]
mod persistable;

/// Derives a factory struct for the annotated data type.
///
/// This macro generates a factory struct with the same fields as the original,
/// but wrapped in `Option<T>` to allow selective field setting. The generated
/// factory includes methods for setting individual fields and a `create()` method
/// for persisting objects to a database when the struct implements `Persistable`.
///
/// # Field Attributes
///
/// - `#[factory(relation = "FactoryType")]` - Creates a factory relation that generates
///   the related object before creating the main object.
///
/// # Generated Methods
///
/// For each field `field_name` of type `T`, the factory generates:
/// - `field_name(value: T) -> Self` - Sets the field value
/// - `for_relation_name(callback: impl FnOnce(RelationFactory) -> RelationFactory) -> Self` -
///   For relation fields, creates a callback-based builder pattern
///
/// Additionally generates:
/// - `new() -> Self` - Creates a new factory instance with all fields set to `None`
/// - `create(connection) -> Result<Struct, Error>` - Creates and persists the object if it implements `Persistable`
#[proc_macro_derive(Factory, attributes(factory, fabrique))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    match FactoryCodegen::from(input) {
        Ok(codegen) => codegen,
        Err(e) => return Error::new(span, e).into_compile_error().into(),
    }
    .generate_factory()
    .into()
}

#[cfg(feature = "sqlx")]
#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    crate::persistable::PersistableCodegen::from(&input)
        .and_then(|codegen| codegen.generate())
        .unwrap_or_else(|err| syn::Error::from(err).into_compile_error())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_derive() {}

    #[cfg(feature = "sqlx")]
    mod persistable_tests {
        use super::*;

        #[test]
        fn test_derive_persistable_generates_valid_code() {
            // Arrange the derive input
            let input: DeriveInput = syn::parse_quote! {
                struct Hammer {
                    id: i64,
                    weight: f64,
                }
            };

            // Act - generate the persistable implementation via codegen
            let result = crate::persistable::PersistableCodegen::from(&input)
                .and_then(|codegen| codegen.generate());

            // Assert that it contains the expected implementation
            assert!(result.is_ok());
            let output_str = result.unwrap().to_string();
            assert!(output_str.contains("impl fabrique :: Persistable for Hammer"));
            assert!(output_str.contains("async fn create"));
            assert!(output_str.contains("async fn all"));
        }

        #[test]
        fn test_derive_persistable_handles_invalid_struct() {
            // Arrange the derive input for an enum
            let input: DeriveInput = syn::parse_quote! {
                enum Hammer {}
            };

            // Act - generate the persistable implementation via codegen
            let result = crate::persistable::PersistableCodegen::from(&input);

            // Assert that it returns an error
            assert!(result.is_err());
        }
    }
}
