use darling::{FromDeriveInput, FromField};
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::Span;
use syn::{Ident, Path, Type};

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

    /// The type referenced by this relation field (belongs_to)
    #[darling(default)]
    relation: Option<Ident>,

    /// The foreign key path for HasMany relationships (e.g.,
    /// `Order::CUSTOMER_ID`)
    #[darling(default)]
    foreign_key: Option<Path>,
}

#[derive(Debug)]
pub struct Relation {
    pub name: String,
    pub referenced_type: Ident,
}

/// A database column field.
#[derive(Debug)]
pub struct ColumnField {
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

    /// The field relation, if any (belongs_to).
    pub relation: Option<Relation>,

    pub soft_delete: bool,
}

/// A HasMany relationship field (not stored in database).
#[derive(Debug)]
pub struct HasManyField {
    /// The field ident (e.g., `orders`).
    pub ident: Ident,

    /// The field span.
    pub span: Span,

    /// The target type (e.g., `Order` from `HasMany<Order>`).
    pub target_type: Ident,

    /// The foreign key path (e.g., `Order::CUSTOMER_ID`).
    pub foreign_key: Path,
}

/// Result of parsing a field - either a column or a HasMany relation.
#[derive(Debug)]
pub enum ParsedField {
    Column(ColumnField),
    HasMany(HasManyField),
}

impl ParsedField {
    pub fn try_from(attrs: ModelFieldAttrs, struct_name: String) -> Result<Self, Error> {
        let ident = attrs.ident.ok_or_else(|| {
            Error::new(attrs.span, ErrorKind::UnsupportedDataStructureTupleStruct)
        })?;

        // Check if this is a HasMany field
        if let Some((outer, target)) = Self::parse_parameterized_type(&attrs.ty) {
            if outer == "HasMany" {
                let foreign_key = attrs.foreign_key.ok_or_else(|| {
                    Error::new(ident.span(), ErrorKind::MissingForeignKeyAttribute)
                })?;

                return Ok(ParsedField::HasMany(HasManyField {
                    ident,
                    span: attrs.span,
                    target_type: target.clone(),
                    foreign_key,
                }));
            }
        }

        // Regular column field
        let column = ident.to_string();
        let const_column_name = Ident::new(&ident.to_string().to_uppercase(), ident.span());
        let pascal_case_field = ident.to_string().to_pascal_case();
        let type_name = format!("{}{}Column", struct_name, pascal_case_field);
        let column_type = syn::Ident::new(&type_name, ident.span());

        let relation = attrs.relation.map(|referenced_type| Relation {
            name: referenced_type.to_string().to_snake_case(),
            referenced_type,
        });

        Ok(ParsedField::Column(ColumnField {
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
        }))
    }

    /// Parse a parameterized type like `HasMany<Order>` into (outer,
    /// parameter).
    fn parse_parameterized_type(ty: &Type) -> Option<(&Ident, &Ident)> {
        let Type::Path(type_path) = ty else {
            return None;
        };
        let segment = type_path.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return None;
        };
        let syn::GenericArgument::Type(Type::Path(inner_path)) = args.args.first()? else {
            return None;
        };
        let parameter = &inner_path.path.segments.last()?.ident;

        Some((&segment.ident, parameter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parsed_field_try_from_without_ident_fails() {
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
            foreign_key: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Anvil".to_owned());

        // Assert the error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(
            error.kind,
            ErrorKind::UnsupportedDataStructureTupleStruct
        ));
    }

    #[test]
    fn test_has_many_field_without_foreign_key_fails() {
        // Arrange a HasMany field without foreign_key attribute
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(orders)),
            ty: parse_quote!(HasMany<Order>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            relation: None,
            foreign_key: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error.kind, ErrorKind::MissingForeignKeyAttribute));
    }

    #[test]
    fn test_has_many_field_with_foreign_key_succeeds() {
        // Arrange a HasMany field with foreign_key attribute
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(orders)),
            ty: parse_quote!(HasMany<Order>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            relation: None,
            foreign_key: Some(parse_quote!(Order::CUSTOMER_ID)),
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert
        assert!(result.is_ok());
        let ParsedField::HasMany(field) = result.unwrap() else {
            panic!("Expected HasManyField");
        };
        assert_eq!(field.target_type, "Order");
        assert!(field.foreign_key.segments.last().is_some());
    }

    #[test]
    fn test_regular_field_is_column() {
        // Arrange a regular field
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(String),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            relation: None,
            foreign_key: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ParsedField::Column(_)));
    }
}
