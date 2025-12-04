use common_lang_types::ServerObjectEntityName;
use isograph_lang_types::{DefinitionLocation, Description, TypeAnnotation};

use crate::{IsographDatabase, NetworkProtocol, OwnedObjectSelectable};

pub fn description<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    definition_location: &OwnedObjectSelectable<TNetworkProtocol>,
) -> Option<Description> {
    match definition_location {
        DefinitionLocation::Server(server_field) => server_field.lookup(db).description,
        DefinitionLocation::Client(client_field) => client_field.description,
    }
}

pub fn output_type_annotation<'a, TNetworkProtocol: NetworkProtocol>(
    db: &'a IsographDatabase<TNetworkProtocol>,
    definition_location: &'a OwnedObjectSelectable<TNetworkProtocol>,
) -> &'a TypeAnnotation<ServerObjectEntityName> {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => &client_pointer.target_object_entity_name,
        DefinitionLocation::Server(server_field) => &server_field.lookup(db).target_object_entity,
    }
}
