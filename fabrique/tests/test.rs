use fabrique::{Factory, Persistable, QueryBuilder};
use sqlx::{FromRow, Pool, Postgres, Type};
use uuid::Uuid;

#[derive(Debug, Default, PartialEq, Type)]
#[sqlx(type_name = "TEXT")]
pub enum Material {
    #[default]
    Iron,
}

// Simple struct to test derive macro compilation
#[derive(Debug, Factory, FromRow, PartialEq, Persistable)]
#[allow(dead_code)]
pub struct Anvil {
    pub id: Uuid,
    pub material: Material,
    pub name: String,
    pub weight: i16,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_persistable_macro_compiles(connection: Pool<Postgres>) {
    let result = Anvil::all(&connection).await;
    println!("result: {:?}", &result);
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
async fn test_query_builder(connection: Pool<Postgres>) {
    Anvil::factory()
        .name("bipbip obliterator".to_owned())
        .weight(i16::MAX)
        .create(&connection)
        .await
        .unwrap();

    let result = Anvil::query()
        .r#where(Anvil::WEIGHT, ">=", 42)
        .fetch_all(connection)
        .await;

    println!("result: {:#?}", &result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
