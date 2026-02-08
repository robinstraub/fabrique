# Introduction

Fabrique is a SQL-first, type-safe, ergonomic database toolkit for Rust. Your
SQL knowledge transfers directly, the compiler catches errors before your code
runs, and derive macros eliminate the boilerplate.

## Core Values

- **SQL-first** — If you know SQL, you know Fabrique. Queries map directly to
  the SQL you'd write by hand.
- **Type-safe** — Column references, joins, and query structure are validated at
  compile time. Errors surface in your IDE, not in production.
- **Ergonomic** — `#[derive(Model)]` generates everything you need. Fluent APIs
  and sensible defaults keep your code concise.

## Quick Example

Derive `Model` on a struct to map it to a database table:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}
# fn main() {}
```

Fabrique maps `Product` to the `products` table and generates type-safe column
constants. Use them to build queries that read like SQL:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let deals = Product::query()
    .r#where(Product::IN_STOCK, "=", true)
    .r#where(Product::PRICE_CENTS, "<=", 5000)
    .order_by(Product::PRICE_CENTS, "ASC")
    .limit(10)
    .get(&pool)
    .await?;
# Ok(())
# }
```

Common operations like inserting a record have convenience methods that wrap
the query builder, so you don't have to assemble it yourself:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let anvil = Product {
    id: Uuid::new_v4(),
    name: "Anvil 3000".to_string(),
    price_cents: 4999,
    in_stock: true,
};
anvil.create(&pool).await?;
# Ok(())
# }
```

> **New to Fabrique?** Start with the
> [Getting Started](tutorials/getting-started.md) tutorial.

## Where to Go Next

- **[Getting Started](tutorials/getting-started.md)** — Build your first
  Fabrique-powered app in minutes
- **[Concepts](concepts/models.md)** — Understand models, the query builder,
  and relations
- **[Cookbook](cookbook/bulk-update-and-upsert.md)** — Recipes for common tasks
- **[API Reference](https://docs.rs/fabrique)** — Full API documentation on
  docs.rs
