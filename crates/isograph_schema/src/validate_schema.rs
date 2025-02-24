use std::collections::HashMap;

use common_lang_types::{
    EnumLiteralValue, GraphQLScalarTypeName, IsoLiteralText, IsographObjectTypeName,
    SelectableFieldName, UnvalidatedTypeName, ValueKeyName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, NameValuePair};
use intern::Lookup;
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, LinkedFieldSelection, LoadableDirectiveParameters,
    NonConstantValue, ScalarFieldSelection, SelectableServerFieldId, SelectionFieldArgument,
    SelectionType, ServerFieldId, ServerFieldSelection, ServerObjectId, ServerScalarId,
    TypeAnnotation, VariableDefinition,
};

use thiserror::Error;

use crate::{
    schema_validation_state::SchemaValidationState,
    validate_client_field::validate_and_transform_client_types,
    validate_server_field::validate_and_transform_server_fields, ClientField, ClientFieldVariant,
    ClientPointer, FieldType, ImperativelyLoadedFieldVariant, OutputFormat, Schema, SchemaIdField,
    SchemaObject, SchemaServerField, ServerFieldData, ServerFieldTypeAssociatedData,
    UnvalidatedSchema, UseRefetchFieldRefetchStrategy, ValidateEntrypointDeclarationError,
};

pub type ValidatedSchemaServerField<TOutputFormat> = SchemaServerField<
    <ValidatedSchemaState as SchemaValidationState>::ServerFieldTypeAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type ValidatedSelection = ServerFieldSelection<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionLinkedFieldAssociatedData,
>;

pub type ValidatedLinkedFieldSelection = LinkedFieldSelection<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionLinkedFieldAssociatedData,
>;
pub type ValidatedScalarFieldSelection = ScalarFieldSelection<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
>;

pub type ValidatedVariableDefinition = VariableDefinition<SelectableServerFieldId>;
pub type ValidatedClientField<TOutputFormat> = ClientField<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionLinkedFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type ValidatedClientPointer<TOutputFormat> = ClientPointer<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionLinkedFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type ValidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientTypeSelectionLinkedFieldAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation = FieldType<ServerFieldId, ClientFieldId>;

pub type ValidatedSchemaIdField = SchemaIdField<ServerScalarId>;

#[derive(Debug, Clone)]
pub struct ValidatedLinkedFieldAssociatedData {
    pub parent_object_id: ServerObjectId,
    pub field_id: FieldType<ServerFieldId, ClientPointerId>,
    // N.B. we don't actually support loadable linked fields
    pub selection_variant: ValidatedIsographSelectionVariant,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,
}

