use fabrique_derive::Model;

#[derive(Model)]
struct Product {
    product_id: uuid::Uuid,
    name: String,
}

fn main() {}
