//! # Model Query Builder (Layer 2)
//!
//! Type-safe query builder that wraps the [SQL Query
//! Builder](crate::sql::QueryBuilder) with model awareness and compile-time
//! column validation.
//!
//! This is the user-facing layer of Fabrique's 2-layer query builder
//! architecture. For a detailed explanation of the architecture, see the
//! [Query Builder Internals](https://fabrique.rs/internals/query-builder.html) documentation.
//!
//! # Overview
//!
//! This layer adds three key features on top of the SQL layer:
//!
//! 1. **Model awareness**: Methods automatically use the model's table name and
//!    columns
//! 2. **Type-safe columns**: WHERE clauses require typed `Column<M>` values
//!    instead of strings
//! 3. **Join tracking**: The `Joins` type parameter tracks which models have
//!    been joined, enabling compile-time validation of cross-model WHERE
//!    clauses
//!
//! # Architecture
//!
//! The `QueryBuilder<S, Joins, Output>` struct has three type parameters:
//! - `S`: Current state (reuses states from [`crate::sql`])
//! - `Joins`: Type-level list of joined models for compile-time validation
//! - `Output`: The output type for execution methods (defaults to `()`)
//!
//! The root model is derived from the innermost element of the `Joins` HList
//! via the [`RootModel`](crate::model::join::RootModel) trait, eliminating
//! redundancy in the type signature.
//!
//! # Example
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # #[derive(Model)]
//! # #[fabrique(table = "products")]
//! # struct Product { id: i32, name: String }
//! # async fn example(pool: &sqlx::Pool<fabrique_core::database::Backend>) -> Result<(), Box<dyn std::error::Error>> {
//! let products = Product::query()
//!     .r#where(Product::ID, "=", 42)
//!     .get(pool).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Module Organization
//!
//! This file declares macros first, then the main `QueryBuilder` struct,
//! followed by impl blocks organized by state. Macro invocations at the end
//! connect states to their available methods.

use crate::database::{self, Backend, Column};
use crate::model::Model;
use crate::model::column_set::ColumnSet;
use crate::model::join::{Contains, Joined, RootModel};
use crate::relation::Joinable;
use crate::sql::operators::{self, Direction};
use crate::sql::{
    Conflicted, Filtered, Initial as SqlInitial, Inserted, Inserting, Joined as SqlJoined, Limited,
    Offsetted, Ordered, QueryBuilder as SqlQueryBuilder, Returned, Selected, SupportsReturning,
    Updated, Updating, Upserted,
};
use sqlx::Database;
use std::marker::PhantomData;

// ############################################################################
// LAYER 2 STATES
// ############################################################################
//
// Pure Layer 2 states that don't involve SQL generation yet.
// These accumulate information until we transition to Building.

/// Initial state - no operations performed yet.
/// This is a pure L2 state, no SQL builder exists yet.
pub struct Initial;

