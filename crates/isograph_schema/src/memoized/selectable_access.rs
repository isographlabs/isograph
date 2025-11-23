use common_lang_types::{Diagnostic, SelectableName, ServerObjectEntityName, WithLocation};
use isograph_lang_types::{DefinitionLocation, DefinitionLocationPostfix, SelectionType};
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, FieldToInsertToServerSelectableError,
    IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol, ServerObjectSelectable,
    ServerScalarSelectable, ServerSelectableNamedError, client_selectable_map,
    client_selectable_named, server_selectable_named, server_selectables_vec_for_entity,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
) -> Result<
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
    SelectableNamedError,
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
        (Err(e), Err(_)) => Err(e.clone().into()),
        (Ok(server), Err(_)) => match server.clone() {
            Some(server_selectable) => server_selectable
                .map_err(
                    |e| SelectableNamedError::FieldToInsertToServerSelectableError { error: e },
                )?
                .server_defined()
                .some()
                .ok(),
            None => Ok(None),
        },
        (Err(_), Ok(client)) => match client.clone() {
            Some(client_selectable) => client_selectable.client_defined().some().ok(),
            None => Ok(None),
        },
        (Ok(server), Ok(client)) => match (server, client) {
            (None, None) => Ok(None),
            (None, Some(client_selectable)) => {
                client_selectable.clone().client_defined().some().ok()
            }
            (Some(server_selectable), None) => server_selectable
                .clone()
                .map_err(
                    |e| SelectableNamedError::FieldToInsertToServerSelectableError { error: e },
                )?
                .server_defined()
                .some()
                .ok(),
            (Some(_), Some(_)) => SelectableNamedError::DuplicateDefinitions {
                parent_object_entity_name: parent_server_object_entity_name,
                selectable_name,
            }
            .err(),
        },
    }
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn selectables_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Vec<
        Result<
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
            SelectableNamedError,
        >,
    >,
    SelectableNamedError,
> {
    let mut selectables = server_selectables_vec_for_entity(db, parent_server_object_entity_name)
        .to_owned()
        .map_err(SelectableNamedError::ParseTypeSystemDocumentsError)?
        .into_iter()
        .map(|(_key, value)| {
            let value =
                value.map_err(
                    |e| SelectableNamedError::FieldToInsertToServerSelectableError { error: e },
                )?;
            value.server_defined().ok()
        })
        .collect::<Vec<_>>();

    selectables.extend(
        client_selectable_map(db)
            .as_ref()
            .map_err(|e| e.clone())?
            .iter()
            .filter(|((entity_name, _selectable_name), _value)| {
                *entity_name == parent_server_object_entity_name
            })
            .map(|(_key, value)| {
                let value = value.as_ref().map_err(|e| e.clone())?.clone();
                value.client_defined().ok()
            }),
    );

    selectables.ok()
}

#[derive(Error, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum SelectableNamedError {
    #[error("{0}")]
    ServerSelectableNamedError(#[from] ServerSelectableNamedError),

    #[error("{}", error.for_display())]
    FieldToInsertToServerSelectableError {
        error: WithLocation<FieldToInsertToServerSelectableError>,
    },

    #[error("`{parent_object_entity_name}.{selectable_name}` has been defined multiple times.")]
    DuplicateDefinitions {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: SelectableName,
    },

    #[error("{0}")]
    MemoizedIsoLiteralError(#[from] MemoizedIsoLiteralError),

    #[error("{0}")]
    ParseTypeSystemDocumentsError(Diagnostic),
}
