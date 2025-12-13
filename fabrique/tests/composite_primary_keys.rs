use fabrique::{Factory, Persistable};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Factory, Persistable)]
pub struct Anvil {
    id: Uuid,
    material: String,
    name: String,
    weight: i16,
}

#[derive(Debug, Factory, Persistable)]
pub struct Order {
    id: Uuid,
}

#[derive(Debug, Factory, PartialEq, Persistable)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key)]
    anvil_id: Uuid,

    #[fabrique(primary_key)]
    order_id: Uuid,

    amount: i16,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_soft_delete(connection: Pool<Postgres>) {
    let anvil = Anvil::factory()
        .id(Uuid::new_v4())
        .create(&connection)
        .await
        .unwrap();

    let order = Order::factory()
        .id(Uuid::new_v4())
        .create(&connection)
        .await
        .unwrap();

    let order_line = OrderLine::factory()
        .anvil_id(anvil.id)
        .order_id(order.id)
        .create(&connection)
        .await
        .unwrap();

    assert_eq!(OrderLine::all(&connection).await.unwrap(), vec![order_line]);

    let result = OrderLine::destroy(&connection, (anvil.id, order.id)).await;
    assert!(result.is_ok());
    assert_eq!(OrderLine::all(&connection).await.unwrap(), vec![]);
}
