use std::collections::HashMap;

use common_lang_types::{
    EnumLiteralValue, Location, SelectableName, ServerObjectEntityName, ServerObjectSelectableName,
    ServerScalarEntityName, ServerScalarSelectableName, UnvalidatedTypeName, ValueKeyName,
    VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation, NameValuePair,
};
use thiserror::Error;

use isograph_lang_types::{
    graphql_type_annotation_from_type_annotation, NonConstantValue, SelectionType,
    VariableDefinition,
};

use crate::{
    NetworkProtocol, ServerEntityData, ServerEntityName, ServerObjectEntityAvailableSelectables,
    ServerObjectSelectable, ServerScalarSelectable, ValidatedVariableDefinition,
};

fn graphql_type_to_non_null_type<TValue>(
    value: GraphQLTypeAnnotation<TValue>,
) -> GraphQLNonNullTypeAnnotation<TValue> {
    match value {
        GraphQLTypeAnnotation::Named(named) => GraphQLNonNullTypeAnnotation::Named(named),
        GraphQLTypeAnnotation::List(list) => GraphQLNonNullTypeAnnotation::List(*list),
        GraphQLTypeAnnotation::NonNull(non_null) => *non_null,
    }
}

fn graphql_type_to_nullable_type<TValue>(
    value: GraphQLNonNullTypeAnnotation<TValue>,
) -> GraphQLTypeAnnotation<TValue> {
    match value {
        GraphQLNonNullTypeAnnotation::Named(named) => GraphQLTypeAnnotation::Named(named),
        GraphQLNonNullTypeAnnotation::List(list) => GraphQLTypeAnnotation::List(Box::new(list)),
    }
}

fn scalar_literal_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    scalar_literal_name: &ServerScalarEntityName,
    type_: &GraphQLTypeAnnotation<ServerEntityName>,
    schema_data: &ServerEntityData<TNetworkProtocol>,
    location: Location,
) -> Result<(), WithLocation<ValidateArgumentTypesError>> {
    match graphql_type_to_non_null_type(type_.clone()) {
        GraphQLNonNullTypeAnnotation::List(_) => {
            let actual = schema_data
                .server_scalar_entity(*scalar_literal_name)
                .expect(
                    "Expected entity to exist. \
                    This is indicative of a bug in Isograph.",
                )
                .name
                .item;

            Err(WithLocation::new(
                ValidateArgumentTypesError::ExpectedTypeFoundScalar {
                    expected: id_annotation_to_typename_annotation(type_, schema_data),
                    actual,
                },
                location,
            ))
        }
        GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.item {
            SelectionType::Scalar(expected_scalar_entity_name) => {
                if expected_scalar_entity_name == *scalar_literal_name {
                    return Ok(());
                }
                let actual = schema_data
                    .server_scalar_entity(*scalar_literal_name)
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .name
                    .item;

                let expected = id_annotation_to_typename_annotation(type_, schema_data);

                Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundScalar { expected, actual },
                    location,
                ))
            }
            SelectionType::Object(_) => {
                let actual = schema_data
                    .server_scalar_entity(*scalar_literal_name)
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .name
                    .item;

                let expected = id_annotation_to_typename_annotation(type_, schema_data);

                Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundScalar { expected, actual },
                    location,
                ))
            }
        },
    }
}

fn variable_type_satisfies_argument_type(
    variable_type: &GraphQLTypeAnnotation<ServerEntityName>,
    argument_type: &GraphQLTypeAnnotation<ServerEntityName>,
) -> bool {
    match argument_type {
        GraphQLTypeAnnotation::List(list_type) => {
            match graphql_type_to_non_null_type(variable_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(list_variable_type) => {
                    // [Value]! satisfies [Value]
                    // or [Value] satisfies [Value]
                    variable_type_satisfies_argument_type(&list_variable_type, &list_type.0)
                }
                GraphQLNonNullTypeAnnotation::Named(_) => false,
            }
        }

        GraphQLTypeAnnotation::Named(named_type) => {
            match graphql_type_to_non_null_type(variable_type.clone()) {
                GraphQLNonNullTypeAnnotation::Named(named_variable_type) => {
                    // Value! satisfies Value
                    // or Value satisfies Value
                    named_variable_type.item == named_type.item
                }
                GraphQLNonNullTypeAnnotation::List(_) => false,
            }
        }
        GraphQLTypeAnnotation::NonNull(non_null_argument_type) => match variable_type {
            // Value! satisfies Value!
            GraphQLTypeAnnotation::NonNull(variable_type) => variable_type_satisfies_argument_type(
                &graphql_type_to_nullable_type(*variable_type.clone()),
                &graphql_type_to_nullable_type(*non_null_argument_type.clone()),
            ),
            // Value does not satisfy Value!
            // or [Value] does not satisfy Value!
            GraphQLTypeAnnotation::Named(_) | GraphQLTypeAnnotation::List(_) => false,
        },
    }
}

