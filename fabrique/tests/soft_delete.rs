use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

// Simple struct to test derive macro compilation
#[derive(Clone, Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct Anvil {
    pub id: Uuid,
    pub material: String,
    pub name: String,
    pub weight: i16,
    #[fabrique(soft_delete)]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_soft_delete(connection: Pool<Postgres>) {
    // Create a new row
    let id = Uuid::new_v4();
    let anvil = Anvil::factory().id(id).create(&connection).await.unwrap();
    assert_eq!(Anvil::all(&connection).await.unwrap(), vec![anvil.clone()]);

    // Soft delete the row
    let result = Anvil::destroy(&connection, anvil.id).await;
    assert!(result.is_ok());
    assert_eq!(Anvil::all(&connection).await.unwrap(), vec![]);

    // Ensure the row still exists with a deleted_at value
    assert!(anvil.trashed(&connection).await.unwrap());
    let result: Anvil = sqlx::query_as("SELECT * FROM anvils WHERE id = $1")
        .bind(id)
        .fetch_one(&connection)
        .await
        .unwrap();
    assert!(result.deleted_at.is_some());

    // Restore the row
    let result = anvil.restore(&connection).await;
    assert!(result.is_ok());
    assert_eq!(Anvil::all(&connection).await.unwrap(), vec![anvil]);
}
