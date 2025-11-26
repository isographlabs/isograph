use common_lang_types::{DiagnosticResult, SelectableName, ServerObjectEntityName};
use isograph_lang_types::{DefinitionLocation, DefinitionLocationPostfix, SelectionType};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, IsographDatabase, NetworkProtocol,
    ServerObjectSelectable, ServerScalarSelectable, client_selectable_map, client_selectable_named,
    multiple_selectable_definitions_found_diagnostic, server_selectable_named,
    server_selectables_vec_for_entity,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<
    Option<
        DefinitionLocation<
            SelectionType<
                ServerScalarSelectable<TNetworkProtocol>,
                ServerObjectSelectable<TNetworkProtocol>,
            >,
            SelectionType<
                ClientScalarSelectable<TNetworkProtocol>,
                ClientObjectSelectable<TNetworkProtocol>,
            >,
        >,
    >,
> {
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
        (Ok(server), Err(_)) => match server.note_todo("Do not clone. Use a MemoRef.").clone() {
            Some(server_selectable) => server_selectable?.server_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Err(_), Ok(client)) => match client.note_todo("Do not clone. Use a MemoRef.").clone() {
            Some(client_selectable) => client_selectable.client_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Ok(server), Ok(client)) => match (server, client) {
            (None, None) => Ok(None),
            (None, Some(client_selectable)) => client_selectable
                .clone()
                .note_todo("Do not clone. Use a MemoRef.")
                .client_defined()
                .wrap_some()
                .wrap_ok(),
            (Some(server_selectable), None) => server_selectable
                .clone()
                .note_todo("Do not clone. Use a MemoRef.")?
                .server_defined()
                .wrap_some()
                .wrap_ok(),
            (Some(s), Some(_)) => multiple_selectable_definitions_found_diagnostic(
                parent_server_object_entity_name,
                selectable_name,
                match s.clone_err()? {
                    SelectionType::Scalar(s) => s.name.location,
                    SelectionType::Object(o) => o.name.location,
                },
            )
            .wrap_err(),
        },
    }
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn selectables_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<
    Vec<
        DiagnosticResult<
            DefinitionLocation<
                SelectionType<
                    ServerScalarSelectable<TNetworkProtocol>,
                    ServerObjectSelectable<TNetworkProtocol>,
                >,
                SelectionType<
                    ClientScalarSelectable<TNetworkProtocol>,
                    ClientObjectSelectable<TNetworkProtocol>,
                >,
            >,
        >,
    >,
> {
    let mut selectables = server_selectables_vec_for_entity(db, parent_server_object_entity_name)
        .to_owned()?
        .into_iter()
        .map(|(_key, value)| value?.server_defined().wrap_ok())
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
