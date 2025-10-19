use std::ops::Deref;

use common_lang_types::{ServerObjectEntityName, ServerScalarEntityName, WithLocation};
use isograph_lang_types::SelectionType;
use isograph_schema::{IsographDatabase, NetworkProtocol, ServerObjectEntity, ServerScalarEntity};
use pico_macros::memo;

#[memo]
pub fn server_object_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Vec<WithLocation<ServerObjectEntity<TNetworkProtocol>>> {
    let memo_ref = TNetworkProtocol::parse_and_process_type_system_documents(db);
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

#[memo]
pub fn server_scalar_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Vec<WithLocation<ServerScalarEntity<TNetworkProtocol>>> {
    let memo_ref = TNetworkProtocol::parse_and_process_type_system_documents(db);
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
