use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt::Debug,
    hash::Hash,
};

use common_lang_types::{
    Location, QueryOperationName, QueryText, RelativePathToSourceFile, UnvalidatedTypeName,
    WithLocation,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLFieldDefinition, RootOperationKind,
};
use isograph_lang_types::SchemaSource;
use pico::{Database, SourceId};

use crate::{
    MergedSelectionMap, RootOperationName, Schema, ServerObjectEntity, ServerScalarEntity,
    ValidatedVariableDefinition,
};

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // These two types should be combined into a single type, this is a
    // GraphQL-ism leaking in
    type TypeSystemDocument: Debug + Clone + 'static;
    type TypeSystemExtensionDocument: Debug + Clone + 'static;

    type SchemaObjectAssociatedData: Debug;

    #[allow(clippy::type_complexity)]
    fn parse_and_process_type_system_documents(
        db: &Database,
        schema_source_id: SourceId<SchemaSource>,
        schema_extension_sources: &BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
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

    // TODO get rid of this
    pub unvalidated_supertype_to_subtype_map:
        HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>,
}

pub struct ProcessObjectTypeDefinitionOutcome<TNetworkProtocol: NetworkProtocol> {
    pub encountered_root_kind: Option<RootOperationKind>,
    pub directives: Vec<GraphQLDirective<GraphQLConstantValue>>,
    pub server_object_entity: ServerObjectEntity<TNetworkProtocol>,
    pub fields_to_insert: Vec<WithLocation<GraphQLFieldDefinition>>,
}
