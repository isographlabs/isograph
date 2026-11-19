use std::collections::BTreeSet;

use common_lang_types::{EntityName, SelectableName};
use graphql_schema_parser::{
    FieldDefinition, InputValueDefinition, NullableTypeAnnotation, TypeAnnotation, TypeDefinition,
    TypeExtension, TypeSystemDefinition, TypeSystemDefinitionOrExtension, TypeSystemExtension,
};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, CompositeShape, Concreteness, DefinitionLocation, EntityShape,
    IdentitySelection, IsographDb, ProtocolEntity, ProtocolSelectable, SelectableShape,
    UnionWrapper, Wrapper, WrapperVariant,
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
    /// A scalar, by its name.
    Scalar(EntityName),
    Enum,
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
    pub id_field: IdField,
}

/// Whether a type has a field `id: ID!`, which `graphql_entity_shape` makes its identity.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IdField {
    Present,
    Absent,
}

/// The selectable GraphQL selects in every selection set on a composite type.
pub(crate) const TYPENAME: &str = "__typename";
const ID: &str = "id";
const STRING: &str = "String";

/// An object, interface, or union is composite; an object is concrete; `id` is the identity
/// where `IdField` is `Present`.
pub(crate) fn graphql_entity_shape(data: &GraphQLEntityData) -> EntityShape {
    match data {
        GraphQLEntityData::Scalar(_) | GraphQLEntityData::Enum => EntityShape::Leaf,
        GraphQLEntityData::InputObject => EntityShape::Input,
        GraphQLEntityData::Object(composite) => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Concrete,
            identity: identity_selection(composite.id_field),
        }),
        GraphQLEntityData::Interface(composite) => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: identity_selection(composite.id_field),
        }),
        GraphQLEntityData::Union => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: IdentitySelection::Omit,
        }),
    }
}

fn identity_selection(id_field: IdField) -> IdentitySelection {
    match id_field {
        IdField::Present => IdentitySelection::Select(ID.intern().to()),
        IdField::Absent => IdentitySelection::Omit,
    }
}

/// A field or input field is a direct selection; a refinement is a conditional.
pub(crate) fn graphql_selectable_shape(data: &GraphQLSelectableData) -> SelectableShape {
    match data {
        GraphQLSelectableData::Field | GraphQLSelectableData::InputField => SelectableShape::Direct,
        GraphQLSelectableData::Refinement => SelectableShape::Refinement,
    }
}

/// `Present` when `name` has a field `id: ID!`. A failed lookup of `id` is `Absent`: the
/// error belongs to lookups of `id`, not to the type.
fn id_field<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> IdField {
    let Ok(Some(field)) = selectable_named(db, client, name, ID.intern().to()) else {
        return IdField::Absent;
    };
    if is_non_null_id(field.item.item.type_.item.reference()) {
        IdField::Present
    } else {
        IdField::Absent
    }
}

/// Whether `type_annotation` is `ID!`.
pub(crate) fn is_non_null_id(type_annotation: &TypeAnnotation) -> bool {
    matches!(
        type_annotation,
        TypeAnnotation::NonNull(non_null) if matches!(
            non_null.0.item.reference(),
            NullableTypeAnnotation::Named(named) if named.0.item.0.lookup() == "ID"
        )
    )
}

