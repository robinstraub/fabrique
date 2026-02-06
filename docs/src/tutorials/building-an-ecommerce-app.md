# Building an E-commerce App

This tutorial builds on [Getting Started](getting-started.md) by adding relations
between models. You'll implement a complete e-commerce backend with users, orders,
and order lines.

## Prerequisites

- Completed the [Getting Started](getting-started.md) tutorial
- Understanding of foreign keys and table relationships

## What You'll Build

A multi-model e-commerce system with:

- **Users** who place orders
- **Orders** that belong to users
- **Order lines** linking orders to products (many-to-many)
- Service functions to create orders and fetch user order history

## The Starting Point

Your team has expanded the database schema and defined these service stubs:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
use fabrique::prelude::*;
use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
}

pub struct OrderLine {
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price_cents: i32,
}

pub struct Product {
    pub id: Uuid,
}

// Get a user with all their orders
pub async fn get_user_with_orders(
    pool: &Pool<Backend>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), Box<dyn std::error::Error>> {
    unimplemented!()
}

// Create an order for a user with multiple products
pub async fn create_order(
    pool: &Pool<Backend>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>, // (product_id, quantity, unit_price_cents)
) -> Result<Order, Box<dyn std::error::Error>> {
    unimplemented!()
}

// Get all products in an order
pub async fn get_order_products(
    pool: &Pool<Backend>,
    order_id: Uuid,
) -> Result<Vec<Product>, Box<dyn std::error::Error>> {
    unimplemented!()
}
# fn main() {}
```

## Database Schema

Extend your database with users, orders, and order lines:

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE
);

CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    status VARCHAR(20) NOT NULL DEFAULT 'pending'
);

CREATE TABLE order_lines (
    order_id UUID NOT NULL REFERENCES orders(id),
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_cents INTEGER NOT NULL,  -- captured at order time
    PRIMARY KEY (order_id, product_id)
);
```

Note that `unit_price_cents` is stored in `order_lines` even though products have
a price. This captures the price at the moment of purchase — if a product's price
changes later, historical orders still reflect what the customer actually paid.

## Defining Models with Relations

### The User Model

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
#
#[derive(Clone, Debug, Factory, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,

    /// A user has many orders
    pub orders: HasMany<Order>,
}
# fn main() {}
```

The `HasMany<Order>` field tells Fabrique that a user can have multiple orders.
This field isn't stored in the database — it generates a method to fetch related
orders.

### The Order Model

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
# #[derive(Clone, Debug, Factory, Model)]
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
#[derive(Clone, Debug, Factory, Model)]
pub struct Order {
    pub id: Uuid,

    /// This order belongs to a user
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,

    pub status: String,

    /// Products in this order, linked through OrderLine
    #[fabrique(through = "OrderLine")]
    pub products: HasMany<Product>,
}
# fn main() {}
```

Key points:

- `#[fabrique(belongs_to = "User")]` marks the foreign key relationship
- `#[fabrique(through = "OrderLine")]` defines a many-to-many relationship via the
  join table

### The OrderLine Model (Join Table)

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
#
#[derive(Clone, Debug, Factory, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    pub order_id: Uuid,

    #[fabrique(primary_key, belongs_to = "Product")]
    pub product_id: Uuid,

    pub quantity: i32,
    pub unit_price_cents: i32,
}
# fn main() {}
```

This join table has:

- A composite primary key (`order_id`, `product_id`)
- Foreign keys to both `Order` and `Product`
- Additional data (`quantity`, `unit_price_cents`)

## Implementing the Service Functions

### Getting a User with Their Orders

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
// -----------------------------------------------------------------------------
// Models
// -----------------------------------------------------------------------------

// --snip--
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
#     pub orders: HasMany<Order>,
# }

// -----------------------------------------------------------------------------
// Service functions
// -----------------------------------------------------------------------------

/// Fetches a user and all their orders.
pub async fn get_user_with_orders(
    pool: &Pool<Backend>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), fabrique::Error> {
    let user: User = User::query()
        .r#where(User::ID, "=", user_id)
        .first_or_fail(pool)
        .await?;
    let orders = user.orders().get(pool).await?;

    Ok((user, orders))
}

// --snip--

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
#     let user = User::factory().create(&pool).await?;
#     let (_, orders) = get_user_with_orders(&pool, user.id).await?;
#     assert_eq!(orders.len(), 0);
#     Ok(())
# }
```

The `orders()` method returns a query builder pre-filtered to this user's orders.
You can add more conditions:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
// -----------------------------------------------------------------------------
// Models
// -----------------------------------------------------------------------------

// --snip--
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
#     pub orders: HasMany<Order>,
# }

// -----------------------------------------------------------------------------
// Service functions
// -----------------------------------------------------------------------------

