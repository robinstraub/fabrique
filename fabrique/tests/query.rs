use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub orders: HasMany<Order>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub status: String,
    pub order_lines: HasMany<OrderLine>,
    #[fabrique(through = "OrderLine")]
    pub products: HasMany<Product>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    pub order_id: Uuid,
    #[fabrique(primary_key, belongs_to = "Product")]
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price_cents: i32,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_join_many_to_many_lazy_loading(connection: Pool<Postgres>) {
    let order = Order::factory()
        .for_user(User::factory())
        .has_products(Product::factory(), 1)
        .create(&connection)
        .await
        .unwrap();

    let result = order.products().get(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
