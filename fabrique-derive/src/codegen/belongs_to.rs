use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for BelongsTo trait implementations.
pub struct BelongsToCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> BelongsToCodegen<'a> {
    /// Creates a new code generator for BelongsTo trait implementations.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates `impl BelongsTo<Parent>` for each field with a belongs_to
    /// relation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        let impls = self.analysis.column_fields.iter().filter_map(|field| {
            field.relation.as_ref().map(|relation| {
                let parent_type = &relation.referenced_type;
                let column_type = &field.column_type;
                let const_column_name = &field.const_column_name;

                quote! {
                    impl ::fabrique::BelongsTo<#parent_type> for #base_struct_ident {
                        type ForeignKeyColumn = #column_type;

                        fn foreign_key_column() -> Self::ForeignKeyColumn {
                            Self::#const_column_name
                        }
                    }
                }
            })
        });

        quote! {
            #(#impls)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_belongs_to_impl() {
        // Arrange
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User")]
                user_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = BelongsToCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::BelongsTo<User> for Order {
                    type ForeignKeyColumn = OrderUserIdColumn;

                    fn foreign_key_column() -> Self::ForeignKeyColumn {
                        Self::USER_ID
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_generate_multiple_belongs_to_impls() {
        // Arrange - OrderLine belongs to both Order and Product
        let input = parse_quote! {
            struct OrderLine {
                id: String,
                #[fabrique(belongs_to = "Order")]
                order_id: String,
                #[fabrique(belongs_to = "Product")]
                product_id: String,
                quantity: i32
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = BelongsToCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::BelongsTo<Order> for OrderLine {
                    type ForeignKeyColumn = OrderLineOrderIdColumn;

                    fn foreign_key_column() -> Self::ForeignKeyColumn {
                        Self::ORDER_ID
                    }
                }
                impl ::fabrique::BelongsTo<Product> for OrderLine {
                    type ForeignKeyColumn = OrderLineProductIdColumn;

                    fn foreign_key_column() -> Self::ForeignKeyColumn {
                        Self::PRODUCT_ID
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_no_belongs_to_generates_empty() {
        // Arrange - Model with no belongs_to relations
        let input = parse_quote! {
            struct User {
                id: String,
                name: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = BelongsToCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - empty output
        assert_eq!(result.to_string(), quote! {}.to_string());
    }
}