pub fn value_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    field_argument_definition_type: &GraphQLTypeAnnotation<ServerEntityName>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerEntityData<TNetworkProtocol>,
    server_scalar_selectables: &HashMap<
        (ServerObjectEntityName, ServerScalarSelectableName),
        ServerScalarSelectable<TNetworkProtocol>,
    >,
    server_object_selectables: &HashMap<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ServerObjectSelectable<TNetworkProtocol>,
    >,
) -> ValidateArgumentTypesResult<()> {
    match &selection_supplied_argument_value.item {
        NonConstantValue::Variable(variable_name) => {
            let variable_type = get_variable_type(
                variable_name,
                variable_definitions,
                selection_supplied_argument_value.location,
            )?;
            if variable_type_satisfies_argument_type(variable_type, field_argument_definition_type)
            {
                Ok(())
            } else {
                let expected = id_annotation_to_typename_annotation(
                    field_argument_definition_type,
                    schema_data,
                );
                let actual = id_annotation_to_typename_annotation(variable_type, schema_data);

                Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundVariable {
                        expected_type: expected,
                        variable_type: actual,
                        variable_name: *variable_name,
                    },
                    selection_supplied_argument_value.location,
                ))
            }
        }
        NonConstantValue::Integer(_) => scalar_literal_satisfies_type(
            &schema_data.int_type_id,
            field_argument_definition_type,
            schema_data,
            selection_supplied_argument_value.location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                &schema_data.float_type_id,
                field_argument_definition_type,
                schema_data,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        })
        .or_else(|error| {
            scalar_literal_satisfies_type(
                &schema_data.id_type_id,
                field_argument_definition_type,
                schema_data,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Boolean(_) => scalar_literal_satisfies_type(
            &schema_data.boolean_type_id,
            field_argument_definition_type,
            schema_data,
            selection_supplied_argument_value.location,
        ),
        NonConstantValue::String(_) => scalar_literal_satisfies_type(
            &schema_data.string_type_id,
            field_argument_definition_type,
            schema_data,
            selection_supplied_argument_value.location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                &schema_data.id_type_id,
                field_argument_definition_type,
                schema_data,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Float(_) => scalar_literal_satisfies_type(
            &schema_data.float_type_id,
            field_argument_definition_type,
            schema_data,
            selection_supplied_argument_value.location,
        ),
        NonConstantValue::Enum(enum_literal_value) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundEnum {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                            schema_data,
                        ),
                        actual: *enum_literal_value,
                    },
                    selection_supplied_argument_value.location,
                )),
                GraphQLNonNullTypeAnnotation::Named(named_type) => enum_satisfies_type(
                    enum_literal_value,
                    &named_type,
                    schema_data,
                    selection_supplied_argument_value.location,
                ),
            }
        }
        NonConstantValue::Null => {
            if field_argument_definition_type.is_nullable() {
                Ok(())
            } else {
                Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedNonNullTypeFoundNull {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                            schema_data,
                        ),
                    },
                    selection_supplied_argument_value.location,
                ))
            }
        }
        NonConstantValue::List(list) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(list_type) => list_satisfies_type(
                    list,
                    list_type,
                    variable_definitions,
                    schema_data,
                    server_scalar_selectables,
                    server_object_selectables,
                ),
                GraphQLNonNullTypeAnnotation::Named(_) => Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundList {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                            schema_data,
                        ),
                    },
                    selection_supplied_argument_value.location,
                )),
            }
        }
        NonConstantValue::Object(object_literal) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => Err(WithLocation::new(
                    ValidateArgumentTypesError::ExpectedTypeFoundObject {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                            schema_data,
                        ),
                    },
                    selection_supplied_argument_value.location,
                )),
                GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.0.item {
                    SelectionType::Scalar(_) => Err(WithLocation::new(
                        ValidateArgumentTypesError::ExpectedTypeFoundObject {
                            expected: id_annotation_to_typename_annotation(
                                field_argument_definition_type,
                                schema_data,
                            ),
                        },
                        selection_supplied_argument_value.location,
                    )),
                    SelectionType::Object(object_entity_name) => object_satisfies_type(
                        selection_supplied_argument_value,
                        variable_definitions,
                        schema_data,
                        server_scalar_selectables,
                        server_object_selectables,
                        object_literal,
                        object_entity_name,
                    ),
                },
            }
        }
    }
}

