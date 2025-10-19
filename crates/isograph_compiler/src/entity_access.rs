use std::ops::Deref;

use common_lang_types::{ServerObjectEntityName, ServerScalarEntityName};
use isograph_schema::{IsographDatabase, NetworkProtocol, ServerObjectEntity, ServerScalarEntity};
use pico_macros::memo;

use crate::create_type_system_schema;

#[memo]
pub fn server_object_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Vec<ServerObjectEntity<TNetworkProtocol>> {
    let memo_ref = TNetworkProtocol::parse_and_process_type_system_documents(db);
    let (outcome, _) = match memo_ref.deref() {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let outcome = outcome.objects.get(&server_object_entity_name);

    match outcome {
        Some(vec) => vec
            .iter()
            .map(|x| x.0.server_object_entity.clone())
            .collect(),
        None => vec![],
    }
}

#[memo]
pub fn server_scalar_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Option<ServerScalarEntity<TNetworkProtocol>> {
    let memo_ref = create_type_system_schema(db);
    let (schema, _expose_as_fields_to_insert, _fields_to_insert) = match memo_ref.deref() {
        Ok(s) => s,
        Err(_) => return None,
    };

    let entity = schema
        .server_entity_data
        .server_scalar_entity(server_scalar_entity_name);

    // TODO return Option<&ServerScalarEntity> when this is supported
    entity.cloned()
}
