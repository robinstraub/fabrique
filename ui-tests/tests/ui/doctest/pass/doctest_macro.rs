extern crate tokio;

use fabrique::prelude::*;

#[derive(Debug, Default, Model, Factory)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
}

#[fabrique::doctest]
async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
    let _user = User::factory().create(&pool).await?;
    Ok(())
}
