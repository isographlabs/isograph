use std::collections::BTreeSet;

use common_lang_types::Diagnostic;
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    ContainsIsoStats, IsographDatabase, NetworkProtocol, create_new_exposed_field,
    create_type_system_schema_with_server_selectables, parse_iso_literals, process_iso_literals,
    server_id_selectable, server_object_entities, server_selectables_map,
    validate_use_of_arguments, validated_entrypoints,
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
) -> Result<ContainsIsoStats, Vec<ValidationError>> {
    let mut errors = BTreeSet::new();

    maybe_extend(
        &mut errors,
        validate_use_of_arguments(db)
            .to_owned()
            .map_err(|e| e.into_iter().map(ValidationError::Diagnostic).collect()),
    );

    maybe_extend(
        &mut errors,
        validate_all_server_selectables_point_to_defined_types(db).to_owned(),
    );

    errors.extend(validate_all_id_fields(db).clone());

    errors.extend(validated_entrypoints(db).values().flat_map(|result| {
        ValidationError::Diagnostic(result.as_ref().err()?.clone()).wrap_some()
    }));

    maybe_extend(&mut errors, validate_all_expose_as_fields(db));

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

fn validate_all_expose_as_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(), Vec<ValidationError>> {
    let expose_as_field_queue = create_type_system_schema_with_server_selectables(db)
        .as_ref()
        .map_err(|e| vec![ValidationError::Diagnostic(e.clone())])?;

    // TODO restructure as a .map or whatnot
    let mut errors = vec![];
    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            if let Err(e) =
                create_new_exposed_field(db, expose_as_field, *parent_object_entity_name)
            {
                errors.push(e.wrap(ValidationError::Diagnostic));
            }
        }
    }

    is_empty_or_ok(errors)
}

fn validate_all_iso_literals<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<ContainsIsoStats, Vec<ValidationError>> {
    let contains_iso = parse_iso_literals(db).to_owned().map_err(|errors| {
        errors
            .into_iter()
            .map(ValidationError::Diagnostic)
            .collect::<Vec<_>>()
    })?;
    let contains_iso_stats = contains_iso.stats();

    if let Err(e) = process_iso_literals(db, contains_iso) {
        return e
            .into_iter()
            .map(ValidationError::Diagnostic)
            .collect::<Vec<_>>()
            .wrap_err();
    }

    Ok(contains_iso_stats)
}

fn is_empty_or_ok<E>(errors: Vec<E>) -> Result<(), Vec<E>> {
    if errors.is_empty() {
        Err(errors)
    } else {
        Ok(())
    }
}

fn maybe_extend<T, E>(errors_acc: &mut impl Extend<E>, result: Result<T, Vec<E>>) {
    if let Err(e) = result {
        errors_acc.extend(e);
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Error, PartialOrd, Ord)]
pub enum ValidationError {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(Diagnostic),

    #[error("{}", error)]
    FieldToInsertToServerSelectableError { error: Diagnostic },

    #[error("{0}")]
    Diagnostic(Diagnostic),
}

#[memo]
fn validate_all_server_selectables_point_to_defined_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(), Vec<ValidationError>> {
    // Note: server_selectables_map is a HashMap<_, Vec<(_, Result)>
    // That result encodes whether the field exists. So, basically, we are collecting
    // each error from that result.
    //
    // This can and should be rethought! Namely, just because the referenced entity doesn't exist
    // doesn't mean that the selectable can't be materialized. Instead, the result should be
    // materialized when we actually need to look at the referenced entity.
    let server_selectables = server_selectables_map(db)
        .as_ref()
        .map_err(|e| ValidationError::ParseTypeSystemDocumentsError(e.clone()))
        .map_err(|e| vec![e])?;

    let mut errors = vec![];

    for selectables in server_selectables.values() {
        for (_, selectable_result) in selectables {
            if let Err(e) = selectable_result {
                errors.push(ValidationError::FieldToInsertToServerSelectableError {
                    error: e.clone(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[memo]
fn validate_all_id_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<ValidationError> {
    let entities = match server_object_entities(db).as_ref() {
        Ok(entities) => entities,
        Err(e) => return vec![ValidationError::Diagnostic(e.clone())],
    };

    entities
        .iter()
        .flat_map(|entity| {
            Result::err(
                server_id_selectable(db, entity.name)
                    .as_ref()
                    .map_err(|e| e.clone()),
            )
        })
        .map(ValidationError::Diagnostic)
        .collect()
}
