use std::collections::BTreeSet;
use std::ops::Deref;

use common_lang_types::{
    FieldArgumentName, Location, ParentObjectEntityNameAndSelectableName, SelectableName,
    ServerObjectEntityName, VariableName, WithLocation, WithSpan,
};

use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, NonConstantValue, ScalarSelectionDirectiveSet, SelectionFieldArgument,
    SelectionType,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    ClientScalarOrObjectSelectable, IsographDatabase, NetworkProtocol, Schema,
    ValidatedVariableDefinition, server_object_selectable_named, server_scalar_selectable_named,
    validate_argument_types::{ValidateArgumentTypesError, value_satisfies_type},
    visit_selection_set::visit_selection_set,
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
pub fn validate_use_of_arguments<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    validated_schema: &Schema<TNetworkProtocol>,
) -> Result<(), Vec<WithLocation<ValidateUseOfArgumentsError>>> {
    let mut errors = vec![];
    for client_scalar_selectable in validated_schema.client_scalar_selectables.values() {
        validate_use_of_arguments_for_client_type(
            db,
            validated_schema,
            client_scalar_selectable,
            &mut errors,
        );
    }
    for client_object_selectable in validated_schema.client_object_selectables.values() {
        validate_use_of_arguments_for_client_type(
            db,
            validated_schema,
            client_object_selectable,
            &mut errors,
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_use_of_arguments_for_client_type<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    client_type: impl ClientScalarOrObjectSelectable,
    errors: &mut Vec<WithLocation<ValidateUseOfArgumentsError>>,
) {
    let mut reachable_variables = BTreeSet::new();

    visit_selection_set(
        client_type.reader_selection_set(),
        &mut |selection| match selection {
            SelectionType::Scalar(scalar_selection) => {
                let field_argument_definitions = match scalar_selection.associated_data {
                    DefinitionLocation::Server((
                        parent_object_entity_name,
                        server_scalar_selectable_name,
                    )) => {
                        let memo_ref = server_scalar_selectable_named(
                            db,
                            parent_object_entity_name,
                            server_scalar_selectable_name.into(),
                        );
                        let server_scalar_selectable = memo_ref
                            .deref()
                            .as_ref()
                            .expect(
                                "Expected validation to have succeeded. \
                                This is indicative of a bug in Isograph.",
                            )
                            .as_ref()
                            .expect(
                                "Expected selectable to exist. \
                                This is indicative of a bug in Isograph.",
                            );

                        server_scalar_selectable
                            .arguments
                            .iter()
                            .map(|x| x.item.clone())
                            .collect::<Vec<_>>()
                    }
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        client_selectable_name,
                    )) => schema
                        .client_scalar_selectable(parent_object_entity_name, client_selectable_name)
                        .expect(
                            "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .variable_definitions
                        .iter()
                        .map(|x| x.item.clone())
                        .collect(),
                };

                // Only loadably selected fields are allowed to have missing arguments
                let can_have_missing_args = matches!(
                    scalar_selection.scalar_selection_directive_set,
                    ScalarSelectionDirectiveSet::Loadable(_)
                );

                validate_use_of_arguments_impl(
                    db,
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
                let field_argument_definitions = match object_selection.associated_data {
                    DefinitionLocation::Server((
                        parent_object_entity_name,
                        server_object_selectable_name,
                    )) => {
                        let memo_ref = server_object_selectable_named(
                            db,
                            parent_object_entity_name,
                            server_object_selectable_name.into(),
                        );
                        memo_ref
                            .deref()
                            .as_ref()
                            .expect(
                                "Expected validation to have succeeded. \
                                This is indicative of a bug in Isograph.",
                            )
                            .as_ref()
                            .expect(
                                "Expected selectable to exist. \
                                This is indicative of a bug in Isograph.",
                            )
                            .arguments
                            .iter()
                            .map(|x| x.item.clone())
                            .collect::<Vec<_>>()
                    }
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        client_object_selectable_name,
                    )) => schema
                        .client_object_selectable(
                            parent_object_entity_name,
                            client_object_selectable_name,
                        )
                        .expect(
                            "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .variable_definitions
                        .iter()
                        .map(|x| x.item.clone())
                        .collect(),
                };

                validate_use_of_arguments_impl(
                    db,
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

#[expect(clippy::too_many_arguments)]
fn validate_use_of_arguments_impl<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    errors: &mut Vec<WithLocation<ValidateUseOfArgumentsError>>,
    reachable_variables: &mut BTreeSet<VariableName>,
    field_argument_definitions: Vec<ValidatedVariableDefinition>,
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
                        db,
                        &selection_supplied_argument.item.value,
                        &field_argument_definition.type_,
                        client_type_variable_definitions,
                        &schema.server_entity_data,
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
    top_level_type_and_field_name: ParentObjectEntityNameAndSelectableName,
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
                type_name: top_level_type_and_field_name.parent_object_entity_name,
                field_name: top_level_type_and_field_name.selectable_name,
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
    field_argument_definitions: &'a [ValidatedVariableDefinition],
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
    field_argument_definitions: &[ValidatedVariableDefinition],
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
        type_name: ServerObjectEntityName,
        field_name: SelectableName,
    },

    #[error("{message}")]
    ValidateArgumentType {
        #[from]
        message: ValidateArgumentTypesError,
    },
}
