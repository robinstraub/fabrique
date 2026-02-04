use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    #[fabrique(unknown_attribute = true)]
    weight: u32,
}

fn main() {}