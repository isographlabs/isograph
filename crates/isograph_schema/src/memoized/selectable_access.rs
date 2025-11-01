use std::ops::Deref;

use common_lang_types::ServerObjectEntityName;
use isograph_lang_types::SelectionType;
use pico_macros::legacy_memo;

use crate::{
    FieldToInsertToServerSelectableError, IsographDatabase, NetworkProtocol, OwnedServerSelectable,
    field_to_insert_to_server_selectable,
};

#[legacy_memo]
pub fn server_selectables_vec<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Vec<Result<OwnedServerSelectable<TNetworkProtocol>, FieldToInsertToServerSelectableError>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (items, _fetchable_types) = memo_ref.deref().as_ref().map_err(|e| e.clone())?;

    Ok(items
        .iter()
        .flat_map(|x| match x {
            SelectionType::Object(o) => {
                let mut fields = vec![];

                for field_to_insert in o.fields_to_insert.iter() {
                    let item = field_to_insert_to_server_selectable(
                        db,
                        parent_server_object_entity_name,
                        field_to_insert,
                    )
                    .map(|x| x.map_scalar(|(scalar, _)| scalar));
                    fields.push(item);
                }
                if let Some(_expose_as_field_to_insert) = o.expose_as_fields_to_insert.iter().next()
                {
                    unimplemented!("expose as fields to insert")
                }

                fields
            }
            SelectionType::Scalar(_s) => {
                unimplemented!("scalar selectable")
            }
        })
        .collect())
}
