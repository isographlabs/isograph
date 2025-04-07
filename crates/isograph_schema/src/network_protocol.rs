use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt::Debug,
    hash::Hash,
};

use common_lang_types::{
    Location, QueryOperationName, QueryText, RelativePathToSourceFile, WithLocation,
};
use graphql_lang_types::GraphQLFieldDefinition;
use isograph_lang_types::{SchemaSource, ServerObjectEntityId};
use pico::{Database, SourceId};

use crate::{
    MergedSelectionMap, RootOperationName, Schema, ServerScalarEntity, TypeRefinementMaps,
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
        schema: &mut Schema<Self>,
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
    pub type_refinement_maps: TypeRefinementMaps,
    pub scalars: Vec<(ServerScalarEntity<TNetworkProtocol>, Location)>,
    pub field_queue: HashMap<ServerObjectEntityId, Vec<WithLocation<GraphQLFieldDefinition>>>,
}
