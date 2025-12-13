use fabrique::{Factory, Persistable, QueryBuilder};
use sqlx::{Pool, Postgres};
use std::str::FromStr;
use strum_macros::{Display, EnumString, IntoStaticStr};
use uuid::Uuid;

#[derive(Debug, Default, Display, EnumString, IntoStaticStr, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Material {
    #[default]
    Iron,
}

impl From<Material> for String {
    fn from(value: Material) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Material {
    type Error = strum::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Material::from_str(&value)
    }
}

// Simple struct to test derive macro compilation
#[derive(Debug, Default, Factory, PartialEq, Persistable)]
#[allow(dead_code)]
pub struct Anvil {
    pub id: Uuid,
    #[fabrique(as = "String")]
    pub material: Material,
    pub name: String,
    pub weight: i16,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_persistable_macro_compiles(connection: Pool<Postgres>) {
    let result = Anvil::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_create(connection: Pool<Postgres>) {
    let result = Anvil::factory()
        .name("bipbip obliterator".to_owned())
        .weight(i16::MAX)
        .create(&connection)
        .await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Anvil {
            id: Uuid::default(),
            material: Material::Iron,
            name: "bipbip obliterator".to_owned(),
            weight: i16::MAX,
        }
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn test_delete(connection: Pool<Postgres>) {
    let anvil = Anvil::factory().create(&connection).await.unwrap();
    let existing = Anvil::all(&connection).await.unwrap();
    assert!(!existing.is_empty());
    let result = anvil.delete(&connection).await;
    assert!(result.is_ok());
    let existing = Anvil::all(&connection).await.unwrap();
    assert!(existing.is_empty());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_destroy(connection: Pool<Postgres>) {
    let id = Uuid::new_v4();
    Anvil::factory().id(id).create(&connection).await.unwrap();
    let result = Anvil::destroy(&connection, id).await;
    assert!(result.is_ok());
    let anvils = Anvil::all(&connection).await.unwrap();
    assert_eq!(anvils.len(), 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_all(connection: Pool<Postgres>) {
    Anvil::factory().create(&connection).await.unwrap();

    let result = Anvil::all(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![Anvil::default()]);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_query_builder(connection: Pool<Postgres>) {
    Anvil::factory()
        .name("bipbip obliterator".to_owned())
        .weight(i16::MAX)
        .create(&connection)
        .await
        .unwrap();

    let result = Anvil::query()
        .r#where(Anvil::WEIGHT, ">=", 42)
        .fetch_all(&connection)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
