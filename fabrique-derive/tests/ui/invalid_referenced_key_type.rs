use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(relation = "Hammer", referenced_key = 123)]
    hammer_id: u32,
    weight: u32,
}

fn main() {}