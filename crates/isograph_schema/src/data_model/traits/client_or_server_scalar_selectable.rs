use common_lang_types::{ScalarSelectableName, ServerObjectEntityName};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{DefinitionLocation, Description};

use crate::{ClientScalarSelectable, NetworkProtocol, ServerScalarSelectable};

pub type ScalarSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ClientScalarSelectable<TNetworkProtocol>,
>;

#[impl_for_definition_location]
pub trait ClientOrServerScalarSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> ScalarSelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerScalarSelectable
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ScalarSelectableName {
        self.name.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerScalarSelectable
    for &ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ScalarSelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }
}
