use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use common_lang_types::{
    DiagnosticResult, EntityName, QueryExtraInfo, QueryOperationName, QueryText, SelectableName,
    WithLocation, WithNonFatalDiagnostics,
};
use isograph_lang_types::VariableDefinition;

use crate::{
    MemoRefSelectable, MemoRefServerEntity, MergedSelectionMap, RefetchStrategy, RootOperationName,
    ServerEntityName, isograph_database::IsographDatabase,
};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct ParseTypeSystemOutcome<TNetworkProtocol: NetworkProtocol> {
    pub entities: BTreeMap<EntityName, WithLocation<MemoRefServerEntity<TNetworkProtocol>>>,

    pub selectables:
        BTreeMap<(EntityName, SelectableName), WithLocation<MemoRefSelectable<TNetworkProtocol>>>,

    pub client_scalar_refetch_strategies:
        Vec<DiagnosticResult<WithLocation<(EntityName, SelectableName, RefetchStrategy)>>>,
}

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type EntityAssociatedData: Debug + PartialEq + Eq + Clone + Hash;

    // TODO this should return a Vec<Result<...>>, not a Result<Vec<...>>, probably
    #[expect(clippy::type_complexity)]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> &DiagnosticResult<(
        WithNonFatalDiagnostics<ParseTypeSystemOutcome<Self>>,
        // TODO just seems awkward that we return fetchable types
        BTreeMap<EntityName, RootOperationName>,
    )>;

    fn generate_query_text<'a>(
        db: &IsographDatabase<Self>,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDefinition<ServerEntityName>> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText;

    fn generate_link_type(
        db: &IsographDatabase<Self>,
        server_object_entity_name: &EntityName,
    ) -> String;

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
