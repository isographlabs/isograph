use std::collections::BTreeSet;

use common_lang_types::{
    Diagnostic, DiagnosticResult, DiagnosticVecResult, FieldArgumentName, Location,
    ParentObjectEntityNameAndSelectableName, VariableName, WithLocation, WithSpan,
};

use isograph_lang_types::{
    DefinitionLocation, NonConstantValue, ScalarSelectionDirectiveSet, SelectionFieldArgument,
    SelectionType,
};
use lazy_static::lazy_static;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientScalarOrObjectSelectable, ID_FIELD_NAME, IsographDatabase, NetworkProtocol,
    ValidatedVariableDefinition, client_selectable_map, selectable_named,
    selectable_validated_reader_selection_set, server_object_entity_named,
    validate_argument_types::value_satisfies_type, visit_selection_set::visit_selection_set,
};

type UsedVariables = BTreeSet<VariableName>;

lazy_static! {
    static ref ID: FieldArgumentName = ID_FIELD_NAME.unchecked_conversion();
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
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<()> {
    let mut errors = vec![];

    for client_selectable in client_selectable_map(db)
        .clone_err()?
        .iter()
        .flat_map(|(_, value)| value.as_ref().ok())
    {
        match client_selectable {
            SelectionType::Scalar(s) => {
                validate_use_of_arguments_for_client_type(db, s.lookup(db), &mut errors);
            }
            SelectionType::Object(o) => {
                validate_use_of_arguments_for_client_type(db, o.lookup(db), &mut errors);
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
    db: &IsographDatabase<TNetworkProtocol>,
    client_type: impl ClientScalarOrObjectSelectable,
    errors: &mut Vec<Diagnostic>,
) {
    let mut reachable_variables = BTreeSet::new();

    let validated_selections = match selectable_validated_reader_selection_set(
        db,
        client_type.parent_object_entity_name(),
        client_type.name(),
    ) {
        Ok(validated_selections) => validated_selections,
        Err(new_errors) => {
            return errors.extend(new_errors);
        }
    };

    let parent_entity = server_object_entity_named(db, client_type.parent_object_entity_name())
        .as_ref()
        .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
        .expect("Expected entity to exist. This is indicative of a bug in Isograph.")
        .lookup(db);

    visit_selection_set(
        db,
        &validated_selections.item.selections,
        parent_entity,
        &mut |selection, parent_object_entity| {
            let selectable = match selectable_named(
                db,
                parent_object_entity.name,
                match selection {
                    SelectionType::Scalar(s) => s.name.item,
                    SelectionType::Object(o) => o.name.item,
                },
            )
            .as_ref()
            .expect(
                "Expected parsing to have succeeded. \
                This is indicative of a bug in Isograph.",
            ) {
                Some(s) => s,
                None => {
                    // We could emit an error, but this is validated already as part of
                    // validate_selection_sets.
                    //
                    // We could combine these validations, though, as arguments live in selection sets!
                    return;
                }
            };

            match selection {
                SelectionType::Scalar(scalar_selection) => {
                    let scalar_selectable = selectable.as_scalar().expect(
                        "Expected selectable to be a scalar. \
                        This is indicative of a bug in Isograph.",
                    );
                    let field_argument_definitions = match scalar_selectable {
                        DefinitionLocation::Server(server_scalar_selectable) => {
                            let server_scalar_selectable = server_scalar_selectable.lookup(db);

                            server_scalar_selectable
                                .arguments
                                .iter()
                                .map(|x| x.item.clone())
                                .collect::<Vec<_>>()
                        }
                        DefinitionLocation::Client(client_scalar_selectable) => {
                            client_scalar_selectable
                                .lookup(db)
                                .variable_definitions
                                .iter()
                                .map(|x| x.item.clone())
                                .collect()
                        }
                    };

                    // Only loadably selected fields are allowed to have missing arguments
                    let can_have_missing_args = matches!(
                        scalar_selection.scalar_selection_directive_set,
                        ScalarSelectionDirectiveSet::Loadable(_)
                    );

                    validate_use_of_arguments_impl(
                        db,
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
                    let object_selectable = selectable.as_object().expect(
                        "Expected selectable to be an object. \
                        This is indicative of a bug in Isograph.",
                    );
                    let field_argument_definitions = match object_selectable {
                        DefinitionLocation::Server(server_object_selectable) => {
                            server_object_selectable
                                .lookup(db)
                                .arguments
                                .iter()
                                .map(|x| x.item.clone())
                                .collect::<Vec<_>>()
                        }
                        DefinitionLocation::Client(client_object_selectable) => {
                            client_object_selectable
                                .lookup(db)
                                .variable_definitions
                                .iter()
                                .map(|x| x.item.clone())
                                .collect()
                        }
                    };

                    validate_use_of_arguments_impl(
                        db,
                        errors,
                        &mut reachable_variables,
                        field_argument_definitions,
                        client_type.variable_definitions(),
                        true,
                        &object_selection.arguments,
                        object_selection.name.location,
                    );
                }
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
            Location::Generated,
        ),
    );
}

#[expect(clippy::too_many_arguments)]
fn validate_use_of_arguments_impl<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    errors: &mut Vec<Diagnostic>,
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
                    ),
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
) -> DiagnosticResult<()> {
    let unused_variables = variable_definitions
        .iter()
        .filter_map(|variable| {
            let is_used = used_variables.contains(&variable.item.name.item);

            if !is_used {
                return variable.clone().wrap_some();
            }
            None
        })
        .collect::<Vec<_>>();

    if !unused_variables.is_empty() {
        let type_name = top_level_type_and_field_name.parent_object_entity_name;
        let field_name = top_level_type_and_field_name.selectable_name;
        return Diagnostic::new(
            format!(
                "The field `{type_name}.{field_name}` has unused variables: {0}",
                unused_variables
                    .iter()
                    .map(|variable| format!("${}", variable.item.name.item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            location.wrap_some(),
        )
        .wrap_err();
    }
    Ok(())
}

fn assert_no_missing_arguments(
    missing_arguments: Vec<ValidatedVariableDefinition>,
    location: Location,
) -> DiagnosticResult<()> {
    if !missing_arguments.is_empty() {
        return Diagnostic::new(
            format!(
                "This field has missing arguments: {0}",
                missing_arguments
                    .iter()
                    .map(|arg| format!("${}", arg.name.item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            location.wrap_some(),
        )
        .wrap_err();
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
                ArgumentType::Provided(field_argument_definition, selection_supplied_argument)
                    .wrap_some()
            } else if field_argument_definition.default_value.is_some()
                || field_argument_definition.type_.is_nullable()
            {
                None
            } else {
                ArgumentType::Missing(field_argument_definition).wrap_some()
            }
        })
}

fn validate_no_extraneous_arguments(
    field_argument_definitions: &[ValidatedVariableDefinition],
    selection_supplied_arguments: &[WithLocation<SelectionFieldArgument>],
    location: Location,
) -> DiagnosticResult<()> {
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
                return arg.clone().wrap_some();
            }
            None
        })
        .collect();

    if !extra_arguments.is_empty() {
        return Diagnostic::new(
            format!(
                "This field has extra arguments: {0}",
                extra_arguments
                    .iter()
                    .map(|arg| format!("{}", arg.item.name.item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            location.wrap_some(),
        )
        .wrap_err();
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
