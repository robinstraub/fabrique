use crate::analysis::ast::{ColumnField, Model, Relation};
use crate::analysis::steps::Input;
use crate::error::Error;
use syn::{DeriveInput, Ident};

pub mod ast;
mod steps;

/// Completed analysis containing parsed input and validated metadata.
#[derive(Debug)]
pub struct Analysis<'a> {
    /// Database column fields.
    pub column_fields: Vec<ColumnField>,

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

    /// Returns column fields with their belongs_to relations.
    pub fn belongs_to(&self) -> impl Iterator<Item = (&ColumnField, &Relation)> {
        self.column_fields.iter().filter_map(|field| {
            let relation = field.relation.as_ref()?;

            Some((field, relation))
        })
    }
}
