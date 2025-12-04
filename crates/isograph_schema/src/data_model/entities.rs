use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{EntityName, JavascriptName, SelectableName};
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};
use pico::MemoRef;

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

// TODO Scalar and Object entities should be a single struct, with all the extra info
// held in a network_protocol_associated_data field. (Some of that data, like the javascript_name,
// is independent of the network protocol, but actually relates to the RuntimeLanguage, but
// we do not (yet) have that as a generic.)
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: EntityName,
    pub javascript_name: JavascriptName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

pub type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: EntityName,
    /// Some if the object is concrete; None otherwise.
    ///
    /// This is a GraphQL-ism! We should get rid of it.
    pub concrete_type: Option<EntityName>,

    pub network_protocol_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

pub type ServerEntity<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarEntity<TNetworkProtocol>,
    &'a ServerObjectEntity<TNetworkProtocol>,
>;

pub type OwnedServerEntity<TNetworkProtocol> =
    SelectionType<ServerScalarEntity<TNetworkProtocol>, ServerObjectEntity<TNetworkProtocol>>;

pub type MemoRefServerEntity<TNetworkProtocol> = SelectionType<
    MemoRef<ServerScalarEntity<TNetworkProtocol>>,
    MemoRef<ServerObjectEntity<TNetworkProtocol>>,
>;

pub type ServerEntityName = SelectionType<EntityName, EntityName>;
