use std::error::Error;

use sqlx::{FromRow, Pool, Postgres, Type};
use uuid::Uuid;

#[derive(Debug, Type)]
#[sqlx(type_name = "TEXT")]
pub enum Material {
    Iron,
}

#[derive(Debug, FromRow)]
pub struct Anvil {
    pub id: Uuid,
    pub material: Material,
    pub name: String,
    pub weight: i16,
}

impl From<Material> for String {
    fn from(value: Material) -> Self {
        match value {
            Material::Iron => "Iron".to_owned(),
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn test_persistable_macro_compiles(connection: Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    sqlx::query_as!(
        Anvil,
        r#"INSERT INTO anvils (material, name, weight) VALUES ($1, $2, $3) RETURNING id as "id: Uuid", material as "material: Material", name as "name: String", weight as "weight: i16""#,
        String::from(Material::Iron),
        "anvil01".to_owned(),
        42
    ).fetch_one(&connection).await?;

    sqlx::query_as!(
        Anvil,
        r#"SELECT id as "id: Uuid", material as "material: _", name as "name: String", weight as "weight: i16" FROM anvils"#
    )
    .fetch_all(&connection)
    .await?;
    Ok(())
}
