use std::collections::HashMap;

use common_lang_types::{
    Diagnostic, DiagnosticResult, JavascriptName, Location, ServerObjectEntityName,
    ServerScalarEntityName, UnvalidatedTypeName,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    IsographDatabase, NetworkProtocol, OwnedServerEntity, ServerEntityName, ServerObjectEntity,
    ServerScalarEntity,
};

/// N.B. we should normally not materialize a map here. However, parse_type_system_documents
/// already fully parses the schema, so until that's refactored, there isn't much upside in
/// not materializing a map here.
#[memo]
fn server_entity_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<HashMap<UnvalidatedTypeName, Vec<OwnedServerEntity<TNetworkProtocol>>>, Diagnostic> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    let mut server_entities: HashMap<_, Vec<_>> = HashMap::new();

    for item in outcome.iter() {
        match item {
            SelectionType::Scalar(s) => server_entities
                .entry(s.item.name.into())
                .or_default()
                .push(s.item.clone().scalar_selected()),
            SelectionType::Object(outcome) => server_entities
                .entry(outcome.server_object_entity.item.name.into())
                .or_default()
                .push(outcome.server_object_entity.item.clone().object_selected()),
        }
    }

    Ok(server_entities)
}

// TODO consider adding a memoized function that creates a map of entities (maybe
// with untracked access?) and going through that.
#[memo]
pub fn server_entities_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> DiagnosticResult<Vec<OwnedServerEntity<TNetworkProtocol>>> {
    let map = server_entity_map(db).as_ref().map_err(|e| e.clone())?;

    map.get(&entity_name).cloned().unwrap_or_default().wrap_ok()
}

#[memo]
pub fn server_object_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<Vec<ServerObjectEntity<TNetworkProtocol>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    outcome
        .iter()
        .filter_map(|x| x.as_ref().as_object())
        .map(|x| x.server_object_entity.item.clone())
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[memo]
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<Option<ServerObjectEntity<TNetworkProtocol>>> {
    let entities = server_entities_named(db, server_object_entity_name.into())
        .as_ref()
        .map_err(|e| e.clone())?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Object(o) => o.clone().wrap_some().wrap_ok(),
                    SelectionType::Scalar(_) => Diagnostic::new(
                        format!(
                            "{server_object_entity_name} is a scalar, but it should be an object"
                        ),
                        entity_definition_location(db, server_object_entity_name.into())
                            .as_ref()
                            .ok()
                            .cloned()
                            .flatten(),
                    )
                    .wrap_err(),
                }
            } else {
                Diagnostic::new(
                    format!("Multiple definitions of {server_object_entity_name} were found."),
                    entity_definition_location(db, server_object_entity_name.into())
                        .as_ref()
                        .ok()
                        .cloned()
                        .flatten(),
                )
                .wrap_err()
            }
        }
        None => Ok(None),
    }
}

#[memo]
pub fn server_scalar_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> DiagnosticResult<Option<ServerScalarEntity<TNetworkProtocol>>> {
    let entities = server_entities_named(db, server_scalar_entity_name.into())
        .as_ref()
        .map_err(|e| e.clone())?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Scalar(s) => Ok(Some(s.clone())),
                    SelectionType::Object(_) => Diagnostic::new(
                        format!(
                            "{server_scalar_entity_name} is an object, but it should be a scalar"
                        ),
                        entity_definition_location(db, server_scalar_entity_name.into())
                            .as_ref()
                            .ok()
                            .cloned()
                            .flatten(),
                    )
                    .wrap_err(),
                }
            } else {
                Diagnostic::new(
                    format!("Multiple definitions of {server_scalar_entity_name} were found"),
                    entity_definition_location(db, server_scalar_entity_name.into())
                        .as_ref()
                        .ok()
                        .cloned()
                        .flatten(),
                )
                .wrap_err()
            }
        }
        None => Ok(None),
    }
}

/// TODO remove once we return references
#[memo]
pub fn server_scalar_entity_javascript_name<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> DiagnosticResult<Option<JavascriptName>> {
    let value = server_scalar_entity_named(db, server_scalar_entity_name)
        .as_ref()
        .map_err(|e| e.clone())?
        .as_ref();

    let entity = match value {
        Some(entity) => entity,
        None => return Ok(None),
    };

    Ok(Some(entity.javascript_name))
}

#[memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: ServerEntityName,
) -> DiagnosticResult<Option<OwnedServerEntity<TNetworkProtocol>>> {
    match name {
        SelectionType::Object(server_object_entity_name) => {
            let server_object_entity =
                server_object_entity_named(db, server_object_entity_name).to_owned()?;
            if let Some(server_object_entity) = server_object_entity {
                server_object_entity.object_selected().wrap_some().wrap_ok()
            } else {
                Ok(None)
            }
        }
        SelectionType::Scalar(server_scalar_entity_name) => {
            let server_scalar_entity =
                server_scalar_entity_named(db, server_scalar_entity_name).to_owned()?;
            if let Some(server_scalar_entity) = server_scalar_entity {
                server_scalar_entity.scalar_selected().wrap_some().wrap_ok()
            } else {
                Ok(None)
            }
        }
    }
}

// TODO define this in terms of server_entities_vec??
#[memo]
pub fn defined_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<HashMap<UnvalidatedTypeName, Vec<ServerEntityName>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    let mut defined_entities: HashMap<UnvalidatedTypeName, Vec<_>> = HashMap::new();

    for defined_entity in outcome.iter() {
        match defined_entity {
            SelectionType::Object(outcome) => defined_entities
                .entry(outcome.server_object_entity.item.name.into())
                .or_default()
                .push(outcome.server_object_entity.item.name.object_selected()),
            SelectionType::Scalar(server_scalar_entity) => defined_entities
                .entry(server_scalar_entity.item.name.into())
                .or_default()
                .push(server_scalar_entity.item.name.scalar_selected()),
        }
    }

    Ok(defined_entities)
}

#[memo]
pub fn defined_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> DiagnosticResult<Option<ServerEntityName>> {
    match defined_entities(db)
        .as_ref()
        .map_err(|e| e.clone())?
        .get(&entity_name)
    {
        Some(items) => {
            match items.split_first() {
                Some((first, rest)) => {
                    if rest.is_empty() {
                        Ok(Some(*first))
                    } else {
                        Diagnostic::new(
                            format!("Multiple definitions of {entity_name} were found"),
                            entity_definition_location(db, entity_name)
                                .as_ref()
                                .ok()
                                .cloned()
                                .flatten(),
                        )
                        .wrap_err()
                    }
                }
                None => {
                    // Empty, this shouldn't happen. We can consider having a NonEmptyVec or something
                    Ok(None)
                }
            }
        }
        None => Ok(None),
    }
}

/// Finds the entity of the first entity with the target name.
#[memo]
pub fn entity_definition_location<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> DiagnosticResult<Option<Location>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    outcome
        .iter()
        .find_map(|item| {
            match item {
                SelectionType::Scalar(s) => {
                    let name: UnvalidatedTypeName = s.item.name.into();
                    if name == entity_name {
                        return Some(s.location);
                    }
                }
                SelectionType::Object(o) => {
                    let name: UnvalidatedTypeName = o.server_object_entity.item.name.into();
                    if name == entity_name {
                        return Some(o.server_object_entity.location);
                    }
                }
            }
            None
        })
        .wrap_ok()
}
