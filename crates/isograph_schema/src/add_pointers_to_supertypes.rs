use common_lang_types::ClientPointerFieldName;
use intern::string_key::Intern;

use crate::{
    generate_refetch_field_strategy, id_arguments, id_selection, id_top_level_arguments,
    ClientField, ClientPointer, ImperativelyLoadedFieldVariant, ObjectTypeAndFieldName,
    ProcessTypeDefinitionResult, RefetchStrategy, RequiresRefinement, TypeRefinementMap,
    UnvalidatedSchema, NODE_FIELD_NAME, REFETCH_FIELD_NAME,
};

impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the pointers for each subtype (e.g. User implements Node)
    /// to supertype (e.g. creating Node.asUser).
    pub fn add_pointers_to_supertypes(
        &mut self,
        subtype_to_supertype_map: &TypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        let query_id = self.query_id();

        for (subtype_id, supertype_ids) in subtype_to_supertype_map {
            let subtype: &crate::SchemaObject = self.server_field_data.object(*subtype_id);

            let field_name: ClientPointerFieldName = format!("as{}", subtype.name).intern().into();

            let next_client_field_id = self.client_fields.len().into();
            let next_client_pointer_id = self.client_pointers.len().into();

            let client_field = ClientField {
                description: Some(
                    format!("A client poiter for the {} type.", subtype.name)
                        .intern()
                        .into(),
                ),
                id: next_client_field_id,
                name: field_name.into(),
                parent_object_id: subtype.id,
                reader_selection_set: None,
                refetch_strategy: subtype.id_field.map(|_| {
                    // Assume that if we have an id field, this implements Node

                    RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                        vec![id_selection()],
                        query_id,
                        format!("refetch__{}", subtype.name).intern().into(),
                        *NODE_FIELD_NAME,
                        id_top_level_arguments(),
                        None,
                        RequiresRefinement::Yes(subtype.name),
                        None,
                        None,
                    ))
                }),
                type_and_field: ObjectTypeAndFieldName {
                    type_name: subtype.name,
                    field_name: field_name.into(),
                },
                variable_definitions: vec![],
                unwraps: vec![],
                // This will be replaced with ClientFieldVariant::ClientPointer
                variant: crate::ClientFieldVariant::ImperativelyLoadedField(
                    ImperativelyLoadedFieldVariant {
                        client_field_scalar_selection_name: *REFETCH_FIELD_NAME,
                        top_level_schema_field_name: *NODE_FIELD_NAME,
                        top_level_schema_field_arguments: id_arguments(),
                        top_level_schema_field_concrete_type: None,
                        primary_field_info: None,

                        root_object_id: query_id,
                    },
                ),
            };

            let client_pointer = ClientPointer {
                id: next_client_pointer_id,
            };

            self.client_fields.push(client_field);
            self.client_pointers.push(client_pointer);

            for supertype_id in supertype_ids {
                let supertype = self.server_field_data.object_mut(*supertype_id);

                if supertype
                    .encountered_fields
                    .insert(
                        field_name.into(),
                        crate::FieldType::ClientField(next_client_field_id),
                    )
                    .is_some()
                {
                    panic!("TODO implement ProcessTypeDefinitionError::FieldExistsOnSupertype");
                    // return Err(WithLocation::new(
                    //     ProcessTypeDefinitionError::FieldExistsOnSubtype {
                    //         field_name: *supertype_field_name,
                    //         parent_type: subtype.name,
                    //     },
                    //     Location::generated(),
                    // ));
                }
            }
        }
        Ok(())
    }
}
