use fabrique::{Factory, Persistable};

// Darling ?
#[derive(Debug, Default, Eq, Factory, PartialEq)]
struct Anvil {
    #[fabrique(primary_key)]
    id: u32,

    #[fabrique(relation = "Hammer", referenced_key = "id")]
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

    async fn all(_connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
        Ok(vec![])
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

    async fn all(_connection: &Self::Connection) -> Result<Vec<Self>, Self::Error> {
        Ok(vec![])
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

    #[tokio::test]
    async fn test_factory_calls_all_method() {
        // Act - call the all method
        let result = Anvil::all(&()).await;

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn test_hammer_factory_with_multiple_fields() {
        // Arrange - create a hammer with specific values
        let result = Hammer::factory().id(42).weight(500).create(&()).await;

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Hammer {
                id: 42,
                weight: 500,
            }
        );
    }
}
