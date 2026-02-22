//! Code generation for `#[fabrique::test]` and `#[fabrique::doctest]`.
//!
//! Parses an async test function, detects the database backend from
//! the `Pool<T>` parameter, and generates a `#[tokio::test]` function
//! that creates a temporary pool with migrations applied.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, GenericParam, Generics, Ident, ItemFn, Pat, PathArguments, Type,
};

// ── Analysis ────────────────────────────────────────────────────

/// Database backend detected from the `Pool<T>` parameter type.
#[derive(Debug, PartialEq)]
pub enum Backend {
    Sqlite,
    Postgres,
    MySql,
    /// Generic backend from `<DB: Dialect>`. Generates one test
    /// per backend, each cfg-gated by feature.
    Multi(Ident),
}

/// Parsed representation of a `#[fabrique::test]` function.
///
/// Extracts the function name, parameter name, body, and database
/// backend from the function signature.
pub struct TestAnalysis {
    pub fn_name: Ident,
    pub param_name: Ident,
    pub body: syn::Block,
    pub backend: Backend,
}

impl TestAnalysis {
    /// Parses an async test function into its components.
    ///
    /// Expects exactly one parameter of type `Pool<Sqlite>`,
    /// `Pool<Postgres>`, `Pool<MySql>`, or `Pool<DB>` where `DB`
    /// is a generic type parameter (multi-backend).
    pub fn from(input: &ItemFn) -> Result<Self, syn::Error> {
        let fn_name = input.sig.ident.clone();
        let body = (*input.block).clone();

        let param = input.sig.inputs.first().ok_or_else(|| {
            syn::Error::new_spanned(
                &input.sig,
                "expected a pool parameter, \
                 e.g. `pool: Pool<Sqlite>`",
            )
        })?;

        let (param_name, backend) = match param {
            FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    Pat::Ident(pi) => pi.ident.clone(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &pat_type.pat,
                            "expected a simple identifier",
                        ));
                    }
                };
                let backend = Self::parse_backend(&pat_type.ty, &input.sig.generics)?;
                (name, backend)
            }
            _ => return Err(syn::Error::new_spanned(param, "expected a typed parameter")),
        };

        Ok(Self {
            fn_name,
            param_name,
            body,
            backend,
        })
    }

    /// Extracts the backend variant from a `Pool<T>` type.
    ///
    /// Recognises `Sqlite`, `Postgres`, and `MySql` as the last
    /// segment of the inner type path (e.g. `sqlx::Sqlite`).
    /// If the inner type matches a generic type parameter from the
    /// function signature, returns `Backend::Multi`.
    fn parse_backend(ty: &Type, generics: &Generics) -> Result<Backend, syn::Error> {
        let type_path = match ty {
            Type::Path(tp) => tp,
            _ => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "expected Pool<Sqlite|Postgres|MySql>",
                ));
            }
        };

        let segment = type_path
            .path
            .segments
            .last()
            .ok_or_else(|| syn::Error::new_spanned(ty, "expected Pool<…>"))?;

        if segment.ident != "Pool" {
            return Err(syn::Error::new_spanned(
                &segment.ident,
                "expected Pool type",
            ));
        }

        let args = match &segment.arguments {
            PathArguments::AngleBracketed(a) => a,
            _ => {
                return Err(syn::Error::new_spanned(
                    segment,
                    "expected Pool<…> with angle brackets",
                ));
            }
        };

        let arg = args
            .args
            .first()
            .ok_or_else(|| syn::Error::new_spanned(args, "expected a type argument"))?;

        let inner = match arg {
            GenericArgument::Type(Type::Path(tp)) => tp,
            _ => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "expected a path type argument",
                ));
            }
        };

        let ident = &inner
            .path
            .segments
            .last()
            .ok_or_else(|| syn::Error::new_spanned(inner, "empty path"))?
            .ident;

        match ident.to_string().as_str() {
            "Sqlite" => Ok(Backend::Sqlite),
            "Postgres" => Ok(Backend::Postgres),
            "MySql" => Ok(Backend::MySql),
            _ => {
                let is_generic = generics
                    .params
                    .iter()
                    .any(|p| matches!(p, GenericParam::Type(tp) if tp.ident == *ident));
                if is_generic {
                    Ok(Backend::Multi(ident.clone()))
                } else {
                    Err(syn::Error::new_spanned(
                        ident,
                        format!("unsupported backend: {ident}"),
                    ))
                }
            }
        }
    }
}

