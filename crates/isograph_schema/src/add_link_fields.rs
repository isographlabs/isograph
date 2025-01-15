use crate::{
    ClientField, ClientFieldVariant, ClientType, FieldType, ProcessTypeDefinitionError,
    ProcessTypeDefinitionResult, UnvalidatedSchema, LINK_FIELD_NAME,
};
use common_lang_types::{Location, ObjectTypeAndFieldName, WithLocation};
use intern::string_key::Intern;

impl UnvalidatedSchema {
    pub fn add_link_fields(&mut self) -> ProcessTypeDefinitionResult<()> {
        for object in &mut self.server_field_data.server_objects {
            let field_name = (*LINK_FIELD_NAME).into();
            let next_client_field_id = self.client_fields.len().into();
            self.client_fields
                .push(ClientType::ClientField(ClientField {
                    description: Some(
                        format!("A store Link for the {} type.", object.name)
                            .intern()
                            .into(),
                    ),
                    id: next_client_field_id,
                    name: field_name,
                    parent_object_id: object.id,
                    variable_definitions: vec![],
                    reader_selection_set: Some(vec![]),
                    variant: ClientFieldVariant::Link,
                    type_and_field: ObjectTypeAndFieldName {
                        field_name,
                        type_name: object.name,
                    },
                    refetch_strategy: None,
                }));

            if object
                .encountered_fields
                .insert(
                    field_name,
                    FieldType::ClientField(ClientType::ClientField(next_client_field_id)),
                )
                .is_some()
            {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::FieldExistsOnType {
                        field_name,
                        parent_type: object.name,
                    },
                    Location::generated(),
                ));
            }
        }
        Ok(())
    }
}
