use darling::{FromDeriveInput, FromField};
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::Span;
use syn::{Expr, Ident, Type};

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
            table_name: attrs.table.unwrap_or_else(|| {
                let singular = ident.to_string().to_lowercase();
                pluralizer::pluralize(&singular, 2, false)
            }),
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

    /// The type referenced by this belongs_to field
    #[darling(default)]
    belongs_to: Option<Ident>,

    /// Custom faker expression for generating random values.
    /// If not specified, `Faker.fake::<FieldType>()` is used by default.
    #[darling(default)]
    faker: Option<String>,

    /// Alias name for disambiguation on belongs_to fields.
    /// Generates a pseudo-Model for named joins.
    #[darling(default)]
    alias: Option<Ident>,
}

#[derive(Debug)]
pub struct Relation {
    pub name: String,
    pub referenced_type: Ident,
    /// Optional alias for named joins. When set, generates a pseudo-Model
    /// with this name for SQL table aliasing and type-safe column access.
    pub alias: Option<Ident>,
    /// Snake_case alias name (e.g., "seller"). Computed when alias is present.
    pub alias_snake: Option<String>,
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

    /// Parsed faker expression for random value generation.
    /// If None, uses `Faker.fake::<FieldType>()` by default.
    pub faker: Option<Expr>,
}

impl ColumnField {
    /// Parses a field's attributes into a `ColumnField`.
    pub fn try_from_attrs(attrs: ModelFieldAttrs, struct_name: String) -> Result<Self, Error> {
        let ident = attrs.ident.ok_or_else(|| {
            Error::new(attrs.span, ErrorKind::UnsupportedDataStructureTupleStruct)
        })?;

        let column = ident.to_string();
        let const_column_name = Ident::new(&ident.to_string().to_uppercase(), ident.span());
        let pascal_case_field = ident.to_string().to_pascal_case();
        let type_name = format!("{}{}Column", struct_name, pascal_case_field);
        let column_type = syn::Ident::new(&type_name, ident.span());

        let alias = attrs.alias;
        if alias.is_some() && attrs.belongs_to.is_none() {
            return Err(Error::new(attrs.span, ErrorKind::AliasWithoutRelation));
        }
        let alias_snake = alias.as_ref().map(|a| a.to_string().to_snake_case());
        let relation = attrs.belongs_to.map(|referenced_type| Relation {
            name: referenced_type.to_string().to_snake_case(),
            referenced_type,
            alias,
            alias_snake,
        });

        // Parse faker expression if provided
        let faker = attrs
            .faker
            .map(|s| syn::parse_str::<Expr>(&s))
            .transpose()
            .map_err(|e| {
                Error::new(attrs.span, ErrorKind::InvalidFakerExpression(e.to_string()))
            })?;

        Ok(ColumnField {
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
            faker,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn attrs(ident: Option<Ident>, ty: Type) -> ModelFieldAttrs {
        ModelFieldAttrs {
            ident,
            ty,
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            faker: None,
            alias: None,
        }
    }

    #[test]
    fn test_try_from_attrs_without_ident_fails() {
        let result =
            ColumnField::try_from_attrs(attrs(None, parse_quote!(u32)), "Anvil".to_owned());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            ErrorKind::UnsupportedDataStructureTupleStruct
        ));
    }

    #[test]
    fn test_regular_field_is_column() {
        let result = ColumnField::try_from_attrs(
            attrs(Some(parse_quote!(name)), parse_quote!(String)),
            "Customer".to_owned(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_reference_type_is_column() {
        let result = ColumnField::try_from_attrs(
            attrs(Some(parse_quote!(name)), parse_quote!(&str)),
            "Customer".to_owned(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_type_is_column() {
        let result = ColumnField::try_from_attrs(
            attrs(Some(parse_quote!(items)), parse_quote!(Vec<&str>)),
            "Customer".to_owned(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_alias_without_belongs_to_fails() {
        let mut a = attrs(Some(parse_quote!(name)), parse_quote!(String));
        a.alias = Some(parse_quote!(Sender));

        let result = ColumnField::try_from_attrs(a, "User".to_owned());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            ErrorKind::AliasWithoutRelation
        ));
    }

    #[test]
    fn test_faker_attribute_parsed() {
        let mut a = attrs(Some(parse_quote!(name)), parse_quote!(String));
        a.faker = Some("Name()".to_string());

        let result = ColumnField::try_from_attrs(a, "User".to_owned());

        assert!(result.unwrap().faker.is_some());
    }

    #[test]
    fn test_faker_range_expression_parsed() {
        let mut a = attrs(Some(parse_quote!(age)), parse_quote!(i32));
        a.faker = Some("0..99".to_string());

        let result = ColumnField::try_from_attrs(a, "User".to_owned());

        assert!(result.unwrap().faker.is_some());
    }

    #[test]
    fn test_faker_without_attribute_is_none() {
        let result = ColumnField::try_from_attrs(
            attrs(Some(parse_quote!(name)), parse_quote!(String)),
            "User".to_owned(),
        );

        assert!(result.unwrap().faker.is_none());
    }

    #[test]
    fn test_invalid_faker_expression_fails() {
        let mut a = attrs(Some(parse_quote!(name)), parse_quote!(String));
        a.faker = Some("invalid { expression".to_string());

        let result = ColumnField::try_from_attrs(a, "User".to_owned());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            ErrorKind::InvalidFakerExpression(_)
        ));
    }
}
