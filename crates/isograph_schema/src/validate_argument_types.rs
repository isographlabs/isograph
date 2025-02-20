use common_lang_types::{
    EnumLiteralValue, Location, SelectableFieldName, Span, UnvalidatedTypeName, ValueKeyName,
    VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation, NameValuePair,
};
use intern::Lookup;
use std::collections::BTreeMap;
use std::fmt::Debug;

use isograph_lang_types::{
    ClientFieldId, ClientPointerId, NonConstantValue, SelectableServerFieldId, SelectionType,
    ServerFieldId, ServerObjectId, ServerScalarId, TypeAnnotation, UnionTypeAnnotation,
    UnionVariant, VariableDefinition,
};

use crate::{
    ClientType, FieldType, OutputFormat, SchemaObject, ServerFieldData, ValidateSchemaError,
    ValidateSchemaResult, ValidatedSchemaServerField, ValidatedVariableDefinition,
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

fn scalar_literal_satisfies_type<TOutputFormat: OutputFormat>(
    scalar_literal: &ServerScalarId,
    type_: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData<TOutputFormat>,
    location: Location,
) -> Result<(), WithLocation<ValidateSchemaError>> {
    match graphql_type_to_non_null_type(type_.clone()) {
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

pub fn value_satisfies_type<TOutputFormat: OutputFormat>(
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    field_argument_definition_type: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerFieldData<TOutputFormat>,
    server_fields: &[ValidatedSchemaServerField<TOutputFormat>],
) -> ValidateSchemaResult<()> {
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
                    ValidateSchemaError::ExpectedTypeFoundVariable {
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
                    ValidateSchemaError::ExpectedTypeFoundEnum {
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
                    ValidateSchemaError::ExpectedNonNullTypeFoundNull {
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
                    server_fields,
                ),
                GraphQLNonNullTypeAnnotation::Named(_) => Err(WithLocation::new(
                    ValidateSchemaError::ExpectedTypeFoundList {
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
                    ValidateSchemaError::ExpectedTypeFoundObject {
                        expected: id_annotation_to_typename_annotation(
                            field_argument_definition_type,
                            schema_data,
                        ),
                    },
                    selection_supplied_argument_value.location,
                )),
                GraphQLNonNullTypeAnnotation::Named(named_type) => match named_type.0.item {
                    SelectionType::Scalar(_) => Err(WithLocation::new(
                        ValidateSchemaError::ExpectedTypeFoundObject {
                            expected: id_annotation_to_typename_annotation(
                                field_argument_definition_type,
                                schema_data,
                            ),
                        },
                        selection_supplied_argument_value.location,
                    )),
                    SelectionType::Object(object_id) => object_satisfies_type(
                        selection_supplied_argument_value,
                        variable_definitions,
                        schema_data,
                        server_fields,
                        object_literal,
                        object_id,
                    ),
                },
            }
        }
    }
}

fn object_satisfies_type<TOutputFormat: OutputFormat>(
    selection_supplied_argument_value: &WithLocation<NonConstantValue>,
    variable_definitions: &[WithSpan<
        VariableDefinition<SelectionType<ServerObjectId, ServerScalarId>>,
    >],
    schema_data: &ServerFieldData<TOutputFormat>,
    server_fields: &[ValidatedSchemaServerField<TOutputFormat>],
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_id: ServerObjectId,
) -> Result<(), WithLocation<ValidateSchemaError>> {
    let object = schema_data.object(object_id);
    validate_no_extraneous_fields(
        &object.encountered_fields,
        object_literal,
        selection_supplied_argument_value.location,
    )?;

    let missing_fields = get_missing_and_provided_fields(server_fields, object_literal, object)
        .iter()
        .filter_map(|field| match field {
            ObjectLiteralFieldType::Provided(
                field_type_annotation,
                selection_supplied_argument_value,
            ) => match value_satisfies_type(
                &selection_supplied_argument_value.value,
                field_type_annotation,
                variable_definitions,
                schema_data,
                server_fields,
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
            ValidateSchemaError::MissingFields {
                missing_fields_names: missing_fields,
            },
            selection_supplied_argument_value.location,
        ))
    }
}

enum ObjectLiteralFieldType {
    Provided(
        GraphQLTypeAnnotation<SelectableServerFieldId>,
        NameValuePair<ValueKeyName, NonConstantValue>,
    ),
    Missing(SelectableFieldName),
}

fn get_missing_and_provided_fields<TOutputFormat: OutputFormat>(
    server_fields: &[ValidatedSchemaServerField<TOutputFormat>],
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object: &SchemaObject<TOutputFormat>,
) -> Vec<ObjectLiteralFieldType> {
    object
        .encountered_fields
        .iter()
        .filter_map(|(field_name, field_type)| {
            let field = &server_fields[field_type.as_server_field()?.as_usize()];

            let field_type_annotation = match &field.associated_data {
                SelectionType::Object(associated_data) => associated_data
                    .type_name
                    .clone()
                    .map(&mut |object_id| SelectionType::Object(object_id)),
                SelectionType::Scalar(type_name) => type_name
                    .clone()
                    .map(&mut |scalar_id| SelectionType::Scalar(scalar_id)),
            };
            let field_type_annotation =
                graphl_type_annotation_from_type_annotation(field_type_annotation);

            let object_literal_supplied_field = object_literal
                .iter()
                .find(|field| field.name.item.lookup() == field_name.lookup());

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
    object_fields: &BTreeMap<
        SelectableFieldName,
        FieldType<ServerFieldId, ClientType<ClientFieldId, ClientPointerId>>,
    >,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    location: Location,
) -> ValidateSchemaResult<()> {
    let extra_fields: Vec<_> = object_literal
        .iter()
        .filter_map(|field| {
            let is_defined = object_fields
                .iter()
                .any(|(field_name, _)| field_name.lookup() == field.name.item.lookup());

            if !is_defined {
                return Some(field.clone());
            }
            None
        })
        .collect();

    if !extra_fields.is_empty() {
        return Err(WithLocation::new(
            ValidateSchemaError::ExtraneousFields { extra_fields },
            location,
        ));
    }
    Ok(())
}

fn id_annotation_to_typename_annotation<TOutputFormat: OutputFormat>(
    type_: &GraphQLTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData<TOutputFormat>,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    type_.clone().map(|type_id| match type_id {
        SelectionType::Scalar(scalar_id) => schema_data.scalar(scalar_id).name.item.into(),
        SelectionType::Object(object_id) => schema_data.object(object_id).name.into(),
    })
}

fn enum_satisfies_type<TOutputFormat: OutputFormat>(
    enum_literal_value: &EnumLiteralValue,
    enum_type: &GraphQLNamedTypeAnnotation<SelectableServerFieldId>,
    schema_data: &ServerFieldData<TOutputFormat>,
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

fn list_satisfies_type<TOutputFormat: OutputFormat>(
    list: &[WithLocation<NonConstantValue>],
    list_type: GraphQLListTypeAnnotation<SelectableServerFieldId>,
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    schema_data: &ServerFieldData<TOutputFormat>,
    server_fields: &[ValidatedSchemaServerField<TOutputFormat>],
) -> ValidateSchemaResult<()> {
    list.iter().try_for_each(|element| {
        value_satisfies_type(
            element,
            &list_type.0,
            variable_definitions,
            schema_data,
            server_fields,
        )
    })
}

fn graphl_type_annotation_from_type_annotation<TValue: Ord + Copy + Debug>(
    other: TypeAnnotation<TValue>,
) -> GraphQLTypeAnnotation<TValue> {
    match other {
        TypeAnnotation::Scalar(scalar_id) => GraphQLTypeAnnotation::Named(
            GraphQLNamedTypeAnnotation(WithSpan::new(scalar_id, Span::todo_generated())),
        ),
        TypeAnnotation::Plural(type_annotation) => {
            GraphQLTypeAnnotation::List(Box::new(GraphQLListTypeAnnotation(
                graphl_type_annotation_from_type_annotation(*type_annotation),
            )))
        }
        TypeAnnotation::Union(union_type_annotation) => {
            graphl_type_annotation_from_union_variant(union_type_annotation)
        }
    }
}

fn graphl_type_annotation_from_union_variant<TValue: Ord + Copy + Debug>(
    union_type_annotation: UnionTypeAnnotation<TValue>,
) -> GraphQLTypeAnnotation<TValue> {
    if union_type_annotation.nullable {
        return match union_type_annotation.variants.into_iter().next().unwrap() {
            UnionVariant::Scalar(scalar_id) => GraphQLTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(scalar_id, Span::todo_generated())),
            ),
            UnionVariant::Plural(type_annotation) => {
                GraphQLTypeAnnotation::List(Box::new(GraphQLListTypeAnnotation(
                    graphl_type_annotation_from_type_annotation(type_annotation),
                )))
            }
        };
    }

    GraphQLTypeAnnotation::NonNull(
        match union_type_annotation.variants.into_iter().next().unwrap() {
            UnionVariant::Scalar(scalar_id) => Box::new(GraphQLNonNullTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(scalar_id, Span::todo_generated())),
            )),
            UnionVariant::Plural(type_annotation) => Box::new(GraphQLNonNullTypeAnnotation::List(
                GraphQLListTypeAnnotation(graphl_type_annotation_from_type_annotation(
                    type_annotation,
                )),
            )),
        },
    )
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
