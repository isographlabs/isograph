use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{EntityName, SelectableName};
use isograph_lang_types::{Description, SelectionType, TypeAnnotation, VariableDefinition};
use pico::MemoRef;

use crate::{ClientFieldVariant, NetworkProtocol, ServerEntityName, UserWrittenClientPointerInfo};

// TODO rename
pub type ClientSelectableId =
    SelectionType<(EntityName, SelectableName), (EntityName, SelectableName)>;

pub type ClientSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ClientScalarSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type MemoRefClientSelectable<TNetworkProtocol> = SelectionType<
    MemoRef<ClientScalarSelectable<TNetworkProtocol>>,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
>;

/// The struct formally known as a client field, and declared with the field keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions: Vec<VariableDefinition<ServerEntityName>>,

    pub parent_object_entity_name: EntityName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

/// The struct formally known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<Description>,
    pub name: SelectableName,
    pub target_object_entity_name: TypeAnnotation<EntityName>,

    pub variable_definitions: Vec<VariableDefinition<ServerEntityName>>,

    pub parent_object_entity_name: EntityName,

    pub network_protocol: PhantomData<TNetworkProtocol>,
    pub info: UserWrittenClientPointerInfo,
}
