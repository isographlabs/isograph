use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, ServerObjectSelectableName, ServerScalarSelectableName, ServerSelectableName,
    WithLocation,
};
use isograph_lang_types::{
    SelectionType, ServerEntityId, ServerObjectId, ServerObjectSelectableId, ServerScalarId,
    ServerScalarSelectableId, TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, SchemaServerObjectSelectableVariant};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<ServerScalarId>,

    pub parent_type_id: ServerObjectId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<ServerObjectId>,

    pub object_selectable_variant: SchemaServerObjectSelectableVariant,

    pub parent_type_id: ServerObjectId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

pub type ServerSelectableId = SelectionType<ServerScalarSelectableId, ServerObjectSelectableId>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;

pub trait ServerScalarOrObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> WithLocation<ServerSelectableName>;
    fn target_entity_id(&self) -> TypeAnnotation<ServerEntityId>;
    fn parent_type_id(&self) -> ServerObjectId;
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

    fn parent_type_id(&self) -> ServerObjectId {
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

    fn parent_type_id(&self) -> ServerObjectId {
        self.parent_type_id
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityId>>] {
        &self.arguments
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectSelectable
    for ServerSelectable<'_, TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        match self {
            SelectionType::Scalar(s) => s.description(),
            SelectionType::Object(o) => o.description(),
        }
    }

    fn name(&self) -> WithLocation<ServerSelectableName> {
        match self {
            SelectionType::Scalar(s) => s.name(),
            SelectionType::Object(o) => o.name(),
        }
    }

    fn target_entity_id(&self) -> TypeAnnotation<ServerEntityId> {
        match self {
            SelectionType::Scalar(s) => s.target_entity_id(),
            SelectionType::Object(o) => o.target_entity_id(),
        }
    }

    fn parent_type_id(&self) -> ServerObjectId {
        match self {
            SelectionType::Scalar(s) => s.parent_type_id(),
            SelectionType::Object(o) => o.parent_type_id(),
        }
    }

    fn arguments(&self) -> &[WithLocation<VariableDefinition<ServerEntityId>>] {
        match self {
            SelectionType::Scalar(s) => s.arguments(),
            SelectionType::Object(o) => o.arguments(),
        }
    }
}