fn object_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    variable_definitions: &[WithSpan<VariableDefinition<ServerEntityName>>],
    server_entity_data: &ServerEntityData<TNetworkProtocol>,
    server_scalar_selectables: &HashMap<
        (ServerObjectEntityName, ServerScalarSelectableName),
        ServerScalarSelectable<TNetworkProtocol>,
    >,
    server_object_selectables: &HashMap<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ServerObjectSelectable<TNetworkProtocol>,
    >,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_entity_name: ServerObjectEntityName,
) -> Result<(), WithLocation<ValidateArgumentTypesError>> {
    validate_no_extraneous_fields(
        &server_entity_data
            .server_object_entity_extra_info
            .get(&object_entity_name)
            .expect(
                "Expected object_entity_name to exist \
                in server_object_entity_available_selectables",
            )
            .selectables,
        object_literal,
        selection_supplied_argument_value.location,
    )?;

    let missing_fields = get_non_nullable_missing_and_provided_fields(
        server_entity_data,
        server_scalar_selectables,
        server_object_selectables,
        object_literal,
        object_entity_name,
    )
    .iter()
    .filter_map(|field| match field {
        ObjectLiteralFieldType::Provided(
            field_type_annotation,
            selection_supplied_argument_value,
        ) => match value_satisfies_type(
            &selection_supplied_argument_value.value,
            field_type_annotation,
            variable_definitions,
            server_entity_data,
            server_scalar_selectables,
            server_object_selectables,
        ) {
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        },
        ObjectLiteralFieldType::Missing(field_name) => Some(Ok(*field_name)),
    })
    .collect::<Result<Vec<_>, _>>()?;

    if missing_fields.is_empty() {
        Ok(())
    } else {
        Err(WithLocation::new(
            ValidateArgumentTypesError::MissingFields {
                missing_fields_names: missing_fields,
            },
            selection_supplied_argument_value.location,
        ))
    }
}

enum ObjectLiteralFieldType {
    Provided(
        GraphQLTypeAnnotation<ServerEntityName>,
        NameValuePair<ValueKeyName, NonConstantValue>,
    ),
    Missing(SelectableName),
}

fn get_non_nullable_missing_and_provided_fields<TNetworkProtocol: NetworkProtocol>(
    server_entity_data: &ServerEntityData<TNetworkProtocol>,
    server_scalar_selectables: &HashMap<
        (ServerObjectEntityName, ServerScalarSelectableName),
        ServerScalarSelectable<TNetworkProtocol>,
    >,
    server_object_selectables: &HashMap<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ServerObjectSelectable<TNetworkProtocol>,
    >,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_entity_name: ServerObjectEntityName,
) -> Vec<ObjectLiteralFieldType> {
    server_entity_data
        .server_object_entity_extra_info
        .get(&object_entity_name)
        .expect(
            "Expected object_entity_name to exist \
            in server_object_entity_available_selectables",
        )
        .selectables
        .iter()
        .filter_map(|(field_name, field_type)| {
            let iso_type_annotation = match field_type.as_server().as_ref()? {
                SelectionType::Scalar((
                    parent_object_entity_name,
                    scalar_scalar_selectable_name,
                )) => {
                    let field = &server_scalar_selectables
                        .get(&(*parent_object_entity_name, *scalar_scalar_selectable_name))
                        .expect("Expected server scalar selectable to exist");
                    let field_type_annotation = &field.target_scalar_entity;
                    field_type_annotation
                        .clone()
                        .map(&mut SelectionType::Scalar)
                }
                SelectionType::Object((
                    parent_object_entity_name,
                    server_object_selectable_name,
                )) => {
                    let field = &server_object_selectables
                        .get(&(*parent_object_entity_name, *server_object_selectable_name))
                        .expect("Expected server object selectable to exist");
                    let field_type_annotation = &field.target_object_entity;
                    field_type_annotation
                        .clone()
                        .map(&mut SelectionType::Object)
                }
            };

            let field_type_annotation =
                graphql_type_annotation_from_type_annotation(&iso_type_annotation);

            let object_literal_supplied_field = object_literal
                .iter()
                .find(|field| field.name.item == *field_name);

            match object_literal_supplied_field {
                Some(selection_supplied_argument_value) => Some(ObjectLiteralFieldType::Provided(
                    field_type_annotation,
                    selection_supplied_argument_value.clone(),
                )),
                None => match field_type_annotation {
                    GraphQLTypeAnnotation::NonNull(_) => {
                        Some(ObjectLiteralFieldType::Missing(*field_name))
                    }
                    GraphQLTypeAnnotation::List(_) | GraphQLTypeAnnotation::Named(_) => None,
                },
            }
        })
        .collect()
}

