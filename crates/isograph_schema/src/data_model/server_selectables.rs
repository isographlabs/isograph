use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    JavascriptName, SelectableName, ServerObjectEntityName, ServerObjectSelectableName,
    ServerScalarEntityName, ServerScalarSelectableName, WithLocation,
};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotation, VariableDefinition, impl_with_target_id,
};

use crate::{NetworkProtocol, SelectableTrait, ServerEntityName, ServerObjectSelectableVariant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<ServerScalarEntityName>,
    /// Normally, we look up the JavaScript type to use by going through the
    /// target scalar entity. However, there are
    pub javascript_type_override: Option<JavascriptName>,

    pub parent_object_entity_name: ServerObjectEntityName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ServerScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> WithLocation<SelectableName> {
        self.name.map(|x| x.into())
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        self.arguments.iter().map(|x| &x.item).collect()
    }
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

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> WithLocation<SelectableName> {
        self.name.map(|x| x.into())
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        self.arguments.iter().map(|x| &x.item).collect()
    }
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

pub type OwnedServerSelectable<TNetworkProtocol> = SelectionType<
    ServerScalarSelectable<TNetworkProtocol>,
    ServerObjectSelectable<TNetworkProtocol>,
>;
