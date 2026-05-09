use crate::{
    database::Column,
    dialect::Dialect,
    model::{Model, Persist},
};

pub trait UniqueBy<M> {
    fn column_names() -> &'static [&'static str];
}

macro_rules! impl_unique_by {
    ($($C:ident),+) => {
        impl<M, $($C),+> UniqueBy<M> for ($($C,)+)
        where
            $($C: Column<M>,)+
        {
            fn column_names() -> &'static [&'static str] {
                &[$($C::NAME),+]
            }
        }
    };
}
impl_unique_by!(C0, C1);
impl_unique_by!(C0, C1, C2);
impl_unique_by!(C0, C1, C2, C3);

pub trait Upsert<DB: Dialect> {
    type Model: Model;

    fn upsert<'e, A, U>(
        self,
        executor: A,
        unique_by: U,
    ) -> impl Future<Output = Result<(), crate::Error>>
    where
        A: sqlx::Acquire<'e, Database = DB> + Send + 'e,
        U: UniqueBy<Self::Model>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockModel;
    struct ColA;
    struct ColB;
    struct ColC;

    impl Column<MockModel> for ColA {
        type Type = i32;
        type DbType = i32;
        const NAME: &'static str = "a";
        const QUALIFIED_NAME: &'static str = "mocks.a";
        fn into_db(value: i32) -> i32 {
            value
        }
    }
    impl Column<MockModel> for ColB {
        type Type = i32;
        type DbType = i32;
        const NAME: &'static str = "b";
        const QUALIFIED_NAME: &'static str = "mocks.b";
        fn into_db(value: i32) -> i32 {
            value
        }
    }
    impl Column<MockModel> for ColC {
        type Type = i32;
        type DbType = i32;
        const NAME: &'static str = "c";
        const QUALIFIED_NAME: &'static str = "mocks.c";
        fn into_db(value: i32) -> i32 {
            value
        }
    }

    #[test]
    fn test_unique_by_tuple_2() {
        let names = <(ColA, ColB)>::column_names();
        assert_eq!(names, &["a", "b"]);
    }

    #[test]
    fn test_unique_by_tuple_3() {
        let names = <(ColA, ColB, ColC)>::column_names();
        assert_eq!(names, &["a", "b", "c"]);
    }
}

impl<M, DB> Upsert<DB> for Vec<M>
where
    M: Persist<DB> + Send,
    DB: Dialect,
    <DB as sqlx::Database>::Arguments: sqlx::IntoArguments<DB>,
    for<'c> &'c mut <DB as sqlx::Database>::Connection: sqlx::Executor<'c, Database = DB>,
{
    type Model = M;

    async fn upsert<'e, A, U>(self, executor: A, _unique_by: U) -> Result<(), crate::Error>
    where
        A: sqlx::Acquire<'e, Database = DB> + Send + 'e,
        U: UniqueBy<M>,
    {
        if self.is_empty() {
            return Ok(());
        }

        let mut conn = executor.acquire().await.map_err(crate::Error::from)?;

        let mut qb = sqlx::QueryBuilder::new("INSERT INTO ");
        qb.push(M::table_name());
        qb.push(" (");
        qb.push(M::columns().join(", "));
        qb.push(") ");

        qb.push_values(self, |separated, model| {
            model.push_bind_values(separated);
        });

        let unique_cols = U::column_names();
        qb.push(DB::on_conflict_sql(unique_cols));

        let update_cols: Vec<&str> = M::columns()
            .iter()
            .copied()
            .filter(|c| !unique_cols.contains(c))
            .collect();
        qb.push(DB::do_update_sql(&update_cols));

        qb.build().execute(&mut *conn).await?;
        Ok(())
    }
}
