#[cfg(not(any(feature = "postgres", feature = "sqlite", feature = "mysql")))]
compile_error!("one of the features \"postgres\", \"sqlite\", or \"mysql\" must be enabled");

#[cfg(feature = "testing")]
#[doc(hidden)]
pub mod __private;
pub mod database;
pub mod dialect;
pub mod error;
#[cfg(feature = "testing")]
pub mod factory;
pub mod model;
pub mod relation;
pub mod sql;

// Re-export for use in generated code
pub use database::Nil;
pub use dialect::Dialect;
pub use error::Error;
#[cfg(feature = "testing")]
pub use factory::DeferredFactory;
#[cfg(feature = "testing")]
pub use factory::SetForeignKey;
pub use relation::Alias;
pub use relation::BelongsTo;
pub use relation::Joinable;
