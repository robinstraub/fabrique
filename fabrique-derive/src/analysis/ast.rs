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

    /// The join model for many-to-many relationships (only valid on HasMany
    /// fields)
    #[darling(default)]
    through: Option<Ident>,

    /// Custom faker expression for generating random values.
    /// If not specified, `Faker.fake::<FieldType>()` is used by default.
    #[darling(default)]
    faker: Option<String>,

    /// Alias name for disambiguation.
    /// - On belongs_to: generates a pseudo-Model for named joins
    /// - On HasMany: references the alias on the child's belongs_to field
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

/// A HasMany relationship field (not stored in database).
#[derive(Debug)]
pub struct HasManyField {
    /// The field ident (e.g., `orders`).
    pub ident: Ident,

    /// The target type (e.g., `Order` from `HasMany<Order>`).
    pub target_type: Ident,

    /// The join model for many-to-many relationships (e.g., `OrderLine`).
    /// When set, this is a many-to-many relationship through the join model.
    pub through: Option<Ident>,

    /// The alias for disambiguation when the child has multiple belongs_to
    /// to this parent type. Must match an alias declared on the child's
    /// belongs_to field. Used for SetForeignKey trait and lazy loading.
    pub alias: Option<Ident>,
}

/// Result of parsing a field - either a column or a HasMany relation.
#[derive(Debug)]
pub enum ParsedField {
    Column(Box<ColumnField>),
    HasMany(HasManyField),
}

impl ParsedField {
    pub fn try_from(attrs: ModelFieldAttrs, struct_name: String) -> Result<Self, Error> {
        let ident = attrs.ident.ok_or_else(|| {
            Error::new(attrs.span, ErrorKind::UnsupportedDataStructureTupleStruct)
        })?;

        // Check if this is a HasMany field
        if let Some((outer, target)) = Self::parse_parameterized_type(&attrs.ty)
            && outer == "HasMany"
        {
            return Ok(ParsedField::HasMany(HasManyField {
                ident,
                target_type: target.clone(),
                through: attrs.through,
                alias: attrs.alias,
            }));
        }

        // Regular column field
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

        Ok(ParsedField::Column(Box::new(ColumnField {
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
        })))
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
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
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
    fn test_has_many_field_succeeds() {
        // Arrange a HasMany field (no foreign_key needed, inferred via BelongsTo)
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(orders)),
            ty: parse_quote!(HasMany<Order>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(matches!(
            parsed,
            ParsedField::HasMany(ref f)
                if f.target_type == "Order"
                && f.through.is_none()
                && f.alias.is_none()
        ));
    }

    #[test]
    fn test_has_many_field_with_through_succeeds() {
        // Arrange a HasMany field with through (many-to-many)
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(anvils)),
            ty: parse_quote!(HasMany<Anvil>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: Some(parse_quote!(OrderLine)),
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Order".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(
            matches!(parsed, ParsedField::HasMany(ref f) if f.target_type == "Anvil" && f.through.as_ref().unwrap() == "OrderLine")
        );
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
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ParsedField::Column(_)));
    }

    #[test]
    fn test_reference_type_is_column() {
        // Arrange a field with a reference type (not Type::Path)
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(&str),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert - reference types are treated as column fields
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ParsedField::Column(_)));
    }

    #[test]
    fn test_generic_with_reference_arg_is_column() {
        // Arrange a field with a generic type whose argument is not Type::Path
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(items)),
            ty: parse_quote!(Vec<&str>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "Customer".to_owned());

        // Assert - generic types with non-Path arguments are treated as column fields
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ParsedField::Column(_)));
    }

    #[test]
    fn test_has_many_field_with_alias_succeeds() {
        // Arrange a HasMany field with alias for disambiguation
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(sent_messages)),
            ty: parse_quote!(HasMany<Message>),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: Some(parse_quote!(Sender)),
        };

        // Act
        let result = ParsedField::try_from(attrs, "User".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(matches!(
            parsed,
            ParsedField::HasMany(ref f)
                if f.target_type == "Message"
                && f.alias.as_ref().unwrap() == "Sender"
        ));
    }

    #[test]
    fn test_alias_without_belongs_to_fails() {
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(String),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: Some(parse_quote!(Sender)),
        };

        let result = ParsedField::try_from(attrs, "User".to_owned());

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error.kind, ErrorKind::AliasWithoutRelation));
    }

    #[test]
    fn test_faker_attribute_parsed() {
        // Arrange a field with custom faker expression
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(String),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: Some("Name()".to_string()),
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "User".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(matches!(parsed, ParsedField::Column(ref f) if f.faker.is_some()));
    }

    #[test]
    fn test_faker_range_expression_parsed() {
        // Arrange a field with range faker expression
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(age)),
            ty: parse_quote!(i32),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: Some("0..99".to_string()),
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "User".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(matches!(parsed, ParsedField::Column(ref f) if f.faker.is_some()));
    }

    #[test]
    fn test_faker_without_attribute_is_none() {
        // Arrange a field without faker expression
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(String),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: None,
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "User".to_owned());

        // Assert
        let parsed = result.expect("should parse successfully");
        assert!(matches!(parsed, ParsedField::Column(ref f) if f.faker.is_none()));
    }

    #[test]
    fn test_invalid_faker_expression_fails() {
        // Arrange a field with invalid faker expression
        let attrs = ModelFieldAttrs {
            ident: Some(parse_quote!(name)),
            ty: parse_quote!(String),
            span: Span::call_site(),
            r#as: None,
            primary_key: false,
            soft_delete: false,
            belongs_to: None,
            through: None,
            faker: Some("invalid { expression".to_string()),
            alias: None,
        };

        // Act
        let result = ParsedField::try_from(attrs, "User".to_owned());

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error.kind, ErrorKind::InvalidFakerExpression(_)));
    }
}
