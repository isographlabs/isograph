use common_lang_types::{EntityName, SelectableName};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{Description, SelectionType, TypeAnnotation, VariableDefinition};

use crate::{NetworkProtocol, ServerEntityName, ServerObjectSelectable, ServerScalarSelectable};

#[impl_for_selection_type]
pub trait ServerScalarOrObjectSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> SelectableName;
    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName>;
    fn parent_type_name(&self) -> EntityName;
    fn arguments(&self) -> &[VariableDefinition<ServerEntityName>];
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectSelectable
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName> {
        self.target_object_entity
            .clone()
            .map(&mut SelectionType::Object)
    }

    fn parent_type_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.arguments
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectSelectable
    for ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName> {
        self.target_scalar_entity
            .clone()
            .map(&mut SelectionType::Scalar)
    }

    fn parent_type_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.arguments
    }
}
