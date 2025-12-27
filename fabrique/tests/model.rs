use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_save(connection: Pool<Postgres>) {
    let result = Product::default().save(&connection).await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update(connection: Pool<Postgres>) {
    let product = Product::factory().create(&connection).await.unwrap();
    let result = Product::update()
        .set(Product::NAME, "Anvil 3000".to_string())
        .r#where(Product::ID, "=", product.id)
        .execute(&connection)
        .await;
    assert!(result.is_ok());
}
