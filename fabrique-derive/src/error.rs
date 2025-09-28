/// Errors that can occur during factory derivation.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Other")]
    _Other,

    #[error("Factory can only be derived from named structs, enum given")]
    UnsupportedDataStructureEnum,

    #[error("Factory can only be derived from named structs, tuple struct given")]
    UnsupportedDataStructureTupleStruct,

    #[error("Factory can only be derived from named structs, union given")]
    UnsupportedDataStructureUnion,

    #[error("Factory can only be derived from named structs, unit struct given")]
    UnsupportedDataStructureUnitStruct,
}
