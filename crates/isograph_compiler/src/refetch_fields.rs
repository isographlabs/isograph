use std::collections::hash_map::Entry;

use common_lang_types::{Location, Span, WithLocation, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    IsographSelectionVariant, ScalarFieldSelection, Selection, ServerFieldSelection, ServerObjectId,
};
use isograph_schema::{
    id_arguments, ClientField, ClientFieldVariant, FieldDefinitionLocation,
    ImperativelyLoadedFieldVariant, ObjectTypeAndFieldName, SchemaObject, UnvalidatedClientField,
    UnvalidatedSchema, NODE_FIELD_NAME, REFETCH_FIELD_NAME,
};

use crate::batch_compile::BatchCompileError;

pub fn add_refetch_fields_to_objects(
    schema: &mut UnvalidatedSchema,
) -> Result<(), BatchCompileError> {
    let query_id = schema.query_id();

    'objects: for object in schema.server_field_data.server_objects.iter_mut() {
        if object.id_field.is_none() {
            continue 'objects;
        }

        if let Some(value) =
            add_refetch_field_to_object(object, &mut schema.client_fields, query_id)
        {
            return value;
        }
    }
    Ok(())
}

fn add_refetch_field_to_object(
    object: &mut SchemaObject,
    client_fields: &mut Vec<UnvalidatedClientField>,
    query_id: ServerObjectId,
) -> Option<Result<(), BatchCompileError>> {
    match object
        .encountered_fields
        .entry((*REFETCH_FIELD_NAME).into())
    {
        Entry::Occupied(_) => return Some(Err(BatchCompileError::DuplicateRefetchField)),
        Entry::Vacant(vacant_entry) => {
            let next_client_field_id = client_fields.len().into();

            vacant_entry.insert(FieldDefinitionLocation::Client(next_client_field_id));

            let id_field_selection = WithSpan::new(
                Selection::ServerField(ServerFieldSelection::ScalarField(ScalarFieldSelection {
                    name: WithLocation::new("id".intern().into(), Location::generated()),
                    reader_alias: None,
                    normalization_alias: None,
                    associated_data: IsographSelectionVariant::Regular,
                    unwraps: vec![],
                    arguments: vec![],
                    directives: vec![],
                })),
                Span::todo_generated(),
            );

            client_fields.push(ClientField {
                description: Some(
                    format!("A refetch field for the {} type.", object.name)
                        .intern()
                        .into(),
                ),
                name: (*REFETCH_FIELD_NAME).into(),
                id: next_client_field_id,
                reader_selection_set: vec![id_field_selection],
                unwraps: vec![],
                variant: ClientFieldVariant::ImperativelyLoadedField(
                    ImperativelyLoadedFieldVariant {
                        client_field_scalar_selection_name: *REFETCH_FIELD_NAME,
                        top_level_schema_field_name: *NODE_FIELD_NAME,
                        top_level_schema_field_arguments: id_arguments(),

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
            });
        }
    }
    None
}
