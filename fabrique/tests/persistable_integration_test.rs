// Integration test to ensure Persistable derive macro is invoked and covered by tests.
// This provides code coverage for the derive_persistable proc macro entry point in lib.rs.

#[cfg(test)]
mod tests {
    use fabrique::Persistable;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    // Simple struct to test derive macro compilation
    // Note: We use SQLX_OFFLINE=true mode to avoid needing a live database
    #[derive(Debug, Persistable)]
    struct Anvil {
        #[allow(dead_code)]
        id: Uuid,
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_persistable_macro_compiles(connection: Pool<Postgres>) {
        let result = <Anvil as Persistable>::all(&connection).await;
        println!("result: {:?}", &result);
        assert!(result.is_ok());
    }
}
