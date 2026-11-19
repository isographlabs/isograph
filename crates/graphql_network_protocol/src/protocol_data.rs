use std::collections::BTreeSet;

use common_lang_types::{EntityName, SelectableName};
use graphql_schema_parser::{
    FieldDefinition, NullableTypeAnnotation, TypeAnnotation, TypeDefinition, TypeExtension,
    TypeSystemDefinition, TypeSystemDefinitionOrExtension, TypeSystemExtension,
};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, IsographDb, ProtocolEntity,
    ProtocolSelectable, UnionWrapper, Wrapper, WrapperVariant,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::schema_files::schema_entries;
use crate::schema_lookup::{
    EntityLookupError, InSchemaFile, SchemaEntity, SelectableLookupError, entity_named,
    schema_entity_fields, selectable_named, type_definition_name,
};

/// What the GraphQL protocol records about a type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLEntityData {
    /// A scalar or enum: selected with no selection set.
    Leaf,
    /// An input object: an argument type, never a selection target.
    InputObject,
    Object(GraphQLCompositeData),
    Interface(GraphQLCompositeData),
    Union,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLCompositeData {
    /// The interfaces this type declares it implements, in definition then extension order.
    pub interfaces: Vec<EntityName>,
}

/// What the GraphQL protocol records about a selectable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphQLSelectableData {
    /// A field of the schema, printed as `name(arguments)`.
    Field,
    /// `asUser` on a union or interface: printed as `... on User`, where `User` is the
    /// selectable's target.
    Refinement,
}

/// The GraphQL type `name` as `client` reads the schema, with the data the protocol records.
#[memo]
pub fn graphql_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, EntityLookupError> {
    let Some(schema_entity) = entity_named(db, client, name).clone()? else {
        return None.wrap_ok();
    };
    let composite =
        |definition_interfaces: &[span::WithSpan<graphql_schema_parser::EntityNameWrapper>]| {
            GraphQLCompositeData {
                interfaces: definition_interfaces
                    .iter()
                    .chain(schema_entity.extensions.iter().flat_map(|extension| {
                        match extension.item.item.reference() {
                            TypeExtension::Object(object) => object.interfaces.as_slice(),
                            TypeExtension::Interface(interface) => interface.interfaces.as_slice(),
                            TypeExtension::Scalar(_)
                            | TypeExtension::Union(_)
                            | TypeExtension::Enum(_)
                            | TypeExtension::InputObject(_) => &[],
                        }
                    }))
                    .map(|interface| interface.item.0)
                    .collect(),
            }
        };
    let data = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Scalar(_) | TypeDefinition::Enum(_) => GraphQLEntityData::Leaf,
        TypeDefinition::InputObject(_) => GraphQLEntityData::InputObject,
        TypeDefinition::Object(object) => {
            GraphQLEntityData::Object(composite(object.interfaces.as_slice()))
        }
        TypeDefinition::Interface(interface) => {
            GraphQLEntityData::Interface(composite(interface.interfaces.as_slice()))
        }
        TypeDefinition::Union(_) => GraphQLEntityData::Union,
    };
    ProtocolEntity {
        definition: definition_location(schema_entity.reference()),
        data,
    }
    .wrap_some()
    .wrap_ok()
}

