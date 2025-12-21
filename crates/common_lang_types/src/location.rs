use intern::string_key::{Intern, Lookup};
use prelude::Postfix;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile, Span};

/// A source, which consists of a filename, and an optional span
/// indicating the subset of the file which corresponds to the
/// source.
///
/// TODO consider whether to replace the span with an index,
/// as this will probably mean that sources are more reusable
/// during watch mode.
///
/// TODO do not include the cwd
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub relative_path_to_source_file: RelativePathToSourceFile,
    pub span: Option<Span>,
}

// This is a horrible hack! If this is printed, we presumably blow up.
pub static GENERATED_FILE_DO_NOT_PRINT: LazyLock<TextSource> = LazyLock::new(|| TextSource {
    relative_path_to_source_file: "generated".intern().into(),
    span: None,
});

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EmbeddedLocation {
    pub text_source: TextSource,
    /// The span is relative to the Source's span, not to the
    /// entire source file.
    pub span: Span,
}

impl EmbeddedLocation {
    /// This function will give us an embedded location that will probably cause
    /// a panic if printed! It's use is indicative that we need to refactor somehow.
    pub fn todo_generated() -> EmbeddedLocation {
        EmbeddedLocation::new(*GENERATED_FILE_DO_NOT_PRINT, Span::todo_generated())
    }
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

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord)]
pub struct WithLocation<T> {
    pub location: Location,
    pub item: T,
}

impl<TValue: PartialOrd> PartialOrd for WithLocation<TValue> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare item first
        match self.item.partial_cmp(&other.item) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.location.partial_cmp(&other.location)
    }
}

pub trait WithLocationPostfix
where
    Self: Sized,
{
    fn with_location(self, location: Location) -> WithLocation<Self> {
        WithLocation::new(self, location)
    }

    fn with_generated_location(self) -> WithLocation<Self> {
        WithLocation::new(self, Location::Generated)
    }

    fn with_embedded_location(self, location: EmbeddedLocation) -> WithEmbeddedLocation<Self> {
        WithEmbeddedLocation::new(self, location)
    }
}

impl<T> WithLocationPostfix for T {}

impl<T> WithLocation<T> {
    pub fn new(item: T, location: Location) -> Self {
        WithLocation { item, location }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord)]
pub struct WithEmbeddedLocation<T> {
    pub embedded_location: EmbeddedLocation,
    pub item: T,
}

impl<TValue: PartialOrd> PartialOrd for WithEmbeddedLocation<TValue> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare item first
        match self.item.partial_cmp(&other.item) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.embedded_location.partial_cmp(&other.embedded_location)
    }
}

impl<T> WithEmbeddedLocation<T> {
    pub fn new(item: T, location: EmbeddedLocation) -> Self {
        WithEmbeddedLocation {
            item,
            embedded_location: location,
        }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithEmbeddedLocation<U> {
        WithEmbeddedLocation::new(map(self.item), self.embedded_location)
    }

    pub fn and_then<U, E>(
        self,
        map: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<WithEmbeddedLocation<U>, E> {
        WithEmbeddedLocation::new(map(self.item)?, self.embedded_location).wrap_ok()
    }

    pub fn into_with_location(self) -> WithLocation<T> {
        self.into()
    }

    pub fn as_ref(&self) -> WithEmbeddedLocation<&T> {
        WithEmbeddedLocation {
            embedded_location: self.embedded_location,
            item: &self.item,
        }
    }
}

pub trait WithEmbeddedLocationPostfix
where
    Self: Sized,
{
    fn with_embedded_location(
        self,
        embedded_location: EmbeddedLocation,
    ) -> WithEmbeddedLocation<Self> {
        WithEmbeddedLocation::new(self, embedded_location)
    }
}

impl<T> From<WithEmbeddedLocation<T>> for WithLocation<T> {
    fn from(value: WithEmbeddedLocation<T>) -> Self {
        WithLocation {
            location: Location::Embedded(value.embedded_location),
            item: value.item,
        }
    }
}

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
