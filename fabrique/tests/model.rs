use fabrique::prelude::*;
use sqlx::Pool;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[fabrique_derive::test]
async fn test_save(connection: Pool<Backend>) {
    let result = Product::default().save(&connection).await;
    assert!(result.is_ok());
}

#[fabrique_derive::test]
async fn test_update(connection: Pool<Backend>) {
    let product = Product::factory().create(&connection).await.unwrap();
    let result = Product::update()
        .set(Product::NAME, "Anvil 3000")
        .r#where(Product::ID, "=", product.id)
        .execute(&connection)
        .await;
    assert!(result.is_ok());
}
