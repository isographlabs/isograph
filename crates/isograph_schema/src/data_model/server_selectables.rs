use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, SchemaServerObjectEntityName, SchemaServerScalarEntityName,
    ServerObjectSelectableName, ServerScalarSelectableName, WithLocation,
};
use isograph_lang_types::{impl_with_target_id, SelectionType, TypeAnnotation, VariableDefinition};

use crate::{NetworkProtocol, SchemaServerObjectSelectableVariant, ServerEntityName};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerScalarSelectableName>,

    pub target_scalar_entity: TypeAnnotation<SchemaServerScalarEntityName>,

    pub parent_object_entity_name: SchemaServerObjectEntityName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

impl_with_target_id!(ServerScalarSelectable<TNetworkProtocol: NetworkProtocol>, ServerEntityName);

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerObjectSelectableName>,

    pub target_object_entity: TypeAnnotation<SchemaServerObjectEntityName>,

    pub object_selectable_variant: SchemaServerObjectSelectableVariant,

    pub parent_object_name: SchemaServerObjectEntityName,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}

// TODO rename
pub type ServerSelectableId = SelectionType<
    (SchemaServerObjectEntityName, ServerScalarSelectableName),
    (SchemaServerObjectEntityName, ServerObjectSelectableName),
>;

pub type ServerSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ServerObjectSelectable<TNetworkProtocol>,
>;
