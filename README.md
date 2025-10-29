# Fabrique

[![CI](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml/badge.svg)](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robinstraub/fabrique/graph/badge.svg?token=5zZr9fVZyz)](https://codecov.io/gh/robinstraub/fabrique)

A lightweight ORM for Rust inspired by Laravel Eloquent, combining ease of use
with Rust's safety guarantees. Fabrique provides elegant model definitions,
relationship management, and built-in factories for testing and seeding.

## Features

- **Eloquent-Inspired API**: Familiar patterns from Laravel with Rust's type safety
- **Primary Key Support**: Mark fields as primary keys with `#[fabrique(primary_key)]`
- **Factory Relations**: Link factories together with explicit referenced keys
using `#[fabrique(relation = "Type", referenced_key = "field")]`
- **Derive Macro**: Automatic factory generation with `#[derive(Factory)]`
- **Database Persistence**: Integrate with databases through the `Persistable` trait
- **Async Support**: Full async/await support for database operations
- **Testing & Seeding**: Built-in factory pattern for easy test data generation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
fabrique = "0.1.0"
```

### Basic Factory

```rust
use fabrique::Factory;

#[derive(Factory)]
struct Anvil {
    weight: u32,
    material: String,
}

let anvil = Anvil::factory()
    .weight(50)
    .material("Steel".to_string());
```

### Factory Relations

Link factories together to manage complex object dependencies:

```rust
use fabrique::Factory;

#[derive(Factory)]
struct Hammer {
    #[fabrique(primary_key)]
    id: u32,
    weight: u32,
    handle_length: u32,
}

#[derive(Factory)]
struct Anvil {
    #[fabrique(primary_key)]
    id: u32,
    weight: u32,
    #[fabrique(relation = "Hammer", referenced_key = "id")]
    hammer_id: u32,
}

// Create an anvil with a related hammer
let anvil = Anvil::factory()
    .weight(100)
    .for_hammer(|hammer_factory| {
        hammer_factory.weight(5).handle_length(30)
    });
```

### Database Persistence

Integrate factories with your database using the `Persistable` trait:

```rust
use fabrique::{Factory, Persistable};

#[derive(Factory)]
struct Anvil {
    #[fabrique(primary_key)]
    id: u32,
    weight: u32,
    #[fabrique(relation = "Hammer", referenced_key = "id")]
    hammer_id: u32,
}

impl Persistable for Anvil {
    type Connection = DatabaseConnection;
    type Error = DatabaseError;

    async fn create(self, connection: &Self::Connection) ->
      Result<Self, Self::Error> {
        // Your database insertion logic here
        database::insert_anvil(connection, self).await
    }
}

// Create and persist to database
let anvil = Anvil::factory()
    .weight(100)
    .for_hammer(|hammer| hammer.weight(5))
    .create(&db_connection)
    .await?;
```

## Why Fabrique?

Fabrique brings Laravel Eloquent's developer-friendly approach to Rust while
maintaining type safety and performance. Perfect for:

- **Testing**: Generate complex test data with relationships in seconds
- **Database Seeding**: Bootstrap your development database with realistic data
- **Prototyping**: Quickly iterate on data models without boilerplate
- **Migration from Laravel**: Familiar patterns for developers coming from the
PHP ecosystem

## License

MIT License
