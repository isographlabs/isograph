use std::fmt::Debug;

use common_lang_types::EntityNameAndSelectableName;
use pico::MemoRef;

use crate::{CompilationProfile, FlattenedDataModelSelectable};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub struct IsInlineFragment(pub bool);

impl From<bool> for IsInlineFragment {
    fn from(value: bool) -> Self {
        IsInlineFragment(value)
    }
}

pub type ServerSelectable<TCompilationProfile> = FlattenedDataModelSelectable<TCompilationProfile>;

impl<TCompilationProfile: CompilationProfile> ServerSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name.item, self.name.item)
    }
}

pub type MemoRefServerSelectable<TCompilationProfile> =
    MemoRef<ServerSelectable<TCompilationProfile>>;
