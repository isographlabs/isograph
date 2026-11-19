use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use graphql_schema_parser::{
    DefinitionError, DefinitionName, EntityNameWrapper, FieldDefinition, TypeDefinition,
    TypeExtension, TypeSystemDefinition, TypeSystemDefinitionOrExtension, TypeSystemExtension,
};
use isograph_compiler::{ClientId, CompilationProfile, IsographDb};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::schema_files::schema_entries;

/// A definition or extension and the schema file it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InSchemaFile<T> {
    pub path: RelativePathToSourceFile,
    pub item: WithSpan<T>,
}

/// Every schema source of one named entity: its one definition and its extensions, in
/// schema file path order then source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaEntity {
    pub definition: InSchemaFile<TypeDefinition>,
    pub extensions: Vec<InSchemaFile<TypeExtension>>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntityLookupError {
    #[error("{0}")]
    Syntax(Box<EntitySyntaxError>),
    #[error("{0}")]
    DuplicateDefinition(Box<DuplicateDefinition>),
    #[error("{0}")]
    ExtensionWithoutDefinition(Box<ExtensionWithoutDefinition>),
    #[error("{0}")]
    ExtensionKindMismatch(Box<ExtensionKindMismatch>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("A definition or extension of `{name}` has a syntax error: {}", .failed.item.item.error.item)]
pub struct EntitySyntaxError {
    pub name: EntityName,
    pub failed: InSchemaFile<DefinitionError>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Type `{name}` is defined more than once in the schema.")]
pub struct DuplicateDefinition {
    pub name: EntityName,
    pub first: InSchemaFile<TypeDefinition>,
    pub second: InSchemaFile<TypeDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Type `{name}` is extended but never defined.")]
pub struct ExtensionWithoutDefinition {
    pub name: EntityName,
    pub extension: InSchemaFile<TypeExtension>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("An extension of `{name}` is not the kind of type `{name}` is defined as.")]
pub struct ExtensionKindMismatch {
    pub name: EntityName,
    pub extension: InSchemaFile<TypeExtension>,
}

/// The kind of a type definition or extension (spec 3.4).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
}

pub(crate) fn type_definition_name(definition: &TypeDefinition) -> WithSpan<EntityNameWrapper> {
    match definition {
        TypeDefinition::Scalar(scalar) => scalar.name,
        TypeDefinition::Object(object) => object.name,
        TypeDefinition::Interface(interface) => interface.name,
        TypeDefinition::Union(union) => union.name,
        TypeDefinition::Enum(enum_definition) => enum_definition.name,
        TypeDefinition::InputObject(input_object) => input_object.name,
    }
}

pub(crate) fn type_definition_kind(definition: &TypeDefinition) -> TypeKind {
    match definition {
        TypeDefinition::Scalar(_) => TypeKind::Scalar,
        TypeDefinition::Object(_) => TypeKind::Object,
        TypeDefinition::Interface(_) => TypeKind::Interface,
        TypeDefinition::Union(_) => TypeKind::Union,
        TypeDefinition::Enum(_) => TypeKind::Enum,
        TypeDefinition::InputObject(_) => TypeKind::InputObject,
    }
}

pub(crate) fn type_extension_name(extension: &TypeExtension) -> WithSpan<EntityNameWrapper> {
    match extension {
        TypeExtension::Scalar(scalar) => scalar.name,
        TypeExtension::Object(object) => object.name,
        TypeExtension::Interface(interface) => interface.name,
        TypeExtension::Union(union) => union.name,
        TypeExtension::Enum(enum_extension) => enum_extension.name,
        TypeExtension::InputObject(input_object) => input_object.name,
    }
}

pub(crate) fn type_extension_kind(extension: &TypeExtension) -> TypeKind {
    match extension {
        TypeExtension::Scalar(_) => TypeKind::Scalar,
        TypeExtension::Object(_) => TypeKind::Object,
        TypeExtension::Interface(_) => TypeKind::Interface,
        TypeExtension::Union(_) => TypeKind::Union,
        TypeExtension::Enum(_) => TypeKind::Enum,
        TypeExtension::InputObject(_) => TypeKind::InputObject,
    }
}

/// The entries of one name across every schema file.
struct NamedSchemaEntries {
    definitions: Vec<InSchemaFile<TypeDefinition>>,
    extensions: Vec<InSchemaFile<TypeExtension>>,
    first_failure: Option<InSchemaFile<DefinitionError>>,
}

/// The named entity `name` as `client` reads the schema; `Ok(None)` when no schema file
/// defines or extends it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<SchemaEntity>, EntityLookupError> {
    let entries =
        schema_entries(db, client).fold(
            NamedSchemaEntries {
                definitions: Vec::new(),
                extensions: Vec::new(),
                first_failure: None,
            },
            |mut entries, schema_entry| {
                let path = schema_entry.path;
                match schema_entry.entry {
                    Ok(entry) => match entry.item.reference() {
                        TypeSystemDefinitionOrExtension::Definition(
                            TypeSystemDefinition::Type(definition),
                        ) => {
                            if type_definition_name(definition).item.0 == name {
                                entries.definitions.push(InSchemaFile {
                                    path,
                                    item: definition.clone().with_span(entry.location),
                                });
                            }
                        }
                        TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Type(
                            extension,
                        )) => {
                            if type_extension_name(extension).item.0 == name {
                                entries.extensions.push(InSchemaFile {
                                    path,
                                    item: extension.clone().with_span(entry.location),
                                });
                            }
                        }
                        TypeSystemDefinitionOrExtension::Definition(
                            TypeSystemDefinition::Schema(_) | TypeSystemDefinition::Directive(_),
                        )
                        | TypeSystemDefinitionOrExtension::Extension(
                            TypeSystemExtension::Schema(_),
                        ) => {}
                    },
                    Err(failed) => {
                        if failed.item.name == DefinitionName::Entity(name).wrap_some()
                            && entries.first_failure.is_none()
                        {
                            entries.first_failure = InSchemaFile {
                                path,
                                item: failed.dereference(),
                            }
                            .wrap_some();
                        }
                    }
                }
                entries
            },
        );
    if let Some(failed) = entries.first_failure {
        return EntityLookupError::Syntax(EntitySyntaxError { name, failed }.boxed()).wrap_err();
    }
    let mut definitions = entries.definitions.into_iter();
    let Some(definition) = definitions.next() else {
        return match entries.extensions.into_iter().next() {
            None => None.wrap_ok(),
            Some(extension) => EntityLookupError::ExtensionWithoutDefinition(
                ExtensionWithoutDefinition { name, extension }.boxed(),
            )
            .wrap_err(),
        };
    };
    if let Some(second) = definitions.next() {
        return EntityLookupError::DuplicateDefinition(
            DuplicateDefinition {
                name,
                first: definition,
                second,
            }
            .boxed(),
        )
        .wrap_err();
    }
    let kind = type_definition_kind(definition.item.item.reference());
    if let Some(extension) = entries
        .extensions
        .iter()
        .find(|extension| type_extension_kind(extension.item.item.reference()) != kind)
    {
        return EntityLookupError::ExtensionKindMismatch(
            ExtensionKindMismatch {
                name,
                extension: extension.clone(),
            }
            .boxed(),
        )
        .wrap_err();
    }
    SchemaEntity {
        definition,
        extensions: entries.extensions,
    }
    .wrap_some()
    .wrap_ok()
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum SelectableLookupError {
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("{0}")]
    DuplicateField(Box<DuplicateField>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{entity}.{selectable}` is defined more than once in the schema.")]
pub struct DuplicateField {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub first: InSchemaFile<FieldDefinition>,
    pub second: InSchemaFile<FieldDefinition>,
}

/// The field `selectable` of the entity `entity` in the schema; `Ok(None)` when the entity
/// does not exist in the schema or has no such field.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<InSchemaFile<FieldDefinition>>, SelectableLookupError> {
    let Some(schema_entity) = entity_named(db, client, entity).clone()? else {
        return None.wrap_ok();
    };
    let mut fields = schema_entity_fields(schema_entity.reference())
        .filter(|field| field.item.item.name.item.0 == selectable);
    let Some(first) = fields.next() else {
        return None.wrap_ok();
    };
    match fields.next() {
        None => first.wrap_some().wrap_ok(),
        Some(second) => SelectableLookupError::DuplicateField(
            DuplicateField {
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

/// Every field of `schema_entity`: the definition's then each extension's, with its file.
pub(crate) fn schema_entity_fields(
    schema_entity: &SchemaEntity,
) -> impl Iterator<Item = InSchemaFile<FieldDefinition>> + '_ {
    let definition_fields = match schema_entity.definition.item.item.reference() {
        TypeDefinition::Object(object) => object.fields.as_slice(),
        TypeDefinition::Interface(interface) => interface.fields.as_slice(),
        TypeDefinition::Scalar(_)
        | TypeDefinition::Union(_)
        | TypeDefinition::Enum(_)
        | TypeDefinition::InputObject(_) => &[],
    }
    .iter()
    .map(|field| InSchemaFile {
        path: schema_entity.definition.path,
        item: field.clone(),
    });
    let extension_fields = schema_entity.extensions.iter().flat_map(|extension| {
        match extension.item.item.reference() {
            TypeExtension::Object(object) => object.fields.as_slice(),
            TypeExtension::Interface(interface) => interface.fields.as_slice(),
            TypeExtension::Scalar(_)
            | TypeExtension::Union(_)
            | TypeExtension::Enum(_)
            | TypeExtension::InputObject(_) => &[],
        }
        .iter()
        .map(|field| InSchemaFile {
            path: extension.path,
            item: field.clone(),
        })
    });
    definition_fields.chain(extension_fields)
}