#[derive(Debug, Clone)]
pub struct ValidatedScalarFieldAssociatedData {
    pub location: ValidatedFieldDefinitionLocation,
    pub selection_variant: ValidatedIsographSelectionVariant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidatedIsographSelectionVariant {
    Regular,
    Loadable(
        (
            LoadableDirectiveParameters,
            // TODO this is unused
            MissingArguments,
        ),
    ),
    Updatable,
}

pub type MissingArguments = Vec<ValidatedVariableDefinition>;

pub type ValidatedServerFieldTypeAssociatedData = SelectionType<
    ServerFieldTypeAssociatedData<TypeAnnotation<ServerObjectId>>,
    TypeAnnotation<ServerScalarId>,
>;

#[derive(Debug)]
pub struct ValidatedSchemaState {}
impl SchemaValidationState for ValidatedSchemaState {
    type ServerFieldTypeAssociatedData = ValidatedServerFieldTypeAssociatedData;
    type ClientTypeSelectionScalarFieldAssociatedData = ValidatedScalarFieldAssociatedData;
    type ClientTypeSelectionLinkedFieldAssociatedData = ValidatedLinkedFieldAssociatedData;
    type VariableDefinitionInnerType = SelectableServerFieldId;
    type Entrypoint = HashMap<ClientFieldId, IsoLiteralText>;
}

pub type ValidatedSchema<TOutputFormat> = Schema<ValidatedSchemaState, TOutputFormat>;

impl<TOutputFormat: OutputFormat> ValidatedSchema<TOutputFormat> {
    pub fn validate_and_construct(
        unvalidated_schema: UnvalidatedSchema<TOutputFormat>,
    ) -> Result<Self, Vec<WithLocation<ValidateSchemaError>>> {
        let mut errors = vec![];

        let mut updated_entrypoints = HashMap::new();
        for (text_source, entrypoint_type_and_field) in unvalidated_schema.entrypoints.iter() {
            match unvalidated_schema
                .validate_entrypoint_type_and_field(*text_source, *entrypoint_type_and_field)
                .map_err(|e| {
                    WithLocation::new(
                        ValidateSchemaError::ErrorValidatingEntrypointDeclaration {
                            message: e.item,
                        },
                        e.location,
                    )
                }) {
                Ok(client_field_id) => {
                    updated_entrypoints.insert(
                        client_field_id,
                        entrypoint_type_and_field.item.iso_literal_text,
                    );
                }
                Err(e) => errors.push(e),
            }
        }

        let Schema {
            server_fields: fields,
            client_types,
            entrypoints: _,
            server_field_data: schema_data,
            fetchable_types: root_types,
            ..
        } = unvalidated_schema;

        let updated_server_fields = match validate_and_transform_server_fields(fields, &schema_data)
        {
            Ok(fields) => fields,
            Err(new_errors) => {
                errors.extend(new_errors);
                return Err(errors);

                // Because fields flows into updated_client_fields, we cannot optimistically
                // continue here.
                // TODO: figure out whether this can be worked around.
            }
        };

        let updated_client_types = match validate_and_transform_client_types(
            client_types,
            &schema_data,
            &updated_server_fields,
        ) {
            Ok(client_types) => client_types,
            Err(new_errors) => {
                errors.extend(new_errors);
                vec![]
            }
        };

        let ServerFieldData {
            server_objects,
            server_scalars,
            defined_types,
            id_type_id: id_type,
            string_type_id: string_type,
            float_type_id,
            boolean_type_id,
            int_type_id,
            null_type_id,
            ..
        } = schema_data;

        if errors.is_empty() {
            let server_objects = server_objects
                .into_iter()
                .map(transform_object_field_ids)
                .collect();

            Ok(Self {
                server_fields: updated_server_fields,
                client_types: updated_client_types,
                entrypoints: updated_entrypoints,
                server_field_data: ServerFieldData {
                    server_objects,
                    server_scalars,
                    defined_types,

                    id_type_id: id_type,
                    string_type_id: string_type,
                    float_type_id,
                    boolean_type_id,
                    int_type_id,
                    null_type_id,
                },
                fetchable_types: root_types,
            })
        } else {
            Err(errors)
        }
    }
}

fn transform_object_field_ids<TOutputFormat: OutputFormat>(
    unvalidated_object: SchemaObject<TOutputFormat>,
) -> SchemaObject<TOutputFormat> {
    let SchemaObject {
        name,
        description,
        id,
        encountered_fields: unvalidated_encountered_fields,
        id_field,
        directives,
        concrete_type,
        output_associated_data,
    } = unvalidated_object;

    let validated_encountered_fields = unvalidated_encountered_fields
        .into_iter()
        .map(|(encountered_field_name, value)| match value {
            FieldType::ServerField(server_field_id) => (encountered_field_name, {
                FieldType::ServerField(server_field_id)
            }),
            FieldType::ClientField(client_field_id) => (
                encountered_field_name,
                FieldType::ClientField(client_field_id),
            ),
        })
        .collect();

    SchemaObject {
        description,
        name,
        id,
        encountered_fields: validated_encountered_fields,
        id_field,
        directives,
        concrete_type,
        output_associated_data,
    }
}

pub(crate) fn get_all_errors_or_all_ok_as_hashmap<K: std::cmp::Eq + std::hash::Hash, V, E>(
    items: impl Iterator<Item = Result<(K, V), E>>,
) -> Result<HashMap<K, V>, Vec<E>> {
    let mut oks = HashMap::new();
    let mut errors = vec![];

    for item in items {
        match item {
            Ok((k, v)) => {
                oks.insert(k, v);
            }
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

pub(crate) fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, E>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

pub(crate) fn get_all_errors_or_tuple_ok<T1, T2, E>(
    a: Result<T1, impl IntoIterator<Item = E>>,
    b: Result<T2, impl IntoIterator<Item = E>>,
) -> Result<(T1, T2), Vec<E>> {
    match (a, b) {
        (Ok(v1), Ok(v2)) => Ok((v1, v2)),
        (Err(e1), Err(e2)) => Err(e1.into_iter().chain(e2).collect()),
        (_, Err(e)) => Err(e.into_iter().collect()),
        (Err(e), _) => Err(e.into_iter().collect()),
    }
}

pub(crate) fn get_all_errors_or_all_ok_iter<T, E>(
    items: impl Iterator<Item = Result<T, impl Iterator<Item = E>>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.extend(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

pub enum Loadability<'a> {
    LoadablySelectedField(&'a LoadableDirectiveParameters),
    ImperativelyLoadedField(&'a ImperativelyLoadedFieldVariant),
}

/// Why do we do this? Because how we handle a field is determined by both the
/// the field defition (e.g. exposed fields can only be fetched imperatively)
/// and the selection (i.e. we can also take non-imperative fields and make them
/// imperative.)
///
/// The eventual plan is to clean this model up. Instead, imperative fields will
/// need to be explicitly selected loadably. If they are not, they will be fetched
/// as an immediate follow-up request. Once we do this, there will always be one
/// source of truth for whether a field is fetched imperatively: the presence of the
/// @loadable directive.
pub fn categorize_field_loadability<'a, TOutputFormat: OutputFormat>(
    client_field: &'a ValidatedClientField<TOutputFormat>,
    selection_variant: &'a ValidatedIsographSelectionVariant,
) -> Option<Loadability<'a>> {
    match &client_field.variant {
        ClientFieldVariant::Link => None,
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ValidatedIsographSelectionVariant::Regular => None,
            ValidatedIsographSelectionVariant::Updatable => None,
            ValidatedIsographSelectionVariant::Loadable((l, _)) => {
                Some(Loadability::LoadablySelectedField(l))
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(i) => {
            Some(Loadability::ImperativelyLoadedField(i))
        }
    }
}

pub fn get_provided_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> Vec<ValidatedVariableDefinition> {
    argument_definitions
        .filter_map(|definition| {
            let user_has_supplied_argument = arguments
                .iter()
                // TODO do not call .lookup
                .any(|arg| definition.name.item.lookup() == arg.item.name.item.lookup());
            if user_has_supplied_argument {
                Some(definition.clone())
            } else {
                None
            }
        })
        .collect()
}

pub(crate) type ValidateSchemaResult<T> = Result<T, WithLocation<ValidateSchemaError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ValidateSchemaError {
    #[error(
        "The field `{parent_type_name}.{field_name}` has inner type `{field_type}`, which does not exist."
    )]
    FieldTypenameDoesNotExist {
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: UnvalidatedTypeName,
    },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        argument_type: UnvalidatedTypeName,
    },

    #[error("Expected input of type {expected_type}, found variable {variable_name} of type {variable_type}")]
    ExpectedTypeFoundVariable {
        expected_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_name: VariableName,
    },

    #[error("Expected input of type {expected}, found {actual} scalar literal")]
    ExpectedTypeFoundScalar {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        actual: GraphQLScalarTypeName,
    },

    #[error("Expected input of type {expected}, found object literal")]
    ExpectedTypeFoundObject {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected input of type {expected}, found list literal")]
    ExpectedTypeFoundList {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected non null input of type {expected}, found null")]
    ExpectedNonNullTypeFoundNull {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    },

    #[error("Expected input of type {expected}, found {actual} enum literal")]
    ExpectedTypeFoundEnum {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        actual: EnumLiteralValue,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected, but that \
        field does not exist on `{field_parent_type_name}`"
    )]
    ClientTypeSelectionFieldDoesNotExist {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a scalar, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ClientTypeSelectionFieldIsNotScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ClientTypeSelectionFieldIsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field is a client field, which can only be selected as a scalar."
    )]
    ClientTypeSelectionClientFieldSelectedAsLinked {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        pointer `{field_parent_type_name}.{field_name}` is selected as a scalar. \
        However, client pointers can only be selected as linked fields."
    )]
    ClientTypeSelectionClientPointerSelectedAsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        client_type: String,
    },

    #[error("`{server_field_name}` is a server field, and cannot be selected with `@loadable`")]
    ServerFieldCannotBeSelectedLoadably {
        server_field_name: SelectableFieldName,
    },

    #[error(
        "This field has missing arguments: {0}",
        missing_arguments.iter().map(|arg| format!("${}", arg.name.item)).collect::<Vec<_>>().join(", ")
    )]
    MissingArguments { missing_arguments: MissingArguments },

    #[error(
        "This object has missing fields: {0}",
        missing_fields_names.iter().map(|field_name| format!("${}", field_name)).collect::<Vec<_>>().join(", ")
    )]
    MissingFields {
        missing_fields_names: Vec<SelectableFieldName>,
    },

    #[error(
        "The variable `{variable_name}` has type `{type_}`, but the inner type \
        `{inner_type}` does not exist."
    )]
    VariableDefinitionInnerTypeDoesNotExist {
        variable_name: VariableName,
        type_: String,
        inner_type: UnvalidatedTypeName,
    },

    #[error("Error when validating iso entrypoint calls.\nMessage: {message}")]
    ErrorValidatingEntrypointDeclaration {
        message: ValidateEntrypointDeclarationError,
    },

    #[error(
        "This field has extra arguments: {0}",
        extra_arguments.iter().map(|arg| format!("{}", arg.item.name)).collect::<Vec<_>>().join(", ")
    )]
    ExtraneousArgument {
        extra_arguments: Vec<WithLocation<SelectionFieldArgument>>,
    },

    #[error(
        "This object has extra fields: {0}",
        extra_fields.iter().map(|field| format!("{}", field.name.item)).collect::<Vec<_>>().join(", ")
    )]
    ExtraneousFields {
        extra_fields: Vec<NameValuePair<ValueKeyName, NonConstantValue>>,
    },

    #[error(
        "The field `{type_name}.{field_name}` has unused variables: {0}",
        unused_variables.iter().map(|variable| format!("${}", variable.item.name.item)).collect::<Vec<_>>().join(", ")
    )]
    UnusedVariables {
        unused_variables: Vec<WithSpan<ValidatedVariableDefinition>>,
        type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },

    #[error("This variable is not defined: ${undefined_variable}")]
    UsedUndefinedVariable { undefined_variable: VariableName },
}
