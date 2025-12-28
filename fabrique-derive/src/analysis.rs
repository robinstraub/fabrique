use std::collections::HashMap;

use crate::analysis::ast::{ColumnField, HasManyField, Model, Relation};
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

    /// HasMany relationship fields.
    pub has_many_fields: Vec<HasManyField>,

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
    pub fn relations(&self) -> impl Iterator<Item = (&ColumnField, &Relation)> {
        self.column_fields.iter().filter_map(|field| {
            let relation = field.relation.as_ref()?;

            Some((field, relation))
        })
    }

    /// Groups belongs_to relations by their parent type name.
    ///
    /// Returns a map where keys are parent type names (e.g., "User") and values
    /// are vectors of (ColumnField, Relation) tuples referencing that parent.
    ///
    /// This is used to detect when a model has multiple belongs_to relations
    /// to the same parent type (e.g., Message with sender_id and recipient_id
    /// both referencing User).
    pub fn belongs_to_by_parent(&self) -> HashMap<String, Vec<(&ColumnField, &Relation)>> {
        let mut map: HashMap<String, Vec<(&ColumnField, &Relation)>> = HashMap::new();

        for (field, relation) in self.relations() {
            let parent_name = relation.referenced_type.to_string();
            map.entry(parent_name).or_default().push((field, relation));
        }

        map
    }
}
