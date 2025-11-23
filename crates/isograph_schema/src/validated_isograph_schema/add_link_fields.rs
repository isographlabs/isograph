use std::collections::HashMap;

use crate::{
    ClientFieldVariant, ClientScalarSelectable, IsographDatabase, LINK_FIELD_NAME, NetworkProtocol,
    server_object_entities,
    validated_isograph_schema::create_type_system_schema::CreateSchemaError,
};
use common_lang_types::{
    ClientScalarSelectableName, ParentObjectEntityNameAndSelectableName, ServerObjectEntityName,
    WithLocationPostfix,
};
use intern::string_key::Intern;
use isograph_lang_types::Description;
use pico_macros::memo;
use prelude::Postfix;

#[memo]
pub fn get_link_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<Vec<ClientScalarSelectable<TNetworkProtocol>>, CreateSchemaError> {
    server_object_entities(db)
        .as_ref()
        .map_err(|e| CreateSchemaError::ParseAndProcessTypeSystemDocument { message: e.clone() })?
        .iter()
        .map(|object| {
            let field_name = *LINK_FIELD_NAME;
            let parent_object_entity_name = object.name;
            ClientScalarSelectable {
                description: Some(Description(
                    format!("A store Link for the {} type.", object.name)
                        .intern()
                        .into(),
                )),
                name: field_name.with_generated_location(),
                parent_object_entity_name,
                variable_definitions: vec![],
                variant: ClientFieldVariant::Link,
                type_and_field: ParentObjectEntityNameAndSelectableName {
                    selectable_name: field_name.into(),
                    parent_object_entity_name: object.name,
                },
                network_protocol: std::marker::PhantomData,
            }
        })
        .collect::<Vec<_>>()
        .ok()
}

#[memo]
pub fn get_link_fields_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        ClientScalarSelectable<TNetworkProtocol>,
    >,
    CreateSchemaError,
> {
    get_link_fields(db)
        .to_owned()?
        .into_iter()
        .map(|link_selectable| {
            (
                (link_selectable.parent_object_entity_name, *LINK_FIELD_NAME),
                link_selectable,
            )
        })
        .collect::<HashMap<_, _>>()
        .ok()
}
