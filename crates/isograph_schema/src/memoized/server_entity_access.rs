use std::collections::HashMap;

use common_lang_types::{
    JavascriptName, ServerObjectEntityName, ServerScalarEntityName, UnvalidatedTypeName,
    WithLocation,
};
use isograph_lang_types::SelectionType;
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    IsographDatabase, NetworkProtocol, OwnedServerEntity, ServerEntityName, ServerObjectEntity,
    ServerScalarEntity,
};

/// N.B. we should normally not materialize a map here. However, parse_type_system_documents
/// already fully parses the schema, so until that's refactored, there isn't much upside in
/// not materializing a map here.
#[legacy_memo]
fn server_entity_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<UnvalidatedTypeName, Vec<OwnedServerEntity<TNetworkProtocol>>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (outcome, _) = match TNetworkProtocol::parse_type_system_documents(db) {
        Ok(outcome) => outcome,
        Err(e) => return Err(e.clone()),
    };

    let mut server_entities: HashMap<_, Vec<_>> = HashMap::new();

    for item in outcome.iter() {
        match item {
            SelectionType::Scalar(s) => server_entities
                .entry(s.item.name.item.into())
                .or_default()
                .push(SelectionType::Scalar(s.clone())),
            SelectionType::Object(outcome) => server_entities
                .entry(outcome.server_object_entity.item.name.item.into())
                .or_default()
                .push(SelectionType::Object(outcome.server_object_entity.clone())),
        }
    }

    Ok(server_entities)
}

// TODO consider adding a memoized function that creates a map of entities (maybe
// with untracked access?) and going through that.
#[legacy_memo]
pub fn server_entities_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> Result<Vec<OwnedServerEntity<TNetworkProtocol>>, TNetworkProtocol::ParseTypeSystemDocumentsError>
{
    let map = server_entity_map(db).as_ref().map_err(|e| e.clone())?;

    Ok(map.get(&entity_name).cloned().unwrap_or_default())
}

#[legacy_memo]
pub fn server_object_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    Vec<WithLocation<ServerObjectEntity<TNetworkProtocol>>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (outcome, _) = match TNetworkProtocol::parse_type_system_documents(db) {
        Ok(outcome) => outcome,
        Err(e) => return Err(e.clone()),
    };

    Ok(outcome
        .iter()
        .filter_map(|x| x.as_ref().as_object())
        .map(|x| x.server_object_entity.clone())
        .collect())
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum EntityAccessError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),

    #[error("Multiple definitions of `{duplicate_entity_name}` were found")]
    MultipleDefinitionsFound {
        duplicate_entity_name: UnvalidatedTypeName,
    },

    #[error(
        "{server_entity_name} is {actual_entity_type}, but it should be a {intended_entity_type}"
    )]
    IncorrectEntitySelectionType {
        server_entity_name: UnvalidatedTypeName,
        actual_entity_type: &'static str,
        intended_entity_type: &'static str,
    },
}

#[legacy_memo]
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Option<WithLocation<ServerObjectEntity<TNetworkProtocol>>>,
    EntityAccessError<TNetworkProtocol>,
> {
    let entities = server_entities_named(db, server_object_entity_name.into())
        .as_ref()
        .map_err(|e| EntityAccessError::ParseTypeSystemDocumentsError(e.clone()))?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Object(o) => Ok(Some(o.clone())),
                    SelectionType::Scalar(_) => {
                        Err(EntityAccessError::IncorrectEntitySelectionType {
                            server_entity_name: server_object_entity_name.into(),
                            actual_entity_type: "a scalar",
                            intended_entity_type: "an object",
                        })
                    }
                }
            } else {
                Err(EntityAccessError::MultipleDefinitionsFound {
                    duplicate_entity_name: server_object_entity_name.into(),
                })
            }
        }
        None => Ok(None),
    }
}

#[legacy_memo]
pub fn server_scalar_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Result<
    Option<WithLocation<ServerScalarEntity<TNetworkProtocol>>>,
    EntityAccessError<TNetworkProtocol>,
