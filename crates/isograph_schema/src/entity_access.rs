use std::ops::Deref;

use common_lang_types::{ServerObjectEntityName, UnvalidatedTypeName, WithLocation};
use isograph_lang_types::SelectionType;
use pico_macros::memo;
use thiserror::Error;

use crate::{IsographDatabase, NetworkProtocol, OwnedServerEntity, ServerObjectEntity};

// TODO consider adding a memoized function that creates a map of entities (maybe
// with untracked access?) and going through that.
#[memo]
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
pub enum EntityAccessError<TParseTypeSystemDocumentsError> {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(#[from] TParseTypeSystemDocumentsError),

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

#[memo]
pub fn server_object_entity_named<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Option<WithLocation<ServerObjectEntity<TNetworkProtocol>>>,
    EntityAccessError<TNetworkProtocol::ParseTypeSystemDocumentsError>,
> {
    let memo_ref = server_entities_named(db, server_object_entity_name.into());
    let entities = memo_ref.deref().as_ref().map_err(|e| e.clone())?;

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
