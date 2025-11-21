use common_lang_types::{SelectableName, ServerObjectEntityName};
use isograph_lang_types::{DefinitionLocation, SelectionType};
use pico_macros::memo;
use thiserror::Error;

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, FieldToInsertToServerSelectableError,
    IsographDatabase, NetworkProtocol, ServerObjectSelectable, ServerScalarSelectable,
    ServerSelectableNamedError, client_selectable_named, server_selectable_named,
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
    SelectableNamedError<TNetworkProtocol>,
> {
    // we don't obviously have a better way to do this besides checking whether this
    // a server selectable and also checking whether it is a client selectable, and
    // error'ing if we have multiple definitions.
    // let server_selectable = server_selectable_named(
    //     db,
    //     parent_server_object_entity_name,
    //     selectable_name.unchecked_conversion(),
    // )
    // .as_ref();

    let client_selectable = client_selectable_named(
        db,
        parent_server_object_entity_name,
        selectable_name.unchecked_conversion(),
    )
    .as_ref();

    // case 1: both are error -> return error
    // case 2: one is error -> assume that is an unrelated error and return the other one
    // case 3: both are ok -> check that there aren't duplicate definitions, and return remaining one or None

    match (client_selectable) {
        (Err(e)) => Err(SelectableNamedError::Foo),
        // (Ok(server)) => match server.clone() {
        //     Some(server_selectable) => Ok(Some(DefinitionLocation::Server(server_selectable?))),
        //     None => Ok(None),
        // },
        (Ok(client)) => match client.clone() {
            Some(client_selectable) => Ok(Some(DefinitionLocation::Client(client_selectable))),
            None => Ok(None),
        },
        // (Ok(server), Ok(client)) => match (server, client) {
        //     (None, None) => Ok(None),
        //     (None, Some(client_selectable)) => {
        //         Ok(Some(DefinitionLocation::Client(client_selectable.clone())))
        //     }
        //     (Some(server_selectable), None) => {
        //         Ok(Some(DefinitionLocation::Server(server_selectable.clone()?)))
        //     }
        //     (Some(_), Some(_)) => Err(SelectableNamedError::DuplicateDefinitions {
        //         parent_object_entity_name: parent_server_object_entity_name,
        //         selectable_name,
        //     }),
        // },
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum SelectableNamedError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ServerSelectableNamedError(#[from] ServerSelectableNamedError<TNetworkProtocol>),

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),

    #[error("`{parent_object_entity_name}.{selectable_name}` has been defined multiple times.")]
    DuplicateDefinitions {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: SelectableName,
    },

    #[error("asdf")]
    Foo,
}
