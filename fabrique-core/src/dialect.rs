/// SQL dialect abstraction for database backends.
///
/// This trait encodes the SQL syntax differences between database backends,
/// allowing Fabrique to generate correct SQL at runtime without relying on
/// mutually exclusive feature flags.
///
/// Each supported database implements this trait with its specific SQL syntax
/// for placeholders, timestamps, and upsert operations.
pub trait Dialect: sqlx::Database {
    /// Returns the SQL placeholder for a 1-based parameter index.
    fn placeholder(index: usize) -> String;

    /// Returns the SQL expression for the current timestamp.
    fn now() -> &'static str;

    /// Returns the SQL clause for declaring a conflict target in an upsert.
    fn on_conflict_sql(columns: &[&str]) -> String;

    /// Returns the SQL clause for updating conflicting rows.
    fn do_update_sql(columns: &[&str]) -> String;

    /// Returns the SQL clause for ignoring conflicting rows.
    fn do_nothing_sql() -> String;
}

#[cfg(feature = "postgres")]
impl Dialect for sqlx::Postgres {
    fn placeholder(index: usize) -> String {
        format!("${}", index)
    }

    fn now() -> &'static str {
        "now()"
    }

    fn on_conflict_sql(columns: &[&str]) -> String {
        format!(" ON CONFLICT ({})", columns.join(", "))
    }

    fn do_update_sql(columns: &[&str]) -> String {
        let set_clause = columns
            .iter()
            .map(|col| format!("{col} = EXCLUDED.{col}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" DO UPDATE SET {set_clause}")
    }

    fn do_nothing_sql() -> String {
        " DO NOTHING".to_string()
    }
}

#[cfg(feature = "sqlite")]
impl Dialect for sqlx::Sqlite {
    fn placeholder(_index: usize) -> String {
        "?".to_string()
    }

    fn now() -> &'static str {
        "datetime('now')"
    }

    fn on_conflict_sql(columns: &[&str]) -> String {
        format!(" ON CONFLICT ({})", columns.join(", "))
    }

    fn do_update_sql(columns: &[&str]) -> String {
        let set_clause = columns
            .iter()
            .map(|col| format!("{col} = EXCLUDED.{col}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" DO UPDATE SET {set_clause}")
    }

    fn do_nothing_sql() -> String {
        " DO NOTHING".to_string()
    }
}

#[cfg(feature = "mysql")]
impl Dialect for sqlx::MySql {
    fn placeholder(_index: usize) -> String {
        "?".to_string()
    }

    fn now() -> &'static str {
        "now()"
    }

    fn on_conflict_sql(_columns: &[&str]) -> String {
        String::new()
    }

    fn do_update_sql(columns: &[&str]) -> String {
        let set_clause = columns
            .iter()
            .map(|col| format!("{col} = VALUES({col})"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" ON DUPLICATE KEY UPDATE {set_clause}")
    }

    fn do_nothing_sql() -> String {
        " ON DUPLICATE KEY UPDATE id = id".to_string()
    }
}
