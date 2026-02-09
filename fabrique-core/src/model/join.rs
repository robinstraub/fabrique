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

/// Trait proving that `(M, Alias)` exists within a type-level list at position
/// `Index`.
///
/// This is a marker trait with no methods. The compiler uses it to verify
/// that `M` has been joined before allowing its columns in queries.
///
/// The `Alias` parameter disambiguates multiple joins to the same model.
/// Use `()` for unnamed joins.
///
/// See [`Joined`], [`Here`], [`There`].
#[diagnostic::on_unimplemented(
    message = "model `{M}` (as `{Alias}`) is not joined in this query",
    label = "this column requires its model to be joined",
    note = "add `.join::<{M}, {Alias}>()` before using columns from this join"
)]
pub trait Contains<M, Alias, Index> {}

// Base case: `(M, Alias)` is at the head of the list.
impl<M, Alias, Tail> Contains<M, Alias, Here> for Joined<(M, Alias), Tail> {}

// Recursive case: search in tail.
impl<M, Alias, I, OtherModel, OtherAlias, Tail> Contains<M, Alias, There<I>>
    for Joined<(OtherModel, OtherAlias), Tail>
where
    Tail: Contains<M, Alias, I>,
{
}

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
impl<M: Model, Alias> RootModel for Joined<(M, Alias), ()>
where
    <M as DatabaseAware>::Database: sqlx::Database,
    <M as DatabaseAware>::Error: From<sqlx::Error>,
{
    type Root = M;
    type Database = <M as DatabaseAware>::Database;
    type Error = <M as DatabaseAware>::Error;
}

// Recursive case: dig into tail to find the root.
impl<M, Alias, Tail> RootModel for Joined<(M, Alias), Tail>
where
    Tail: RootModel,
{
    type Root = Tail::Root;
    type Database = Tail::Database;
    type Error = Tail::Error;
}
