use std::collections::BTreeMap;

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, MemoRefServerEntity, NetworkProtocol, ServerObjectEntity, ServerScalarEntity,
};

/// This function just drops the locations
#[memo]
pub fn server_entities_map_without_locations<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<MemoRef<BTreeMap<EntityName, MemoRefServerEntity<TNetworkProtocol>>>, Diagnostic> {
    let (outcome, _fetchable_types) =
        TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .iter()
        .map(|(entity_name, entities)| (*entity_name, entities.item))
        .collect::<BTreeMap<_, _>>()
        .interned_value(db)
        .wrap_ok()
}

#[memo]
pub fn server_object_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<Vec<MemoRef<ServerObjectEntity<TNetworkProtocol>>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .iter()
        .filter_map(|(_, x)| x.item.as_object())
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[memo]
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: EntityName,
) -> DiagnosticResult<Option<MemoRef<ServerObjectEntity<TNetworkProtocol>>>> {
    let map = server_entities_map_without_locations(db)
        .clone_err()?
        .lookup(db);

    match map.get(&server_object_entity_name) {
        Some(entity) => match entity {
            SelectionType::Scalar(_) => {
                let location = entity_definition_location(db, server_object_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten();
                entity_wrong_type_diagnostic(
                    server_object_entity_name,
                    "a scalar",
                    "an object",
                    location,
                )
                .wrap_err()
            }
            SelectionType::Object(o) => (*o).wrap_some().wrap_ok(),
        },
        None => None.wrap_ok(),
    }
}

#[memo]
pub fn server_scalar_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: EntityName,
) -> DiagnosticResult<Option<MemoRef<ServerScalarEntity<TNetworkProtocol>>>> {
    let map = server_entities_map_without_locations(db)
        .clone_err()?
        .lookup(db);

    match map.get(&server_scalar_entity_name) {
        Some(entity) => match entity {
            SelectionType::Object(_) => {
                let location = entity_definition_location(db, server_scalar_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten();
                entity_wrong_type_diagnostic(
                    server_scalar_entity_name,
                    "an object",
                    "a scalar",
                    location,
                )
                .wrap_err()
            }
            SelectionType::Scalar(s) => (*s).wrap_some().wrap_ok(),
        },
        None => None.wrap_ok(),
    }
}

#[memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: EntityName,
) -> DiagnosticResult<Option<MemoRefServerEntity<TNetworkProtocol>>> {
    if let Ok(Some(server_object_entity)) = server_object_entity_named(db, name).to_owned() {
        return server_object_entity.object_selected().wrap_some().wrap_ok();
    };

    let server_scalar_entity = server_scalar_entity_named(db, name).to_owned()?;
    if let Some(server_scalar_entity) = server_scalar_entity {
        return server_scalar_entity.scalar_selected().wrap_some().wrap_ok();
    }
    Ok(None)
}

// TODO what is this for?? We should get rid of this.
#[memo]
pub fn defined_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: EntityName,
) -> DiagnosticResult<Option<SelectionType<EntityName, EntityName>>> {
    match server_entities_map_without_locations(db)
        .clone_err()?
        .lookup(db)
        .get(&entity_name)
    {
        Some(entity) => match entity {
            SelectionType::Scalar(s) => s.lookup(db).name.scalar_selected().wrap_some().wrap_ok(),
            SelectionType::Object(o) => o.lookup(db).name.object_selected().wrap_some().wrap_ok(),
        },
        None => None.wrap_ok(),
    }
}

#[memo]
pub fn entity_definition_location<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: EntityName,
) -> DiagnosticResult<Option<Location>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .get(&entity_name)
        .map(|x| x.location)
        .wrap_ok()
}

pub fn entity_wrong_type_diagnostic(
    entity_name: EntityName,
    actual_type: &'static str,
    intended_type: &'static str,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("{entity_name} is {actual_type}, but it should be {intended_type}"),
        location,
    )
}

pub fn entity_not_defined_diagnostic(entity_name: EntityName, location: Location) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}
