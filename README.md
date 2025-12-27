# Fabrique

[![CI](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml/badge.svg)](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robinstraub/fabrique/graph/badge.svg?token=5zZr9fVZyz)](https://codecov.io/gh/robinstraub/fabrique)
[![docs.rs](https://docs.rs/fabrique/badge.svg)](https://docs.rs/fabrique)

A lightweight ORM for Rust that combines ease of use with Rust's safety guarantees.

## Features

- **Fluent API** — Expressive, chainable query building
- **Derive-based** — Models and factories generated from struct definitions
- **Factory support** — Test data generation with relationship handling

## Quick Example

```rust
use fabrique::prelude::*;

#[derive(Model, Factory)]
pub struct Product {
    #[fabrique(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
}

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
```

## Documentation

- **[User Guide](https://robinstraub.github.io/fabrique/)** — Tutorials, concepts, and how-to guides
- **[API Reference](https://docs.rs/fabrique)** — Technical documentation on docs.rs

## License

MIT License
