use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, ServerObjectSelectableName, ServerScalarSelectableName, WithLocation,
};
use isograph_lang_types::{
    impl_with_id, impl_with_target_id, SelectionType, ServerEntityId, ServerObjectEntityId,
    ServerObjectSelectableId, ServerScalarEntityId, ServerScalarSelectableId, TypeAnnotation,
    VariableDefinition,
};

use crate::{NetworkProtocol, SchemaServerObjectSelectableVariant};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<ServerScalarEntityId>,

    pub parent_object_entity_id: ServerObjectEntityId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityId);
impl_with_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerScalarSelectableId);

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<ServerObjectEntityId>,

    pub object_selectable_variant: SchemaServerObjectSelectableVariant,

    pub parent_object_entity_id: ServerObjectEntityId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerObjectSelectable<TNetworkProtocol: NetworkProtocol>, ServerObjectSelectableId);
impl_with_target_id!(ServerObjectSelectable<TNetworkProtocol: NetworkProtocol>, ServerObjectEntityId);

pub type ServerSelectableId = SelectionType<ServerScalarSelectableId, ServerObjectSelectableId>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;
