use common_lang_types::{DescriptionValue, ServerObjectEntityName};
use isograph_lang_types::{DefinitionLocation, TypeAnnotation};

use crate::{ClientObjectSelectable, NetworkProtocol, ServerObjectSelectable};

#[allow(clippy::type_complexity)]
pub fn description<TNetworkProtocol: NetworkProtocol>(
    definition_location: &DefinitionLocation<
        &ServerObjectSelectable<TNetworkProtocol>,
        &ClientObjectSelectable<TNetworkProtocol>,
    >,
) -> Option<DescriptionValue> {
    match definition_location {
        DefinitionLocation::Server(server_field) => server_field.description,
        DefinitionLocation::Client(client_field) => client_field.description,
    }
}

pub fn output_type_annotation<'a, TNetworkProtocol: NetworkProtocol>(
    definition_location: &'a DefinitionLocation<
        &ServerObjectSelectable<TNetworkProtocol>,
        &ClientObjectSelectable<TNetworkProtocol>,
    >,
) -> &'a TypeAnnotation<ServerObjectEntityName> {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => &client_pointer.target_object_entity_name,
        DefinitionLocation::Server(server_field) => &server_field.target_object_entity,
    }
}
