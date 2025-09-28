//! Procedural macro for generating factory structs.
//!
//! This crate provides the `#[derive(Factory)]` macro to automatically generate
//! factory structs for your data types. Each field in the original struct becomes
//! an `Option<T>` field in the factory, allowing selective value setting.

use crate::codegen::FactoryCodegen;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod analysis;
mod codegen;
mod error;

/// Derives a factory struct for the annotated data type.
///
/// This macro generates a factory struct with the same fields as the original,
/// but wrapped in `Option<T>` to allow selective field setting.
#[proc_macro_derive(Factory)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    FactoryCodegen::from(input).generate_factory().into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_make_derive() {}
}
