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
    pub fn belongs_to(&self) -> impl Iterator<Item = (&ColumnField, &Relation)> {
        self.column_fields.iter().filter_map(|field| {
            let relation = field.relation.as_ref()?;

            Some((field, relation))
        })
    }

    /// Returns non-ambiguous belongs_to relations (one per parent type).
    ///
    /// When a model has multiple belongs_to to the same parent type
    /// (e.g., Message with sender_id and recipient_id both referencing User),
    /// those fields are filtered out to avoid ambiguity.
    pub fn belongs_to_non_ambiguous(&self) -> impl Iterator<Item = (&ColumnField, &Relation)> {
        // Group by parent type to detect duplicates
        let mut by_parent: HashMap<String, Vec<(&ColumnField, &Relation)>> = HashMap::new();
        for (field, relation) in self.belongs_to() {
            let parent_name = relation.referenced_type.to_string();
            by_parent
                .entry(parent_name)
                .or_default()
                .push((field, relation));
        }

        self.belongs_to().filter(move |(_, relation)| {
            let parent_name = relation.referenced_type.to_string();
            by_parent
                .get(&parent_name)
                .map(|fields| fields.len() == 1)
                .unwrap_or(true)
        })
    }

    /// Returns one-to-many HasMany fields (without `through`).
    pub fn one_to_many_fields(&self) -> impl Iterator<Item = &HasManyField> {
        self.has_many_fields.iter().filter(|f| f.through.is_none())
    }

    /// Returns many-to-many HasMany fields (with `through`).
    pub fn many_to_many_fields(&self) -> impl Iterator<Item = &HasManyField> {
        self.has_many_fields.iter().filter(|f| f.through.is_some())
    }
}
