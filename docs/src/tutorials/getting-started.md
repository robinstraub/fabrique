# Getting Started

This tutorial walks you through adding Fabrique to an existing e-commerce
application. You'll learn how to convert plain Rust structs into database-backed
models and implement service functions that your views can call.

## Prerequisites

- Rust 1.75 or later
- A supported database (SQLite, PostgreSQL, or MySQL)
- Basic knowledge of SQL and async Rust

## Scenario

You're working on an e-commerce website. The frontend team has defined the API
contract, and you need to implement the persistence layer. Your task is to make
all the service functions work with a database.

## The Starting Point

Here's what you're given — a `Product` struct and service function stubs that
need implementation:

```rust,ignore
use fabrique::prelude::*;
use uuid::Uuid;

pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

// Find a product by its ID
pub async fn find_product_by_id(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<Product, Box<dyn std::error::Error>> {
    unimplemented!()
}

// List all products currently in stock
pub async fn list_available_products(
    pool: &Pool<sqlx::Sqlite>,
) -> Result<Vec<Product>, Box<dyn std::error::Error>> {
    unimplemented!()
}

// Create a new product
pub async fn create_product(
    pool: &Pool<sqlx::Sqlite>,
    name: String,
    price_cents: i32,
) -> Result<Product, Box<dyn std::error::Error>> {
    unimplemented!()
}

// Update a product's price
pub async fn update_product_price(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
    new_price_cents: i32,
) -> Result<Product, Box<dyn std::error::Error>> {
    unimplemented!()
}

// Delete a product
pub async fn delete_product(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!()
}
```

Let's implement each function using Fabrique.

## Database Setup

First, create the products table:

```sql
CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    price_cents INTEGER NOT NULL,
    in_stock BOOLEAN NOT NULL DEFAULT 1
);
```

## Adding Fabrique

Add dependencies to `Cargo.toml`:

```toml
[dependencies]
fabrique = { version = "0.2", features = ["sqlite"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "uuid"] }
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
fabrique = { version = "0.2", features = ["testing"] }
```

Now derive `Model` on the `Product` struct:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
use fabrique::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}
# fn main() {}
```

Fabrique automatically:

- Maps `Product` to the `products` table
- Uses `id` as the primary key
- Generates column constants (`Product::ID`, `Product::NAME`, etc.)
- Provides query and persistence methods

## Implementing the Service Functions

### Finding a Product by ID

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn find_product_by_id(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<Product, fabrique::Error> {
    Product::find(pool, id).await
}
# fn main() {}
```

This queries `SELECT * FROM products WHERE id = $1` and returns the matching
record. If no product is found, it returns an error.

### Listing Available Products

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn list_available_products(
    pool: &Pool<sqlx::Sqlite>,
) -> Result<Vec<Product>, fabrique::Error> {
    Product::query()
        .r#where(Product::IN_STOCK, "=", true)
        .get(pool)
        .await
}
# fn main() {}
```

The query builder provides a fluent API. Here we:

1. Start a query with `Product::query()`
2. Add a WHERE clause with `.r#where()`
3. Execute and collect results with `.get(pool)`

Column constants like `Product::IN_STOCK` are type-safe — passing the wrong type
won't compile.

### Creating a Product

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn create_product(
    pool: &Pool<sqlx::Sqlite>,
    name: String,
    price_cents: i32,
) -> Result<Product, fabrique::Error> {
    let product = Product {
        id: Uuid::new_v4(),
        name,
        price_cents,
        in_stock: true,
    };

    product.create(pool).await
}
# fn main() {}
```

The `create` method inserts the record and returns it. If a record with the same
primary key already exists, it returns an error.

### Updating a Product's Price

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn update_product_price(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
    new_price_cents: i32,
) -> Result<Product, fabrique::Error> {
    let mut product: Product = Product::query()
        .r#where(Product::ID, "=", id)
        .first_or_fail(pool)
        .await?;
    product.price_cents = new_price_cents;
    product.save(pool).await
}
# fn main() {}
```

The pattern is: fetch, modify, save. The `save` method performs an upsert — it
inserts if the record is new, or updates if it exists.

### Deleting a Product

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
pub async fn delete_product(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<(), fabrique::Error> {
    Product::destroy(pool, id).await
}
# fn main() {}
```

The `destroy` method deletes by primary key without fetching the record first.

## The Complete Implementation

Here's the full working code:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
use fabrique::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

pub async fn find_product_by_id(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<Product, fabrique::Error> {
    Product::find(pool, id).await
}

pub async fn list_available_products(
    pool: &Pool<sqlx::Sqlite>,
) -> Result<Vec<Product>, fabrique::Error> {
    Product::query()
        .r#where(Product::IN_STOCK, "=", true)
        .get(pool)
        .await
}

pub async fn create_product(
    pool: &Pool<sqlx::Sqlite>,
    name: String,
    price_cents: i32,
) -> Result<Product, fabrique::Error> {
    let product = Product {
        id: Uuid::new_v4(),
        name,
        price_cents,
        in_stock: true,
    };
    product.create(pool).await
}

pub async fn update_product_price(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
    new_price_cents: i32,
) -> Result<Product, fabrique::Error> {
    let mut product: Product = Product::query()
        .r#where(Product::ID, "=", id)
        .first_or_fail(pool)
        .await?;
    product.price_cents = new_price_cents;
    product.save(pool).await
}

pub async fn delete_product(
    pool: &Pool<sqlx::Sqlite>,
    id: Uuid,
) -> Result<(), fabrique::Error> {
    Product::destroy(pool, id).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an in-memory SQLite database and run migrations
    let pool = sqlx::SqlitePool::connect(":memory:").await?;
    sqlx::migrate!("../migrations/sqlite").run(&pool).await?;

    // Create products
    let anvil = create_product(&pool, "Anvil 3000".to_string(), 4999).await?;
    let rocket = create_product(&pool, "Rocket Skates".to_string(), 14999).await?;
    println!("Created: {} and {}", anvil.name, rocket.name);

    // Find a specific product
    let found = find_product_by_id(&pool, anvil.id).await?;
    println!("Found: {}", found.name);

    // List available products
    let available = list_available_products(&pool).await?;
    println!("Available: {} products", available.len());

    // Update a price
    let updated = update_product_price(&pool, anvil.id, 3999).await?;
    let price = updated.price_cents as f64 / 100.0;
    println!("Updated {} to ${:.2}", updated.name, price);

    // Delete a product
    delete_product(&pool, rocket.id).await?;
    println!("Deleted Rocket Skates");

    Ok(())
}
```

## What's Next: Testing

Now that the service functions work, you'll want to test them.
The [Testing with Fabrique](testing-with-fabrique.md) tutorial
shows how to write database-backed tests with isolated
databases and generated test data.

## Summary

You've learned how to integrate Fabrique into an existing application:

1. **Derive `Model`** on your structs to enable database operations
2. **Use the query builder** with `first_or_fail` for primary key lookups
3. **Use the query builder** with `r#where` for filtered queries
4. **Use `create` and `save`** for persistence
5. **Use `destroy`** for deletion

## Next Steps

- Learn about [Relations](../concepts/relations.md) to model users, orders, and
  products together
- Explore the [Query Builder](../concepts/query-builder.md)
  for complex queries
- Read about
  [Transactions](../concepts/database.md#connections) for
  atomic operations
- Learn how to [test your code](testing-with-fabrique.md)
  with factories and isolated databases
