use std::{error::Error, fmt::Debug, hash::Hash};

use common_lang_types::{
    DescriptionValue, Location, QueryOperationName, QueryText, ServerObjectEntityName,
    ServerSelectableName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLInputValueDefinition, GraphQLTypeAnnotation, RootOperationKind};
use pico::Database;

use crate::{
    ExposeFieldDirective, MergedSelectionMap, RootOperationName, Schema, ServerObjectEntity,
    ServerScalarEntity, ValidatedVariableDefinition,
};

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // We're using this type to constrain the types of sources that we accept. i.e.
    // in theory, you can have a TNetworkProtocol with a different Source associated
    // type, but for now, we get a source + set of extensions, and have to restrict
    // TNetworkProtocol accordingly. Perhaps the config can have a generic, and
    // thus we can thread this further back, but that is not yet implemented.
    type Sources;

    type SchemaObjectAssociatedData: Debug;

    #[allow(clippy::type_complexity)]
    fn parse_and_process_type_system_documents(
        db: &Database,
        sources: &Self::Sources,
    ) -> Result<ProcessTypeSystemDocumentOutcome<Self>, Box<dyn Error>>;

    fn generate_query_text<'a>(
        query_name: QueryOperationName,
        schema: &Schema<Self>,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
    ) -> QueryText;
}

pub struct ProcessTypeSystemDocumentOutcome<TNetworkProtocol: NetworkProtocol> {
    pub scalars: Vec<(ServerScalarEntity<TNetworkProtocol>, Location)>,
    pub objects: Vec<(
        ProcessObjectTypeDefinitionOutcome<TNetworkProtocol>,
        Location,
    )>,
}

#[derive(Debug)]
pub struct ProcessObjectTypeDefinitionOutcome<TNetworkProtocol: NetworkProtocol> {
    // TODO this is a GraphQLism, remove
    pub encountered_root_kind: Option<RootOperationKind>,
    pub server_object_entity: ServerObjectEntity<TNetworkProtocol>,
    pub fields_to_insert: Vec<WithLocation<FieldToInsert>>,
    // TODO this seems sketch
    pub expose_as_fields_to_insert: Vec<ExposeAsFieldToInsert>,
}

#[derive(Debug)]
pub struct FieldToInsert {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ServerSelectableName>,
    pub type_: GraphQLTypeAnnotation<UnvalidatedTypeName>,
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

#[derive(Debug)]
pub struct ExposeAsFieldToInsert {
    pub expose_field_directive: ExposeFieldDirective,
    // e.g. Query or Mutation
    pub parent_object_name: ServerObjectEntityName,
    pub description: Option<DescriptionValue>,
}