/// State accumulating joins before SELECT.
/// Stores pending joins that will be flushed when calling select().
pub struct Joining {
    pub joins: Vec<(&'static str, &'static str, &'static str)>,
}

/// Wrapper state containing the SQL QueryBuilder.
/// Transitioning to this state triggers actual SQL generation.
pub struct Building<DB: Database, S> {
    pub inner: SqlQueryBuilder<DB, S>,
}

// ############################################################################
// MACRO DEFINITIONS
// ############################################################################
//
// Shared infrastructure for generating methods across multiple states.
// These macros wrap the SQL layer's methods with model-aware type safety.

/// Implements the `where` method for Building states.
///
/// Syntax: `impl_where!(InputSqlState => OutputSqlState)` - transitions to
/// `Building<Filtered<OutputSqlState>>`
macro_rules! impl_where {
    ($input:ty => $output:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $input>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
        {
            /// Adds a WHERE clause to the query.
            pub fn r#where<Column, Operator, JoinedModel, Index>(
                self,
                column: Column,
                operator: Operator,
                value: impl Into<Column::Type>,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, (), Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
                Operator: Into<operators::Operator>,
            {
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.r#where(
                            column.qualified_name(),
                            operator,
                            value.into(),
                        ),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds a WHERE clause on a named join.
            pub fn where_on<Alias, Column, Operator, JoinedModel, Index>(
                self,
                column: Column,
                operator: Operator,
                value: Column::Type,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Alias: crate::Alias<Target = JoinedModel>,
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Alias, Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
                Operator: Into<operators::Operator>,
            {
                let qualified = format!("{}.{}", Alias::NAME, column.name());
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.r#where(&qualified, operator, value),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds a WHERE IS NULL clause to the query.
            pub fn where_null<Column, JoinedModel, Index>(
                self,
                column: Column,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, (), Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
            {
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.where_null(column.qualified_name()),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds a WHERE IS NULL clause on a named join.
            pub fn where_null_on<Alias, Column, JoinedModel, Index>(
                self,
                column: Column,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Alias: crate::Alias<Target = JoinedModel>,
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Alias, Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
            {
                let qualified = format!("{}.{}", Alias::NAME, column.name());
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.where_null(&qualified),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds a WHERE IS NOT NULL clause to the query.
            pub fn where_not_null<Column, JoinedModel, Index>(
                self,
                column: Column,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, (), Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
            {
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.where_not_null(column.qualified_name()),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds a WHERE IS NOT NULL clause on a named join.
            pub fn where_not_null_on<Alias, Column, JoinedModel, Index>(
                self,
                column: Column,
            ) -> QueryBuilder<Building<Joins::Database, $output>, Joins, Output>
            where
                Alias: crate::Alias<Target = JoinedModel>,
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Alias, Index>,
                for<'q> Column::Type:
                    sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
            {
                let qualified = format!("{}.{}", Alias::NAME, column.name());
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.where_not_null(&qualified),
                    },
                    _marker: PhantomData,
                }
            }
        }
    };
}

/// Implements the `order_by` method for Building states.
macro_rules! impl_order_by {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
        {
            /// Adds an `ORDER BY` clause to the query.
            pub fn order_by<Column, JoinedModel, Index>(
                self,
                column: Column,
                direction: impl Into<Direction>,
            ) -> QueryBuilder<Building<Joins::Database, Ordered>, Joins, Output>
            where
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, (), Index>,
            {
                QueryBuilder {
                    state: Building {
                        inner: self
                            .state
                            .inner
                            .order_by(column.qualified_name(), direction),
                    },
                    _marker: PhantomData,
                }
            }

            /// Adds an `ORDER BY` clause on a named join.
            pub fn order_by_on<Alias, Column, JoinedModel, Index>(
                self,
                column: Column,
                direction: impl Into<Direction>,
            ) -> QueryBuilder<Building<Joins::Database, Ordered>, Joins, Output>
            where
                Alias: crate::Alias<Target = JoinedModel>,
                Column: database::Column<JoinedModel>,
                Joins: Contains<JoinedModel, Alias, Index>,
            {
                let qualified = format!("{}.{}", Alias::NAME, column.name());
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.order_by(&qualified, direction),
                    },
                    _marker: PhantomData,
                }
            }
        }
    };
}

/// Implements the `limit` method for Building states.
///
/// Syntax: `impl_limit!(SqlState)` - transitions to [`Building<Limited>`]
macro_rules! impl_limit {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
        {
            /// Adds a `LIMIT` clause to the query.
            ///
            /// Transitions to [`Limited`] state.
            pub fn limit<'a>(
                self,
                count: i64,
            ) -> QueryBuilder<Building<Joins::Database, Limited>, Joins, Output>
            where
                i64: sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
            {
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.limit(count),
                    },
                    _marker: PhantomData,
                }
            }
        }
    };
}

/// Implements the `returning` method for Building states.
///
/// Syntax: `impl_returning!(SqlState)` - transitions to [`Building<Returned>`]
macro_rules! impl_returning {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database + SupportsReturning,
        {
            /// Specifies the columns to return after the statement.
            ///
            /// Generates `RETURNING col1, col2, ...`.
            /// Transitions to [`Returned`] state.
            ///
            /// Only available on backends that support `RETURNING` (PostgreSQL,
            /// SQLite).
            pub fn returning(
                self,
            ) -> QueryBuilder<Building<Joins::Database, Returned>, Joins, Output> {
                QueryBuilder {
                    state: Building {
                        inner: self.state.inner.returning(Joins::Root::qualified_columns()),
                    },
                    _marker: PhantomData,
                }
            }
        }
    };
}

/// Implements the `get` method for Building states.
///
/// Syntax: `impl_get!(SqlState)` - adds execution capability
macro_rules! impl_get {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
            Joins::Error: From<sqlx::Error>,
        {
            /// Executes the query and returns all matching rows.
            pub async fn get<'e, E>(self, executor: E) -> Result<Vec<Output>, Joins::Error>
            where
                E: sqlx::Executor<'e, Database = Joins::Database>,
                Output: for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row>
                    + Send
                    + Unpin,
                <Joins::Database as sqlx::Database>::Arguments:
                    sqlx::IntoArguments<Joins::Database>,
            {
                self.state.inner.get(executor).await.map_err(Into::into)
            }
        }
    };
}

