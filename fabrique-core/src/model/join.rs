use crate::database::DatabaseAware;
use crate::model::Model;
use std::marker::PhantomData;

/// Type-level HList for tracking joined models.
///
/// Forms a recursive cons-list structure where `()` terminates the list. For
/// example, `Joined<A, Joined<B, ()>>` represents the type-level list `[A, B]`.
///
/// This is a zero-sized type with no runtime cost.
///
/// See [`Here`], [`There`], [`Contains`], [`RootModel`].
pub struct Joined<Head, Tail>(PhantomData<(Head, Tail)>);

/// Type-level index indicating the element is found at the current position.
///
/// This is the base case when searching through a [`Joined`] list.
///
/// See [`There`], [`Contains`].
pub struct Here;

/// Type-level index indicating the element is found deeper in the list.
///
/// The parameter `I` represents the remaining path within the tail.
///
/// See [`Here`], [`Contains`].
pub struct There<I>(PhantomData<I>);

/// Trait proving that `M` exists within a type-level list at position `Index`.
///
/// This is a marker trait with no methods. The compiler uses it to verify
/// that `M` has been joined before allowing its columns in queries.
///
/// The `Index` parameter encodes the path to `M` using [`Here`] and [`There`].
///
/// See [`Joined`], [`Here`], [`There`].
#[diagnostic::on_unimplemented(
    message = "model `{M}` is not joined in this query",
    label = "this column requires its model to be joined",
    note = "add `.join::<{M}>()` before using columns from `{M}`"
)]
pub trait Contains<M, Index> {}

// Base case: `M` is at the head of the list, so the index is [`Here`].
impl<M, Tail> Contains<M, Here> for Joined<M, Tail> {}

// Recursive case: `M` is somewhere in `Tail` at index `I`, so the full index is
// [`There<I>`].
impl<M, I, Head, Tail> Contains<M, There<I>> for Joined<Head, Tail> where Tail: Contains<M, I> {}

/// Extracts the root model (innermost) from a [`Joined`] HList.
///
/// The root is the original model that started the query, found at the
/// deepest position after joins are prepended.
#[diagnostic::on_unimplemented(
    message = "cannot determine root model from `{Self}`",
    label = "expected a non-empty Joined<M, ...> list",
    note = "the Joins type must be a Joined<M, Tail> where M: Model"
)]
pub trait RootModel {
    /// The root model type at the innermost position of the HList.
    type Root: Model;

    /// The database type of the root model.
    type Database: sqlx::Database;

    /// The error type of the root model.
    type Error: From<sqlx::Error>;
}

// Base case: single element list - M is the root.
impl<M: Model> RootModel for Joined<M, ()>
where
    <M as DatabaseAware>::Database: sqlx::Database,
    <M as DatabaseAware>::Error: From<sqlx::Error>,
{
    type Root = M;
    type Database = <M as DatabaseAware>::Database;
    type Error = <M as DatabaseAware>::Error;
}

// Recursive case: dig into tail to find the root.
impl<Head, Tail> RootModel for Joined<Head, Tail>
where
    Tail: RootModel,
{
    type Root = Tail::Root;
    type Database = Tail::Database;
    type Error = Tail::Error;
}
