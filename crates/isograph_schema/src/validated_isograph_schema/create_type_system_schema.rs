use std::collections::HashMap;

use crate::{
    ExposeFieldToInsert, FieldToInsertToServerSelectableError, IsographDatabase, NetworkProtocol,
};
use common_lang_types::ServerObjectEntityName;
use pico_macros::memo;
use thiserror::Error;

#[memo]
pub fn create_type_system_schema_with_server_selectables<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<ServerObjectEntityName, Vec<ExposeFieldToInsert>>,
    CreateSchemaError<TNetworkProtocol>,
> {
    let (items, _fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| CreateSchemaError::ParseAndProcessTypeSystemDocument { message: e.clone() })?;

    let mut expose_as_field_queue = HashMap::new();

    for item in items.iter().flat_map(|x| x.as_ref().as_object()) {
        expose_as_field_queue.insert(
            item.server_object_entity.item.name,
            item.expose_fields_to_insert.clone(),
        );
    }

    Ok(expose_as_field_queue)
}

#[derive(Error, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum CreateSchemaError<TNetworkProtocol: NetworkProtocol> {
    #[error("{message}")]
    ParseAndProcessTypeSystemDocument {
        message: TNetworkProtocol::ParseTypeSystemDocumentsError,
    },

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),
}
