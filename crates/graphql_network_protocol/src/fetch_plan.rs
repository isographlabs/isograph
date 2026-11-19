use common_lang_types::EntityName;
use graphql_schema_parser::OperationType;
use isograph_compiler::{ClientId, CompilationProfile, IsographDb, MergedTarget, ProtocolEntity};
use pico_macros::memo;
use prelude::Postfix;

use crate::root_operation_types::{RootOperationTypesError, root_operation_types};
use crate::schema_lookup::EntityLookupError;
use crate::{GraphQLEntityData, graphql_entity};

/// How the GraphQL protocol fetches an entity that parents an entrypoint: the entity is the
/// root type of this operation type, and its selections are the operation's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchPlan {
    pub operation_type: OperationType,
    /// The root type, whose data decides the top-level `id` and `__typename`.
    pub entity: MergedTarget<GraphQLEntityData>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLNotFetchable {
    #[error("{0}")]
    RootOperationTypes(RootOperationTypesError),
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownEntity(EntityName),
    #[error("`{0}` is not a root operation type; entrypoints must be on a root operation type.")]
    NotRootOperationType(EntityName),
    #[error("`{0}` cannot be fetched: only object and interface types can parent an entrypoint.")]
    NotObjectOrInterface(EntityName),
}

/// How `entity` is fetched when it parents an entrypoint, as `client` reads the schema.
#[memo]
pub fn graphql_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<GraphQLFetchPlan, GraphQLNotFetchable> {
    let Some(ProtocolEntity { data, .. }) = graphql_entity(db, client, entity)
        .clone()
        .map_err(GraphQLNotFetchable::Entity)?
    else {
        return GraphQLNotFetchable::UnknownEntity(entity).wrap_err();
    };
    let roots = root_operation_types(db, client)
        .clone()
        .map_err(GraphQLNotFetchable::RootOperationTypes)?;
    let operation_type = roots
        .operation_type_of(entity)
        .ok_or(GraphQLNotFetchable::NotRootOperationType(entity))?;
    match data.reference() {
        GraphQLEntityData::Object(_) | GraphQLEntityData::Interface(_) => {}
        GraphQLEntityData::Scalar(_)
        | GraphQLEntityData::Enum
        | GraphQLEntityData::InputObject
        | GraphQLEntityData::Union => {
            return GraphQLNotFetchable::NotObjectOrInterface(entity).wrap_err();
        }
    }
    GraphQLFetchPlan {
        operation_type,
        entity: MergedTarget { name: entity, data },
    }
    .wrap_ok()
}
