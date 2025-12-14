//! Model traits for database operations.
//!
//! This module provides the core traits that define model behavior in Fabrique:
//! - `Model`: Defines model identity with primary key and table name
//! - `Query`: Provides query building and retrieval operations
//! - `Persist`: Handles model creation and deletion
//! - `SoftDelete`: Enables soft deletion functionality
//! - `HardDelete`: Provides permanent deletion operations

pub use fabrique_core::model::*;
pub use fabrique_derive::Model;
