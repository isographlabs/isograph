use std::collections::btree_map::Entry;

use common_lang_types::ObjectTypeAndFieldName;
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, SelectionType, ServerObjectId, ServerStrongIdFieldId,
};
use isograph_schema::{
    generate_refetch_field_strategy, id_arguments, id_selection, id_top_level_arguments,
    ClientField, ClientFieldVariant, ImperativelyLoadedFieldVariant, OutputFormat, RefetchStrategy,
    RequiresRefinement, SchemaObject, UnvalidatedClientField, UnvalidatedClientPointer,
    UnvalidatedSchema, NODE_FIELD_NAME, REFETCH_FIELD_NAME,
};

use crate::batch_compile::BatchCompileError;

pub fn add_refetch_fields_to_objects<TOutputFormat: OutputFormat>(
    schema: &mut UnvalidatedSchema<TOutputFormat>,
) -> Result<(), BatchCompileError> {
    let query_id = schema.query_id();

    for object in schema.server_field_data.server_objects.iter_mut() {
        if let Some(id_field) = object.id_field {
            add_refetch_field_to_object(object, &mut schema.client_types, query_id, id_field)?;
        }
    }
    Ok(())
}

fn add_refetch_field_to_object<TOutputFormat: OutputFormat>(
    object: &mut SchemaObject<TOutputFormat>,
    client_fields: &mut Vec<
        SelectionType<
            UnvalidatedClientField<TOutputFormat>,
            UnvalidatedClientPointer<TOutputFormat>,
        >,
    >,
    query_id: ServerObjectId,
    _id_field: ServerStrongIdFieldId,
) -> Result<(), BatchCompileError> {
    match object
        .encountered_fields
        .entry((*REFETCH_FIELD_NAME).into())
    {
        Entry::Occupied(_) => Err(BatchCompileError::DuplicateRefetchField),
        Entry::Vacant(vacant_entry) => {
            let next_client_field_id = client_fields.len().into();

            vacant_entry.insert(DefinitionLocation::Client(SelectionType::Scalar(
                next_client_field_id,
            )));

            client_fields.push(SelectionType::Scalar(ClientField {
                description: Some(
                    format!("A refetch field for the {} type.", object.name)
                        .intern()
                        .into(),
                ),
                name: *REFETCH_FIELD_NAME,
                id: next_client_field_id,
                reader_selection_set: vec![],
                variant: ClientFieldVariant::ImperativelyLoadedField(
                    ImperativelyLoadedFieldVariant {
                        client_field_scalar_selection_name: *REFETCH_FIELD_NAME,
                        top_level_schema_field_name: *NODE_FIELD_NAME,
                        top_level_schema_field_arguments: id_arguments(),
                        top_level_schema_field_concrete_type: None,
                        primary_field_info: None,

                        root_object_id: query_id,
                    },
                ),
                variable_definitions: vec![],
                type_and_field: ObjectTypeAndFieldName {
                    type_name: object.name,
                    field_name: "__refetch".intern().into(),
                },
                parent_object_id: object.id,
                refetch_strategy: Some(RefetchStrategy::UseRefetchField(
                    generate_refetch_field_strategy(
                        vec![id_selection()],
                        query_id,
                        format!("refetch__{}", object.name).intern().into(),
                        *NODE_FIELD_NAME,
                        id_top_level_arguments(),
                        None,
                        RequiresRefinement::Yes(object.name),
                        None,
                        None,
                    ),
                )),
                output_format: std::marker::PhantomData,
            }));
            Ok(())
        }
    }
}
