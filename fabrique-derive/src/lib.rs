//! Procedural macros for generating factory and persistence code.
//!
//! This crate provides two derive macros:
//! - `#[derive(Factory)]` - Generates factory structs with optional fields for flexible object creation
//! - `#[derive(Persistable)]` - Generates persistence implementations for data storage

use crate::factory::FactoryCodegen;
use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input, spanned::Spanned};

mod analysis;
mod error;
mod factory;
mod persistable;

/// Derives a `Persistable` implementation for the annotated struct.
#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    crate::persistable::PersistableCodegen::from(&input)
        .and_then(|codegen| codegen.generate())
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error())
        .into()
}

/// Derives a factory struct for the annotated type.
#[proc_macro_derive(Factory, attributes(factory, fabrique))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    FactoryCodegen::from(input)
        .map(|codegen| codegen.generate_factory())
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error())
        .into()
}
