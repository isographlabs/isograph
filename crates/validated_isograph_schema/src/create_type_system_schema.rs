use std::{collections::HashMap, ops::Deref};

use common_lang_types::{ServerObjectEntityName, WithLocation};
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::SelectionType;
use isograph_schema::{
    CreateAdditionalFieldsError, ExposeFieldToInsert, FieldToInsert,
    FieldToInsertToServerSelectableError, ID_FIELD_NAME, IsographDatabase, NetworkProtocol,
    ScalarSelectionAndNonNullType, Schema, ServerObjectEntityExtraInfo, ServerObjectSelectable,
    UnprocessedClientFieldItem, UnprocessedClientPointerItem, create_new_exposed_field,
    field_to_insert_to_server_selectable,
};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::set_and_validate_id_field::set_and_validate_id_field;

#[legacy_memo]
#[expect(clippy::type_complexity)]
pub fn create_type_system_schema<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        // TODO combine these into one hashmap?
        HashMap<ServerObjectEntityName, Vec<ExposeFieldToInsert>>,
        HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    ),
    CreateSchemaError<TNetworkProtocol>,
> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (items, _fetchable_types) = memo_ref
        .deref()
        .as_ref()
        .map_err(|e| CreateSchemaError::ParseAndProcessTypeSystemDocument { message: e.clone() })?;

    let mut field_queue = HashMap::new();
    let mut expose_as_field_queue = HashMap::new();

    for item in items.iter().flat_map(|x| x.as_ref().as_object()) {
        field_queue.insert(
            item.server_object_entity.item.name.item,
            item.fields_to_insert.clone(),
        );

        expose_as_field_queue.insert(
            item.server_object_entity.item.name.item,
            item.expose_as_fields_to_insert.clone(),
        );
    }

    Ok((expose_as_field_queue, field_queue))
}

/// Create a schema from the type system document, i.e. avoid parsing any
/// iso literals. It *does* set server fields. Parsing iso literals is done in a future step.
///
/// This is sufficient for some queries, like answering "Where is a server field defined."
#[legacy_memo]
#[expect(clippy::type_complexity)]
pub(crate) fn create_type_system_schema_with_server_selectables<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        Schema<TNetworkProtocol>,
        Vec<SelectionType<UnprocessedClientFieldItem, UnprocessedClientPointerItem>>,
    ),
    CreateSchemaError<TNetworkProtocol>,
> {
    let (expose_as_field_queue, field_queue) = create_type_system_schema(db).to_owned()?;

    let mut unvalidated_isograph_schema = Schema::new();

    process_field_queue(
        db,
        &mut unvalidated_isograph_schema,
        field_queue,
        &db.get_isograph_config().options,
    )?;

    // Step one: we can create client selectables. However, we must create all
    // client selectables before being able to create their selection sets, because
    // selection sets refer to client selectables. We hold onto these selection sets
    // (both reader selection sets and refetch selection sets) in the unprocess_items
    // vec, then process it later.
    let mut unprocessed_items = vec![];

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let (unprocessed_scalar_item, client_scalar_selectable, payload_object_entity_name) =
                create_new_exposed_field(
                    db,
                    &unvalidated_isograph_schema,
                    expose_as_field,
                    parent_object_entity_name,
                )?;

            let client_scalar_selectable_name = client_scalar_selectable.name.item;
            let parent_object_entity_name = client_scalar_selectable.parent_object_entity_name;

            unvalidated_isograph_schema
                .client_scalar_selectables
                .insert(
                    (
                        client_scalar_selectable.parent_object_entity_name,
                        client_scalar_selectable_name,
                    ),
                    client_scalar_selectable,
                );

            unvalidated_isograph_schema.insert_client_field_on_object(
                parent_object_entity_name,
                client_scalar_selectable_name,
                payload_object_entity_name,
            )?;

            unprocessed_items.push(SelectionType::Scalar(unprocessed_scalar_item));
        }
    }

    Ok((unvalidated_isograph_schema, unprocessed_items))
}

/// Now that we have processed all objects and scalars, we can process fields (i.e.
/// selectables), as we have the knowledge of whether the field points to a scalar
/// or object.
///
/// For each field:
/// - insert it into to the parent object's encountered_fields
/// - append it to schema.server_fields
/// - if it is an id field, modify the parent object
fn process_field_queue<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    field_queue: HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    options: &CompilerConfigOptions,
) -> Result<(), CreateSchemaError<TNetworkProtocol>> {
    for selectable in process_field_queue_inner(db, field_queue) {
        match selectable? {
            SelectionType::Scalar((server_scalar_selectable, inner_non_null_named_type)) => {
                let server_scalar_selectable_name = server_scalar_selectable.name.item;
                let parent_object_entity_name = server_scalar_selectable.parent_object_entity_name;

                schema.insert_server_scalar_selectable(server_scalar_selectable)?;

                let ServerObjectEntityExtraInfo { id_field, .. } = schema
                    .server_entity_data
                    .entry(parent_object_entity_name)
                    .or_default();
                // TODO do not do this here, this is a GraphQL-ism
                if server_scalar_selectable_name == *ID_FIELD_NAME {
                    set_and_validate_id_field::<TNetworkProtocol>(
                        id_field,
                        server_scalar_selectable_name,
                        parent_object_entity_name,
                        options,
                        inner_non_null_named_type.as_ref(),
                    )?;
                }
            }
            SelectionType::Object(server_object_selectable) => {
                schema.insert_server_object_selectable(server_object_selectable)?;
            }
        }
    }

    Ok(())
}

fn process_field_queue_inner<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    field_queue: HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
) -> impl Iterator<
    Item = Result<
        SelectionType<
            ScalarSelectionAndNonNullType<TNetworkProtocol>,
            ServerObjectSelectable<TNetworkProtocol>,
        >,
        CreateSchemaError<TNetworkProtocol>,
    >,
> {
    field_queue.into_iter().flat_map(
        move |(parent_object_entity_name, field_definitions_to_insert)| {
            field_definitions_to_insert
                .into_iter()
                .map(move |server_field_to_insert| {
                    field_to_insert_to_server_selectable(
                        db,
                        parent_object_entity_name,
                        &server_field_to_insert,
                    )
                    .map_err(|e| e.into())
                })
        },
    )
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateSchemaError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error("{message}")]
    ParseAndProcessTypeSystemDocument {
        message: TNetworkProtocol::ParseTypeSystemDocumentsError,
    },

    #[error("{}", message)]
    CreateAdditionalFields {
        message: CreateAdditionalFieldsError<TNetworkProtocol>,
    },

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<CreateAdditionalFieldsError<TNetworkProtocol>> for CreateSchemaError<TNetworkProtocol>
{
    fn from(value: CreateAdditionalFieldsError<TNetworkProtocol>) -> Self {
        CreateSchemaError::CreateAdditionalFields { message: value }
    }
}
