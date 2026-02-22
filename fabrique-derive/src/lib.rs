#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

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
#[cfg(feature = "testing")]
mod test;

use crate::analysis::Analysis;
use crate::codegen::*;

/// Derives a `Model` implementation for the annotated struct.
///
/// This generates implementations for:
/// - `Model` trait (primary key and table name)
/// - `Query` trait (query building and retrieval)
/// - `Persist` trait (creation operations)
/// - `Delete` trait (delete and destroy operations)
/// - `HardDelete` trait (permanent deletion)
/// - `SoftDelete` trait (conditional, if soft delete field is present)
// Tested via UI tests (trybuild) - coverage can't be measured for proc macros
#[cfg_attr(coverage_nightly, coverage(off))]
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

    // Trait implementations
    let from_row = FromRowCodegen::new(&analysis).generate();
    let model = ModelCodegen::new(&analysis).generate();
    let columns = ColumnsCodegen::new(&analysis).generate();
    let belongs_to = BelongsToCodegen::new(&analysis).generate();
    let joinable = JoinableCodegen::new(&analysis).generate();
    let has_many = HasManyCodegen::new(&analysis).generate();
    let alias = AliasCodegen::new(&analysis).generate();
    let query = QueryCodegen::new(&analysis).generate();
    let persist = PersistCodegen::new(&analysis).generate();
    let delete = DeleteCodegen::new(&analysis).generate();
    let hard_delete = HardDeleteCodegen::new(&analysis).generate();
    let soft_delete = SoftDeleteCodegen::new(&analysis).generate();

    quote::quote! {
        #from_row
        #model
        #columns
        #belongs_to
        #joinable
        #has_many
        #alias
        #query
        #persist
        #delete
        #hard_delete
        #soft_delete
    }
    .into()
}

/// Derives a factory struct for the annotated type.
///
/// Requires the `testing` feature to be enabled.
// Tested via UI tests (trybuild) - coverage can't be measured for proc macros
#[cfg(feature = "testing")]
#[cfg_attr(coverage_nightly, coverage(off))]
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

    let factory = FactoryCodegen::new(&analysis).generate_factory();
    let has_many_factory = HasManyFactoryCodegen::new(&analysis).generate();

    quote::quote! {
        #factory
        #has_many_factory
    }
    .into()
}

/// Sets up a test database with migrations for the detected backend.
///
/// Transforms an async test function into a `#[tokio::test]` function
/// that creates a pool, runs migrations, and cleans up temporary
/// databases (Postgres, MySQL) after the test completes.
///
/// The backend is detected from the `Pool<T>` parameter type.
///
/// ```rust,ignore
/// #[fabrique::test]
/// async fn test_create(pool: Pool<Sqlite>) {
///     let product = Product::factory().create(&pool).await.unwrap();
///     assert_eq!(product.name.is_empty(), false);
/// }
/// ```
#[cfg(feature = "testing")]
#[cfg_attr(coverage_nightly, coverage(off))]
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    match test::expand_test(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

/// Creates an in-memory SQLite database with migrations for
/// documentation examples.
///
/// This macro transforms an async function into an executable
/// doctest. It sets up a Tokio runtime, creates an in-memory SQLite
/// database, runs migrations, and provides the connection pool to
/// your test code.
///
/// Requires the `testing` and `sqlite` features. Doctests will fail
/// to compile without them — use `--lib --tests` to skip doctests
/// when running against other backends.
///
/// ```rust,ignore
/// # extern crate fabrique;
/// # extern crate tokio;
/// # extern crate uuid;
/// # use fabrique::prelude::*;
/// # #[derive(Model, Factory)]
/// # pub struct User { id: uuid::Uuid, name: String, email: String }
/// #[fabrique::doctest]
/// async fn main(pool: Pool<sqlx::Sqlite>) -> Result<(), fabrique::Error> {
///     let user = User::factory().create(&pool).await?;
///     user.delete(&pool).await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "testing")]
#[cfg_attr(coverage_nightly, coverage(off))]
#[proc_macro_attribute]
pub fn doctest(_attr: TokenStream, item: TokenStream) -> TokenStream {
    match test::expand_doctest(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
