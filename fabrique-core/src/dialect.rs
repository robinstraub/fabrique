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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_dialect() {
        assert_eq!(sqlx::MySql::placeholder(1), "?");
        assert_eq!(sqlx::MySql::now(), "now()");
        assert_eq!(sqlx::MySql::on_conflict_sql(&["id", "slug"]), "");
        assert_eq!(
            sqlx::MySql::do_update_sql(&["name", "slug"]),
            " ON DUPLICATE KEY UPDATE name = VALUES(name), slug = VALUES(slug)"
        );
        assert_eq!(
            sqlx::MySql::do_nothing_sql(),
            " ON DUPLICATE KEY UPDATE id = id"
        );
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgres_dialect() {
        assert_eq!(sqlx::Postgres::placeholder(1), "$1");
        assert_eq!(sqlx::Postgres::now(), "now()");
        assert_eq!(
            sqlx::Postgres::on_conflict_sql(&["id", "slug"]),
            " ON CONFLICT (id, slug)"
        );
        assert_eq!(
            sqlx::Postgres::do_update_sql(&["name", "slug"]),
            " DO UPDATE SET name = EXCLUDED.name, slug = EXCLUDED.slug"
        );
        assert_eq!(sqlx::Postgres::do_nothing_sql(), " DO NOTHING");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_sqlite_dialect() {
        assert_eq!(sqlx::Sqlite::placeholder(1), "?");
        assert_eq!(sqlx::Sqlite::now(), "datetime('now')");
        assert_eq!(
            sqlx::Sqlite::on_conflict_sql(&["id", "slug"]),
            " ON CONFLICT (id, slug)"
        );
        assert_eq!(
            sqlx::Sqlite::do_update_sql(&["name", "slug"]),
            " DO UPDATE SET name = EXCLUDED.name, slug = EXCLUDED.slug"
        );
        assert_eq!(sqlx::Sqlite::do_nothing_sql(), " DO NOTHING");
    }
}