/// What the GraphQL protocol records about a selectable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphQLSelectableData {
    /// A field of the schema, printed as `name(arguments)`.
    Field,
    /// `asUser` on a union or interface: printed as `... on User`, where `User` is the
    /// selectable's target.
    Refinement,
    /// A field of an input object: read in parameter types, never selected.
    InputField,
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
                id_field: id_field(db, client, name),
            }
        };
    let data = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Scalar(_) => GraphQLEntityData::Scalar(name),
        TypeDefinition::Enum(_) => GraphQLEntityData::Enum,
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
    #[error("{0}")]
    #[from(skip)]
    DuplicateInputField(Box<DuplicateInputField>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Input field `{entity}.{selectable}` is defined more than once in the schema.")]
pub struct DuplicateInputField {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub first: InSchemaFile<InputValueDefinition>,
    pub second: InSchemaFile<InputValueDefinition>,
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
    if selectable.lookup() == TYPENAME
        && let Ok(Some(schema_entity)) = entity_named(db, client, entity)
        && let TypenameSelectable::Present = typename_selectable(schema_entity)
    {
        return typename().wrap_some().wrap_ok();
    }
    if let Some(input_field) = input_field_named(db, client, entity, selectable)? {
        return input_field.wrap_some().wrap_ok();
    }
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

/// The input field `selectable` of the input object `entity`; `Ok(None)` when `entity` is not
/// an input object or has no such field.
fn input_field_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLSelectableLookupError> {
    let Some(schema_entity) = entity_named(db, client, entity)
        .clone()
        .map_err(|error| GraphQLSelectableLookupError::Selectable(error.to()))?
    else {
        return None.wrap_ok();
    };
    let mut fields = input_fields(schema_entity.reference())
        .filter(|field| field.item.item.name.item.0.to::<SelectableName>() == selectable);
    let Some(first) = fields.next() else {
        return None.wrap_ok();
    };
    match fields.next() {
        None => input_field_selectable(first).wrap_some().wrap_ok(),
        Some(second) => GraphQLSelectableLookupError::DuplicateInputField(
            DuplicateInputField {
                entity,
                selectable,
                first,
                second,
            }
            .boxed(),
        )
        .wrap_err(),
    }
}

/// Every field of an input object's definition and its input object extensions, with its
/// file. Empty for every other kind of type.
pub(crate) fn input_fields(
    schema_entity: &SchemaEntity,
) -> impl Iterator<Item = InSchemaFile<InputValueDefinition>> + '_ {
    let definition_fields = match schema_entity.definition.item.item.reference() {
        TypeDefinition::InputObject(input_object) => input_object.fields.as_slice(),
        TypeDefinition::Scalar(_)
        | TypeDefinition::Object(_)
        | TypeDefinition::Interface(_)
        | TypeDefinition::Union(_)
        | TypeDefinition::Enum(_) => &[],
    }
    .iter()
    .map(|field| InSchemaFile {
        path: schema_entity.definition.path,
        item: field.clone(),
    });
    let extension_fields = schema_entity.extensions.iter().flat_map(|extension| {
        match extension.item.item.reference() {
            TypeExtension::InputObject(input_object) => input_object.fields.as_slice(),
            TypeExtension::Scalar(_)
            | TypeExtension::Object(_)
            | TypeExtension::Interface(_)
            | TypeExtension::Union(_)
            | TypeExtension::Enum(_) => &[],
        }
        .iter()
        .map(|field| InSchemaFile {
            path: extension.path,
            item: field.clone(),
        })
    });
    definition_fields.chain(extension_fields)
}

/// The protocol selectable an input object field defines.
fn input_field_selectable(
    field: InSchemaFile<InputValueDefinition>,
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
        }
        .wrap_some(),
        data: GraphQLSelectableData::InputField,
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
            definition: definition.wrap_some(),
            data: GraphQLSelectableData::Refinement,
        }
        .wrap_some()
        .wrap_ok(),
        Some(ProtocolEntity {
            data:
                GraphQLEntityData::Scalar(_)
                | GraphQLEntityData::Enum
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
        }
        .wrap_some(),
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
    let input_fields = input_fields(schema_entity).map(|field| {
        (
            field.item.item.name.item.0.to::<SelectableName>(),
            input_field_selectable(field),
        )
    });
    let typename = match typename_selectable(schema_entity) {
        TypenameSelectable::Present => (TYPENAME.intern().to(), typename()).wrap_some(),
        TypenameSelectable::Absent => None,
    };
    schema_entity_fields(schema_entity)
        .map(|field| (field.item.item.name.item.0, field_selectable(field)))
        .chain(input_fields)
        .chain(refinements)
        .chain(typename)
        .collect()
}

/// Whether a type has the `__typename` selectable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypenameSelectable {
    Present,
    Absent,
}

/// Every object, interface, and union has `__typename` (upstream's
/// `parse_nested_server_schema.rs`, which adds it to each).
fn typename_selectable(schema_entity: &SchemaEntity) -> TypenameSelectable {
    match schema_entity.definition.item.item.reference() {
        TypeDefinition::Object(_) | TypeDefinition::Interface(_) | TypeDefinition::Union(_) => {
            TypenameSelectable::Present
        }
        TypeDefinition::Scalar(_) | TypeDefinition::Enum(_) | TypeDefinition::InputObject(_) => {
            TypenameSelectable::Absent
        }
    }
}

/// `__typename`: a field of type `String!` that the schema text does not define.
fn typename() -> ProtocolSelectable<GraphQLSelectableData> {
    ProtocolSelectable {
        description: None,
        target: Wrapper::Inner(STRING.intern().to()),
        definition: None,
        data: GraphQLSelectableData::Field,
    }
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