/// Implements the `first` method for Building states.
///
/// Syntax: `impl_first!(SqlState)` - adds execution capability
macro_rules! impl_first {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
            Joins::Error: From<sqlx::Error>,
        {
            /// Retrieves the first row from the query result.
            ///
            /// Returns `None` if no rows match the query.
            pub async fn first<'e, E>(self, executor: E) -> Result<Option<Output>, Joins::Error>
            where
                E: sqlx::Executor<'e, Database = Joins::Database>,
                Output: for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row>
                    + Send
                    + Unpin,
                <Joins::Database as sqlx::Database>::Arguments:
                    sqlx::IntoArguments<Joins::Database>,
            {
                self.state.inner.first(executor).await.map_err(Into::into)
            }
        }
    };
}

/// Implements the `first_or_fail` method for Building states.
///
/// Syntax: `impl_first_or_fail!(SqlState)` - adds execution capability
macro_rules! impl_first_or_fail {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
            Joins::Error: From<sqlx::Error>,
        {
            /// Retrieves the first row from the query result, or fails if none exists.
            ///
            /// Returns an error if no rows match the query.
            pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<Output, Joins::Error>
            where
                E: sqlx::Executor<'e, Database = Joins::Database>,
                Output: for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row>
                    + Send
                    + Unpin,
                <Joins::Database as sqlx::Database>::Arguments:
                    sqlx::IntoArguments<Joins::Database>,
            {
                self.state
                    .inner
                    .first_or_fail(executor)
                    .await
                    .map_err(Into::into)
            }
        }
    };
}

/// Implements the `execute` method for Building states.
///
/// Syntax: `impl_execute!(SqlState)` - adds execution capability
macro_rules! impl_execute {
    ($state:ty) => {
        impl<Joins, Output> QueryBuilder<Building<Joins::Database, $state>, Joins, Output>
        where
            Joins: RootModel,
            Joins::Database: Database,
        {
            /// Executes the statement without returning any rows.
            pub async fn execute<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
            where
                E: sqlx::Executor<'e, Database = Joins::Database>,
                <Joins::Database as sqlx::Database>::Arguments:
                    sqlx::IntoArguments<Joins::Database>,
            {
                self.state.inner.execute(executor).await
            }
        }
    };
}

// ############################################################################
// QUERY BUILDER
// ############################################################################

/// Type-safe query builder with model awareness and join tracking.
///
/// # Architecture
///
/// Uses a 2-layer design:
/// - **Layer 2 states** (`Initial`, `Joining`): Pure model-level states that
///   accumulate information before SQL generation
/// - **Building states** (`Building<DB, S>`): Wrap the SQL QueryBuilder for
///   actual SQL construction
///
/// # Type Parameters
///
/// - `S`: Current state - either a pure L2 state or `Building<DB, SqlState>`
/// - `Joins`: Type-level list tracking joined models, enabling compile-time
///   validation that WHERE clauses reference accessible models. The root model
///   is derived via [`RootModel::Root`].
/// - `Output`: The type returned when executing the query
pub struct QueryBuilder<S = Initial, Joins = (), Output = ()> {
    /// Current state data. For L2 states like `Joining`, contains accumulated
    /// data. For `Building<S>`, contains the SQL QueryBuilder.
    state: S,

    /// Phantom data to track joined models at compile time.
    _marker: PhantomData<(Joins, Output)>,
}

// ============================================================================
// Initial
// ============================================================================

impl<M: Model> Default for QueryBuilder<Initial, Joined<(M, ()), ()>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Model> QueryBuilder<Initial, Joined<(M, ()), ()>> {
    /// Creates a new query builder for the given model.
    pub fn new() -> Self {
        Self {
            state: Initial,
            _marker: PhantomData,
        }
    }
}

