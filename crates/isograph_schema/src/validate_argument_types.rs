use common_lang_types::{
    Diagnostic, EnumLiteralValue, Location, SelectableName, ServerObjectEntityName,
    ServerScalarEntityName, UnvalidatedTypeName, ValueKeyName, VariableName, WithLocation,
    WithLocationPostfix, WithSpan,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation, NameValuePair,
};
use intern::Lookup;
use prelude::Postfix;
use thiserror::Error;

use isograph_lang_types::{
    NonConstantValue, SelectionType, VariableDefinition,
    graphql_type_annotation_from_type_annotation,
};

use crate::{
    BOOLEAN_ENTITY_NAME, FLOAT_ENTITY_NAME, ID_ENTITY_NAME, INT_ENTITY_NAME, IsographDatabase,
    NetworkProtocol, STRING_ENTITY_NAME, ServerEntityName, ValidatedVariableDefinition,
    server_selectables_map_for_entity,
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
        GraphQLNonNullTypeAnnotation::List(list) => GraphQLTypeAnnotation::List(list.boxed()),
    }
}

fn scalar_literal_satisfies_type(
    scalar_literal_name: ServerScalarEntityName,
    type_: &GraphQLTypeAnnotation<ServerEntityName>,
    location: Location,
) -> Result<(), WithLocation<ValidateArgumentTypesError>> {
    match graphql_type_to_non_null_type(type_.clone()) {
        GraphQLNonNullTypeAnnotation::List(_) => {
            ValidateArgumentTypesError::ExpectedTypeFoundScalar {
                expected: id_annotation_to_typename_annotation(type_),
                actual: scalar_literal_name,
            }
            .with_location(location)
            .err()
        }
        GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.item {
            SelectionType::Scalar(expected_scalar_entity_name) => {
                if expected_scalar_entity_name == scalar_literal_name {
                    return Ok(());
                }

                let expected = id_annotation_to_typename_annotation(type_);

                ValidateArgumentTypesError::ExpectedTypeFoundScalar {
                    expected,
                    actual: scalar_literal_name,
                }
                .with_location(location)
                .err()
            }
            SelectionType::Object(_) => {
                let expected = id_annotation_to_typename_annotation(type_);

                ValidateArgumentTypesError::ExpectedTypeFoundScalar {
                    expected,
                    actual: scalar_literal_name,
                }
                .with_location(location)
                .err()
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
    db: &IsographDatabase<TNetworkProtocol>,
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    field_argument_definition_type: &GraphQLTypeAnnotation<ServerEntityName>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
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
                let expected = id_annotation_to_typename_annotation(field_argument_definition_type);
                let actual = id_annotation_to_typename_annotation(variable_type);

                ValidateArgumentTypesError::ExpectedTypeFoundVariable {
                    expected_type: expected,
                    variable_type: actual,
                    variable_name: *variable_name,
                }
                .with_location(selection_supplied_argument_value.location)
                .err()
            }
        }
        NonConstantValue::Integer(_) => scalar_literal_satisfies_type(
            *INT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *FLOAT_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        })
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Boolean(_) => scalar_literal_satisfies_type(
            *BOOLEAN_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
        ),
        NonConstantValue::String(_) => scalar_literal_satisfies_type(
            *STRING_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Float(_) => scalar_literal_satisfies_type(
            *FLOAT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.location,
        ),
        NonConstantValue::Enum(enum_literal_value) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => {
                    ValidateArgumentTypesError::ExpectedTypeFoundEnum {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                        ),
                        actual: *enum_literal_value,
                    }
                    .with_location(selection_supplied_argument_value.location)
                    .err()
                }
                GraphQLNonNullTypeAnnotation::Named(named_type) => enum_satisfies_type(
                    enum_literal_value,
                    &named_type,
                    selection_supplied_argument_value.location,
                ),
            }
        }
        NonConstantValue::Null => {
            if field_argument_definition_type.is_nullable() {
                Ok(())
            } else {
                ValidateArgumentTypesError::ExpectedNonNullTypeFoundNull {
                    expected: id_annotation_to_typename_annotation(field_argument_definition_type),
                }
                .with_location(selection_supplied_argument_value.location)
                .err()
            }
        }
        NonConstantValue::List(list) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(list_type) => {
                    list_satisfies_type(db, list, list_type, variable_definitions)
                }
                GraphQLNonNullTypeAnnotation::Named(_) => {
                    ValidateArgumentTypesError::ExpectedTypeFoundList {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                        ),
                    }
                    .with_location(selection_supplied_argument_value.location)
                    .err()
                }
            }
        }
        NonConstantValue::Object(object_literal) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => {
                    ValidateArgumentTypesError::ExpectedTypeFoundObject {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                        ),
                    }
                    .with_location(selection_supplied_argument_value.location)
                    .err()
                }
                GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.0.item {
                    SelectionType::Scalar(_) => {
                        ValidateArgumentTypesError::ExpectedTypeFoundObject {
                            expected: id_annotation_to_typename_annotation(
                                field_argument_definition_type,
                            ),
                        }
                        .with_location(selection_supplied_argument_value.location)
                        .err()
                    }
                    SelectionType::Object(object_entity_name) => object_satisfies_type(
                        db,
                        selection_supplied_argument_value,
                        variable_definitions,
                        object_literal,
                        object_entity_name,
                    ),
                },
            }
        }
    }
}

