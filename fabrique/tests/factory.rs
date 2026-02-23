use fabrique::prelude::*;
use fabrique::sql::QueryBuilder;
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Address {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub label: String,
    pub street: String,
    pub city: String,
    pub zip: String,
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

#[fabrique::test]
async fn test_auto_creates_belongs_to<DB: Dialect>(connection: Pool<DB>) {
    // Order has belongs_to User — create without for_user()
    let order = Order::factory().create(&connection).await.unwrap();

    // A User was auto-created and the FK is valid
    let user = User::find(&connection, order.user_id).await.unwrap();
    assert_eq!(user.id, order.user_id);
}

#[fabrique::test]
async fn test_factory_for_relations_accept_models<DB: Dialect>(connection: Pool<DB>) {
    let user = User::factory().create(&connection).await.unwrap();
    let product = Product::factory().create(&connection).await.unwrap();
    let order = Order::factory()
        .for_user(user)
        .create(&connection)
        .await
        .unwrap();

    OrderLine::factory()
        .for_order(order)
        .for_product(product)
        .create(&connection)
        .await
        .unwrap();
}

#[fabrique::test]
async fn test_factory_for_relations_accept_factories<DB: Dialect>(connection: Pool<DB>) {
    OrderLine::factory()
        .for_order(Order::factory())
        .for_product(Product::factory())
        .create(&connection)
        .await
        .unwrap();
}

#[fabrique::test]
async fn test_has_many_creates_children<DB: Dialect>(connection: Pool<DB>) {
    // Arrange a User with 1 Address via has_addresses (generated from Address's
    // belongs_to)
    let user = User::factory()
        .name("Wile E. Coyote".to_string())
        .has_addresses(Address::factory().label("Desert Cliff #42".to_string()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Assert the address was created for this user
    let count: (i64,) = QueryBuilder::table("addresses")
        .select(&["COUNT(*)"])
        .r#where("user_id", "=", user.id)
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[fabrique::test]
async fn test_has_many_through_join_model<DB: Dialect>(connection: Pool<DB>) {
    // Arrange an Order with 1 OrderLine (which auto-creates a Product via
    // belongs_to)
    let order = Order::factory()
        .has_order_lines(OrderLine::factory(), 1)
        .create(&connection)
        .await
        .unwrap();

    // Assert the order line was created for this order
    let count: (i64,) = QueryBuilder::table("order_lines")
        .select(&["COUNT(*)"])
        .r#where("order_id", "=", order.id)
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);

    // Assert a product was auto-created via OrderLine's belongs_to Product
    let product_count: (i64,) = QueryBuilder::table("products")
        .select(&["COUNT(*)"])
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(product_count.0, 1);
}