// --snip--

/// Fetches only the pending orders for a user.
pub async fn get_user_pending_orders(
    pool: &Pool<Backend>,
    user_id: Uuid,
) -> Result<Vec<Order>, fabrique::Error> {
    let user: User = User::query()
        .r#where(User::ID, "=", user_id)
        .first_or_fail(pool)
        .await?;

    user.orders()
        .r#where(Order::STATUS, "=", "pending".to_string())
        .get(pool)
        .await
}

// --snip--

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Insert a user with 2 pending orders and 1 shipped order
let user = User::factory()
    .has_orders(Order::factory().status("pending".to_string()), 2)
    .has_orders(Order::factory().status("shipped".to_string()), 1)
    .create(&pool).await?;

// get_user_pending_orders should only return pending orders
let pending = get_user_pending_orders(&pool, user.id).await?;
assert_eq!(pending.len(), 2);
assert!(pending.iter().all(|o| o.status == "pending"));
# Ok(())
# }
```

### Creating an Order with Items

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
// -----------------------------------------------------------------------------
// Models
// -----------------------------------------------------------------------------

// --snip--
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
#     #[fabrique(through = "OrderLine")]
#     pub products: HasMany<Product>,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }
# #[derive(Clone, Debug, Factory, Model)]
# #[fabrique(table = "order_lines")]
# pub struct OrderLine {
#     #[fabrique(primary_key, belongs_to = "Order")]
#     pub order_id: Uuid,
#     #[fabrique(primary_key, belongs_to = "Product")]
#     pub product_id: Uuid,
#     pub quantity: i32,
#     pub unit_price_cents: i32,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }

// -----------------------------------------------------------------------------
// Service functions
// -----------------------------------------------------------------------------

// --snip--

/// Creates an order for a user with the given items.
pub async fn create_order(
    pool: &Pool<Backend>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>, // (product_id, quantity, unit_price_cents)
) -> Result<Order, fabrique::Error> {
    // Create the order
    let order = Order {
        id: Uuid::new_v4(),
        user_id,
        status: "pending".to_string(),
        products: HasMany::new(),
    };
    let order = order.create(pool).await?;

    // Create order lines for each item
    for (product_id, quantity, unit_price_cents) in items {
        let line = OrderLine {
            order_id: order.id,
            product_id,
            quantity,
            unit_price_cents,
        };
        line.create(pool).await?;
    }

    Ok(order)
}

// --snip--

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Insert a user and two products in the database
let user = User::factory().create(&pool).await?;
let product_a = Product::factory().price_cents(1000).create(&pool).await?;
let product_b = Product::factory().price_cents(2500).create(&pool).await?;

// create_order should return a pending order with both products
let order_items = vec![
    (product_a.id, 2, product_a.price_cents),
    (product_b.id, 1, product_b.price_cents),
];
let order = create_order(&pool, user.id, order_items).await?;

assert_eq!(order.status, "pending");
let products = order.products().get(&pool).await?;
assert_eq!(products.len(), 2);
# Ok(())
# }
```

### Getting Products in an Order

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#
// -----------------------------------------------------------------------------
// Models
// -----------------------------------------------------------------------------

