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

    /// Generates `impl BelongsTo<Parent>` for each unique belongs_to relation.
    ///
    /// Only generates the trait implementation when there is exactly ONE field
    /// referencing a given parent type. When multiple fields reference the same
    /// parent (e.g., `sender_id` and `recipient_id` both referencing `User`),
    /// no trait is generated to avoid duplicate implementations.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        let impls = self
            .analysis
            .belongs_to_non_ambiguous()
            .map(|(field, relation)| {
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
    fn test_generate_multiple_belongs_to_impls_to_different_parents() {
        // Arrange - OrderLine belongs to both Order and Product (different parents)
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
        let result = codegen.generate().to_string();

        // Assert - both impls should be generated (order doesn't matter due to HashMap)
        assert!(
            result.contains("impl :: fabrique :: BelongsTo < Order > for OrderLine"),
            "Should contain BelongsTo<Order> impl"
        );
        assert!(
            result.contains("impl :: fabrique :: BelongsTo < Product > for OrderLine"),
            "Should contain BelongsTo<Product> impl"
        );
    }

    #[test]
    fn test_multiple_belongs_to_same_parent_generates_nothing() {
        // Arrange - Message has two belongs_to to the same parent (User)
        let input = parse_quote! {
            struct Message {
                id: String,
                #[fabrique(belongs_to = "User")]
                sender_id: String,
                #[fabrique(belongs_to = "User")]
                recipient_id: String,
                content: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = BelongsToCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - no BelongsTo impl should be generated for ambiguous relationships
        assert_eq!(
            result.to_string(),
            quote! {}.to_string(),
            "Should not generate BelongsTo trait when multiple fields reference same parent"
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
