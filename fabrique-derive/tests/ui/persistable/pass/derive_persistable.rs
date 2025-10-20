use fabrique_derive::Persistable;
use uuid::Uuid;

#[derive(Persistable)]
struct Anvil {
    id: Uuid,
}

fn main() {}
