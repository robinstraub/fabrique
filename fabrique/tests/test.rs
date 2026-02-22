use fabrique::prelude::*;
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
#[allow(dead_code)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[fabrique::test]
async fn test_persistable_macro_compiles<DB: Dialect>(connection: Pool<DB>) {
    let result = Product::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[fabrique::test]
async fn test_create<DB: Dialect>(connection: Pool<DB>) {
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

#[fabrique::test]
async fn test_delete<DB: Dialect>(connection: Pool<DB>) {
    let product = Product::factory().create(&connection).await.unwrap();
    let existing = Product::all(&connection).await.unwrap();
    assert!(!existing.is_empty());
    let result = product.delete(&connection).await;
    assert!(result.is_ok());
    let existing = Product::all(&connection).await.unwrap();
    assert!(existing.is_empty());
}

#[fabrique::test]
async fn test_destroy<DB: Dialect>(connection: Pool<DB>) {
    let id = Uuid::new_v4();
    Product::factory().id(id).create(&connection).await.unwrap();
    let result = Product::destroy(&connection, id).await;
    assert!(result.is_ok());
    let products = Product::all(&connection).await.unwrap();
    assert_eq!(products.len(), 0);
}

#[fabrique::test]
async fn test_all<DB: Dialect>(connection: Pool<DB>) {
    let product = Product::factory().create(&connection).await.unwrap();

    let result = Product::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![product]);
}

#[fabrique::test]
async fn test_query_builder<DB: Dialect>(connection: Pool<DB>) {
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
