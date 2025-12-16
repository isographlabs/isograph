use std::collections::BTreeSet;

use common_lang_types::{Diagnostic, DiagnosticVecResult, Location};
use isograph_lang_types::{DefinitionLocation, SelectionSet, SelectionType};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientFieldVariant, ContainsIsoStats, IsographDatabase, NetworkProtocol, ServerObjectEntity,
    client_selectable_declaration_map_from_iso_literals, client_selectable_map,
    entity_not_defined_diagnostic, memoized_unvalidated_reader_selection_set_map,
    parse_iso_literals, process_iso_literals, selectable_is_not_defined_diagnostic,
    selectable_is_wrong_type_diagnostic, selectable_named, server_entities_map_without_locations,
    server_id_selectable, server_object_entities, server_object_entity_named,
    server_selectables_map, validate_use_of_arguments, validated_entrypoints,
};

/// In the world of pico, we minimally validate. For example, if the
/// schema contains a field `foo: Bar`, and `Bar` is undefined and
/// unreferenced, then we will never actually ensure that `Bar` is
/// actually defined!
///
/// So, we need to define a function where we do all of the validation.
///
/// This is opt-in, but it makes sense to call this before we generate
/// artifacts. However, whether we do these strictly-unnecessary
/// validations should be controllable by the user.
#[memo]
pub fn validate_entire_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<ContainsIsoStats> {
    let mut errors = BTreeSet::new();

    maybe_extend(&mut errors, validate_use_of_arguments(db));

    errors.extend(validate_selection_sets(db));

    maybe_extend(
        &mut errors,
        validate_all_server_selectables_point_to_defined_types(db),
    );

    errors.extend(validate_all_id_fields(db));

    errors.extend(
        validated_entrypoints(db)
            .values()
            .flat_map(|result| result.as_ref().err()?.clone().wrap_some()),
    );

    errors.extend(validate_scalar_selectable_directive_sets(db));

    if let Ok((outcome, _)) = TNetworkProtocol::parse_type_system_documents(db) {
        errors.extend(outcome.non_fatal_diagnostics.clone());
    }

    errors.extend(
        client_selectable_declaration_map_from_iso_literals(db)
            .non_fatal_diagnostics
            .clone(),
    );

    let contains_iso_stats = match validate_all_iso_literals(db) {
        Ok(stats) => stats,
        Err(e) => {
            errors.extend(e);
            return errors.into_iter().collect::<Vec<_>>().wrap_err();
        }
    };

    if errors.is_empty() {
        Ok(contains_iso_stats)
    } else {
        errors.into_iter().collect::<Vec<_>>().wrap_err()
    }
}

fn validate_all_iso_literals<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<ContainsIsoStats> {
    let contains_iso = parse_iso_literals(db).to_owned()?;
    let contains_iso_stats = contains_iso.stats();

    process_iso_literals(db, contains_iso)?;

    Ok(contains_iso_stats)
}

fn maybe_extend<T, E>(errors_acc: &mut impl Extend<E>, result: Result<T, Vec<E>>) {
    if let Err(e) = result {
        errors_acc.extend(e);
    }
}

