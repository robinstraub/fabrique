use std::future::Future;

/// Backend-specific test database lifecycle.
///
/// Each supported database implements this trait to handle creating isolated
/// test databases, running migrations, and cleaning up after tests.
pub trait TestBackend: sqlx::Database + Sized {
    /// Creates an isolated test database and runs migrations.
    ///
    /// Returns `None` if the backend's URL environment variable is not set,
    /// causing the test to be silently skipped.
    fn setup(
        test_path: &str,
        migrator: &sqlx::migrate::Migrator,
    ) -> impl Future<Output = Option<(sqlx::Pool<Self>, String)>> + Send;

    /// Drops the test database after a successful test run.
    ///
    /// Only called when the test passes — on failure the database is left
    /// behind for debugging.
    fn cleanup(db_name: &str) -> impl Future<Output = ()> + Send;
}

/// Runs a test function against an isolated database.
///
/// This is the entry point called by `#[fabrique::test]`-generated code.
/// It creates a single-threaded Tokio runtime, sets up a test database via
/// `TestBackend::setup`, executes the test, then cleans up on success.
pub fn run_test<DB, F, Fut>(test_path: &str, migrator: &sqlx::migrate::Migrator, test_fn: F)
where
    DB: TestBackend,
    F: FnOnce(sqlx::Pool<DB>) -> Fut,
    Fut: Future<Output = ()>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let Some((pool, db_name)) = DB::setup(test_path, migrator).await else {
            eprintln!("skipping {test_path}: no database URL configured");
            return;
        };
        test_fn(pool.clone()).await;
        pool.close().await;
        DB::cleanup(&db_name).await;
    });
}

/// Generates a deterministic database name from a test path.
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn db_name(test_path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::hash::DefaultHasher::new();
    test_path.hash(&mut h);
    format!("_fabrique_test_{:016x}", h.finish())
}

/// Reads a backend-specific URL with fallback to `DATABASE_URL`.
///
/// Returns `None` if neither the backend var nor a matching `DATABASE_URL`
/// is found.
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn backend_url(env_var: &str, prefixes: &[&str]) -> Option<String> {
    if let Ok(url) = std::env::var(env_var) {
        return Some(url);
    }
    if let Ok(url) = std::env::var("DATABASE_URL")
        && prefixes.iter().any(|p| url.starts_with(p))
    {
        return Some(url);
    }
    None
}

/// Executes a raw DDL statement.
///
/// Uses `raw_sql` + `AssertSqlSafe` because DDL cannot use prepared
/// statements. The input is safe — database names are deterministic hashes.
#[cfg(any(feature = "postgres", feature = "mysql"))]
async fn ddl<'a, E>(executor: E, sql: String) -> sqlx::Result<()>
where
    E: sqlx::Executor<'a>,
{
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
        .execute(executor)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------
#[cfg(feature = "sqlite")]
#[allow(clippy::manual_async_fn)]
impl TestBackend for sqlx::Sqlite {
    fn setup(
        _test_path: &str,
        migrator: &sqlx::migrate::Migrator,
    ) -> impl Future<Output = Option<(sqlx::Pool<Self>, String)>> + Send {
        async move {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .expect("failed to create in-memory SQLite pool");
            migrator
                .run(&pool)
                .await
                .expect("failed to run SQLite migrations");
            Some((pool, String::new()))
        }
    }

    fn cleanup(_db_name: &str) -> impl Future<Output = ()> + Send {
        async {}
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------
#[cfg(feature = "postgres")]
impl TestBackend for sqlx::Postgres {
    fn setup(
        test_path: &str,
        migrator: &sqlx::migrate::Migrator,
    ) -> impl Future<Output = Option<(sqlx::Pool<Self>, String)>> + Send {
        let test_path = test_path.to_owned();
        async move {
            let url = backend_url("POSTGRES_URL", &["postgres://", "postgresql://"])?;
            let name = db_name(&test_path);

            let master_opts: sqlx::postgres::PgConnectOptions =
                url.parse().expect("failed to parse POSTGRES_URL");

            let master = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_with(master_opts.clone())
                .await
                .expect("failed to connect to PostgreSQL master");

            let _ = ddl(&master, format!("DROP DATABASE IF EXISTS \"{name}\"")).await;
            ddl(&master, format!("CREATE DATABASE \"{name}\""))
                .await
                .expect("failed to create test database");
            master.close().await;

            let test_opts = master_opts.database(&name);
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .connect_with(test_opts)
                .await
                .expect("failed to connect to test database");
            migrator
                .run(&pool)
                .await
                .expect("failed to run PostgreSQL migrations");

            Some((pool, name))
        }
    }

    fn cleanup(db_name: &str) -> impl Future<Output = ()> + Send {
        let db_name = db_name.to_owned();
        async move {
            if db_name.is_empty() {
                return;
            }
            let Some(url) = backend_url("POSTGRES_URL", &["postgres://", "postgresql://"]) else {
                return;
            };
            let master_opts: sqlx::postgres::PgConnectOptions =
                url.parse().expect("failed to parse POSTGRES_URL");
            let master = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_with(master_opts)
                .await
                .expect("failed to connect for cleanup");
            let _ = ddl(&master, format!("DROP DATABASE IF EXISTS \"{db_name}\"")).await;
            master.close().await;
        }
    }
}

// ---------------------------------------------------------------------------
// MySQL
// ---------------------------------------------------------------------------
#[cfg(feature = "mysql")]
impl TestBackend for sqlx::MySql {
    fn setup(
        test_path: &str,
        migrator: &sqlx::migrate::Migrator,
    ) -> impl Future<Output = Option<(sqlx::Pool<Self>, String)>> + Send {
        let test_path = test_path.to_owned();
        async move {
            let url = backend_url("MYSQL_URL", &["mysql://"])?;
            let name = db_name(&test_path);

            let master_opts: sqlx::mysql::MySqlConnectOptions =
                url.parse().expect("failed to parse MYSQL_URL");

            let master = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(1)
                .connect_with(master_opts.clone())
                .await
                .expect("failed to connect to MySQL master");

            let _ = ddl(&master, format!("DROP DATABASE IF EXISTS `{name}`")).await;
            ddl(&master, format!("CREATE DATABASE `{name}`"))
                .await
                .expect("failed to create test database");
            master.close().await;

            let test_opts = master_opts.database(&name);
            let pool = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(5)
                .connect_with(test_opts)
                .await
                .expect("failed to connect to test database");
            migrator
                .run(&pool)
                .await
                .expect("failed to run MySQL migrations");

            Some((pool, name))
        }
    }

    fn cleanup(db_name: &str) -> impl Future<Output = ()> + Send {
        let db_name = db_name.to_owned();
        async move {
            if db_name.is_empty() {
                return;
            }
            let Some(url) = backend_url("MYSQL_URL", &["mysql://"]) else {
                return;
            };
            let master_opts: sqlx::mysql::MySqlConnectOptions =
                url.parse().expect("failed to parse MYSQL_URL");
            let master = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(1)
                .connect_with(master_opts)
                .await
                .expect("failed to connect for cleanup");
            let _ = ddl(&master, format!("DROP DATABASE IF EXISTS `{db_name}`")).await;
            master.close().await;
        }
    }
}
