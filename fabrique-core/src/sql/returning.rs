/// Marker trait for database backends that support the `RETURNING` clause.
///
/// Implemented for PostgreSQL and SQLite. MySQL does not support `RETURNING`,
/// so methods gated by this trait will not be available for MySQL.
pub trait SupportsReturning: sqlx::Database {}

#[cfg(feature = "postgres")]
impl SupportsReturning for sqlx::Postgres {}

#[cfg(feature = "sqlite")]
impl SupportsReturning for sqlx::Sqlite {}
