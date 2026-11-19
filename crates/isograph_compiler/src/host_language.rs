use std::collections::BTreeSet;
use std::fmt::{self, Debug};
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use thiserror::Error;

use isograph_parser::ParseError;

use crate::{
    ArtifactPath, ArtifactSource, ClientId, IsographDb, ProtocolEntity, ProtocolSelectable,
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

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;

    /// The path and text of every artifact of a project whose iso literals are `sources`.
    fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<(ArtifactPath, String)>;
}

/// One choice of host language and network protocol. A binary names one.
pub trait CompilationProfile: 'static {
    type HostLanguage: HostLanguage;
    type NetworkProtocol: NetworkProtocol;
}

/// The way the compiler communicates with the upstream.
pub trait NetworkProtocol: Sized + 'static {
    /// What the protocol records about an entity it defines.
    type EntityData: Clone + Debug + PartialEq + Eq + 'static;
    /// What the protocol records about a selectable it defines.
    type SelectableData: Clone + Debug + PartialEq + Eq + 'static;
    /// Why the protocol's lookup of an entity or selectable failed.
    type LookupError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile;

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
