//! Column set trait for selecting individual columns.
//!
//! The [`ColumnSet`] trait is implemented for tuples of column ZSTs,
//! enabling type-safe selection of specific columns in queries.

use crate::database::Column;
use crate::model::join::Contains;

/// A set of columns that can be selected in a query.
///
/// Implemented for tuples of column ZSTs up to arity 16.
/// Each column must belong to a model that is present in the `Joins` HList,
/// verified at compile time via [`Contains`] bounds.
///
/// # Type Parameters
///
/// - `Joins`: The type-level HList tracking joined models in the query.
/// - `Models`: Tuple of model types that each column belongs to. Constrained by
///   the impl but inferred by the compiler — never specified manually.
/// - `Indices`: Tuple of type-level indices proving each model is in `Joins`.
///   Same pattern as [`Contains`]'s `Index` parameter — inferred, never
///   specified manually.
///
/// # Associated Types
///
/// - `Output`: The Rust type that the selected columns decode into (a tuple of
///   the column value types).
pub trait ColumnSet<Joins, Models, Indices> {
    /// The output type when these columns are fetched (tuple of column types).
    type Output;

    /// Returns the qualified column names as a static slice.
    fn qualified_names() -> &'static [&'static str];
}

/// Implements [`ColumnSet`] for a tuple of N columns.
///
/// Each invocation takes a semicolon-separated list of `C, M, I` triplets
/// where:
///
/// - `C` is the column ZST type (e.g., `ProductIdColumn`)
/// - `M` is the model the column belongs to (e.g., `Product`), inferred from
///   `C: Column<M>`
/// - `I` is the type-level index used by [`Contains`] to prove `M` is in the
///   `Joins` HList (e.g., `Here` or `There<Here>`), inferred by the compiler
///
/// The macro generates an impl that:
///
/// 1. Constrains each `C: Column<M>` — the column belongs to the model
/// 2. Constrains each `Joins: Contains<M, I>` — the model is joined
/// 3. Sets `Output` to a tuple of each column's value type (`C::Type`)
/// 4. Returns the qualified names from each column's associated const
///
/// # Example expansion
///
/// `impl_column_set!(C0, M0, I0; C1, M1, I1;)` expands to:
///
/// ```rust,ignore
/// impl<Joins, C0, M0, I0, C1, M1, I1>
///     ColumnSet<Joins, (M0, M1), (I0, I1)> for (C0, C1)
/// where
///     C0: Column<M0>,
///     Joins: Contains<M0, I0>,
///     C1: Column<M1>,
///     Joins: Contains<M1, I1>,
/// {
///     type Output = (C0::Type, C1::Type);
///
///     fn qualified_names() -> &'static [&'static str] {
///         &[C0::QUALIFIED_NAME, C1::QUALIFIED_NAME]
///     }
/// }
/// ```
macro_rules! impl_column_set {
    ($($C:ident, $M:ident, $I:ident);+;) => {
        impl<Joins, $($C, $M, $I),+> ColumnSet<Joins, ($($M,)+), ($($I,)+)> for ($($C,)+)
        where
            $(
                $C: Column<$M>,
                Joins: Contains<$M, (), $I>,
            )+
        {
            type Output = ($($C::DbType,)+);

            fn qualified_names() -> &'static [&'static str] {
                &[$($C::QUALIFIED_NAME),+]
            }
        }
    };
}

// Arity 1–16
impl_column_set!(C0, M0, I0;);
impl_column_set!(C0, M0, I0; C1, M1, I1;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10; C11, M11, I11;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10; C11, M11, I11; C12, M12, I12;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10; C11, M11, I11; C12, M12, I12; C13, M13, I13;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10; C11, M11, I11; C12, M12, I12; C13, M13, I13; C14, M14, I14;);
impl_column_set!(C0, M0, I0; C1, M1, I1; C2, M2, I2; C3, M3, I3; C4, M4, I4; C5, M5, I5; C6, M6, I6; C7, M7, I7; C8, M8, I8; C9, M9, I9; C10, M10, I10; C11, M11, I11; C12, M12, I12; C13, M13, I13; C14, M14, I14; C15, M15, I15;);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::join::{Here, Joined};

    struct MockModel;

    struct MockIdColumn;
    struct MockNameColumn;

    impl Column<MockModel> for MockIdColumn {
        type Type = i32;
        type DbType = i32;
        const NAME: &'static str = "id";
        const QUALIFIED_NAME: &'static str = "mocks.id";
        fn into_db(value: i32) -> i32 {
            value
        }
    }

    impl Column<MockModel> for MockNameColumn {
        type Type = String;
        type DbType = String;
        const NAME: &'static str = "name";
        const QUALIFIED_NAME: &'static str = "mocks.name";
        fn into_db(value: String) -> String {
            value
        }
    }

    #[test]
    fn single_column_returns_its_qualified_name() {
        type Joins = Joined<(MockModel, ()), ()>;

        let names = <(MockIdColumn,) as ColumnSet<Joins, (MockModel,), (Here,)>>::qualified_names();

        assert_eq!(names, &["mocks.id"]);
    }

    #[test]
    fn two_columns_return_both_qualified_names_in_order() {
        type Joins = Joined<(MockModel, ()), ()>;

        let names = <(MockIdColumn, MockNameColumn) as ColumnSet<
            Joins,
            (MockModel, MockModel),
            (Here, Here),
        >>::qualified_names();

        assert_eq!(names, &["mocks.id", "mocks.name"]);
    }
}
