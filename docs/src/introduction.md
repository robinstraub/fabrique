# Introduction

Fabrique is a lightweight ORM for Rust that combines ease of use with Rust's safety guarantees.

## Features

- **Fluent API** — Expressive, chainable query building
- **Derive-based** — Models and factories generated from struct definitions
- **Factory support** — Test data generation with relationship handling

## Quick Example

```rust,no_run
# use fabrique::prelude::*;
# use uuid::Uuid;
# use sqlx::{Pool, Postgres};
#
# #[derive(Model, Factory)]
# #[fabrique(table = "anvils")]
# pub struct Anvil {
#     #[fabrique(primary_key)]
#     pub id: Uuid,
#     pub name: String,
#     pub weight: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Query
let anvils = Anvil::query()
    .select()
    .r#where(Anvil::WEIGHT, ">=", 50)
    .get(&pool)
    .await?;

// Create test data
let anvil = Anvil::factory()
    .name("Heavy Duty".to_string())
    .create(&pool)
    .await?;
# Ok(())
# }
```

## Documentation Structure

This documentation follows the [Diátaxis](https://diataxis.fr/) framework:

- **[Tutorials](tutorials/getting-started.md)** — Learn by building a complete example
- **[Concepts](concepts/models.md)** — Understand how Fabrique works
- **[Guides](guides/persisting-data.md)** — Solve specific problems
- **[API Reference](https://docs.rs/fabrique)** — Technical details on docs.rs
