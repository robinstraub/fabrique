use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(relation = "Not A Valid Type", referenced_key = "id")]
    hammer_id: u32,
    weight: u32,
}

fn main() {}