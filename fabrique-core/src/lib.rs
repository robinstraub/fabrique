/// Trait for objects that can be persisted to a database or storage backend.
///
/// This trait enables factories to create and persist objects using the `create()` method.
/// The trait is designed to be flexible, allowing different connection types and error handling
/// strategies based on your specific database or persistence layer.
///
/// # Example
///
/// ```rust
/// use fabrique_core::Persistable;
///
/// struct Anvil {
///     id: u32,
///     weight: u32,
/// }
///
/// impl Persistable for Anvil {
///     type Connection = ();
///     type Error = ();
///
///     async fn create(self, _connection: &Self::Connection) -> Result<Self, Self::Error> {
///         println!("saving anvil #{} into database...", &self.id);
///         Ok(self)
///     }
/// }
/// ```
pub trait Persistable: Sized {
    /// The connection type used for database operations (e.g., database connection pool)
    type Connection: Clone;

    /// The error type returned by persistence operations
    type Error;

    /// Creates and persists this object using the provided connection.
    ///
    /// This method should handle the actual database insertion or persistence logic
    /// and return the created object with any auto-generated fields (like IDs) populated.
    fn create(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Self, Self::Error>>;

    /// Retrieves all instances of this model from the persistence layer
    ///
    /// This method should handle querying the persistence layer for all records
    /// and return them as a vector of model instances.
    fn all(
        self,
        connection: &Self::Connection,
    ) -> impl Future<Output = Result<Vec<Self>, Self::Error>>;
}
