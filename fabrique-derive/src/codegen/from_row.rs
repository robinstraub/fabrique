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
    /// This implementation handles automatic type conversions for fields with
    /// the `as` attribute. HasMany fields are initialized with their default
    /// value.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        // Generate column field assignments
        let column_assignments = self.analysis.column_fields.iter().map(|field| {
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

        // Generate HasMany field assignments (initialized with default)
        let has_many_assignments = self.analysis.has_many_fields.iter().map(|field| {
            let field_ident = &field.ident;
            quote! {
                #field_ident: ::fabrique::HasMany::default()
            }
        });

        let field_assignments = column_assignments.chain(has_many_assignments);

        quote! {
            impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for #base_struct_ident {
                fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
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
        // Arrange
        let input = parse_quote! { struct Anvil { id: String } };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FromRowCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for Anvil {
                    fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
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
    fn test_generate_from_row_with_has_many() {
        // Arrange - HasMany no longer requires foreign_key attribute
        let input = parse_quote! {
            struct Customer {
                id: String,
                orders: HasMany<Order>
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = FromRowCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert!(
            result
                .to_string()
                .contains("orders : :: fabrique :: HasMany :: default ()")
        );
    }
}
