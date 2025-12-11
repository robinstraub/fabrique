use fabrique::{Factory, Persistable};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

// Simple struct to test derive macro compilation
#[derive(Debug, Default, Factory, PartialEq, Persistable)]
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
    let id = Uuid::new_v4();
    let anvil = Anvil::factory().id(id).create(&connection).await.unwrap();

    let result = anvil.delete(&connection).await;
    assert!(result.is_ok());

    let rows = Anvil::all(&connection).await.unwrap();
    assert_eq!(rows, vec![]);

    let result: Anvil = sqlx::query_as("SELECT * FROM anvils WHERE id = $1")
        .bind(id)
        .fetch_one(&connection)
        .await
        .unwrap();
    assert!(result.deleted_at.is_some());
}
