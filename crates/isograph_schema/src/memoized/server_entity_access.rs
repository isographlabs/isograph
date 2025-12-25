use std::collections::BTreeMap;

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{IsographDatabase, MemoRefServerEntity, NetworkProtocol};

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
) -> DiagnosticResult<Vec<MemoRefServerEntity<TNetworkProtocol>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .iter()
        .filter_map(|(_, x)| {
            if x.item.lookup(db).selection_info.as_object().is_some() {
                x.item.wrap_some()
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: EntityName,
) -> DiagnosticResult<Option<MemoRefServerEntity<TNetworkProtocol>>> {
    let map = server_entities_map_without_locations(db)
        .clone_err()?
        .lookup(db);

    map.get(&name).copied().wrap_ok()
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

pub fn entity_not_defined_diagnostic(entity_name: EntityName, location: Location) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}
