use std::ops::Deref;

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

// TODO consider adding a memoized function that creates a map of entities (maybe
// with untracked access?) and going through that.
#[legacy_memo]
pub fn server_entities_named<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: UnvalidatedTypeName,
) -> Result<Vec<OwnedServerEntity<TNetworkProtocol>>, TNetworkProtocol::ParseTypeSystemDocumentsError>
{
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (outcome, _) = match memo_ref.deref() {
        Ok(outcome) => outcome,
        Err(e) => return Err(e.clone()),
    };

    Ok(outcome
        .iter()
        .filter_map(|x| match x {
            SelectionType::Object(o) => {
                // Why??
                let name: UnvalidatedTypeName =
                    o.server_object_entity.item.name.item.unchecked_conversion();
                if name == entity_name {
                    Some(SelectionType::Object(o.server_object_entity.clone()))
                } else {
                    None
                }
            }
            SelectionType::Scalar(s) => {
                // Why??
                let name: UnvalidatedTypeName = s.item.name.item.unchecked_conversion();
                if name == entity_name {
                    Some(SelectionType::Scalar(s.clone()))
                } else {
                    None
                }
            }
        })
        .collect::<Vec<_>>())
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum EntityAccessError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),

    #[error("Multiple definitions of {server_entity_name} found")]
    MultipleDefinitionsFound {
        server_entity_name: UnvalidatedTypeName,
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
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Option<WithLocation<ServerObjectEntity<TNetworkProtocol>>>,
    EntityAccessError<TNetworkProtocol>,
> {
    let memo_ref = server_entities_named(db, server_object_entity_name.into());
    let entities = memo_ref
        .deref()
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
                    server_entity_name: server_object_entity_name.into(),
                })
            }
        }
        None => Ok(None),
    }
}

#[legacy_memo]
pub fn server_scalar_entity_named<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Result<
    Option<WithLocation<ServerScalarEntity<TNetworkProtocol>>>,
    EntityAccessError<TNetworkProtocol>,
> {
    let memo_ref = server_entities_named(db, server_scalar_entity_name.into());
    let entities = memo_ref
        .deref()
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
                    server_entity_name: server_scalar_entity_name.into(),
                })
            }
        }
        None => Ok(None),
    }
}

#[legacy_memo]
pub fn server_scalar_entity_javascript_name<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Result<Option<JavascriptName>, EntityAccessError<TNetworkProtocol>> {
    let memo_ref = server_scalar_entity_named(db, server_scalar_entity_name);
    let value = memo_ref.deref().as_ref().map_err(|e| e.clone())?.as_ref();

    let entity = match value {
        Some(entity) => entity,
        None => return Ok(None),
    };

    Ok(Some(entity.item.javascript_name))
}

#[legacy_memo]
pub fn server_entity_named<TNetworkProtocol: NetworkProtocol + 'static>(
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