impl<Joins> QueryBuilder<Initial, Joins>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Internal helper for select operations.
    fn select_impl<Output: Model>(
        self,
    ) -> QueryBuilder<Building<Joins::Database, Selected>, Joins, Output> {
        let inner =
            SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
                .select(Output::qualified_columns());
        QueryBuilder {
            state: Building { inner },
            _marker: PhantomData,
        }
    }

    /// Starts a SELECT query for individual columns.
    ///
    /// Accepts a tuple of column ZSTs (e.g., `(Product::ID, Product::NAME)`).
    /// The output type is a tuple of the corresponding column value types.
    ///
    /// Transitions to [`Building<Selected>`] state.
    pub fn select<C, Models, Indices>(
        self,
        _columns: C,
    ) -> QueryBuilder<Building<Joins::Database, Selected>, Joins, C::Output>
    where
        C: ColumnSet<Joins, Models, Indices>,
    {
        let inner =
            SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
                .select(C::qualified_names());
        QueryBuilder {
            state: Building { inner },
            _marker: PhantomData,
        }
    }

    /// Starts a SELECT query returning columns from a specific model.
    ///
    /// For queries without joins, this can only select the base model.
    /// Transitions to [`Building<Selected>`] state.
    pub fn select_as<Output, Index>(
        self,
    ) -> QueryBuilder<Building<Joins::Database, Selected>, Joins, Output>
    where
        Output: Model,
        Joins: Contains<Output, (), Index>,
    {
        self.select_impl::<Output>()
    }

    /// Adds a WHERE clause, implicitly selecting all columns from the root
    /// model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().r#where(...)`.
    pub fn r#where<Column, Operator, JoinedModel, Index>(
        self,
        column: Column,
        operator: Operator,
        value: impl Into<Column::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
        Operator: Into<operators::Operator>,
    {
        self.select_impl::<Joins::Root>()
            .r#where(column, operator, value)
    }

    /// Adds a WHERE IS NULL clause, implicitly selecting all columns from the
    /// root model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().where_null(...)`.
    pub fn where_null<Column, JoinedModel, Index>(
        self,
        column: Column,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().where_null(column)
    }

    /// Adds a WHERE IS NOT NULL clause, implicitly selecting all columns from
    /// the root model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().where_not_null(...)`.
    pub fn where_not_null<Column, JoinedModel, Index>(
        self,
        column: Column,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().where_not_null(column)
    }

    /// Adds an ORDER BY clause, implicitly selecting all columns from the root
    /// model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().order_by(...)`.
    pub fn order_by<Column, JoinedModel, Index>(
        self,
        column: Column,
        direction: impl Into<Direction>,
    ) -> QueryBuilder<Building<Joins::Database, Ordered>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
    {
        self.select_impl::<Joins::Root>()
            .order_by(column, direction)
    }

    /// Adds a LIMIT clause, implicitly selecting all columns from the root
    /// model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().limit(...)`.
    pub fn limit<'a>(
        self,
        count: i64,
    ) -> QueryBuilder<Building<Joins::Database, Limited>, Joins, Joins::Root>
    where
        i64: sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().limit(count)
    }

    /// Executes the query, implicitly selecting all columns from the root
    /// model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().get(...)`.
    pub async fn get<'e, E>(self, executor: E) -> Result<Vec<Joins::Root>, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().get(executor).await
    }

    /// Retrieves the first row, implicitly selecting all columns from the root
    /// model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().first(...)`.
    pub async fn first<'e, E>(self, executor: E) -> Result<Option<Joins::Root>, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().first(executor).await
    }

    /// Retrieves the first row or fails, implicitly selecting all columns from
    /// the root model.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().first_or_fail(...)`.
    pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<Joins::Root, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>()
            .first_or_fail(executor)
            .await
    }

    /// Adds a JOIN before SELECT.
    ///
    /// Transitions to [`Joining`] state to accumulate joins.
    pub fn join<J>(self) -> QueryBuilder<Joining, Joined<(J, ()), Joins>>
    where
        J: Model<Database = Joins::Database> + Joinable<Joins::Root>,
    {
        QueryBuilder {
            state: Joining {
                joins: vec![(
                    J::table_name(),
                    <J as Joinable<Joins::Root>>::left_column().qualified_name(),
                    <J as Joinable<Joins::Root>>::right_column().qualified_name(),
                )],
            },
            _marker: PhantomData,
        }
    }

    /// Adds a named JOIN before SELECT.
    ///
    /// Use this for disambiguation when joining the same model multiple times.
    /// Transitions to [`Joining`] state to accumulate joins.
    pub fn join_as<J, Alias>(self) -> QueryBuilder<Joining, Joined<(J, Alias), Joins>>
    where
        J: Model<Database = Joins::Database> + Joinable<Joins::Root, Alias>,
        Alias: crate::Alias<Target = J>,
    {
        let table_as_alias = format!("{} AS {}", J::table_name(), Alias::NAME);
        let left = format!(
            "{}.{}",
            Alias::NAME,
            <J as Joinable<Joins::Root, Alias>>::left_column().name()
        );
        let right = <J as Joinable<Joins::Root, Alias>>::right_column().qualified_name();
        QueryBuilder {
            state: Joining {
                joins: vec![(table_as_alias.leak(), left.leak(), right)],
            },
            _marker: PhantomData,
        }
    }

    /// Starts an UPDATE query for this model.
    ///
    /// Transitions to [`Building<Updating>`] state.
    pub fn update(self) -> QueryBuilder<Building<Joins::Database, Updating>, Joins> {
        let inner =
            SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
                .update();
        QueryBuilder {
            state: Building { inner },
            _marker: PhantomData,
        }
    }

    /// Starts an INSERT query for this model.
    ///
    /// Transitions to [`Building<Inserting>`] state.
    pub fn insert(self) -> QueryBuilder<Building<Joins::Database, Inserting>, Joins> {
        let inner =
            SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
                .insert();
        QueryBuilder {
            state: Building { inner },
            _marker: PhantomData,
        }
    }
}

