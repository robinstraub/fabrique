use fabrique_derive::Factory;

#[derive(Factory)]
union Anvil {
    heavy: u32,
    light: u16,
}

fn main() {}