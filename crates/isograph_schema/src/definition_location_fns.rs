use common_lang_types::DescriptionValue;
use isograph_lang_types::{DefinitionLocation, ServerObjectId, TypeAnnotation};

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
) -> &'a TypeAnnotation<ServerObjectId> {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => &client_pointer.to,
        DefinitionLocation::Server(server_field) => &server_field.target_object_entity,
    }
}
