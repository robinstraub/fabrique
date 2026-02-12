use fabrique::prelude::*;
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
async fn test_save<DB: Dialect>(connection: Pool<DB>) {
    let result = Product::default().save(&connection).await;
    assert!(result.is_ok());
}

#[fabrique_derive::test]
async fn test_update<DB: Dialect>(connection: Pool<DB>) {
    let product = Product::factory().create(&connection).await.unwrap();
    let result = Product::update()
        .set(Product::NAME, "Anvil 3000")
        .r#where(Product::ID, "=", product.id)
        .execute(&connection)
        .await;
    assert!(result.is_ok());
}
