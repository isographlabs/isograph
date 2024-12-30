use std::{
    collections::{BTreeSet, HashMap},
    vec,
};

use common_lang_types::{
    EnumLiteralValue, FieldArgumentName, Location, SelectableFieldName, UnvalidatedTypeName,
    VariableName, WithLocation, WithSpan,
};

use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    reachable_variables, ClientFieldId, IsographSelectionVariant, LinkedFieldSelection,
    NonConstantValue, ScalarFieldSelection, SelectableServerFieldId, SelectionFieldArgument,
    SelectionType, ServerScalarId, UnvalidatedScalarFieldSelection, UnvalidatedSelection,
    VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    get_all_errors_or_all_ok, get_all_errors_or_all_ok_as_hashmap, get_all_errors_or_all_ok_iter,
    get_all_errors_or_tuple_ok, ClientField, ClientType, FieldType, ObjectTypeAndFieldName,
    RefetchStrategy, SchemaObject, ServerFieldData, UnvalidatedClientField,
    UnvalidatedLinkedFieldSelection, UnvalidatedRefetchFieldStrategy,
    UnvalidatedVariableDefinition, ValidateSchemaError, ValidateSchemaResult, ValidatedClientField,
    ValidatedIsographSelectionVariant, ValidatedLinkedFieldAssociatedData,
    ValidatedLinkedFieldSelection, ValidatedRefetchFieldStrategy,
    ValidatedScalarFieldAssociatedData, ValidatedScalarFieldSelection, ValidatedSchemaServerField,
    ValidatedSelection, ValidatedVariableDefinition,
};

type UsedVariables = BTreeSet<VariableName>;
type ClientFieldArgsMap =
    HashMap<ClientType<ClientFieldId>, Vec<WithSpan<ValidatedVariableDefinition>>>;

lazy_static! {
    static ref ID: FieldArgumentName = "id".intern().into();
}

pub(crate) fn validate_and_transform_client_fields(
    client_fields: Vec<ClientType<UnvalidatedClientField>>,
    schema_data: &ServerFieldData,
    server_fields: &[ValidatedSchemaServerField],
) -> Result<Vec<ClientType<ValidatedClientField>>, Vec<WithLocation<ValidateSchemaError>>> {
    // TODO this smells. We probably should do this in two passes instead of doing it this
    // way. We are validating client fields, which includes validating their selections. When
    // validating a selection of a client field, we need to ensure that we pass the correct
    // arguments to the client field (e.g. no missing fields unless it was selected loadably.)
    //
    // For now, we'll make a new datastructure containing all of the client field's arguments,
    // cloned.
    let client_field_args =
        get_all_errors_or_all_ok_as_hashmap(client_fields.iter().map(|unvalidated_client| {
            match unvalidated_client {
                ClientType::ClientField(unvalidated_client_field) => {
                    let validated_variable_definitions = validate_variable_definitions(
                        schema_data,
                        unvalidated_client_field.variable_definitions.clone(),
                    )?;
                    Ok((
                        ClientType::ClientField(unvalidated_client_field.id),
                        validated_variable_definitions,
                    ))
                }
            }
        }))?;

    get_all_errors_or_all_ok_iter(client_fields.into_iter().map(|client_field| {
        match client_field {
            ClientType::ClientField(client_field) => validate_client_field_selection_set(
                schema_data,
                client_field,
                server_fields,
                &client_field_args,
            )
            .map(ClientType::ClientField)
            .map_err(|err| err.into_iter()),
        }
    }))
}

fn validate_all_variables_are_used(
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    used_variables: UsedVariables,
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> ValidateSchemaResult<()> {
    let unused_variables: Vec<_> = variable_definitions
        .iter()
        .filter_map(|variable| {
            let is_used = used_variables.contains(&variable.item.name.item);

            if !is_used {
                return Some(variable.clone());
            }
            None
        })
        .collect();

    if !unused_variables.is_empty() {
        return Err(WithLocation::new(
            ValidateSchemaError::UnusedVariables {
                unused_variables,
                type_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .type_name,
                field_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .field_name,
            },
            Location::generated(),
        ));
    }
    Ok(())
}

// So that we don't have to pass five params to all the time,
// encapsulate them in a single struct.
struct ValidateSchemaSharedInfo<'a> {
    client_field_args: &'a ClientFieldArgsMap,
    client_field_type_and_field_name: ObjectTypeAndFieldName,
    client_field_parent_object: &'a SchemaObject,
    schema_data: &'a ServerFieldData,
    server_fields: &'a [ValidatedSchemaServerField],
}

