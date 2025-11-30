use std::collections::BTreeSet;

use common_lang_types::{Diagnostic, DiagnosticVecResult};
use isograph_lang_types::SelectionType;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientFieldVariant, ContainsIsoStats, IsographDatabase, NetworkProtocol, client_selectable_map,
    parse_iso_literals, process_iso_literals, server_id_selectable, server_object_entities,
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
    // Note: server_selectables_map is a HashMap<_, Vec<(_, Result)>
    // That result encodes whether the field exists. So, basically, we are collecting
    // each error from that result.
    //
    // This can and should be rethought! Namely, just because the referenced entity doesn't exist
    // doesn't mean that the selectable can't be materialized. Instead, the result should be
    // materialized when we actually need to look at the referenced entity.
    let server_selectables = server_selectables_map(db).clone_err()?;

    let mut errors = vec![];

    // TODO use iterator methods
    for selectables in server_selectables.values() {
        for (_, selectable_result) in selectables {
            if let Err(e) = selectable_result {
                errors.push(e.clone());
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
        .flat_map(|entity| Result::err(server_id_selectable(db, entity.name).clone_err()))
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
                    if let ClientFieldVariant::UserWritten(u) = &s.variant
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
