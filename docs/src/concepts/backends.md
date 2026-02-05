# Backends

Fabrique provides a unified API across multiple database backends. Your code remains
portable — the same models and queries work with PostgreSQL, MySQL, or SQLite without
modification.

## Backend Abstraction

The core abstraction is the `Backend` type alias, which resolves at compile time
to the appropriate SQLx database type:

| Feature Flag | `Backend` resolves to |
| ------------ | --------------------- |
| `postgres`   | `sqlx::Postgres`      |
| `mysql`      | `sqlx::MySql`         |
| `sqlite`     | `sqlx::Sqlite`        |

Backend selection happens at compile time through Cargo features. Enable exactly
one in your `Cargo.toml`:

```toml
[dependencies]
fabrique = { version = "0.2", features = ["postgres"] }
```

Features are mutually exclusive — enabling multiple backends causes a compile
error. This constraint ensures type safety: `Pool<Backend>` always refers to a
single, known database type.

## Database-Agnostic Code

This abstraction lets you write code that works across all backends:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
use fabrique::prelude::*;  // Includes Backend, Pool, Model, Factory, etc.
# use uuid::Uuid;
# #[derive(fabrique::Model)]
# pub struct User { id: Uuid }

async fn count_users(pool: &Pool<Backend>) -> Result<usize, fabrique::Error> {
    let users = User::all(pool).await?;
    Ok(users.len())
}
# fn main() {}
```

The same code compiles against any backend — only the feature flag changes.

## Backend-Specific Behavior

Fabrique abstracts away most database differences internally. Your model code stays
the same regardless of backend.

| Capability         | PostgreSQL    | MySQL        | SQLite |
| ------------------ | ------------- | ------------ | ------ |
| `RETURNING` clause | Native        | Emulated     | Native |
| UUID storage       | `UUID` type   | `BINARY(16)` | `TEXT` |
| Timestamps         | `TIMESTAMPTZ` | `DATETIME`   | `TEXT` |

## SQLite for Testing

SQLite's in-memory mode makes it ideal for testing: no external database setup,
fast execution, and complete isolation between tests. You can use a different
backend in development/production while keeping SQLite for tests:

```toml
[dependencies]
fabrique = { version = "0.2", features = ["postgres"] }

[dev-dependencies]
fabrique = { version = "0.2", features = ["sqlite"] }
```

The `#[fabrique::doctest]` macro leverages this approach for documentation examples.
