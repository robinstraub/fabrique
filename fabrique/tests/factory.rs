use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Anvil {
    pub id: Uuid,
    pub material: String,
    pub name: String,
    pub weight: i16,
    pub order_lines: HasMany<OrderLine>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Order {
    pub id: Uuid,
    pub order_lines: HasMany<OrderLine>,
    #[fabrique(through = "OrderLine")]
    pub anvils: HasMany<Anvil>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    pub order_id: Uuid,

    #[fabrique(primary_key, belongs_to = "Anvil")]
    pub anvil_id: Uuid,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_factory_for_relations_accept_models(connection: Pool<Postgres>) {
    let anvil = Anvil::factory().create(&connection).await.unwrap();
    let order = Order::factory().create(&connection).await.unwrap();

    OrderLine::factory()
        .for_anvil(anvil)
        .for_order(order)
        .create(&connection)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn test_factory_for_relations_accept_factories(connection: Pool<Postgres>) {
    OrderLine::factory()
        .for_anvil(Anvil::factory())
        .for_order(Order::factory())
        .create(&connection)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn test_has_many_creates_children(connection: Pool<Postgres>) {
    // Create an Order with 1 OrderLine (which also creates an Anvil)
    let order = Order::factory()
        .has_order_lines(OrderLine::factory().for_anvil(Anvil::factory()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Verify the order line was created for this order
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM order_lines WHERE order_id = $1")
        .bind(order.id)
        .fetch_one(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_many_to_many_through_creates_join_records(connection: Pool<Postgres>) {
    // Create an Order with 1 Anvil through OrderLine (many-to-many)
    let order = Order::factory()
        .has_anvils(Anvil::factory(), 1)
        .create(&connection)
        .await
        .unwrap();

    // Verify the order line was created linking order to anvil
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM order_lines WHERE order_id = $1")
        .bind(order.id)
        .fetch_one(&connection)
        .await
        .unwrap();

    assert_eq!(count.0, 1);

    // Verify an anvil was also created
    let anvil_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM anvils")
        .fetch_one(&connection)
        .await
        .unwrap();

    assert_eq!(anvil_count.0, 1);
}
