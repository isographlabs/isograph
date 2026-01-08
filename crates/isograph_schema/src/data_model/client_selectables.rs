use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{EntityName, EntityNameAndSelectableName, SelectableName, WithNoLocation};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use pico::MemoRef;

use crate::{ClientFieldVariant, CompilationProfile, UserWrittenClientPointerInfo};

// TODO rename
pub type ClientSelectableId =
    SelectionType<(EntityName, SelectableName), (EntityName, SelectableName)>;

pub type ClientSelectable<'a, TCompilationProfile> = SelectionType<
    &'a ClientScalarSelectable<TCompilationProfile>,
    &'a ClientObjectSelectable<TCompilationProfile>,
>;

pub type MemoRefClientSelectable<TCompilationProfile> = SelectionType<
    MemoRef<ClientScalarSelectable<TCompilationProfile>>,
    MemoRef<ClientObjectSelectable<TCompilationProfile>>,
>;

/// The struct formally known as a client field, and declared with the field keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientScalarSelectable<TCompilationProfile: CompilationProfile> {
    pub name: SelectableName,
    pub parent_entity_name: EntityName,
    pub description: Option<WithNoLocation<Description>>,

    pub arguments: Vec<VariableDeclaration>,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub phantom_data: PhantomData<TCompilationProfile>,
}

impl<TCompilationProfile: CompilationProfile> ClientScalarSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

/// The struct formally known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientObjectSelectable<TCompilationProfile: CompilationProfile> {
    pub name: SelectableName,
    pub parent_entity_name: EntityName,
    pub description: Option<WithNoLocation<Description>>,

    pub arguments: Vec<VariableDeclaration>,
    pub target_entity: TypeAnnotationDeclaration,

    pub phantom_data: PhantomData<TCompilationProfile>,
    pub info: UserWrittenClientPointerInfo,
}

impl<TCompilationProfile: CompilationProfile> ClientObjectSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}
