use std::collections::btree_map::Entry;

use common_lang_types::ObjectTypeAndFieldName;
use intern::string_key::Intern;
use isograph_lang_types::{DefinitionLocation, SelectionType, WithId};
use isograph_schema::{
    generate_refetch_field_strategy, id_arguments, id_selection, id_top_level_arguments,
    ClientFieldVariant, ClientScalarSelectable, ImperativelyLoadedFieldVariant, NetworkProtocol,
    RefetchStrategy, RequiresRefinement, Schema, UnprocessedClientFieldItem, UnprocessedItem,
    NODE_FIELD_NAME, REFETCH_FIELD_NAME,
};

use crate::batch_compile::BatchCompileError;

pub fn add_refetch_fields_to_objects<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
) -> Result<Vec<UnprocessedItem>, BatchCompileError> {
    let query_id = schema.query_id();

    let mut errors = vec![];
    let mut results = vec![];

    let id_type_id = schema.server_entity_data.id_type_id;

    let items = schema
        .server_entity_data
        .server_object_entities_and_ids()
        .flat_map(
            |WithId {
                 id: object_entity_id,
                 item: object,
             }| {
                if object.id_field.is_some() {
                    Some((object_entity_id, object.name))
                } else {
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    for (object_entity_id, object_name) in items {
        let schema: &mut Schema<TNetworkProtocol> = schema;
        let result = match schema
            .server_entity_data
            .server_object_entity_available_selectables
            .entry(object_entity_id)
            .or_default()
            .entry((*REFETCH_FIELD_NAME).into())
        {
            Entry::Occupied(_) => Err(BatchCompileError::DuplicateRefetchField),
            Entry::Vacant(vacant_entry) => {
                let next_client_field_id = schema.client_scalar_selectables.len().into();

                vacant_entry.insert(DefinitionLocation::Client(SelectionType::Scalar(
                    next_client_field_id,
                )));

                schema
                    .client_scalar_selectables
                    .push(ClientScalarSelectable {
                        description: Some(
                            format!("A refetch field for the {} type.", object_name)
                                .intern()
                                .into(),
                        ),
                        name: *REFETCH_FIELD_NAME,
                        reader_selection_set: vec![],
                        variant: ClientFieldVariant::ImperativelyLoadedField(
                            ImperativelyLoadedFieldVariant {
                                client_field_scalar_selection_name: *REFETCH_FIELD_NAME,
                                top_level_schema_field_name: *NODE_FIELD_NAME,
                                top_level_schema_field_arguments: id_arguments(id_type_id),
                                top_level_schema_field_concrete_type: None,
                                primary_field_info: None,

                                root_object_entity_id: query_id,
                            },
                        ),
                        variable_definitions: vec![],
                        type_and_field: ObjectTypeAndFieldName {
                            type_name: object_name,
                            field_name: "__refetch".intern().into(),
                        },
                        parent_object_entity_id: object_entity_id,
                        refetch_strategy: None,
                        output_format: std::marker::PhantomData,
                    });

                let refetch_strategy =
                    RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                        vec![id_selection()],
                        query_id,
                        format!("refetch__{}", object_name).intern().into(),
                        *NODE_FIELD_NAME,
                        id_top_level_arguments(),
                        None,
                        RequiresRefinement::Yes(object_name),
                        None,
                        None,
                    ));

                Ok(UnprocessedClientFieldItem {
                    client_field_id: next_client_field_id,
                    reader_selection_set: vec![],
                    refetch_strategy: Some(refetch_strategy),
                })
            }
        };
        match result {
            Ok(item) => results.push(SelectionType::Scalar(item)),
            Err(e) => {
                errors.push(Box::new(e) as Box<dyn std::error::Error>);
            }
        }
    }

    if errors.is_empty() {
        Ok(results)
    } else {
        Err(BatchCompileError::MultipleErrors { messages: errors })
    }
}
