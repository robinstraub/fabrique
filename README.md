# Fabrique

[![CI](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml/badge.svg)](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robinstraub/fabrique/graph/badge.svg?token=5zZr9fVZyz)](https://codecov.io/gh/robinstraub/fabrique)
[![Crates.io](https://img.shields.io/crates/v/fabrique.svg)](https://crates.io/crates/fabrique)
[![Downloads](https://img.shields.io/crates/d/fabrique.svg)](https://crates.io/crates/fabrique)
[![docs.rs](https://docs.rs/fabrique/badge.svg)](https://docs.rs/fabrique)

SQL-first, type-safe, ergonomic database toolkit for Rust.

## Features

- **SQL-first** — Builds on SQL semantics rather than hiding
  them; the query builder maps directly to the SQL you'd write
- **Type-safe** — Column constants, typed where clauses, and
  compile-time join validation catch errors before they run
- **Ergonomic** — Convention-driven models, fluent query
  builder, and factories for test data generation

## Quick Example

Add fabrique to your `Cargo.toml` with the feature matching your database backend:

```toml
[dependencies]
fabrique = { version = "0.2", features = ["postgres"] }
```

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
    .r#where(Product::PRICE_CENTS, ">=", 1000)
    .get(&pool)
    .await?;

// Create test data
let product = Product::factory()
    .name("Anvil 3000")
    .create(&pool)
    .await?;
```

For tutorials and detailed documentation, see the **[User Guide](https://robinstraub.github.io/fabrique/)**.

## Documentation

- **[User Guide](https://robinstraub.github.io/fabrique/)** — Tutorials,
  concepts, and how-to guides
- **[API Reference](https://docs.rs/fabrique)** — Technical documentation on
  docs.rs

## Running Tests

**SQLite** (no external dependency):

```bash
cargo test --features sqlite,testing
```

**PostgreSQL** and **MySQL** require a running database. Start them with Docker,
then run the tests:

```bash
docker compose up -d

DATABASE_URL="postgres://postgres:postgres@localhost:5432/postgres" \
  cargo test --features postgres,testing

DATABASE_URL="mysql://root:mysql@localhost:3306/fabrique" \
  cargo test --features mysql,testing
```

## Contributors

[![Contributors](https://contrib.rocks/image?repo=robinstraub/fabrique)](https://github.com/robinstraub/fabrique/graphs/contributors)

## License

[MIT License](LICENSE)
