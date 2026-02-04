use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(primery_key)]
    id: u32,
    weight: u32,
}

fn main() {}