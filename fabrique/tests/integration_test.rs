use fabrique::{Factory, Persistable};

// Darling ?
#[derive(Debug, Default, Eq, Factory, PartialEq)]
struct Anvil {
    #[fabrique(primary_key)]
    id: u32,

    #[fabrique(relation = "Hammer")]
    hammer_id: u32,
    hardness: u32,
    weight: u32,
}

impl Persistable for Anvil {
    type Connection = ();

    type Error = ();

    async fn create(self, _connection: &Self::Connection) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

#[derive(Debug, Default, Eq, Factory, PartialEq)]
struct Hammer {
    #[fabrique(primary_key)]
    id: u32,
    weight: u32,
}

impl Persistable for Hammer {
    type Connection = ();

    type Error = ();

    async fn create(self, _connection: &Self::Connection) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory() {
        let result = Anvil::factory()
            .for_hammer(|factory| factory.id(100))
            .create(&())
            .await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Anvil {
                hammer_id: 100,
                ..Default::default()
            }
        );
    }
}
