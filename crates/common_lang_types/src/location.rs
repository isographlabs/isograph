use intern::string_key::{Intern, Lookup};
use lazy_static::lazy_static;
use prelude::Postfix;
use std::path::PathBuf;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile, Span};

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

lazy_static! {
    // This is a horrible hack! If this is printed, we presumably blow up.
    pub static ref GENERATED_FILE_DO_NOT_PRINT: TextSource = TextSource {
        relative_path_to_source_file: "generated".intern().into(),
        span: None,
    };
}

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

impl<TItem> From<WithEmbeddedLocation<TItem>> for WithLocation<TItem> {
    fn from(value: WithEmbeddedLocation<TItem>) -> Self {
        WithGenericLocation {
            item: value.item,
            location: value.location.into(),
        }
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, Hash)]
pub struct WithGenericLocation<TItem, TLocation> {
    pub item: TItem,
    pub location: TLocation,
}

impl<T, TLocation> WithGenericLocation<T, TLocation> {
    pub fn new(item: T, location: TLocation) -> Self {
        WithGenericLocation { item, location }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithGenericLocation<U, TLocation>
    where
        TLocation: Copy,
    {
        WithGenericLocation::new(map(self.item), self.location)
    }

    pub fn map_location<U>(self, map: impl FnOnce(TLocation) -> U) -> WithGenericLocation<T, U> {
        WithGenericLocation::new(self.item, map(self.location))
    }

    pub fn and_then<U, E>(
        self,
        map: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<WithGenericLocation<U, TLocation>, E>
    where
        TLocation: Copy,
    {
        WithGenericLocation::new(map(self.item)?, self.location).wrap_ok()
    }

    pub fn as_ref(&self) -> WithGenericLocation<&T, TLocation>
    where
        TLocation: Copy,
    {
        WithGenericLocation {
            location: self.location,
            item: &self.item,
        }
    }

    pub fn drop_location(self) -> WithNoLocation<T> {
        self.map_location(|_| ())
    }

    pub fn item(self) -> T {
        self.item
    }
}

impl<TValue: PartialOrd, TLocation: PartialOrd> PartialOrd
    for WithGenericLocation<TValue, TLocation>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare item first
        match self.item.partial_cmp(&other.item) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.location.partial_cmp(&other.location)
    }
}

pub type WithNoLocation<TItem> = WithGenericLocation<TItem, ()>;

impl<TItem: std::fmt::Display> std::fmt::Display for WithNoLocation<TItem> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.item.fmt(f)
    }
}
