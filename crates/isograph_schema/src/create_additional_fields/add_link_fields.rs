use crate::{ClientFieldVariant, ClientScalarSelectable, LINK_FIELD_NAME, NetworkProtocol, Schema};
use common_lang_types::{Location, ParentObjectEntityNameAndSelectableName, WithLocation};
use intern::string_key::Intern;
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, ProcessTypeDefinitionResult,
};

impl<TNetworkProtocol: NetworkProtocol + 'static> Schema<TNetworkProtocol> {
    pub fn add_link_fields(&mut self) -> ProcessTypeDefinitionResult<(), TNetworkProtocol> {
        let mut selectables_to_process = vec![];
        for object in &mut self.server_entity_data.server_object_entities_mut() {
            let field_name = *LINK_FIELD_NAME;
            let parent_object_entity_name = object.name;
            self.client_scalar_selectables.insert(
                (parent_object_entity_name.item, field_name),
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
                },
            );

            selectables_to_process.push((parent_object_entity_name, field_name, object.name));
        }

        // Awkward: we can only get one mutable reference to self at once, so within the server_object_entities_and_ids_mut()
        // loop, we can't also update self.server_entity_data.server_object_entity_available_selectables!
        //
        // This is temporary: when everything moves to pico, this will be easier!
        for (parent_object_entity_name, field_name, object_entity_name) in selectables_to_process {
            if self
                .server_entity_data
                .server_object_entity_extra_info
                .entry(parent_object_entity_name.item)
                .or_default()
                .selectables
                .insert(
                    field_name.into(),
                    DefinitionLocation::Client(SelectionType::Scalar((
                        parent_object_entity_name.item,
                        field_name,
                    ))),
                )
                .is_some()
            {
                return Err(
                    CreateAdditionalFieldsError::CompilerCreatedFieldExistsOnType {
                        selectable_name: field_name.into(),
                        parent_object_entity_name: object_entity_name.item,
                    },
                );
            }
        }
        Ok(())
    }
}
