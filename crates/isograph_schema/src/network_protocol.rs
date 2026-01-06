use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use common_lang_types::{
    DiagnosticResult, EntityName, QueryExtraInfo, QueryOperationName, QueryText, SelectableName,
    WithLocation,
};
use isograph_lang_types::VariableDeclaration;

use crate::{
    CompilationProfile, MemoRefSelectable, MemoRefServerEntity, MergedSelectionMap,
    RefetchStrategy, RootOperationName, isograph_database::IsographDatabase,
};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct DeprecatedParseTypeSystemOutcome<TCompilationProfile: CompilationProfile> {
    pub entities: BTreeMap<EntityName, WithLocation<MemoRefServerEntity<TCompilationProfile>>>,

    pub selectables: BTreeMap<
        (EntityName, SelectableName),
        WithLocation<MemoRefSelectable<TCompilationProfile>>,
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
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText;

    // TODO: include `QueryText` to incrementally adopt persisted documents
    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: EntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo;
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Pretty,
    Compact,
}
