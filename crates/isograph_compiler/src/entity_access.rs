use std::ops::Deref;

use common_lang_types::{ServerObjectEntityName, ServerScalarEntityName};
use isograph_schema::{IsographDatabase, NetworkProtocol, ServerObjectEntity, ServerScalarEntity};
use pico_macros::memo;

use crate::create_type_system_schema;

#[memo]
pub fn server_object_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    database: &IsographDatabase<TNetworkProtocol>,
    server_object_entity_name: ServerObjectEntityName,
) -> Option<ServerObjectEntity<TNetworkProtocol>> {
    let memo_ref = create_type_system_schema(database);
    let (schema, _unprocessed_field_items) = match memo_ref.deref() {
        Ok(s) => s,
        Err(_) => return None,
    };

    let entity = schema
        .server_entity_data
        .server_object_entity(server_object_entity_name);

    // TODO return Option<&ServerObjectEntity> when this is supported
    entity.cloned()
}

#[memo]
pub fn server_scalar_entity<TNetworkProtocol: NetworkProtocol + 'static>(
    database: &IsographDatabase<TNetworkProtocol>,
    server_scalar_entity_name: ServerScalarEntityName,
) -> Option<ServerScalarEntity<TNetworkProtocol>> {
    let memo_ref = create_type_system_schema(database);
    let (schema, _unprocessed_field_items) = match memo_ref.deref() {
        Ok(s) => s,
        Err(_) => return None,
    };

    let entity = schema
        .server_entity_data
        .server_scalar_entity(server_scalar_entity_name);

    // TODO return Option<&ServerScalarEntity> when this is supported
    entity.cloned()
}
