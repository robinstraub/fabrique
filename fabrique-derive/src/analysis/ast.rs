use darling::{FromDeriveInput, FromField};
use syn::{Field, Ident, Type};

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

    /// Wether this field is a primary key
    #[darling(default)]
    primary_key: bool,

    /// The type referenced by this relation field
    #[darling(default)]
    relation: Option<Ident>,

    /// The key field of the referenced type
    #[darling(default)]
    referenced_key: Option<Ident>,
}

#[derive(Debug)]
pub struct ModelField<'a> {
    field: &'a Field,

    pub r#as: Option<Type>,
    pub ident: Option<&'a Ident>,
    pub ty: &'a Type,
}

impl<'a> ModelField<'a> {
    pub fn new(field: &'a Field, attributes: ModelFieldAttrs) -> Self {
        Self {
            field,
            ident: field.ident.as_ref(),
            r#as: attributes.r#as,
            ty: &field.ty,
        }
    }
}
