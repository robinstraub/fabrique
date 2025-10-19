/// Errors that can occur during factory derivation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Factory can only be derived from named structs, enum given")]
    UnsupportedDataStructureEnum,

    #[error("Factory can only be derived from named structs, tuple struct given")]
    UnsupportedDataStructureTupleStruct,

    #[error("Factory can only be derived from named structs, union given")]
    UnsupportedDataStructureUnion,

    #[error("Factory can only be derived from named structs, unit struct given")]
    UnsupportedDataStructureUnitStruct,

    #[error("Missing `referenced_key` attribute for relation {0}")]
    MissingReferencedKey(String),

    #[error("{0}")]
    Darling(#[from] darling::Error),
}
