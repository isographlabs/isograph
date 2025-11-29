use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    JavascriptName, SelectableName, ServerObjectEntityName, ServerScalarEntityName,
    ServerScalarSelectableName,
};
use isograph_lang_types::{DefinitionLocation, Description, SelectionType};
use pico::MemoRef;

use crate::{
    ClientSelectableId, ID_FIELD_NAME, NetworkProtocol, ServerObjectEntityDirectives,
    ServerSelectableId,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: ServerScalarEntityName,
    pub javascript_name: JavascriptName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

pub type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: ServerObjectEntityName,
    /// Some if the object is concrete; None otherwise.
    ///
    /// This is a GraphQL-ism! We should get rid of it.
    pub concrete_type: Option<ServerObjectEntityName>,
    pub server_object_entity_directives: ServerObjectEntityDirectives,
    pub network_protocol_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

impl<TNetworkProtocol: NetworkProtocol> ServerObjectEntity<TNetworkProtocol> {
    pub fn canonical_id_field_name(&self) -> ServerScalarSelectableName {
        self.server_object_entity_directives
            .canonical_id
            .as_ref()
            .map(|canonical_id| canonical_id.field_name.unchecked_conversion())
            .unwrap_or_else(|| *ID_FIELD_NAME)
    }
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

pub type ServerEntityName = SelectionType<ServerScalarEntityName, ServerObjectEntityName>;
