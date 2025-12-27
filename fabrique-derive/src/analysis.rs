use crate::analysis::ast::{FieldKind, Model, ModelField, Relation};
use crate::analysis::steps::Input;
use crate::error::Error;
use syn::{DeriveInput, Ident};

pub mod ast;
mod steps;

/// Completed analysis containing parsed input and validated metadata.
#[derive(Debug)]
pub struct Analysis<'a> {
    /// Named fields of the analyzed struct.
    pub fields: Vec<ModelField>,

    /// Identifier of the analyzed struct.
    #[allow(dead_code)]
    pub ident: &'a Ident,

    /// The model information.
    pub model: Model,
}

impl<'a> Analysis<'a> {
    /// Performs complete analysis of the derive input.
    pub fn from(input: &'a DeriveInput) -> Result<Self, Error> {
        let analysis = Input::new(input).parse_struct()?.parse_fields()?.build()?;

        Ok(analysis)
    }

    pub fn relations(&self) -> impl Iterator<Item = (&ModelField, &Relation)> {
        self.fields.iter().filter_map(|field| {
            let relation = field.relation.as_ref()?;

            Some((field, relation))
        })
    }

    /// Returns only database column fields (excludes HasMany).
    pub fn column_fields(&self) -> impl Iterator<Item = &ModelField> {
        self.fields.iter().filter(|f| f.kind == FieldKind::Column)
    }

    /// Returns only HasMany relationship fields.
    pub fn has_many_fields(&self) -> impl Iterator<Item = &ModelField> {
        self.fields
            .iter()
            .filter(|f| matches!(f.kind, FieldKind::HasMany(_)))
    }
}
