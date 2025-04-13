use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    SelectionType, ServerEntityId, ServerObjectEntityId, TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, ServerObjectSelectable, ServerScalarSelectable};

#[impl_for_selection_type]
pub trait ServerScalarOrObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> WithLocation<ServerSelectableName>;
    fn target_entity_id(&self) -> TypeAnnotation<ServerEntityId>;
    fn parent_type_id(&self) -> ServerObjectEntityId;
    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityId>>];
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectSelectable
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> WithLocation<ServerSelectableName> {
        self.name.map(|x| x.into())
    }

    fn target_entity_id(&self) -> TypeAnnotation<ServerEntityId> {
        self.target_object_entity
            .clone()
            .map(&mut SelectionType::Object)
    }

    fn parent_type_id(&self) -> ServerObjectEntityId {
        self.parent_type_id
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityId>>] {
        &self.arguments
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectSelectable
    for ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> WithLocation<ServerSelectableName> {
        self.name.map(|x| x.into())
    }

    fn target_entity_id(&self) -> TypeAnnotation<ServerEntityId> {
        self.target_scalar_entity
            .clone()
            .map(&mut SelectionType::Scalar)
    }

    fn parent_type_id(&self) -> ServerObjectEntityId {
        self.parent_type_id
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityId>>] {
        &self.arguments
    }
}
