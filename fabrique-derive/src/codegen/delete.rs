use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

pub struct DeleteCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> DeleteCodegen<'a> {
    /// Creates a new code generator for the `fabrique::Delete` trait
    /// implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `fabrique::Delete` trait implementation as a token stream.
    ///
    /// The implementation delegates to either `SoftDelete` or `HardDelete`
    /// based on whether the model has a soft delete field.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;
        let fn_delete = self.generate_fn_delete();
        let fn_destroy = self.generate_fn_destroy();

        let has_soft_delete = self
            .analysis
            .column_fields
            .iter()
            .any(|field| field.soft_delete);

        // PK-field Encode/Type bounds (needed by the delegated trait)
        let pk_bounds = self
            .analysis
            .column_fields
            .iter()
            .filter(|f| f.primary_key)
            .map(|f| {
                let db_ty = f.r#as.as_ref().unwrap_or(&f.ty);
                quote! {
                    for<'q> #db_ty: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>
                }
            });

        // When delegating to SoftDelete, we need its extra bounds
        let soft_delete_bounds = if has_soft_delete {
            quote! {
                usize: ::sqlx::ColumnIndex<<DB as ::sqlx::Database>::Row>,
                for<'d> bool: ::sqlx::decode::Decode<'d, DB> + ::sqlx::Type<DB>,
            }
        } else {
            quote! {}
        };

        quote! {
            impl<DB: ::fabrique::Dialect> ::fabrique::Delete<DB> for #base_struct_ident
            where
                #(#pk_bounds,)*
                for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                #soft_delete_bounds
            {
                #fn_delete
                #fn_destroy
            }
        }
    }

    fn generate_fn_delete(&self) -> TokenStream {
        let soft_delete = self
            .analysis
            .column_fields
            .iter()
            .find(|field| field.soft_delete);

        match soft_delete {
            Some(_) => quote! {
                fn delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                where
                    A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                {
                    <Self as ::fabrique::SoftDelete<DB>>::soft_delete(self, executor)
                }
            },
            None => quote! {
                fn delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                where
                    A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                {
                    <Self as ::fabrique::HardDelete<DB>>::hard_delete(self, executor)
                }
            },
        }
    }

    fn generate_fn_destroy(&self) -> TokenStream {
        let soft_delete = self
            .analysis
            .column_fields
            .iter()
            .find(|field| field.soft_delete);

        match soft_delete {
            Some(_) => quote! {
                fn destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                where
                    A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                {
                    <Self as ::fabrique::SoftDelete<DB>>::soft_destroy(executor, id)
                }
            },
            None => quote! {
                fn destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                where
                    A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                {
                    <Self as ::fabrique::HardDelete<DB>>::hard_destroy(executor, id)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_basic_struct_delegates_to_hard_delete() {
        // Arrange the codegen
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::Delete<DB> for Anvil
                where
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                {
                    fn delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        <Self as ::fabrique::HardDelete<DB>>::hard_delete(self, executor)
                    }

                    fn destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        <Self as ::fabrique::HardDelete<DB>>::hard_destroy(executor, id)
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_soft_delete_struct_delegates_to_soft_delete() {
        // Arrange the codegen
        let input = parse_quote! {
            struct Anvil {
                id: String,

                #[fabrique(soft_delete)]
                deleted_at: Option<chrono::DateTime<chrono::Utc>>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = DeleteCodegen::new(&analysis);

        // Act the call to the generate method
        let result = codegen.generate();

        // Assert the result
        assert_eq!(
            result.to_string(),
            quote! {
                impl<DB: ::fabrique::Dialect> ::fabrique::Delete<DB> for Anvil
                where
                    for<'q> String: ::sqlx::Encode<'q, DB> + ::sqlx::Type<DB>,
                    for<'c> &'c mut <DB as ::sqlx::Database>::Connection: ::sqlx::Executor<'c, Database = DB>,
                    <DB as ::sqlx::Database>::Arguments: ::sqlx::IntoArguments<DB>,
                    usize: ::sqlx::ColumnIndex<<DB as ::sqlx::Database>::Row>,
                    for<'d> bool: ::sqlx::decode::Decode<'d, DB> + ::sqlx::Type<DB>,
                {
                    fn delete<'e, A>(self, executor: A) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        <Self as ::fabrique::SoftDelete<DB>>::soft_delete(self, executor)
                    }

                    fn destroy<'e, A>(executor: A, id: Self::PrimaryKey) -> impl ::std::future::Future<Output = Result<(), ::fabrique::Error>> + Send + 'e
                    where
                        A: ::sqlx::Acquire<'e, Database = DB> + Send + 'e,
                    {
                        <Self as ::fabrique::SoftDelete<DB>>::soft_destroy(executor, id)
                    }
                }
            }
            .to_string()
        );
    }
}
