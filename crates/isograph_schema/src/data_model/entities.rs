use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    JavascriptName, SelectableName, ServerObjectEntityName, ServerScalarEntityName,
    WithEmbeddedLocation, WithLocation, WithSpan,
};
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithSpan<Description>>,
    pub name: WithLocation<ServerScalarEntityName>,
    pub javascript_name: JavascriptName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

pub type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: WithEmbeddedLocation<ServerObjectEntityName>,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<ServerObjectEntityName>,

    pub network_protocol_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

pub type ServerEntity<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarEntity<TNetworkProtocol>,
    &'a ServerObjectEntity<TNetworkProtocol>,
>;

pub type ServerEntityName = SelectionType<ServerScalarEntityName, ServerObjectEntityName>;