// ── Codegen ─────────────────────────────────────────────────────

/// All concrete backends, in generation order.
const CONCRETE_BACKENDS: [Backend; 3] = [Backend::Sqlite, Backend::Postgres, Backend::MySql];

impl Backend {
    /// Feature flag name for cfg-gating.
    fn feature(&self) -> &'static str {
        match self {
            Backend::Sqlite => "sqlite",
            Backend::Postgres => "postgres",
            Backend::MySql => "mysql",
            Backend::Multi(_) => unreachable!(),
        }
    }

    /// Fully-qualified sqlx type path.
    fn sqlx_type(&self) -> TokenStream {
        match self {
            Backend::Sqlite => quote! { ::sqlx::Sqlite },
            Backend::Postgres => quote! { ::sqlx::Postgres },
            Backend::MySql => quote! { ::sqlx::MySql },
            Backend::Multi(_) => unreachable!(),
        }
    }
}

/// Generates pool creation tokens for a concrete backend.
fn pool_setup_tokens(backend: &Backend, param: &Ident, path: &str) -> TokenStream {
    match backend {
        Backend::Sqlite => quote! {
            let #param =
                ::fabrique::__private::create_sqlite_pool(#path)
                    .await
                    .expect("Failed to create test pool");
        },
        Backend::Postgres => quote! {
            let (#param, __base_url, __db_name) =
                ::fabrique::__private::create_postgres_pool(#path)
                    .await
                    .expect("Failed to create test pool");
        },
        Backend::MySql => quote! {
            let (#param, __base_url, __db_name) =
                ::fabrique::__private::create_mysql_pool(#path)
                    .await
                    .expect("Failed to create test pool");
        },
        Backend::Multi(_) => unreachable!(),
    }
}

/// Generates cleanup tokens for a concrete backend.
fn cleanup_tokens(backend: &Backend) -> TokenStream {
    match backend {
        Backend::Sqlite => quote! {},
        Backend::Postgres => quote! {
            ::fabrique::__private::cleanup_test_db_postgres(
                &__base_url,
                &__db_name,
            )
            .await;
        },
        Backend::MySql => quote! {
            ::fabrique::__private::cleanup_test_db_mysql(
                &__base_url,
                &__db_name,
            )
            .await;
        },
        Backend::Multi(_) => unreachable!(),
    }
}

/// Code generator for `#[fabrique::test]` and `#[fabrique::doctest]`.
///
/// Takes a parsed test function and generates a `#[tokio::test]`
/// wrapper that creates a pool with migrations applied and runs
/// cleanup when needed.
pub struct TestCodegen<'a> {
    analysis: &'a TestAnalysis,
    migration_path: String,
}

impl<'a> TestCodegen<'a> {
    /// Creates a code generator, resolving the migration path
    /// from the workspace root.
    pub fn new(analysis: &'a TestAnalysis) -> Self {
        let migration_path = match &analysis.backend {
            Backend::Multi(_) => String::new(),
            backend => Self::resolve_migration_path(backend),
        };
        Self {
            analysis,
            migration_path,
        }
    }

    /// Creates a code generator with an explicit migration path.
    ///
    /// Useful for testing without filesystem dependency.
    #[cfg(test)]
    fn with_path(analysis: &'a TestAnalysis, path: &str) -> Self {
        Self {
            analysis,
            migration_path: path.to_owned(),
        }
    }

    /// Generates test function(s). For a concrete backend, emits
    /// a single `#[tokio::test]`. For `Multi`, emits one per
    /// backend, each cfg-gated.
    pub fn generate(&self) -> TokenStream {
        match &self.analysis.backend {
            Backend::Multi(type_param) => self.generate_multi(type_param),
            _ => self.generate_single(),
        }
    }

    /// Generates a doctest wrapper with a blocking Tokio runtime.
    ///
    /// Unlike `generate()`, this wraps the body in a manual Tokio
    /// runtime since doctests cannot use `#[tokio::test]`.
    /// Multi-backend is not supported for doctests.
    pub fn generate_doctest(&self) -> TokenStream {
        if matches!(self.analysis.backend, Backend::Multi(_)) {
            return syn::Error::new_spanned(
                &self.analysis.fn_name,
                "#[fabrique::doctest] does not support \
                 generic backends",
            )
            .into_compile_error();
        }

        let body = &self.analysis.body;
        let param = &self.analysis.param_name;
        let path = &self.migration_path;

        quote! {
            fn main() {
                ::tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime")
                    .block_on(async {
                        let #param =
                            ::fabrique::__private::create_sqlite_pool(#path)
                                .await
                                .expect("Failed to create doctest pool");
                        let __result: Result<(), ::fabrique::Error> =
                            async #body .await;
                        __result.expect("Doctest failed");
                    });
            }
        }
    }

    /// Generates a single `#[tokio::test]` for a concrete backend.
    fn generate_single(&self) -> TokenStream {
        let fn_name = &self.analysis.fn_name;
        let stmts = &self.analysis.body.stmts;
        let param = &self.analysis.param_name;
        let setup = pool_setup_tokens(&self.analysis.backend, param, &self.migration_path);
        let cleanup = cleanup_tokens(&self.analysis.backend);

        quote! {
            #[::tokio::test]
            async fn #fn_name() {
                #setup
                #(#stmts)*
                #cleanup
            }
        }
    }

    /// Generates three cfg-gated `#[tokio::test]` functions,
    /// one per backend. Each introduces a `type DB = ...;` alias
    /// so the body can reference the generic type parameter.
    fn generate_multi(&self, type_param: &Ident) -> TokenStream {
        let stmts = &self.analysis.body.stmts;
        let param = &self.analysis.param_name;

        let fns = CONCRETE_BACKENDS.iter().map(|backend| {
            let feature = backend.feature();
            let db_type = backend.sqlx_type();
            let suffix = feature;
            let fn_name = format_ident!("{}_{}", self.analysis.fn_name, suffix,);
            let path = Self::resolve_migration_path(backend);
            let setup = pool_setup_tokens(backend, param, &path);
            let cleanup = cleanup_tokens(backend);

            quote! {
                #[cfg(feature = #feature)]
                #[::tokio::test]
                async fn #fn_name() {
                    type #type_param = #db_type;
                    #setup
                    #(#stmts)*
                    #cleanup
                }
            }
        });

        quote! { #(#fns)* }
    }

    /// Returns the absolute migration path for the given backend.
    fn resolve_migration_path(backend: &Backend) -> String {
        let subdir = backend.feature();
        workspace_root()
            .join("migrations")
            .join(subdir)
            .to_string_lossy()
            .into_owned()
    }
}

// ── Helpers ─────────────────────────────────────────────────────

/// Finds the workspace root by walking up from
/// `CARGO_MANIFEST_DIR`.
///
/// Looks for a `Cargo.toml` containing a `[workspace]` section.
/// Panics if no workspace root is found.
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let mut path = std::path::PathBuf::from(manifest_dir);

    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml).expect("failed to read Cargo.toml");
            if content.contains("[workspace]") {
                return path;
            }
        }
        if !path.pop() {
            panic!(
                "could not find workspace root \
                 (Cargo.toml with [workspace])"
            );
        }
    }
}

