//! Procedural macros for generating factory and model code.
//!
//! This crate provides two derive macros:
//! - `#[derive(Factory)]` - Generates factory structs with optional fields for
//!   flexible object creation
//! - `#[derive(Model)]` - Generates model implementations with database
//!   operations

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod analysis;
mod codegen;
mod error;

use crate::analysis::Analysis;
use crate::codegen::*;

/// Derives a `Model` implementation for the annotated struct.
///
/// This generates implementations for:
/// - `Database` trait (connection and error types)
/// - `Model` trait (primary key and table name)
/// - `Query` trait (query building and retrieval)
/// - `Persist` trait (creation operations)
/// - `Delete` trait (delete and destroy operations)
/// - `HardDelete` trait (permanent deletion)
/// - `SoftDelete` trait (conditional, if soft delete field is present)
#[proc_macro_derive(Model, attributes(fabrique))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let analysis = match Analysis::from(&input) {
        Ok(analysis) => analysis,
        Err(e) => {
            let syn_error: syn::Error = e.into();
            return syn_error.into_compile_error().into();
        }
    };

    // Query builder codegen
    let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
    let query_builder_ident = &query_builder_codegen.query_builder_ident;

    // Trait implementations
    let from_row = FromRowCodegen::new(&analysis).generate();
    let database = DatabaseCodegen::new(&analysis).generate();
    let model = ModelCodegen::new(&analysis).generate();
    let query = QueryCodegen::new(&analysis, query_builder_ident).generate();
    let persist = PersistCodegen::new(&analysis).generate();
    let delete = DeleteCodegen::new(&analysis).generate();
    let hard_delete = HardDeleteCodegen::new(&analysis).generate();
    let soft_delete = SoftDeleteCodegen::new(&analysis).generate();
    let columns = ColumnsCodegen::new(&analysis).generate();
    let query_builder = query_builder_codegen.generate();

    quote::quote! {
        #from_row
        #database
        #model
        #query
        #persist
        #delete
        #hard_delete
        #soft_delete
        #columns
        #query_builder
    }
    .into()
}

/// Derives a factory struct for the annotated type.
#[proc_macro_derive(Factory, attributes(factory, fabrique))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let analysis = match Analysis::from(&input) {
        Ok(analysis) => analysis,
        Err(e) => {
            let syn_error: syn::Error = e.into();
            return syn_error.into_compile_error().into();
        }
    };

    FactoryCodegen::new(&analysis).generate_factory().into()
}
