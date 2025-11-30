use darling::{FromDeriveInput, FromField};
use proc_macro2::Span;
use syn::{Ident, Type};

use crate::error::Error;

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
    /// The database type for conversion
    #[darling(default)]
    r#as: Option<Type>,

    ident: Option<Ident>,

    /// Wether this field is a primary key
    #[darling(default)]
    primary_key: bool,

    /// The type referenced by this relation field
    #[darling(default)]
    relation: Option<Ident>,

    /// The key field of the referenced type
    #[darling(default)]
    referenced_key: Option<Ident>,

    #[darling(skip, default = "Span::call_site")]
    span: Span,

    ty: Type,
}

#[derive(Debug)]
pub struct Relation {
    pub name: String,
    pub referenced_type: Ident,
    pub referenced_key: Ident,
}

#[derive(Debug)]
pub struct ModelField {
    /// Type marker to (de)serialize from/to the persistance layer.
    pub r#as: Option<Type>,

    /// The field ident.
    pub ident: Ident,

    /// The field relation, if any.
    pub relation: Option<Relation>,

    /// The field span.
    pub span: Span,

    /// The field type.
    pub ty: Type,
}

impl ModelField {
    pub fn try_from(attrs: ModelFieldAttrs) -> Result<Self, Error> {
        let ident = attrs
            .ident
            .ok_or(Error::UnsupportedDataStructureTupleStruct)?;

        let relation = match attrs.relation {
            Some(referenced_type) => {
                let field_name = ident.to_string();
                let referenced_key = attrs
                    .referenced_key
                    .ok_or(Error::MissingReferencedKey(field_name.clone()))?;
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
            r#as: attrs.r#as,
            ident,
            relation,
            span: attrs.span,
            ty: attrs.ty,
        })
    }
}
