use std::collections::HashMap;

use common_lang_types::{
    EnumLiteralValue, GraphQLScalarTypeName, IsoLiteralText, IsographObjectTypeName,
    SelectableName, UnvalidatedTypeName, ValueKeyName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, NameValuePair};
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, DefinitionLocation, LinkedFieldSelection,
    LoadableDirectiveParameters, NonConstantValue, ObjectSelectionDirectiveSet,
    ScalarFieldSelection, ScalarSelectionDirectiveSet, SelectionFieldArgument, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectId, ServerScalarSelectableId,
    VariableDefinition,
};

use thiserror::Error;

use crate::{
    schema_validation_state::SchemaValidationState, ClientField, ClientFieldVariant, ClientPointer,
    ImperativelyLoadedFieldVariant, OutputFormat, Schema, SchemaObject, ServerFieldData,
    ServerScalarSelectable, UnvalidatedSchema, UseRefetchFieldRefetchStrategy,
    ValidateEntrypointDeclarationError,
};

pub type ValidatedSchemaServerField<TOutputFormat> = ServerScalarSelectable<TOutputFormat>;

pub type ValidatedSelection = SelectionTypeContainingSelections<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

pub type ValidatedLinkedFieldSelection = LinkedFieldSelection<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;
pub type ValidatedScalarFieldSelection =
    ScalarFieldSelection<ValidatedScalarSelectionAssociatedData>;

pub type ValidatedVariableDefinition = VariableDefinition<ServerEntityId>;
pub type ValidatedClientField<TOutputFormat> = ClientField<TOutputFormat>;

pub type ValidatedClientPointer<TOutputFormat> = ClientPointer<TOutputFormat>;

pub type ValidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation =
    DefinitionLocation<ServerScalarSelectableId, ClientFieldId>;

#[derive(Debug, Clone)]
pub struct ValidatedLinkedFieldAssociatedData {
    pub parent_object_id: ServerObjectId,
    pub field_id: DefinitionLocation<ServerScalarSelectableId, ClientPointerId>,
    pub selection_variant: ObjectSelectionDirectiveSet,
    /// Some if the (destination?) object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,
}

// TODO this should encode whether the scalar selection points to a
// client field or to a server scalar
#[derive(Debug, Clone)]
pub struct ValidatedScalarSelectionAssociatedData {
    pub location: ValidatedFieldDefinitionLocation,
    pub selection_variant: ScalarSelectionDirectiveSet,
}

pub type MissingArguments = Vec<ValidatedVariableDefinition>;

pub type ValidatedSelectionType<'a, TOutputFormat> = SelectionType<
    &'a ValidatedClientField<TOutputFormat>,
    &'a ValidatedClientPointer<TOutputFormat>,
>;

#[derive(Debug)]
pub struct ValidatedSchemaState {}
impl SchemaValidationState for ValidatedSchemaState {
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
            server_scalar_selectables: fields,
            client_types,
            entrypoints: _,
            server_field_data: schema_data,
            fetchable_types: root_types,
            ..
        } = unvalidated_schema;

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
                server_scalar_selectables: fields,
                client_types,
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
            DefinitionLocation::Server(server_field_id) => (encountered_field_name, {
                DefinitionLocation::Server(server_field_id)
            }),
            DefinitionLocation::Client(client_field_id) => (
                encountered_field_name,
                DefinitionLocation::Client(client_field_id),
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

pub fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, Vec<E>>>,
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
    selection_variant: &'a ScalarSelectionDirectiveSet,
) -> Option<Loadability<'a>> {
    match &client_field.variant {
        ClientFieldVariant::Link => None,
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ScalarSelectionDirectiveSet::None(_) => None,
            ScalarSelectionDirectiveSet::Updatable(_) => None,
            ScalarSelectionDirectiveSet::Loadable(l) => {
                Some(Loadability::LoadablySelectedField(&l.loadable))
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
                .any(|arg| definition.name.item == arg.item.name.item);
            if user_has_supplied_argument {
                Some(definition.clone())
            } else {
                None
            }
        })
        .collect()
}

pub type ValidateSchemaResult<T> = Result<T, WithLocation<ValidateSchemaError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ValidateSchemaError {
    #[error(
        "The field `{parent_type_name}.{field_name}` has inner type `{field_type}`, which does not exist."
    )]
    FieldTypenameDoesNotExist {
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        field_type: UnvalidatedTypeName,
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
    SelectionTypeSelectionFieldDoesNotExist {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a scalar, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    SelectionTypeSelectionFieldIsNotScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field's type is `{target_type_name}`, which is a scalar."
    )]
    SelectionTypeSelectionFieldIsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field is a client field, which can only be selected as a scalar."
    )]
    SelectionTypeSelectionClientFieldSelectedAsLinked {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        pointer `{field_parent_type_name}.{field_name}` is selected as a scalar. \
        However, client pointers can only be selected as linked fields."
    )]
    SelectionTypeSelectionClientPointerSelectedAsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error("`{server_field_name}` is a server field, and cannot be selected with `@loadable`")]
    ServerFieldCannotBeSelectedLoadably { server_field_name: SelectableName },

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
        missing_fields_names: Vec<SelectableName>,
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
        field_name: SelectableName,
    },

    #[error("This variable is not defined: ${undefined_variable}")]
    UsedUndefinedVariable { undefined_variable: VariableName },
}
