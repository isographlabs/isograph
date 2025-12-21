use common_lang_types::{
    Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, Location, SelectableName,
    ValueKeyName, WithEmbeddedLocation,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation, NameValuePair,
};
use intern::{Lookup, string_key::StringKey};
use prelude::{ErrClone, Postfix};

use isograph_lang_types::{
    NonConstantValue, SelectionType, VariableDefinition, VariableNameWrapper,
    graphql_type_annotation_from_type_annotation,
};

use crate::{
    BOOLEAN_ENTITY_NAME, FLOAT_ENTITY_NAME, ID_ENTITY_NAME, INT_ENTITY_NAME, IsographDatabase,
    NetworkProtocol, STRING_ENTITY_NAME, server_selectables_map_for_entity,
};

fn graphql_type_to_non_null_type(value: GraphQLTypeAnnotation) -> GraphQLNonNullTypeAnnotation {
    match value {
        GraphQLTypeAnnotation::Named(named) => GraphQLNonNullTypeAnnotation::Named(named),
        GraphQLTypeAnnotation::List(list) => GraphQLNonNullTypeAnnotation::List(*list),
        GraphQLTypeAnnotation::NonNull(non_null) => *non_null,
    }
}

fn graphql_type_to_nullable_type(value: GraphQLNonNullTypeAnnotation) -> GraphQLTypeAnnotation {
    match value {
        GraphQLNonNullTypeAnnotation::Named(named) => GraphQLTypeAnnotation::Named(named),
        GraphQLNonNullTypeAnnotation::List(list) => GraphQLTypeAnnotation::List(list.boxed()),
    }
}

fn scalar_literal_satisfies_type(
    scalar_literal_name: EntityName,
    type_: &GraphQLTypeAnnotation,
    location: EmbeddedLocation,
) -> DiagnosticResult<()> {
    match graphql_type_to_non_null_type(type_.clone()) {
        GraphQLNonNullTypeAnnotation::List(_) => {
            expected_type_found_something_else_named_diagnostic(
                id_annotation_to_typename_annotation(type_),
                scalar_literal_name.unchecked_conversion(),
                "scalar literal",
                location,
            )
            .wrap_err()
        }
        GraphQLNonNullTypeAnnotation::Named(named_type) => {
            let expected_name = named_type.0;
            if expected_name == scalar_literal_name {
                return Ok(());
            }

            let expected = id_annotation_to_typename_annotation(type_);

            expected_type_found_something_else_named_diagnostic(
                expected,
                scalar_literal_name.unchecked_conversion(),
                "scalar literal",
                location,
            )
            .wrap_err()
        }
    }
}

