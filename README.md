# Fabrique

[![CI](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml/badge.svg)](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robinstraub/fabrique/graph/badge.svg?token=5zZr9fVZyz)](https://codecov.io/gh/robinstraub/fabrique)
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

## Contributors

[![Contributors](https://contrib.rocks/image?repo=robinstraub/fabrique)](https://github.com/robinstraub/fabrique/graphs/contributors)

## License

MIT License
