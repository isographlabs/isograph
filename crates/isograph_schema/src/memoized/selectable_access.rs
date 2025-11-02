use std::ops::Deref;

use common_lang_types::{ServerObjectEntityName, ServerSelectableName};
use pico_macros::legacy_memo;

use crate::{
    FieldToInsertToServerSelectableError, IsographDatabase, NetworkProtocol, OwnedServerSelectable,
    field_to_insert_to_server_selectable,
};

/// A vector of all server selectables that are defined in the type system schema
#[legacy_memo]
#[expect(clippy::type_complexity)]
pub fn server_selectables_vec<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> Result<
    // TODO return the SelectableId with each Result, i.e. we should know
    // the parent type and selectable name infallibly
    Vec<(
        ServerSelectableName,
        Result<OwnedServerSelectable<TNetworkProtocol>, FieldToInsertToServerSelectableError>,
    )>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (items, _fetchable_types) = memo_ref.deref().as_ref().map_err(|e| e.clone())?;

    Ok(items
        .iter()
        .flat_map(|selection_type| selection_type.as_ref().as_object())
        .filter(|o| o.server_object_entity.item.name.item == parent_server_object_entity_name)
        .flat_map(|o| {
            o.fields_to_insert.iter().map(|field_to_insert| {
                (
                    field_to_insert.item.name.item,
                    field_to_insert_to_server_selectable(
                        db,
                        parent_server_object_entity_name,
                        field_to_insert,
                    )
                    .map(|x| x.map_scalar(|(scalar, _)| scalar)),
                )
            })
        })
        .collect())
}
