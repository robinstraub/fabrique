//! Code generation for `#[fabrique::test]` and `#[fabrique::doctest]`.
//!
//! Parses an async test function, detects the database backend from
//! the `Pool<T>` parameter, and generates a `#[tokio::test]` function
//! that creates a temporary pool with migrations applied.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments, Type};

// ── Analysis ────────────────────────────────────────────────────

/// Database backend detected from the `Pool<T>` parameter type.
#[derive(Debug, PartialEq)]
pub enum Backend {
    Sqlite,
    Postgres,
    MySql,
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
    /// `Pool<Postgres>`, or `Pool<MySql>`.
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
                let backend = Self::parse_backend(&pat_type.ty)?;
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
    fn parse_backend(ty: &Type) -> Result<Backend, syn::Error> {
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
            other => Err(syn::Error::new_spanned(
                ident,
                format!("unsupported backend: {other}"),
            )),
        }
    }
}

// ── Codegen ─────────────────────────────────────────────────────

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
        let migration_path = Self::resolve_migration_path(&analysis.backend);
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

    /// Generates a `#[tokio::test]` function with pool setup and
    /// optional cleanup.
    pub fn generate(&self) -> TokenStream {
        let fn_name = &self.analysis.fn_name;
        let stmts = &self.analysis.body.stmts;
        let pool_setup = self.generate_pool_setup();
        let cleanup = self.generate_cleanup();

        quote! {
            #[::tokio::test]
            async fn #fn_name() {
                #pool_setup
                #(#stmts)*
                #cleanup
            }
        }
    }

    /// Generates a doctest wrapper with a blocking Tokio runtime.
    ///
    /// Unlike `generate()`, this wraps the body in a manual Tokio
    /// runtime since doctests cannot use `#[tokio::test]`.
    pub fn generate_doctest(&self) -> TokenStream {
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

    /// Generates the pool creation expression for the detected
    /// backend.
    fn generate_pool_setup(&self) -> TokenStream {
        let param = &self.analysis.param_name;
        let path = &self.migration_path;

        match self.analysis.backend {
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
        }
    }

    /// Generates cleanup code for backends that use temporary
    /// databases (Postgres, MySQL).
    fn generate_cleanup(&self) -> TokenStream {
        match self.analysis.backend {
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
        }
    }

    /// Returns the absolute migration path for the given backend.
    fn resolve_migration_path(backend: &Backend) -> String {
        let subdir = match backend {
            Backend::Sqlite => "sqlite",
            Backend::Postgres => "postgres",
            Backend::MySql => "mysql",
        };
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
