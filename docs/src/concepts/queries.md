# Queries

Once you have created a model and its corresponding database table, you are ready to start retrieving data from your database. You can think of each Fabrique model as a powerful query builder allowing you to fluently query the database table associated with the model.

## Retrieving All Records

The model's `all` method retrieves all of the records from the model's associated database table:

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
#     id: Uuid,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let products: Vec<Product> = Product::all(&pool).await?;
# Ok(())
# }
```

## Building Queries

The `all` method returns all results in the model's table. Since each Fabrique model serves as a query builder, you may add additional constraints to queries and then invoke the `get` method to retrieve the results:

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
#     id: Uuid,
#     price_cents: i32,
# }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let products: Vec<Product> = Product::query()
    .select()
    .r#where(Product::PRICE_CENTS, ">=", 42)
    .get(&pool)
    .await?;
# Ok(())
# }
```

## Retrieving Results

Fabrique provides several methods to execute a query and retrieve results:

### `get` — All Matching Records

Returns all records matching the query as a `Vec<T>`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid, price_cents: i32 }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let products: Vec<Product> = Product::query()
    .select()
    .r#where(Product::PRICE_CENTS, ">", 50)
    .get(&pool)
    .await?;
# Ok(())
# }
```

### `first` — First or None

Returns the first matching record as `Option<T>`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid, price_cents: i32 }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let product: Option<Product> = Product::query()
    .select()
    .r#where(Product::PRICE_CENTS, ">", 100)
    .first(&pool)
    .await?;
# Ok(())
# }
```

### `first_or_fail` — First or Error

Returns the first matching record, or an error if none found:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid, name: String, price_cents: i32 }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
# Product::factory().price_cents(150).create(&pool).await?;
let product: Product = Product::query()
    .select()
    .r#where(Product::PRICE_CENTS, ">", 100)
    .first_or_fail(&pool)
    .await?;
# Ok(())
# }
```

## Column Constants

When you derive the `Model` macro, Fabrique generates column constants for each field. These constants are used in query methods to provide type-safe column references:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    id: Uuid,
    price_cents: i32,
    name: String,
}

// Generated constants:
// Product::ID
// Product::PRICE_CENTS
// Product::NAME
# fn main() {}
```

## Joins

Fabrique supports bidirectional joins between related models. When you define a `belongs_to` relationship, both directions of the join become available automatically via the `Joinable` trait.

### Basic Join

Use the `join::<T>()` method to add an INNER JOIN to your query:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct User { id: Uuid, email: String, orders: HasMany<Order> }
# #[derive(Factory, Model)]
# pub struct Order { id: Uuid, #[fabrique(belongs_to = "User")] user_id: Uuid }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Parent → Child: User joining Orders
let users = User::query()
    .join::<Order>()
    .select()
    .r#where(User::EMAIL, "=", "john@example.com".to_string())
    .get(&pool)
    .await?;

// Child → Parent: Order joining User
let orders = Order::query()
    .join::<User>()
    .select()
    .get(&pool)
    .await?;
# Ok(())
# }
```

Both directions work seamlessly — Fabrique generates the appropriate `Joinable` implementations when you define a `belongs_to` relationship.

### Many-to-Many Joins

For many-to-many relationships with a join table, use `join_through`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Order { id: Uuid, #[fabrique(through = "OrderLine")] products: HasMany<Product> }
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid }
# #[derive(Factory, Model)]
# #[fabrique(table = "order_lines")]
# pub struct OrderLine { #[fabrique(primary_key, belongs_to = "Order")] order_id: Uuid, #[fabrique(primary_key, belongs_to = "Product")] product_id: Uuid }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
let orders = Order::query()
    .join::<OrderLine>()
    .join_through::<Product, OrderLine, _>()
    .select()
    .get(&pool)
    .await?;
# Ok(())
# }
```

### Selecting from Joined Models

By default, `select()` returns the root model's columns. To select columns from a joined model instead, use `select_as`:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct User { id: Uuid, email: String, orders: HasMany<Order> }
# #[derive(Factory, Model)]
# pub struct Order { id: Uuid, status: String, #[fabrique(belongs_to = "User")] user_id: Uuid }
#
# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Returns Vec<Order> instead of Vec<User>
let orders: Vec<Order> = User::query()
    .join::<Order>()
    .select_as::<Order, _>()
    .r#where(User::EMAIL, "=", "john@example.com".to_string())
    .get(&pool)
    .await?;
# Ok(())
# }
```

The compiler verifies that the selected model is in the join list. Attempting to select from a model that hasn't been joined causes a compile-time error.

## Type-Safe Columns

Column constants are not just names — they carry type information. When using `r#where`, the value must match the column's type:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid, price_cents: i32 }
#
# fn example() {
# let _ = Product::query().select()
// ✓ Compiles: PRICE_CENTS is i32, 42 is i32
.r#where(Product::PRICE_CENTS, ">", 42);
# }
# fn main() {}
```

```rust,compile_fail
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
# #[derive(Factory, Model)]
# pub struct Product { id: Uuid, price_cents: i32 }
#
# fn example() {
# let _ = Product::query().select()
// ✗ Won't compile: PRICE_CENTS is i32, "heavy" is &str
.r#where(Product::PRICE_CENTS, ">", "heavy");
# }
# fn main() {}
```
