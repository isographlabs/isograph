use std::collections::BTreeSet;

use common_lang_types::{
    FieldArgumentName, IsographObjectTypeName, Location, ObjectTypeAndFieldName, SelectableName,
    VariableName, WithLocation, WithSpan,
};

use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, NonConstantValue, ScalarSelectionDirectiveSet, SelectionFieldArgument,
    SelectionType,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    validate_argument_types::{value_satisfies_type, ValidateArgumentTypesError},
    visit_selection_set::visit_selection_set,
    ClientFieldOrPointer, NetworkProtocol, Schema, ValidatedVariableDefinition,
};

type UsedVariables = BTreeSet<VariableName>;

lazy_static! {
    static ref ID: FieldArgumentName = "id".intern().into();
}

/// For all client types, validate that
/// - there are no unused arguments
/// - all arguments are used
/// - there are no missing arguments, and
/// - all args type-check
///
/// In addition, validate that no server field is selected loadably.
/// This should not be validated here, and can be fixed with better modeling (i.e.
/// have different associated data for fields that points to server objects and
/// fields that point to client objects.)
pub fn validate_use_of_arguments<TNetworkProtocol: NetworkProtocol>(
    validated_schema: &Schema<TNetworkProtocol>,
) -> Result<(), Vec<WithLocation<ValidateUseOfArgumentsError>>> {
    let mut errors = vec![];
    for client_type in &validated_schema.client_types {
        match client_type {
            SelectionType::Scalar(client_field) => {
                validate_use_of_arguments_for_client_type(
                    validated_schema,
                    client_field,
                    &mut errors,
                );
            }
            SelectionType::Object(client_pointer) => {
                validate_use_of_arguments_for_client_type(
                    validated_schema,
                    client_pointer,
                    &mut errors,
                );
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_use_of_arguments_for_client_type<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    client_type: impl ClientFieldOrPointer,
    errors: &mut Vec<WithLocation<ValidateUseOfArgumentsError>>,
) {
    let mut reachable_variables = BTreeSet::new();

    visit_selection_set(
        client_type.reader_selection_set(),
        &mut |selection| match selection {
            SelectionType::Scalar(scalar_selection) => {
                let field_argument_definitions = match scalar_selection.associated_data.location {
                    DefinitionLocation::Server(s) => schema
                        .server_scalar_selectable(s)
                        .arguments
                        .iter()
                        .map(|x| &x.item)
                        .collect::<Vec<_>>(),
                    DefinitionLocation::Client(c) => schema
                        .client_field(c)
                        .variable_definitions
                        .iter()
                        .map(|x| &x.item)
                        .collect(),
                };

                // Only loadably selected fields are allowed to have missing arguments
                let can_have_missing_args = matches!(
                    scalar_selection.associated_data.selection_variant,
                    ScalarSelectionDirectiveSet::Loadable(_)
                );

                validate_use_of_arguments_impl(
                    schema,
                    errors,
                    &mut reachable_variables,
                    field_argument_definitions,
                    client_type.variable_definitions(),
                    can_have_missing_args,
                    &scalar_selection.arguments,
                    scalar_selection.name.location,
                );
            }
            SelectionType::Object(object_selection) => {
                let field_argument_definitions = match object_selection.associated_data.field_id {
                    DefinitionLocation::Server(s) => schema
                        .server_scalar_selectable(s)
                        .arguments
                        .iter()
                        .map(|x| &x.item)
                        .collect::<Vec<_>>(),
                    DefinitionLocation::Client(c) => schema
                        .client_pointer(c)
                        .variable_definitions
                        .iter()
                        .map(|x| &x.item)
                        .collect(),
                };

                validate_use_of_arguments_impl(
                    schema,
                    errors,
                    &mut reachable_variables,
                    field_argument_definitions,
                    client_type.variable_definitions(),
                    true,
                    &object_selection.arguments,
                    object_selection.name.location,
                );
            }
        },
    );

    maybe_push_errors(
        errors,
        validate_all_variables_are_used(
            client_type.variable_definitions(),
            reachable_variables,
            client_type.type_and_field(),
            // TODO client_type name needs a location
            Location::generated(),
        ),
    );
}

#[allow(clippy::too_many_arguments)]
fn validate_use_of_arguments_impl<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    errors: &mut Vec<WithLocation<ValidateUseOfArgumentsError>>,
    reachable_variables: &mut BTreeSet<VariableName>,
    field_argument_definitions: Vec<&ValidatedVariableDefinition>,
    client_type_variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    can_have_missing_args: bool,
    selection_supplied_arguments: &[WithLocation<SelectionFieldArgument>],
    name_location: Location,
) {
    let mut missing_args = vec![];
    for argument_type in get_missing_and_provided_arguments(
        &field_argument_definitions,
        selection_supplied_arguments,
    ) {
        match argument_type {
            ArgumentType::Missing(variable_definition) => {
                missing_args.push(variable_definition.clone())
            }
            ArgumentType::Provided(field_argument_definition, selection_supplied_argument) => {
                maybe_push_errors(
                    errors,
                    value_satisfies_type(
                        &selection_supplied_argument.item.value,
                        &field_argument_definition.type_,
                        client_type_variable_definitions,
                        &schema.server_field_data,
                        &schema.server_scalar_selectables,
                    )
                    .map_err(|with_location| with_location.map(|e| e.into())),
                );
            }
        }
    }

    maybe_push_errors(
        errors,
        validate_no_extraneous_arguments(
            &field_argument_definitions,
            selection_supplied_arguments,
            name_location,
        ),
    );

    extend_reachable_variables_with_args(reachable_variables, selection_supplied_arguments);

    if !can_have_missing_args {
        maybe_push_errors(
            errors,
            assert_no_missing_arguments(missing_args, name_location),
        );
    }
}

fn validate_all_variables_are_used(
    variable_definitions: &[WithSpan<ValidatedVariableDefinition>],
    used_variables: UsedVariables,
    top_level_type_and_field_name: ObjectTypeAndFieldName,
    location: Location,
) -> ValidateUseOfArgumentsResult<()> {
    let unused_variables = variable_definitions
        .iter()
        .filter_map(|variable| {
            let is_used = used_variables.contains(&variable.item.name.item);

            if !is_used {
                return Some(variable.clone());
            }
            None
        })
        .collect::<Vec<_>>();

    if !unused_variables.is_empty() {
        return Err(WithLocation::new(
            ValidateUseOfArgumentsError::UnusedVariables {
                unused_variables,
                type_name: top_level_type_and_field_name.type_name,
                field_name: top_level_type_and_field_name.field_name,
            },
            location,
        ));
    }
    Ok(())
}

fn assert_no_missing_arguments(
    missing_arguments: Vec<ValidatedVariableDefinition>,
    location: Location,
) -> ValidateUseOfArgumentsResult<()> {
    if !missing_arguments.is_empty() {
        return Err(WithLocation::new(
            ValidateUseOfArgumentsError::MissingArguments { missing_arguments },
            location,
        ));
    }
    Ok(())
}

enum ArgumentType<'a> {
    Missing(&'a ValidatedVariableDefinition),
    Provided(
        &'a ValidatedVariableDefinition,
        &'a WithLocation<SelectionFieldArgument>,
    ),
}

fn get_missing_and_provided_arguments<'a>(
    field_argument_definitions: &'a [&'a ValidatedVariableDefinition],
    selection_supplied_arguments: &'a [WithLocation<SelectionFieldArgument>],
) -> impl Iterator<Item = ArgumentType<'a>> {
    field_argument_definitions
        .iter()
        .filter_map(move |field_argument_definition| {
            let selection_supplied_argument = selection_supplied_arguments
                .iter()
                .find(|arg| field_argument_definition.name.item == arg.item.name.item);

            if let Some(selection_supplied_argument) = selection_supplied_argument {
                Some(ArgumentType::Provided(
                    field_argument_definition,
                    selection_supplied_argument,
                ))
            } else if field_argument_definition.default_value.is_some()
                || field_argument_definition.type_.is_nullable()
            {
                None
            } else {
                Some(ArgumentType::Missing(field_argument_definition))
            }
        })
}

