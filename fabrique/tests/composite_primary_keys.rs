use fabrique::prelude::*;
use sqlx::Pool;
use uuid::Uuid;

#[derive(Debug, Factory, Model)]
pub struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Debug, Factory, Model)]
pub struct Product {
    id: Uuid,
    name: String,
    price_cents: i32,
    in_stock: bool,
}

#[derive(Debug, Factory, Model)]
pub struct Order {
    id: Uuid,
    #[fabrique(belongs_to = "User")]
    user_id: Uuid,
    status: String,
}

#[derive(Debug, Factory, PartialEq, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    order_id: Uuid,

    #[fabrique(primary_key, belongs_to = "Product")]
    product_id: Uuid,

    quantity: i32,
    unit_price_cents: i32,
}

#[fabrique_derive::test]
async fn test_composite_primary_key(connection: Pool<Backend>) {
    let user = User::factory()
        .id(Uuid::new_v4())
        .create(&connection)
        .await
        .unwrap();

    let product = Product::factory()
        .id(Uuid::new_v4())
        .create(&connection)
        .await
        .unwrap();

    let order = Order::factory()
        .id(Uuid::new_v4())
        .user_id(user.id)
        .create(&connection)
        .await
        .unwrap();

    let order_line = OrderLine::factory()
        .order_id(order.id)
        .product_id(product.id)
        .create(&connection)
        .await
        .unwrap();

    assert_eq!(OrderLine::all(&connection).await.unwrap(), vec![order_line]);

    let result = OrderLine::destroy(&connection, (order.id, product.id)).await;
    assert!(result.is_ok());
    assert_eq!(OrderLine::all(&connection).await.unwrap(), vec![]);
}
