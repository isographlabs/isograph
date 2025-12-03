use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use common_lang_types::{
    ClientScalarSelectableName, Diagnostic, DiagnosticResult, JavascriptName, QueryExtraInfo,
    QueryOperationName, QueryText, ServerObjectEntityName, ServerSelectableName,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLInputValueDefinition, GraphQLTypeAnnotation};
use isograph_lang_types::{Description, SelectionType};

use crate::{
    ClientScalarSelectable, ExposeFieldDirective, MemoRefServerEntity, MergedSelectionMap,
    RefetchStrategy, RootOperationName, ServerObjectEntity, ServerObjectSelectable,
    ServerScalarEntity, ServerScalarSelectable, ValidatedVariableDefinition,
    isograph_database::IsographDatabase,
};

type UnvalidatedRefetchStrategy = RefetchStrategy<(), ()>;

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct ParseTypeSystemOutcome<TNetworkProtocol: NetworkProtocol> {
    pub entities:
        BTreeMap<UnvalidatedTypeName, Vec<WithLocation<MemoRefServerEntity<TNetworkProtocol>>>>,

    // TODO these should all be MemoRef
    pub server_object_selectables:
        Vec<DiagnosticResult<WithLocation<ServerObjectSelectable<TNetworkProtocol>>>>,
    pub server_scalar_selectables:
        Vec<DiagnosticResult<WithLocation<ServerScalarSelectable<TNetworkProtocol>>>>,

    // expose_as fields...
    pub client_scalar_selectables:
        Vec<DiagnosticResult<WithLocation<ClientScalarSelectable<TNetworkProtocol>>>>,
    pub client_scalar_refetch_strategies: Vec<
        DiagnosticResult<
            WithLocation<(
                ServerObjectEntityName,
                ClientScalarSelectableName,
                UnvalidatedRefetchStrategy,
            )>,
        >,
    >,

    // This should contain errors that
    // 1. do not prevent us from proceeding, and
    // 2. are not associated with a specific entity/selectable, but
    // 3. should stop us from generating artifacts.
    pub errors: Vec<Diagnostic>,
}

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type SchemaObjectAssociatedData: Debug + PartialEq + Eq + Clone + Hash;

    // TODO this should return a Vec<Result<...>>, not a Result<Vec<...>>, probably
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> &DiagnosticResult<(
        ParseTypeSystemOutcome<Self>,
        // TODO just seems awkward that we return fetchable types
        BTreeMap<ServerObjectEntityName, RootOperationName>,
    )>;

    fn generate_query_text<'a>(
        db: &IsographDatabase<Self>,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText;

    fn generate_link_type(
        db: &IsographDatabase<Self>,
        server_object_entity_name: &ServerObjectEntityName,
    ) -> String;

    // TODO: include `QueryText` to incrementally adopt persisted documents
    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: ServerObjectEntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo;
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ProcessObjectTypeDefinitionOutcome<TNetworkProtocol: NetworkProtocol> {
    pub server_object_entity: WithLocation<ServerObjectEntity<TNetworkProtocol>>,
    pub fields_to_insert: Vec<WithLocation<FieldToInsert>>,
    // TODO this seems sketch
    pub expose_fields_to_insert: Vec<ExposeFieldToInsert>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct FieldToInsert {
    pub description: Option<WithSpan<Description>>,
    pub name: WithLocation<ServerSelectableName>,
    pub graphql_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    /// An override type for the typename field. Normally, the JavaScript type is
    /// acquired by going through graphql_type.inner(), but there is no separate
    /// 'UserTypename' type in GraphQL. So we do this instead. This is horrible
    /// data modeling, and should be fixed.
    pub javascript_type_override: Option<JavascriptName>,
    pub arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,

    // TODO we can probably restructure things to make this less awkward.
    // As in, we should not return GraphQLFieldDefinitions to the isograph side,
    // which is GraphQL-agnostic, and instead pass field definitions. These field
    // definitions should have an associated_data: TNetworkProtocol::FieldAssociatedData
    // or the like, which should carry this info.
    //
    // Then, that should be consumed by NetworkProtocol::generate_query_text, and also
    // somehow by generate_merged_selection_set. (Is a merged selection set something
    // that the network protocol should care about?? I don't think so, but how else
    // do we add the __typename and link selections?)
    pub is_inline_fragment: bool,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ExposeFieldToInsert {
    pub expose_field_directive: ExposeFieldDirective,
    // e.g. Query or Mutation
    pub parent_object_name: ServerObjectEntityName,
    pub description: Option<Description>,
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Pretty,
    Compact,
}
