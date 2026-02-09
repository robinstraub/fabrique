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

    /// Generates `impl BelongsTo<T>` for each belongs_to relation.
    pub fn generate(self) -> TokenStream {
        let base_struct_ident = &self.analysis.ident;

        let impls = self.analysis.belongs_to().map(|(field, relation)| {
            let parent = &relation.referenced_type;
            let column_type = &field.column_type;
            let const_column_name = &field.const_column_name;

            let trait_params = match &relation.alias {
                Some(alias) => quote! { #parent, #alias },
                None => quote! { #parent },
            };

            quote! {
                impl ::fabrique::BelongsTo<#trait_params> for #base_struct_ident {
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
    fn test_aliased_belongs_to_generates_belongs_to_alias() {
        // Arrange - Message has two belongs_to to the same parent, disambiguated with
        // aliases
        let input = parse_quote! {
            struct Message {
                id: String,
                #[fabrique(belongs_to = "User", alias = "Sender")]
                sender_id: String,
                #[fabrique(belongs_to = "User", alias = "Recipient")]
                recipient_id: String,
                content: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = BelongsToCodegen::new(&analysis);

        // Act
        let result = codegen.generate().to_string();

        // Assert - BelongsTo<Parent, Alias> for each aliased relation
        assert!(
            result.contains("impl :: fabrique :: BelongsTo < User , Sender > for Message"),
            "Should generate BelongsTo<User, Sender>"
        );
        assert!(
            result.contains("impl :: fabrique :: BelongsTo < User , Recipient > for Message"),
            "Should generate BelongsTo<User, Recipient>"
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
