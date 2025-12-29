use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Code generator for Joinable trait implementations.
pub struct JoinableCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> JoinableCodegen<'a> {
    /// Creates a new code generator for Joinable trait implementations.
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates bidirectional `impl Joinable` for each unique belongs_to
    /// relation.
    ///
    /// For each `belongs_to` relationship, generates two implementations:
    /// - Child → Parent: `impl Joinable<Parent> for Child`
    /// - Parent → Child: `impl Joinable<Child> for Parent`
    pub fn generate(self) -> TokenStream {
        let child_struct = &self.analysis.ident;

        let impls = self
            .analysis
            .belongs_to_non_ambiguous()
            .map(|(field, relation)| {
                let parent_type = &relation.referenced_type;
                let fk_column_type = &field.column_type;
                let fk_const = &field.const_column_name;

                // Construct parent's PK column type (assumes `id` field convention)
                let parent_pk_column_type =
                    Ident::new(&format!("{}IdColumn", parent_type), parent_type.span());

                // Child → Parent: Order: Joinable<User>
                // Left = FK column (orders.user_id), Right = PK column (users.id)
                let child_to_parent = quote! {
                    impl ::fabrique::Joinable<#parent_type> for #child_struct {
                        type LeftColumn = #fk_column_type;
                        type RightColumn = #parent_pk_column_type;

                        fn left_column() -> Self::LeftColumn {
                            Self::#fk_const
                        }

                        fn right_column() -> Self::RightColumn {
                            #parent_type::ID
                        }
                    }
                };

                // Parent → Child: User: Joinable<Order>
                // Left = PK column (users.id), Right = FK column (orders.user_id)
                let parent_to_child = quote! {
                    impl ::fabrique::Joinable<#child_struct> for #parent_type {
                        type LeftColumn = #parent_pk_column_type;
                        type RightColumn = #fk_column_type;

                        fn left_column() -> Self::LeftColumn {
                            Self::ID
                        }

                        fn right_column() -> Self::RightColumn {
                            #child_struct::#fk_const
                        }
                    }
                };

                quote! {
                    #child_to_parent
                    #parent_to_child
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
    fn test_generate_bidirectional_joinable_impls() {
        // Arrange
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User")]
                user_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert
        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Joinable<User> for Order {
                    type LeftColumn = OrderUserIdColumn;
                    type RightColumn = UserIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::USER_ID
                    }

                    fn right_column() -> Self::RightColumn {
                        User::ID
                    }
                }
                impl ::fabrique::Joinable<Order> for User {
                    type LeftColumn = UserIdColumn;
                    type RightColumn = OrderUserIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::ID
                    }

                    fn right_column() -> Self::RightColumn {
                        Order::USER_ID
                    }
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_multiple_belongs_to_different_parents() {
        // Arrange - OrderLine belongs to both Order and Product
        let input = parse_quote! {
            struct OrderLine {
                id: String,
                #[fabrique(belongs_to = "Order")]
                order_id: String,
                #[fabrique(belongs_to = "Product")]
                product_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate().to_string();

        // Assert - both directions for both relationships
        assert!(result.contains("impl :: fabrique :: Joinable < Order > for OrderLine"));
        assert!(result.contains("impl :: fabrique :: Joinable < OrderLine > for Order"));
        assert!(result.contains("impl :: fabrique :: Joinable < Product > for OrderLine"));
        assert!(result.contains("impl :: fabrique :: Joinable < OrderLine > for Product"));
    }

    #[test]
    fn test_ambiguous_belongs_to_generates_nothing() {
        // Arrange - Message has two belongs_to to the same parent (User)
        let input = parse_quote! {
            struct Message {
                id: String,
                #[fabrique(belongs_to = "User")]
                sender_id: String,
                #[fabrique(belongs_to = "User")]
                recipient_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - no Joinable impl for ambiguous relationships
        assert_eq!(result.to_string(), quote! {}.to_string());
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
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - empty output
        assert_eq!(result.to_string(), quote! {}.to_string());
    }
}
