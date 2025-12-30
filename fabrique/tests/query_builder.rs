//! Query Builder Integration Tests
//!
//! Tests organized by state, covering all transitions and execution methods
//! for each state in the query builder state machine.

use fabrique::prelude::*;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

// ============================================================================
// Test Models
// ============================================================================

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

// ============================================================================
// Initial
// ============================================================================

mod initial {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn select_transitions_to_selected(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query().select().get(&pool).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_transitions_to_inserting(pool: Pool<Postgres>) {
        let result: Result<Option<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_transitions_to_updating(_pool: Pool<Postgres>) {
        let _query = Product::update().set(Product::NAME, "Updated".to_string());
    }
}

// ============================================================================
// Selected
// ============================================================================

mod selected {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn join_transitions_to_joined(pool: Pool<Postgres>) {
        let result: Result<Vec<User>, _> = User::query().select().join::<Order>().get(&pool).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn join_through_transitions_to_joined(pool: Pool<Postgres>) {
        let result: Result<Vec<Order>, _> = Order::query()
            .select()
            .join_through::<OrderLine, Product>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_transitions_to_filtered(pool: Pool<Postgres>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_null_transitions_to_filtered(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .where_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_not_null_transitions_to_filtered(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .where_not_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn order_by_transitions_to_ordered(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .order_by("name", "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn limit_transitions_to_limited(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query().select().limit(10).get(&pool).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::query().select().get(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Option<Product>, _> = Product::query().select().first(&pool).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Product, _> = Product::query().select().first_or_fail(&pool).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_fails_when_empty(pool: Pool<Postgres>) {
        let result: Result<Product, _> = Product::query().select().first_or_fail(&pool).await;
        assert!(result.is_err());
    }
}

// ============================================================================
// Joined<Selected>
// ============================================================================

mod joined_selected {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn where_transitions_to_filtered(pool: Pool<Postgres>) {
        let user = User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .select()
            .join::<Order>()
            .r#where(User::EMAIL, "=", user.email)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_on_joined_model_column(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory().status("pending".to_string()), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .select()
            .join::<Order>()
            .r#where(Order::STATUS, "=", "pending".to_string())
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn join_through_transitions_to_joined(pool: Pool<Postgres>) {
        let result: Result<Vec<Order>, _> = Order::query()
            .select()
            .join::<User>()
            .join_through::<OrderLine, Product>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn order_by_transitions_to_ordered(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .select()
            .join::<Order>()
            .order_by("users.name", "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn limit_transitions_to_limited(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .select()
            .join::<Order>()
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query().select().join::<Order>().get(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_executes(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Option<User>, _> =
            User::query().select().join::<Order>().first(&pool).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_executes(pool: Pool<Postgres>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<User, _> = User::query()
            .select()
            .join::<Order>()
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Filtered<Selected>
// ============================================================================

mod filtered_selected {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn where_chains(pool: Pool<Postgres>) {
        Product::factory()
            .in_stock(true)
            .price_cents(100)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .r#where(Product::PRICE_CENTS, ">", 50)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_null_chains(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .where_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_not_null_chains(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .where_not_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn order_by_transitions_to_ordered(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .order_by("name", "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn limit_transitions_to_limited(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_executes(pool: Pool<Postgres>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Option<Product>, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_executes(pool: Pool<Postgres>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Product, _> = Product::query()
            .select()
            .r#where(Product::IN_STOCK, "=", true)
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Ordered
// ============================================================================

mod ordered {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn limit_transitions_to_limited(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .order_by("name", "ASC")
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .order_by("name", "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Option<Product>, _> = Product::query()
            .select()
            .order_by("name", "ASC")
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Product, _> = Product::query()
            .select()
            .order_by("name", "ASC")
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Limited
// ============================================================================

mod limited {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn offset_transitions_to_offsetted(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .limit(10)
            .offset(5)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::query().select().limit(10).get(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}

// ============================================================================
// Offsetted
// ============================================================================

mod offsetted {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        for _ in 0..3 {
            Product::factory().create(&pool).await.expect("setup");
        }

        let result: Result<Vec<Product>, _> = Product::query()
            .select()
            .limit(10)
            .offset(1)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}

// ============================================================================
// Updating
// ============================================================================

mod updating {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn set_transitions_to_updated(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Updated
// ============================================================================

mod updated {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn set_chains(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .set(Product::PRICE_CENTS, 999)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn where_transitions_to_filtered(pool: Pool<Postgres>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .r#where(Product::ID, "=", product.id)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn returning_transitions_to_returned(pool: Pool<Postgres>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        let products = result.unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].name, "Updated");
    }
}

// ============================================================================
// Filtered<Updated>
// ============================================================================

mod filtered_updated {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn where_chains(pool: Pool<Postgres>) {
        let product = Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .r#where(Product::ID, "=", product.id)
            .r#where(Product::IN_STOCK, "=", true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn returning_transitions_to_returned(pool: Pool<Postgres>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .r#where(Product::ID, "=", product.id)
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].name, "Updated");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn execute_executes(pool: Pool<Postgres>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated".to_string())
            .r#where(Product::ID, "=", product.id)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Inserting
// ============================================================================

mod inserting {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn set_transitions_to_inserted(pool: Pool<Postgres>) {
        let result: Result<Option<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}

// ============================================================================
// Inserted
// ============================================================================

mod inserted {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn set_chains(pool: Pool<Postgres>) {
        let result: Result<Option<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn on_conflict_transitions_to_conflicted(pool: Pool<Postgres>) {
        let id = Uuid::new_v4();

        // First insert
        Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await
            .expect("setup");

        // Conflict handling
        let result = Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Conflict".to_string())
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_nothing()
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn returning_transitions_to_returned(pool: Pool<Postgres>) {
        let result: Result<Option<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn execute_executes(pool: Pool<Postgres>) {
        let result = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Conflicted
// ============================================================================

mod conflicted {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn do_update_transitions_to_upserted(pool: Pool<Postgres>) {
        let id = Uuid::new_v4();

        // First insert
        Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await
            .expect("setup");

        // Upsert with do_update
        let result: Result<Vec<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated".to_string())
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_update()
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].name, "Updated");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn do_nothing_transitions_to_upserted(pool: Pool<Postgres>) {
        let id = Uuid::new_v4();

        // First insert
        Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await
            .expect("setup");

        // Upsert with do_nothing
        let result = Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Ignored".to_string())
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_nothing()
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Upserted
// ============================================================================

mod upserted {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn returning_transitions_to_returned(pool: Pool<Postgres>) {
        let id = Uuid::new_v4();

        // First insert
        Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await
            .expect("setup");

        // Upsert with returning
        let result: Result<Vec<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated".to_string())
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_update()
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn execute_executes(pool: Pool<Postgres>) {
        let id = Uuid::new_v4();

        // First insert
        Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Original".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await
            .expect("setup");

        // Upsert with execute
        let result = Product::query()
            .insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated".to_string())
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_nothing()
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Returned
// ============================================================================

mod returned {
    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn get_executes(pool: Pool<Postgres>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_executes(pool: Pool<Postgres>) {
        let result: Result<Option<Product>, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn first_or_fail_executes(pool: Pool<Postgres>) {
        let result: Result<Product, _> = Product::query()
            .insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test".to_string())
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }
}
