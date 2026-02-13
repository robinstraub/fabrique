use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, GenericParam, Ident, ItemFn, Pat, PathArguments, Stmt, Type,
    TypeParamBound,
};

use crate::error::{Error, ErrorKind};

/// Two modes of operation for `#[fabrique::test]`.
enum TestMode<'a> {
    /// `async fn test<DB: Dialect>(pool: Pool<DB>)` — one test per active
    /// backend.
    Generic { generic_name: &'a Ident },
    /// `async fn test(pool: Pool<Postgres>)` — single backend.
    Concrete { backend: BackendConfig },
}

/// Parsed representation of a `#[fabrique::test]` function.
struct TestAnalysis<'a> {
    fn_name: &'a Ident,
    mode: TestMode<'a>,
    param_name: &'a Ident,
    stmts: &'a [Stmt],
}

impl<'a> TestAnalysis<'a> {
    fn from(input: &'a ItemFn) -> Result<Self, Error> {
        let param_name = Self::extract_param_name(input)?;

        let mode = if input.sig.generics.params.is_empty() {
            // Concrete mode: no generics, extract backend from Pool<Type>
            let backend = Self::extract_concrete_backend(input)?;
            TestMode::Concrete { backend }
        } else {
            // Generic mode: one generic bounded by Dialect
            let generic_name = Self::extract_dialect_generic(input)?;
            TestMode::Generic { generic_name }
        };

        Ok(Self {
            fn_name: &input.sig.ident,
            mode,
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

    /// Extracts the concrete backend from `Pool<Postgres>` / `Pool<Sqlite>` /
    /// `Pool<MySql>`.
    fn extract_concrete_backend(input: &ItemFn) -> Result<BackendConfig, Error> {
        let err = || Error::new(input.sig.ident.span(), ErrorKind::InvalidTestSignature);

        let param = input.sig.inputs.first().ok_or_else(err)?;
        let FnArg::Typed(pat_type) = param else {
            return Err(err());
        };

        // Dig into the type: Pool<SomeType>
        let Type::Path(type_path) = &*pat_type.ty else {
            return Err(err());
        };

        let pool_seg = type_path.path.segments.last().ok_or_else(err)?;
        if pool_seg.ident != "Pool" {
            return Err(err());
        }

        let PathArguments::AngleBracketed(args) = &pool_seg.arguments else {
            return Err(err());
        };

        if args.args.len() != 1 {
            return Err(err());
        }

        let GenericArgument::Type(Type::Path(inner_path)) = &args.args[0] else {
            return Err(err());
        };

        let type_name = inner_path
            .path
            .segments
            .last()
            .ok_or_else(err)?
            .ident
            .to_string();

        let feature = match type_name.as_str() {
            "Postgres" => "postgres",
            "Sqlite" => "sqlite",
            "MySql" => "mysql",
            _ => {
                return Err(Error::new(
                    input.sig.ident.span(),
                    ErrorKind::UnknownBackendType(type_name),
                ));
            }
        };

        backends()
            .into_iter()
            .find(|b| b.suffix == feature)
            .ok_or_else(|| {
                Error::new(
                    input.sig.ident.span(),
                    ErrorKind::BackendFeatureNotEnabled(feature.to_owned()),
                )
            })
    }
}

/// Generates test functions from a `#[fabrique::test]` annotated function.
///
/// In generic mode (`<DB: Dialect>`), one concrete test is generated per active
/// backend feature. In concrete mode (`Pool<Postgres>`), a single test is
/// generated for that specific backend.
pub fn generate(input: &ItemFn) -> Result<TokenStream, Error> {
    let analysis = TestAnalysis::from(input)?;

    match &analysis.mode {
        TestMode::Generic { generic_name } => {
            let active_backends = backends();
            if active_backends.is_empty() {
                return Err(Error::new(
                    analysis.fn_name.span(),
                    ErrorKind::NoBackendFeature,
                ));
            }

            let tests = active_backends.iter().map(|backend| {
                let db_type = &backend.db_type;
                let migrations = &backend.migrations;
                let stmts = analysis.stmts;
                let suffixed_name = format_ident!("{}_{}", analysis.fn_name, backend.suffix);
                let param = analysis.param_name;
                let generic = *generic_name;

                quote! {
                    #[test]
                    fn #suffixed_name() {
                        static MIGRATOR: ::sqlx::migrate::Migrator =
                            ::sqlx::migrate!(#migrations);
                        ::fabrique::testing::run_test::<#db_type, _, _>(
                            concat!(module_path!(), "::", stringify!(#suffixed_name)),
                            &MIGRATOR,
                            |#param: ::sqlx::Pool<#db_type>| async move {
                                type #generic = #db_type;

                                #(#stmts)*
                            },
                        );
                    }
                }
            });

            Ok(quote! { #(#tests)* })
        }
        TestMode::Concrete { backend } => {
            let db_type = &backend.db_type;
            let migrations = &backend.migrations;
            let stmts = analysis.stmts;
            let fn_name = analysis.fn_name;
            let param = analysis.param_name;

            Ok(quote! {
                #[test]
                fn #fn_name() {
                    static MIGRATOR: ::sqlx::migrate::Migrator =
                        ::sqlx::migrate!(#migrations);
                    ::fabrique::testing::run_test::<#db_type, _, _>(
                        concat!(module_path!(), "::", stringify!(#fn_name)),
                        &MIGRATOR,
                        |#param: ::sqlx::Pool<#db_type>| async move {
                            #(#stmts)*
                        },
                    );
                }
            })
        }
    }
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
        quote! { ::sqlx::MySql },
    ));

    #[cfg(feature = "postgres")]
    backends.push(BackendConfig::new(
        "postgres",
        "../migrations/postgres",
        quote! { ::sqlx::Postgres },
    ));

    #[cfg(feature = "sqlite")]
    backends.push(BackendConfig::new(
        "sqlite",
        "../migrations/sqlite",
        quote! { ::sqlx::Sqlite },
    ));

    backends
}
