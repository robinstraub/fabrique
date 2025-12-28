use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[fabrique(soft_delete)]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_soft_delete(connection: Pool<Postgres>) {
    // Create a new row
    let id = Uuid::new_v4();
    let user = User::factory()
        .id(id)
        .deleted_at(None)
        .create(&connection)
        .await
        .unwrap();
    assert_eq!(User::all(&connection).await.unwrap(), vec![user.clone()]);

    // Soft delete the row
    let result = User::destroy(&connection, user.id).await;
    assert!(result.is_ok());
    assert_eq!(User::all(&connection).await.unwrap(), vec![]);

    // Ensure the row still exists with a deleted_at value
    assert!(user.trashed(&connection).await.unwrap());
    let result: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&connection)
        .await
        .unwrap();
    assert!(result.deleted_at.is_some());

    // Restore the row
    let result = user.restore(&connection).await;
    assert!(result.is_ok());
    assert_eq!(User::all(&connection).await.unwrap(), vec![user]);
}
