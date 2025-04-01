use common_lang_types::DescriptionValue;
use isograph_lang_types::{DefinitionLocation, SelectionType, ServerObjectId, TypeAnnotation};

use crate::{ClientPointer, NetworkProtocol, ServerScalarSelectable};

#[allow(clippy::type_complexity)]
pub fn description<TNetworkProtocol: NetworkProtocol>(
    definition_location: &DefinitionLocation<
        &ServerScalarSelectable<TNetworkProtocol>,
        &ClientPointer<TNetworkProtocol>,
    >,
) -> Option<DescriptionValue> {
    match definition_location {
        DefinitionLocation::Server(server_field) => server_field.description,
        DefinitionLocation::Client(client_field) => client_field.description,
    }
}

pub fn output_type_annotation<'a, TNetworkProtocol: NetworkProtocol>(
    definition_location: &'a DefinitionLocation<
        &ServerScalarSelectable<TNetworkProtocol>,
        &ClientPointer<TNetworkProtocol>,
    >,
) -> &'a TypeAnnotation<ServerObjectId> {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => &client_pointer.to,
        DefinitionLocation::Server(server_field) => match &server_field.target_server_entity {
            SelectionType::Scalar(_) => {
                panic!(
                    "output_type_id should be an object. \
                    This is indicative of a bug in Isograph.",
                )
            }
            SelectionType::Object((_, type_annotation)) => type_annotation,
        },
    }
}
