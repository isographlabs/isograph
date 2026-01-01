use std::fmt::Debug;

use common_lang_types::{
    EntityName, EntityNameAndSelectableName, JavascriptName, SelectableName, WithNoLocation,
};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol, ServerObjectSelectableVariant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerSelectable<TCompilationProfile: CompilationProfile> {
    pub description: Option<WithNoLocation<Description>>,
    pub name: SelectableName,

    pub target_entity_name: TypeAnnotationDeclaration,

    // Hack alert
    // TODO remove this field as follows:
    // - for scalars, the override is because all __typename selectables point
    // to the same entity. But there should actually be a unique typename entity
    // per typename (i.e. per server object entity), which has a JavascriptName
    // that is the string literal of the typename. This should be easy to fix!
    // - For objects, ServerObjectSelectableVariant belongs in the
    // network_protocol_associated_data field. Since it is (presumably) only used
    // by query text generation.
    // TODO move this onto target_platform_associated_data
    pub selection_info: SelectionType<Option<JavascriptName>, ServerObjectSelectableVariant>,

    pub parent_entity_name: EntityName,
    // TODO we shouldn't support default values here
    pub arguments: Vec<VariableDeclaration>,

    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableAssociatedData,
}

impl<TCompilationProfile: CompilationProfile> ServerSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

pub type MemoRefServerSelectable<TCompilationProfile> =
    MemoRef<ServerSelectable<TCompilationProfile>>;
