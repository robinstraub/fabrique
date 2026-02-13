use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for FromRow trait implementation.
pub struct FromRowCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> FromRowCodegen<'a> {
    /// Creates a new code generator for FromRow trait implementation.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates the `FromRow` trait implementation.
    ///
    /// This implementation is generic over any sqlx Row type, allowing models
    /// to work with any database backend that has the `Dialect` trait
    /// implemented.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        // Generate where-clause bounds for each field's decode type
        let field_bounds = self.analysis.column_fields.iter().map(|field| {
            let decode_ty = field.r#as.as_ref().unwrap_or(&field.ty);
            quote! {
                #decode_ty: ::sqlx::decode::Decode<'r, R::Database> + ::sqlx::Type<R::Database>
            }
        });

        // Generate column field assignments
        let field_assignments = self.analysis.column_fields.iter().map(|field| {
            let field_ident = &field.ident;
            let column_name = field.ident.to_string();

            match &field.r#as {
                Some(db_ty) => {
                    // Field has `as` attribute, need to convert from intermediate type
                    let rust_ty = &field.ty;
                    quote! {
                        #field_ident: {
                            let db_value: #db_ty = row.try_get(#column_name)?;
                            let value_str = format!("{:?}", &db_value);
                            <#rust_ty>::try_from(db_value).map_err(|e| {
                                ::sqlx::Error::Decode(Box::new(::fabrique::Error::Conversion {
                                    field: #column_name.to_string(),
                                    from: stringify!(#db_ty),
                                    to: stringify!(#rust_ty),
                                    value: value_str,
                                    reason: e.to_string(),
                                    direction: ::fabrique::error::ConversionDirection::FromDb,
                                }))
                            })?
                        }
                    }
                }
                None => {
                    // No conversion needed, read directly
                    quote! {
                        #field_ident: row.try_get(#column_name)?
                    }
                }
            }
        });

        quote! {
            impl<'r, R: ::sqlx::Row> ::sqlx::FromRow<'r, R> for #base_struct_ident
            where
                &'r str: ::sqlx::ColumnIndex<R>,
                #(#field_bounds,)*
            {
                fn from_row(row: &'r R) -> ::sqlx::Result<Self> {
                    use ::sqlx::Row;
                    Ok(Self {
                        #(#field_assignments),*
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_from_row() {
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FromRowCodegen::new(&analysis);
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl<'r, R: ::sqlx::Row> ::sqlx::FromRow<'r, R> for Anvil
                where
                    &'r str: ::sqlx::ColumnIndex<R>,
                    String: ::sqlx::decode::Decode<'r, R::Database> + ::sqlx::Type<R::Database>,
                {
                    fn from_row(row: &'r R) -> ::sqlx::Result<Self> {
                        use ::sqlx::Row;
                        Ok(Self {
                            id: row.try_get("id")?
                        })
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_from_row_with_type_conversion() {
        let input = parse_quote! {
            struct Account {
                id: String,
                #[fabrique(as = "String")]
                status: Status,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FromRowCodegen::new(&analysis);
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl<'r, R: ::sqlx::Row> ::sqlx::FromRow<'r, R> for Account
                where
                    &'r str: ::sqlx::ColumnIndex<R>,
                    String: ::sqlx::decode::Decode<'r, R::Database> + ::sqlx::Type<R::Database>,
                    String: ::sqlx::decode::Decode<'r, R::Database> + ::sqlx::Type<R::Database>,
                {
                    fn from_row(row: &'r R) -> ::sqlx::Result<Self> {
                        use ::sqlx::Row;
                        Ok(Self {
                            id: row.try_get("id")?,
                            status: {
                                let db_value: String = row.try_get("status")?;
                                let value_str = format!("{:?}", &db_value);
                                <Status>::try_from(db_value).map_err(|e| {
                                    ::sqlx::Error::Decode(Box::new(::fabrique::Error::Conversion {
                                        field: "status".to_string(),
                                        from: stringify!(String),
                                        to: stringify!(Status),
                                        value: value_str,
                                        reason: e.to_string(),
                                        direction: ::fabrique::error::ConversionDirection::FromDb,
                                    }))
                                })?
                            }
                        })
                    }
                }
            }
            .to_string()
        );
    }
}
