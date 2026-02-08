# Query Builder

The query builder is generic over a model. It infers the table name
from the model declaration, so you never specify it manually — no
risk of referencing the wrong table. Subsequent operations follow the
same logic: columns are resolved from the model, and the compiler
validates types at each step.

Rather than writing `QueryBuilder::<Product>::new()`, models provide
direct entry points:

- `Product::query()` — starts a SELECT path
- `Product::query().insert()` — starts an INSERT
- `Product::update()` — starts an UPDATE

> **Why `query()` and not `select()`?** In SQL, `SELECT` comes
> first. Fabrique inverts this: joins and filters come before column
> selection. This is what enables type-safe selects — the compiler
> needs to know which models are joined before it can validate which
> columns you pick. `select()` is reserved for choosing columns
> explicitly (see [Column Selection](#column-selection)).

## Reading Data

`Product::query()` starts a SELECT query. Chain methods to add
clauses, then execute the query by passing a connection:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(3000).in_stock(true).create(&pool).await?;
let deals: Vec<Product> = Product::query()
    .r#where(Product::IN_STOCK, "=", true)
    .r#where(Product::PRICE_CENTS, "<=", 5000)
    .get(&pool)
    .await?;
# Ok(())
# }
```

`.get()` executes the query asynchronously and returns all matching
records. Multiple `.r#where()` calls are combined with `AND`. All
executors take a connection — pool or transaction. See the
[API reference](https://docs.rs/fabrique) for the full executor
specification, and [Database](database.md) for pool vs transaction
handling.

### Null Checks

`r#where` takes a column, an operator, and a value — but NULL isn't
a value of the column's type. Rather than compromising type safety,
Fabrique provides dedicated methods:

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
#     pub deleted_at: Option<String>,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# User::factory().deleted_at(None).create(&pool).await?;
let active = User::query()
    .where_null(User::DELETED_AT)
    .get(&pool)
    .await?;

let archived = User::query()
    .where_not_null(User::DELETED_AT)
    .get(&pool)
    .await?;
# Ok(())
# }
```

> **Tip:** For systematic soft delete handling, see
> [Soft Delete and Restore Records](../cookbook/soft-delete-and-restore-records.md).

### Ordering and Pagination

`order_by` sorts results by a column. `limit` and `offset` control
how many records are returned and where to start:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().in_stock(true).create(&pool).await?;
let page = Product::query()
    .r#where(Product::IN_STOCK, "=", true)
    .order_by(Product::PRICE_CENTS, "ASC")
    .limit(20)
    .offset(40)
    .get(&pool)
    .await?;
# Ok(())
# }
```

### Single Record

When you expect a single record, `first` returns an `Option<T>`,
and `first_or_fail` returns `T` directly — raising
`Error::NotFound` rather than leaving you to unwrap or convert to
a `Result`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(3000).create(&pool).await?;
let cheapest: Option<Product> = Product::query()
    .r#where(Product::PRICE_CENTS, "<=", 5000)
    .first(&pool)
    .await?;

let cheapest: Product = Product::query()
    .r#where(Product::PRICE_CENTS, "<=", 5000)
    .first_or_fail(&pool)
    .await?;
# Ok(())
# }
```

## Writing Data

### Inserting

`Product::query().insert()` starts an INSERT query. `.set()` assigns
column values, then `.execute()` runs the query without returning
data:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
Product::query()
    .insert()
    .set(Product::ID, Uuid::new_v4())
    .set(Product::NAME, "Anvil 3000")
    .set(Product::PRICE_CENTS, 4999)
    .set(Product::IN_STOCK, true)
    .execute(&pool)
    .await?;
# Ok(())
# }
```

When you need the inserted record back, chain `.returning()` before
the executor — this avoids a separate SELECT roundtrip:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let anvil: Product = Product::query()
    .insert()
    .set(Product::ID, Uuid::new_v4())
    .set(Product::NAME, "Anvil 3000")
    .set(Product::PRICE_CENTS, 4999)
    .set(Product::IN_STOCK, true)
    .returning()
    .first_or_fail(&pool)
    .await?;
# Ok(())
# }
```

### Updating

`Product::update()` starts an UPDATE query. Combine `.set()` with
`.r#where()` to target specific records:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(30).create(&pool).await?;
Product::update()
    .set(Product::PRICE_CENTS, 100)
    .r#where(Product::PRICE_CENTS, "<", 50)
    .execute(&pool)
    .await?;
# Ok(())
# }
```

`.returning()` works here too — use it to get the updated records
back without a separate query.

> **Note:** `model.create()` and `model.save()` (see
> [Models](models.md#convenience-methods)) wrap this query builder
> internally. They return the persisted record using `RETURNING`.
> MySQL doesn't support `RETURNING` — Fabrique emulates the
> behavior with two queries (INSERT then SELECT).

## Joins

`join::<T>()` adds an INNER JOIN to a related model. The compiler
enforces that `T` is joinable from the current query context — if
no relationship is declared between the models, it won't compile:

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
#     pub orders: HasMany<Order>,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# Order::factory().for_user(user).create(&pool).await?;
// Both directions work — the relation is declared once
let users = User::query()
    .join::<Order>()
    .r#where(User::EMAIL, "=", "wile@acme.com")
    .get(&pool)
    .await?;

let orders = Order::query()
    .join::<User>()
    .get(&pool)
    .await?;
# Ok(())
# }
```

When a table isn't directly related but reachable through an
intermediate table, `join_through` chains the join. The
intermediate must be joined first — just like in SQL, the join
chain must be valid at each step:

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
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     #[fabrique(through = "OrderLine")]
#     pub products: HasMany<Product>,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
# #[derive(Clone, Factory, Model)]
# #[fabrique(table = "order_lines")]
# pub struct OrderLine {
#     #[fabrique(primary_key, belongs_to = "Order")]
#     pub order_id: Uuid,
#     #[fabrique(primary_key, belongs_to = "Product")]
#     pub product_id: Uuid,
#     pub quantity: i32,
#     pub unit_price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Order::factory().for_user(User::factory()).create(&pool).await?;
// Order → OrderLine (direct) → Product (through OrderLine)
let orders = Order::query()
    .join::<OrderLine>()
    .join_through::<Product, OrderLine, _>()
    .get(&pool)
    .await?;
# Ok(())
# }
```

This is standard SQL join semantics — Fabrique adds compile-time
validation on top. See [Relations](relations.md) for how to
declare relationships between models.

## Column Selection

When a query executes, Fabrique maps each database row into a
struct instance. `select_as::<Model, _>()` specifies which model
to build:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().create(&pool).await?;
let products: Vec<Product> = Product::query()
    .select_as::<Product, _>()
    .get(&pool)
    .await?;
# Ok(())
# }
```

By default, the query builder infers the root model — so
`select_as` can be omitted. This is what all the examples above
do:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().create(&pool).await?;
// Equivalent — Product is inferred
let products: Vec<Product> = Product::query()
    .get(&pool)
    .await?;
# Ok(())
# }
```

After a join, `select_as` switches the output to a different
model:

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
#     pub orders: HasMany<Order>,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# Order::factory().for_user(user).create(&pool).await?;
let orders: Vec<Order> = User::query()
    .join::<Order>()
    .select_as::<Order, _>()
    .r#where(User::EMAIL, "=", "wile@acme.com")
    .get(&pool)
    .await?;
# Ok(())
# }
```

To select specific columns, use `select` with a tuple of column
constants — the return type matches:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Clone, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().create(&pool).await?;
let rows: Vec<(String, i32)> = Product::query()
    .select((Product::NAME, Product::PRICE_CENTS))
    .get(&pool)
    .await?;
# Ok(())
# }
```

This works across joined models too, as long as the join is
present:

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
#     pub orders: HasMany<Order>,
# }
# #[derive(Clone, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     pub status: String,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# let user = User::factory().create(&pool).await?;
# Order::factory().for_user(user).create(&pool).await?;
let rows: Vec<(String, String)> = User::query()
    .join::<Order>()
    .select((User::NAME, Order::STATUS))
    .get(&pool)
    .await?;
# Ok(())
# }
```

The compiler verifies that each selected column belongs to a model
present in the query — selecting from an unjoined model won't
compile.

---

Next: [Relations](relations.md) — declaring relationships between
models.
