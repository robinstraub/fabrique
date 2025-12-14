use fabrique_derive::Model;
use uuid::Uuid;

#[derive(Model)]
pub struct Anvil {
    pub id: Uuid,
}

fn main() {}
