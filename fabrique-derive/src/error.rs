/// Errors that can occur during factory derivation.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Expected a literal str, got {0:?}")]
    UnparsableLiteral(String),

    #[error("Could not parse literal to an ident: {0}")]
    UnparsableType(String),

    #[error("Could not parse attribute: {0}")]
    UnparsableAttribute(String),

    #[error("Factory can only be derived from named structs, enum given")]
    UnsupportedDataStructureEnum,

    #[error("Factory can only be derived from named structs, tuple struct given")]
    UnsupportedDataStructureTupleStruct,

    #[error("Factory can only be derived from named structs, union given")]
    UnsupportedDataStructureUnion,

    #[error("Factory can only be derived from named structs, unit struct given")]
    UnsupportedDataStructureUnitStruct,

    #[error("Unknown attribute: {0}")]
    UnknownAttribute(String),

    #[error(
        "The relation {0} is missing a referenced key. By default, the suffix of the field is used (e.g. the referenced key of the relation `hammer_id` is `id`). Please use the `referenced_key` attribute or give this field a suffix."
    )]
    MissingReferencedKey(String),
}
