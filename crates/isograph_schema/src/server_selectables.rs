use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, ServerObjectSelectableName, ServerScalarSelectableName, ServerSelectableName,
    WithLocation,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    impl_with_id, SelectionType, ServerEntityId, ServerObjectEntityId, ServerObjectSelectableId,
    ServerScalarEntityId, ServerScalarSelectableId, TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, SchemaServerObjectSelectableVariant};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<ServerScalarEntityId>,

    pub parent_type_id: ServerObjectEntityId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerScalarSelectableId);

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<ServerObjectEntityId>,

    pub object_selectable_variant: SchemaServerObjectSelectableVariant,

    pub parent_type_id: ServerObjectEntityId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerObjectSelectable<TNetworkProtocol: NetworkProtocol>, ServerObjectSelectableId);

pub type ServerSelectableId = SelectionType<ServerScalarSelectableId, ServerObjectSelectableId>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;

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
