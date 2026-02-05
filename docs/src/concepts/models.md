# Models

When using Fabrique, each database table has a corresponding "Model" that is used
to interact with that table. Models allow you to retrieve, insert, update, and
delete records from the database table.

## Defining a Model

To define a model, create a struct and derive the `Model` macro:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    id: Uuid,
    name: String,
    price_cents: i32,
}
# fn main() {}
```

## Table Names

By convention, the "snake case", plural name of the struct will be used as the
table name unless another name is explicitly specified. So, in this case,
Fabrique will assume the `Product` model stores records in the `products` table,
while a `RocketShoe` model would store records in a `rocket_shoes` table.

If your model's corresponding database table does not fit this convention, you
may manually specify the model's table name by defining a table attribute on the
model:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
#[fabrique(table = "acme_products")]
pub struct Product {
    id: Uuid,
    name: String,
}
# fn main() {}
```

## Primary Keys

Fabrique will also assume that each model has a primary key column named `id`.
Otherwise, you must annotate a field with a `fabrique(primary_key)` attribute to
specify which field serves as your model's primary key:

```rust
# extern crate fabrique;
# extern crate sqlx;
# extern crate uuid;
# use fabrique::prelude::*;
# use uuid::Uuid;
#[derive(Model)]
pub struct Product {
    #[fabrique(primary_key)]
    product_id: Uuid,
    name: String,
}
# fn main() {}
```

## Composite Primary Keys

Fabrique has out-of-the-box support for composite primary keys through the use
of multiple `#[fabrique(primary_key)]` attributes:

```rust
# extern crate fabrique;
# extern crate sqlx;
# use fabrique::prelude::*;
#[derive(Model)]
pub struct OrderLine {
    #[fabrique(primary_key)]
    order_id: i32,

    #[fabrique(primary_key)]
    product_id: i32,

    quantity: i32,
}
# fn main() {}
```
