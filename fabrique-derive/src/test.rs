use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, GenericParam, Ident, ItemFn, Pat, Stmt, TypeParamBound};

use crate::error::{Error, ErrorKind};

/// Parsed representation of a `#[fabrique::test]` function.
///
/// Validates that the function has exactly one generic parameter bounded by
/// `Dialect` and exactly one pool parameter.
struct TestAnalysis<'a> {
    fn_name: &'a Ident,
    generic_name: &'a Ident,
    param_name: &'a Ident,
    stmts: &'a [Stmt],
}

impl<'a> TestAnalysis<'a> {
    /// Analyzes a `#[fabrique::test]` function signature.
    ///
    /// Returns an error if the signature doesn't match
    /// `async fn name<DB: Dialect>(pool: Pool<DB>)`.
    fn from(input: &'a ItemFn) -> Result<Self, Error> {
        let generic_name = Self::extract_dialect_generic(input)?;
        let param_name = Self::extract_param_name(input)?;

        Ok(Self {
            fn_name: &input.sig.ident,
            generic_name,
            param_name,
            stmts: &input.block.stmts,
        })
    }

    fn extract_dialect_generic(input: &'a ItemFn) -> Result<&'a Ident, Error> {
        let err = || Error::new(input.sig.ident.span(), ErrorKind::InvalidTestSignature);

        if input.sig.generics.params.len() != 1 {
            return Err(err());
        }

        let param = input.sig.generics.params.first().ok_or_else(err)?;
        let GenericParam::Type(type_param) = param else {
            return Err(err());
        };

        let has_dialect = type_param.bounds.iter().any(|bound| {
            matches!(bound, TypeParamBound::Trait(tb)
                if tb.path.segments.last().is_some_and(|s| s.ident == "Dialect"))
        });

        if has_dialect {
            Ok(&type_param.ident)
        } else {
            Err(err())
        }
    }

    fn extract_param_name(input: &'a ItemFn) -> Result<&'a Ident, Error> {
        let err = || Error::new(input.sig.ident.span(), ErrorKind::InvalidTestSignature);

        if input.sig.inputs.len() != 1 {
            return Err(err());
        }

        match input.sig.inputs.first().ok_or_else(err)? {
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pat_ident) => Ok(&pat_ident.ident),
                _ => Err(err()),
            },
            _ => Err(err()),
        }
    }
}

/// Generates test functions from a `#[fabrique::test]` annotated function.
///
/// One concrete test is generated per active backend feature, each with a
/// type alias so the body can reference the generic name.
pub fn generate(input: &ItemFn) -> Result<TokenStream, Error> {
    let analysis = TestAnalysis::from(input)?;

    let backends = backends();
    if backends.is_empty() {
        return Err(Error::new(
            analysis.fn_name.span(),
            ErrorKind::NoBackendFeature,
        ));
    }

    if backends.len() > 1 {
        return Err(Error::new(
            analysis.fn_name.span(),
            ErrorKind::MultipleBackendFeatures,
        ));
    }

    let tests = backends.iter().map(|backend| {
        let db_type = &backend.db_type;
        let migrations = &backend.migrations;
        let stmts = analysis.stmts;
        let suffixed_name = format_ident!("{}_{}", analysis.fn_name, backend.suffix);
        let param = analysis.param_name;
        let generic = analysis.generic_name;

        quote! {
            #[::sqlx::test(migrations = #migrations)]
            async fn #suffixed_name(#param: ::sqlx::Pool<#db_type>) {
                type #generic = #db_type;

                #(#stmts)*
            }
        }
    });

    Ok(quote! { #(#tests)* })
}

struct BackendConfig {
    migrations: &'static str,
    suffix: &'static str,
    db_type: TokenStream,
}

impl BackendConfig {
    pub fn new(suffix: &'static str, migrations: &'static str, db_type: TokenStream) -> Self {
        BackendConfig {
            migrations,
            suffix,
            db_type,
        }
    }
}

fn backends() -> Vec<BackendConfig> {
    let mut backends = Vec::new();

    #[cfg(feature = "mysql")]
    backends.push(BackendConfig::new(
        "mysql",
        "../migrations/mysql",
        quote! {::sqlx::Mysql},
    ));

    #[cfg(feature = "postgres")]
    backends.push(BackendConfig::new(
        "postgres",
        "../migrations/postgres",
        quote! { ::sqlx::Postgres},
    ));

    #[cfg(feature = "sqlite")]
    backends.push(BackendConfig::new(
        "sqlite",
        "../migrations/sqlite",
        quote! { ::sqlx::Sqlite },
    ));

    backends
}