// ############################################################################
// SELECT FLOW
// ############################################################################
//
// Initial → Joining → Selected/Joined<Selected> → Filtered<Selected>
//        → Ordered → Limited → Offsetted

// ============================================================================
// Joining
// ============================================================================

impl<Joins, Output> QueryBuilder<Joining, Joins, Output>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Adds another JOIN before SELECT.
    ///
    /// Chains multiple joins in [`Joining`] state.
    pub fn join<J>(mut self) -> QueryBuilder<Joining, Joined<(J, ()), Joins>>
    where
        J: Model<Database = Joins::Database> + Joinable<Joins::Root>,
    {
        self.state.joins.push((
            J::table_name(),
            <J as Joinable<Joins::Root>>::left_column().qualified_name(),
            <J as Joinable<Joins::Root>>::right_column().qualified_name(),
        ));

        QueryBuilder {
            state: self.state,
            _marker: PhantomData,
        }
    }

    /// Adds another named JOIN before SELECT.
    ///
    /// Use this for disambiguation when joining the same model multiple times.
    /// Chains multiple joins in [`Joining`] state.
    pub fn join_as<J, Alias>(mut self) -> QueryBuilder<Joining, Joined<(J, Alias), Joins>>
    where
        J: Model<Database = Joins::Database> + Joinable<Joins::Root, Alias>,
        Alias: crate::Alias<Target = J>,
    {
        let table_as_alias = format!("{} AS {}", J::table_name(), Alias::NAME);
        let left = format!(
            "{}.{}",
            Alias::NAME,
            <J as Joinable<Joins::Root, Alias>>::left_column().name()
        );
        let right = <J as Joinable<Joins::Root, Alias>>::right_column().qualified_name();
        self.state
            .joins
            .push((table_as_alias.leak(), left.leak(), right));

        QueryBuilder {
            state: self.state,
            _marker: PhantomData,
        }
    }

    /// Joins a model through an already-joined pivot table (many-to-many).
    ///
    /// Use this instead of [`join`] when `JoinModel` is related to
    /// `PivotModel`, not to the root model.
    ///
    /// Chains multiple joins in [`Joining`] state.
    pub fn join_through<JoinModel, PivotModel, Index>(
        mut self,
    ) -> QueryBuilder<Joining, Joined<(JoinModel, ()), Joins>>
    where
        PivotModel: Model<Database = Joins::Database>,
        JoinModel: Model<Database = Joins::Database> + Joinable<PivotModel>,
        Joins: Contains<PivotModel, (), Index>,
    {
        self.state.joins.push((
            JoinModel::table_name(),
            <JoinModel as Joinable<PivotModel>>::left_column().qualified_name(),
            <JoinModel as Joinable<PivotModel>>::right_column().qualified_name(),
        ));

        QueryBuilder {
            state: self.state,
            _marker: PhantomData,
        }
    }

    /// Adds a WHERE clause, implicitly selecting all columns from the root
    /// model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().r#where(...)`.
    pub fn r#where<Column, Operator, JoinedModel, Index>(
        self,
        column: Column,
        operator: Operator,
        value: impl Into<Column::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
        Operator: Into<operators::Operator>,
    {
        self.select_impl::<Joins::Root>()
            .r#where(column, operator, value)
    }

    /// Adds a WHERE IS NULL clause, implicitly selecting all columns from the
    /// root model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().where_null(...)`.
    pub fn where_null<Column, JoinedModel, Index>(
        self,
        column: Column,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().where_null(column)
    }

    /// Adds a WHERE IS NOT NULL clause, implicitly selecting all columns from
    /// the root model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().where_not_null(...)`.
    pub fn where_not_null<Column, JoinedModel, Index>(
        self,
        column: Column,
    ) -> QueryBuilder<Building<Joins::Database, Filtered<Selected>>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
        for<'q> Column::Type: sqlx::Encode<'q, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().where_not_null(column)
    }

    /// Adds an ORDER BY clause, implicitly selecting all columns from the root
    /// model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().order_by(...)`.
    pub fn order_by<Column, JoinedModel, Index>(
        self,
        column: Column,
        direction: impl Into<Direction>,
    ) -> QueryBuilder<Building<Joins::Database, Ordered>, Joins, Joins::Root>
    where
        Column: database::Column<JoinedModel>,
        Joins: Contains<JoinedModel, (), Index>,
    {
        self.select_impl::<Joins::Root>()
            .order_by(column, direction)
    }

    /// Adds a LIMIT clause, implicitly selecting all columns from the root
    /// model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().limit(...)`.
    pub fn limit<'a>(
        self,
        count: i64,
    ) -> QueryBuilder<Building<Joins::Database, Limited>, Joins, Joins::Root>
    where
        i64: sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().limit(count)
    }

    /// Executes the query, implicitly selecting all columns from the root
    /// model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().get(...)`.
    pub async fn get<'e, E>(self, executor: E) -> Result<Vec<Joins::Root>, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().get(executor).await
    }

    /// Retrieves the first row, implicitly selecting all columns from the root
    /// model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().first(...)`.
    pub async fn first<'e, E>(self, executor: E) -> Result<Option<Joins::Root>, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>().first(executor).await
    }

    /// Retrieves the first row or fails, implicitly selecting all columns from
    /// the root model and flushing accumulated joins.
    ///
    /// Equivalent to `.select_as::<RootModel, _>().first_or_fail(...)`.
    pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<Joins::Root, Joins::Error>
    where
        Joins::Error: From<sqlx::Error>,
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.select_impl::<Joins::Root>()
            .first_or_fail(executor)
            .await
    }

    /// Internal helper for select operations with join flushing.
    fn select_impl<SelectModel: Model>(
        self,
    ) -> QueryBuilder<Building<Joins::Database, SqlJoined<Selected>>, Joins, SelectModel> {
        // Create SELECT ... FROM
        let qb = SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
            .select(SelectModel::qualified_columns());

        // Flush accumulated joins
        let mut joins_iter = self.state.joins.into_iter();
        let (table, left, right) = joins_iter
            .next()
            .expect("Joining state guarantees at least one join");
        let mut qb = qb.join(table, left, right);

        for (table, left, right) in joins_iter {
            qb = qb.join(table, left, right);
        }

        QueryBuilder {
            state: Building { inner: qb },
            _marker: PhantomData,
        }
    }

    /// Starts a SELECT query for individual columns, flushing all accumulated
    /// joins.
    ///
    /// Accepts a tuple of column ZSTs (e.g., `(User::NAME, Order::STATUS)`).
    /// The output type is a tuple of the corresponding column value types.
    ///
    /// Transitions to [`Building<SqlJoined<Selected>>`] state.
    pub fn select<C, Models, Indices>(
        self,
        _columns: C,
    ) -> QueryBuilder<Building<Joins::Database, SqlJoined<Selected>>, Joins, C::Output>
    where
        C: ColumnSet<Joins, Models, Indices>,
    {
        // Create SELECT ... FROM
        let qb = SqlQueryBuilder::<Joins::Database, SqlInitial>::table(Joins::Root::table_name())
            .select(C::qualified_names());

        // Flush accumulated joins
        let mut joins_iter = self.state.joins.into_iter();
        let (table, left, right) = joins_iter
            .next()
            .expect("Joining state guarantees at least one join");
        let mut qb = qb.join(table, left, right);

        for (table, left, right) in joins_iter {
            qb = qb.join(table, left, right);
        }

        QueryBuilder {
            state: Building { inner: qb },
            _marker: PhantomData,
        }
    }

    /// Starts a SELECT query returning columns from a joined model.
    ///
    /// The compiler verifies that `JoinedModel` is in the join list.
    /// Transitions to [`Building<SqlJoined<Selected>>`] state.
    pub fn select_as<JoinedModel, Index>(
        self,
    ) -> QueryBuilder<Building<Joins::Database, SqlJoined<Selected>>, Joins, JoinedModel>
    where
        JoinedModel: Model,
        Joins: Contains<JoinedModel, (), Index>,
    {
        self.select_impl::<JoinedModel>()
    }
}

