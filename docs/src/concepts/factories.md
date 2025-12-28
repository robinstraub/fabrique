# Factories

Factories provide a convenient way to generate model instances for testing and
database seeding. Instead of manually specifying each attribute, factories let
you define defaults and override only what you need.

## Defining a Factory

To define a factory, derive the `Factory` macro alongside `Model`:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model, Factory)]
pub struct Product {
    #[fabrique(primary_key)]
    id: Uuid,
    name: String,
    price_cents: i32,
}
# fn main() {}
```

## Creating Instances

Use the `factory()` method to get a factory builder, then call `create()` to
persist to the database:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Product {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
#     price_cents: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let product = Product::factory()
    .name("Anvil 3000".to_string())
    .price_cents(100)
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Relations

Factories support creating related models. When a model has a foreign key, use
the `for_<relation>` method to link it:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Customer {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     name: String,
# }
#
# #[derive(Model, Factory)]
# pub struct Order {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     #[fabrique(belongs_to = "Customer")]
#     customer_id: Uuid,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create an order with an existing customer
let customer = Customer::factory().create(&pool).await?;
let order = Order::factory()
    .for_customer(customer)
    .create(&pool)
    .await?;

// Or create the related customer inline
let order = Order::factory()
    .for_customer(Customer::factory().name("Acme Corp".to_string()))
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

The `for_customer` method accepts either a `Customer` instance or a
`CustomerFactory`, giving you flexibility in how you set up test data.

## Has Many Relations

For one-to-many relationships, use `HasMany<T>` on the parent and `belongs_to`
on the child to define the relation, then use `has_<relation>()` in factories
to create child records:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
#[derive(Model, Factory)]
pub struct Customer {
    #[fabrique(primary_key)]
    id: Uuid,
    name: String,
    orders: HasMany<Order>,
}

#[derive(Model, Factory)]
pub struct Order {
    #[fabrique(primary_key)]
    id: Uuid,
    #[fabrique(belongs_to = "Customer")]
    customer_id: Uuid,
    total: i32,
}

# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create a customer with 3 orders
let customer = Customer::factory()
    .name("Acme Corp".to_string())
    .has_orders(Order::factory().total(100), 3)
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

The `has_orders` method:

- Takes a child factory and a count
- Creates the parent first, then creates the specified number of children
- Automatically sets the foreign key on each child

You can chain multiple `has_<relation>` calls to create children with different configurations:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
# #[derive(Model, Factory)]
# pub struct Customer {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     orders: HasMany<Order>,
# }
#
# #[derive(Model, Factory)]
# pub struct Order {
#     #[fabrique(primary_key)]
#     id: Uuid,
#     #[fabrique(belongs_to = "Customer")]
#     customer_id: Uuid,
#     total: i32,
# }
#
# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
let customer = Customer::factory()
    .has_orders(Order::factory().total(50), 2)   // 2 small orders
    .has_orders(Order::factory().total(500), 1)  // 1 large order
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

## Many-to-Many Relations

For many-to-many relationships using a join table, define the relationship with
`through` on the `HasMany` field, and use `has_<relation>()` in factories:

```rust,no_run
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use sqlx::{Pool, Postgres};
# use uuid::Uuid;
#
#[derive(Model, Factory)]
pub struct Order {
    #[fabrique(primary_key)]
    id: Uuid,

    #[fabrique(through = "OrderLine")]
    products: HasMany<Product>,
}

#[derive(Model, Factory)]
pub struct Product {
    #[fabrique(primary_key)]
    id: Uuid,
    name: String,
}

/// Join table with composite primary key
#[derive(Model, Factory)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    order_id: Uuid,

    #[fabrique(primary_key, belongs_to = "Product")]
    product_id: Uuid,

    quantity: i32,
}

# async fn example(pool: Pool<Postgres>) -> Result<(), fabrique::Error> {
// Create an order with 3 products
let order = Order::factory()
    .has_products(Product::factory().name("Anvil".to_string()), 3)
    .create(&pool)
    .await?;
# Ok(())
# }
# fn main() {}
```

The `has_products` method:

- Creates the parent `Order` first
- Creates the specified number of `Product` records
- Automatically creates `OrderLine` join records linking them
