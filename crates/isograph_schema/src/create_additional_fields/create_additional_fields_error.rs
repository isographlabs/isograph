use crate::ServerSelectableNamedError;
use common_lang_types::{Diagnostic, SelectableName, ServerObjectEntityName, StringLiteralValue};
use intern::{Lookup, string_key::Intern};

use serde::Deserialize;
use thiserror::Error;

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

pub type ProcessTypeDefinitionResult<T> = Result<T, CreateAdditionalFieldsError>;

/// Errors that make semantic sense when referring to creating a GraphQL schema in-memory representation
///
/// TODO some variants here should contain locations, since we used to have
/// WithLocation<CreateAdditionalFieldsError> everywhere, but we removed that. But it makes sense
/// in some cases!
#[derive(Error, Clone, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub enum CreateAdditionalFieldsError {
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

    #[error("{0}")]
    EntityAccessError(Diagnostic),

    #[error("{0}")]
    ServerSelectableNamedError(#[from] ServerSelectableNamedError),

    #[error("{error}")]
    FieldToInsertToServerSelectableError { error: Diagnostic },
}

pub type CreateAdditionalFieldsResult<T> = Result<T, CreateAdditionalFieldsError>;
