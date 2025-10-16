# Fabrique

[![CI](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml/badge.svg)](https://github.com/robinstraub/fabrique/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/robinstraub/fabrique/graph/badge.svg?token=5zZr9fVZyz)](https://codecov.io/gh/robinstraub/fabrique)

Factory pattern library with support for relations and persistence, enabling clean bootstrapping
of complex object graphs with database integration.

## Features

- **Primary Key Support**: Mark fields as primary keys with `#[fabrique(primary_key)]`
- **Factory Relations**: Link factories together with explicit referenced keys using `#[fabrique(relation = "Type", referenced_key = "field")]`
- **Derive Macro**: Automatic factory generation with `#[derive(Factory)]`
- **Database Persistence**: Integrate with databases through the `Persistable` trait
- **Async Support**: Full async/await support for database operations

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

    async fn create(self, connection: &Self::Connection) -> Result<Self, Self::Error> {
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

This approach eliminates the complexity of manually managing object creation
order and dependencies in your test setup and data seeding scenarios.

## License

MIT License
