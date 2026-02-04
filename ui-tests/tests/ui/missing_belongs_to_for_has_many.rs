use fabrique::prelude::*;

// Parent with HasMany<Child>
#[derive(Factory, Model)]
pub struct Parent {
    #[fabrique(primary_key)]
    id: i32,
    children: HasMany<Child>,
}

// Child WITHOUT belongs_to = "Parent" - should produce a clear error
#[derive(Factory, Model)]
pub struct Child {
    #[fabrique(primary_key)]
    id: i32,
    parent_id: i32,  // Missing: #[fabrique(belongs_to = "Parent")]
}

fn main() {}
