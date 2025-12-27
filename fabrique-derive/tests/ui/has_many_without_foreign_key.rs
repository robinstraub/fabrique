use fabrique::prelude::*;

#[derive(Model)]
pub struct Customer {
    id: String,
    orders: HasMany<Order>,
}

fn main() {}
