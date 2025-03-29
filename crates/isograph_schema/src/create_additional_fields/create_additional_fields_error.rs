use std::collections::HashMap;

use crate::{OutputFormat, UnvalidatedSchema};
use common_lang_types::{IsographObjectTypeName, SelectableName, StringLiteralValue, WithLocation};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::ServerObjectId;

use serde::Deserialize;
use thiserror::Error;

// When constructing the final map, we can replace object type names with ids.
pub type ValidatedTypeRefinementMap = HashMap<ServerObjectId, Vec<ServerObjectId>>;

impl<TOutputFormat: OutputFormat> UnvalidatedSchema<TOutputFormat> {}

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

pub(crate) type ProcessTypeDefinitionResult<T> =
    Result<T, WithLocation<CreateAdditionalFieldsError>>;

/// Errors that make semantic sense when referring to creating a GraphQL schema in-memory representation
#[derive(Error, Clone, Eq, PartialEq, Debug)]
pub enum CreateAdditionalFieldsError {
    #[error(
        "The Isograph compiler attempted to create a field named \
    \"{field_name}\" on type \"{parent_type}\", but a field with that name already exists."
    )]
    FieldExistsOnType {
        field_name: SelectableName,
        parent_type: IsographObjectTypeName,
    },

    #[error("Invalid field in @exposeField directive")]
    InvalidField,

    #[error("Invalid mutation field")]
    InvalidMutationField,

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{mutation_object_name}.{mutation_field_name}` does not have argument `{field_name}`, \
        or it was previously processed by another field_map item."
    )]
    PrimaryDirectiveArgumentDoesNotExistOnField {
        primary_type_name: IsographObjectTypeName,
        mutation_object_name: IsographObjectTypeName,
        mutation_field_name: SelectableName,
        field_name: StringLiteralValue,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{field_name}` is an object, and cannot be remapped. Remap scalars only."
    )]
    PrimaryDirectiveCannotRemapObject {
        primary_type_name: IsographObjectTypeName,
        field_name: String,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{field_name}` is not found."
    )]
    PrimaryDirectiveFieldNotFound {
        primary_type_name: IsographObjectTypeName,
        field_name: StringLiteralValue,
    },

    #[error("Failed to deserialize {0}")]
    FailedToDeserialize(String),
}

#[derive(Debug)]
pub struct TypeRefinementMaps {
    pub subtype_to_supertype_map: ValidatedTypeRefinementMap,
    pub supertype_to_subtype_map: ValidatedTypeRefinementMap,
}
