use std::collections::BTreeSet;

use common_lang_types::{
    Diagnostic, DiagnosticResult, DiagnosticVecResult, EmbeddedLocation,
    EntityNameAndSelectableName, FieldArgumentName, Location, WithEmbeddedLocation,
};

use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, NonConstantValue, ScalarSelectionDirectiveSet,
    SelectionFieldArgument, SelectionType, VariableDeclaration, VariableNameWrapper,
};
use lazy_static::lazy_static;
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, ID_FIELD_NAME, IsographDatabase, MemoRefClientSelectable,
    client_selectable_map, flattened_entity_named, selectable_named,
    selectable_reader_selection_set, validate_argument_types::value_satisfies_type,
    visit_selection_set::visit_selection_set,
};

type UsedVariables = BTreeSet<VariableNameWrapper>;

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
pub fn validate_use_of_arguments<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticVecResult<()> {
    let mut errors = vec![];

    for client_selectable in client_selectable_map(db)
        .clone_err()?
        .iter()
        .flat_map(|(_, value)| value.as_ref().ok())
    {
        validate_use_of_arguments_for_client_type(db, client_selectable.dereference(), &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_use_of_arguments_for_client_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_type: MemoRefClientSelectable<TCompilationProfile>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut reachable_variables = BTreeSet::new();

    let (parent_entity_name, client_selectable_name, variable_definitions) = match client_type {
        SelectionType::Scalar(s) => {
            let s = s.lookup(db);
            (s.parent_entity_name, s.name, &s.arguments)
        }
        SelectionType::Object(o) => {
            let o = o.lookup(db);
            (o.parent_entity_name, o.name, &o.arguments)
        }
    };

    let selection_set =
        match selectable_reader_selection_set(db, parent_entity_name, client_selectable_name) {
            Ok(selection_set) => selection_set.lookup(db),
            Err(error) => {
                return errors.push(error);
            }
        };

    let parent_entity = match flattened_entity_named(db, parent_entity_name) {
        Some(entity) => entity.lookup(db),
        None => {
            // We could emit an error, but this is validated already.
            // Anyway, this is clearly a smell.
            return;
        }
    };

    visit_selection_set(
        db,
        selection_set.item.selections.reference(),
        parent_entity,
        &mut |selection, parent_object_entity| {
            let selectable = match selectable_named(
                db,
                parent_object_entity.name.item,
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
                    let scalar_selectable = match selectable {
                        DefinitionLocation::Server(s) => {
                            let selectable = s.lookup(db);
                            let target_entity_name = match selectable.target_entity.item.as_ref() {
                                Ok(annotation) => annotation.inner().0,
                                Err(_) => return,
                            };
                            let entity = flattened_entity_named(db, target_entity_name);
                            let entity = match entity {
                                Some(entity) => entity.lookup(db),
                                None => {
                                    return;
                                }
                            };

                            if entity.selection_info.as_object().is_some() {
                                return;
                            }

                            selectable.server_defined()
                        }
                        DefinitionLocation::Client(c) => match c {
                            SelectionType::Scalar(s) => s.client_defined(),
                            SelectionType::Object(_) => {
                                return;
                            }
                        },
                    };

                    let field_argument_definitions = match scalar_selectable {
                        DefinitionLocation::Server(server_scalar_selectable) => {
                            server_scalar_selectable.arguments.to_vec()
                        }
                        DefinitionLocation::Client(client_scalar_selectable) => {
                            client_scalar_selectable.lookup(db).arguments.to_vec()
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
                        variable_definitions,
                        can_have_missing_args,
                        &scalar_selection.arguments,
                        scalar_selection.name.location,
                    );
                }
                SelectionType::Object(object_selection) => {
                    let object_selectable = match selectable {
                        DefinitionLocation::Server(s) => {
                            let selectable = s.lookup(db);
                            let target_entity_name = match selectable.target_entity.item.as_ref() {
                                Ok(annotation) => annotation.inner().0,
                                Err(_) => return,
                            };
                            let entity = flattened_entity_named(db, target_entity_name);
                            let entity = match entity {
                                Some(entity) => entity.lookup(db),
                                None => {
                                    return;
                                }
                            };

                            if entity.selection_info.as_scalar().is_some() {
                                return;
                            }

                            selectable.server_defined()
                        }
                        DefinitionLocation::Client(c) => match c {
                            SelectionType::Scalar(_) => {
                                return;
                            }
                            SelectionType::Object(o) => o.client_defined(),
                        },
                    };

                    let field_argument_definitions = match object_selectable {
                        DefinitionLocation::Server(server_object_selectable) => {
                            server_object_selectable.arguments.to_vec()
                        }
                        DefinitionLocation::Client(client_object_selectable) => {
                            client_object_selectable.lookup(db).arguments.to_vec()
                        }
                    };

                    validate_use_of_arguments_impl(
                        db,
                        errors,
                        &mut reachable_variables,
                        field_argument_definitions,
                        variable_definitions,
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
            variable_definitions,
            reachable_variables,
            EntityNameAndSelectableName {
                parent_entity_name,
                selectable_name: client_selectable_name,
            },
            // TODO client_type name needs a location
            Location::Generated,
        ),
    );
}

#[expect(clippy::too_many_arguments)]
fn validate_use_of_arguments_impl<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    errors: &mut Vec<Diagnostic>,
    reachable_variables: &mut UsedVariables,
    field_argument_definitions: Vec<VariableDeclaration>,
    client_type_variable_definitions: &[VariableDeclaration],
    can_have_missing_args: bool,
    selection_supplied_arguments: &[WithEmbeddedLocation<SelectionFieldArgument>],
    name_location: EmbeddedLocation,
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
                        selection_supplied_argument.item.value.reference(),
                        field_argument_definition.type_.item.reference(),
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
    variable_definitions: &[VariableDeclaration],
    used_variables: UsedVariables,
    top_level_type_and_field_name: EntityNameAndSelectableName,
    location: Location,
) -> DiagnosticResult<()> {
    let unused_variables = variable_definitions
        .iter()
        .filter_map(|variable| {
            let is_used = used_variables.contains(&variable.name.item);

            if !is_used {
                return variable.clone().wrap_some();
            }
            None
        })
        .collect::<Vec<_>>();

    if !unused_variables.is_empty() {
        let type_name = top_level_type_and_field_name.parent_entity_name;
        let field_name = top_level_type_and_field_name.selectable_name;
        return Diagnostic::new(
            format!(
                "The field `{type_name}.{field_name}` has unused variables: {0}",
                unused_variables
                    .iter()
                    .map(|variable| format!("${}", variable.name.item))
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
    missing_arguments: Vec<VariableDeclaration>,
    location: EmbeddedLocation,
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
            location.to::<Location>().wrap_some(),
        )
        .wrap_err();
    }
    Ok(())
}

enum ArgumentType<'a> {
    Missing(&'a VariableDeclaration),
    Provided(
        &'a VariableDeclaration,
        &'a WithEmbeddedLocation<SelectionFieldArgument>,
    ),
}

fn get_missing_and_provided_arguments<'a>(
    field_argument_definitions: &'a [VariableDeclaration],
    selection_supplied_arguments: &'a [WithEmbeddedLocation<SelectionFieldArgument>],
) -> impl Iterator<Item = ArgumentType<'a>> {
    field_argument_definitions
        .iter()
        .filter_map(move |field_argument_definition| {
            let selection_supplied_argument = selection_supplied_arguments
                .iter()
                .find(|arg| field_argument_definition.name.item.0 == arg.item.name.item);

            if let Some(selection_supplied_argument) = selection_supplied_argument {
                ArgumentType::Provided(field_argument_definition, selection_supplied_argument)
                    .wrap_some()
            } else if field_argument_definition.default_value.is_some()
                || field_argument_definition.type_.item.is_nullable()
            {
                None
            } else {
                ArgumentType::Missing(field_argument_definition).wrap_some()
            }
        })
}

fn validate_no_extraneous_arguments(
    field_argument_definitions: &[VariableDeclaration],
    selection_supplied_arguments: &[WithEmbeddedLocation<SelectionFieldArgument>],
    location: EmbeddedLocation,
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
                .any(|definition| definition.name.item.0 == arg.item.name.item);

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
            location.to::<Location>().wrap_some(),
        )
        .wrap_err();
    }
    Ok(())
}

pub fn extend_reachable_variables_with_arg(
    non_constant_value: &WithEmbeddedLocation<NonConstantValue>,
    reachable_variables: &mut UsedVariables,
) {
    // TODO implement this more efficiently with accumulator-passing-style
    match non_constant_value.item.reference() {
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
    reachable_variables: &mut UsedVariables,
    arguments: &[WithEmbeddedLocation<SelectionFieldArgument>],
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
