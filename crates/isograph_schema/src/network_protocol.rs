use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use common_lang_types::{
    DiagnosticResult, EntityName, QueryExtraInfo, QueryOperationName, QueryText, SelectableName,
    WithLocation,
};
use isograph_lang_types::VariableDeclaration;

use crate::{
    CompilationProfile, MemoRefClientSelectable, MergedSelectionMap, RefetchStrategy,
    WrappedMergedSelectionMap, isograph_database::IsographDatabase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapMergedSelectionMapResult {
    pub root_entity: EntityName,
    pub merged_selection_map: WrappedMergedSelectionMap,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct DeprecatedParseTypeSystemOutcome<TCompilationProfile: CompilationProfile> {
    pub selectables: BTreeMap<
        (EntityName, SelectableName),
        WithLocation<MemoRefClientSelectable<TCompilationProfile>>,
    >,

    pub client_scalar_refetch_strategies:
        Vec<DiagnosticResult<WithLocation<(EntityName, SelectableName, RefetchStrategy)>>>,
}

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type EntityAssociatedData: Debug + PartialEq + Eq + Clone + Hash + Ord + PartialOrd;
    type SelectableAssociatedData: Debug + PartialEq + Eq + Clone + Hash + Ord + PartialOrd;

    fn generate_query_text<'a, TCompilationProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDatabase<TCompilationProfile>,
        root_entity: EntityName,
        query_name: QueryOperationName,
        selection_map: &WrappedMergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
        format: Format,
    ) -> QueryText;

    fn wrap_merged_selection_map<TCompilationProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDatabase<TCompilationProfile>,
        root_entity: EntityName,
        merged_selection_map: MergedSelectionMap,
    ) -> DiagnosticResult<WrapMergedSelectionMapResult>;

    /// Returns the entity that should be used as the root for queries targeting
    /// the given entity. For GraphQL with the Node interface, this returns the
    /// Query root when wrapping is needed; otherwise returns the entity itself.
    fn get_query_root_entity<TCompilationProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDatabase<TCompilationProfile>,
        entity_name: EntityName,
    ) -> DiagnosticResult<EntityName>;

    // TODO: include `QueryText` to incrementally adopt persisted documents
    fn generate_query_extra_info(
        query_name: QueryOperationName,
        root_entity: EntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo;
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Pretty,
    Compact,
}
