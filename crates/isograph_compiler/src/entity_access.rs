use std::ops::Deref;

use common_lang_types::{
    ServerObjectEntityName, ServerScalarEntityName, UnvalidatedTypeName, WithLocation,
};
use isograph_lang_types::SelectionType;
use isograph_schema::{
    IsographDatabase, NetworkProtocol, OwnedServerEntity, ServerObjectEntity, ServerScalarEntity,
};
use pico_macros::memo;

// TODO go through server_entities
// TODO what to do about scalars with the same name. Should this return an Err? Should it ignore them?
#[memo]
pub fn server_object_entities<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Vec<WithLocation<ServerObjectEntity<TNetworkProtocol>>> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (outcome, _) = match memo_ref.deref() {
        Ok(outcome) => outcome,
        Err(_) => return vec![],
    };

    outcome
        .iter()
        .filter_map(|x| match x {
            SelectionType::Object(o) => {
                if o.server_object_entity.item.name.item == server_object_entity_name {
                    Some(o.server_object_entity.clone())
                } else {
                    None
                }
            }
            SelectionType::Scalar(_) => None,
        })
        .collect::<Vec<_>>()
}

// TODO go through server_entities
// TODO what to do about objects with the same name. Should this return an Err? Should it ignore them?
#[memo]
pub fn server_scalar_entities<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Vec<WithLocation<ServerScalarEntity<TNetworkProtocol>>> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (outcome, _) = match memo_ref.deref() {
        Ok(outcome) => outcome,
        Err(_) => return vec![],
    };

    outcome
        .iter()
        .filter_map(|x| match x {
            SelectionType::Object(_) => None,
            SelectionType::Scalar(s) => {
                if s.item.name.item == server_scalar_entity_name {
                    Some(s.clone())
                } else {
                    None
                }
            }
        })
        .collect::<Vec<_>>()
}

#[memo]
pub fn server_entities<TNetworkProtocol: NetworkProtocol + 'static>(
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