fn variable_type_satisfies_argument_type(
    variable_type: &GraphQLTypeAnnotation,
    argument_type: &GraphQLTypeAnnotation,
) -> bool {
    match argument_type {
        GraphQLTypeAnnotation::List(list_type) => {
            match graphql_type_to_non_null_type(variable_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(list_variable_type) => {
                    // [Value]! satisfies [Value]
                    // or [Value] satisfies [Value]
                    variable_type_satisfies_argument_type(
                        list_variable_type.item.reference(),
                        list_type.item.reference(),
                    )
                }
                GraphQLNonNullTypeAnnotation::Named(_) => false,
            }
        }

        GraphQLTypeAnnotation::Named(named_type) => {
            match graphql_type_to_non_null_type(variable_type.clone()) {
                GraphQLNonNullTypeAnnotation::Named(named_variable_type) => {
                    // Value! satisfies Value
                    // or Value satisfies Value
                    named_variable_type == named_type.dereference()
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
    selection_supplied_argument_value: &WithEmbeddedLocation<NonConstantValue>,
    field_argument_definition_type: &GraphQLTypeAnnotation,
    variable_definitions: &[VariableDefinition],
) -> DiagnosticResult<()> {
    match &selection_supplied_argument_value.item {
        NonConstantValue::Variable(variable_name) => {
            let variable_type = get_variable_type(
                variable_name,
                variable_definitions,
                selection_supplied_argument_value.embedded_location,
            )?;
            if variable_type_satisfies_argument_type(variable_type, field_argument_definition_type)
            {
                Ok(())
            } else {
                let expected = id_annotation_to_typename_annotation(field_argument_definition_type);
                let actual = id_annotation_to_typename_annotation(variable_type);

                Diagnostic::new(
                    format!("Expected input of type {expected}, found {actual} scalar literal"),
                    selection_supplied_argument_value
                        .embedded_location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err()
            }
        }
        NonConstantValue::Integer(_) => scalar_literal_satisfies_type(
            *INT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.embedded_location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *FLOAT_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.embedded_location,
            )
            .map_err(|_| error)
        })
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.embedded_location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Boolean(_) => scalar_literal_satisfies_type(
            *BOOLEAN_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.embedded_location,
        ),
        NonConstantValue::String(_) => scalar_literal_satisfies_type(
            *STRING_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.embedded_location,
        )
        .or_else(|error| {
            scalar_literal_satisfies_type(
                *ID_ENTITY_NAME,
                field_argument_definition_type,
                selection_supplied_argument_value.embedded_location,
            )
            .map_err(|_| error)
        }),
        NonConstantValue::Float(_) => scalar_literal_satisfies_type(
            *FLOAT_ENTITY_NAME,
            field_argument_definition_type,
            selection_supplied_argument_value.embedded_location,
        ),
        NonConstantValue::Enum(enum_literal_value) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => {
                    expected_type_found_something_else_named_diagnostic(
                        id_annotation_to_typename_annotation(field_argument_definition_type),
                        (*enum_literal_value).unchecked_conversion(),
                        "enum literal",
                        selection_supplied_argument_value.embedded_location,
                    )
                    .wrap_err()
                }
                GraphQLNonNullTypeAnnotation::Named(_) => enum_satisfies_type(),
            }
        }
        NonConstantValue::Null => {
            if field_argument_definition_type.is_nullable() {
                Ok(())
            } else {
                let expected = id_annotation_to_typename_annotation(field_argument_definition_type);
                Diagnostic::new(
                    format!("Expected non null input of type {expected}, found null"),
                    selection_supplied_argument_value
                        .embedded_location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err()
            }
        }
        NonConstantValue::List(list) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(list_type) => {
                    list_satisfies_type(db, list, list_type, variable_definitions)
                }
                GraphQLNonNullTypeAnnotation::Named(_) => {
                    expected_type_found_something_else_anonymous_diagnostic(
                        id_annotation_to_typename_annotation(field_argument_definition_type),
                        "list literal",
                        selection_supplied_argument_value.embedded_location,
                    )
                    .wrap_err()
                }
            }
        }
        NonConstantValue::Object(object_literal) => {
            match graphql_type_to_non_null_type(field_argument_definition_type.clone()) {
                GraphQLNonNullTypeAnnotation::List(_) => {
                    expected_type_found_something_else_anonymous_diagnostic(
                        id_annotation_to_typename_annotation(field_argument_definition_type),
                        "object literal",
                        selection_supplied_argument_value.embedded_location,
                    )
                    .wrap_err()
                }
                GraphQLNonNullTypeAnnotation::Named(named_type) => object_satisfies_type(
                    db,
                    selection_supplied_argument_value,
                    variable_definitions,
                    object_literal,
                    named_type.0,
                ),
            }
        }
    }
}

fn object_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_supplied_argument_value: &WithEmbeddedLocation<NonConstantValue>,
    variable_definitions: &[VariableDefinition],
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    object_entity_name: EntityName,
) -> DiagnosticResult<()> {
    validate_no_extraneous_fields(
        db,
        object_entity_name,
        object_literal,
        selection_supplied_argument_value.embedded_location,
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
                    field_type_annotation.item.reference(),
                    variable_definitions,
                ) {
                    Ok(_) => None,
                    Err(e) => e.wrap_err().wrap_some(),
                },
                ObjectLiteralFieldType::Missing(field_name) => (*field_name).wrap_ok().wrap_some(),
            })
            .collect::<Result<Vec<_>, _>>()?;

    if missing_fields.is_empty() {
        Ok(())
    } else {
        Diagnostic::new(
            format!(
                "This object has missing fields: {}",
                // TODO smart joining: a, b, and c
                // TODO don't materialize a vec... reduce
                missing_fields
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            selection_supplied_argument_value
                .embedded_location
                .to::<Location>()
                .wrap_some(),
        )
        .wrap_err()
    }
}

enum ObjectLiteralFieldType {
    Provided(
        WithEmbeddedLocation<GraphQLTypeAnnotation>,
        NameValuePair<ValueKeyName, NonConstantValue>,
    ),
    Missing(SelectableName),
}

