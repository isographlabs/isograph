use crate::{ClientFieldVariant, ClientScalarSelectable, NetworkProtocol, Schema, LINK_FIELD_NAME};
use common_lang_types::{Location, ObjectTypeAndFieldName, WithLocation};
use intern::string_key::Intern;
use isograph_lang_types::{DefinitionLocation, SelectionType, WithId};

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, ProcessTypeDefinitionResult,
};

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn add_link_fields(&mut self) -> ProcessTypeDefinitionResult<()> {
        let mut selectables_to_process = vec![];
        for WithId {
            id: parent_object_entity_name,
            item: object,
        } in &mut self.server_entity_data.server_object_entities_and_ids_mut()
        {
            let field_name = *LINK_FIELD_NAME;
            self.client_scalar_selectables.insert(
                (parent_object_entity_name, field_name),
                ClientScalarSelectable {
                    description: Some(
                        format!("A store Link for the {} type.", object.name)
                            .intern()
                            .into(),
                    ),
                    name: field_name,
                    parent_object_entity_name,
                    variable_definitions: vec![],
                    reader_selection_set: vec![],
                    variant: ClientFieldVariant::Link,
                    type_and_field: ObjectTypeAndFieldName {
                        field_name: field_name.into(),
                        type_name: object.name,
                    },
                    refetch_strategy: None,
                    output_format: std::marker::PhantomData,
                },
            );

            selectables_to_process.push((parent_object_entity_name, field_name, object.name));
        }

        // Awkward: we can only get one mutable reference to self at once, so within the server_object_entities_and_ids_mut()
        // loop, we can't also update self.server_entity_data.server_object_entity_available_selectables!
        //
        // This is temporary: when everything moves to pico, this will be easier!
        for (parent_object_entity_name, field_name, object_name) in selectables_to_process {
            if self
                .server_entity_data
                .server_object_entity_extra_info
                .entry(parent_object_entity_name)
                .or_default()
                .selectables
                .insert(
                    field_name.into(),
                    DefinitionLocation::Client(SelectionType::Scalar((
                        parent_object_entity_name,
                        field_name,
                    ))),
                )
                .is_some()
            {
                return Err(WithLocation::new(
                    CreateAdditionalFieldsError::CompilerCreatedFieldExistsOnType {
                        field_name: field_name.into(),
                        parent_type: object_name,
                    },
                    Location::generated(),
                ));
            }
        }
        Ok(())
    }
}
