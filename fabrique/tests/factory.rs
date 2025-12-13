use fabrique::{Factory, Persistable};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Persistable)]
pub struct Anvil {
    pub id: Uuid,
    pub material: String,
    pub name: String,
    pub weight: i16,
}

#[derive(Debug, Default, Factory, PartialEq, Persistable)]
pub struct Order {
    pub id: Uuid,
}

#[derive(Debug, Default, Factory, PartialEq, Persistable)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, relation = "Order")]
    pub order_id: Uuid,

    #[fabrique(primary_key, relation = "Anvil")]
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
