use std::collections::HashMap;

use crate::{ExposeFieldToInsert, IsographDatabase, NetworkProtocol};
use common_lang_types::{Diagnostic, ServerObjectEntityName};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

#[memo]
pub fn create_type_system_schema_with_server_selectables<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<HashMap<ServerObjectEntityName, Vec<ExposeFieldToInsert>>, Diagnostic> {
    let (items, _fetchable_types) =
        TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    let mut expose_as_field_queue = HashMap::new();

    for item in items.iter().flat_map(|x| x.as_ref().as_object()) {
        expose_as_field_queue.insert(
            item.server_object_entity.item.name,
            item.expose_fields_to_insert
                .clone()
                .note_todo("Do not clone. Use a MemoRef."),
        );
    }

    Ok(expose_as_field_queue)
}
