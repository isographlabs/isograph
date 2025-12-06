use std::collections::BTreeMap;

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, SelectableName};
use isograph_lang_types::SelectionType;
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone as _, Postfix};

use crate::{
    ID_ENTITY_NAME, ID_FIELD_NAME, IsographDatabase, MemoRefServerSelectable, NetworkProtocol,
    ServerObjectSelectable, ServerScalarSelectable, entity_definition_location,
    server_scalar_entity_named,
};

#[memo]
/// This just drops the location (but not internal locations...) and filters out client fields
pub fn server_selectables_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<
    BTreeMap<(EntityName, SelectableName), MemoRefServerSelectable<TNetworkProtocol>>,
> {
    let (outcome, _fetchable_types) =
        TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .selectables
        .iter()
        .filter_map(|(key, value)| value.item.as_server().map(|server| (*key, server)))
        .collect::<BTreeMap<_, _>>()
        .wrap_ok()
}

#[memo]
pub fn server_selectables_map_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
) -> DiagnosticResult<BTreeMap<SelectableName, MemoRefServerSelectable<TNetworkProtocol>>> {
    let map = server_selectables_map(db).clone_err()?;

    map.iter()
        .filter_map(|(key, value)| {
            if key.0 == parent_server_object_entity_name {
                Some((key.1.unchecked_conversion(), *value))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>()
        .wrap_ok()
}

#[memo]
pub fn server_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
    server_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRefServerSelectable<TNetworkProtocol>>> {
    server_selectables_map_for_entity(db, parent_server_object_entity_name)
        .clone_err()?
        .get(&server_selectable_name)
        .cloned()
        .wrap_ok()
}

#[memo]
pub fn server_id_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
) -> DiagnosticResult<Option<MemoRef<ServerScalarSelectable<TNetworkProtocol>>>> {
    let selectable = server_selectable_named(db, parent_server_object_entity_name, *ID_FIELD_NAME)
        .clone_err()?;

    let selectable = match selectable {
        Some(s) => s,
        None => return Ok(None),
    };

    // TODO check if it is a client field...
    let memo_ref = match selectable {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(_) => {
            let selectable_name = *ID_FIELD_NAME;
            return Diagnostic::new(
                format!(
                    "Expected `{parent_server_object_entity_name}.{selectable_name}` \
                    to be a scalar, but it was an object."
                ),
                entity_definition_location(db, parent_server_object_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten(),
            )
            .wrap_err();
        }
    };

    let selectable = memo_ref.lookup(db);

    let target_scalar_entity_name = selectable.target_scalar_entity.inner();
    let target_scalar_entity = server_scalar_entity_named(db, *target_scalar_entity_name)
        .clone_err()?
        .as_ref()
        // It must exist
        .ok_or_else(|| {
            let id_field_name = *ID_FIELD_NAME;
            Diagnostic::new(
                // TODO: it doesn't seem like this error is actually suppresable (here).
                format!(
                    "The `{id_field_name}` field on \
                    `{target_scalar_entity_name}` must have type `ID!`.\n\
                    This error can be suppressed using the \
                    \"on_invalid_id_type\" config parameter."
                ),
                // TODO use the location of the selectable
                entity_definition_location(db, *target_scalar_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten(),
            )
        })?;

    let options = &db.get_isograph_config().options;

    // And must have the right inner type
    if target_scalar_entity
        .lookup(db)
        .name
        .note_todo("Compare with *target_scalar_entity_name here")
        != *ID_ENTITY_NAME
    {
        options.on_invalid_id_type.on_failure(|| {
            let strong_field_name = *ID_FIELD_NAME;
            Diagnostic::new(
                format!(
                    "The `{strong_field_name}` field on \
                    `{parent_server_object_entity_name}` must have type `ID!`.\n\
                    This error can be suppressed using the \
                    \"on_invalid_id_type\" config parameter."
                ),
                entity_definition_location(db, parent_server_object_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten(),
            )
        })?;
    }

    // TODO disallow [ID] etc, ID, etc.

    memo_ref.dereference().wrap_some().wrap_ok()
}

#[memo]
pub fn server_object_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
    server_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ServerObjectSelectable<TNetworkProtocol>>>> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .clone_err()?;

    match item {
        Some(item) => match item.as_ref().as_object() {
            Some(obj) => (*obj).wrap_some().wrap_ok(),
            None => Diagnostic::new(
                format!(
                    "Expected `{parent_server_object_entity_name}.{server_selectable_name}`\
                        to be an object, but it was a scalar."
                ),
                entity_definition_location(db, parent_server_object_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten(),
            )
            .wrap_err(),
        },
        None => Ok(None),
    }
}

#[memo]
pub fn server_scalar_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityName,
    server_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ServerScalarSelectable<TNetworkProtocol>>>> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .clone_err()?;

    match item {
        Some(item) => match item.as_ref().as_scalar() {
            Some(scalar) => (*scalar).wrap_some().wrap_ok(),
            None => Diagnostic::new(
                format!(
                    "Expected `{parent_server_object_entity_name}.{server_selectable_name}` \
                        to be a scalar, but it was an object."
                ),
                entity_definition_location(db, parent_server_object_entity_name)
                    .as_ref()
                    .ok()
                    .cloned()
                    .flatten(),
            )
            .wrap_err(),
        },
        None => Ok(None),
    }
}
