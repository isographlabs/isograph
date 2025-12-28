use common_lang_types::{EntityName, JavascriptName};
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::NetworkProtocol;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub struct IsConcrete(pub bool);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: EntityName,
    // TODO this is obviously a hack
    pub selection_info: SelectionType<JavascriptName, IsConcrete>,
    pub network_protocol_associated_data: TNetworkProtocol::EntityAssociatedData,
}

pub type MemoRefServerEntity<TNetworkProtocol> = MemoRef<ServerEntity<TNetworkProtocol>>;
