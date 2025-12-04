use common_lang_types::{DiagnosticResult, SelectableName, ServerObjectEntityName};
use isograph_lang_types::{DefinitionLocationPostfix, SelectionType};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, MemoRefSelectable, NetworkProtocol, client_selectable_map,
    client_selectable_named, multiple_selectable_definitions_found_diagnostic,
    server_selectable_named, server_selectables_map_for_entity,
};

#[memo]
pub fn selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRefSelectable<TNetworkProtocol>>> {
    // we don't obviously have a better way to do this besides checking whether this
    // a server selectable and also checking whether it is a client selectable, and
    // error'ing if we have multiple definitions.
    let server_selectable = server_selectable_named(
        db,
        parent_server_object_entity_name,
        selectable_name.unchecked_conversion(),
    )
    .as_ref();

    let client_selectable = client_selectable_named(
        db,
        parent_server_object_entity_name,
        selectable_name.unchecked_conversion(),
    )
    .as_ref();

    // case 1: both are error -> return error
    // case 2: one is error -> assume that is an unrelated error and return the other one
    // case 3: both are ok -> check that there aren't duplicate definitions, and return remaining one or None

    match (server_selectable, client_selectable) {
        (Err(e), Err(_)) => e.clone().wrap_err(),
        (Ok(server), Err(_)) => match *server.note_todo("Do not clone. Use a MemoRef.") {
            Some(server_selectable) => server_selectable.server_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Err(_), Ok(client)) => match *client.note_todo("Do not clone. Use a MemoRef.") {
            Some(client_selectable) => client_selectable.client_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Ok(server), Ok(client)) => match (server, client) {
            (None, None) => Ok(None),
            (None, Some(client_selectable)) => (*client_selectable)
                .note_todo("Do not clone. Use a MemoRef.")
                .client_defined()
                .wrap_some()
                .wrap_ok(),
            (Some(server_selectable), None) => (*server_selectable)
                .note_todo("Do not clone. Use a MemoRef.")
                .server_defined()
                .wrap_some()
                .wrap_ok(),
            (Some(s), Some(_)) => multiple_selectable_definitions_found_diagnostic(
                parent_server_object_entity_name,
                selectable_name,
                match s {
                    SelectionType::Scalar(s) => s.lookup(db).name.location,
                    SelectionType::Object(o) => o.lookup(db).name.location,
                },
            )
            .wrap_err(),
        },
    }
}

#[memo]
pub fn selectables_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<Vec<DiagnosticResult<MemoRefSelectable<TNetworkProtocol>>>> {
    let mut selectables = server_selectables_map_for_entity(db, parent_server_object_entity_name)
        .to_owned()?.into_values().map(|value| {
            value
                .server_defined()
                .note_todo("Do not wrap in a Result here when client selectables aren't wrapped in results")
                .wrap_ok()
        })
        .collect::<Vec<_>>();

    selectables.extend(
        client_selectable_map(db)
            .clone_err()?
            .iter()
            .filter(|((entity_name, _selectable_name), _value)| {
                *entity_name == parent_server_object_entity_name
            })
            .map(|(_key, value)| {
                let value = value.clone().note_todo("Do not clone. Use a MemoRef.")?;
                value.client_defined().wrap_ok()
            }),
    );

    selectables.wrap_ok()
}
