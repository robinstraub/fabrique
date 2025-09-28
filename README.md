# Fabrique

Laravel/Eloquent-like factories for Rust structs.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
fabrique = "0.1.0"
```

Derive a factory for your struct:

```rust
use fabrique::Factory;

#[derive(Factory)]
struct User {
    name: String,
    age: u32,
}

// Creates a factory with Option<T> fields
let factory = User::factory();
```

## License

MIT