fn validate_client_field_selection_set(
    schema_data: &ServerFieldData,
    top_level_client_field: UnvalidatedClientField,
    server_fields: &[ValidatedSchemaServerField],
    client_field_args: &ClientFieldArgsMap,
) -> Result<ValidatedClientField, Vec<WithLocation<ValidateSchemaError>>> {
    let top_level_client_field_info = ValidateSchemaSharedInfo {
        client_field_args,
        client_field_type_and_field_name: top_level_client_field.type_and_field,
        client_field_parent_object: schema_data.object(top_level_client_field.parent_object_id),
        schema_data,
        server_fields,
    };

    let variable_definitions = client_field_args
        .get(&ClientType::ClientField(top_level_client_field.id))
        .expect(
            "Expected variable definitions to exist. \
            This is indicative of a bug in Isograph",
        )
        .clone();

    let selection_set_result = top_level_client_field
        .reader_selection_set
        .map(|selection_set| {
            validate_client_field_definition_selections_exist_and_types_match(
                selection_set,
                &variable_definitions,
                &top_level_client_field_info,
            )
        })
        .transpose();

    let refetch_strategy_result = top_level_client_field
        .refetch_strategy
        .map(|refetch_strategy| match refetch_strategy {
            RefetchStrategy::UseRefetchField(use_refetch_field_strategy) => {
                Ok::<_, Vec<WithLocation<ValidateSchemaError>>>(RefetchStrategy::UseRefetchField(
                    validate_use_refetch_field_strategy(
                        use_refetch_field_strategy,
                        &top_level_client_field_info,
                    )?,
                ))
            }
        })
        .transpose();

    let (selection_set, refetch_strategy) =
        get_all_errors_or_tuple_ok(selection_set_result, refetch_strategy_result)?;

    Ok(ClientField {
        description: top_level_client_field.description,
        name: top_level_client_field.name,
        id: top_level_client_field.id,
        reader_selection_set: selection_set,
        variant: top_level_client_field.variant,
        variable_definitions,
        type_and_field: top_level_client_field.type_and_field,
        parent_object_id: top_level_client_field.parent_object_id,
        refetch_strategy,
    })
}