fn validate_no_extraneous_fields(
    object_fields: &ServerObjectEntityAvailableSelectables,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    location: Location,
) -> ValidateArgumentTypesResult<()> {
    let extra_fields: Vec<_> = object_literal
        .iter()
        .filter_map(|field| {
            let is_defined = object_fields
                .iter()
                .any(|(field_name, _)| *field_name == field.name.item);

            if !is_defined {
                return Some(field.clone());
            }
            None
        })
        .collect();

    if !extra_fields.is_empty() {
        return Err(WithLocation::new(
            ValidateArgumentTypesError::ExtraneousFields { extra_fields },
            location,
        ));
    }
    Ok(())
}

fn id_annotation_to_typename_annotation<TNetworkProtocol: NetworkProtocol>(
    type_: &GraphQLTypeAnnotation<ServerEntityName>,
    schema_data: &ServerEntityData<TNetworkProtocol>,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    type_.clone().map(|type_id| match type_id {
        SelectionType::Scalar(scalar_entity_name) => schema_data
            .server_scalar_entity(scalar_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name
            .item
            .into(),
        SelectionType::Object(object_entity_name) => schema_data
            .server_object_entity(object_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name.item
            .into(),
    })
}

fn enum_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    enum_literal_value: &EnumLiteralValue,
    enum_type: &GraphQLNamedTypeAnnotation<ServerEntityName>,
    schema_data: &ServerEntityData<TNetworkProtocol>,
    location: Location,
) -> ValidateArgumentTypesResult<()> {
    match enum_type.item {
        SelectionType::Object(object_entity_name) => {
            let expected = GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                enum_type.clone().map(|_| {
                    schema_data
                        .server_object_entity(object_entity_name)
                        .expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .name.item
                        .into()
                }),
            ));

            Err(WithLocation::new(
                ValidateArgumentTypesError::ExpectedTypeFoundEnum {
                    expected,
                    actual: *enum_literal_value,
                },
                location,
            ))
        }
        SelectionType::Scalar(_scalar_entity_name) => {
            todo!("Validate enum literal. Parser doesn't support enum literals yet")
        }
    }
}

fn list_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    list: &[WithLocation<NonConstantValue>],
    list_type: GraphQLListTypeAnnotation<ServerEntityName>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerEntityData<TNetworkProtocol>,
    server_scalar_selectables: &HashMap<
        (ServerObjectEntityName, ServerScalarSelectableName),
        ServerScalarSelectable<TNetworkProtocol>,
    >,
    server_object_selectables: &HashMap<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ServerObjectSelectable<TNetworkProtocol>,
    >,
) -> ValidateArgumentTypesResult<()> {
    list.iter().try_for_each(|element| {
        value_satisfies_type(
            element,
            &list_type.0,
            variable_definitions,
            schema_data,
            server_scalar_selectables,
            server_object_selectables,
        )
    })
}

fn get_variable_type<'a>(
    variable_name: &'a VariableName,
    variable_definitions: &'a [WithSpan<ValidatedVariableDefinition>],
    location: Location,
) -> ValidateArgumentTypesResult<&'a GraphQLTypeAnnotation<ServerEntityName>> {
    match variable_definitions
        .iter()
        .find(|definition| definition.item.name.item == *variable_name)
    {
        Some(variable) => Ok(&variable.item.type_),
        None => Err(WithLocation::new(
            ValidateArgumentTypesError::UsedUndefinedVariable {
                undefined_variable: *variable_name,
            },
            location,
        )),
    }
}

type ValidateArgumentTypesResult<T> = Result<T, WithLocation<ValidateArgumentTypesError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ValidateArgumentTypesError {
    #[error("Expected input of type {expected_type}, found variable {variable_name} of type {variable_type}")]
    ExpectedTypeFoundVariable {
        expected_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        variable_name: VariableName,
    },

    #[error("Expected input of type {expected}, found {actual} scalar literal")]
    ExpectedTypeFoundScalar {
        expected: GraphQLTypeAnnotation<UnvalidatedTypeName>,
        actual: ServerScalarEntityName,
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

    #[error("This variable is not defined: ${undefined_variable}")]
    UsedUndefinedVariable { undefined_variable: VariableName },

    #[error(
        "This object has missing fields: {0}",
        missing_fields_names.iter().map(|field_name| format!("${field_name}")).collect::<Vec<_>>().join(", ")
    )]
    MissingFields {
        missing_fields_names: Vec<SelectableName>,
    },

    #[error(
        "This object has extra fields: {0}",
        extra_fields.iter().map(|field| format!("{}", field.name.item)).collect::<Vec<_>>().join(", ")
    )]
    ExtraneousFields {
        extra_fields: Vec<NameValuePair<ValueKeyName, NonConstantValue>>,
    },
}
