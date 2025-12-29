use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub orders: HasMany<Order>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
    pub in_stock: bool,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Order {
    pub id: Uuid,
    #[fabrique(belongs_to = "User")]
    pub user_id: Uuid,
    pub status: String,
    pub order_lines: HasMany<OrderLine>,
    #[fabrique(through = "OrderLine")]
    pub products: HasMany<Product>,
}

#[derive(Debug, Default, Factory, PartialEq, Model)]
#[fabrique(table = "order_lines")]
pub struct OrderLine {
    #[fabrique(primary_key, belongs_to = "Order")]
    pub order_id: Uuid,
    #[fabrique(primary_key, belongs_to = "Product")]
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price_cents: i32,
}

#[sqlx::test(migrations = "../migrations")]
async fn test_join_many_to_many_lazy_loading(connection: Pool<Postgres>) {
    let order = Order::factory()
        .for_user(User::factory())
        .has_products(Product::factory(), 1)
        .create(&connection)
        .await
        .unwrap();

    let result = order.products().get(&connection).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_join_parent_to_child(connection: Pool<Postgres>) {
    let user = User::factory()
        .has_orders(Order::factory().status("shipped".to_string()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Parent → Child: User joining Order
    let result = User::query()
        .select()
        .join::<Order>()
        .r#where(User::EMAIL, "=", user.email)
        .get(&connection)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_join_child_to_parent(connection: Pool<Postgres>) {
    User::factory()
        .has_orders(Order::factory().status("pending".to_string()), 1)
        .create(&connection)
        .await
        .unwrap();

    // Child → Parent: Order joining User
    let result = Order::query()
        .select()
        .join::<User>()
        .r#where(Order::STATUS, "=", "pending".to_string())
        .get(&connection)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_where_not_null(connection: Pool<Postgres>) {
    Product::factory().create(&connection).await.unwrap();

    let result = Product::query()
        .select()
        .where_not_null(Product::NAME)
        .get(&connection)
        .await;

    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_chained_where_clauses(connection: Pool<Postgres>) {
    Product::factory()
        .in_stock(true)
        .create(&connection)
        .await
        .unwrap();

    // where -> where (chained)
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .r#where(Product::PRICE_CENTS, ">", 0)
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // where -> where_null (chained)
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .where_null(Product::NAME)
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // where -> where_not_null (chained)
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .where_not_null(Product::NAME)
        .get(&connection)
        .await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_order_by_and_limit(connection: Pool<Postgres>) {
    Product::factory().create(&connection).await.unwrap();

    // order_by on Selected
    let result = Product::query()
        .select()
        .order_by("name", "ASC")
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // order_by on Filtered
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .order_by("name", "ASC")
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // limit on Selected
    let result = Product::query().select().limit(10).get(&connection).await;
    assert!(result.is_ok());

    // limit on Filtered
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .limit(10)
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // limit on Ordered
    let result = Product::query()
        .select()
        .order_by("name", "ASC")
        .limit(10)
        .get(&connection)
        .await;
    assert!(result.is_ok());

    // offset on Limited
    let result = Product::query()
        .select()
        .limit(10)
        .offset(5)
        .get(&connection)
        .await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_first_and_first_or_fail(connection: Pool<Postgres>) {
    Product::factory()
        .in_stock(true)
        .create(&connection)
        .await
        .unwrap();

    // first on Selected
    let result = Product::query().select().first(&connection).await;
    assert!(result.is_ok());

    // first on Filtered
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .first(&connection)
        .await;
    assert!(result.is_ok());

    // first on Ordered
    let result = Product::query()
        .select()
        .order_by("name", "ASC")
        .first(&connection)
        .await;
    assert!(result.is_ok());

    // first_or_fail on Selected (with existing data)
    let result = Product::query().select().first_or_fail(&connection).await;
    assert!(result.is_ok());

    // first_or_fail on Filtered
    let result = Product::query()
        .select()
        .r#where(Product::IN_STOCK, "=", true)
        .first_or_fail(&connection)
        .await;
    assert!(
        result.is_ok(),
        "first_or_fail filtered failed: {:?}",
        result
    );

    // first_or_fail on Ordered
    let result = Product::query()
        .select()
        .order_by("name", "ASC")
        .first_or_fail(&connection)
        .await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update_query(connection: Pool<Postgres>) {
    let product = Product::factory().create(&connection).await.unwrap();

    // update -> set -> where -> execute
    let result = Product::update()
        .set(Product::NAME, "Updated".to_string())
        .r#where(Product::ID, "=", product.id)
        .execute(&connection)
        .await;
    assert!(result.is_ok(), "execute failed: {:?}", result);

    // update -> set -> set (chained) -> where -> returning -> first
    let result = Product::update()
        .set(Product::NAME, "Updated2".to_string())
        .set(Product::PRICE_CENTS, 999)
        .r#where(Product::ID, "=", product.id)
        .returning(Product::columns())
        .first(&connection)
        .await;
    assert!(result.is_ok(), "returning first failed: {:?}", result);
}

#[sqlx::test(migrations = "../migrations")]
async fn test_insert_query(connection: Pool<Postgres>) {
    // insert -> set -> set (chained) -> returning -> first
    let result = Product::query()
        .insert()
        .set(Product::ID, Uuid::new_v4())
        .set(Product::NAME, "New Product".to_string())
        .set(Product::PRICE_CENTS, 100)
        .set(Product::IN_STOCK, true)
        .returning()
        .first(&connection)
        .await;
    assert!(result.is_ok());

    // insert -> on_conflict -> do_nothing -> execute
    let existing_id = result.unwrap().unwrap().id;
    let result = Product::query()
        .insert()
        .set(Product::ID, existing_id)
        .set(Product::NAME, "Conflict".to_string())
        .set(Product::PRICE_CENTS, 200)
        .set(Product::IN_STOCK, false)
        .on_conflict()
        .do_nothing()
        .execute(&connection)
        .await;
    assert!(result.is_ok());

    // insert -> on_conflict -> do_update -> returning -> get
    let result = Product::query()
        .insert()
        .set(Product::ID, existing_id)
        .set(Product::NAME, "Upserted".to_string())
        .set(Product::PRICE_CENTS, 300)
        .set(Product::IN_STOCK, true)
        .on_conflict()
        .do_update()
        .returning()
        .get(&connection)
        .await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../migrations")]
async fn test_joined_state_methods(connection: Pool<Postgres>) {
    let user = User::factory()
        .has_orders(Order::factory(), 1)
        .create(&connection)
        .await
        .unwrap();

    // Joined<Selected> -> order_by, limit, first, first_or_fail
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .order_by("users.name", "ASC")
            .get(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .limit(10)
            .get(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .first(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .first_or_fail(&connection)
            .await
            .is_ok()
    );

    // Joined<Selected> -> where -> order_by, limit, first, first_or_fail
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .r#where(User::EMAIL, "=", user.email.clone())
            .order_by("users.name", "ASC")
            .get(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .r#where(User::EMAIL, "=", user.email.clone())
            .limit(10)
            .get(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .r#where(User::EMAIL, "=", user.email.clone())
            .first(&connection)
            .await
            .is_ok()
    );
    assert!(
        User::query()
            .select()
            .join::<Order>()
            .r#where(User::EMAIL, "=", user.email)
            .first_or_fail(&connection)
            .await
            .is_ok()
    );
}
