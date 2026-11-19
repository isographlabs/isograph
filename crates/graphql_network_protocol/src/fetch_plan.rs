use common_lang_types::{EntityName, InputValueName};
use graphql_schema_parser::{NullableTypeAnnotation, OperationType, TypeAnnotation};
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, MergedTarget, NonNullVariableType, ProtocolEntity,
    VariableType,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::protocol_data::union_members;
use crate::root_operation_types::{RootOperationTypesError, root_operation_types};
use crate::schema_lookup::{
    EntityLookupError, SelectableLookupError, entity_named, selectable_named,
};
use crate::{GraphQLEntityData, graphql_entity};

/// How the GraphQL protocol fetches an entity that parents an entrypoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLFetchPlan {
    /// The entity is a root operation type: its selections are the operation's.
    Root(GraphQLRootFetch),
    /// A `query` that selects the query root's `node` field, which returns the entity or an
    /// interface containing it.
    Node(GraphQLNodeFetch),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLRootFetch {
    pub operation_type: OperationType,
    /// The root type, whose data decides the top-level `id` and `__typename`.
    pub entity: MergedTarget<GraphQLEntityData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLNodeFetch {
    /// `node`'s arguments, each passed as the operation variable of the same name.
    pub arguments: Vec<GraphQLFetchArgument>,
    /// The type `node` returns, which the operation builder completes like any selection
    /// target.
    pub field_target: MergedTarget<GraphQLEntityData>,
    pub refinement: FetchRefinement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchArgument {
    pub name: InputValueName,
    /// The argument's type, made nullable when the argument has a default, so the caller
    /// may omit it (spec 5.8.5, All Variable Usages Are Allowed).
    pub variable_type: VariableType,
}

/// How the entity's selections sit under `node`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchRefinement {
    /// `node` returns the entity itself.
    Exact,
    /// `node` returns an interface or union that contains the entity:
    /// `... on Entity { }`. The entity's data decides the fragment's `id`.
    InlineFragment(MergedTarget<GraphQLEntityData>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLNotFetchable {
    #[error("{0}")]
    RootOperationTypes(RootOperationTypesError),
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownEntity(EntityName),
    #[error("`{0}` cannot be fetched: only object and interface types can parent an entrypoint.")]
    NotObjectOrInterface(EntityName),
    #[error(
        "`{0}` cannot be fetched: it is not a root operation type and does not implement `Node`."
    )]
    NoFetchRoot(EntityName),
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldTypeMismatch {
    pub query_root: EntityName,
    pub returns: EntityName,
    pub entity: EntityName,
}

const NODE: &str = "Node";

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
    let interfaces = match data.reference() {
        GraphQLEntityData::Object(composite) | GraphQLEntityData::Interface(composite) => {
            composite.interfaces.clone()
        }
        GraphQLEntityData::Leaf | GraphQLEntityData::InputObject | GraphQLEntityData::Union => {
            return GraphQLNotFetchable::NotObjectOrInterface(entity).wrap_err();
        }
    };
    let roots = root_operation_types(db, client)
        .clone()
        .map_err(GraphQLNotFetchable::RootOperationTypes)?;
    let target = MergedTarget { name: entity, data };
    if let Some(operation_type) = roots.operation_type_of(entity) {
        return GraphQLFetchPlan::Root(GraphQLRootFetch {
            operation_type,
            entity: target,
        })
        .wrap_ok();
    }
    let node: EntityName = NODE.intern().to();
    if entity == node || interfaces.contains(&node) {
        return node_fetch(db, client, roots.query, target)
            .map(GraphQLFetchPlan::Node)
            .map_err(GraphQLNotFetchable::NodeField);
    }
    GraphQLNotFetchable::NoFetchRoot(entity).wrap_err()
}

/// The fetch of `entity` through the `node` field of the query root `query_root`.
fn node_fetch<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    query_root: EntityName,
    entity: MergedTarget<GraphQLEntityData>,
) -> Result<GraphQLNodeFetch, NodeFieldError> {
    let field = selectable_named(db, client, query_root, "node".intern().to())
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
    let field_target = graphql_entity(db, client, returns)
        .clone()
        .map_err(NodeFieldError::Target)?
        .ok_or(NodeFieldError::UnknownTarget(returns))?;
    let refinement = if returns == entity.name {
        FetchRefinement::Exact
    } else if contains(
        db,
        client,
        returns,
        field_target.data.reference(),
        entity.reference(),
    ) {
        FetchRefinement::InlineFragment(entity)
    } else {
        return NodeFieldError::TypeMismatch(NodeFieldTypeMismatch {
            query_root,
            returns,
            entity: entity.name,
        })
        .wrap_err();
    };
    GraphQLNodeFetch {
        arguments: field
            .item
            .item
            .arguments
            .iter()
            .map(|argument| GraphQLFetchArgument {
                name: argument.item.name.item.0,
                variable_type: match argument.item.default_value {
                    None => variable_type(argument.item.type_.item.reference()),
                    Some(_) => nullable_variable_type(argument.item.type_.item.reference()),
                },
            })
            .collect(),
        field_target: MergedTarget {
            name: returns,
            data: field_target.data,
        },
        refinement,
    }
    .wrap_ok()
}

/// Whether the interface or union `returns`, whose data is `returns_data`, contains `entity`.
fn contains<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    returns: EntityName,
    returns_data: &GraphQLEntityData,
    entity: &MergedTarget<GraphQLEntityData>,
) -> bool {
    match returns_data {
        GraphQLEntityData::Interface(_) => match entity.data.reference() {
            GraphQLEntityData::Object(composite) | GraphQLEntityData::Interface(composite) => {
                composite.interfaces.contains(&returns)
            }
            GraphQLEntityData::Leaf | GraphQLEntityData::InputObject | GraphQLEntityData::Union => {
                false
            }
        },
        GraphQLEntityData::Union => match entity_named(db, client, returns) {
            Ok(Some(schema_entity)) => union_members(schema_entity).contains(&entity.name),
            Ok(None) | Err(_) => false,
        },
        GraphQLEntityData::Leaf | GraphQLEntityData::InputObject | GraphQLEntityData::Object(_) => {
            false
        }
    }
}

/// A GraphQL type annotation as a variable type.
fn variable_type(type_annotation: &TypeAnnotation) -> VariableType {
    match type_annotation {
        TypeAnnotation::NonNull(non_null) => {
            VariableType::NonNull(non_null_variable_type(non_null.0.item.reference()))
        }
        TypeAnnotation::Named(_) | TypeAnnotation::List(_) => {
            nullable_variable_type(type_annotation)
        }
    }
}

/// A GraphQL type annotation as a variable type, with its outer `!` dropped.
fn nullable_variable_type(type_annotation: &TypeAnnotation) -> VariableType {
    VariableType::Nullable(match type_annotation {
        TypeAnnotation::Named(named) => NonNullVariableType::Named(named.0.item.0),
        TypeAnnotation::List(list) => {
            NonNullVariableType::List(variable_type(list.0.item.reference()).boxed())
        }
        TypeAnnotation::NonNull(non_null) => non_null_variable_type(non_null.0.item.reference()),
    })
}

fn non_null_variable_type(nullable: &NullableTypeAnnotation) -> NonNullVariableType {
    match nullable {
        NullableTypeAnnotation::Named(named) => NonNullVariableType::Named(named.0.item.0),
        NullableTypeAnnotation::List(list) => {
            NonNullVariableType::List(variable_type(list.0.item.reference()).boxed())
        }
    }
}
