use std::collections::BTreeMap;

use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, RelativePathToSourceFile,
    SelectableName,
};
use isograph_parser::{
    IsoLiteralItem, ListTypeAnnotation, SelectableDeclaration, TypeAnnotation, UnionVariant,
};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;

use crate::{
    ClientId, CompilationProfile, IsographDb, parsed_iso_literals_in_file, source_file_paths,
};

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub name: EntityName,
    pub definition: DefinitionLocation,
}

/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    /// `None` for a field declared without `to`.
    pub target_entity: Option<EntityName>,
    pub definition: DefinitionLocation,
}

/// Where an entity or selectable is defined: a span of a file, in that file's byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub path: RelativePathToSourceFile,
    pub span: Span,
}

/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Entity> {
    match source_entity_definitions(db, client, entity_name).as_slice() {
        [definition] => Entity {
            name: entity_name,
            definition: definition.dereference(),
        }
        .wrap_ok(),
        [] => Diagnostic::new(format!("Entity `{entity_name}` is not defined."), None).wrap_err(),
        [_, _, ..] => Diagnostic::new(
            format!("Entity `{entity_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
    }
}

/// The selectable named `selectable_name` on the entity named `entity_name` in the schema as
/// `client` reads it.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    match source_selectable_definitions(db, client, entity_name, selectable_name).as_slice() {
        [selectable] => selectable.clone().wrap_ok(),
        [] => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is not defined."),
            None,
        )
        .wrap_err(),
        [_, _, ..] => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
    }
}

/// Every selectable on the entity named `entity_name` that exactly one source file
/// declaration defines, by name.
#[memo]
pub fn selectables_for_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Vec<Selectable>> {
    entity_named(db, client, entity_name).clone()?;
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => {
                (declaration.parent_type.item.0 == entity_name).then(|| {
                    selectable_of_declaration(
                        source_item.path,
                        source_item.literal_start,
                        declaration,
                    )
                })
            }
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        })
        .fold(
            BTreeMap::<SelectableName, Vec<Selectable>>::new(),
            |mut by_name, selectable| {
                by_name.entry(selectable.name).or_default().push(selectable);
                by_name
            },
        )
        .into_values()
        .filter_map(|definitions| {
            <[Selectable; 1]>::try_from(definitions)
                .ok()
                .map(|[selectable]| selectable)
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

/// Where `entity <entity_name>` is declared in the source files `client` reads.
#[memo]
pub(crate) fn source_entity_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> Vec<DefinitionLocation> {
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Entity(declaration) => {
                (declaration.name.item.0 == entity_name).then(|| DefinitionLocation {
                    path: source_item.path,
                    span: declaration
                        .name
                        .location
                        .with_offset(source_item.literal_start),
                })
            }
            IsoLiteralItem::Entrypoint(_) | IsoLiteralItem::Selectable(_) => None,
        })
        .collect()
}

/// The selectable each `field <entity_name>.<selectable_name>` declaration in the source files
/// `client` reads defines.
#[memo]
pub(crate) fn source_selectable_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Vec<Selectable> {
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => (declaration.parent_type.item.0
                == entity_name
                && declaration.name.item.0 == selectable_name)
                .then(|| {
                    selectable_of_declaration(
                        source_item.path,
                        source_item.literal_start,
                        declaration,
                    )
                }),
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        })
        .collect()
}

/// An iso literal item parsed in a source file, with the byte offset of its literal in that file.
pub(crate) struct SourceIsoLiteralItem<'db> {
    pub path: RelativePathToSourceFile,
    pub literal_start: u32,
    pub item: &'db IsoLiteralItem,
}

/// Every iso literal item parsed in a source file `client` reads. A literal with parse errors
/// contributes the item it parsed.
pub(crate) fn source_iso_literal_items<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> impl Iterator<Item = SourceIsoLiteralItem<'_>> {
    source_file_paths(db, client).iter().flat_map(move |path| {
        parsed_iso_literals_in_file(db, client, path.dereference())
            .iter()
            .flatten()
            .filter_map(move |(extraction, parsed)| {
                let item = parsed.item.as_ref()?.item.item.as_ref()?;
                SourceIsoLiteralItem {
                    path: path.dereference(),
                    literal_start: extraction.iso_literal_start_index.0 as u32,
                    item: item.item.reference(),
                }
                .wrap_some()
            })
    })
}

/// The selectable `declaration`, in the literal at byte `literal_start` of `path`, defines.
fn selectable_of_declaration(
    path: RelativePathToSourceFile,
    literal_start: u32,
    declaration: &SelectableDeclaration,
) -> Selectable {
    Selectable {
        name: declaration.name.item.0,
        parent_entity: declaration.parent_type.item.0,
        description: declaration
            .description
            .map(|description| description.item.0),
        target_entity: declaration
            .target_type
            .as_ref()
            .and_then(|target_type| named_entity_of_type_annotation(target_type.item.reference())),
        definition: DefinitionLocation {
            path,
            span: declaration.name.location.with_offset(literal_start),
        },
    }
}

/// The entity a type annotation names, under its list and nullability wrappers. `None` for
/// a list whose inner type did not parse (`[!]`), which names no entity.
fn named_entity_of_type_annotation(type_annotation: &TypeAnnotation) -> Option<EntityName> {
    match type_annotation {
        TypeAnnotation::Named(named) => named.name.item.0.wrap_some(),
        TypeAnnotation::List(list) => named_entity_of_list(list),
        TypeAnnotation::Union(union) => {
            union
                .0
                .iter()
                .find_map(|variant| match variant.item.reference() {
                    UnionVariant::Named(named) => named.name.item.0.wrap_some(),
                    UnionVariant::List(list) => named_entity_of_list(list),
                    UnionVariant::Null(_) => None,
                })
        }
    }
}

fn named_entity_of_list(list: &ListTypeAnnotation) -> Option<EntityName> {
    named_entity_of_type_annotation(list.inner.as_ref()?.item.reference())
}
