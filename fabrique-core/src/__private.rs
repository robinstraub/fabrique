//! Internal module for macro support, not part of the public API.
//!
//! The `#[fabrique::test]` and `#[fabrique::doctest]` macros generate
//! code that calls functions from this module to set up test databases
//! with migrations. The module must be public for the generated code
//! to access it, but is marked `#[doc(hidden)]` to exclude it from
//! the public documentation.

use crate::{database::Pool, error::Error};
#[cfg(any(feature = "postgres", feature = "mysql"))]
use sqlx::AssertSqlSafe;
use sqlx::migrate::Migrator;
use std::path::Path;

/// Creates an in-memory SQLite pool with migrations applied.
#[cfg(feature = "sqlite")]
pub async fn create_sqlite_pool(migration_path: &str) -> Result<Pool<sqlx::Sqlite>, Error> {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    Migrator::new(Path::new(migration_path))
        .await
        .map_err(|e| Error::Other(Box::new(e)))?
        .run(&pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;
    Ok(pool)
}

/// Creates a temporary Postgres database with migrations applied.
///
/// Reads the connection URL from `POSTGRES_URL`. Returns the pool,
/// the base URL, and the temporary database name (for cleanup).
#[cfg(feature = "postgres")]
pub async fn create_postgres_pool(
    migration_path: &str,
) -> Result<(Pool<sqlx::Postgres>, String, String), Error> {
    let base_url =
        std::env::var("POSTGRES_URL").expect("POSTGRES_URL must be set for postgres tests");
    let db_name = temp_db_name();

    let base_pool = sqlx::PgPool::connect(&base_url).await?;
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE \"{db_name}\"")))
        .execute(&base_pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;
    drop(base_pool);

    let test_url = replace_db_in_url(&base_url, &db_name);
    let pool = sqlx::PgPool::connect(&test_url).await?;
    Migrator::new(Path::new(migration_path))
        .await
        .map_err(|e| Error::Other(Box::new(e)))?
        .run(&pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;

    Ok((pool, base_url, db_name))
}

/// Drops a temporary Postgres database.
#[cfg(feature = "postgres")]
pub async fn cleanup_test_db_postgres(base_url: &str, db_name: &str) {
    let pool = sqlx::PgPool::connect(base_url)
        .await
        .expect("cleanup: failed to connect");
    sqlx::raw_sql(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"
    )))
    .execute(&pool)
    .await
    .expect("cleanup: failed to drop test database");
}

/// Creates a temporary MySQL database with migrations applied.
///
/// Reads the connection URL from `MYSQL_URL`. Returns the pool,
/// the base URL, and the temporary database name (for cleanup).
#[cfg(feature = "mysql")]
pub async fn create_mysql_pool(
    migration_path: &str,
) -> Result<(Pool<sqlx::MySql>, String, String), Error> {
    let base_url = std::env::var("MYSQL_URL").expect("MYSQL_URL must be set for mysql tests");
    let db_name = temp_db_name();

    let base_pool = sqlx::MySqlPool::connect(&base_url).await?;
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE `{db_name}`")))
        .execute(&base_pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;
    drop(base_pool);

    let test_url = replace_db_in_url(&base_url, &db_name);
    let pool = sqlx::MySqlPool::connect(&test_url).await?;
    Migrator::new(Path::new(migration_path))
        .await
        .map_err(|e| Error::Other(Box::new(e)))?
        .run(&pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;

    Ok((pool, base_url, db_name))
}

/// Drops a temporary MySQL database.
#[cfg(feature = "mysql")]
pub async fn cleanup_test_db_mysql(base_url: &str, db_name: &str) {
    let pool = sqlx::MySqlPool::connect(base_url)
        .await
        .expect("cleanup: failed to connect");
    sqlx::raw_sql(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS `{db_name}`"
    )))
    .execute(&pool)
    .await
    .expect("cleanup: failed to drop test database");
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn temp_db_name() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("fabrique_test_{}_{}", std::process::id(), ts)
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn replace_db_in_url(url: &str, db_name: &str) -> String {
    match url.rsplit_once('/') {
        Some((base, _)) => format!("{base}/{db_name}"),
        None => format!("{url}/{db_name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn create_sqlite_pool_applies_migrations() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations/sqlite");
        let pool = create_sqlite_pool(path).await.expect("should create pool");
        let result: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
        assert_eq!(result.0, 1);
    }

    #[cfg(any(feature = "postgres", feature = "mysql"))]
    #[test]
    fn temp_db_name_has_expected_prefix() {
        let name = temp_db_name();
        assert!(name.starts_with("fabrique_test_"));
    }

    #[cfg(any(feature = "postgres", feature = "mysql"))]
    #[test]
    fn replace_db_in_url_swaps_last_segment() {
        assert_eq!(
            replace_db_in_url("postgres://user:pass@host/olddb", "newdb",),
            "postgres://user:pass@host/newdb"
        );
    }

    #[cfg(any(feature = "postgres", feature = "mysql"))]
    #[test]
    fn replace_db_in_url_appends_when_no_slash() {
        assert_eq!(
            replace_db_in_url("postgres://host", "newdb"),
            "postgres://host/newdb"
        );
    }
}
