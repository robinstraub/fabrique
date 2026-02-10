#[cfg(all(feature = "postgres", feature = "mysql"))]
compile_error!("features \"postgres\" and \"mysql\" are mutually exclusive");
#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("features \"postgres\" and \"sqlite\" are mutually exclusive");
#[cfg(all(feature = "mysql", feature = "sqlite"))]
compile_error!("features \"mysql\" and \"sqlite\" are mutually exclusive");
#[cfg(not(any(feature = "postgres", feature = "sqlite", feature = "mysql")))]
compile_error!("one of the features \"postgres\", \"sqlite\", or \"mysql\" must be enabled");

pub mod database;
pub mod error;
#[cfg(feature = "testing")]
pub mod factory;
pub mod model;
pub mod relation;
pub mod sql;

#[cfg(feature = "doctests")]
#[doc(hidden)]
pub mod __private;

// Re-export for use in generated code
pub use database::Backend;
pub use database::Nil;
pub use error::Error;
#[cfg(feature = "testing")]
pub use factory::SetForeignKey;
pub use relation::Alias;
pub use relation::BelongsTo;
pub use relation::HasMany;
pub use relation::Joinable;
