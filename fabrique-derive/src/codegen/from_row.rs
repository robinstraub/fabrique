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
    /// the `as` attribute.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        // Generate field assignments
        let field_assignments = self.analysis.fields.iter().map(|field| {
            let field_ident = &field.ident;
            let column_name = field.ident.to_string();

            match &field.r#as {
                Some(intermediate_ty) => {
                    // Field has `as` attribute, need to convert from intermediate type using
                    // TryFrom
                    quote! {
                        #field_ident: row.try_get::<#intermediate_ty, _>(#column_name)?
                            .try_into()
                            .map_err(|e| ::sqlx::Error::Decode(Box::new(e)))?
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
}
