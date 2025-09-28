use fabrique_derive::Factory;

#[derive(Factory)]
struct Anvil {
    _hardness: u32,
    _weight: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory() {
        let factory = Anvil::factory();
        assert!(factory._hardness.is_none());
        assert!(factory._weight.is_none());
    }
}
