use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(relation = "")]
    hammer_id: u32,
    weight: u32,
}

fn main() {}