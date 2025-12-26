# Queries

Once you have created a model and its corresponding database table, you are ready to start retrieving data from your database. You can think of each Fabrique model as a powerful query builder allowing you to fluently query the database table associated with the model.

## Retrieving All Records

The model's `all` method retrieves all of the records from the model's associated database table:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvils: Vec<Anvil> = Anvil::all(&pool).await?;
# Ok(())
# }
# fn main() {}
```

## Building Queries

The `all` method returns all results in the model's table. Since each Fabrique model serves as a query builder, you may add additional constraints to queries and then invoke the `get` method to retrieve the results:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil {
#     id: Uuid,
#     weight: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvils: Vec<Anvil> = Anvil::query()
    .select()
    .r#where(Anvil::WEIGHT, ">=", 42)
    .get(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Retrieving Results

Fabrique provides several methods to execute a query and retrieve results:

### `get` — All Matching Records

Returns all records matching the query as a `Vec<T>`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil { id: Uuid, weight: i32 }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvils: Vec<Anvil> = Anvil::query()
    .select()
    .r#where(Anvil::WEIGHT, ">", 50)
    .get(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

### `first` — First or None

Returns the first matching record as `Option<T>`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil { id: Uuid, weight: i32 }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvil: Option<Anvil> = Anvil::query()
    .select()
    .r#where(Anvil::WEIGHT, ">", 100)
    .first(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

### `first_or_fail` — First or Error

Returns the first matching record, or an error if none found:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil { id: Uuid, weight: i32 }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let anvil: Anvil = Anvil::query()
    .select()
    .r#where(Anvil::WEIGHT, ">", 100)
    .first_or_fail(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Column Constants

When you derive the `Model` macro, Fabrique generates column constants for each field. These constants are used in query methods to provide type-safe column references:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Anvil {
    id: Uuid,
    weight: i32,
    name: String,
}

// Generated constants:
// Anvil::ID
// Anvil::WEIGHT
// Anvil::NAME
# fn main() {}
```

## Type-Safe Columns

Column constants are not just names — they carry type information. When using `r#where`, the value must match the column's type:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil { id: Uuid, weight: i32 }
#
# fn example() {
# let _ = Anvil::query().select()
// ✓ Compiles: WEIGHT is i32, 42 is i32
.r#where(Anvil::WEIGHT, ">", 42);
# }
# fn main() {}
```

```rust,compile_fail
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Anvil { id: Uuid, weight: i32 }
#
# fn example() {
# let _ = Anvil::query().select()
// ✗ Won't compile: WEIGHT is i32, "heavy" is &str
.r#where(Anvil::WEIGHT, ">", "heavy");
# }
# fn main() {}
```
