use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use thiserror::Error;

use isograph_parser::ParseError;

use crate::{
    ArtifactPath, ClientId, EntityKind, EntityShape, GenerationOptions, IsographDb,
    ProjectArtifacts, ProtocolEntity, ProtocolSelectable, SelectableShape,
};

/// Byte offset of the iso literal text in its file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IsoLiteralStartIndex(pub usize);

pub struct IsoLiteralExtraction<THostLanguage: HostLanguage> {
    pub iso_literal_text: String,
    pub iso_literal_start_index: IsoLiteralStartIndex,
    pub context: THostLanguage::LiteralContext,
}

impl<THostLanguage: HostLanguage> Clone for IsoLiteralExtraction<THostLanguage> {
    fn clone(&self) -> Self {
        Self {
            iso_literal_text: self.iso_literal_text.clone(),
            iso_literal_start_index: self.iso_literal_start_index,
            context: self.context.clone(),
        }
    }
}

impl<THostLanguage: HostLanguage> PartialEq for IsoLiteralExtraction<THostLanguage> {
    fn eq(&self, other: &Self) -> bool {
        self.iso_literal_text == other.iso_literal_text
            && self.iso_literal_start_index == other.iso_literal_start_index
            && self.context == other.context
    }
}

impl<THostLanguage: HostLanguage> Eq for IsoLiteralExtraction<THostLanguage> {}

impl<THostLanguage: HostLanguage> Debug for IsoLiteralExtraction<THostLanguage> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsoLiteralExtraction")
            .field("iso_literal_text", &self.iso_literal_text)
            .field("iso_literal_start_index", &self.iso_literal_start_index)
            .field("context", &self.context)
            .finish()
    }
}

impl<THostLanguage: HostLanguage> IsoLiteralExtraction<THostLanguage> {
    pub fn span(&self) -> span::Span {
        span::Span::from_usize(
            self.iso_literal_start_index.0,
            self.iso_literal_start_index.0 + self.iso_literal_text.len(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncludeFile {
    Include,
    Skip,
}

pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;
    /// A project this host language cannot print.
    type GenerationError: std::fmt::Display + std::error::Error + 'static;
    /// The host-language type a profile gives a custom scalar.
    type CustomScalarType: Clone + Debug + PartialEq + Eq + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;

    /// The path and text of every artifact of `project`.
    fn generate_artifacts(
        project: &ProjectArtifacts<Self::LiteralContext>,
        options: &GenerationOptions<Self::CustomScalarType>,
    ) -> Result<Vec<(ArtifactPath, String)>, Self::GenerationError>;
}

/// One choice of host language and network protocol. A binary names one.
pub trait CompilationProfile: 'static {
    type HostLanguage: HostLanguage;
    type NetworkProtocol: NetworkProtocol;

    /// The host-language type of each custom scalar, by the scalar's name. A custom scalar
    /// not listed has the host language's unknown type.
    fn custom_scalar_types()
    -> BTreeMap<EntityName, <Self::HostLanguage as HostLanguage>::CustomScalarType>;
}

/// The way the compiler communicates with the upstream.
pub trait NetworkProtocol: Sized + 'static {
    /// What the protocol records about an entity it defines.
    type EntityData: Clone + Debug + PartialEq + Eq + 'static;
    /// What the protocol records about a selectable it defines.
    type SelectableData: Clone + Debug + PartialEq + Eq + 'static;
    /// How the protocol fetches an entity that parents an entrypoint.
    type FetchPlan: Clone + Debug + PartialEq + Eq + 'static;
    /// Why an entity defined by the protocol cannot parent an entrypoint.
    type NotFetchable: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;
    /// Why the protocol's lookup of an entity or selectable failed.
    type LookupError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile;

    /// The compiler's kind of an entity the protocol defines.
    fn entity_kind(data: &Self::EntityData) -> EntityKind;

    /// The object types a value of the protocol entity `entity` can be: the entity itself
    /// when it is concrete, its implementors or members when it is abstract.
    fn concrete_types<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        data: &Self::EntityData,
    ) -> Vec<EntityName>;

    /// The entity `name` as the protocol defines it; `Ok(None)` when it defines none.
    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        name: EntityName,
    ) -> Result<Option<ProtocolEntity<Self::EntityData>>, Self::LookupError>;

    /// The selectable `selectable` of `entity` as the protocol defines it; `Ok(None)` when it
    /// defines none.
    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<Self::SelectableData>>, Self::LookupError>;

    /// Every selectable the protocol defines on `entity`, by name. A name may repeat;
    /// `selectables_for_entity` keeps names defined exactly once.
    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<Self::SelectableData>)>;

    /// How `entity`, which the protocol defines, is fetched when it parents an entrypoint.
    fn fetch_plan<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Result<Self::FetchPlan, Self::NotFetchable>;

    /// What the core reads about an entity with `data`: whether a selection of it has
    /// selections, and for one that does, the identity selection the protocol adds to every
    /// selection set on it.
    fn entity_shape(data: &Self::EntityData) -> EntityShape;

    /// What a selection of a selectable with `data` is in a merged selection map.
    fn selectable_shape(data: &Self::SelectableData) -> SelectableShape;

    /// The selectable the protocol selects in every selection set on a composite entity,
    /// whose value names the response object's concrete entity.
    fn discriminator() -> SelectableName;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithErrors<T, E> {
    pub item: T,
    pub errors: E,
}

#[derive(Error)]
pub enum IsoLiteralError<THostLanguage: HostLanguage> {
    #[error("{0}")]
    Host(THostLanguage::Error),
    #[error("{0}")]
    Parse(ParseError),
}

impl<THostLanguage: HostLanguage> Clone for IsoLiteralError<THostLanguage> {
    fn clone(&self) -> Self {
        match self {
            Self::Host(error) => Self::Host(error.clone()),
            Self::Parse(error) => Self::Parse(error.clone()),
        }
    }
}

impl<THostLanguage: HostLanguage> PartialEq for IsoLiteralError<THostLanguage> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Host(left), Self::Host(right)) => left == right,
            (Self::Parse(left), Self::Parse(right)) => left == right,
            _ => false,
        }
    }
}

impl<THostLanguage: HostLanguage> Eq for IsoLiteralError<THostLanguage> {}

impl<THostLanguage: HostLanguage> Debug for IsoLiteralError<THostLanguage> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => f.debug_tuple("Host").field(error).finish(),
            Self::Parse(error) => f.debug_tuple("Parse").field(error).finish(),
        }
    }
}