/// The path of `schema_entity`'s definition and the span of its name.
fn definition_location(schema_entity: &SchemaEntity) -> DefinitionLocation {
    DefinitionLocation {
        path: schema_entity.definition.path,
        span: type_definition_name(schema_entity.definition.item.item.reference()).location,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum GraphQLSelectableLookupError {
    #[error("{0}")]
    Selectable(SelectableLookupError),
    #[error("{0}")]
    #[from(skip)]
    RefinementTarget(RefinementTargetError),
    #[error("{0}")]
    #[from(skip)]
    RefinementCollision(Box<RefinementCollision>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{entity}.{selectable}` refines to `{target}`, whose lookup failed: {error}")]
pub struct RefinementTargetError {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub target: EntityName,
    pub error: EntityLookupError,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{entity}.{selectable}` is defined in the schema and is also a refinement.")]
pub struct RefinementCollision {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub field: InSchemaFile<FieldDefinition>,
}

/// The selectable `selectable` of the GraphQL type `entity`: a field of the type, or a
/// refinement `as<Type>` of a union or interface.
#[memo]
pub fn graphql_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLSelectableLookupError> {
    let field = selectable_named(db, client, entity, selectable).clone()?;
    let refinement = refinement_named(db, client, entity, selectable)?;
    match (field, refinement) {
        (Some(field), Some(_)) => GraphQLSelectableLookupError::RefinementCollision(
            RefinementCollision {
                entity,
                selectable,
                field,
            }
            .boxed(),
        )
        .wrap_err(),
        (Some(field), None) => field_selectable(field).wrap_some().wrap_ok(),
        (None, refinement) => refinement.wrap_ok(),
    }
}

/// The refinement `selectable` of `entity`: `as<T>` where `T` is a member of the union
/// `entity`, or an object type implementing the interface `entity`.
fn refinement_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLSelectableLookupError> {
    let Some(target) = selectable
        .lookup()
        .strip_prefix(REFINEMENT_PREFIX)
        .map(|target| target.intern().to::<EntityName>())
    else {
        return None.wrap_ok();
    };
    let Ok(Some(schema_entity)) = entity_named(db, client, entity) else {
        return None.wrap_ok();
    };
    let is_candidate = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Union(_) => union_members(schema_entity).contains(&target),
        TypeDefinition::Interface(_) => match graphql_entity(db, client, target) {
            Ok(Some(ProtocolEntity {
                data: GraphQLEntityData::Object(object),
                ..
            })) => object.interfaces.contains(&entity),
            Ok(_) | Err(_) => false,
        },
        TypeDefinition::Scalar(_)
        | TypeDefinition::Object(_)
        | TypeDefinition::Enum(_)
        | TypeDefinition::InputObject(_) => false,
    };
    if !is_candidate {
        return None.wrap_ok();
    }
    refinement_to(db, client, target).map_err(|error| {
        GraphQLSelectableLookupError::RefinementTarget(RefinementTargetError {
            entity,
            selectable,
            target,
            error,
        })
    })
}

const REFINEMENT_PREFIX: &str = "as";

/// The refinement to `target` when it is an object type.
fn refinement_to<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    target: EntityName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, EntityLookupError> {
    match graphql_entity(db, client, target).clone()? {
        Some(ProtocolEntity {
            definition,
            data: GraphQLEntityData::Object(_),
        }) => ProtocolSelectable {
            description: None,
            target: Wrapper::Union(UnionWrapper(vec![
                WrapperVariant::Inner(target),
                WrapperVariant::Null,
            ])),
            definition,
            data: GraphQLSelectableData::Refinement,
        }
        .wrap_some()
        .wrap_ok(),
        Some(ProtocolEntity {
            data:
                GraphQLEntityData::Leaf
                | GraphQLEntityData::InputObject
                | GraphQLEntityData::Interface(_)
                | GraphQLEntityData::Union,
            ..
        })
        | None => None.wrap_ok(),
    }
}

/// The member types of a union's definition and its union extensions, sorted, each once.
pub(crate) fn union_members(schema_entity: &SchemaEntity) -> BTreeSet<EntityName> {
    let definition_members = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Union(union) => union.member_types.as_slice(),
        TypeDefinition::Scalar(_)
        | TypeDefinition::Object(_)
        | TypeDefinition::Interface(_)
        | TypeDefinition::Enum(_)
        | TypeDefinition::InputObject(_) => &[],
    };
    definition_members
        .iter()
        .chain(schema_entity.extensions.iter().flat_map(|extension| {
            match extension.item.item.reference() {
                TypeExtension::Union(union) => union.member_types.as_slice(),
                TypeExtension::Scalar(_)
                | TypeExtension::Object(_)
                | TypeExtension::Interface(_)
                | TypeExtension::Enum(_)
                | TypeExtension::InputObject(_) => &[],
            }
        }))
        .map(|member| member.item.0)
        .collect()
}

/// The protocol selectable a schema field defines.
fn field_selectable(
    field: InSchemaFile<FieldDefinition>,
) -> ProtocolSelectable<GraphQLSelectableData> {
    ProtocolSelectable {
        description: field
            .item
            .item
            .description
            .map(|description| description.item.0),
        target: wrapper_of_graphql_type(field.item.item.type_.item.reference()),
        definition: DefinitionLocation {
            path: field.path,
            span: field.item.item.name.location,
        },
        data: GraphQLSelectableData::Field,
    }
}

/// A GraphQL type annotation as a wrapper: `Foo!` is `Inner(Foo)`, `Foo` is
/// `Union([Inner(Foo), Null])`.
fn wrapper_of_graphql_type(type_annotation: &TypeAnnotation) -> Wrapper<EntityName> {
    match type_annotation {
        TypeAnnotation::Named(named) => nullable(WrapperVariant::Inner(named.0.item.0)),
        TypeAnnotation::List(list) => nullable(WrapperVariant::List(
            wrapper_of_graphql_type(list.0.item.reference()).boxed(),
        )),
        TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
            NullableTypeAnnotation::Named(named) => Wrapper::Inner(named.0.item.0),
            NullableTypeAnnotation::List(list) => {
                Wrapper::List(wrapper_of_graphql_type(list.0.item.reference()).boxed())
            }
        },
    }
}

fn nullable(variant: WrapperVariant<EntityName>) -> Wrapper<EntityName> {
    Wrapper::Union(UnionWrapper(vec![variant, WrapperVariant::Null]))
}

/// Every selectable of the GraphQL type `entity`: its fields, then its refinements whose
/// target resolves. A name may repeat. Empty when the type's lookup fails.
#[memo]
pub fn graphql_selectables<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Vec<(SelectableName, ProtocolSelectable<GraphQLSelectableData>)> {
    let Ok(Some(schema_entity)) = entity_named(db, client, entity) else {
        return Vec::new();
    };
    let candidates: Vec<EntityName> = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Union(_) => union_members(schema_entity).into_iter().collect(),
        TypeDefinition::Interface(_) => object_types_implementing(db, client, entity).clone(),
        TypeDefinition::Scalar(_)
        | TypeDefinition::Object(_)
        | TypeDefinition::Enum(_)
        | TypeDefinition::InputObject(_) => Vec::new(),
    };
    let refinements = candidates.into_iter().filter_map(|target| {
        refinement_to(db, client, target)
            .ok()
            .flatten()
            .map(|refinement| (refinement_name(target), refinement))
    });
    schema_entity_fields(schema_entity)
        .map(|field| (field.item.item.name.item.0, field_selectable(field)))
        .chain(refinements)
        .collect()
}

/// `as<target>`.
fn refinement_name(target: EntityName) -> SelectableName {
    format!("{REFINEMENT_PREFIX}{target}").intern().to()
}

/// Every object type whose definition or an object extension lists `interface` in
/// `implements`, sorted, each once.
#[memo]
pub fn object_types_implementing<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    interface: EntityName,
) -> Vec<EntityName> {
    schema_entries(db, client)
        .filter_map(|schema_entry| {
            let (name, interfaces) = match schema_entry.entry.as_ref().ok()?.item.reference() {
                TypeSystemDefinitionOrExtension::Definition(TypeSystemDefinition::Type(
                    TypeDefinition::Object(object),
                )) => (object.name, object.interfaces.as_slice()),
                TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Type(
                    TypeExtension::Object(object),
                )) => (object.name, object.interfaces.as_slice()),
                TypeSystemDefinitionOrExtension::Definition(_)
                | TypeSystemDefinitionOrExtension::Extension(_) => return None,
            };
            interfaces
                .iter()
                .any(|implemented| implemented.item.0 == interface)
                .then_some(name.item.0)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
