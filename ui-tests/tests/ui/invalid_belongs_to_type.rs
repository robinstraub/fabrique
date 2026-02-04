use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(belongs_to = "Not A Valid Type")]
    hammer_id: u32,
    weight: u32,
}

fn main() {}
