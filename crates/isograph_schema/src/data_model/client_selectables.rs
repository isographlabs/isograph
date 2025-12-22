use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{EntityName, EntityNameAndSelectableName, SelectableName, WithNoLocation};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use pico::MemoRef;

use crate::{ClientFieldVariant, NetworkProtocol, UserWrittenClientPointerInfo};

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
    pub description: Option<WithNoLocation<Description>>,
    pub name: SelectableName,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions: Vec<VariableDeclaration>,

    pub parent_entity_name: EntityName,
    pub network_protocol: PhantomData<TNetworkProtocol>,
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarSelectable<TNetworkProtocol> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

/// The struct formally known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithNoLocation<Description>>,
    pub name: SelectableName,
    pub target_entity_name: TypeAnnotationDeclaration,

    pub variable_definitions: Vec<VariableDeclaration>,

    pub parent_entity_name: EntityName,

    pub network_protocol: PhantomData<TNetworkProtocol>,
    pub info: UserWrittenClientPointerInfo,
}

impl<TNetworkProtocol: NetworkProtocol> ClientObjectSelectable<TNetworkProtocol> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}
