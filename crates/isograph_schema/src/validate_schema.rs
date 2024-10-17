use std::collections::{BTreeSet, HashMap, HashSet};

use common_lang_types::{
    FieldArgumentName, IsographObjectTypeName, Location, SelectableFieldName, UnvalidatedTypeName,
    VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use intern::Lookup;
use isograph_lang_types::{
    ClientFieldId, IsographSelectionVariant, LinkedFieldSelection, LoadableDirectiveParameters,
    ScalarFieldSelection, SelectableServerFieldId, Selection, SelectionFieldArgument,
    ServerFieldId, ServerObjectId, ServerScalarId, TypeAnnotation, UnvalidatedScalarFieldSelection,
    UnvalidatedSelection, VariableDefinition,
};
use thiserror::Error;

use crate::{
    ClientField, ClientFieldVariant, FieldDefinitionLocation, ImperativelyLoadedFieldVariant,
    RefetchStrategy, Schema, SchemaIdField, SchemaObject, SchemaServerField, SchemaValidationState,
    ServerFieldData, UnvalidatedClientField, UnvalidatedLinkedFieldSelection,
    UnvalidatedRefetchFieldStrategy, UnvalidatedSchema, UnvalidatedSchemaSchemaField,
    UnvalidatedSchemaState, UnvalidatedVariableDefinition, UseRefetchFieldRefetchStrategy,
    ValidateEntrypointDeclarationError,
};

use intern::string_key::Intern;
use lazy_static::lazy_static;

pub type ValidatedSchemaServerField = SchemaServerField<
    <ValidatedSchemaState as SchemaValidationState>::ServerFieldTypeAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type ValidatedSelection = Selection<
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;

pub type ValidatedLinkedFieldSelection = LinkedFieldSelection<
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;
pub type ValidatedScalarFieldSelection = ScalarFieldSelection<
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
>;

pub type ValidatedVariableDefinition = VariableDefinition<SelectableServerFieldId>;
pub type ValidatedClientField = ClientField<
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type ValidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <ValidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation = FieldDefinitionLocation<ServerFieldId, ClientFieldId>;

pub type ValidatedSchemaIdField = SchemaIdField<ServerScalarId>;

#[derive(Debug)]
pub struct ValidatedLinkedFieldAssociatedData {
    pub parent_object_id: ServerObjectId,
    // N.B. we don't actually support loadable linked fields
    pub selection_variant: ValidatedIsographSelectionVariant,
}

#[derive(Debug)]
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
}

pub type MissingArguments = Vec<ValidatedVariableDefinition>;

#[derive(Debug)]
pub struct ValidatedSchemaState {}
impl SchemaValidationState for ValidatedSchemaState {
    type ServerFieldTypeAssociatedData = TypeAnnotation<SelectableServerFieldId>;
    type ClientFieldSelectionScalarFieldAssociatedData = ValidatedScalarFieldAssociatedData;
    type ClientFieldSelectionLinkedFieldAssociatedData = ValidatedLinkedFieldAssociatedData;
    type VariableDefinitionInnerType = SelectableServerFieldId;
    type Entrypoint = HashSet<ClientFieldId>;
}

pub type ValidatedSchema = Schema<ValidatedSchemaState>;

impl ValidatedSchema {
    pub fn validate_and_construct(
        unvalidated_schema: UnvalidatedSchema,
    ) -> Result<Self, Vec<WithLocation<ValidateSchemaError>>> {
        let mut errors = vec![];

        let mut updated_entrypoints = HashSet::new();
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
                    updated_entrypoints.insert(client_field_id);
                }
                Err(e) => errors.push(e),
            }
        }

        let Schema {
            server_fields: fields,
            client_fields,
            entrypoints: _,
            server_field_data: schema_data,
            id_type_id: id_type,
            string_type_id: string_type,
            float_type_id,
            boolean_type_id,
            int_type_id,
            null_type_id,
            fetchable_types: root_types,
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

        let updated_client_fields = match validate_and_transform_client_fields(
            client_fields,
            &schema_data,
            &updated_server_fields,
        ) {
            Ok(client_fields) => client_fields,
            Err(new_errors) => {
                errors.extend(new_errors);
                vec![]
            }
        };

        let ServerFieldData {
            server_objects,
            server_scalars,
            defined_types,
        } = schema_data;

        if errors.is_empty() {
            let server_objects = server_objects
                .into_iter()
                .map(transform_object_field_ids)
                .collect();

            Ok(Self {
                server_fields: updated_server_fields,
                client_fields: updated_client_fields,
                entrypoints: updated_entrypoints,
                server_field_data: ServerFieldData {
                    server_objects,
                    server_scalars,
                    defined_types,
                },
                id_type_id: id_type,
                string_type_id: string_type,
                float_type_id,
                boolean_type_id,
                int_type_id,
                fetchable_types: root_types,
                null_type_id,
            })
        } else {
            Err(errors)
        }
    }
}

