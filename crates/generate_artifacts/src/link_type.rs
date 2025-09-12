use common_lang_types::{
    ArtifactPathAndContent, ParentObjectEntityNameAndSelectableName, ServerObjectEntityName,
};
use intern::string_key::Intern;

use crate::generate_artifacts::LINK_TYPE_FILE_NAME;

pub fn generate_link_type_artifact<TNetworkProtocol: isograph_schema::NetworkProtocol>(
    schema: &isograph_schema::Schema<TNetworkProtocol>,
    server_object_entity_name: &ServerObjectEntityName,
) -> ArtifactPathAndContent {
    let link_type = TNetworkProtocol::generate_link_type(schema, &server_object_entity_name);

    let link_type_content = format!(
        "import type {{ Link }} from '@isograph/react';\n
export type {server_object_entity_name}LinkFutureType = Link<\"%{server_object_entity_name} future type%\">;\n
export type {server_object_entity_name}Link = {link_type};\n",
    );

    ArtifactPathAndContent {
        file_content: link_type_content,
        file_name: *LINK_TYPE_FILE_NAME,
        type_and_field: Some(ParentObjectEntityNameAndSelectableName {
            field_name: "".intern().into(),
            type_name: *server_object_entity_name,
        }),
    }
}