// ============================================================================
// Selected (via Building)
// ============================================================================

// Transitions from Selected
// NOTE: impl_join!(Selected) removed - joins now happen before select via
// Joining state
impl_where!(Selected => Filtered<Selected>);
impl_order_by!(Selected);
impl_limit!(Selected);

// Execution from Selected
impl_get!(Selected);
impl_first!(Selected);
impl_first_or_fail!(Selected);

// ============================================================================
// Joined<Selected>
// ============================================================================

// Transitions from Joined<Selected>
impl_where!(SqlJoined<Selected> => Filtered<Selected>);
impl_order_by!(SqlJoined<Selected>);
impl_limit!(SqlJoined<Selected>);

// Execution from Joined<Selected>
impl_get!(SqlJoined<Selected>);
impl_first!(SqlJoined<Selected>);
impl_first_or_fail!(SqlJoined<Selected>);

// ============================================================================
// Filtered<Selected>
// ============================================================================

// Transitions from Filtered<Selected>
impl_where!(Filtered<Selected> => Filtered<Selected>);
impl_order_by!(Filtered<Selected>);
impl_limit!(Filtered<Selected>);

// Execution from Filtered<Selected>
impl_get!(Filtered<Selected>);
impl_first!(Filtered<Selected>);
impl_first_or_fail!(Filtered<Selected>);