// --snip--
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product {
#     pub id: Uuid,
#     pub name: String,
#     pub price_cents: i32,
#     pub in_stock: bool,
# }
# #[derive(Clone, Debug, Factory, Model)]
# #[fabrique(table = "order_lines")]
# pub struct OrderLine {
#     #[fabrique(primary_key, belongs_to = "Order")]
#     pub order_id: Uuid,
#     #[fabrique(primary_key, belongs_to = "Product")]
#     pub product_id: Uuid,
#     pub quantity: i32,
#     pub unit_price_cents: i32,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order {
#     pub id: Uuid,
#     #[fabrique(belongs_to = "User")]
#     pub user_id: Uuid,
#     pub status: String,
#     #[fabrique(through = "OrderLine")]
#     pub products: HasMany<Product>,
# }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User {
#     pub id: Uuid,
#     pub name: String,
#     pub email: String,
# }

// -----------------------------------------------------------------------------
// Service functions
// -----------------------------------------------------------------------------

// --snip--

/// Returns all products in an order.
pub async fn get_order_products(
    pool: &Pool<Backend>,
    order_id: Uuid,
) -> Result<Vec<Product>, fabrique::Error> {
    let order = Order::find(pool, order_id).await?;
    order.products().get(pool).await
}

// --snip--

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Insert a user, a product, an order, and an order line
let user = User::factory().create(&pool).await?;
let product = Product::factory().create(&pool).await?;

let order = Order {
    id: Uuid::new_v4(),
    user_id: user.id,
    status: "pending".to_string(),
    products: HasMany::new(),
};
let order = order.create(&pool).await?;

let line = OrderLine {
    order_id: order.id,
    product_id: product.id,
    quantity: 1,
    unit_price_cents: 999,
};
line.create(&pool).await?;

// get_order_products should return the product linked to the order
let products = get_order_products(&pool, order.id).await?;
assert_eq!(products.len(), 1);
assert_eq!(products[0].id, product.id);
# Ok(())
# }
```

The `products()` method automatically joins through the `order_lines` table.

## The Complete Implementation

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
use fabrique::prelude::*;
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Models
// -----------------------------------------------------------------------------

/// A user who can place orders.
#[derive(Clone, Debug, Factory, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub orders: HasMany<Order>,
}

/// A product available for purchase.
#[derive(Clone, Debug, Factory, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

/// An order placed by a user.
#[derive(Clone, Debug, Factory, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub status: String,
    #[fabrique(through = "OrderLine")]
    pub products: HasMany<Product>,
}

/// A line item linking an order to a product.
#[derive(Clone, Debug, Factory, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    pub order_id: Uuid,
    #[fabrique(primary_key, belongs_to = "Product")]
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price_cents: i32,
}

// -----------------------------------------------------------------------------
// Service functions
// -----------------------------------------------------------------------------

/// Fetches a user and all their orders.
pub async fn get_user_with_orders(
    pool: &Pool<Backend>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), fabrique::Error> {
    let user = User::find(pool, user_id).await?;
    let orders = user.orders().get(pool).await?;
    Ok((user, orders))
}

/// Creates an order for a user with the given items.
pub async fn create_order(
    pool: &Pool<Backend>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>,
) -> Result<Order, fabrique::Error> {
    let order = Order {
        id: Uuid::new_v4(),
        user_id,
        status: "pending".to_string(),
        products: HasMany::new(),
    };
    let order = order.create(pool).await?;

    for (product_id, quantity, unit_price_cents) in items {
        let line = OrderLine {
            order_id: order.id,
            product_id,
            quantity,
            unit_price_cents,
        };
        line.create(pool).await?;
    }

    Ok(order)
}

/// Returns all products in an order.
pub async fn get_order_products(
    pool: &Pool<Backend>,
    order_id: Uuid,
) -> Result<Vec<Product>, fabrique::Error> {
    let order = Order::find(pool, order_id).await?;
    order.products().get(pool).await
}

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// Insert a user and two products in the database
let user = User::factory()
    .name("Wile E. Coyote".to_string())
    .email("wile@acme.example".to_string())
    .create(&pool).await?;

let anvil = Product::factory()
    .name("Anvil 3000".to_string())
    .price_cents(4999)
    .create(&pool).await?;

let rocket = Product::factory()
    .name("Rocket Skates".to_string())
    .price_cents(14999)
    .create(&pool).await?;

// create_order should return a pending order
let order = create_order(
    &pool,
    user.id,
    vec![
        (anvil.id, 2, anvil.price_cents),
        (rocket.id, 1, rocket.price_cents),
    ],
)
.await?;
assert_eq!(order.status, "pending");

// get_user_with_orders should return the created order
let (_, orders) = get_user_with_orders(&pool, user.id).await?;
assert_eq!(orders.len(), 1);

// get_order_products should return both products
let products = get_order_products(&pool, order.id).await?;
assert_eq!(products.len(), 2);
# Ok(())
# }
```

## Testing with Factories

Factories understand relations and can create related records automatically:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate tokio;
# extern crate uuid;
use fabrique::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, Factory, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub orders: HasMany<Order>,
}

#[derive(Clone, Debug, Factory, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub status: String,
}

# #[fabrique::doctest]
# async fn main(pool: Pool<Backend>) -> Result<(), fabrique::Error> {
// has_orders creates related orders in a single call
let user = User::factory()
    .name("Test User".to_string())
    .has_orders(Order::factory().status("pending".to_string()), 3)
    .create(&pool)
    .await?;

// The 3 orders should be linked to the user
let orders = user.orders().get(&pool).await?;
assert_eq!(orders.len(), 3);
# Ok(())
# }
```

The `has_orders` method creates the user first, then creates 3 orders with the
correct `user_id`.

## Summary

You've learned how to model related data with Fabrique:

1. **`belongs_to`** marks foreign key fields
2. **`HasMany<T>`** declares one-to-many relationships
3. **`through`** enables many-to-many relationships via join tables
4. **Composite primary keys** work naturally with `#[fabrique(primary_key)]`
5. **Relation methods** return query builders for lazy loading

## Next Steps

- Read the [Relations](../concepts/relations.md) concept for more details
- Learn about [Working with Transactions](../guides/working-with-transactions.md)
  to make order creation atomic
- Explore [Error Handling](../concepts/error-handling.md) for robust error management
