use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName, ServerObjectSelectableName,
    ServerScalarSelectableName, WithLocation,
};
use isograph_lang_types::{
    impl_with_id, impl_with_target_id, SelectionType, ServerEntityId, ServerObjectSelectableId,
    ServerScalarSelectableId, TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, SchemaServerObjectSelectableVariant};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<GraphQLScalarTypeName>,

    pub parent_object_entity_name: IsographObjectTypeName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityId);
impl_with_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerScalarSelectableId);

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<IsographObjectTypeName>,

    pub object_selectable_variant: SchemaServerObjectSelectableVariant,

    pub parent_object_name: IsographObjectTypeName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerObjectSelectable<TNetworkProtocol: NetworkProtocol>, ServerObjectSelectableId);

pub type ServerSelectableId = SelectionType<ServerScalarSelectableId, ServerObjectSelectableId>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;
