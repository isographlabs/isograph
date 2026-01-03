use std::fmt::Debug;

use common_lang_types::{EntityName, EntityNameAndSelectableName, SelectableName, WithNoLocation};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol, ServerObjectSelectableVariant, TargetPlatform};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerSelectable<TCompilationProfile: CompilationProfile> {
    pub description: Option<WithNoLocation<Description>>,
    pub name: SelectableName,

    pub target_entity_name: TypeAnnotationDeclaration,

    // TODO move ServerObjectSelectableVariant into target_platform_associated_data.
    // Note that this is hard, because it is used inside of merge_server_object_field (etc) in
    // a way that isn't obviously easily extracted.
    pub selection_info: SelectionType<(), ServerObjectSelectableVariant>,

    pub parent_entity_name: EntityName,
    // TODO we shouldn't support default values here
    pub arguments: Vec<VariableDeclaration>,

    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableAssociatedData,
    pub target_platform_associated_data:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::SelectableAssociatedData,
}

impl<TCompilationProfile: CompilationProfile> ServerSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

pub type MemoRefServerSelectable<TCompilationProfile> =
    MemoRef<ServerSelectable<TCompilationProfile>>;
