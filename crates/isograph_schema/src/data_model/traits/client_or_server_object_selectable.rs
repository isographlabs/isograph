use common_lang_types::{DescriptionValue, ObjectSelectableName};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{ServerObjectEntityId, TypeAnnotation};

use crate::{ClientObjectSelectable, NetworkProtocol, ServerObjectSelectable};

#[impl_for_definition_location]
pub trait ClientOrServerObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> ObjectSelectableName;
    fn parent_object_entity_id(&self) -> ServerObjectEntityId;
    fn target_object_entity_id(&self) -> TypeAnnotation<ServerObjectEntityId>;
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

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_object_entity_id
    }

    fn target_object_entity_id(&self) -> TypeAnnotation<ServerObjectEntityId> {
        self.target_object_entity.clone()
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

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_type_id
    }

    fn target_object_entity_id(&self) -> TypeAnnotation<ServerObjectEntityId> {
        self.target_object_entity.clone()
    }
}
