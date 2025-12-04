use fabrique_derive::Persistable;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Persistable, FromRow)]
pub struct Anvil {
    pub id: Uuid,
}

fn main() {}
