use std::fmt::Debug;

use common_lang_types::{EntityName, EntityNameAndSelectableName, SelectableName, WithNoLocation};
use isograph_lang_types::{Description, TypeAnnotationDeclaration, VariableDeclaration};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol, TargetPlatform};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub struct IsInlineFragment(pub bool);

impl From<bool> for IsInlineFragment {
    fn from(value: bool) -> Self {
        IsInlineFragment(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerSelectable<TCompilationProfile: CompilationProfile> {
    pub description: Option<WithNoLocation<Description>>,
    pub name: SelectableName,

    pub target_entity: TypeAnnotationDeclaration,

    // TODO this is obviously a GraphQL-ism! But it's used in a bunch of places, so it's
    // not really easy to move it to TargetPlatform. However, we know it at parse time,
    // because only asConcreteType fields are inline fragments.
    pub is_inline_fragment: IsInlineFragment,

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
