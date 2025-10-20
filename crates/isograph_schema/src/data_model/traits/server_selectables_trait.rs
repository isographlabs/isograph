use common_lang_types::{ServerObjectEntityName, ServerSelectableName, WithLocation};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{Description, SelectionType, TypeAnnotation, VariableDefinition};

use crate::{NetworkProtocol, ServerEntityName, ServerObjectSelectable, ServerScalarSelectable};

#[impl_for_selection_type]
pub trait ServerScalarOrObjectSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> WithLocation<ServerSelectableName>;
    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName>;
    fn parent_type_name(&self) -> ServerObjectEntityName;
    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityName>>];
}

impl<TNetworkProtocol: NetworkProtocol + 'static> ServerScalarOrObjectSelectable
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> WithLocation<ServerSelectableName> {
        self.name.map(|x| x.into())
    }

    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName> {
        self.target_object_entity
            .clone()
            .map(&mut SelectionType::Object)
    }

    fn parent_type_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityName>>] {
        &self.arguments
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> ServerScalarOrObjectSelectable
    for ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> WithLocation<ServerSelectableName> {
        self.name.map(|x| x.into())
    }

    fn target_entity_name(&self) -> TypeAnnotation<ServerEntityName> {
        self.target_scalar_entity
            .clone()
            .map(&mut SelectionType::Scalar)
    }

    fn parent_type_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityName>>] {
        &self.arguments
    }
}
