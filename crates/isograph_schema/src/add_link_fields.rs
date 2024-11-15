use common_lang_types::{Location, WithLocation};
use intern::string_key::Intern;

use crate::{
    ClientField, ClientFieldVariant, ClientType, FieldType, ObjectTypeAndFieldName,
    ProcessTypeDefinitionError, ProcessTypeDefinitionResult, UnvalidatedSchema,
};

impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the fields defined on it (e.g. Node.MyComponent)
    /// to subtypes (e.g. creating User.MyComponent).
    ///
    /// We do not transfer server fields (because that makes no sense in GraphQL, but does
    /// it make sense otherwise??) and refetch fields (which are already defined on all valid
    /// types.)
    ///
    /// TODO confirm we don't do this for unions...
    pub fn add_link_fields(&mut self) -> ProcessTypeDefinitionResult<()> {
        for object in &mut self.server_field_data.server_objects {
            let field_name = "__link".intern().into();
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
                        type_name: "ID".intern().into(),
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
