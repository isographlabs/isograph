use common_lang_types::{EntityName, InputValueName};
use graphql_schema_parser::{NullableTypeAnnotation, OperationType, TypeAnnotation};
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, EntrypointWrapper, IsographDb, ProtocolEntity,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::protocol_data::{is_non_null_id, union_members};
use crate::root_operation_types::{RootOperationTypesError, root_operation_types};
use crate::schema_lookup::{
    EntityLookupError, SelectableLookupError, entity_named, selectable_named,
};
use crate::{GraphQLEntityData, graphql_entity};

/// How the GraphQL protocol fetches an entity by itself: the operation type, and how the
/// operation's response reaches the entity's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchPlan {
    pub operation_type: OperationType,
    pub wrapper: EntrypointWrapper,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLNotFetchable {
    #[error("{0}")]
    RootOperationTypes(RootOperationTypesError),
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownEntity(EntityName),
    #[error("`{0}` is not an object type, so it cannot be fetched by itself.")]
    NotObject(EntityName),
    #[error(
        "`{0}` is not a root operation type and does not implement `Node`, so it cannot be \
        fetched by itself."
    )]
    NeitherRootNorNode(EntityName),
    #[error("{0}")]
    NodeField(NodeFieldError),
}

/// Why the query root's `node` field cannot fetch an entity. Each `EntityName` payload is the
/// query root.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NodeFieldError {
    #[error("{0}")]
    Selectable(SelectableLookupError),
    #[error("`{0}.node` is not defined.")]
    NotFound(EntityName),
    #[error("`{0}.node` returns a list; it returns one value.")]
    ReturnsList(EntityName),
    #[error(
        "`{}.node` returns `{}`, which is not `{}` or an interface or union containing it.",
        .0.query_root, .0.returns, .0.entity
    )]
    TypeMismatch(NodeFieldTypeMismatch),
    #[error("{0}")]
    Target(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownTarget(EntityName),
    #[error("`{0}.node` has no argument `id: ID!`.")]
    IdArgument(EntityName),
    #[error(
        "`{}.node` requires `{}`, which a fetch by id does not pass.",
        .0.query_root, .0.argument
    )]
    RequiredArgument(NodeFieldArgument),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldTypeMismatch {
    pub query_root: EntityName,
    pub returns: EntityName,
    pub entity: EntityName,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldArgument {
    pub query_root: EntityName,
    pub argument: InputValueName,
}

const NODE_INTERFACE: &str = "Node";
const NODE_FIELD: &str = "node";
const ID_ARGUMENT: &str = "id";

/// How `entity` is fetched by itself, as `client` reads the schema.
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
    let composite = match data {
        GraphQLEntityData::Object(composite) => composite,
        GraphQLEntityData::Scalar(_)
        | GraphQLEntityData::Enum
        | GraphQLEntityData::InputObject
        | GraphQLEntityData::Interface(_)
        | GraphQLEntityData::Union => return GraphQLNotFetchable::NotObject(entity).wrap_err(),
    };
    let roots = root_operation_types(db, client)
        .clone()
        .map_err(GraphQLNotFetchable::RootOperationTypes)?;
    if let Some(operation_type) = roots.operation_type_of(entity) {
        return GraphQLFetchPlan {
            operation_type,
            wrapper: EntrypointWrapper::Unwrapped(roots.root_entities(entity)),
        }
        .wrap_ok();
    }
    let node: EntityName = NODE_INTERFACE.intern().to();
    if !composite.interfaces.contains(node.reference()) {
        return GraphQLNotFetchable::NeitherRootNorNode(entity).wrap_err();
    }
    node_field(
        db,
        client,
        roots.query,
        entity,
        composite.interfaces.as_slice(),
    )
    .map_err(GraphQLNotFetchable::NodeField)?;
    GraphQLFetchPlan {
        operation_type: OperationType::Query,
        wrapper: EntrypointWrapper::Wrapped,
    }
    .wrap_ok()
}

/// Whether the `node` field of the query root `query_root` fetches the object type `entity`,
/// which implements `interfaces`, by id; the reason when it does not.
fn node_field<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    query_root: EntityName,
    entity: EntityName,
    interfaces: &[EntityName],
) -> Result<(), NodeFieldError> {
    let field = selectable_named(db, client, query_root, NODE_FIELD.intern().to())
        .clone()
        .map_err(NodeFieldError::Selectable)?
        .ok_or(NodeFieldError::NotFound(query_root))?;
    let returns = match field.item.item.type_.item.reference() {
        TypeAnnotation::Named(named) => named.0.item.0,
        TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
            NullableTypeAnnotation::Named(named) => named.0.item.0,
            NullableTypeAnnotation::List(_) => {
                return NodeFieldError::ReturnsList(query_root).wrap_err();
            }
        },
        TypeAnnotation::List(_) => return NodeFieldError::ReturnsList(query_root).wrap_err(),
    };
    let returns_data = graphql_entity(db, client, returns)
        .clone()
        .map_err(NodeFieldError::Target)?
        .ok_or(NodeFieldError::UnknownTarget(returns))?
        .data;
    if returns != entity
        && !contains(
            db,
            client,
            returns,
            returns_data.reference(),
            entity,
            interfaces,
        )
    {
        return NodeFieldError::TypeMismatch(NodeFieldTypeMismatch {
            query_root,
            returns,
            entity,
        })
        .wrap_err();
    }
    let arguments = field
        .item
        .item
        .arguments
        .iter()
        .map(|argument| argument.item.reference());
    let id: InputValueName = ID_ARGUMENT.intern().to();
    if !arguments.clone().any(|argument| {
        argument.name.item.0 == id && is_non_null_id(argument.type_.item.reference())
    }) {
        return NodeFieldError::IdArgument(query_root).wrap_err();
    }
    if let Some(required) = arguments.clone().find(|argument| {
        argument.name.item.0 != id
            && argument.default_value.is_none()
            && matches!(argument.type_.item.reference(), TypeAnnotation::NonNull(_))
    }) {
        return NodeFieldError::RequiredArgument(NodeFieldArgument {
            query_root,
            argument: required.name.item.0,
        })
        .wrap_err();
    }
    ().wrap_ok()
}

/// Whether the interface or union `returns`, whose data is `returns_data`, contains the object
/// type `entity`, which implements `interfaces`.
fn contains<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    returns: EntityName,
    returns_data: &GraphQLEntityData,
    entity: EntityName,
    interfaces: &[EntityName],
) -> bool {
    match returns_data {
        GraphQLEntityData::Interface(_) => interfaces.contains(returns.reference()),
        GraphQLEntityData::Union => match entity_named(db, client, returns) {
            Ok(Some(schema_entity)) => union_members(schema_entity).contains(entity.reference()),
            Ok(None) | Err(_) => false,
        },
        GraphQLEntityData::Scalar(_)
        | GraphQLEntityData::Enum
        | GraphQLEntityData::InputObject
        | GraphQLEntityData::Object(_) => false,
    }
}
