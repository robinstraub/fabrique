use fabrique_derive::Persistable;
use uuid::Uuid;

#[derive(Persistable)]
pub struct Anvil {
    pub id: Uuid,
}

fn main() {}
