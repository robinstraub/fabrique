use darling::{FromDeriveInput, FromField};
use heck::{ToPascalCase, ToSnakeCase};
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
}

#[derive(Debug)]
pub struct Relation {
    pub name: String,
    pub referenced_type: Ident,
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

    /// The column type (e.g. AnvilId).
    pub column_type: Ident,

    /// The const column name (e.g. NAME).
    pub const_column_name: Ident,

    /// True if the field is primary key.
    pub primary_key: bool,

    /// The field relation, if any.
    pub relation: Option<Relation>,

    pub soft_delete: bool,
}

impl ModelField {
    pub fn try_from(attrs: ModelFieldAttrs, struct_name: String) -> Result<Self, Error> {
        let ident = attrs.ident.ok_or_else(|| {
            Error::new(attrs.span, ErrorKind::UnsupportedDataStructureTupleStruct)
        })?;

        // Simple column name without type annotations (for runtime queries)
        let column = ident.to_string();

        let const_column_name = Ident::new(&ident.to_string().to_uppercase(), ident.span());

        let pascal_case_field = ident.to_string().to_pascal_case();

        let type_name = format!("{}{}Column", struct_name, pascal_case_field);
        let column_type = syn::Ident::new(&type_name, ident.span());

        let relation = attrs.relation.map(|referenced_type| Relation {
            name: referenced_type.to_string().to_snake_case(),
            referenced_type,
        });

        Ok(Self {
            ident,
            span: attrs.span,
            ty: attrs.ty,
            r#as: attrs.r#as,
            column,
            column_type,
            const_column_name,
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
        // Arrange a ModelFieldAttrs without an identifier (simulating tuple struct
        // field)
        let attrs = ModelFieldAttrs {
            ident: None,
            ty: parse_quote!(u32),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            relation: None,
        };

        // Act the call to the `ModelField::try_from` method
        let result = ModelField::try_from(attrs, "Anvil".to_owned());

        // Assert the error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(
            error.kind,
            ErrorKind::UnsupportedDataStructureTupleStruct
        ));
    }
}
