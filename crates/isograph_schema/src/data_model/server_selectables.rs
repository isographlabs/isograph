use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ServerObjectEntityName, ServerObjectSelectableName, ServerScalarEntityName,
    ServerScalarSelectableName, WithLocation,
};
use isograph_lang_types::{
    impl_with_target_id, Description, SelectionType, TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, ServerEntityName, ServerObjectSelectableVariant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<ServerScalarEntityName>,

    pub parent_object_entity_name: ServerObjectEntityName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityName);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<ServerObjectEntityName>,

    pub object_selectable_variant: ServerObjectSelectableVariant,

    pub parent_object_entity_name: ServerObjectEntityName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

// TODO rename
pub type ServerSelectableId = SelectionType<
    (ServerObjectEntityName, ServerScalarSelectableName),
    (ServerObjectEntityName, ServerObjectSelectableName),
>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;
