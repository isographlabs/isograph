use std::{collections::HashMap, ops::Deref};

use crate::{
    ClientFieldVariant, ClientScalarSelectable, IsographDatabase, LINK_FIELD_NAME, NetworkProtocol,
    Schema, server_object_entities,
    validated_isograph_schema::create_type_system_schema::CreateSchemaError,
};
use common_lang_types::{
    ClientScalarSelectableName, Location, ParentObjectEntityNameAndSelectableName,
    ServerObjectEntityName, WithLocation,
};
use intern::string_key::Intern;
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};
use pico_macros::legacy_memo;

pub fn add_link_fields_to_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
) -> Result<(), CreateSchemaError<TNetworkProtocol>> {
    let link_fields = get_link_fields(db).to_owned()?;

    for link_field in link_fields {
        if schema
            .server_entity_data
            .entry(link_field.parent_object_entity_name)
            .or_default()
            .selectables
            .insert(
                link_field.name.item.into(),
                DefinitionLocation::Client(SelectionType::Scalar((
                    link_field.parent_object_entity_name,
                    link_field.name.item,
                ))),
            )
            .is_some()
        {
            return Err(CreateSchemaError::CompilerCreatedFieldExistsOnType {
                selectable_name: link_field.name.item.into(),
                parent_object_entity_name: link_field.parent_object_entity_name,
            });
        }

        schema.client_scalar_selectables.insert(
            (link_field.parent_object_entity_name, link_field.name.item),
            link_field,
        );
    }

    Ok(())
}

#[legacy_memo]
pub fn get_link_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<Vec<ClientScalarSelectable<TNetworkProtocol>>, CreateSchemaError<TNetworkProtocol>> {
    let memo_ref = server_object_entities(db);
    Ok(memo_ref
        .deref()
        .as_ref()
        .map_err(|e| CreateSchemaError::ParseAndProcessTypeSystemDocument { message: e.clone() })?
        .iter()
        .map(|object| {
            let object = &object.item;
            let field_name = *LINK_FIELD_NAME;
            let parent_object_entity_name = object.name;
            ClientScalarSelectable {
                description: Some(Description(
                    format!("A store Link for the {} type.", object.name.item)
                        .intern()
                        .into(),
                )),
                name: WithLocation::new(field_name, Location::generated()),
                parent_object_entity_name: parent_object_entity_name.item,
                variable_definitions: vec![],
                reader_selection_set: vec![],
                variant: ClientFieldVariant::Link,
                type_and_field: ParentObjectEntityNameAndSelectableName {
                    selectable_name: field_name.into(),
                    parent_object_entity_name: object.name.item,
                },
                refetch_strategy: None,
                network_protocol: std::marker::PhantomData,
            }
        })
        .collect())
}

#[expect(clippy::type_complexity)]
#[legacy_memo]
pub fn get_link_fields_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        ClientScalarSelectable<TNetworkProtocol>,
    >,
    CreateSchemaError<TNetworkProtocol>,
> {
    let memo_ref = get_link_fields(db);
    Ok(memo_ref
        .to_owned()?
        .into_iter()
        .map(|link_selectable| {
            (
                (link_selectable.parent_object_entity_name, *LINK_FIELD_NAME),
                link_selectable,
            )
        })
        .collect())
}
