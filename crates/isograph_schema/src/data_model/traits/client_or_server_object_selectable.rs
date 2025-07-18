use common_lang_types::{
    ClientObjectSelectableName, DescriptionValue, ObjectSelectableName, ServerObjectEntityName,
    ServerObjectSelectableName,
};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{DefinitionLocation, TypeAnnotation};

use crate::{ClientObjectSelectable, NetworkProtocol, ServerObjectSelectable};

pub type ObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerObjectSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type ObjectSelectableId = DefinitionLocation<
    (ServerObjectEntityName, ServerObjectSelectableName),
    (ServerObjectEntityName, ClientObjectSelectableName),
>;

#[impl_for_definition_location]
pub trait ClientOrServerObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> ObjectSelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName>;
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ObjectSelectableName {
        self.name.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ObjectSelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity.clone()
    }
}
