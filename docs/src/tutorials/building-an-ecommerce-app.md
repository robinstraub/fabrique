# Building an E-commerce App

This tutorial builds on [Getting Started](getting-started.md) by adding
relations between models. You'll implement a complete e-commerce backend with
users, orders, and order lines.

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

```rust,ignore
use sqlx::{Pool, Postgres};
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

// Get a user with all their orders
pub async fn get_user_with_orders(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), Box<dyn std::error::Error>> {
    unimplemented!()
}

// Create an order for a user with multiple products
pub async fn create_order(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>, // (product_id, quantity, unit_price_cents)
) -> Result<Order, Box<dyn std::error::Error>> {
    unimplemented!()
}

// Get all products in an order
pub async fn get_order_products(
    pool: &Pool<Postgres>,
    order_id: Uuid,
) -> Result<Vec<Product>, Box<dyn std::error::Error>> {
    unimplemented!()
}
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
    unit_price_cents INTEGER NOT NULL,
    PRIMARY KEY (order_id, product_id)
);
```

## Defining Models with Relations

### The User Model

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order { pub id: Uuid, #[fabrique(belongs_to = "User")] pub user_id: Uuid, pub status: String }
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
This field isn't stored in the database — it generates a method to fetch
related orders.

### The Order Model

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
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
- `#[fabrique(through = "OrderLine")]` defines a many-to-many relationship via
  the join table

### The OrderLine Model (Join Table)

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order { pub id: Uuid, #[fabrique(belongs_to = "User")] pub user_id: Uuid, pub status: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
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

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order { pub id: Uuid, #[fabrique(belongs_to = "User")] pub user_id: Uuid, pub status: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String, pub orders: HasMany<Order> }
#
pub async fn get_user_with_orders(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), fabrique::Error> {
    let user: User = User::query()
        .select()
        .r#where(User::ID, "=", user_id)
        .first_or_fail(pool)
        .await?;
    let orders = user.orders().get(pool).await?;

    Ok((user, orders))
}
# fn main() {}
```

The `orders()` method returns a query builder pre-filtered to this user's
orders. You can add more conditions:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order { pub id: Uuid, #[fabrique(belongs_to = "User")] pub user_id: Uuid, pub status: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String, pub orders: HasMany<Order> }
#
pub async fn get_user_pending_orders(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<Vec<Order>, fabrique::Error> {
    let user: User = User::query()
        .select()
        .r#where(User::ID, "=", user_id)
        .first_or_fail(pool)
        .await?;

    user.orders()
        .r#where(Order::STATUS, "=", "pending".to_string())
        .get(pool)
        .await
}
# fn main() {}
```

### Creating an Order with Items

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Order { pub id: Uuid, #[fabrique(belongs_to = "User")] pub user_id: Uuid, pub status: String }
# #[derive(Clone, Debug, Factory, Model)]
# pub struct User { pub id: Uuid, pub name: String, pub email: String }
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
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
#
pub async fn create_order(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>, // (product_id, quantity, unit_price_cents)
) -> Result<Order, fabrique::Error> {
    // Create the order
    let order = Order {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        user_id,
        status: "pending".to_string(),
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
# fn main() {}
```

### Getting Products in an Order

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Clone, Debug, Factory, Model)]
# pub struct Product { pub id: Uuid, pub name: String, pub price_cents: i32, pub in_stock: bool }
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
# pub struct User { pub id: Uuid, pub name: String, pub email: String }
#
pub async fn get_order_products(
    pool: &Pool<Postgres>,
    order_id: Uuid,
) -> Result<Vec<Product>, fabrique::Error> {
    let order = Order::find(pool, order_id).await?;
    order.products().get(pool).await
}
# fn main() {}
```

The `products()` method automatically joins through the `order_lines` table.

## The Complete Implementation

```rust,ignore
use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

// Models

#[derive(Clone, Debug, Factory, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub orders: HasMany<Order>,
}

#[derive(Clone, Debug, Factory, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[derive(Clone, Debug, Factory, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub status: String,
    #[fabrique(through = "OrderLine")]
    pub products: HasMany<Product>,
}

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

// Service functions

pub async fn get_user_with_orders(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<(User, Vec<Order>), fabrique::Error> {
    let user = User::find(pool, user_id).await?;
    let orders = user.orders().get(pool).await?;
    Ok((user, orders))
}

pub async fn create_order(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    items: Vec<(Uuid, i32, i32)>,
) -> Result<Order, fabrique::Error> {
    let order = Order {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        user_id,
        status: "pending".to_string(),
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

pub async fn get_order_products(
    pool: &Pool<Postgres>,
    order_id: Uuid,
) -> Result<Vec<Product>, fabrique::Error> {
    let order = Order::find(pool, order_id).await?;
    order.products().get(pool).await
}

#[tokio::main]
async fn main() -> Result<(), fabrique::Error> {
    let pool = Pool::<Postgres>::connect("postgres://localhost/shop").await?;

    // Create a user
    let user = User {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        name: "Wile E. Coyote".to_string(),
        email: "wile@acme.example".to_string(),
    };
    let user = user.create(&pool).await?;
    println!("Created user: {}", user.name);

    // Create some products
    let anvil = Product {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        name: "Anvil 3000".to_string(),
        price_cents: 4999,
        in_stock: true,
    };
    let anvil = anvil.create(&pool).await?;

    let rocket = Product {
        id: Uuid::nil(), // In production, use Uuid::new_v4()
        name: "Rocket Skates".to_string(),
        price_cents: 14999,
        in_stock: true,
    };
    let rocket = rocket.create(&pool).await?;

    // Create an order with items
    let order = create_order(
        &pool,
        user.id,
        vec![
            (anvil.id, 2, anvil.price_cents),
            (rocket.id, 1, rocket.price_cents),
        ],
    )
    .await?;
    println!("Created order: {}", order.id);

    // Fetch user with orders
    let (user, orders) = get_user_with_orders(&pool, user.id).await?;
    println!("{} has {} order(s)", user.name, orders.len());

    // Fetch products in order
    let products = get_order_products(&pool, order.id).await?;
    println!("Order contains {} product(s):", products.len());
    for product in products {
        println!("  - {}", product.name);
    }

    Ok(())
}
```

## Testing with Factories

Factories understand relations and can create related records automatically:

```rust,ignore
use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
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

#[sqlx::test(migrations = "migrations")]
async fn test_user_orders_relation(pool: Pool<Postgres>) {
    // Create a user with 3 orders
    let user = User::factory()
        .name("Test User".to_string())
        .has_orders(Order::factory().status("pending".to_string()), 3)
        .create(&pool)
        .await
        .unwrap();

    // Verify the relation
    let orders = user.orders().get(&pool).await.unwrap();
    assert_eq!(orders.len(), 3);
}
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
- Learn about
  [Working with Transactions](../guides/working-with-transactions.md)
  to make order creation atomic
- Explore [Error Handling](../concepts/error-handling.md) for robust error
  management