/// Validate the selection set on the RefetchFieldStrategy, in particular, associate
/// id's with each selection in the refetch_selection_set
fn validate_use_refetch_field_strategy(
    use_refetch_field_strategy: UnvalidatedRefetchFieldStrategy,
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> Result<ValidatedRefetchFieldStrategy, Vec<WithLocation<ValidateSchemaError>>> {
    let refetch_selection_set = validate_client_field_definition_selections_exist_and_types_match(
        use_refetch_field_strategy.refetch_selection_set,
        &[],
        top_level_client_field_info,
    )?;

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

fn validate_client_field_definition_selections_exist_and_types_match(
    field_selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    field_variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> Result<Vec<WithSpan<ValidatedSelection>>, Vec<WithLocation<ValidateSchemaError>>> {
    // Currently, we only check that each field exists and has an appropriate type, not that
    // there are no selection conflicts due to aliases or parameters.

    let mut used_variables = BTreeSet::new();

    let validated_selection_set_result =
        get_all_errors_or_all_ok(field_selection_set.into_iter().map(|selection| {
            validate_client_field_definition_selection_exists_and_type_matches(
                selection,
                top_level_client_field_info.client_field_parent_object,
                &mut used_variables,
                field_variable_definitions,
                top_level_client_field_info,
            )
        }));

    let (validated_selection_set, _) = get_all_errors_or_tuple_ok(
        validated_selection_set_result,
        validate_all_variables_are_used(
            field_variable_definitions,
            used_variables,
            top_level_client_field_info,
        )
        .map_err(|err| vec![err]),
    )?;

    Ok(validated_selection_set)
}

fn validate_client_field_definition_selection_exists_and_type_matches(
    selection: WithSpan<UnvalidatedSelection>,
    field_parent_object: &SchemaObject,
    used_variables: &mut UsedVariables,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> ValidateSchemaResult<WithSpan<ValidatedSelection>> {
    let mut used_variables2 = BTreeSet::new();

    let validated_selection = selection.and_then(|selection| {
        selection.and_then(
            &mut |scalar_field_selection| {
                validate_field_type_exists_and_is_scalar(
                    field_parent_object,
                    scalar_field_selection,
                    used_variables,
                    variable_definitions,
                    top_level_client_field_info,
                )
            },
            &mut |linked_field_selection| {
                validate_field_type_exists_and_is_linked(
                    field_parent_object,
                    linked_field_selection,
                    &mut used_variables2,
                    variable_definitions,
                    top_level_client_field_info,
                )
            },
        )
    });

    used_variables.append(&mut used_variables2);

    validated_selection
}

/// Given that we selected a scalar field, the field should exist on the parent,
/// and type should be a client field (which is a scalar) or a server scalar type.
fn validate_field_type_exists_and_is_scalar(
    scalar_field_selection_parent_object: &SchemaObject,
    scalar_field_selection: UnvalidatedScalarFieldSelection,
    used_variables: &mut UsedVariables,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> ValidateSchemaResult<ValidatedScalarFieldSelection> {
    let scalar_field_name = scalar_field_selection.name.item.into();
    match scalar_field_selection_parent_object
        .encountered_fields
        .get(&scalar_field_name)
    {
        Some(defined_field_type) => match defined_field_type {
            FieldType::ServerField(server_field_id) => {
                let server_field =
                    &top_level_client_field_info.server_fields[server_field_id.as_usize()];
                let missing_arguments = get_missing_arguments_and_validate_argument_types(
                    top_level_client_field_info.schema_data,
                    server_field
                        .arguments
                        .iter()
                        .map(|variable_definition| &variable_definition.item),
                    &scalar_field_selection.arguments,
                    false,
                    scalar_field_selection.name.location,
                    used_variables,
                    variable_definitions,
                )?;

                match &server_field.associated_data {
                    SelectionType::Scalar(_) => Ok(ScalarFieldSelection {
                        name: scalar_field_selection.name,
                        associated_data: ValidatedScalarFieldAssociatedData {
                            location: FieldType::ServerField(*server_field_id),
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
                        arguments: scalar_field_selection.arguments,
                        directives: scalar_field_selection.directives,
                    }),
                    SelectionType::Object(object_id) => Err(WithLocation::new(
                        ValidateSchemaError::ClientFieldSelectionFieldIsNotScalar {
                            field_parent_type_name: scalar_field_selection_parent_object.name,
                            field_name: scalar_field_name,
                            field_type: "an object",
                            target_type_name: top_level_client_field_info
                                .schema_data
                                .object(object_id.type_name.inner_non_null())
                                .name
                                .into(),
                            client_field_parent_type_name: top_level_client_field_info
                                .client_field_type_and_field_name
                                .type_name,
                            client_field_name: top_level_client_field_info
                                .client_field_type_and_field_name
                                .field_name,
                        },
                        scalar_field_selection.name.location,
                    )),
                }
            }
            FieldType::ClientField(ClientType::ClientField(client_field_id)) => {
                validate_client_field(
                    client_field_id,
                    scalar_field_selection,
                    used_variables,
                    variable_definitions,
                    top_level_client_field_info,
                )
            }
        },
        None => Err(WithLocation::new(
            ValidateSchemaError::ClientFieldSelectionFieldDoesNotExist {
                client_field_parent_type_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .type_name,
                client_field_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .field_name,
                field_parent_type_name: scalar_field_selection_parent_object.name,
                field_name: scalar_field_name,
            },
            scalar_field_selection.name.location,
        )),
    }
}

fn validate_client_field(
    client_field_id: &ClientFieldId,
    scalar_field_selection: UnvalidatedScalarFieldSelection,
    used_variables: &mut UsedVariables,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> ValidateSchemaResult<ValidatedScalarFieldSelection> {
    let argument_definitions = top_level_client_field_info
        .client_field_args
        .get(&ClientType::ClientField(*client_field_id))
        .expect(
            "Expected client field to exist in map. \
            This is indicative of a bug in Isograph.",
        );
    let missing_arguments = get_missing_arguments_and_validate_argument_types(
        top_level_client_field_info.schema_data,
        argument_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &scalar_field_selection.arguments,
        false,
        scalar_field_selection.name.location,
        used_variables,
        variable_definitions,
    )?;

    Ok(ScalarFieldSelection {
        name: scalar_field_selection.name,
        reader_alias: scalar_field_selection.reader_alias,
        associated_data: ValidatedScalarFieldAssociatedData {
            location: FieldType::ClientField(*client_field_id),
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
    field_parent_object: &SchemaObject,
    linked_field_selection: UnvalidatedLinkedFieldSelection,
    used_variables: &mut UsedVariables,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    top_level_client_field_info: &ValidateSchemaSharedInfo<'_>,
) -> ValidateSchemaResult<ValidatedLinkedFieldSelection> {
    let linked_field_name = linked_field_selection.name.item.into();
    match (field_parent_object.encountered_fields).get(&linked_field_name) {
        Some(defined_field_type) => match defined_field_type {
            FieldType::ServerField(server_field_id) => {
                let server_field =
                    &top_level_client_field_info.server_fields[server_field_id.as_usize()];
                match &server_field.associated_data {
                    SelectionType::Scalar(scalar_id) => Err(WithLocation::new(
                        ValidateSchemaError::ClientFieldSelectionFieldIsScalar {
                            field_parent_type_name: field_parent_object.name,
                            field_name: linked_field_name,
                            field_type: "a scalar",
                            target_type_name: top_level_client_field_info
                                .schema_data
                                .scalar(scalar_id.inner_non_null())
                                .name
                                .item
                                .into(),
                            client_field_parent_type_name: top_level_client_field_info
                                .client_field_type_and_field_name
                                .type_name,
                            client_field_name: top_level_client_field_info
                                .client_field_type_and_field_name
                                .field_name,
                        },
                        linked_field_selection.name.location,
                    )),
                    SelectionType::Object(object_id) => {
                        let linked_field_target_object = top_level_client_field_info
                            .schema_data
                            .server_objects
                            .get(object_id.type_name.inner_non_null().as_usize())
                            .unwrap();

                        let missing_arguments = get_missing_arguments_and_validate_argument_types(
                            top_level_client_field_info.schema_data,
                            server_field
                                .arguments
                                .iter()
                                .map(|variable_definition| &variable_definition.item),
                            &linked_field_selection.arguments,
                            false,
                            linked_field_selection.name.location,
                            used_variables,
                            variable_definitions,
                        )?;

                        Ok(LinkedFieldSelection {
                            name: linked_field_selection.name,
                            reader_alias: linked_field_selection.reader_alias,
                            selection_set: linked_field_selection.selection_set.into_iter().map(
                                |selection| {
                                    validate_client_field_definition_selection_exists_and_type_matches(
                                        selection,
                                        linked_field_target_object,
                                        used_variables,
                                        variable_definitions,
                                        top_level_client_field_info
                                    )
                                },
                            ).collect::<Result<Vec<_>, _>>()?,
                            associated_data: ValidatedLinkedFieldAssociatedData {
                                concrete_type: linked_field_target_object.concrete_type,
                                parent_object_id: object_id.type_name.inner_non_null(),
                                field_id: FieldType::ServerField(server_field.id),
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
                            directives: linked_field_selection.directives,
                        })
                    }
                }
            }
            FieldType::ClientField(_) => Err(WithLocation::new(
                ValidateSchemaError::ClientFieldSelectionClientFieldSelectedAsLinked {
                    field_parent_type_name: field_parent_object.name,
                    field_name: linked_field_name,
                    client_field_parent_type_name: top_level_client_field_info
                        .client_field_type_and_field_name
                        .type_name,
                    client_field_name: top_level_client_field_info
                        .client_field_type_and_field_name
                        .field_name,
                },
                linked_field_selection.name.location,
            )),
        },
        None => Err(WithLocation::new(
            ValidateSchemaError::ClientFieldSelectionFieldDoesNotExist {
                client_field_parent_type_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .type_name,
                client_field_name: top_level_client_field_info
                    .client_field_type_and_field_name
                    .field_name,
                field_parent_type_name: field_parent_object.name,
                field_name: linked_field_name,
            },
            linked_field_selection.name.location,
        )),
    }
}

fn server_field_cannot_be_selected_loadably(
    server_field_name: SelectableFieldName,
    location: Location,
) -> ValidateSchemaResult<()> {
    Err(WithLocation::new(
        ValidateSchemaError::ServerFieldCannotBeSelectedLoadably { server_field_name },
        location,
    ))
}

fn assert_no_missing_arguments(
    missing_arguments: Vec<ValidatedVariableDefinition>,
    location: Location,
) -> ValidateSchemaResult<()> {
    if !missing_arguments.is_empty() {
        return Err(WithLocation::new(
            ValidateSchemaError::MissingArguments { missing_arguments },
            location,
        ));
    }
    Ok(())
}

fn get_missing_arguments_and_validate_argument_types<'a>(
    schema_data: &ServerFieldData,
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
    include_optional_args: bool,
    location: Location,
    used_variables: &mut UsedVariables,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
) -> ValidateSchemaResult<Vec<ValidatedVariableDefinition>> {
    let reachable_variables = validate_no_undefined_variables_and_get_reachable_variables(
        arguments,
        variable_definitions,
    )?;
    used_variables.extend(reachable_variables.iter().map(|x| x.item));

    let argument_definitions_vec: Vec<_> = argument_definitions.collect();
    validate_no_extraneous_arguments(&argument_definitions_vec, arguments, location)?;

    get_missing_arguments_and_validate_types(
        schema_data,
        &argument_definitions_vec,
        arguments,
        include_optional_args,
        variable_definitions,
    )
}

fn scalar_literal_satisfies_type(
    scalar_literal: &ServerScalarId,
    type_: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData,
    location: Location,
) -> Result<(), WithLocation<ValidateSchemaError>> {
    match type_.clone().into() {
        GraphQLNonNullTypeAnnotation::List(_) => {
            let actual = schema_data.scalar(*scalar_literal).name.item;

            Err(WithLocation::new(
                ValidateSchemaError::ExpectedTypeFoundScalar {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                    actual,
                },
                location,
            ))
        }
        GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.item {
            SelectionType::Scalar(expected_scalar_id) => {
                if expected_scalar_id == *scalar_literal {
                    return Ok(());
                }
                let actual = schema_data.scalar(*scalar_literal).name.item;

                let expected = id_annotation_to_typename_annotation(type_, schema_data);

                Err(WithLocation::new(
                    ValidateSchemaError::ExpectedTypeFoundScalar { expected, actual },
                    location,
                ))
            }
            SelectionType::Object(_) => {
                let actual = schema_data.scalar(*scalar_literal).name.item;

                let expected = id_annotation_to_typename_annotation(type_, schema_data);

                Err(WithLocation::new(
                    ValidateSchemaError::ExpectedTypeFoundScalar { expected, actual },
                    location,
                ))
            }
        },
    }
}

fn variable_type_satisfies_argument_type(
    variable_type: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    argument_type: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData,
    location: Location,
) -> Result<(), WithLocation<ValidateSchemaError>> {
    match (variable_type.clone().into(), argument_type) {
        (
            // [Value]! satisfies [Value]
            // or [Value] satisfies [Value]
            GraphQLNonNullTypeAnnotation::List(list_variable_type),
            GraphQLTypeAnnotation::List(list_type),
        ) => variable_type_satisfies_argument_type(
            &list_variable_type,
            &list_type.0,
            schema_data,
            location,
        ),
        (
            // Value! satisfies Value
            // or Value satisfies Value
            GraphQLNonNullTypeAnnotation::Named(named_variable_type),
            GraphQLTypeAnnotation::Named(named_type),
        ) if named_variable_type.item == named_type.item => Ok(()),

        (_, GraphQLTypeAnnotation::NonNull(non_null_argument_type)) => match variable_type {
            // Value! satisfies Value!
            GraphQLTypeAnnotation::NonNull(variable_type) => variable_type_satisfies_argument_type(
                &GraphQLTypeAnnotation::from(*variable_type.clone()),
                &GraphQLTypeAnnotation::from(*non_null_argument_type.clone()),
                schema_data,
                location,
            ),
            // Value does not satisfy Value!
            // or [Value] does not satisfy Value!
            GraphQLTypeAnnotation::Named(_) | GraphQLTypeAnnotation::List(_) => {
                let expected = id_annotation_to_typename_annotation(argument_type, schema_data);
                let actual = id_annotation_to_typename_annotation(variable_type, schema_data);

                Err(WithLocation::new(
                    ValidateSchemaError::ExpectedType { expected, actual },
                    location,
                ))
            }
        },
        (_, _) => {
            let expected = id_annotation_to_typename_annotation(argument_type, schema_data);
            let actual = id_annotation_to_typename_annotation(variable_type, schema_data);

            Err(WithLocation::new(
                ValidateSchemaError::ExpectedType { expected, actual },
                location,
            ))
        }
    }
}

fn value_satisfies_type(
    value: &WithLocation<NonConstantValue>,
    type_: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerFieldData,
) -> ValidateSchemaResult<()> {
    match &value.item {
        NonConstantValue::Variable(variable_name) => variable_type_satisfies_argument_type(
            get_variable_type(variable_name, variable_definitions, value.location)?,
            type_,
            schema_data,
            value.location,
        ),
        NonConstantValue::Integer(_) => scalar_literal_satisfies_type(
            &schema_data.int_type_id,
            type_,
            schema_data,
            value.location,
        ),
        NonConstantValue::Boolean(_) => scalar_literal_satisfies_type(
            &schema_data.boolean_type_id,
            type_,
            schema_data,
            value.location,
        ),
        NonConstantValue::String(_) => scalar_literal_satisfies_type(
            &schema_data.string_type_id,
            type_,
            schema_data,
            value.location,
        ),
        NonConstantValue::Float(_) => scalar_literal_satisfies_type(
            &schema_data.float_type_id,
            type_,
            schema_data,
            value.location,
        ),
        NonConstantValue::Enum(enum_literal_value) => match type_.clone().into() {
            GraphQLNonNullTypeAnnotation::List(_) => Err(WithLocation::new(
                ValidateSchemaError::ExpectedTypeFoundEnum {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                    actual: *enum_literal_value,
                },
                value.location,
            )),
            GraphQLNonNullTypeAnnotation::Named(named_type) => {
                enum_satisfies_type(enum_literal_value, &named_type, schema_data, value.location)
            }
        },
        NonConstantValue::Null => match type_ {
            GraphQLTypeAnnotation::NonNull(_) => Err(WithLocation::new(
                ValidateSchemaError::ExpectedNonNullTypeFoundNull {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                },
                value.location,
            )),
            GraphQLTypeAnnotation::List(_) => Ok(()),
            GraphQLTypeAnnotation::Named(_) => Ok(()),
        },
        NonConstantValue::List(list) => match type_.clone().into() {
            GraphQLNonNullTypeAnnotation::List(list_type) => {
                list_satisfies_type(list, list_type, variable_definitions, schema_data)
            }
            GraphQLNonNullTypeAnnotation::Named(_) => Err(WithLocation::new(
                ValidateSchemaError::ExpectedTypeFoundList {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                },
                value.location,
            )),
        },
        NonConstantValue::Object(_object_literal) => match type_.clone().into() {
            GraphQLNonNullTypeAnnotation::List(_) => Err(WithLocation::new(
                ValidateSchemaError::ExpectedTypeFoundObject {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                },
                value.location,
            )),
            GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.0.item {
                SelectionType::Scalar(_) => Err(WithLocation::new(
                    ValidateSchemaError::ExpectedTypeFoundObject {
                        expected: id_annotation_to_typename_annotation(type_, schema_data),
                    },
                    value.location,
                )),
                SelectionType::Object(object_id) => {
                    let _object = schema_data.object(object_id);

                    todo!("Validate object literal. Parser doesn't support object literals yet");
                }
            },
        },
    }
}

fn id_annotation_to_typename_annotation(
    type_: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    type_.clone().map(|type_id| match type_id {
        SelectionType::Scalar(scalar_id) => schema_data.scalar(scalar_id).name.item.into(),
        SelectionType::Object(object_id) => schema_data.object(object_id).name.into(),
    })
}

fn enum_satisfies_type(
    enum_literal_value: &EnumLiteralValue,
    enum_type: &GraphQLNamedTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData,
    location: Location,
) -> ValidateSchemaResult<()> {
    match enum_type.item {
        SelectionType::Object(object_id) => {
            let expected = GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                enum_type
                    .clone()
                    .map(|_| schema_data.object(object_id).name.into()),
            ));

            Err(WithLocation::new(
                ValidateSchemaError::ExpectedTypeFoundEnum {
                    expected,
                    actual: *enum_literal_value,
                },
                location,
            ))
        }
        SelectionType::Scalar(_scalar_id) => {
            todo!("Validate enum literal. Parser doesn't support enum literals yet")
        }
    }
}

fn list_satisfies_type(
    list: &[WithLocation<NonConstantValue>],
    list_type: GraphQLListTypeAnnotation<SelectableServerFieldId>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerFieldData,
) -> ValidateSchemaResult<()> {
    list.iter().try_for_each(|element| {
        value_satisfies_type(element, &list_type.0, variable_definitions, schema_data)
    })
}

fn get_variable_type<'a>(
    variable_name: &'a VariableName,
    variable_definitions: &'a [WithSpan<ValidatedVariableDefinition>],
    location: Location,
) -> ValidateSchemaResult<&'a GraphQLTypeAnnotation<SelectableServerFieldId>> {
    match variable_definitions
        .iter()
        .find(|definition| definition.item.name.item == *variable_name)
    {
        Some(variable) => Ok(&variable.item.type_),
        None => Err(WithLocation::new(
            ValidateSchemaError::UsedUndefinedVariable {
                undefined_variable: *variable_name,
            },
            location,
        )),
    }
}

pub fn get_missing_arguments_and_validate_types(
    schema_data: &ServerFieldData,
    argument_definitions: &[&ValidatedVariableDefinition],
    arguments: &[WithLocation<SelectionFieldArgument>],
    include_optional_args: bool,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
) -> ValidateSchemaResult<Vec<ValidatedVariableDefinition>> {
    argument_definitions
        .iter()
        .filter_map(|definition| {
            if definition.default_value.is_some()
                || definition.type_.is_nullable() && !include_optional_args
            {
                return None;
            }

            let user_supplied_argument = arguments
                .iter()
                // TODO do not call .lookup
                .find(|arg| definition.name.item.lookup() == arg.item.name.item.lookup());

            if let Some(user_supplied_argument) = user_supplied_argument {
                match value_satisfies_type(
                    &user_supplied_argument.item.value,
                    &definition.type_,
                    variable_definitions,
                    schema_data,
                ) {
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                }
            } else {
                Some(Ok((*definition).clone()))
            }
        })
        .collect()
}

fn validate_no_undefined_variables_and_get_reachable_variables(
    arguments: &[WithLocation<SelectionFieldArgument>],
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
) -> ValidateSchemaResult<Vec<WithLocation<VariableName>>> {
    let mut all_reachable_variables = vec![];
    for argument in arguments {
        let reachable_variables = reachable_variables(&argument.item.value);
        for reachable_variable in reachable_variables.iter() {
            if variable_definitions.iter().all(|variable_definition| {
                variable_definition.item.name.item != reachable_variable.item
            }) {
                return Err(WithLocation::new(
                    ValidateSchemaError::UsedUndefinedVariable {
                        undefined_variable: reachable_variable.item,
                    },
                    argument.location,
                ));
            }
        }
        all_reachable_variables.extend(reachable_variables);
    }

    Ok(all_reachable_variables)
}

fn validate_no_extraneous_arguments(
    argument_definitions: &[&ValidatedVariableDefinition],
    arguments: &[WithLocation<SelectionFieldArgument>],
    location: Location,
) -> ValidateSchemaResult<()> {
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
            ValidateSchemaError::ExtraneousArgument { extra_arguments },
            location,
        ));
    }
    Ok(())
}
