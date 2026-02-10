/// Returns the SQL placeholder for the given 1-based index.
///
/// PostgreSQL uses `$1, $2, ...`, SQLite and MySQL use `?`.
pub fn placeholder(index: usize) -> String {
    #[cfg(feature = "postgres")]
    return format!("${}", index);

    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    {
        let _ = index;
        "?".to_string()
    }
}

/// Returns the SQL expression for the current timestamp.
///
/// PostgreSQL and MySQL use `now()`, SQLite uses `datetime('now')`.
pub fn now() -> &'static str {
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    return "now()";

    #[cfg(feature = "sqlite")]
    return "datetime('now')";
}

pub mod alias;
pub mod belongs_to;
pub mod columns;
pub mod database;
pub mod delete;
#[cfg(feature = "testing")]
pub mod factory;
pub mod find;
pub mod from_row;
pub mod hard_delete;
pub mod joinable;
pub mod model;
pub mod persist;
pub mod query;
pub mod soft_delete;

pub use alias::AliasCodegen;
pub use belongs_to::BelongsToCodegen;
pub use columns::ColumnsCodegen;
pub use database::DatabaseAwareCodegen;
pub use delete::DeleteCodegen;
#[cfg(feature = "testing")]
pub use factory::FactoryCodegen;
pub use find::FindCodegen;
pub use from_row::FromRowCodegen;
pub use hard_delete::HardDeleteCodegen;
pub use joinable::JoinableCodegen;
pub use model::ModelCodegen;
pub use persist::PersistCodegen;
pub use query::QueryCodegen;
pub use soft_delete::SoftDeleteCodegen;
