use crate::Analysis;
use proc_macro2::TokenStream;
use quote::quote;

/// Code generator for alias pseudo-Models.
pub struct AliasCodegen<'a> {
    analysis: &'a Analysis<'a>,
}

impl<'a> AliasCodegen<'a> {
    pub fn new(analysis: &'a Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// Generates alias struct, Alias, DatabaseAware, and Model impls.
    pub fn generate(self) -> TokenStream {
        let impls = self.analysis.belongs_to().filter_map(|(_, relation)| {
            let alias = relation.alias.as_ref()?;
            let alias_snake = relation.alias_snake.as_ref()?;
            let target = &relation.referenced_type;

            Some(quote! {
                pub struct #alias;

                impl ::fabrique::Alias for #alias {
                    type Target = #target;
                    const NAME: &'static str = #alias_snake;
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
    fn test_no_alias_generates_empty() {
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User")]
                user_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = AliasCodegen::new(&analysis);

        let result = codegen.generate();

        assert_eq!(result.to_string(), quote! {}.to_string());
    }

    #[test]
    fn test_generate() {
        let input = parse_quote! {
            struct Order {
                id: String,
                #[fabrique(belongs_to = "User", alias = "Seller")]
                seller_id: String
            }
        };
        let analysis = Analysis::from(&input).unwrap();
        let codegen = AliasCodegen::new(&analysis);

        let result = codegen.generate();

        assert_eq!(
            result.to_string(),
            quote! {
                pub struct Seller;

                impl ::fabrique::Alias for Seller {
                    type Target = User;
                    const NAME: &'static str = "seller";
                }
            }
            .to_string()
        );
    }
}
