use std::collections::BTreeSet;

use common_lang_types::{Diagnostic, DiagnosticVecResult, Location};
use isograph_lang_types::{DefinitionLocation, SelectionType};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientFieldVariant, CompilationProfile, ContainsIsoStats, IsographDatabase,
    client_selectable_declaration_map_from_iso_literals, client_selectable_map,
    deprecated_server_selectables_map, entity_not_defined_diagnostic, flattened_entities,
    flattened_entity_named, flattened_server_object_entities, parse_iso_literals,
    process_iso_literals, selectables, server_id_selectable,
    validate_selection_sets::validate_selection_sets, validate_use_of_arguments,
    validated_entrypoints,
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
pub fn validate_entire_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticVecResult<ContainsIsoStats> {
    let mut errors = BTreeSet::new();

    maybe_extend(&mut errors, validate_use_of_arguments(db));

    errors.extend(validate_selectables(db));

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

    if let Ok((outcome, _)) = TCompilationProfile::deprecated_parse_type_system_documents(db) {
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

fn validate_all_iso_literals<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
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

fn validate_all_server_selectables_point_to_defined_types<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticVecResult<()> {
    let server_selectables = deprecated_server_selectables_map(db);
    let entities = flattened_entities(db).to_owned();

    let mut errors = vec![];

    // TODO use iterator methods
    for ((parent_object_entity_name, selectable_name), selectable) in server_selectables.iter() {
        let (target, arguments) = {
            let selectable = selectable.lookup(db);
            (
                selectable.target_entity.item.clone_err()?.inner(),
                &selectable.arguments,
            )
        };

        if !entities.contains_key(&target) {
            errors.push(Diagnostic::new(
                format!(
                    "`{parent_object_entity_name}.{selectable_name}` has inner \
                    type `{target}, but that type has not been defined"
                ),
                None.note_todo("Get the location from the declaration here"),
            ))
        }

        for argument in arguments {
            let arg_target = argument.type_.item.inner();

            if !entities.contains_key(&arg_target) {
                let arg_name = argument.name.item;
                errors.push(Diagnostic::new(
                    format!(
                        "In `{parent_object_entity_name}.{selectable_name}`, the argument `{arg_name}` has inner \
                        type `{arg_target}, but that type has not been defined"
                    ),
                    None.note_todo("Get the location from the declaration here"),
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

fn validate_all_id_fields<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<Diagnostic> {
    let entities = flattened_server_object_entities(db);

    entities
        .iter()
        .flat_map(|entity| {
            server_id_selectable(db, entity.lookup(db).name.item)
                .clone_err()
                .err()
        })
        .collect()
}

fn validate_scalar_selectable_directive_sets<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
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

/// Validate selectables:
/// - that each variable definition points to an actual type
/// - TODO move the rest of the valuations that relate to entities into this
fn validate_selectables<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<Diagnostic> {
    let selectables = match selectables(db).clone_err() {
        Ok(selectables) => selectables,
        Err(e) => return e.wrap_vec(),
    };

    let mut errors = vec![];

    for selectable in selectables {
        let arguments = match selectable {
            DefinitionLocation::Server(s) => s.lookup(db).arguments.reference(),
            DefinitionLocation::Client(c) => match c {
                SelectionType::Scalar(s) => s.lookup(db).variable_definitions.reference(),
                SelectionType::Object(o) => o.lookup(db).variable_definitions.reference(),
            },
        };

        for argument in arguments {
            let target = argument.type_.item.inner().0;

            let entity = flattened_entity_named(db, target);
            match entity {
                Some(_) => {
                    // Note: we should also validate that the entity is an input entity (i.e.
                    // scalar or input type). That's a GraphQL-ism, though, so we have to figure
                    // out a good mechanism for that (realistically, call out to a method on
                    // NetworkProtocol).
                }
                None => errors.push(entity_not_defined_diagnostic(
                    target,
                    argument
                        .name
                        .location
                        .to::<Location>()
                        .note_todo("Variable definition will not have location at some point"),
                )),
            }
        }
    }

    errors
}
