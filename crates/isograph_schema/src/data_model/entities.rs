use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    JavascriptName, SelectableName, ServerObjectEntityName, ServerScalarEntityName,
};
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    // TODO make this a WithLocation or just an Option<Description>
    pub description: Option<Description>,
    pub name: ServerScalarEntityName,
    pub javascript_name: JavascriptName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

pub type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: ServerObjectEntityName,
    /// Some if the object is concrete; None otherwise.
    ///
    /// This is a GraphQL-ism! We should get rid of it.
    pub concrete_type: Option<ServerObjectEntityName>,

    pub network_protocol_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

pub type ServerEntity<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarEntity<TNetworkProtocol>,
    &'a ServerObjectEntity<TNetworkProtocol>,
>;

pub type OwnedServerEntity<TNetworkProtocol> =
    SelectionType<ServerScalarEntity<TNetworkProtocol>, ServerObjectEntity<TNetworkProtocol>>;

pub type ServerEntityName = SelectionType<ServerScalarEntityName, ServerObjectEntityName>;
