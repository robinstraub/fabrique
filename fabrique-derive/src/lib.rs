//! Procedural macro for generating factory structs.
//!
//! This crate provides the `#[derive(Factory)]` macro to automatically generate
//! factory structs for your data types. Each field in the original struct becomes
//! an `Option<T>` field in the factory, allowing selective value setting.

use crate::factory::FactoryCodegen;
use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input, spanned::Spanned};

mod analysis;
mod error;
mod factory;
mod persistable;

#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    crate::persistable::PersistableCodegen::from(&input)
        .and_then(|codegen| codegen.generate())
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error())
        .into()
}

#[proc_macro_derive(Factory, attributes(factory, fabrique))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    FactoryCodegen::from(input)
        .map(|codegen| codegen.generate_factory())
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error())
        .into()
}
