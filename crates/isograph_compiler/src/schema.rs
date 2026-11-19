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

/// An entity the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolEntity<TEntityData> {
    pub definition: DefinitionLocation,
    pub data: TEntityData,
}

/// A selectable the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolSelectable<TSelectableData> {
    pub description: Option<DescriptionValue>,
    pub target: Wrapper<EntityName>,
    pub definition: DefinitionLocation,
    pub data: TSelectableData,
}

impl<TSelectableData> ProtocolSelectable<TSelectableData> {
    fn into_selectable(
        self,
        parent_entity: EntityName,
        name: SelectableName,
    ) -> Selectable<TSelectableData> {
        Selectable {
            name,
            parent_entity,
            description: self.description,
            definition: self.definition,
            kind: SelectableKind::Server(ServerSelectable {
                target: self.target,
                data: self.data,
            }),
        }
    }
}

pub type ProfileLookupError<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::LookupError;

/// What the source files and the protocol define under one name.
pub(crate) enum Definitions<T, TLookupError> {
    Undefined,
    Unique(T),
    /// More than one definition among the source declarations and the protocol.
    Duplicated,
    /// The protocol's lookup failed and no source declaration has the name.
    ProtocolLookupFailed(TLookupError),
}

impl<T, TLookupError> Definitions<T, TLookupError> {
    /// `source` chained with the protocol's definition. A protocol `Err` counts as one
    /// definition.
    fn count(source: impl Iterator<Item = T>, protocol: Result<Option<T>, TLookupError>) -> Self {
        let mut source = source.peekable();
        match (source.next(), source.peek().is_some(), protocol) {
            (None, _, Ok(None)) => Definitions::Undefined,
            (None, _, Ok(Some(definition))) | (Some(definition), false, Ok(None)) => {
                Definitions::Unique(definition)
            }
            (None, _, Err(error)) => Definitions::ProtocolLookupFailed(error),
            (Some(_), true, _) | (Some(_), false, Ok(Some(_)) | Err(_)) => Definitions::Duplicated,
        }
    }
}

/// The definitions of the entity named `entity_name`.
pub(crate) fn entity_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> Definitions<ProfileEntity<TProfile>, ProfileLookupError<TProfile>> {
    Definitions::count(
        source_entity_definitions(db, client, entity_name)
            .iter()
            .map(|definition| Entity {
                name: entity_name,
                definition: definition.dereference(),
                origin: EntityOrigin::Source,
            }),
        TProfile::NetworkProtocol::entity(db, client, entity_name).map(|protocol| {
            protocol.map(|protocol| Entity {
                name: entity_name,
                definition: protocol.definition,
                origin: EntityOrigin::Protocol(protocol.data),
            })
        }),
    )
}

/// The definitions of the selectable named `selectable_name` on the entity named
/// `entity_name`.
pub(crate) fn selectable_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Definitions<ProfileSelectable<TProfile>, ProfileLookupError<TProfile>> {
    Definitions::count(
        source_selectable_definitions(db, client, entity_name, selectable_name)
            .iter()
            .map(SourceSelectableDefinition::selectable),
        TProfile::NetworkProtocol::selectable(db, client, entity_name, selectable_name).map(
            |protocol| {
                protocol.map(|protocol| protocol.into_selectable(entity_name, selectable_name))
            },
        ),
    )
}

/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<ProfileEntity<TProfile>> {
    match entity_definitions(db, client, entity_name) {
        Definitions::Unique(entity) => entity.wrap_ok(),
        Definitions::Undefined => {
            Diagnostic::new(format!("Entity `{entity_name}` is not defined."), None).wrap_err()
        }
        Definitions::Duplicated => Diagnostic::new(
            format!("Entity `{entity_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
        Definitions::ProtocolLookupFailed(error) => {
            Diagnostic::new(error.to_string(), None).wrap_err()
        }
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
    match selectable_definitions(db, client, entity_name, selectable_name) {
        Definitions::Unique(selectable) => selectable.wrap_ok(),
        Definitions::Undefined => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is not defined."),
            None,
        )
        .wrap_err(),
        Definitions::Duplicated => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
        Definitions::ProtocolLookupFailed(error) => {
            Diagnostic::new(error.to_string(), None).wrap_err()
        }
    }
}

/// Every selectable on the entity named `entity_name` that exactly one source file
/// declaration or protocol definition defines, by name.
#[memo]
pub fn selectables_for_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Vec<ProfileSelectable<TProfile>>> {
    entity_named(db, client, entity_name).clone()?;
    let source =
        source_iso_literal_items(db, client).filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => {
                (declaration.parent_type.item.0 == entity_name).then(|| {
                    selectable_of_declaration(
                        source_item.path,
                        source_item.literal_start,
                        declaration,
                    )
                    .selectable()
                })
            }
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        });
    let protocol = TProfile::NetworkProtocol::selectables(db, client, entity_name)
        .into_iter()
        .map(|(name, protocol)| protocol.into_selectable(entity_name, name));
    source
        .chain(protocol)
        .fold(
            BTreeMap::<SelectableName, Vec<ProfileSelectable<TProfile>>>::new(),
            |mut by_name, selectable| {
                by_name.entry(selectable.name).or_default().push(selectable);
                by_name
            },
        )
        .into_values()
        .filter_map(|definitions| {
            <[ProfileSelectable<TProfile>; 1]>::try_from(definitions)
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