fn transform_object_field_ids(unvalidated_object: SchemaObject) -> SchemaObject {
    let SchemaObject {
        name,
        description,
        id,
        encountered_fields: unvalidated_encountered_fields,
        id_field,
        directives,
        concrete_type,
    } = unvalidated_object;

    let validated_encountered_fields = unvalidated_encountered_fields
        .into_iter()
        .map(|(encountered_field_name, value)| match value {
            FieldDefinitionLocation::Server(server_field_id) => (encountered_field_name, {
                FieldDefinitionLocation::Server(server_field_id)
            }),
            FieldDefinitionLocation::Client(client_field_id) => (
                encountered_field_name,
                FieldDefinitionLocation::Client(client_field_id),
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
    }
}

fn validate_and_transform_server_fields(
    fields: Vec<UnvalidatedSchemaSchemaField>,
    schema_data: &ServerFieldData,
) -> Result<Vec<ValidatedSchemaServerField>, Vec<WithLocation<ValidateSchemaError>>> {
    get_all_errors_or_all_ok_iter(
        fields
            .into_iter()
            .map(|field| validate_and_transform_field(field, schema_data)),
    )
}

fn get_all_errors_or_all_ok_as_hashmap<K: std::cmp::Eq + std::hash::Hash, V, E>(
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

fn get_all_errors_or_all_ok<T, E>(
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

fn get_all_errors_or_tuple_ok<T1, T2, E>(
    a: Result<T1, impl IntoIterator<Item = E>>,
    b: Result<T2, impl IntoIterator<Item = E>>,
) -> Result<(T1, T2), Vec<E>> {
    match (a, b) {
        (Ok(v1), Ok(v2)) => Ok((v1, v2)),
        (Err(e1), Err(e2)) => Err(e1.into_iter().chain(e2.into_iter()).collect()),
        (_, Err(e)) => Err(e.into_iter().collect()),
        (Err(e), _) => Err(e.into_iter().collect()),
    }
}

fn get_all_errors_or_all_ok_iter<T, E>(
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

fn validate_and_transform_field(
    field: UnvalidatedSchemaSchemaField,
    schema_data: &ServerFieldData,
) -> Result<ValidatedSchemaServerField, impl Iterator<Item = WithLocation<ValidateSchemaError>>> {
    // TODO rewrite as field.map(...).transpose()
    let (empty_field, server_field_type) = field.split();

    let mut errors = vec![];

    let field_type =
        match validate_server_field_type_exists(schema_data, &server_field_type, &empty_field) {
            Ok(type_annotation) => Some(type_annotation),
            Err(e) => {
                errors.push(e);
                None
            }
        };

    let valid_arguments =
        match get_all_errors_or_all_ok(empty_field.arguments.into_iter().map(|argument| {
            validate_server_field_argument(
                argument,
                schema_data,
                empty_field.parent_type_id,
                empty_field.name,
            )
        })) {
            Ok(arguments) => Some(arguments),
            Err(e) => {
                errors.extend(e);
                None
            }
        };

    if let Some(field_type) = field_type {
        if let Some(valid_arguments) = valid_arguments {
            return Ok(SchemaServerField {
                description: empty_field.description,
                name: empty_field.name,
                id: empty_field.id,
                associated_data: field_type,
                parent_type_id: empty_field.parent_type_id,
                arguments: valid_arguments,
                is_discriminator: empty_field.is_discriminator,
            });
        }
    }

    Err(errors.into_iter())
}

fn validate_server_field_type_exists(
    schema_data: &ServerFieldData,
    server_field_type: &GraphQLTypeAnnotation<UnvalidatedTypeName>,
    field: &SchemaServerField<
        (),
        <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    >,
) -> ValidateSchemaResult<TypeAnnotation<SelectableServerFieldId>> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(server_field_type.inner()) {
        // Why do we need to clone here? Can we avoid this?
        Some(type_id) => Ok(TypeAnnotation::from_graphql_type_annotation(
            server_field_type.clone().map(|_| *type_id),
        )),
        None => Err(WithLocation::new(
            ValidateSchemaError::FieldTypenameDoesNotExist {
                parent_type_name: schema_data.object(field.parent_type_id).name,
                field_name: field.name.item,
                field_type: *server_field_type.inner(),
            },
            field.name.location,
        )),
    }
}

fn validate_server_field_argument(
    argument: WithLocation<UnvalidatedVariableDefinition>,
    schema_data: &ServerFieldData,
    parent_type_id: ServerObjectId,
    name: WithLocation<SelectableFieldName>,
) -> ValidateSchemaResult<WithLocation<ValidatedVariableDefinition>> {
    // Isograph doesn't care about the default value, and that remains
    // unvalidated.

    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(argument.item.type_.inner()) {
        Some(selectable_server_field_id) => Ok(WithLocation::new(
            VariableDefinition {
                name: argument.item.name,
                type_: argument.item.type_.map(|_| *selectable_server_field_id),
                default_value: argument.item.default_value,
            },
            argument.location,
        )),
        None => Err(WithLocation::new(
            ValidateSchemaError::FieldArgumentTypeDoesNotExist {
                parent_type_name: schema_data.object(parent_type_id).name,
                field_name: name.item,
                argument_name: argument.item.name.item,
                argument_type: *argument.item.type_.inner(),
            },
            name.location,
        )),
    }
}

type ClientFieldArgsMap = HashMap<ClientFieldId, Vec<WithSpan<ValidatedVariableDefinition>>>;

fn validate_and_transform_client_fields(
    client_fields: Vec<UnvalidatedClientField>,
    schema_data: &ServerFieldData,
    server_fields: &[ValidatedSchemaServerField],
) -> Result<Vec<ValidatedClientField>, Vec<WithLocation<ValidateSchemaError>>> {
    // TODO this smells. We probably should do this in two passes instead of doing it this
    // way. We are validating client fields, which includes validating their selections. When
    // validating a selection of a client field, we need to ensure that we pass the correct
    // arguments to the client field (e.g. no missing fields unless it was selected loadably.)
    //
    // For now, we'll make a new datastructure containing all of the client field's arguments,
    // cloned.
    let client_field_args = get_all_errors_or_all_ok_as_hashmap(client_fields.iter().map(
        |unvalidated_client_field| {
            let validated_variable_definitions = validate_variable_definitions(
                schema_data,
                unvalidated_client_field.variable_definitions.clone(),
            )?;
            Ok((unvalidated_client_field.id, validated_variable_definitions))
        },
    ))?;

    get_all_errors_or_all_ok_iter(client_fields.into_iter().map(|client_field| {
        validate_client_field_selection_set(
            schema_data,
            client_field,
            server_fields,
            &client_field_args,
        )
        .map_err(|err| err.into_iter())
    }))
}

fn validate_all_variables_are_used(
    variable_definitions: Vec<WithSpan<UnvalidatedVariableDefinition>>,
    used_variables: BTreeSet<VariableName>,
    parent_type_name: IsographObjectTypeName,
    client_field_name: SelectableFieldName,
) -> ValidateSelectionsResult<()> {
    let unused_variables: Vec<_> = variable_definitions
        .into_iter()
        .filter_map(|variable| {
            let is_used = used_variables.contains(&variable.item.name.item);

            if !is_used {
                return Some(variable);
            }
            None
        })
        .collect();

    if !unused_variables.is_empty() {
        return Err(WithLocation::new(
            ValidateSelectionsError::UnusedVariables {
                unused_variables,
                type_name: parent_type_name,
                field_name: client_field_name,
            },
            Location::generated(),
        ));
    }
    Ok(())
}

fn validate_client_field_selection_set(
    schema_data: &ServerFieldData,
    unvalidated_client_field: UnvalidatedClientField,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
) -> Result<ValidatedClientField, Vec<WithLocation<ValidateSchemaError>>> {
    let variable_definitions = client_field_args
        .get(&unvalidated_client_field.id)
        .expect(
            "Expected variable definitions to exist. \
            This is indicative of a bug in Isograph",
        )
        .clone();

    let parent_object = schema_data.object(unvalidated_client_field.parent_object_id);
    let selection_set_result = unvalidated_client_field
        .reader_selection_set
        .map(|selection_set| {
            validate_client_field_definition_selections_exist_and_types_match(
                schema_data,
                selection_set,
                parent_object,
                server_fields,
                client_field_args,
                unvalidated_client_field.variable_definitions,
                unvalidated_client_field.name,
            )
            .map_err(|errs| {
                errs.into_iter().map(|err| {
                    validate_selections_error_to_validate_schema_error(
                        err,
                        parent_object,
                        unvalidated_client_field.name,
                    )
                })
            })
        })
        .transpose();

    let refetch_strategy_result = unvalidated_client_field
        .refetch_strategy
        .map(|refetch_strategy| match refetch_strategy {
            RefetchStrategy::UseRefetchField(use_refetch_field_strategy) => {
                Ok::<_, Vec<WithLocation<ValidateSchemaError>>>(RefetchStrategy::UseRefetchField(
                    validate_use_refetch_field_strategy(
                        schema_data,
                        use_refetch_field_strategy,
                        server_fields,
                        parent_object,
                        unvalidated_client_field.name,
                        client_field_args,
                    )?,
                ))
            }
        })
        .transpose();

    let (selection_set, refetch_strategy) =
        get_all_errors_or_tuple_ok(selection_set_result, refetch_strategy_result)?;

    Ok(ClientField {
        description: unvalidated_client_field.description,
        name: unvalidated_client_field.name,
        id: unvalidated_client_field.id,
        reader_selection_set: selection_set,
        unwraps: unvalidated_client_field.unwraps,
        variant: unvalidated_client_field.variant,
        variable_definitions,
        type_and_field: unvalidated_client_field.type_and_field,
        parent_object_id: unvalidated_client_field.parent_object_id,
        refetch_strategy,
    })
}

/// Validate the selection set on the RefetchFieldStrategy, in particular, associate
/// id's with each selection in the refetch_selection_set
fn validate_use_refetch_field_strategy(
    schema_data: &ServerFieldData,
    use_refetch_field_strategy: UnvalidatedRefetchFieldStrategy,
    server_fields: &[ValidatedSchemaServerField],
    parent_object: &SchemaObject,
    client_field_name: SelectableFieldName,
    client_field_args: &ClientFieldArgsMap,
) -> Result<ValidatedRefetchFieldStrategy, Vec<WithLocation<ValidateSchemaError>>> {
    let refetch_selection_set = validate_client_field_definition_selections_exist_and_types_match(
        schema_data,
        use_refetch_field_strategy.refetch_selection_set,
        parent_object,
        server_fields,
        client_field_args,
        vec![],
        client_field_name,
    )
    .map_err(|errs| {
        errs.into_iter()
            .map(|err| {
                validate_selections_error_to_validate_schema_error(
                    err,
                    parent_object,
                    client_field_name,
                )
            })
            .collect::<Vec<_>>()
    })?;

    Ok(ValidatedRefetchFieldStrategy {
        refetch_selection_set,
        root_fetchable_type: use_refetch_field_strategy.root_fetchable_type,
        generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
        refetch_query_name: use_refetch_field_strategy.refetch_query_name,
    })
}

fn validate_variable_definitions(
    schema_data: &ServerFieldData,
    variable_definitions: Vec<WithSpan<UnvalidatedVariableDefinition>>,
) -> ValidateSchemaResult<Vec<WithSpan<ValidatedVariableDefinition>>> {
    variable_definitions
        .into_iter()
        .map(|with_span| {
            with_span.and_then(|vd| {
                // TODO this should be doable in the error branch
                let type_string = vd.type_.to_string();
                let inner_type = *vd.type_.inner();
                Ok(VariableDefinition {
                    name: vd.name,
                    type_: vd.type_.and_then(|type_name| {
                        match schema_data.defined_types.get(&type_name) {
                            Some(type_id) => Ok(*type_id),
                            None => Err(WithLocation::new(
                                ValidateSchemaError::VariableDefinitionInnerTypeDoesNotExist {
                                    variable_name: vd.name.item,
                                    type_: type_string,
                                    inner_type,
                                },
                                vd.name.location,
                            )),
                        }
                    })?,
                    default_value: vd.default_value,
                })
            })
        })
        .collect()
}

fn validate_selections_error_to_validate_schema_error(
    err: WithLocation<ValidateSelectionsError>,
    parent_object: &SchemaObject,
    client_field_name: SelectableFieldName,
) -> WithLocation<ValidateSchemaError> {
    err.map(|item| match item {
        ValidateSelectionsError::FieldDoesNotExist(field_parent_type_name, field_name) => {
            ValidateSchemaError::ClientFieldSelectionFieldDoesNotExist {
                client_field_parent_type_name: parent_object.name,
                client_field_name,
                field_parent_type_name,
                field_name,
            }
        }
        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
            field_parent_type_name: parent_type_name,
            field_name,
            target_type,
            target_type_name,
        } => ValidateSchemaError::ClientFieldSelectionFieldIsNotScalar {
            client_field_parent_type_name: parent_object.name,
            client_field_name,
            field_parent_type_name: parent_type_name,
            field_name,
            field_type: target_type,
            target_type_name,
        },
        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
            field_parent_type_name,
            field_name,
            target_type,
            target_type_name,
        } => ValidateSchemaError::ClientFieldSelectionFieldIsScalar {
            client_field_parent_type_name: parent_object.name,
            client_field_name,
            field_parent_type_name,
            field_name,
            field_type: target_type,
            target_type_name,
        },
        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsClientField {
            field_parent_type_name,
            field_name,
        } => ValidateSchemaError::ClientFieldSelectionClientFieldSelectedAsLinked {
            client_field_parent_type_name: parent_object.name,
            client_field_name: field_name,
            field_parent_type_name,
            field_name,
        },
        ValidateSelectionsError::ServerFieldCannotBeSelectedLoadably { server_field_name } => {
            ValidateSchemaError::ServerFieldCannotBeSelectedLoadably { server_field_name }
        }
        ValidateSelectionsError::MissingArguments { missing_arguments } => {
            ValidateSchemaError::MissingArguments { missing_arguments }
        }
        ValidateSelectionsError::ExtraneousArgument { extra_arguments } => {
            ValidateSchemaError::ExtraneousArgument { extra_arguments }
        }
        ValidateSelectionsError::UnusedVariables {
            unused_variables,
            type_name,
            field_name,
        } => ValidateSchemaError::UnusedVariables {
            unused_variables,
            type_name,
            field_name,
        },
    })
}

type ValidateSelectionsResult<T> = Result<T, WithLocation<ValidateSelectionsError>>;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum ValidateSelectionsError {
    FieldDoesNotExist(IsographObjectTypeName, SelectableFieldName),
    FieldSelectedAsScalarButTypeIsNotScalar {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsScalar {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsClientField {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },
    ServerFieldCannotBeSelectedLoadably {
        server_field_name: SelectableFieldName,
    },
    MissingArguments {
        missing_arguments: Vec<ValidatedVariableDefinition>,
    },
    ExtraneousArgument {
        extra_arguments: Vec<WithLocation<SelectionFieldArgument>>,
    },
    UnusedVariables {
        unused_variables: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
        type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },
}

fn validate_client_field_definition_selections_exist_and_types_match(
    schema_data: &ServerFieldData,
    selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    parent_object: &SchemaObject,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
    variable_definitions: Vec<WithSpan<UnvalidatedVariableDefinition>>,
    client_field_name: SelectableFieldName,
) -> Result<Vec<WithSpan<ValidatedSelection>>, Vec<WithLocation<ValidateSelectionsError>>> {
    // Currently, we only check that each field exists and has an appropriate type, not that
    // there are no selection conflicts due to aliases or parameters.

    let mut used_variables: BTreeSet<VariableName> = BTreeSet::new();

    let validated_selection_set_result =
        get_all_errors_or_all_ok(selection_set.into_iter().map(|selection| {
            validate_client_field_definition_selection_exists_and_type_matches(
                selection,
                parent_object,
                schema_data,
                server_fields,
                client_field_args,
                &mut used_variables,
            )
        }));

    let (validated_selection_set, _) = get_all_errors_or_tuple_ok(
        validated_selection_set_result,
        validate_all_variables_are_used(
            variable_definitions,
            used_variables,
            parent_object.name,
            client_field_name,
        )
        .map_err(|err| vec![err]),
    )?;

    Ok(validated_selection_set)
}

fn validate_client_field_definition_selection_exists_and_type_matches(
    selection: WithSpan<UnvalidatedSelection>,
    parent_object: &SchemaObject,
    schema_data: &ServerFieldData,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
    used_variables: &mut BTreeSet<VariableName>,
) -> ValidateSelectionsResult<WithSpan<ValidatedSelection>> {
    let mut used_variables2 = BTreeSet::new();

    let validated_selection = selection.and_then(|selection| {
        selection.and_then(&mut |field_selection| {
            field_selection.and_then(
                &mut |scalar_field_selection| {
                    validate_field_type_exists_and_is_scalar(
                        schema_data,
                        parent_object,
                        scalar_field_selection,
                        server_fields,
                        client_field_args,
                        used_variables,
                    )
                },
                &mut |linked_field_selection| {
                    validate_field_type_exists_and_is_linked(
                        schema_data,
                        parent_object,
                        linked_field_selection,
                        server_fields,
                        client_field_args,
                        &mut used_variables2,
                    )
                },
            )
        })
    });

    used_variables.append(&mut used_variables2);

    validated_selection
}

/// Given that we selected a scalar field, the field should exist on the parent,
/// and type should be a client field (which is a scalar) or a server scalar type.
fn validate_field_type_exists_and_is_scalar(
    schema_data: &ServerFieldData,
    parent_object: &SchemaObject,
    scalar_field_selection: UnvalidatedScalarFieldSelection,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
    used_variables: &mut BTreeSet<VariableName>,
) -> ValidateSelectionsResult<ValidatedScalarFieldSelection> {
    let scalar_field_name = scalar_field_selection.name.item.into();
    match parent_object.encountered_fields.get(&scalar_field_name) {
        Some(defined_field_type) => match defined_field_type {
            FieldDefinitionLocation::Server(server_field_id) => {
                let server_field = &server_fields[server_field_id.as_usize()];
                let missing_arguments = get_missing_arguments_and_validate_argument_types(
                    server_field
                        .arguments
                        .iter()
                        .map(|variable_definition| &variable_definition.item),
                    &scalar_field_selection.arguments,
                    false,
                    scalar_field_selection.name.location,
                    used_variables,
                )?;

                match server_field.associated_data.inner_non_null() {
                    SelectableServerFieldId::Scalar(_) => Ok(ScalarFieldSelection {
                        name: scalar_field_selection.name,
                        associated_data: ValidatedScalarFieldAssociatedData {
                            location: FieldDefinitionLocation::Server(*server_field_id),
                            selection_variant: match scalar_field_selection.associated_data {
                                IsographSelectionVariant::Regular => {
                                    assert_no_missing_arguments(
                                        missing_arguments,
                                        scalar_field_selection.name.location,
                                    )?;
                                    ValidatedIsographSelectionVariant::Regular
                                }
                                IsographSelectionVariant::Loadable(l) => {
                                    server_field_cannot_be_selected_loadably(
                                        scalar_field_name,
                                        scalar_field_selection.name.location,
                                    )?;
                                    ValidatedIsographSelectionVariant::Loadable((
                                        l,
                                        missing_arguments,
                                    ))
                                }
                            },
                        },
                        reader_alias: scalar_field_selection.reader_alias,
                        unwraps: scalar_field_selection.unwraps,
                        arguments: scalar_field_selection.arguments,
                        directives: scalar_field_selection.directives,
                    }),
                    SelectableServerFieldId::Object(object_id) => Err(WithLocation::new(
                        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
                            field_parent_type_name: parent_object.name,
                            field_name: scalar_field_name,
                            target_type: "an object",
                            target_type_name: schema_data.object(object_id).name.into(),
                        },
                        scalar_field_selection.name.location,
                    )),
                }
            }
            FieldDefinitionLocation::Client(client_field_id) => validate_client_field(
                client_field_args,
                client_field_id,
                scalar_field_selection,
                used_variables,
            ),
        },
        None => Err(WithLocation::new(
            ValidateSelectionsError::FieldDoesNotExist(parent_object.name, scalar_field_name),
            scalar_field_selection.name.location,
        )),
    }
}

fn validate_client_field(
    client_field_args: &ClientFieldArgsMap,
    client_field_id: &ClientFieldId,
    scalar_field_selection: UnvalidatedScalarFieldSelection,
    used_variables: &mut BTreeSet<VariableName>,
) -> ValidateSelectionsResult<ValidatedScalarFieldSelection> {
    let argument_definitions = client_field_args.get(client_field_id).expect(
        "Expected client field to exist in map. \
                    This is indicative of a bug in Isograph.",
    );
    let missing_arguments = get_missing_arguments_and_validate_argument_types(
        argument_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &scalar_field_selection.arguments,
        false,
        scalar_field_selection.name.location,
        used_variables,
    )?;

    Ok(ScalarFieldSelection {
        name: scalar_field_selection.name,
        reader_alias: scalar_field_selection.reader_alias,
        unwraps: scalar_field_selection.unwraps,
        associated_data: ValidatedScalarFieldAssociatedData {
            location: FieldDefinitionLocation::Client(*client_field_id),
            selection_variant: match scalar_field_selection.associated_data {
                IsographSelectionVariant::Regular => {
                    assert_no_missing_arguments(
                        missing_arguments,
                        scalar_field_selection.name.location,
                    )?;
                    ValidatedIsographSelectionVariant::Regular
                }
                IsographSelectionVariant::Loadable(l) => {
                    ValidatedIsographSelectionVariant::Loadable((l, missing_arguments))
                }
            },
        },
        arguments: scalar_field_selection.arguments,
        directives: scalar_field_selection.directives,
    })
}

/// Given that we selected a linked field, the field should exist on the parent,
/// and type should be a server interface, object or union.
fn validate_field_type_exists_and_is_linked(
    schema_data: &ServerFieldData,
    parent_object: &SchemaObject,
    linked_field_selection: UnvalidatedLinkedFieldSelection,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
    used_variables: &mut BTreeSet<VariableName>,
) -> ValidateSelectionsResult<ValidatedLinkedFieldSelection> {
    let linked_field_name = linked_field_selection.name.item.into();
    match (parent_object.encountered_fields).get(&linked_field_name) {
        Some(defined_field_type) => match defined_field_type {
            FieldDefinitionLocation::Server(server_field_id) => {
                let server_field = &server_fields[server_field_id.as_usize()];
                match server_field.associated_data.inner_non_null() {
                    SelectableServerFieldId::Scalar(scalar_id) => Err(WithLocation::new(
                        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
                            field_parent_type_name: parent_object.name,
                            field_name: linked_field_name,
                            target_type: "a scalar",
                            target_type_name: schema_data.scalar(scalar_id).name.item.into(),
                        },
                        linked_field_selection.name.location,
                    )),
                    SelectableServerFieldId::Object(object_id) => {
                        let object = schema_data
                            .server_objects
                            .get(object_id.as_usize())
                            .unwrap();

                        let missing_arguments = get_missing_arguments_and_validate_argument_types(
                            server_field
                                .arguments
                                .iter()
                                .map(|variable_definition| &variable_definition.item),
                            &linked_field_selection.arguments,
                            false,
                            linked_field_selection.name.location,
                            used_variables,
                        )?;

                        Ok(LinkedFieldSelection {
                                name: linked_field_selection.name,
                                reader_alias: linked_field_selection.reader_alias,
                                selection_set: linked_field_selection.selection_set.into_iter().map(
                                    |selection| {
                                        validate_client_field_definition_selection_exists_and_type_matches(
                                            selection,
                                            object,
                                            schema_data,
                                            server_fields,
                                            client_field_args,
                                            used_variables
                                        )
                                    },
                                ).collect::<Result<Vec<_>, _>>()?,
                                unwraps: linked_field_selection.unwraps,
                                associated_data: ValidatedLinkedFieldAssociatedData {
                                    parent_object_id: object_id,
                                    selection_variant: match linked_field_selection.associated_data {
                                        IsographSelectionVariant::Regular => {
                                            assert_no_missing_arguments(missing_arguments, linked_field_selection.name.location)?;
                                            ValidatedIsographSelectionVariant::Regular
                                        },
                                        IsographSelectionVariant::Loadable(l) => {
                                            server_field_cannot_be_selected_loadably(linked_field_name, linked_field_selection.name.location)?;
                                            ValidatedIsographSelectionVariant::Loadable((l, missing_arguments))
                                        },
                                    },
                                },
                                arguments: linked_field_selection.arguments,
                                directives: linked_field_selection.directives
                            })
                    }
                }
            }
            FieldDefinitionLocation::Client(_) => Err(WithLocation::new(
                ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsClientField {
                    field_parent_type_name: parent_object.name,
                    field_name: linked_field_name,
                },
                linked_field_selection.name.location,
            )),
        },
        None => Err(WithLocation::new(
            ValidateSelectionsError::FieldDoesNotExist(parent_object.name, linked_field_name),
            linked_field_selection.name.location,
        )),
    }
}

fn server_field_cannot_be_selected_loadably(
    server_field_name: SelectableFieldName,
    location: Location,
) -> ValidateSelectionsResult<()> {
    Err(WithLocation::new(
        ValidateSelectionsError::ServerFieldCannotBeSelectedLoadably { server_field_name },
        location,
    ))
}

fn assert_no_missing_arguments(
    missing_arguments: Vec<ValidatedVariableDefinition>,
    location: Location,
) -> ValidateSelectionsResult<()> {
    if !missing_arguments.is_empty() {
        return Err(WithLocation::new(
            ValidateSelectionsError::MissingArguments { missing_arguments },
            location,
        ));
    }
    Ok(())
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
pub fn categorize_field_loadability<'a>(
    client_field: &'a ValidatedClientField,
    selection_variant: &'a ValidatedIsographSelectionVariant,
) -> Option<Loadability<'a>> {
    match &client_field.variant {
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ValidatedIsographSelectionVariant::Regular => None,
            ValidatedIsographSelectionVariant::Loadable((l, _)) => {
                Some(Loadability::LoadablySelectedField(l))
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(i) => {
            Some(Loadability::ImperativelyLoadedField(i))
        }
    }
}

lazy_static! {
    static ref ID: FieldArgumentName = "id".intern().into();
}

fn validate_no_extraneous_arguments(
    argument_definitions: &[&ValidatedVariableDefinition],
    arguments: &[WithLocation<SelectionFieldArgument>],
    location: Location,
) -> ValidateSelectionsResult<()> {
    let extra_arguments: Vec<_> = arguments
        .iter()
        .filter_map(|arg| {
            // TODO remove this
            // With @exposeField on Query, id field is needed because the generated
            // query is like node(id: $id) { ... everything else }, but that
            // id field is added in somewhere else

            if arg.item.name.item == *ID {
                return None;
            }

            let is_defined = argument_definitions
                .iter()
                .any(|definition| definition.name.item.lookup() == arg.item.name.item.lookup());

            if !is_defined {
                return Some(arg.clone());
            }
            None
        })
        .collect();

    if !extra_arguments.is_empty() {
        return Err(WithLocation::new(
            ValidateSelectionsError::ExtraneousArgument { extra_arguments },
            location,
        ));
    }
    Ok(())
}

fn push_used_variables(
    arguments: &[WithLocation<SelectionFieldArgument>],
    used_variables: &mut BTreeSet<VariableName>,
) {
    for argument in arguments {
        used_variables.extend(argument.item.value.item.reachable_variables().iter());
    }
}

fn get_missing_arguments_and_validate_argument_types<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
    include_optional_args: bool,
    location: Location,
    used_variables: &mut BTreeSet<VariableName>,
) -> ValidateSelectionsResult<Vec<ValidatedVariableDefinition>> {
    push_used_variables(arguments, used_variables);

    let argument_definitions_vec: Vec<_> = argument_definitions.collect();
    validate_no_extraneous_arguments(&argument_definitions_vec, arguments, location)?;

    // TODO validate argument types
    Ok(get_missing_arguments(
        argument_definitions_vec.into_iter(),
        arguments,
        include_optional_args,
    ))
}

pub fn get_missing_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
    include_optional_args: bool,
) -> Vec<ValidatedVariableDefinition> {
    argument_definitions
        .filter_map(|definition| {
            if definition.default_value.is_some()
                || definition.type_.is_nullable() && !include_optional_args
            {
                return None;
            }

            let user_has_supplied_argument = arguments
                .iter()
                // TODO do not call .lookup
                .any(|arg| definition.name.item.lookup() == arg.item.name.item.lookup());
            if user_has_supplied_argument {
                None
            } else {
                Some(definition.clone())
            }
        })
        .collect()
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

type ValidateSchemaResult<T> = Result<T, WithLocation<ValidateSchemaError>>;

#[derive(Debug, Error)]
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

    #[error(
        "In the client field `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected, but that \
        field does not exist on `{field_parent_type_name}`"
    )]
    ClientFieldSelectionFieldDoesNotExist {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },

    #[error(
        "In the client field `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a scalar, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ClientFieldSelectionFieldIsNotScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "In the client field `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ClientFieldSelectionFieldIsScalar {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "In the client field `{client_field_parent_type_name}.{client_field_name}`, the \
        field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field is a client field, which can only be selected as a scalar."
    )]
    ClientFieldSelectionClientFieldSelectedAsLinked {
        client_field_parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
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
        "The field `{type_name}.{field_name}` has unused variables: {0}",
        unused_variables.iter().map(|variable| format!("${}", variable.item.name.item)).collect::<Vec<_>>().join(", ")
    )]
    UnusedVariables {
        unused_variables: Vec<WithSpan<UnvalidatedVariableDefinition>>,
        type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },
}