fn get_non_nullable_missing_and_provided_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    server_object_entity_name: EntityName,
) -> DiagnosticResult<Vec<ObjectLiteralFieldType>> {
    let server_selectables =
        server_selectables_map_for_entity(db, server_object_entity_name).clone_err()?;

    server_selectables
        .iter()
        .filter_map(|(field_name, selectable)| {
            let iso_type_annotation = match selectable.as_ref() {
                SelectionType::Scalar(server_scalar_selectable) => {
                    let field_type_annotation =
                        &server_scalar_selectable.lookup(db).target_entity_name;
                    field_type_annotation.clone()
                }
                SelectionType::Object(server_object_selectable) => {
                    let field_type_annotation =
                        &server_object_selectable.lookup(db).target_entity_name;
                    field_type_annotation.clone()
                }
            };

            let field_type_annotation = iso_type_annotation
                .as_ref()
                .map(graphql_type_annotation_from_type_annotation);

            let object_literal_supplied_field = object_literal
                .iter()
                .find(|field| field.name.item.lookup() == (*field_name).lookup());

            match object_literal_supplied_field {
                Some(selection_supplied_argument_value) => ObjectLiteralFieldType::Provided(
                    field_type_annotation,
                    selection_supplied_argument_value.clone(),
                )
                .wrap_some(),
                None => match field_type_annotation.item {
                    GraphQLTypeAnnotation::NonNull(_) => {
                        ObjectLiteralFieldType::Missing(*field_name).wrap_some()
                    }
                    GraphQLTypeAnnotation::List(_) | GraphQLTypeAnnotation::Named(_) => None,
                },
            }
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

fn validate_no_extraneous_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
    object_literal: &[NameValuePair<ValueKeyName, NonConstantValue>],
    location: EmbeddedLocation,
) -> DiagnosticResult<()> {
    let object_fields =
        server_selectables_map_for_entity(db, parent_server_object_entity_name).clone_err()?;

    let extra_fields: Vec<_> = object_literal
        .iter()
        .filter_map(|field| {
            let is_defined = object_fields
                .get(&field.name.item.unchecked_conversion())
                .is_some();

            if !is_defined {
                return field.clone().wrap_some();
            }
            None
        })
        .collect();

    if !extra_fields.is_empty() {
        return Diagnostic::new(
            format!(
                "This object has extra fields: {0}",
                // TODO smart join
                extra_fields
                    .iter()
                    .map(|field| format!("{}", field.name.item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            location.to::<Location>().wrap_some(),
        )
        .wrap_err();
    }
    Ok(())
}

// TODO this should not exist
fn id_annotation_to_typename_annotation(type_: &GraphQLTypeAnnotation) -> GraphQLTypeAnnotation {
    type_.clone()
}

fn enum_satisfies_type() -> DiagnosticResult<()> {
    todo!("Validate enum literal. Parser doesn't support enum literals yet")
}

fn list_satisfies_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    list: &[WithEmbeddedLocation<NonConstantValue>],
    list_type: GraphQLListTypeAnnotation,
    variable_definitions: &[VariableDefinition],
) -> DiagnosticResult<()> {
    list.iter().try_for_each(|element| {
        value_satisfies_type(
            db,
            element,
            list_type.item.reference(),
            variable_definitions,
        )
    })
}

fn get_variable_type<'a>(
    variable_name: &'a VariableNameWrapper,
    variable_definitions: &'a [VariableDefinition],
    location: EmbeddedLocation,
) -> DiagnosticResult<&'a GraphQLTypeAnnotation> {
    match variable_definitions
        .iter()
        .find(|definition| definition.name.item == *variable_name)
    {
        Some(variable) => (variable.type_.item.reference()).wrap_ok(),
        None => Diagnostic::new(
            format!("This variable is not defined: ${}", *variable_name),
            location.to::<Location>().wrap_some(),
        )
        .wrap_err(),
    }
}

fn expected_type_found_something_else_named_diagnostic(
    expected: GraphQLTypeAnnotation,
    actual: StringKey,
    type_description: &str,
    location: EmbeddedLocation,
) -> Diagnostic {
    Diagnostic::new(
        format!("Expected input of type {expected}, found {actual} {type_description}"),
        location.to::<Location>().wrap_some(),
    )
}

fn expected_type_found_something_else_anonymous_diagnostic(
    expected: GraphQLTypeAnnotation,
    type_description: &str,
    location: EmbeddedLocation,
) -> Diagnostic {
    Diagnostic::new(
        format!("Expected input of type {expected}, found {type_description}"),
        location.to::<Location>().wrap_some(),
    )
}
