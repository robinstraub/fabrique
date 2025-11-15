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

#[cfg(feature = "sqlx")]
mod persistable;

#[cfg(feature = "sqlx")]
mod query_builder;

/// Derives a `Persistable` implementation for the annotated struct.
#[cfg(feature = "sqlx")]
#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();

    let query_builder_codegen = match crate::query_builder::QueryBuilderCodegen::from(&input) {
        Ok(codegen) => codegen,
        Err(e) => return Error::new(span, e).into_compile_error().into(),
    };

    let persistable = crate::persistable::PersistableCodegen::from(&input)
        .and_then(|codegen| codegen.generate(&query_builder_codegen))
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error());

    let query_builder = query_builder_codegen
        .generate()
        .unwrap_or_else(|e| Error::new(span, e).into_compile_error());

    quote::quote! {
        #persistable
        #query_builder
    }
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
