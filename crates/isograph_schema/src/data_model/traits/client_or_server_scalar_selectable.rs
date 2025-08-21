use common_lang_types::{SelectableName, ServerObjectEntityName};
use isograph_lang_types::{DefinitionLocation, Description};

use crate::{ClientScalarSelectable, NetworkProtocol, SelectableTrait, ServerScalarSelectable};

pub type ScalarSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ClientScalarSelectable<TNetworkProtocol>,
>;

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for &ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }
}
