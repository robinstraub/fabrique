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
async fn test_save(connection: Pool<Postgres>) {
    let result = Anvil::default().save(&connection).await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update(connection: Pool<Postgres>) {
    let anvil = Anvil::factory().create(&connection).await.unwrap();
    let result = Anvil::query()
        .update()
        .set(Anvil::MATERIAL, "Steel".to_string())
        .r#where(Anvil::ID, "=", anvil.id)
        .execute(&connection)
        .await;
    assert!(result.is_ok());
}
