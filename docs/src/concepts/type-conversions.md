# Type Conversions

Fabrique models map Rust types to database columns. While primitive types like
`String`, `i32`, and `Uuid` map directly, you often need custom types — enums,
newtypes, or domain-specific wrappers — that don't have a direct database
representation.

## The Problem

Consider an order status. In Rust, you want type safety:

```rust,ignore
pub enum OrderStatus {
    Pending,
    Shipped,
    Delivered,
}
```

But databases don't have Rust enums. You need to store this as a `String` or
`i32` column. Type conversions bridge this gap.

## The `as` Attribute

The `#[fabrique(as = "DatabaseType")]` attribute tells Fabrique how to store a
field:

```rust,ignore
#[derive(Model)]
pub struct Order {
    pub id: Uuid,

    #[fabrique(as = "String")]
    pub status: OrderStatus,  // Stored as TEXT in database
}
```

Fabrique will:

- Convert `OrderStatus` → `String` when writing to the database
- Convert `String` → `OrderStatus` when reading from the database

## Trait Requirements

For conversions to work, your type must implement two traits:

| Direction | Trait | Purpose |
|-----------|-------|---------|
| Reading | `TryFrom<DatabaseType>` | Database → Rust (may fail) |
| Writing | `From<YourType> for DatabaseType` | Rust → Database (infallible) |

Reading uses `TryFrom` because database values might be invalid (e.g., an
unknown status string). Writing uses `From` because your Rust type should always
produce a valid database value.

Note: Implementing `From<T> for U` automatically provides `TryInto<U> for T`,
which Fabrique uses internally.

## Common Use Cases

**Enums as strings** — Store enum variants as human-readable strings. Good for
debugging and when values might be queried directly in SQL.

**Enums as integers** — Store enum variants as integers for compact storage.
Useful when storage size matters or when integrating with systems that expect
numeric codes.

**Newtypes** — Wrap primitive types for type safety
(e.g., `CustomerId(Uuid)` vs `ProductId(Uuid)`). Prevents accidentally passing
the wrong ID type to a function.

## Query Behavior

When a field uses `as`, query parameters use the database type:

```rust,ignore
// Use String (database type), not OrderStatus
Order::query()
    .select()
    .r#where(Order::STATUS, "=", "pending".to_string())
    .get(pool)
    .await?;
```

The generated column constant (`Order::STATUS`) is typed as the database type.

## Error Handling

When conversion fails during a read, Fabrique returns `Error::Conversion` with
detailed context: which field failed, what value was encountered, and the error
message from your `TryFrom` implementation. See
[Error Handling](error-handling.md) for details.

## Summary

- Use `#[fabrique(as = "Type")]` to map custom types to database columns
- Implement `TryFrom<DatabaseType>` for reading (fallible)
- Implement `From<YourType> for DatabaseType` for writing (infallible)
- Query parameters use the database type
