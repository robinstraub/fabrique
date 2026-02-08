# Database

Fabrique builds on [sqlx](https://github.com/launchbadge/sqlx) for
all database access. sqlx handles the runtime foundation — connection
pooling, parameterized queries (no risk of SQL injection), and
driver-level communication with the database. Fabrique adds
structurally correct query construction, type-safe column validation,
model mapping, and abstracts away SQL dialect differences (placeholder
syntax, `RETURNING` emulation on MySQL) on top.

For the internal design of the query builder and how it delegates to
sqlx, see [Architecture](architecture.md). For query construction,
see [Query Builder](query-builder.md).

## Backends

The `Backend` type alias resolves at compile time to the sqlx
database type selected by a Cargo feature flag (`postgres`, `mysql`,
or `sqlite`). Features are mutually exclusive — this ensures
`Pool<Backend>` always refers to a single, known database type.

SQL dialects differ across backends — for example, PostgreSQL
supports `RETURNING` natively while MySQL doesn't, and placeholder
syntax varies (`$1` vs `?`). Fabrique abstracts these differences
internally so that model code and queries stay the same regardless
of backend.

Common Rust types (`String`, `i32`, `bool`, `Uuid`, ...) map
directly to SQL columns through sqlx. For the full list of supported
type mappings, see the sqlx documentation:
[Postgres](https://docs.rs/sqlx/latest/sqlx/postgres/types/index.html),
[MySQL](https://docs.rs/sqlx/latest/sqlx/mysql/types/index.html),
[SQLite](https://docs.rs/sqlx/latest/sqlx/sqlite/types/index.html).
For custom Rust types, see
[Models > Type Conversions](models.md#type-conversions).

## Connections

Every Fabrique operation takes a connection — either a
`Pool<Backend>` or a transaction. The API is uniform: the same
query works with both, and code remains backend-agnostic:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
#
# #[fabrique::doctest]
# async fn main(
#     pool: Pool<Backend>,
# ) -> Result<(), fabrique::Error> {
async fn count_users(
    pool: &Pool<Backend>,
) -> Result<usize, fabrique::Error> {
    let users = User::all(pool).await?;
    Ok(users.len())
}

# let count = count_users(&pool).await?;
# Ok(())
# }
```

Transactions are acquired from the pool with `pool.begin()` and
committed explicitly. A transaction that is dropped without
`commit()` rolls back automatically:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let id = Uuid::new_v4();
# Product::factory()
#     .id(id).in_stock(true).price_cents(5000)
#     .create(&pool).await?;
let mut tx = pool.begin().await?;

Product::update()
    .set(Product::IN_STOCK, false)
    .r#where(Product::PRICE_CENTS, ">", 1000)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
# Ok(())
# }
```

sqlx's `Acquire` trait abstracts over both pools and transactions,
enabling generic function signatures that accept either.

## Error Handling

All Fabrique operations return `Result<T, fabrique::Error>`. Two
variants come up frequently.

`NotFound` is returned when a query expects exactly one record but
none matches — typically from `find` or `first_or_fail`. When an
empty result is a valid outcome rather than an error, `first`
returns `Option<T>` instead.

`Conversion` is returned when a
[type conversion](models.md#type-conversions) fails — for example,
a database column contains `"unknown"` but the Rust enum only
accepts `"active"` or `"inactive"`. The error carries the field
name, source and target types, and whether the conversion failed
reading from or writing to the database.

See the [API reference](https://docs.rs/fabrique) for the full
`Error` type.

## SQLite for Testing

SQLite's in-memory mode makes it a natural choice for testing:
no external database setup, fast execution, and complete
isolation between tests.

Enable the `sqlite` feature in your project:

```toml
[dependencies]
fabrique = { version = "0.2", features = ["sqlite"] }
```

> **Note:** Backend features are mutually exclusive. If your
> production environment uses PostgreSQL or MySQL, use Cargo
> feature flags or a workspace setup to switch backends between
> development and deployment — do not enable two backend features
> simultaneously.

The `#[fabrique::doctest]` macro leverages SQLite's in-memory
mode for documentation examples.

---

Next: [Architecture](architecture.md) — the internal design of the
query builder.
