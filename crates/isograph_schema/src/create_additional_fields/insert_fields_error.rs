use common_lang_types::{
    IsographObjectTypeName, SelectableName, UnvalidatedTypeName, VariableName,
};
use thiserror::Error;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum InsertFieldsError {
    #[error(
        "The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".\n\
        This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_type: IsographObjectTypeName,
        strong_field_name: &'static str,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableName,
        parent_type: IsographObjectTypeName,
    },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },

    #[error("This field has type {target_entity_type_name}, which does not exist")]
    FieldTypenameDoesNotExist {
        target_entity_type_name: UnvalidatedTypeName,
    },

    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    #[error("Expected {type_name} to be an object, but it was a scalar.")]
    GenericObjectIsScalar { type_name: UnvalidatedTypeName },
}

pub(crate) type InsertFieldsResult<T> = Result<T, InsertFieldsError>;