fn validate_all_server_selectables_point_to_defined_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<()> {
    let server_selectables = server_selectables_map(db).clone_err()?;
    let entities = server_entities_map_without_locations(db)
        .to_owned()?
        .lookup(db);

    let mut errors = vec![];

    // TODO use iterator methods
    for ((parent_object_entity_name, selectable_name), selectable) in server_selectables.iter() {
        let (target, name_location, arguments) = match selectable {
            SelectionType::Scalar(s) => {
                let scalar = s.lookup(db);
                (
                    scalar.target_scalar_entity.inner().dereference(),
                    scalar.name.location,
                    &scalar.arguments,
                )
            }
            SelectionType::Object(o) => {
                let object = o.lookup(db);
                (
                    object.target_object_entity.inner().dereference(),
                    object.name.location,
                    &object.arguments,
                )
            }
        };

        if !entities.contains_key(&target) {
            errors.push(Diagnostic::new(
                format!(
                    "`{parent_object_entity_name}.{selectable_name}` has inner \
                    type `{target}, but that type has not been defined"
                ),
                name_location.wrap_some(),
            ))
        }

        for argument in arguments {
            let arg_target = match argument.item.type_.inner().dereference() {
                SelectionType::Scalar(s) => s,
                SelectionType::Object(o) => o,
            };

            if !entities.contains_key(&arg_target) {
                let arg_name = argument.item.name.item;
                errors.push(Diagnostic::new(
                    format!(
                        "In `{parent_object_entity_name}.{selectable_name}`, the argument `{arg_name}` has inner \
                        type `{arg_target}, but that type has not been defined"
                    ),
                    argument.location.wrap_some(),
                ))
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_all_id_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<Diagnostic> {
    let entities = match server_object_entities(db).as_ref() {
        Ok(entities) => entities,
        Err(e) => return vec![e.clone()],
    };

    entities
        .iter()
        .flat_map(|entity| {
            server_id_selectable(db, entity.lookup(db).name)
                .clone_err()
                .err()
        })
        .collect()
}

fn validate_scalar_selectable_directive_sets<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<Diagnostic> {
    let selectables = match client_selectable_map(db) {
        Ok(s) => s,
        Err(e) => return vec![e.clone()],
    };

    selectables
        .values()
        .flat_map(|x| {
            let selection = match x {
                Ok(x) => x,
                Err(e) => return Some(e.clone()),
            };

            match selection {
                SelectionType::Scalar(s) => {
                    if let ClientFieldVariant::UserWritten(u) = &s.lookup(db).variant
                        && let Err(e) = &u.client_scalar_selectable_directive_set
                    {
                        return Some(e.clone());
                    }
                }
                SelectionType::Object(_) => {
                    // Intentionally do nothing
                }
            }

            None
        })
        .collect()
}

fn validate_selection_sets<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<Diagnostic> {
    let selection_sets = memoized_unvalidated_reader_selection_set_map(db);

    let mut errors = vec![];
    for (key, selection_set) in selection_sets {
        let selection_set = match selection_set.clone_err() {
            Ok(s) => s,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let selection_set = match selection_set {
            SelectionType::Scalar(s) => &s.item,
            SelectionType::Object(o) => &o.item,
        };

        let parent_entity = match server_object_entity_named(db, key.0).clone_err() {
            Ok(entity) => {
                match entity {
                    Some(s) => s,
                    None => {
                        // TODO better location... and this is probably already validated elsewhere.
                        // Maybe we can just continue
                        errors.push(entity_not_defined_diagnostic(key.0, Location::Generated));
                        continue;
                    }
                }
            }
            Err(e) => {
                errors.push(e);
                continue;
            }
        }
        .lookup(db);

        validate_selection_set(db, &mut errors, selection_set, parent_entity)
    }

    errors
}

/// for each selection, validate that it corresponds to a selectable of the correct SelectionType
fn validate_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    errors: &mut Vec<Diagnostic>,
    selection_set: &SelectionSet,
    parent_entity: &ServerObjectEntity<TNetworkProtocol>,
) {
    for selection in selection_set.selections.iter() {
        match selection.item.reference() {
            SelectionType::Scalar(scalar_selection) => {
                let selectable_name = scalar_selection.name.item;
                let selectable =
                    match selectable_named(db, parent_entity.name, selectable_name).clone_err() {
                        Ok(s) => match s {
                            Some(s) => s,
                            None => {
                                errors.push(selectable_is_not_defined_diagnostic(
                                    parent_entity.name,
                                    selectable_name,
                                    scalar_selection.name.location,
                                ));
                                continue;
                            }
                        },
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    };

                match selectable.as_scalar() {
                    Some(_) => {}
                    None => errors.push(selectable_is_wrong_type_diagnostic(
                        parent_entity.name,
                        selectable_name,
                        "a scalar",
                        "an object",
                        scalar_selection.name.location,
                    )),
                };
            }
            SelectionType::Object(object_selection) => {
                let selectable_name = object_selection.name.item;
                let selectable =
                    match selectable_named(db, parent_entity.name, selectable_name).clone_err() {
                        Ok(s) => match s {
                            Some(s) => s,
                            None => {
                                errors.push(selectable_is_not_defined_diagnostic(
                                    parent_entity.name,
                                    selectable_name,
                                    object_selection.name.location,
                                ));
                                continue;
                            }
                        },
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    };

                let target_entity_name = match selectable.as_object() {
                    Some(o) => match o {
                        DefinitionLocation::Server(s) => {
                            s.lookup(db).target_object_entity.inner().dereference()
                        }
                        DefinitionLocation::Client(c) => {
                            c.lookup(db).target_object_entity_name.inner().dereference()
                        }
                    },
                    None => {
                        errors.push(selectable_is_wrong_type_diagnostic(
                            parent_entity.name,
                            selectable_name,
                            "an object",
                            "a scalar",
                            object_selection.name.location,
                        ));
                        continue;
                    }
                };

                let new_parent_entity =
                    match server_object_entity_named(db, target_entity_name).clone_err() {
                        Ok(entity) => match entity {
                            Some(s) => s,
                            None => {
                                // This was probably validated elsewhere??
                                errors.push(entity_not_defined_diagnostic(
                                    target_entity_name,
                                    object_selection.name.location,
                                ));
                                continue;
                            }
                        },
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    }
                    .lookup(db);

                validate_selection_set(
                    db,
                    errors,
                    &object_selection.selection_set.item,
                    new_parent_entity,
                );
            }
        }
    }
}
