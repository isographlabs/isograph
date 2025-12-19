use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{EntityName, JavascriptName, SelectableName};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotation, VariableDefinition, impl_with_target_id,
};
use pico::MemoRef;

use crate::{NetworkProtocol, SelectableTrait, ServerEntityName, ServerObjectSelectableVariant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,

    pub target_scalar_entity: TypeAnnotation<EntityName>,
    /// Normally, we look up the JavaScript type to use by going through the
    /// target scalar entity. However, there are
    pub javascript_type_override: Option<JavascriptName>,

    pub parent_object_entity_name: EntityName,
    pub arguments: Vec<VariableDefinition<ServerEntityName>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ServerScalarSelectable<TNetworkProtocol>
{
    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityName);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,

    pub target_object_entity: TypeAnnotation<EntityName>,

    pub object_selectable_variant: ServerObjectSelectableVariant,

    pub parent_object_entity_name: EntityName,
    pub arguments: Vec<VariableDefinition<ServerEntityName>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }
}

pub type MemoRefServerSelectable<TNetworkProtocol> = SelectionType<
    MemoRef<ServerScalarSelectable<TNetworkProtocol>>,
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
>;
