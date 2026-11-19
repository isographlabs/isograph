mod protocol_data;
mod schema_files;
mod schema_lookup;
#[cfg(test)]
mod tests;

use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use isograph_compiler::{
    ClientId, CompilationProfile, IncludeFile, IsographDb, NetworkProtocol, ProtocolEntity,
    ProtocolSelectable,
};

pub use protocol_data::{
    GraphQLCompositeData, GraphQLEntityData, GraphQLSelectableData, GraphQLSelectableLookupError,
    RefinementCollision, RefinementTargetError, graphql_entity, graphql_selectable,
    graphql_selectables, object_types_implementing,
};
pub use schema_files::{parsed_schema_file, schema_file_paths};
pub use schema_lookup::{
    DuplicateDefinition, DuplicateField, EntityLookupError, EntitySyntaxError,
    ExtensionKindMismatch, ExtensionWithoutDefinition, InSchemaFile, SchemaEntity,
    SelectableLookupError, entity_named, selectable_named,
};

pub struct GraphQLNetworkProtocol;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum GraphQLLookupError {
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("{0}")]
    Selectable(GraphQLSelectableLookupError),
}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityData = GraphQLEntityData;
    type SelectableData = GraphQLSelectableData;
    type LookupError = GraphQLLookupError;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql" | "graphqls") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }

    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        name: EntityName,
    ) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, GraphQLLookupError> {
        graphql_entity(db, client, name)
            .clone()
            .map_err(GraphQLLookupError::from)
    }

    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLLookupError> {
        graphql_selectable(db, client, entity, selectable)
            .clone()
            .map_err(GraphQLLookupError::from)
    }

    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<GraphQLSelectableData>)> {
        graphql_selectables(db, client, entity).clone()
    }
}