fn object_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    variable_definitions: &[WithSpan<VariableDefinition<ServerEntityName>>],
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_entity_name: ServerObjectEntityName,
) -> Result<(), WithLocation<ValidateArgumentTypesError>> {
    validate_no_extraneous_fields(
        db,
        object_entity_name,
        object_literal,
        selection_supplied_argument_value.location,
    )?;

    let missing_fields =
        get_non_nullable_missing_and_provided_fields(db, object_literal, object_entity_name)?
            .iter()
            .filter_map(|field| match field {
                ObjectLiteralFieldType::Provided(
                    field_type_annotation,
                    selection_supplied_argument_value,
                ) => match value_satisfies_type(
                    db,
                    &selection_supplied_argument_value.value,
                    field_type_annotation,
                    variable_definitions,
                ) {
                    Ok(_) => None,
                    Err(e) => e.err().some(),
                },
                ObjectLiteralFieldType::Missing(field_name) => (*field_name).ok().some(),
            })
            .collect::<Result<Vec<_>, _>>()?;

    if missing_fields.is_empty() {
        Ok(())
    } else {
        ValidateArgumentTypesError::MissingFields {
            missing_fields_names: missing_fields,
        }
        .with_location(selection_supplied_argument_value.location)
        .err()
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
    db: &IsographDatabase<TNetworkProtocol>,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    server_object_entity_name: ServerObjectEntityName,
) -> Result<Vec<ObjectLiteralFieldType>, WithLocation<ValidateArgumentTypesError>> {
    let server_selectables = server_selectables_map_for_entity(db, server_object_entity_name)
        .as_ref()
        .map_err(|e| {
            ValidateArgumentTypesError::ParseTypeSystemDocumentsError(e.clone())
                .with_generated_location()
        })?;

    server_selectables
        .iter()
        .filter_map(|(field_name, selectables)| {
            let first_selectable = selectables
                .first()
                .as_ref()
                .expect("Expected at least one selectable")
                .as_ref()
                .ok()?;

            let iso_type_annotation = match first_selectable.as_ref() {
                SelectionType::Scalar(server_scalar_selectable) => {
                    let field_type_annotation = &server_scalar_selectable.target_scalar_entity;
                    field_type_annotation
                        .clone()
                        .map(&mut SelectionType::Scalar)
                }
                SelectionType::Object(server_object_selectable) => {
                    let field_type_annotation = &server_object_selectable.target_object_entity;
                    field_type_annotation
                        .clone()
                        .map(&mut SelectionType::Object)
                }
            };

            let field_type_annotation =
                graphql_type_annotation_from_type_annotation(&iso_type_annotation);

            let object_literal_supplied_field = object_literal
                .iter()
                .find(|field| field.name.item.lookup() == (*field_name).lookup());

            match object_literal_supplied_field {
                Some(selection_supplied_argument_value) => ObjectLiteralFieldType::Provided(
                    field_type_annotation,
                    selection_supplied_argument_value.clone(),
                )
                .some(),
                None => match field_type_annotation {
                    GraphQLTypeAnnotation::NonNull(_) => {
                        ObjectLiteralFieldType::Missing((*field_name).into()).some()
                    }
                    GraphQLTypeAnnotation::List(_) | GraphQLTypeAnnotation::Named(_) => None,
                },
            }
        })
        .collect::<Vec<_>>()
        .ok()
}

fn validate_no_extraneous_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    location: Location,
) -> ValidateArgumentTypesResult<()> {
    let object_fields = server_selectables_map_for_entity(db, parent_server_object_entity_name)
        .as_ref()
        .map_err(|e| {
            ValidateArgumentTypesError::ParseTypeSystemDocumentsError(e.clone())
                .with_generated_location()
        })?;

    let extra_fields: Vec<_> = object_literal
        .iter()
        .filter_map(|field| {
            let is_defined = object_fields
                .get(&field.name.item.unchecked_conversion())
                .is_some();

            if !is_defined {
                return field.clone().some();
            }
            None
        })
        .collect();

    if !extra_fields.is_empty() {
        return ValidateArgumentTypesError::ExtraneousFields { extra_fields }
            .with_location(location)
            .err();
    }
    Ok(())
}

fn id_annotation_to_typename_annotation(
    type_: &GraphQLTypeAnnotation<ServerEntityName>,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    type_.clone().map(|type_id| match type_id {
        SelectionType::Scalar(scalar_entity_name) => scalar_entity_name.into(),
        SelectionType::Object(object_entity_name) => object_entity_name.into(),
    })
}

fn enum_satisfies_type(
    enum_literal_value: &EnumLiteralValue,
    enum_type: &GraphQLNamedTypeAnnotation<ServerEntityName>,
    location: Location,
) -> ValidateArgumentTypesResult<()> {
    match enum_type.item {
        SelectionType::Object(object_entity_name) => {
            let expected = GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                enum_type.clone().map(|_| object_entity_name.into()),
            ));

            ValidateArgumentTypesError::ExpectedTypeFoundEnum {
                expected,
                actual: *enum_literal_value,
            }
            .with_location(location)
            .err()
        }
        SelectionType::Scalar(_scalar_entity_name) => {
            todo!("Validate enum literal. Parser doesn't support enum literals yet")
        }
    }
}

fn list_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    list: &[WithLocation<NonConstantValue>],
    list_type: GraphQLListTypeAnnotation<ServerEntityName>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
) -> ValidateArgumentTypesResult<()> {
    list.iter().try_for_each(|element| {
        value_satisfies_type(db, element, &list_type.0, variable_definitions)
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
        Some(variable) => (&variable.item.type_).ok(),
        None => ValidateArgumentTypesError::UsedUndefinedVariable {
            undefined_variable: *variable_name,
        }
        .with_location(location)
        .err(),
    }
}

type ValidateArgumentTypesResult<T> = Result<T, WithLocation<ValidateArgumentTypesError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum ValidateArgumentTypesError {
    #[error(
        "Expected input of type {expected_type}, found variable {variable_name} of type {variable_type}"
    )]
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

    #[error("{0}")]
    ParseTypeSystemDocumentsError(Diagnostic),
}
