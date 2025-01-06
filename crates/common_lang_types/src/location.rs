use std::{error::Error, fmt, path::PathBuf};

use intern::string_key::{Intern, Lookup};

use crate::{
    text_with_carats::text_with_carats, CurrentWorkingDirectory, RelativePathToSourceFile, Span,
    WithSpan,
};

/// A source, which consists of a path from the config's project root
/// to the source file, and an optional span indicating the subset of
/// the file which corresponds to the source.
///
/// TODO consider whether to replace the span with an index,
/// as this will probably mean that sources are more reusable
/// during watch mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub current_working_directory: CurrentWorkingDirectory,
    pub relative_path_to_source_file: RelativePathToSourceFile,
    pub span: Option<Span>,
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
pub struct EmbeddedRelativeLocation {
    pub text_source: RelativeTextSource,
    /// The span is relative to the Source's span, not to the
    /// entire source file.
    pub span: Span,
}

/// What is happening here? EmbeddedRelativeLocation's only contain
/// a path that is relative to the Isograph config's project_root
/// field. Displaying the EmbeddedRelativeLocation involves reading the
/// file and displaying a subset of it.
///
/// The AbsoluteEmbeddedLocation struct knows how to turn that relative
/// path to an absolute path (i.e. it contains the absolute path to
/// the project_root) for use when reading the file.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AbsoluteEmbeddedLocation {
    pub embedded_location: EmbeddedRelativeLocation,
}

impl std::fmt::Display for AbsoluteEmbeddedLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (file_path, read_out_text) = self.embedded_location.text_source.read_to_string();
        let text_with_carats = text_with_carats(&read_out_text, self.embedded_location.span);

        write!(f, "{}\n{}", file_path, text_with_carats)
    }
}

impl From<EmbeddedRelativeLocation> for Location {
    fn from(value: EmbeddedRelativeLocation) -> Self {
        Location::Embedded(value)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Location {
    Embedded(EmbeddedRelativeLocation),
    Generated,
}

impl Location {
    pub fn generated() -> Self {
        Location::Generated
    }
    pub fn new(text_source: RelativeTextSource, span: Span) -> Self {
        Location::Embedded(EmbeddedRelativeLocation::new(text_source, span))
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Location::Embedded(embedded) => Some(embedded.span),
            Location::Generated => None,
        }
    }
}
impl EmbeddedRelativeLocation {
    pub fn new(text_source: RelativeTextSource, span: Span) -> Self {
        EmbeddedRelativeLocation { text_source, span }
    }
}

struct AbsoluteLocation {
    pub location: Location,
}

impl fmt::Display for AbsoluteLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.location {
            Location::Embedded(embedded_location) => {
                let wrapper = AbsoluteEmbeddedLocation { embedded_location };
                wrapper.fmt(f)
            }
            Location::Generated => {
                write!(f, "<generated>")
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct WithLocation<T> {
    pub location: Location,
    pub item: T,
}

impl<T: Error> Error for WithLocation<T> {
    fn description(&self) -> &str {
        #[allow(deprecated)]
        self.item.description()
    }
}

impl<T: fmt::Display> fmt::Display for WithLocation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let absolute_location = AbsoluteLocation {
            location: self.location,
        };
        write!(f, "{}\n{}", self.item, absolute_location)
    }
}

impl<T> WithLocation<T> {
    pub fn new(item: T, location: Location) -> Self {
        WithLocation { item, location }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithLocation<U> {
        WithLocation::new(map(self.item), self.location)
    }

    pub fn and_then<U, E>(self, map: impl FnOnce(T) -> Result<U, E>) -> Result<WithLocation<U>, E> {
        Ok(WithLocation::new(map(self.item)?, self.location))
    }

    /// This method should not be called. It exists because in some places,
    /// we have locations where we want spans, which needs to be fixed with
    /// refactoring. We can probably instead enforce that the type has a
    /// EmbeddedLocation or WithEmbeddedLocation
    pub fn hack_to_with_span(self) -> WithSpan<T> {
        let span = match self.location {
            Location::Embedded(EmbeddedRelativeLocation { span, .. }) => span,
            Location::Generated => Span::todo_generated(),
        };
        WithSpan {
            item: self.item,
            span,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct WithEmbeddedRelativeLocation<T> {
    pub location: EmbeddedRelativeLocation,
    pub item: T,
}

impl<T> WithEmbeddedRelativeLocation<T> {
    pub fn new(item: T, location: EmbeddedRelativeLocation) -> Self {
        WithEmbeddedRelativeLocation { item, location }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithEmbeddedRelativeLocation<U> {
        WithEmbeddedRelativeLocation::new(map(self.item), self.location)
    }

    pub fn and_then<U, E>(
        self,
        map: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<WithEmbeddedRelativeLocation<U>, E> {
        Ok(WithEmbeddedRelativeLocation::new(
            map(self.item)?,
            self.location,
        ))
    }

    pub fn into_with_location(self) -> WithLocation<T> {
        self.into()
    }
}

impl<T> From<WithEmbeddedRelativeLocation<T>> for WithLocation<T> {
    fn from(value: WithEmbeddedRelativeLocation<T>) -> Self {
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