// ── Entry points ────────────────────────────────────────────────

/// Entry point for `#[fabrique::test]`.
pub fn expand_test(_attr: TokenStream, item: TokenStream) -> Result<TokenStream, syn::Error> {
    let input: ItemFn = syn::parse2(item)?;
    let analysis = TestAnalysis::from(&input)?;
    let codegen = TestCodegen::new(&analysis);
    Ok(codegen.generate())
}

/// Entry point for `#[fabrique::doctest]`.
pub fn expand_doctest(item: TokenStream) -> Result<TokenStream, syn::Error> {
    let input: ItemFn = syn::parse2(item)?;
    let analysis = TestAnalysis::from(&input)?;
    let codegen = TestCodegen::new(&analysis);
    Ok(codegen.generate_doctest())
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_analysis_parses_sqlite_backend() {
        let input: ItemFn = parse_quote! {
            async fn my_test(pool: Pool<Sqlite>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert_eq!(analysis.fn_name, "my_test");
        assert_eq!(analysis.param_name, "pool");
        assert_eq!(analysis.backend, Backend::Sqlite);
    }

    #[test]
    fn test_analysis_parses_postgres_backend() {
        let input: ItemFn = parse_quote! {
            async fn my_test(pool: Pool<Postgres>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert_eq!(analysis.backend, Backend::Postgres);
    }

    #[test]
    fn test_analysis_parses_mysql_backend() {
        let input: ItemFn = parse_quote! {
            async fn my_test(pool: Pool<MySql>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert_eq!(analysis.backend, Backend::MySql);
    }

    #[test]
    fn test_analysis_parses_qualified_path() {
        let input: ItemFn = parse_quote! {
            async fn my_test(pool: Pool<sqlx::Sqlite>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert_eq!(analysis.backend, Backend::Sqlite);
    }

    #[test]
    fn test_analysis_rejects_missing_param() {
        let input: ItemFn = parse_quote! {
            async fn my_test() {}
        };
        let result = TestAnalysis::from(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_analysis_rejects_unsupported_backend() {
        let input: ItemFn = parse_quote! {
            async fn my_test(pool: Pool<Mssql>) {}
        };
        let result = TestAnalysis::from(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_sqlite_test() {
        let input: ItemFn = parse_quote! {
            async fn test_create(pool: Pool<Sqlite>) {
                let result = Product::all(&pool).await;
                assert!(result.is_ok());
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::with_path(&analysis, "/ws/migrations/sqlite");

        let generated = codegen.generate();

        assert_eq!(
            generated.to_string(),
            quote! {
                #[::tokio::test]
                async fn test_create() {
                    let pool =
                        ::fabrique::__private::create_sqlite_pool(
                            "/ws/migrations/sqlite"
                        )
                            .await
                            .expect("Failed to create test pool");
                    let result = Product::all(&pool).await;
                    assert!(result.is_ok());
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_postgres_test() {
        let input: ItemFn = parse_quote! {
            async fn test_create(pool: Pool<Postgres>) {
                let result = Product::all(&pool).await;
                assert!(result.is_ok());
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::with_path(&analysis, "/ws/migrations/postgres");

        let generated = codegen.generate();

        assert_eq!(
            generated.to_string(),
            quote! {
                #[::tokio::test]
                async fn test_create() {
                    let (pool, __base_url, __db_name) =
                        ::fabrique::__private::create_postgres_pool(
                            "/ws/migrations/postgres"
                        )
                            .await
                            .expect("Failed to create test pool");
                    let result = Product::all(&pool).await;
                    assert!(result.is_ok());
                    ::fabrique::__private::cleanup_test_db_postgres(
                        &__base_url,
                        &__db_name,
                    )
                    .await;
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_mysql_test() {
        let input: ItemFn = parse_quote! {
            async fn test_create(pool: Pool<MySql>) {
                let result = Product::all(&pool).await;
                assert!(result.is_ok());
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::with_path(&analysis, "/ws/migrations/mysql");

        let generated = codegen.generate();

        assert_eq!(
            generated.to_string(),
            quote! {
                #[::tokio::test]
                async fn test_create() {
                    let (pool, __base_url, __db_name) =
                        ::fabrique::__private::create_mysql_pool(
                            "/ws/migrations/mysql"
                        )
                            .await
                            .expect("Failed to create test pool");
                    let result = Product::all(&pool).await;
                    assert!(result.is_ok());
                    ::fabrique::__private::cleanup_test_db_mysql(
                        &__base_url,
                        &__db_name,
                    )
                    .await;
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_analysis_parses_multi_backend() {
        let input: ItemFn = parse_quote! {
            async fn my_test<DB: Dialect>(pool: Pool<DB>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert_eq!(analysis.fn_name, "my_test");
        assert_eq!(analysis.param_name, "pool");
        assert!(matches!(&analysis.backend, Backend::Multi(id) if id == "DB"),);
    }

    #[test]
    fn test_analysis_parses_multi_custom_name() {
        let input: ItemFn = parse_quote! {
            async fn my_test<T: Dialect>(pool: Pool<T>) {}
        };
        let analysis = TestAnalysis::from(&input).unwrap();

        assert!(matches!(&analysis.backend, Backend::Multi(id) if id == "T"),);
    }

    #[test]
    fn test_generate_multi_backend() {
        let input: ItemFn = parse_quote! {
            async fn test_create<DB: Dialect>(pool: Pool<DB>) {
                let result = Product::all(&pool).await;
                assert!(result.is_ok());
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::new(&analysis);

        let output = codegen.generate().to_string();

        // Three cfg-gated functions are generated
        assert!(output.contains("fn test_create_sqlite"));
        assert!(output.contains("fn test_create_postgres"));
        assert!(output.contains("fn test_create_mysql"));
        assert!(output.contains("# [cfg (feature = \"sqlite\")]"));
        assert!(output.contains("# [cfg (feature = \"postgres\")]"));
        assert!(output.contains("# [cfg (feature = \"mysql\")]"));

        // Type aliases for the generic parameter
        assert!(output.contains("type DB = :: sqlx :: Sqlite"));
        assert!(output.contains("type DB = :: sqlx :: Postgres"));
        assert!(output.contains("type DB = :: sqlx :: MySql"));
    }

    #[test]
    fn test_doctest_rejects_multi_backend() {
        let input: ItemFn = parse_quote! {
            async fn main<DB: Dialect>(
                pool: Pool<DB>,
            ) -> Result<(), fabrique::Error> {
                Ok(())
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::with_path(&analysis, "/ws/migrations/sqlite");

        let output = codegen.generate_doctest().to_string();
        assert!(output.contains("compile_error"));
    }

    #[test]
    fn test_generate_doctest() {
        let input: ItemFn = parse_quote! {
            async fn main(
                pool: Pool<Sqlite>,
            ) -> Result<(), fabrique::Error> {
                let _user = User::factory().create(&pool).await?;
                Ok(())
            }
        };
        let analysis = TestAnalysis::from(&input).unwrap();
        let codegen = TestCodegen::with_path(&analysis, "/ws/migrations/sqlite");

        let generated = codegen.generate_doctest();

        assert_eq!(
            generated.to_string(),
            quote! {
                fn main() {
                    ::tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create Tokio runtime")
                        .block_on(async {
                            let pool =
                                ::fabrique::__private::create_sqlite_pool(
                                    "/ws/migrations/sqlite"
                                )
                                    .await
                                    .expect("Failed to create doctest pool");
                            let __result: Result<(), ::fabrique::Error> =
                                async {
                                    let _user = User::factory()
                                        .create(&pool).await?;
                                    Ok(())
                                }.await;
                            __result.expect("Doctest failed");
                        });
                }
            }
            .to_string()
        );
    }
}
