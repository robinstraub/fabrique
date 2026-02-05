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

## Running Tests

Requires Docker, [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov),
and [lcov](https://github.com/linux-test-project/lcov). This runs unit tests and
integration tests against all three backends (PostgreSQL, SQLite, MySQL), then
merges the coverage reports.

```bash
docker compose up -d
cargo llvm-cov --features sqlite \
  -p fabrique-derive -p fabrique-core --lib \
  --lcov --output-path lcov-unit.info
DATABASE_URL="postgres://postgres:postgres@localhost:5432/postgres" \
  cargo llvm-cov --features postgres -p fabrique \
  --lcov --output-path lcov-postgres.info
cargo llvm-cov --features sqlite -p fabrique \
  --lcov --output-path lcov-sqlite.info
DATABASE_URL="mysql://root:mysql@localhost:3306/fabrique" \
  cargo llvm-cov --features mysql -p fabrique \
  --lcov --output-path lcov-mysql.info
lcov -a lcov-unit.info -a lcov-postgres.info \
  -a lcov-sqlite.info -a lcov-mysql.info -o lcov-total.info
lcov --list lcov-total.info
```

## Documentation

- **[User Guide](https://robinstraub.github.io/fabrique/)** — Tutorials,
  concepts, and how-to guides
- **[API Reference](https://docs.rs/fabrique)** — Technical documentation on
  docs.rs

## License

MIT License
