use common_lang_types::{EntityName, JavascriptName};
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::NetworkProtocol;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub struct IsConcrete(pub bool);

// TODO Scalar and Object entities should be a single struct, with all the extra info
// held in a network_protocol_associated_data field. (Some of that data, like the javascript_name,
// is independent of the network protocol, but actually relates to the RuntimeLanguage, but
// we do not (yet) have that as a generic.)
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: EntityName,
    pub selection_info: SelectionType<JavascriptName, IsConcrete>,
    pub network_protocol_associated_data: TNetworkProtocol::EntityAssociatedData,
}

pub type MemoRefServerEntity<TNetworkProtocol> = MemoRef<ServerEntity<TNetworkProtocol>>;
