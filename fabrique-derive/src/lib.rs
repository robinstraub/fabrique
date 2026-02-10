#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Procedural macros for generating factory and model code.
//!
//! This crate provides two derive macros:
//! - `#[derive(Factory)]` - Generates factory structs with optional fields for
//!   flexible object creation
//! - `#[derive(Model)]` - Generates model implementations with database
//!   operations

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

mod analysis;
mod codegen;
mod error;

use crate::analysis::Analysis;
use crate::codegen::*;

/// Derives a `Model` implementation for the annotated struct.
///
/// This generates implementations for:
/// - `DatabaseAware` trait (database and error types)
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
    let database_aware = DatabaseAwareCodegen::new(&analysis).generate();
    let model = ModelCodegen::new(&analysis).generate();
    let columns = ColumnsCodegen::new(&analysis).generate();
    let belongs_to = BelongsToCodegen::new(&analysis).generate();
    let joinable = JoinableCodegen::new(&analysis).generate();
    let alias = AliasCodegen::new(&analysis).generate();
    let query = QueryCodegen::new(&analysis).generate();
    let persist = PersistCodegen::new(&analysis).generate();
    let delete = DeleteCodegen::new(&analysis).generate();
    let hard_delete = HardDeleteCodegen::new(&analysis).generate();
    let soft_delete = SoftDeleteCodegen::new(&analysis).generate();

    quote::quote! {
        #from_row
        #database_aware
        #model
        #columns
        #belongs_to
        #joinable
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

    FactoryCodegen::new(&analysis).generate_factory().into()
}

/// Test helper that wraps `#[sqlx::test]` with the correct migrations path
/// for the active database backend.
///
/// Requires the `testing` feature to be enabled.
///
/// ```rust,ignore
/// #[fabrique::test]
/// async fn test_create(pool: Pool<Backend>) {
///     let product = Product::factory().create(&pool).await.unwrap();
/// }
/// ```
#[cfg(feature = "testing")]
#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    #[cfg(feature = "postgres")]
    let migrations = "../migrations/postgres";
    #[cfg(feature = "sqlite")]
    let migrations = "../migrations/sqlite";
    #[cfg(feature = "mysql")]
    let migrations = "../migrations/mysql";

    quote::quote! {
        #[::sqlx::test(migrations = #migrations)]
        #input
    }
    .into()
}

/// Creates an in-memory SQLite database with migrations for documentation
/// examples.
///
/// This macro transforms an async function into an executable doctest. It sets
/// up a Tokio runtime, creates an in-memory SQLite database, runs migrations,
/// and provides the connection pool to your test code. Use `pool` as the
/// parameter name.
///
/// ```rust,ignore
/// # extern crate fabrique;
/// # extern crate tokio;
/// # extern crate uuid;
/// # use fabrique::prelude::*;
/// # #[derive(Model, Factory)]
/// # pub struct User { id: uuid::Uuid, name: String, email: String }
/// #[fabrique::doctest]
/// async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
///     let user = User::factory().create(&pool).await?;
///     user.delete(&pool).await?;
///     Ok(())
/// }
/// ```
// Tested via mdbook doctests, not unit tests - coverage measured separately
#[cfg_attr(coverage_nightly, coverage(off))]
#[proc_macro_attribute]
pub fn doctest(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let block = &input.block;

    quote::quote! {
        fn main() {
            ::tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime")
                .block_on(async {
                    let pool = ::fabrique::__private::doctest_pool()
                        .await
                        .expect("Failed to create doctest pool");
                    let __result: Result<(), ::fabrique::Error> = async #block.await;
                    __result.expect("Doctest failed");
                });
        }
    }
    .into()
}