// ============================================================================
// Ordered
// ============================================================================

// Transitions from Ordered
impl_limit!(Ordered);

// Execution from Ordered
impl_get!(Ordered);
impl_first!(Ordered);
impl_first_or_fail!(Ordered);

// ============================================================================
// Limited
// ============================================================================

impl<Joins, Output> QueryBuilder<Building<Joins::Database, Limited>, Joins, Output>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Adds an `OFFSET` clause to the query.
    ///
    /// Transitions to [`Offsetted`] state.
    pub fn offset<'a>(
        self,
        count: i64,
    ) -> QueryBuilder<Building<Joins::Database, Offsetted>, Joins, Output>
    where
        i64: sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.offset(count),
            },
            _marker: PhantomData,
        }
    }
}

// Execution from Limited
impl_get!(Limited);

// ============================================================================
// Offsetted
// ============================================================================

// Execution from Offsetted
impl_get!(Offsetted);

// ############################################################################
// UPDATE FLOW
// ############################################################################
//
// Initial → Updating → Updated → Filtered<Updated>

// ============================================================================
// Updating
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Updating>, Joins>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Sets a column value for the UPDATE statement.
    ///
    /// Transitions to [`Updated`] state.
    pub fn set<'a, C>(
        self,
        column: C,
        value: impl Into<C::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Updated>, Joins>
    where
        C: Column<Joins::Root>,
        C::Type: 'a + sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.set(column.name(), value.into()),
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Updated
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Updated>, Joins>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Sets an additional column value for the UPDATE statement.
    ///
    /// Remains in [`Updated`] state.
    pub fn set<'a, C>(
        self,
        column: C,
        value: impl Into<C::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Updated>, Joins>
    where
        C: Column<Joins::Root>,
        C::Type: 'a + sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database>,
    {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.set(column.name(), value.into()),
            },
            _marker: PhantomData,
        }
    }
}

// Transitions from Updated
impl_where!(Updated => Filtered<Updated>);
impl_returning!(Updated);

// Execution from Updated
impl_execute!(Updated);

// ============================================================================
// Filtered<Updated>
// ============================================================================

// Transitions from Filtered<Updated>
impl_where!(Filtered<Updated> => Filtered<Updated>);
impl_returning!(Filtered<Updated>);

// Execution from Filtered<Updated>
impl_execute!(Filtered<Updated>);

// ############################################################################
// INSERT FLOW
// ############################################################################
//
// Initial → Inserting → Inserted → Conflicted → Upserted

// ============================================================================
// Inserting
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Inserting>, Joins>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Sets a column value for the INSERT statement.
    ///
    /// Transitions to [`Inserted`] state.
    #[allow(clippy::type_complexity)]
    pub fn set<'a, C>(
        self,
        column: C,
        value: impl Into<C::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Inserted<Joins::Database>>, Joins>
    where
        C: Column<Joins::Root>,
        C::Type:
            'a + sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database> + Send + 'static,
    {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.set(column.name(), value.into()),
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Inserted
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Inserted<Joins::Database>>, Joins>
where
    Joins: RootModel,
    Joins::Database: Database,
{
    /// Sets an additional column value for the INSERT statement.
    ///
    /// Remains in [`Inserted`] state.
    #[allow(clippy::type_complexity)]
    pub fn set<'a, C>(
        self,
        column: C,
        value: impl Into<C::Type>,
    ) -> QueryBuilder<Building<Joins::Database, Inserted<Joins::Database>>, Joins>
    where
        C: Column<Joins::Root>,
        C::Type:
            'a + sqlx::Encode<'a, Joins::Database> + sqlx::Type<Joins::Database> + Send + 'static,
    {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.set(column.name(), value.into()),
            },
            _marker: PhantomData,
        }
    }
}

