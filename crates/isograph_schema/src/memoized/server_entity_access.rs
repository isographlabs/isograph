use std::collections::HashMap;

use common_lang_types::{
    Diagnostic, DiagnosticResult, JavascriptName, Location, ServerObjectEntityName,
    ServerScalarEntityName, UnvalidatedTypeName,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, MemoRefServerEntity, NetworkProtocol, ServerEntityName, ServerObjectEntity,
    ServerScalarEntity,
};

/// N.B. we should normally not materialize a map here. However, parse_type_system_documents
/// already fully parses the schema, so until that's refactored, there isn't much upside in
/// not materializing a map here.
#[memo]
fn server_entity_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<HashMap<UnvalidatedTypeName, Vec<MemoRefServerEntity<TNetworkProtocol>>>, Diagnostic> {
    let (outcome, _fetchable_types) =
        TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    let mut server_entities: HashMap<_, Vec<_>> = HashMap::new();

    for with_location in outcome.server_scalar_entities.iter().flatten() {
        server_entities
            .entry(with_location.item.name.into())
            .or_default()
            .push(db.intern_ref(&with_location.item).scalar_selected());
    }

    for with_location in outcome.server_object_entities.iter().flatten() {
        server_entities
            .entry(with_location.item.name.into())
            .or_default()
            .push(db.intern_ref(&with_location.item).object_selected())
    }

    Ok(server_entities)
}

// TODO consider adding a memoized function that creates a map of entities (maybe
// with untracked access?) and going through that.
#[memo]
pub fn server_entities_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> DiagnosticResult<Vec<MemoRefServerEntity<TNetworkProtocol>>> {
    let map = server_entity_map(db).clone_err()?;

    map.get(&entity_name).cloned().unwrap_or_default().wrap_ok()
}

#[memo]
pub fn server_object_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<Vec<ServerObjectEntity<TNetworkProtocol>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .server_object_entities
        .iter()
        .filter_map(|x| match x {
            Ok(object) => Some(object.item.clone()),
            Err(_) => None,
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[memo]
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<Option<MemoRef<ServerObjectEntity<TNetworkProtocol>>>> {
    let entities = server_entities_named(db, server_object_entity_name.into()).clone_err()?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Object(o) => (*o).wrap_some().wrap_ok(),
                    SelectionType::Scalar(_) => {
                        let location =
                            entity_definition_location(db, server_object_entity_name.into())
                                .as_ref()
                                .ok()
                                .cloned()
                                .flatten();
                        entity_wrong_type_diagnostic(
                            server_object_entity_name.into(),
                            "a scalar",
                            "an object",
                            location,
                        )
                        .wrap_err()
                    }
                }
            } else {
                let location = entity_definition_location(db, server_object_entity_name.into())
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten();

                multiple_entity_definitions_found_diagnostic(
                    server_object_entity_name.into(),
                    location,
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
) -> DiagnosticResult<Option<MemoRef<ServerScalarEntity<TNetworkProtocol>>>> {
    let entities = server_entities_named(db, server_scalar_entity_name.into()).clone_err()?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Scalar(s) => (*s)
                        .note_todo("Do not clone. Use a MemoRef.")
                        .wrap_some()
                        .wrap_ok(),
                    SelectionType::Object(_) => {
                        let location =
                            entity_definition_location(db, server_scalar_entity_name.into())
                                .as_ref()
                                .ok()
                                .cloned()
                                .flatten();
                        entity_wrong_type_diagnostic(
                            server_scalar_entity_name.into(),
                            "an object",
                            "a scalar",
                            location,
                        )
                        .wrap_err()
                    }
                }
            } else {
                let location = entity_definition_location(db, server_scalar_entity_name.into())
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten();
                multiple_entity_definitions_found_diagnostic(
                    server_scalar_entity_name.into(),
                    location,
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
    let value = server_scalar_entity_named(db, server_scalar_entity_name).clone()?;

    let entity = match value {
        Some(entity) => entity,
        None => return Ok(None),
    };

    entity.lookup(db).javascript_name.wrap_some().wrap_ok()
}

#[memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: ServerEntityName,
) -> DiagnosticResult<Option<MemoRefServerEntity<TNetworkProtocol>>> {
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
// What is this for??? This is a useless function.
#[memo]
pub fn defined_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<HashMap<UnvalidatedTypeName, Vec<ServerEntityName>>> {
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    let mut defined_entities: HashMap<UnvalidatedTypeName, Vec<_>> = HashMap::new();

    for scalar in outcome.server_scalar_entities.iter().flatten() {
        defined_entities
            .entry(scalar.item.name.into())
            .or_default()
            .push(scalar.item.name.scalar_selected());
    }

    for object in outcome.server_object_entities.iter().flatten() {
        defined_entities
            .entry(object.item.name.into())
            .or_default()
            .push(object.item.name.object_selected());
    }

    Ok(defined_entities)
}

#[memo]
pub fn defined_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> DiagnosticResult<Option<ServerEntityName>> {
    match defined_entities(db).clone_err()?.get(&entity_name) {
        Some(items) => {
            match items.split_first() {
                Some((first, rest)) => {
                    if rest.is_empty() {
                        Ok(Some(*first))
                    } else {
                        let location = entity_definition_location(db, entity_name)
                            .as_ref()
                            .ok()
                            .cloned()
                            .flatten();
                        multiple_entity_definitions_found_diagnostic(entity_name, location)
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
    let (outcome, _) = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .server_object_entities
        .iter()
        .find_map(|result| {
            if let Ok(with_location) = result
                && with_location.item.name == entity_name
            {
                return Some(with_location.location);
            }
            None
        })
        .or_else(|| {
            outcome.server_scalar_entities.iter().find_map(|result| {
                if let Ok(with_location) = result
                    && with_location.item.name == entity_name
                {
                    return Some(with_location.location);
                }
                None
            })
        })
        .wrap_ok()
}

pub fn multiple_entity_definitions_found_diagnostic(
    server_object_entity_name: UnvalidatedTypeName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("Multiple definitions of {server_object_entity_name} were found."),
        location,
    )
}

pub fn entity_wrong_type_diagnostic(
    entity_name: UnvalidatedTypeName,
    actual_type: &'static str,
    intended_type: &'static str,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("{entity_name} is {actual_type}, but it should be {intended_type}"),
        location,
    )
}

pub fn entity_not_defined_diagnostic(
    entity_name: ServerObjectEntityName,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}
