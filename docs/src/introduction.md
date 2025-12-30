# Introduction

Fabrique is a lightweight ORM for Rust that combines ease of use with Rust's
safety guarantees.

## Features

- **Fluent API** — Expressive, chainable query building
- **Derive-based** — Models and factories generated from struct definitions
- **Factory support** — Test data generation with relationship handling

## Quick Example

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
use fabrique::Model;
use uuid::Uuid;

#[derive(Default, Model)]
pub struct Product {
    pub id: Uuid,
}

# fn main() {
let product = Product::default();
assert_eq!(product.id, Uuid::default());
# }
```

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# use sqlx::{Pool, Postgres};
#
# #[derive(Model, Factory)]
# pub struct Product {
#     #[fabrique(primary_key)]
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Query
let products = Product::query()
    .select()
    .r#where(Product::PRICE_CENTS, ">=", 1000)
    .get(&pool)
    .await?;

// Create test data
let product = Product::factory()
    .name("Anvil 3000".to_string())
    .create(&pool)
    .await?;
# Ok(())
# }
```

## Documentation Structure

This documentation follows the [Diátaxis](https://diataxis.fr/) framework:

- **[Tutorials](./tutorials/getting-started.md)** — Learn by building a
  complete example
- **[Concepts](./concepts/models.md)** — Understand how Fabrique works
- **[Guides](./guides/persisting-data.md)** — Solve specific problems
- **[API Reference](https://docs.rs/fabrique)** — Technical details on docs.rs
