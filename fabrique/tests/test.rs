use fabrique::prelude::*;
use sqlx::Pool;
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[fabrique_derive::test]
async fn test_persistable_macro_compiles(connection: Pool<Backend>) {
    let result = Product::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[fabrique_derive::test]
async fn test_create(connection: Pool<Backend>) {
    let result = Product::factory()
        .name("Anvil 3000".to_owned())
        .price_cents(9999)
        .in_stock(true)
        .create(&connection)
        .await;

    assert!(result.is_ok());
    let product = result.unwrap();
    assert_eq!(product.name, "Anvil 3000");
    assert_eq!(product.price_cents, 9999);
    assert!(product.in_stock);
}

#[fabrique_derive::test]
async fn test_delete(connection: Pool<Backend>) {
    let product = Product::factory().create(&connection).await.unwrap();
    let existing = Product::all(&connection).await.unwrap();
    assert!(!existing.is_empty());
    let result = product.delete(&connection).await;
    assert!(result.is_ok());
    let existing = Product::all(&connection).await.unwrap();
    assert!(existing.is_empty());
}

#[fabrique_derive::test]
async fn test_destroy(connection: Pool<Backend>) {
    let id = Uuid::new_v4();
    Product::factory().id(id).create(&connection).await.unwrap();
    let result = Product::destroy(&connection, id).await;
    assert!(result.is_ok());
    let products = Product::all(&connection).await.unwrap();
    assert_eq!(products.len(), 0);
}

#[fabrique_derive::test]
async fn test_all(connection: Pool<Backend>) {
    let product = Product::factory().create(&connection).await.unwrap();

    let result = Product::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![product]);
}

#[fabrique_derive::test]
async fn test_query_builder(connection: Pool<Backend>) {
    Product::factory()
        .name("Anvil 3000".to_owned())
        .price_cents(9999)
        .create(&connection)
        .await
        .unwrap();

    let result = Product::query()
        .r#where(Product::PRICE_CENTS, ">=", 1000)
        .get(&connection)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
