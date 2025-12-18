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
///
/// TODO do not include the cwd
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub current_working_directory: CurrentWorkingDirectory,
    pub relative_path_to_source_file: RelativePathToSourceFile,
    pub span: Option<Span>,
}

lazy_static! {
    // This is a horrible hack! If this is printed, we presumably blow up.
    pub static ref GENERATED_FILE_DO_NOT_PRINT: TextSource = TextSource {
        current_working_directory: "".intern().into(),
        relative_path_to_source_file: "generated".intern().into(),
        span: None,
    };
}

const ISO_PRINT_ABSOLUTE_FILEPATH: &str = "ISO_PRINT_ABSOLUTE_FILEPATH";

impl TextSource {
    pub fn read_to_string(&self) -> (String, String) {
        // TODO maybe intern these or somehow avoid reading a bajillion times.
        // This is especially important for when we display many errors.
        let mut file_path = PathBuf::from(self.current_working_directory.lookup());
        let relative_path = self.relative_path_to_source_file.lookup();
        file_path.push(relative_path);

        // HACK
        //
        // When we run pnpm build-pet-demo (etc), then the terminal's working directory is
        // the isograph folder. But the process thinks that the working directory is
        // /demos/pet-demo. As a result, if we print relative paths, we can't command-click
        // on them, leading to a worse developer experience when working on Isograph.
        //
        // On the other hand, printing relative paths (from the current working directory):
        // - is a nice default
        // - means that if we capture that output, e.g. for fixtures, we can have consistent
        //   fixture output, no matter what machine the fixtures were generated on.
        //
        // So, we need both options. This can probably be improved somewhat.
        let absolute_or_relative_file_path = if std::env::var(ISO_PRINT_ABSOLUTE_FILEPATH).is_ok() {
            file_path
                .to_str()
                .expect("Expected path to be able to be stringified.")
                .to_string()
        } else {
            relative_path.to_string()
        };

        let file_contents =
            std::fs::read_to_string(&absolute_or_relative_file_path).expect("file should exist");
        if let Some(span) = self.span {
            // TODO we're cloning here unnecessarily, I think!
            (
                absolute_or_relative_file_path,
                file_contents[span.as_usize_range()].to_string(),
            )
        } else {
            (absolute_or_relative_file_path, file_contents)
        }
    }
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
    pub location: EmbeddedLocation,
    pub item: T,
}

impl<TValue: PartialOrd> PartialOrd for WithEmbeddedLocation<TValue> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare item first
        match self.item.partial_cmp(&other.item) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.location.partial_cmp(&other.location)
    }
}

impl<T> WithEmbeddedLocation<T> {
    pub fn new(item: T, location: EmbeddedLocation) -> Self {
        WithEmbeddedLocation { item, location }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithEmbeddedLocation<U> {
        WithEmbeddedLocation::new(map(self.item), self.location)
    }

    pub fn and_then<U, E>(
        self,
        map: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<WithEmbeddedLocation<U>, E> {
        WithEmbeddedLocation::new(map(self.item)?, self.location).wrap_ok()
    }

    pub fn into_with_location(self) -> WithLocation<T> {
        self.into()
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
            location: Location::Embedded(value.location),
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
