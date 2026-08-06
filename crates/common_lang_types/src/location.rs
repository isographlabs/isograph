use intern::string_key::{Intern, Lookup};
use prelude::Postfix;
use std::path::PathBuf;

pub use span::WithGenericLocation;
use span::Span;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile};

/// A source, which consists of a filename, and an optional span
/// indicating the subset of the file which corresponds to the
/// source.
///
/// TODO consider whether to replace the span with an index,
/// as this will probably mean that sources are more reusable
/// during watch mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub relative_path_to_source_file: RelativePathToSourceFile,
    pub span: Option<Span>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EmbeddedLocation {
    pub text_source: TextSource,
    /// The span is relative to the Source's span, not to the
    /// entire source file.
    pub span: Span,
}

impl From<EmbeddedLocation> for Location {
    fn from(value: EmbeddedLocation) -> Self {
        Location::Embedded(value)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Location {
    Embedded(EmbeddedLocation),
    Generated,
}

impl Location {
    pub fn new(text_source: TextSource, span: Span) -> Self {
        Location::Embedded(EmbeddedLocation::new(text_source, span))
    }

    pub fn as_embedded_location(self) -> Option<EmbeddedLocation> {
        match self {
            Location::Embedded(embedded_location) => embedded_location.wrap_some(),
            Location::Generated => None,
        }
    }
}
impl EmbeddedLocation {
    pub fn new(text_source: TextSource, span: Span) -> Self {
        EmbeddedLocation { text_source, span }
    }
}

pub type WithLocation<TItem> = WithGenericLocation<TItem, Location>;

pub trait WithLocationPostfix
where
    Self: Sized,
{
    fn with_generated_location(self) -> WithLocation<Self> {
        WithLocation::new(self, Location::Generated)
    }

    fn with_no_location(self) -> WithNoLocation<Self> {
        WithGenericLocation::new(self, ())
    }

    fn with_location<TLocation>(self, item: TLocation) -> WithGenericLocation<Self, TLocation> {
        WithGenericLocation::new(self, item)
    }

    fn with_missing_location<TLocation>(self) -> WithGenericLocation<Self, Option<TLocation>> {
        WithGenericLocation::new(self, None)
    }

    fn with_some_location<TLocation>(
        self,
        location: TLocation,
    ) -> WithGenericLocation<Self, Option<TLocation>> {
        WithGenericLocation::new(self, location.wrap_some())
    }
}

pub type WithOptionalLocation<TItem> = WithGenericLocation<TItem, Option<EmbeddedLocation>>;
pub type WithEmbeddedLocation<TItem> = WithGenericLocation<TItem, EmbeddedLocation>;

impl<T> WithLocationPostfix for T {}

pub fn relative_path_from_absolute_and_working_directory(
    current_working_directory: CurrentWorkingDirectory,
    absolute_path: &PathBuf,
) -> RelativePathToSourceFile {
    pathdiff::diff_paths(
        absolute_path,
        PathBuf::from(current_working_directory.lookup()),
    )
    .expect("Expected path to be diffable")
    .to_str()
    .expect("Expected path to be able to be stringified")
    .intern()
    .into()
}

pub type WithNoLocation<TItem> = WithGenericLocation<TItem, ()>;
