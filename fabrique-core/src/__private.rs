//! Internal module for macro support, not part of the public API.
//!
//! Proc-macro crates can only export macros, not runtime functions. The
//! `#[fabrique::doctest]` macro generates code that calls functions from this
//! module to set up test databases with migrations. The module must be public
//! for the generated code to access it, but is marked `#[doc(hidden)]` to
//! exclude it from the public documentation.

use crate::{Backend, database::Pool, error::Error};

/// Creates an in-memory SQLite test pool with migrations applied.
///
/// The `sqlx::migrate!()` macro embeds migration files at compile time,
/// resolving the path when this crate is compiled. Inlining the migrate call in
/// macro-generated code would fail since doctests compile in a temporary
/// directory without access to the migrations folder. By defining this function
/// here, migrations are embedded into the library and available at runtime.
#[cfg(feature = "sqlite")]
pub async fn doctest_pool() -> Result<Pool<Backend>, Error> {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    sqlx::migrate!("../migrations/sqlite")
        .run(&pool)
        .await
        .map_err(|e| Error::Other(Box::new(e)))?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn doctest_pool_creates_pool_with_migrations() {
        let pool = doctest_pool().await.expect("should create pool");
        // Verify the pool is functional by running a simple query
        let result: Result<(i64,), _> = sqlx::query_as("SELECT 1").fetch_one(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);
    }
}
