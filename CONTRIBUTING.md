# Contributing

## Testing

Follow the **Arrange / Act / Assert** pattern:

```rust
#[fabrique::test]
async fn products_in_stock_are_returned(pool: Pool<Backend>) {
    // Arrange two products, one in stock and one out of stock
    Product::factory().in_stock(true).create(&pool).await.unwrap();
    Product::factory().in_stock(false).create(&pool).await.unwrap();

    // Act the retrieval of all products
    let products = Product::all(&pool).await.unwrap();

    // Assert only the in-stock product is returned
    assert_eq!(products.len(), 1);
}
```

All testable code must be covered by tests.

`#[coverage(off)]` is strictly reserved for proc-macro entry points
(`#[proc_macro_derive]` and `#[proc_macro_attribute]` functions). These
functions must remain minimal and delegate to internal functions that can be
covered by tests.

Use [trybuild](https://docs.rs/trybuild) UI tests in the `ui-tests`
crate for compile-time error assertions.
