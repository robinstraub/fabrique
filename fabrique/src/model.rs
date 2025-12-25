//! # Fabrique Models
//!
//! Model traits and conventions for database operations.
//!
//! ## Introduction
//!
//! When using Fabrique, each database table has a corresponding "Model" that is
//! used to interact with that table. Models allow you to retrieve, insert,
//! update, and delete records from the database table.
//!
//! ## Model Conventions
//!
//! Let's examine a basic model class and discuss some of Fabrique's key
//! conventions:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! #[derive(Model)]
//! pub struct Anvil {
//!     id: uuid::Uuid,
//!     // --snip--
//! }
//! ```
//!
//! ### Table Names
//!
//! After glancing at the example above, you may have noticed that we did not
//! tell Fabrique which database table corresponds to our `Anvil` model. By
//! convention, the "snake case", plural name of the struct will be used as the
//! table name unless another name is explicitly specified. So, in this case,
//! Fabrique will assume the `Anvil` model stores records in the `anvils` table,
//! while a `RocketShoe` model would store records in a `rocket_shoes` table.
//!
//! If your model's corresponding database table does not fit this convention,
//! you may manually specify the model's table name by defining a table
//! attribute on the model:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! #[derive(Model)]
//! #[fabrique(table = "acme_anvil")]
//! pub struct Anvil {
//!     id: uuid::Uuid,
//!     // --snip--
//! }
//! ```
//!
//! ### Primary Keys
//!
//! Fabrique will also assume that each model has a primary key column named
//! `id`. Otherwise, you must annotate a field with a `fabrique(primary_key)`
//! attribute to specify which field serves as your model's primary key:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//!
//! #[derive(Model)]
//! pub struct Anvil {
//!     #[fabrique(primary_key)]
//!     anvil_id: uuid::Uuid,
//! }
//! ```
//!
//! ### "Composite" Primary Keys
//!
//! Fabrique requires each model to have at least one uniquely identifying "ID"
//! that can serve as its primary key. Unlike some ORMs, "composite" primary
//! keys are fully supported by Fabrique models through the use of multiple
//! `#[fabrique(primary_key)]` attributes:
//!
//! ```rust,no_run
//! # use fabrique::{Model, Persist};
//! # use uuid::Uuid;
//!
//! #[derive(Model)]
//! pub struct Anvil {
//!     #[fabrique(primary_key)]
//!     production_batch: String,
//!
//!     #[fabrique(primary_key)]
//!     sequence_number: i32,
//!
//!     // --snip--
//! }
//! ```
//!
//! ## Retrieving Models
//!
//! Once you have created a model and created its corresponding database table
//! (using your preferred migration tool or SQL), you are ready to start
//! retrieving data from your database. You can think of each Fabrique model as
//! a powerful query builder allowing you to fluently query the database table
//! associated with the model. The model's [`all`][Query::all] method will
//! retrieve all of the records from the model's associated database table:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! let anvils: Vec<Anvil> = Anvil::all(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Building Queries
//!
//! The Fabrique [`all`][Query::all] method will return all of the results in
//! the model's table. However, since each Fabrique model serves as a query
//! builder, you may add additional constraints to queries and then invoke the
//! get method to retrieve the results:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     weight: i32,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! let anvils: Vec<Anvil> = Anvil::query()
//!     .select()
//!     .r#where(Anvil::WEIGHT, ">=", 42)
//!     .get(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Inserting and Updating Models
//!
//! ### Inserts
//!
//! Of course, when using Fabrique, we don't only need to retrieve models from
//! the database. We also need to insert new records. Thankfully, Fabrique
//! makes it simple. To insert a new record into the database, you should
//! instantiate a new model instance and set attributes on the model. Then,
//! call the [`save`][Persist::save] method on the model instance:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     name: String,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! let anvil = Anvil {
//!     id: uuid::Uuid::new_v4(),
//!     name: "Heavy Duty".to_string(),
//! };
//!
//! anvil.save(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! When we call the [`save`][Persist::save] method, a record will be inserted
//! into the database. If a record with the same primary key already exists,
//! it will be updated instead.
//!
//! Alternatively, you may use the [`create`][Persist::create] method to insert
//! a new model. The [`create`][Persist::create] method will fail if a record
//! with the same primary key already exists:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
//! anvil.create(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Updates
//!
//! The [`save`][Persist::save] method may also be used to update models that
//! already exist in the database. To update a model, you should retrieve it
//! and set any attributes you wish to update. Then, you should call the
//! model's [`save`][Persist::save] method:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     name: String,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, mut anvil: Anvil) -> Result<(), fabrique::Error> {
//! anvil.name = "Super Heavy Duty".to_string();
//!
//! anvil.save(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Mass Updates
//!
//! Updates can also be performed against models that match a given query. In
//! this example, all anvils that have a weight less than 50 will be updated to
//! have a weight of 100:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! #     weight: i32,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>) -> Result<(), fabrique::Error> {
//! Anvil::update()
//!     .set(Anvil::WEIGHT, 100)
//!     .r#where(Anvil::WEIGHT, "<", 50)
//!     .execute(&connection)
//!     .await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Deleting Models
//!
//! To delete a model, you may call the [`delete`][Persist::delete] method on
//! the model instance:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
//! anvil.delete(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Deleting an Existing Model by its Primary Key
//!
//! In the example above, we are retrieving the model from the database before
//! calling the [`delete`][Persist::delete] method. However, if you know the
//! primary key of the model, you may delete the model without explicitly
//! retrieving it by calling the [`destroy`][Persist::destroy] method.
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//! #
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #     id: uuid::Uuid,
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, id: Uuid) -> Result<(), fabrique::Error> {
//! Anvil::destroy(&connection, id).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Soft Deleting
//!
//! In addition to actually removing records from your database, Fabrique can
//! also "soft delete" models. When models are soft deleted, they are not
//! actually removed from your database. Instead, a soft delete attribute is set
//! on the model indicating the date and time at which the model was "deleted".
//! To enable soft deletes for a model, annotate an attribute with the
//! `#[fabrique(soft_delete)]` annotation. The attribute type must be an
//! optional date, such as `Option<chrono::DateTime<chrono::Utc>>`.
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use uuid::Uuid;
//! use chrono::{DateTime, Utc};
//!
//! #[derive(Factory, Model)]
//! pub struct Anvil {
//!     id: uuid::Uuid,
//!
//!     #[fabrique(soft_delete)]
//!     deleted_at: Option<DateTime<Utc>>
//! }
//! ```
//!
//! Now, when you call the delete method on the model, the `deleted_at` column
//! will be set to the current date and time. However, the model's database
//! record will be left in the table. When querying a model that uses soft
//! deletes, the soft deleted models will automatically be excluded from all
//! query results.
//!
//! To determine if a given model instance has been soft deleted, you may use
//! the [`trashed`][SoftDelete::trashed] method:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
//! if anvil.trashed(&connection).await? {
//!     // --snip--
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! ### Restoring Soft Deleted Models
//!
//! Sometimes you may wish to "un-delete" a soft deleted model. To restore a
//! soft deleted model, you may call the [`restore`][SoftDelete::restore] method
//! on a model instance. The restore method will set the model's soft delete
//! column to null:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
//! anvil.restore(&connection).await?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Permanently Deleting Models
//!
//! Sometimes you may need to truly remove a model from your database. You may
//! use the [`hard_delete`][HardDelete::hard_delete] method to permanently
//! remove a soft deleted model from the database table:
//!
//! ```rust,no_run
//! # use fabrique::prelude::*;
//! # use sqlx::{Pool, Postgres};
//! # use uuid::Uuid;
//! # use chrono::{DateTime, Utc};
//!
//! # #[derive(Factory, Model)]
//! # pub struct Anvil {
//! #    id: uuid::Uuid,
//! #
//! #    #[fabrique(soft_delete)]
//! #    deleted_at: Option<DateTime<Utc>>
//! # }
//! #
//! # async fn example(connection: Pool<Postgres>, anvil: Anvil) -> Result<(), fabrique::Error> {
//! anvil.hard_delete(&connection).await?;
//! #     Ok(())
//! # }
//! ```

pub use fabrique_core::model::*;
pub use fabrique_derive::Model;
