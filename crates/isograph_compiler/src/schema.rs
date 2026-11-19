use std::collections::BTreeMap;

use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, RelativePathToSourceFile,
    SelectableName,
};
use isograph_parser::{IsoLiteralItem, SelectableDeclaration};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;

use crate::wrapper::{Wrapper, wrapper_of_type_annotation};
use crate::{
    ClientId, CompilationProfile, IsographDb, NetworkProtocol, parsed_iso_literals_in_file,
    source_file_paths,
};

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity<TEntityData> {
    pub name: EntityName,
    pub definition: DefinitionLocation,
    pub origin: EntityOrigin<TEntityData>,
}

/// Who defines an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityOrigin<TEntityData> {
    /// `entity Owner` in a source file.
    Source,
    Protocol(TEntityData),
}

/// A named pointer from an entity to a wrapper of an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable<TSelectableData> {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub definition: DefinitionLocation,
    pub kind: SelectableKind<TSelectableData>,
}

/// Who defines a selectable, and what it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientPointerTarget {
    Parsed(Wrapper<EntityName>),
    /// A `to` type with a list whose inner type did not parse (`to [!]`).
    Unparsed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerSelectable<TSelectableData> {
    pub target: Wrapper<EntityName>,
    pub data: TSelectableData,
}

impl<TSelectableData> Selectable<TSelectableData> {
    /// The entity selections under a selection of this selectable select on: the first
    /// inner entity of its target.
    pub fn target_entity(&self) -> Option<EntityName> {
        match self.kind.reference() {
            SelectableKind::ClientField => None,
            SelectableKind::ClientPointer(target) => target.target_entity(),
            SelectableKind::Server(server) => server.target.inner_values().next().copied(),
        }
    }
}

impl ClientPointerTarget {
    /// The first inner entity of the `to` type.
    pub fn target_entity(&self) -> Option<EntityName> {
        match self {
            ClientPointerTarget::Parsed(target) => target.inner_values().next().copied(),
            ClientPointerTarget::Unparsed => None,
        }
    }
}

/// The entity and selectable types of a profile.
pub type ProfileEntity<TProfile> =
    Entity<<<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityData>;
pub type ProfileSelectable<TProfile> = Selectable<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableData,
>;

/// What a `field <entity>.<selectable>` declaration in a source file defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSelectableDefinition {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub kind: SourceSelectableKind,
    pub definition: DefinitionLocation,
}

/// The kinds of selectable a `field` declaration defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSelectableKind {
    ClientField,
    ClientPointer(ClientPointerTarget),
}

impl SourceSelectableDefinition {
    /// The entity selections under a selection of this selectable select on.
    pub fn target_entity(&self) -> Option<EntityName> {
        match self.kind.reference() {
            SourceSelectableKind::ClientField => None,
            SourceSelectableKind::ClientPointer(target) => target.target_entity(),
        }
    }

    fn selectable<TSelectableData>(&self) -> Selectable<TSelectableData> {
        Selectable {
            name: self.name,
            parent_entity: self.parent_entity,
            description: self.description,
            definition: self.definition,
            kind: match self.kind.reference() {
                SourceSelectableKind::ClientField => SelectableKind::ClientField,
                SourceSelectableKind::ClientPointer(target) => {
                    SelectableKind::ClientPointer(target.clone())
                }
            },
        }
    }
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
) -> DiagnosticResult<ProfileEntity<TProfile>> {
    match source_entity_definitions(db, client, entity_name).as_slice() {
        [definition] => Entity {
            name: entity_name,
            definition: definition.dereference(),
            origin: EntityOrigin::Source,
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
) -> DiagnosticResult<ProfileSelectable<TProfile>> {
    match source_selectable_definitions(db, client, entity_name, selectable_name).as_slice() {
        [selectable] => selectable.selectable().wrap_ok(),
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
) -> DiagnosticResult<Vec<ProfileSelectable<TProfile>>> {
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
            BTreeMap::<SelectableName, Vec<SourceSelectableDefinition>>::new(),
            |mut by_name, selectable| {
                by_name.entry(selectable.name).or_default().push(selectable);
                by_name
            },
        )
        .into_values()
        .filter_map(|definitions| {
            <[SourceSelectableDefinition; 1]>::try_from(definitions)
                .ok()
                .map(|[selectable]| selectable.selectable())
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
) -> Vec<SourceSelectableDefinition> {
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
) -> SourceSelectableDefinition {
    SourceSelectableDefinition {
        name: declaration.name.item.0,
        parent_entity: declaration.parent_type.item.0,
        description: declaration
            .description
            .map(|description| description.item.0),
        kind: match declaration.target_type.as_ref() {
            None => SourceSelectableKind::ClientField,
            Some(target_type) => SourceSelectableKind::ClientPointer(
                wrapper_of_type_annotation(target_type.item.reference())
                    .map_or(ClientPointerTarget::Unparsed, ClientPointerTarget::Parsed),
            ),
        },
        definition: DefinitionLocation {
            path,
            span: declaration.name.location.with_offset(literal_start),
        },
    }
}
