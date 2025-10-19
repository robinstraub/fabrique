pub use fabrique_core::Persistable;
pub use fabrique_derive::Factory;

#[cfg(feature = "sqlx")]
pub use fabrique_derive::Persistable;
