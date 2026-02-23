//! Query Builder Integration Tests
//!
//! Tests organized by state, covering all transitions and execution methods
//! for each state in the query builder state machine.

use fabrique::prelude::*;
use uuid::Uuid;

// ============================================================================
// Test Models
// ============================================================================

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
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

#[derive(Debug, Default, Factory, PartialEq, Model)]
pub struct Message {
    pub id: Uuid,
    #[fabrique(belongs_to = "User", alias = "Sender")]
    pub sender_id: Uuid,
    #[fabrique(belongs_to = "User", alias = "Recipient")]
    pub recipient_id: Uuid,
    pub content: String,
}

// ############################################################################
// SELECT FLOW
// ############################################################################
//
// Initial → Joining → Selected/Joined<Selected> → Filtered<Selected>
//        → Ordered → Limited → Offsetted

// ============================================================================
// Initial
// ============================================================================

mod initial {
    use super::*;
    use fabrique::model::QueryBuilder;

    /// Validates that QueryBuilder implements Default with the new signature.
    #[test]
    fn default_creates_query_builder() {
        let _qb = QueryBuilder::<(), _, fabrique::model::Joined<(Product, ()), ()>>::default();
    }

    #[fabrique::test]
    async fn select_as_transitions_to_selected<DB: Dialect>(pool: Pool<DB>) {
        // select_as on Initial can only select the base model (no joins available)
        let result: Result<Vec<Product>, _> =
            Product::query().select_as::<Product, _>().get(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn insert_transitions_to_inserting<DB: Dialect>(pool: Pool<DB>) {
        let result = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn update_transitions_to_updating<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn join_transitions_to_joining<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query().join::<Order>().get(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn select_columns_transitions_to_selected<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<(String, i32)>, _> = Product::query()
            .select((Product::NAME, Product::PRICE_CENTS))
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .r#where(Product::PRICE_CENTS, ">=", 1000)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .where_not_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_implicit_select_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .order_by(Product::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn limit_implicit_select_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query().limit(10).get(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<Product>, _> = Product::query().first(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Product, _> = Product::query().first_or_fail(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query().get(&pool).await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Joining
// ============================================================================

mod joining {
    use super::*;

    #[fabrique::test]
    async fn join_chains<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Order>, _> = Order::query()
            .join::<User>()
            .join::<OrderLine>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn join_as_chains<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .join_as::<User, Recipient>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn join_through_chains<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Order>, _> = Order::query()
            .join::<OrderLine>()
            .join_through::<Product, OrderLine, _>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn select_as_transitions_to_selected<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn select_columns_transitions_to_selected<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<(String, String)>, _> = Order::query()
            .join::<OrderLine>()
            .join_through::<Product, OrderLine, _>()
            .select((Order::STATUS, Product::NAME))
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .r#where(User::EMAIL, "=", "test@example.com")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_null_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .where_null(User::EMAIL)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .where_not_null(User::EMAIL)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_implicit_select_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .order_by(User::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn limit_implicit_select_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query().join::<Order>().limit(10).get(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<User>, _> = User::query().join::<Order>().first(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<User, _> = User::query().join::<Order>().first_or_fail(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_implicit_select_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query().join::<Order>().get(&pool).await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_on_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .where_on::<Sender, _, _, _, _>(User::NAME, "=", "Alice".to_string())
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_null_on_implicit_select_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .where_null_on::<Sender, _, _, _>(User::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_on_implicit_select_transitions_to_filtered<DB: Dialect>(
        pool: Pool<DB>,
    ) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .where_not_null_on::<Sender, _, _, _>(User::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_on_implicit_select_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .order_by_on::<Sender, _, _, _>(User::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Selected
// ============================================================================

mod selected {
    use super::*;

    #[fabrique::test]
    async fn where_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn where_null_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .where_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .where_not_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .order_by(Product::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn limit_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> =
            Product::query().select_as::<Product, _>().get(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn first_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .first(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Product, _> = Product::query()
            .select_as::<Product, _>()
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_fails_when_empty<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Product, _> = Product::query()
            .select_as::<Product, _>()
            .first_or_fail(&pool)
            .await;
        assert!(result.is_err());
    }
}

// ============================================================================
// Joined<Selected>
// ============================================================================

mod joined_selected {
    use super::*;

    #[fabrique::test]
    async fn where_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let user = User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .r#where(User::EMAIL, "=", user.email)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn where_on_joined_model_column<DB: Dialect>(pool: Pool<DB>) {
        User::factory()
            .has_orders(Order::factory().status("pending".to_string()), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .r#where(Order::STATUS, "=", "pending")
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn where_on_named_join<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .select_as::<Message, _>()
            .where_on::<Sender, _, _, _, _>(User::NAME, "=", "Alice".to_string())
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_null_on_named_join<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .select_as::<Message, _>()
            .where_null_on::<Sender, _, _, _>(User::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_on_named_join<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .select_as::<Message, _>()
            .where_not_null_on::<Sender, _, _, _>(User::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .order_by(User::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_on_named_join<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Message>, _> = Message::query()
            .join_as::<User, Sender>()
            .select_as::<Message, _>()
            .order_by_on::<Sender, _, _, _>(User::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn limit_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn first_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<User>, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
            .first(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_executes<DB: Dialect>(pool: Pool<DB>) {
        User::factory()
            .has_orders(Order::factory(), 1)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<User, _> = User::query()
            .join::<Order>()
            .select_as::<User, _>()
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

    #[fabrique::test]
    async fn where_chains<DB: Dialect>(pool: Pool<DB>) {
        Product::factory()
            .in_stock(true)
            .price_cents(100)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .r#where(Product::PRICE_CENTS, ">", 50)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn where_null_chains<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .where_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_not_null_chains<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .where_not_null(Product::NAME)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn order_by_transitions_to_ordered<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .order_by(Product::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn limit_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn first_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .r#where(Product::IN_STOCK, "=", true)
            .first(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result: Result<Product, _> = Product::query()
            .select_as::<Product, _>()
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

    #[fabrique::test]
    async fn limit_transitions_to_limited<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .order_by(Product::NAME, "ASC")
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .order_by(Product::NAME, "ASC")
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn first_executes<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Option<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .order_by(Product::NAME, "ASC")
            .first(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn first_or_fail_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Product, _> = Product::query()
            .select_as::<Product, _>()
            .order_by(Product::NAME, "ASC")
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

    #[fabrique::test]
    async fn offset_transitions_to_offsetted<DB: Dialect>(pool: Pool<DB>) {
        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .limit(10)
            .offset(5)
            .get(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .limit(10)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}

// ============================================================================
// Offsetted
// ============================================================================

mod offsetted {
    use super::*;

    #[fabrique::test]
    async fn get_executes<DB: Dialect>(pool: Pool<DB>) {
        for _ in 0..3 {
            Product::factory().create(&pool).await.expect("setup");
        }

        let result: Result<Vec<Product>, _> = Product::query()
            .select_as::<Product, _>()
            .limit(10)
            .offset(1)
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}

// ############################################################################
// UPDATE FLOW
// ############################################################################
//
// Initial → Updating → Updated → Filtered<Updated>

// ============================================================================
// Updating
// ============================================================================

mod updating {
    use super::*;

    #[fabrique::test]
    async fn set_transitions_to_updated<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
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

    #[fabrique::test]
    async fn set_chains<DB: Dialect>(pool: Pool<DB>) {
        Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
            .set(Product::PRICE_CENTS, 999)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn where_transitions_to_filtered<DB: Dialect>(pool: Pool<DB>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
            .r#where(Product::ID, "=", product.id)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    // MySQL does not support RETURNING — keep this test sqlite-only.
    #[fabrique::test]
    async fn returning_transitions_to_returned(pool: Pool<Sqlite>) {
        Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::update()
            .set(Product::NAME, "Updated")
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

    #[fabrique::test]
    async fn where_chains<DB: Dialect>(pool: Pool<DB>) {
        let product = Product::factory()
            .in_stock(true)
            .create(&pool)
            .await
            .expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
            .r#where(Product::ID, "=", product.id)
            .r#where(Product::IN_STOCK, "=", true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    // MySQL does not support RETURNING — keep this test sqlite-only.
    #[fabrique::test]
    async fn returning_transitions_to_returned(pool: Pool<Sqlite>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result: Result<Vec<Product>, _> = Product::update()
            .set(Product::NAME, "Updated")
            .r#where(Product::ID, "=", product.id)
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].name, "Updated");
    }

    #[fabrique::test]
    async fn execute_executes<DB: Dialect>(pool: Pool<DB>) {
        let product = Product::factory().create(&pool).await.expect("setup");

        let result = Product::update()
            .set(Product::NAME, "Updated")
            .r#where(Product::ID, "=", product.id)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ############################################################################
// INSERT FLOW
// ############################################################################
//
// Initial → Inserting → Inserted → Conflicted → Upserted

// ============================================================================
// Inserting
// ============================================================================

mod inserting {
    use super::*;

    #[fabrique::test]
    async fn set_transitions_to_inserted<DB: Dialect>(pool: Pool<DB>) {
        let result = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Inserted
// ============================================================================

mod inserted {
    use super::*;

    #[fabrique::test]
    async fn set_chains<DB: Dialect>(pool: Pool<DB>) {
        let result = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    #[fabrique::test]
    async fn on_conflict_transitions_to_conflicted<DB: Dialect>(pool: Pool<DB>) {
        let id = Uuid::new_v4();

        // First insert
        Product::factory()
            .id(id)
            .create(&pool)
            .await
            .expect("setup");

        // Conflict handling
        let result = Product::insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Conflict")
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_nothing()
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }

    // MySQL does not support RETURNING — keep this test sqlite-only.
    #[fabrique::test]
    async fn returning_transitions_to_returned(pool: Pool<Sqlite>) {
        let result: Result<Option<Product>, _> = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[fabrique::test]
    async fn execute_executes<DB: Dialect>(pool: Pool<DB>) {
        let result = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Original")
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

    // MySQL does not support RETURNING — keep this test sqlite-only.
    #[fabrique::test]
    async fn do_update_transitions_to_upserted(pool: Pool<Sqlite>) {
        let id = Uuid::new_v4();

        // First insert
        Product::factory()
            .id(id)
            .create(&pool)
            .await
            .expect("setup");

        // Upsert with do_update
        let result: Result<Vec<Product>, _> = Product::insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated")
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

    #[fabrique::test]
    async fn do_nothing_transitions_to_upserted<DB: Dialect>(pool: Pool<DB>) {
        let id = Uuid::new_v4();

        // First insert
        Product::factory()
            .id(id)
            .create(&pool)
            .await
            .expect("setup");

        // Upsert with do_nothing
        let result = Product::insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Ignored")
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

    // MySQL does not support RETURNING — keep this test sqlite-only.
    #[fabrique::test]
    async fn returning_transitions_to_returned(pool: Pool<Sqlite>) {
        let id = Uuid::new_v4();

        // First insert
        Product::factory()
            .id(id)
            .create(&pool)
            .await
            .expect("setup");

        // Upsert with returning
        let result: Result<Vec<Product>, _> = Product::insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated")
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

    #[fabrique::test]
    async fn execute_executes<DB: Dialect>(pool: Pool<DB>) {
        let id = Uuid::new_v4();

        // First insert
        Product::factory()
            .id(id)
            .create(&pool)
            .await
            .expect("setup");

        // Upsert with execute
        let result = Product::insert()
            .set(Product::ID, id)
            .set(Product::NAME, "Updated")
            .set(Product::PRICE_CENTS, 200)
            .set(Product::IN_STOCK, false)
            .on_conflict()
            .do_nothing()
            .execute(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ############################################################################
// TERMINAL STATE
// ############################################################################
//
// Returned is reached from INSERT/UPDATE flows via returning()

// ============================================================================
// Returned
// ============================================================================

// MySQL does not support RETURNING — keep these tests sqlite-only.
mod returned {
    use super::*;

    #[fabrique::test]
    async fn get_executes(pool: Pool<Sqlite>) {
        let result: Result<Vec<Product>, _> = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .get(&pool)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[fabrique::test]
    async fn first_executes(pool: Pool<Sqlite>) {
        let result: Result<Option<Product>, _> = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first(&pool)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[fabrique::test]
    async fn first_or_fail_executes(pool: Pool<Sqlite>) {
        let result: Result<Product, _> = Product::insert()
            .set(Product::ID, Uuid::new_v4())
            .set(Product::NAME, "Test")
            .set(Product::PRICE_CENTS, 100)
            .set(Product::IN_STOCK, true)
            .returning()
            .first_or_fail(&pool)
            .await;
        assert!(result.is_ok());
    }
}
