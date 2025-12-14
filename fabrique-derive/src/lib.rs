//! Procedural macros for generating factory and model code.
//!
//! This crate provides two derive macros:
//! - `#[derive(Factory)]` - Generates factory structs with optional fields for
//!   flexible object creation
//! - `#[derive(Model)]` - Generates model implementations with database operations

use crate::{
    analysis::Analysis, delete::DeleteCodegen, factory::FactoryCodegen,
    soft_delete::SoftDeleteCodegen,
};
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod analysis;
mod delete;
mod error;
mod factory;
mod persistable;
mod query_builder;
mod soft_delete;

/// Derives a `Model` implementation for the annotated struct.
///
/// This generates implementations for:
/// - `Database` trait (connection and error types)
/// - `Model` trait (primary key and table name)
/// - `Query` trait (query building and retrieval)
/// - `Persist` trait (creation and deletion)
/// - `HardDelete` trait (permanent deletion)
/// - `SoftDelete` trait (conditional, if soft delete field is present)
#[proc_macro_derive(Model, attributes(fabrique))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    use crate::{
        analysis::Analysis, persistable::PersistableCodegen, query_builder::QueryBuilderCodegen,
    };

    let input = parse_macro_input!(input as DeriveInput);

    let analysis = match Analysis::from(&input) {
        Ok(analysis) => analysis,
        Err(e) => {
            let syn_error: syn::Error = e.into();
            return syn_error.into_compile_error().into();
        }
    };

    let delete_codegen = DeleteCodegen::new(&analysis);
    let soft_delete_codegen = SoftDeleteCodegen::new(&analysis);
    let query_builder_codegen = QueryBuilderCodegen::new(&analysis);
    let persistable_codegen = PersistableCodegen::new(&analysis, &query_builder_codegen);

    let delete = delete_codegen.generate();
    let soft_delete = soft_delete_codegen.generate();
    let persistable = persistable_codegen.generate();
    let query_builder = query_builder_codegen.generate();

    quote::quote! {
        #delete
        #soft_delete
        #persistable
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
