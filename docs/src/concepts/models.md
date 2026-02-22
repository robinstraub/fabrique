# Models

A model is a Rust struct mapped to a database table. Deriving `Model` on a
struct generates the table mapping, column constants, and convenience methods
for querying and persisting data.

Fabrique favors convention over configuration to minimize boilerplate. Table
names and primary keys are inferred from your struct definition. When your
schema doesn't follow these conventions, attributes let you opt in explicitly.

## Defining a Model

Create a struct and derive `Model`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    id: Uuid,
    name: String,
    price_cents: i32,
}
# fn main() {}
```

## Table Names

By convention, the snake_case plural name of the struct is used as the table
name. `Product` maps to `products`, `RocketShoe` maps to `rocket_shoes`.

To override the convention, use the `table` attribute:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
#[fabrique(table = "acme_products")]
pub struct Product {
    id: Uuid,
    name: String,
}
# fn main() {}
```

## Primary Keys

By convention, Fabrique uses a field named `id` as the primary key. To use a
different field, annotate it with `#[fabrique(primary_key)]`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    #[fabrique(primary_key)]
    product_id: Uuid,
    name: String,
}
# fn main() {}
```

If neither an `id` field nor a `#[fabrique(primary_key)]` annotation exists,
compilation fails:

```text
error: Missing primary key, either add an id column or mark an existing
       column as primary
 --> src/product.rs:4:8
  |
4 | struct Product {
  |        ^^^^^^^
```

Composite primary keys are supported by annotating multiple fields:

```rust
# extern crate fabrique;
# extern crate sqlx;
# use fabrique::prelude::*;
#[derive(Model)]
pub struct OrderLine {
    #[fabrique(primary_key)]
    order_id: i32,

    #[fabrique(primary_key)]
    product_id: i32,

    quantity: i32,
}
# fn main() {}
```

## Column Constants

Deriving `Model` generates a constant for each field:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    id: Uuid,
    name: String,
    price_cents: i32,
}

// Generated constants:
// Product::ID
// Product::NAME
// Product::PRICE_CENTS
# fn main() {
# let _ = Product::ID;
# let _ = Product::NAME;
# let _ = Product::PRICE_CENTS;
# }
```

Each constant represents a specific column and its associated type at compile
time, with no runtime cost. This is what enables type-safe queries — the
compiler knows exactly which column `Product::PRICE_CENTS` refers to, and will
reject any operation with an incompatible type. See the
[Query Builder](query-builder.md) for details.

## Type Conversions

Common Rust types (`String`, `i32`, `bool`, `Uuid`, ...) map directly to
SQL columns — see [Database](database.md) for the full list. For custom
types, the `#[fabrique(as = "Type")]` attribute tells Fabrique to convert
the field through a Rust type that maps to SQL:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug)]
# pub enum Material { Wood, Metal, Rubber }
# impl From<Material> for String {
#     fn from(m: Material) -> String {
#         match m {
#             Material::Wood => "wood",
#             Material::Metal => "metal",
#             Material::Rubber => "rubber",
#         }.to_string()
#     }
# }
# impl TryFrom<String> for Material {
#     type Error = String;
#     fn try_from(s: String) -> Result<Self, Self::Error> {
#         match s.as_str() {
#             "wood" => Ok(Material::Wood),
#             "metal" => Ok(Material::Metal),
#             "rubber" => Ok(Material::Rubber),
#             _ => Err(format!("unknown material: {}", s)),
#         }
#     }
# }
#[derive(Model)]
pub struct Product {
    id: Uuid,
    name: String,

    #[fabrique(as = "String")]
    material: Material, // stored as TEXT
}
# fn main() {}
```

Fabrique uses `TryFrom` in both directions. Implementing `From`
satisfies this and is the common choice for writing:

```rust
# #[derive(Clone, Debug)]
# pub enum Material { Wood, Metal, Rubber }
#
// Writing: Material → String (infallible)
impl From<Material> for String {
    // --snip--
#     fn from(m: Material) -> String {
#         match m {
#             Material::Wood => "wood".to_string(),
#             Material::Metal => "metal".to_string(),
#             Material::Rubber => "rubber".to_string(),
#         }
#     }
}

// Reading: String → Material (fallible)
impl TryFrom<String> for Material {
    // --snip--
#     type Error = String;
#     fn try_from(s: String) -> Result<Self, Self::Error> {
#         match s.as_str() {
#             "wood" => Ok(Material::Wood),
#             "metal" => Ok(Material::Metal),
#             "rubber" => Ok(Material::Rubber),
#             other => Err(format!("unknown material: {other}")),
#         }
#     }
}
# fn main() {}
```

If conversion fails when reading, Fabrique returns
`Error::Conversion`. See [Database](database.md) for details.

## Convenience Methods

Deriving `Model` also generates convenience methods that wrap the
query builder for common operations. For instance:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# extern crate tokio;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
// Insert a new record
let anvil = Product {
    id: Uuid::new_v4(),
    name: "Anvil 3000".to_string(),
    price_cents: 4999,
};
let anvil: Product = anvil.create(&pool).await?;

// Fetch by primary key
let anvil: Product = Product::find(&pool, anvil.id).await?;

// Upsert (insert or update on PK conflict)
let anvil: Product = anvil.save(&pool).await?;

// Delete by primary key, no instance needed
Product::destroy(&pool, anvil.id).await?;
# Ok(())
# }
```

See the [API reference](https://docs.rs/fabrique) for the full list.
For more complex queries, or if you prefer an explicit approach, use
the [Query Builder](query-builder.md) directly.
