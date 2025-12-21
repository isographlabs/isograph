use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{EntityName, EntityNameAndSelectableName, JavascriptName, SelectableName};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotation, VariableDefinition, impl_with_target_id,
};
use pico::MemoRef;

use crate::{NetworkProtocol, ServerEntityName, ServerObjectSelectableVariant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,

    pub target_entity_name: TypeAnnotation,
    /// Normally, we look up the JavaScript type to use by going through the
    /// target scalar entity. However, there are
    pub javascript_type_override: Option<JavascriptName>,

    pub parent_entity_name: EntityName,
    pub arguments: Vec<VariableDefinition>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarSelectable<TNetworkProtocol> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityName);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,

    pub target_entity_name: TypeAnnotation,

    pub object_selectable_variant: ServerObjectSelectableVariant,

    pub parent_entity_name: EntityName,
    pub arguments: Vec<VariableDefinition>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> ServerObjectSelectable<TNetworkProtocol> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

pub type MemoRefServerSelectable<TNetworkProtocol> = SelectionType<
    MemoRef<ServerScalarSelectable<TNetworkProtocol>>,
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
>;