fn validate_no_extraneous_arguments(
    field_argument_definitions: &[&ValidatedVariableDefinition],
    selection_supplied_arguments: &[WithLocation<SelectionFieldArgument>],
    location: Location,
) -> ValidateUseOfArgumentsResult<()> {
    let extra_arguments: Vec<_> = selection_supplied_arguments
        .iter()
        .filter_map(|arg| {
            // TODO remove this
            // With @exposeField on Query, id field is needed because the generated
            // query is like node(id: $id) { ... everything else }, but that
            // id field is added in somewhere else

            if arg.item.name.item == *ID {
                return None;
            }

            let is_defined = field_argument_definitions
                .iter()
                .any(|definition| definition.name.item == arg.item.name.item);

            if !is_defined {
                return Some(arg.clone());
            }
            None
        })
        .collect();

    if !extra_arguments.is_empty() {
        return Err(WithLocation::new(
            ValidateUseOfArgumentsError::ExtraneousArgument { extra_arguments },
            location,
        ));
    }
    Ok(())
}

pub fn extend_reachable_variables_with_arg(
    non_constant_value: &WithLocation<NonConstantValue>,
    reachable_variables: &mut UsedVariables,
) {
    // TODO implement this more efficiently with accumulator-passing-style
    match &non_constant_value.item {
        NonConstantValue::Variable(name) => {
            reachable_variables.insert(*name);
        }
        NonConstantValue::Object(object) => {
            for arg in object.iter() {
                extend_reachable_variables_with_arg(&arg.value, reachable_variables);
            }
        }
        NonConstantValue::List(list) => {
            for arg in list.iter() {
                extend_reachable_variables_with_arg(arg, reachable_variables);
            }
        }
        _ => {}
    }
}

fn extend_reachable_variables_with_args(
    reachable_variables: &mut BTreeSet<VariableName>,
    arguments: &[WithLocation<SelectionFieldArgument>],
) {
    for arg in arguments.iter() {
        extend_reachable_variables_with_arg(&arg.item.value, reachable_variables);
    }
}

fn maybe_push_errors<E>(errors: &mut Vec<E>, result: Result<(), E>) {
    if let Err(e) = result {
        errors.push(e)
    }
}

type MissingArguments = Vec<ValidatedVariableDefinition>;

type ValidateUseOfArgumentsResult<T> = Result<T, WithLocation<ValidateUseOfArgumentsError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ValidateUseOfArgumentsError {
    #[error(
        "This field has missing arguments: {0}",
        missing_arguments.iter().map(|arg| format!("${}", arg.name.item)).collect::<Vec<_>>().join(", ")
    )]
    MissingArguments { missing_arguments: MissingArguments },

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
        unused_variables: Vec<WithSpan<ValidatedVariableDefinition>>,
        type_name: IsographObjectTypeName,
        field_name: SelectableName,
    },

    #[error("{message}")]
    ValidateArgumentType {
        #[from]
        message: ValidateArgumentTypesError,
    },
}
