//! Procedural macros for generating factory and persistence code.
//!
//! This crate provides two derive macros:
//! - `#[derive(Factory)]` - Generates factory structs with optional fields for flexible object creation
//! - `#[derive(Persistable)]` - Generates persistence implementations for data storage

use crate::{analysis::Analysis, factory::FactoryCodegen};
use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input, spanned::Spanned};

mod analysis;
mod error;
mod factory;
mod persistable;
mod query_builder;

/// Derives a `Persistable` implementation for the annotated struct.
#[proc_macro_derive(Persistable, attributes(fabrique))]
pub fn derive_persistable(input: TokenStream) -> TokenStream {
    use crate::{
        analysis::Analysis, persistable::PersistableCodegen, query_builder::QueryBuilderCodegen,
    };

    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();

    let analysis = match Analysis::from(&input) {
        Ok(analysis) => analysis,
        Err(e) => {
            return Error::new(span, e).into_compile_error().into();
        }
    };

    let query_builder_codegen = QueryBuilderCodegen::new(&analysis);

    let persistable = PersistableCodegen::new(&analysis, &query_builder_codegen).generate();
    let query_builder = query_builder_codegen.generate();

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

    let analysis = match Analysis::from(&input) {
        Ok(analysis) => analysis,
        Err(e) => {
            return Error::new(span, e).into_compile_error().into();
        }
    };

    FactoryCodegen::new(&analysis).generate_factory().into()
}
