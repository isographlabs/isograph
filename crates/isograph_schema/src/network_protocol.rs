use std::{collections::BTreeMap, error::Error, fmt::Debug, hash::Hash};

use common_lang_types::{
    JavascriptName, QueryExtraInfo, QueryOperationName, QueryText, ServerObjectEntityName,
    ServerSelectableName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLInputValueDefinition, GraphQLTypeAnnotation};
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::{
    ExposeFieldDirective, MergedSelectionMap, RootOperationName, ServerObjectEntity,
    ServerScalarEntity, ValidatedVariableDefinition, isograph_database::IsographDatabase,
};

pub type ParseTypeSystemOutcome<TNetworkProtocol> = Vec<
    SelectionType<
        WithLocation<ServerScalarEntity<TNetworkProtocol>>,
        ProcessObjectTypeDefinitionOutcome<TNetworkProtocol>,
    >,
>;

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    type SchemaObjectAssociatedData: Debug + PartialEq + Eq + Clone;
    type ParseTypeSystemDocumentsError: Error + PartialEq + Eq + Clone + 'static;

    #[allow(clippy::type_complexity)]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> MemoRef<
        // TODO this should return a Vec<Result<...>>, not a Result<Vec<...>>, probably
        Result<
            (
                ParseTypeSystemOutcome<Self>,
                BTreeMap<ServerObjectEntityName, RootOperationName>,
            ),
            Self::ParseTypeSystemDocumentsError,
        >,
    >;

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
pub struct ProcessObjectTypeDefinitionOutcome<TNetworkProtocol: NetworkProtocol + 'static> {
    pub server_object_entity: WithLocation<ServerObjectEntity<TNetworkProtocol>>,
    pub fields_to_insert: Vec<WithLocation<FieldToInsert>>,
    // TODO this seems sketch
    pub expose_as_fields_to_insert: Vec<ExposeAsFieldToInsert>,
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
pub struct ExposeAsFieldToInsert {
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
