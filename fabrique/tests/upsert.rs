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

#[fabrique::test]
async fn test_upsert_inserts_new_records<DB: Dialect>(pool: Pool<DB>) {
    let products = vec![
        Product::factory::<DB>()
            .name("Anvil 3000".to_owned())
            .make(),
        Product::factory::<DB>()
            .name("Rocket Skates".to_owned())
            .make(),
    ];

    products.upsert(&pool, Product::ID).await.unwrap();

    let all = Product::all(&pool).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[fabrique::test]
async fn test_upsert_updates_existing_records<DB: Dialect>(pool: Pool<DB>) {
    let product = Product::factory()
        .name("Anvil 3000".to_owned())
        .price_cents(9999)
        .create(&pool)
        .await
        .unwrap();

    let updated = vec![Product {
        price_cents: 4999,
        ..product
    }];

    updated.upsert(&pool, Product::ID).await.unwrap();

    let all = Product::all(&pool).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].price_cents, 4999);
}

// SQLite requires a UNIQUE index on the conflict target columns;
// the test schema only has a PK on `id`, not a composite unique
// on (name, price_cents).
#[cfg(not(feature = "sqlite"))]
#[fabrique::test]
async fn test_upsert_with_composite_unique_by<DB: Dialect>(pool: Pool<DB>) {
    let products = vec![
        Product::factory::<DB>()
            .name("Anvil 3000".to_owned())
            .price_cents(9999)
            .make(),
        Product::factory::<DB>()
            .name("Anvil 3000".to_owned())
            .price_cents(4999)
            .make(),
    ];

    products
        .upsert(&pool, (Product::NAME, Product::PRICE_CENTS))
        .await
        .unwrap();

    let all = Product::all(&pool).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[fabrique::test]
async fn test_upsert_empty_vec_is_noop<DB: Dialect>(pool: Pool<DB>) {
    let products: Vec<Product> = vec![];

    products.upsert(&pool, Product::ID).await.unwrap();

    let all = Product::all(&pool).await.unwrap();
    assert_eq!(all.len(), 0);
}

#[fabrique::test]
async fn test_upsert_with_tuples<DB: Dialect>(pool: Pool<DB>) {
    let products = vec![
        Product::factory::<DB>()
            .name("Anvil 3000".to_owned())
            .make(),
        Product::factory::<DB>()
            .name("Rocket Skates".to_owned())
            .make(),
    ];

    let result = products.clone().upsert(&pool, (Product::ID,)).await;
    assert!(result.is_ok());

    let result = products
        .clone()
        .upsert(&pool, (Product::ID, Product::NAME))
        .await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "database error: error returned from database: (code: 1) ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE constraint"
    );
}
