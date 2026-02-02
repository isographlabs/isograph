use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ConstExportName, Diagnostic, EntityName, EntityNameAndSelectableName, RelativePathToSourceFile,
    SelectableName, WithNoLocation,
};
use isograph_lang_types::{
    ClientScalarSelectableDirectiveSet, Description, SelectionType, TypeAnnotationDeclaration,
    VariableDeclaration,
};
use pico::MemoRef;

use crate::{CompilationProfile, FieldMapItem, WrappedSelectionMapSelection};

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

/// The struct formerly known as a client field, and declared with the field keyword
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

/// The struct formerly known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClientObjectSelectable<TCompilationProfile: CompilationProfile> {
    pub name: SelectableName,
    pub parent_entity_name: EntityName,
    pub description: Option<WithNoLocation<Description>>,

    pub arguments: Vec<VariableDeclaration>,
    pub target_entity: TypeAnnotationDeclaration,

    pub phantom_data: PhantomData<TCompilationProfile>,
    pub variant: IsoLiteralExportInfo,
}

impl<TCompilationProfile: CompilationProfile> ClientObjectSelectable<TCompilationProfile> {
    pub fn entity_name_and_selectable_name(&self) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName::new(self.parent_entity_name, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub selectable_name: SelectableName,

    // Mutation or Query or whatnot. Awkward! A GraphQL-ism!
    pub root_object_entity_name: EntityName,
    pub subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
    pub field_map: Vec<FieldMapItem>,
    /// The arguments we must pass to the top level schema field, e.g. id: ID!
    /// for node(id: $id). These are already encoded in the subfields_or_inline_fragments,
    /// but we nonetheless need to put them into the query definition, and we need
    /// the variable's type, not just the variable.
    pub top_level_schema_field_arguments: Vec<VariableDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserWrittenClientTypeInfo {
    pub info: IsoLiteralExportInfo,
    pub client_scalar_selectable_directive_set:
        Result<ClientScalarSelectableDirectiveSet, Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsoLiteralExportInfo {
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClientFieldVariant {
    UserWritten(UserWrittenClientTypeInfo),
    ImperativelyLoadedField(ImperativelyLoadedFieldVariant),
    Link,
}

impl ClientFieldVariant {
    pub fn unwrap_user_written_variant(self) -> UserWrittenClientTypeInfo {
        if let ClientFieldVariant::UserWritten(u) = self {
            u
        } else {
            panic!(
                "Unexpected non-user-written variant. \
                This is indicative of a bug in Isograph."
            )
        }
    }
}