// Transitions from Inserted
impl_returning!(Inserted<Joins::Database>);

// Execution from Inserted
impl_execute!(Inserted<Joins::Database>);

// ============================================================================
// Inserted — ON CONFLICT
// ============================================================================

impl<Joins> QueryBuilder<Building<Backend, Inserted<Backend>>, Joins>
where
    Joins: RootModel<Database = Backend>,
{
    /// Specifies the conflict target for an UPSERT operation.
    ///
    /// Uses the model's primary key columns as the conflict target.
    /// Transitions to [`Conflicted`] state.
    pub fn on_conflict(self) -> QueryBuilder<Building<Backend, Conflicted>, Joins> {
        QueryBuilder {
            state: Building {
                inner: self
                    .state
                    .inner
                    .on_conflict(Joins::Root::primary_key_columns()),
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Conflicted — DO UPDATE / DO NOTHING
// ============================================================================

impl<Joins> QueryBuilder<Building<Backend, Conflicted>, Joins>
where
    Joins: RootModel<Database = Backend>,
{
    /// Specifies that conflicting rows should be updated.
    ///
    /// Updates all non-primary-key columns with `col = EXCLUDED.col`.
    /// Transitions to [`Upserted`] state.
    pub fn do_update(self) -> QueryBuilder<Building<Backend, Upserted>, Joins> {
        let pk_columns = Joins::Root::primary_key_columns();
        let update_columns: Vec<&str> = Joins::Root::columns()
            .iter()
            .copied()
            .filter(|c| !pk_columns.contains(c))
            .collect();

        QueryBuilder {
            state: Building {
                inner: self.state.inner.do_update(&update_columns),
            },
            _marker: PhantomData,
        }
    }

    /// Specifies that conflicting rows should be ignored.
    ///
    /// Generates `DO NOTHING`. Transitions to [`Upserted`] state.
    pub fn do_nothing(self) -> QueryBuilder<Building<Backend, Upserted>, Joins> {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.do_nothing(),
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Upserted
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Upserted>, Joins>
where
    Joins: RootModel,
    Joins::Database: Database + SupportsReturning,
{
    /// Specifies the columns to return after the UPSERT.
    ///
    /// Uses all model columns. Transitions to [`Returned`] state.
    ///
    /// Only available on backends that support `RETURNING` (PostgreSQL,
    /// SQLite).
    pub fn returning(self) -> QueryBuilder<Building<Joins::Database, Returned>, Joins> {
        QueryBuilder {
            state: Building {
                inner: self.state.inner.returning(Joins::Root::columns()),
            },
            _marker: PhantomData,
        }
    }
}

// Execution from Upserted
impl_execute!(Upserted);

// ############################################################################
// TERMINAL STATE
// ############################################################################
//
// Returned is reached from INSERT/UPDATE flows via returning()

// ============================================================================
// Returned
// ============================================================================

impl<Joins> QueryBuilder<Building<Joins::Database, Returned>, Joins>
where
    Joins: RootModel,
    Joins::Database: sqlx::Database,
    Joins::Error: From<sqlx::Error>,
{
    /// Executes the query and returns all resulting rows.
    pub async fn get<'e, E>(self, executor: E) -> Result<Vec<Joins::Root>, Joins::Error>
    where
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.state.inner.get(executor).await.map_err(Into::into)
    }

    /// Executes the query and returns the first resulting row.
    pub async fn first<'e, E>(self, executor: E) -> Result<Option<Joins::Root>, Joins::Error>
    where
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.state.inner.first(executor).await.map_err(Into::into)
    }

    /// Executes the query and returns the first resulting row, or fails.
    pub async fn first_or_fail<'e, E>(self, executor: E) -> Result<Joins::Root, Joins::Error>
    where
        E: sqlx::Executor<'e, Database = Joins::Database>,
        Joins::Root:
            for<'r> sqlx::FromRow<'r, <Joins::Database as sqlx::Database>::Row> + Send + Unpin,
        <Joins::Database as sqlx::Database>::Arguments: sqlx::IntoArguments<Joins::Database>,
    {
        self.state
            .inner
            .first_or_fail(executor)
            .await
            .map_err(Into::into)
    }
}
