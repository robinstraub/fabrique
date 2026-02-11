use fabrique::prelude::*;
use fake::Dummy;
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    #[fabrique(as = "String")]
    pub status: Status,
}

#[derive(Clone, Debug, Default, Dummy, PartialEq)]
pub enum Status {
    #[default]
    Pending,
    Paid,
    Cancelled,
}

impl From<Status> for String {
    fn from(value: Status) -> Self {
        match value {
            Status::Cancelled => "cancelled".to_owned(),
            Status::Paid => "paid".to_owned(),
            Status::Pending => "pending".to_owned(),
        }
    }
}

impl TryFrom<String> for Status {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "cancelled" => Ok(Status::Cancelled),
            "paid" => Ok(Status::Paid),
            "pending" => Ok(Status::Pending),
            other => Err(format!("unknown status: {other}")),
        }
    }
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[fabrique::test]
async fn test_where_accepts_a_rust_type(pool: Pool<Backend>) {
    // Arrange a fixture
    Order::factory()
        .status(Status::Pending)
        .create(&pool)
        .await
        .unwrap();

    // Act the call to the query
    let result = Order::query()
        .r#where(Order::STATUS, "=", Status::Pending)
        .get(&pool)
        .await;

    // Assert the result
    assert!(result.is_ok());
    assert_eq!(result.iter().len(), 1);
}

#[fabrique::test]
async fn test_set_accepts_a_rust_type(pool: Pool<Backend>) {
    // Arrange a fixture
    let order = Order::factory().create(&pool).await.unwrap();

    // Act the call to the query
    Order::update()
        .set(Order::STATUS, Status::Paid)
        .r#where(Order::ID, "=", order.id)
        .execute(&pool)
        .await
        .unwrap();

    // Assert the result
    let updated = Order::query()
        .r#where(Order::ID, "=", order.id)
        .first_or_fail(&pool)
        .await
        .unwrap();
    assert_eq!(updated.status, Status::Paid);
}
