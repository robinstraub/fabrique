use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(primary_key = "invalid")]
    id: u32,
    weight: u32,
}

fn main() {}