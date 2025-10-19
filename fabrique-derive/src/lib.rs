//! Procedural macro for generating factory structs.
//!
//! This crate provides the `#[derive(Factory)]` macro to automatically generate
//! factory structs for your data types. Each field in the original struct becomes
//! an `Option<T>` field in the factory, allowing selective value setting.

use crate::{factory::FactoryCodegen, persistable::PersistableCodegen};
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

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
#[proc_macro_derive(Factory, attributes(factory))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    FactoryCodegen::from(input).generate_factory().into()
}

#[cfg(feature = "sqlx")]
#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    PersistableCodegen::from(&input)
        .and_then(|codegen| codegen.generate())
        .unwrap_or_else(|err| syn::Error::from(err).into_compile_error())
        .into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_make_derive() {}
}
