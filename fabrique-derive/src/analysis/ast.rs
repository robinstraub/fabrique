use darling::{FromDeriveInput, FromField};
use proc_macro2::Span;
use syn::{Ident, Type};

use crate::error::{Error, ErrorKind};

/// Attributes parsed from `#[fabrique(...)]` struct annotations.
#[derive(FromDeriveInput)]
#[darling(attributes(fabrique))]
pub struct ModelAttrs {
    /// The table name for this model
    #[darling(default)]
    pub table: Option<String>,
}

#[derive(Debug)]
pub struct Model {
    pub table_name: String,
}

impl Model {
    pub fn new(ident: &Ident, attrs: ModelAttrs) -> Self {
        Self {
            table_name: attrs
                .table
                .unwrap_or_else(|| format!("{}s", ident.to_string().to_lowercase())),
        }
    }
}

#[derive(FromField, Debug)]
#[darling(attributes(fabrique))]
pub struct ModelFieldAttrs {
    /// The field base `syn::Ident`
    ident: Option<Ident>,

    /// The field base `syn::Type`
    ty: Type,

    /// The field base `syn::Span`
    #[darling(skip, default = "Span::call_site")]
    span: Span,

    /// The database type for conversion
    #[darling(default, rename = "as")]
    r#as: Option<Type>,

    /// Wether this field is a primary key
    #[darling(default)]
    primary_key: bool,

    /// Wether this field is a soft delete key
    #[darling(default)]
    soft_delete: bool,

    /// The type referenced by this relation field
    #[darling(default)]
    relation: Option<Ident>,

    /// The key field of the referenced type
    #[darling(default)]
    referenced_key: Option<Ident>,
}

#[derive(Debug)]
pub struct Relation {
    pub name: String,
    pub referenced_type: Ident,
    pub referenced_key: Ident,
}

#[derive(Debug)]
pub struct ModelField {
    /// The field ident.
    pub ident: Ident,

    /// The field span.
    pub span: Span,

    /// The field type.
    pub ty: Type,

    /// Type marker to (de)serialize from/to the persistance layer.
    pub r#as: Option<Type>,

    /// The column name with its type marker.
    pub column: String,

    /// True if the field is primary key.
    pub primary_key: bool,

    /// The field relation, if any.
    pub relation: Option<Relation>,

    pub soft_delete: bool,
}

impl ModelField {
    pub fn try_from(attrs: ModelFieldAttrs) -> Result<Self, Error> {
        let ident = attrs.ident.ok_or_else(|| {
            Error::new(attrs.span, ErrorKind::UnsupportedDataStructureTupleStruct)
        })?;

        // Simple column name without type annotations (for runtime queries)
        let column = ident.to_string();

        let relation = match attrs.relation {
            Some(referenced_type) => {
                let field_name = ident.to_string();
                let referenced_key = attrs.referenced_key.ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        ErrorKind::MissingReferencedKey(field_name.clone()),
                    )
                })?;
                let name = field_name
                    .strip_suffix(&format!("_{}", referenced_key))
                    .unwrap_or(&field_name)
                    .to_owned();
                Some(Relation {
                    name,
                    referenced_key,
                    referenced_type,
                })
            }
            None => None,
        };

        Ok(Self {
            ident,
            span: attrs.span,
            ty: attrs.ty,
            r#as: attrs.r#as,
            column,
            primary_key: attrs.primary_key,
            relation,
            soft_delete: attrs.soft_delete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_model_field_try_from_without_ident_fails() {
        // Arrange a ModelFieldAttrs without an identifier (simulating tuple struct field)
        let attrs = ModelFieldAttrs {
            ident: None,
            ty: parse_quote!(u32),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            relation: None,
            referenced_key: None,
        };

        // Act the call to the `ModelField::try_from` method
        let result = ModelField::try_from(attrs);

        // Assert the error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(
            error.kind,
            ErrorKind::UnsupportedDataStructureTupleStruct
        ));
    }
}
