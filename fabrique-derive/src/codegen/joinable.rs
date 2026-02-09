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

    /// Generates bidirectional `impl Joinable` for each belongs_to relation.
    pub fn generate(self) -> TokenStream {
        let child_struct = &self.analysis.ident;

        let impls = self.analysis.belongs_to().map(|(field, relation)| {
            let parent_type = &relation.referenced_type;
            let fk_column_type = &field.column_type;
            let fk_const = &field.const_column_name;
            let parent_pk_column_type =
                Ident::new(&format!("{}IdColumn", parent_type), parent_type.span());

            let (trait_params, reverse_trait_params) = match &relation.alias {
                Some(name) => (
                    quote! { #parent_type, #name },
                    quote! { #child_struct, #name },
                ),
                None => (quote! { #parent_type }, quote! { #child_struct }),
            };

            quote! {
                impl ::fabrique::Joinable<#trait_params> for #child_struct {
                    type LeftColumn = #fk_column_type;
                    type RightColumn = #parent_pk_column_type;

                    fn left_column() -> Self::LeftColumn {
                        Self::#fk_const
                    }

                    fn right_column() -> Self::RightColumn {
                        #parent_type::ID
                    }
                }

                impl ::fabrique::Joinable<#reverse_trait_params> for #parent_type {
                    type LeftColumn = #parent_pk_column_type;
                    type RightColumn = #fk_column_type;

                    fn left_column() -> Self::LeftColumn {
                        Self::ID
                    }

                    fn right_column() -> Self::RightColumn {
                        #child_struct::#fk_const
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
    fn test_generate_bidirectional_joinable_impls() {
        // Arrange
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User", alias = "Buyer")]
                buyer_id: String,

                #[fabrique(belongs_to = "User", alias = "Seller")]
                seller_id: String,
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                impl ::fabrique::Joinable<User, Buyer> for Order {
                    type LeftColumn = OrderBuyerIdColumn;
                    type RightColumn = UserIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::BUYER_ID
                    }

                    fn right_column() -> Self::RightColumn {
                        User::ID
                    }
                }
                impl ::fabrique::Joinable<Order, Buyer> for User {
                    type LeftColumn = UserIdColumn;
                    type RightColumn = OrderBuyerIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::ID
                    }

                    fn right_column() -> Self::RightColumn {
                        Order::BUYER_ID
                    }
                }
                impl ::fabrique::Joinable<User, Seller> for Order {
                    type LeftColumn = OrderSellerIdColumn;
                    type RightColumn = UserIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::SELLER_ID
                    }

                    fn right_column() -> Self::RightColumn {
                        User::ID
                    }
                }
                impl ::fabrique::Joinable<Order, Seller> for User {
                    type LeftColumn = UserIdColumn;
                    type RightColumn = OrderSellerIdColumn;

                    fn left_column() -> Self::LeftColumn {
                        Self::ID
                    }

                    fn right_column() -> Self::RightColumn {
                        Order::SELLER_ID
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
        let codegen = JoinableCodegen::new(&analysis);

        // Act
        let result = codegen.generate();

        // Assert - empty output
        assert_eq!(result.to_string(), quote! {}.to_string());
    }
}