> {
    let entities = server_entities_named(db, server_scalar_entity_name.into())
        .as_ref()
        .map_err(|e| EntityAccessError::ParseTypeSystemDocumentsError(e.clone()))?;

    match entities.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                match first {
                    SelectionType::Scalar(s) => Ok(Some(s.clone())),
                    SelectionType::Object(_) => {
                        Err(EntityAccessError::IncorrectEntitySelectionType {
                            server_entity_name: server_scalar_entity_name.into(),
                            actual_entity_type: "an object",
                            intended_entity_type: "a scalar",
                        })
                    }
                }
            } else {
                Err(EntityAccessError::MultipleDefinitionsFound {
                    duplicate_entity_name: server_scalar_entity_name.into(),
                })
            }
        }
        None => Ok(None),
    }
}

/// TODO remove once we return references
#[legacy_memo]
pub fn server_scalar_entity_javascript_name<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Result<Option<JavascriptName>, EntityAccessError<TNetworkProtocol>> {
    let value = server_scalar_entity_named(db, server_scalar_entity_name)
        .as_ref()
        .map_err(|e| e.clone())?
        .as_ref();

    let entity = match value {
        Some(entity) => entity,
        None => return Ok(None),
    };

    Ok(Some(entity.item.javascript_name))
}

#[legacy_memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: ServerEntityName,
) -> Result<Option<OwnedServerEntity<TNetworkProtocol>>, EntityAccessError<TNetworkProtocol>> {
    match name {
        SelectionType::Object(server_object_entity_name) => {
            let server_object_entity =
                server_object_entity_named(db, server_object_entity_name).to_owned()?;
            if let Some(server_object_entity) = server_object_entity {
                Ok(Some(SelectionType::Object(server_object_entity)))
            } else {
                Ok(None)
            }
        }
        SelectionType::Scalar(server_scalar_entity_name) => {
            let server_scalar_entity =
                server_scalar_entity_named(db, server_scalar_entity_name).to_owned()?;
            if let Some(server_scalar_entity) = server_scalar_entity {
                Ok(Some(SelectionType::Scalar(server_scalar_entity)))
            } else {
                Ok(None)
            }
        }
    }
}

// TODO define this in terms of server_entities_vec??
#[legacy_memo]
pub fn defined_entities<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<UnvalidatedTypeName, Vec<ServerEntityName>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (outcome, _) = match TNetworkProtocol::parse_type_system_documents(db) {
        Ok(outcome) => outcome,
        Err(e) => return Err(e.clone()),
    };

    let mut defined_entities: HashMap<UnvalidatedTypeName, Vec<_>> = HashMap::new();

    for defined_entity in outcome.iter() {
        match defined_entity {
            SelectionType::Object(outcome) => defined_entities
                .entry(outcome.server_object_entity.item.name.item.into())
                .or_default()
                .push(SelectionType::Object(
                    outcome.server_object_entity.item.name.item,
                )),
            SelectionType::Scalar(server_scalar_entity) => defined_entities
                .entry(server_scalar_entity.item.name.item.into())
                .or_default()
                .push(SelectionType::Scalar(server_scalar_entity.item.name.item)),
        }
    }

    Ok(defined_entities)
}

#[legacy_memo]
pub fn defined_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> Result<Option<ServerEntityName>, DefinedEntityError<TNetworkProtocol>> {
    match defined_entities(db)
        .as_ref()
        .map_err(|e| DefinedEntityError::ParseTypeSystemDocumentsError(e.clone()))?
        .get(&entity_name)
    {
        Some(items) => {
            match items.split_first() {
                Some((first, rest)) => {
                    if rest.is_empty() {
                        Ok(Some(*first))
                    } else {
                        Err(DefinedEntityError::MultipleDefinitionsFound {
                            duplicate_entity_name: entity_name,
                        })
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

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DefinedEntityError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),

    // TODO include additional locations
    #[error("Multiple definitions of `{duplicate_entity_name}` were found")]
    MultipleDefinitionsFound {
        duplicate_entity_name: UnvalidatedTypeName,
    },
}
