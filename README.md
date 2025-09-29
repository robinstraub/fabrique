# Fabrique

Factory pattern library with support for relations, enabling clean bootstrapping of complex object graphs.

## Features

- **Factory Relations**: Link factories together to manage dependencies between objects
- **Derive Macro**: Automatic factory generation with `#[derive(Factory)]`

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

The real power comes from linking factories together:

```rust
use fabrique::Factory;

#[derive(Factory)]
struct Hammer {
    weight: u32,
    handle_length: u32,
}

#[derive(Factory)]
struct Anvil {
    weight: u32,
    #[factory(relation = "HammerFactory")]
    hammer_id: u32,
}

// Create an anvil with a related hammer
let anvil = Anvil::factory()
    .weight(100)
    .for_hammer(|hammer_factory| {
        hammer_factory.weight(5).handle_length(30)
    });
```

This approach eliminates the complexity of manually managing object creation order and dependencies in your test setup and data seeding scenarios.

## License

MIT