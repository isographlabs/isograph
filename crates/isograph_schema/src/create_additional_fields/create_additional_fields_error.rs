use crate::{
    EntityAccessError, FieldToInsertToServerSelectableError, NetworkProtocol, Schema,
    ServerSelectableNamedError,
};
use common_lang_types::{
    ClientScalarSelectableName, SelectableName, ServerObjectEntityName, StringLiteralValue,
    UnvalidatedTypeName, VariableName,
};
use intern::{Lookup, string_key::Intern};

use serde::Deserialize;
use thiserror::Error;

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields)]
pub struct FieldMapItem {
    // TODO eventually, we want to support . syntax here, too
    pub from: StringLiteralValue,
    pub to: StringLiteralValue,
}

pub struct SplitToArg {
    pub to_argument_name: StringLiteralValue,
    pub to_field_names: Vec<StringLiteralValue>,
}

impl FieldMapItem {
    pub fn split_to_arg(&self) -> SplitToArg {
        let mut split = self.to.lookup().split('.');
        let to_argument_name = split.next().expect(
            "Expected at least one item returned \
                by split. This is indicative of a bug in Isograph.",
        );

        SplitToArg {
            to_argument_name: to_argument_name.intern().into(),
            to_field_names: split.map(|x| x.intern().into()).collect(),
        }
    }
}

// TODO this should be a different type.
pub(crate) struct ProcessedFieldMapItem(pub FieldMapItem);

pub type ProcessTypeDefinitionResult<T, TNetworkProtocol> =
    Result<T, CreateAdditionalFieldsError<TNetworkProtocol>>;

/// Errors that make semantic sense when referring to creating a GraphQL schema in-memory representation
///
/// TODO some variants here should contain locations, since we used to have
/// WithLocation<CreateAdditionalFieldsError> everywhere, but we removed that. But it makes sense
/// in some cases!
#[derive(Error, Clone, Eq, PartialEq, Debug)]
pub enum CreateAdditionalFieldsError<TNetworkProtocol: NetworkProtocol> {
    #[error(
        "The Isograph compiler attempted to create a field named \
        `{client_scalar_selectable_name}` on entity `{parent_object_entity_name}`, \
        but a field with that name already exists."
    )]
    CompilerCreatedFieldExistsOnType {
        client_scalar_selectable_name: ClientScalarSelectableName,
        parent_object_entity_name: ServerObjectEntityName,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named `{selectable_name}` on type `{parent_object_entity_name}`")]
    DuplicateField {
        selectable_name: SelectableName,
        parent_object_entity_name: ServerObjectEntityName,
    },

    #[error("Invalid field `{field_arg}` in @exposeField directive")]
    InvalidField { field_arg: String },

    #[error(
        "Error when processing @exposeField directive on type `{primary_object_entity_name}`. \
        The field `{mutation_object_entity_name}.{mutation_selectable_name}` does not have argument `{field_name}`, \
        or it was previously processed by another field_map item."
    )]
    PrimaryDirectiveArgumentDoesNotExistOnField {
        primary_object_entity_name: ServerObjectEntityName,
        mutation_object_entity_name: ServerObjectEntityName,
        mutation_selectable_name: SelectableName,
        field_name: StringLiteralValue,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_object_entity_name}`. \
        The field `{field_name}` is an object, and cannot be remapped. Remap scalars only."
    )]
    PrimaryDirectiveCannotRemapObject {
        primary_object_entity_name: ServerObjectEntityName,
        field_name: String,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_object_entity_name}`. \
        The field `{field_name}` is not found."
    )]
    PrimaryDirectiveFieldNotFound {
        primary_object_entity_name: ServerObjectEntityName,
        field_name: StringLiteralValue,
    },

    #[error("Failed to deserialize {0}")]
    FailedToDeserialize(String),

    #[error(
        "The `{strong_field_name}` field on `{parent_object_entity_name}` must have type `ID!`.\n\
        This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_object_entity_name: ServerObjectEntityName,
        strong_field_name: &'static str,
    },

    #[error(
        "The argument `{argument_name}` on field `{parent_object_entity_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_object_entity_name: ServerObjectEntityName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },

    #[error("This field has type `{target_entity_name}`, which does not exist")]
    FieldTypenameDoesNotExist {
        target_entity_name: UnvalidatedTypeName,
    },

    #[error("Duplicate type definition (`{type_definition_type}`) named `{duplicate_entity_name}`")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        duplicate_entity_name: UnvalidatedTypeName,
    },

    #[error("{0}")]
    EntityAccessError(#[from] EntityAccessError<TNetworkProtocol>),

    #[error("{0}")]
    ServerSelectableNamedError(#[from] ServerSelectableNamedError<TNetworkProtocol>),

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),
}

pub type CreateAdditionalFieldsResult<T, TNetworkProtocol> =
    Result<T, CreateAdditionalFieldsError<TNetworkProtocol>>;
