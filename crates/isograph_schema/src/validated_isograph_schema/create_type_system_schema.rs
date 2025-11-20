use std::collections::HashMap;

use crate::{
    CreateAdditionalFieldsError, ExposeFieldToInsert, FieldToInsert,
    FieldToInsertToServerSelectableError, IsographDatabase, NetworkProtocol,
};
use common_lang_types::{SelectableName, ServerObjectEntityName, WithLocation};
use pico_macros::memo;
use thiserror::Error;

#[memo]
#[expect(clippy::type_complexity)]
pub fn create_type_system_schema_with_server_selectables<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        HashMap<ServerObjectEntityName, Vec<ExposeFieldToInsert>>,
        HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    ),
    CreateSchemaError<TNetworkProtocol>,
> {
    let (items, _fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
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
            item.expose_fields_to_insert.clone(),
        );
    }

    Ok((expose_as_field_queue, field_queue))
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateSchemaError<TNetworkProtocol: NetworkProtocol> {
    #[error("{message}")]
    ParseAndProcessTypeSystemDocument {
        message: TNetworkProtocol::ParseTypeSystemDocumentsError,
    },

    #[error("{}", message)]
    CreateAdditionalFields {
        message: CreateAdditionalFieldsError<TNetworkProtocol>,
    },

    #[error(
        "The Isograph compiler attempted to create a field named \
        `{selectable_name}` on type `{parent_object_entity_name}`, but a field with that name already exists."
    )]
    CompilerCreatedFieldExistsOnType {
        selectable_name: SelectableName,
        parent_object_entity_name: ServerObjectEntityName,
    },

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),
}

impl<TNetworkProtocol: NetworkProtocol> From<CreateAdditionalFieldsError<TNetworkProtocol>>
    for CreateSchemaError<TNetworkProtocol>
{
    fn from(value: CreateAdditionalFieldsError<TNetworkProtocol>) -> Self {
        CreateSchemaError::CreateAdditionalFields { message: value }
    }
}
