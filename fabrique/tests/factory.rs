use fabrique::prelude::*;
use fabrique::sql::QueryBuilder;
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub addresses: HasMany<Address>,
    pub orders: HasMany<Order>,
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

#[fabrique_derive::test]
async fn test_factory_for_relations_accept_models(connection: Pool<Backend>) {
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

#[fabrique_derive::test]
async fn test_factory_for_relations_accept_factories(connection: Pool<Backend>) {
    OrderLine::factory()
        .for_order(Order::factory().for_user(User::factory()))
        .for_product(Product::factory())
        .create(&connection)
        .await
        .unwrap();
}

#[fabrique_derive::test]
async fn test_has_many_creates_children(connection: Pool<Backend>) {
    // Create a User with 1 Address
    let user = User::factory()
        .name("Wile E. Coyote".to_string())
        .has_addresses(Address::factory().label("Desert Cliff #42".to_string()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Verify the address was created for this user
    let count: (i64,) = QueryBuilder::table("addresses")
        .select(&["COUNT(*)"])
        .r#where("user_id", "=", user.id)
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[fabrique_derive::test]
async fn test_many_to_many_through_creates_join_records(connection: Pool<Backend>) {
    // Create an Order with 1 Product through OrderLine (many-to-many)
    let order = Order::factory()
        .for_user(User::factory())
        .has_products(Product::factory().name("Anvil 3000".to_string()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Verify the order line was created linking order to product
    let count: (i64,) = QueryBuilder::table("order_lines")
        .select(&["COUNT(*)"])
        .r#where("order_id", "=", order.id)
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);

    // Verify product was also created
    let product_count: (i64,) = QueryBuilder::table("products")
        .select(&["COUNT(*)"])
        .first_or_fail(&connection)
        .await
        .unwrap();

    assert_eq!(product_count.0, 1);
}
