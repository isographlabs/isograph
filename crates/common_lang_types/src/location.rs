use std::fmt;

use intern::Lookup;

use crate::{text_with_carats::text_with_carats, SourceFileName, Span};

/// The location of a source.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SourceLocationKey {
    /// A source embedded within a file. The index is the starting character
    /// of the embedded source within the file.
    Embedded {
        // TODO include a span here
        path: SourceFileName,
        start_index: usize,
        len: usize,
    },
    Generated,
}

impl SourceLocationKey {
    /// Returns a `SourceLocationKey` that's not backed by a real file. In most
    /// cases it's preferred to use a related real file.
    pub fn generated() -> Self {
        SourceLocationKey::Generated
    }

    pub fn path(self) -> &'static str {
        match self {
            SourceLocationKey::Embedded { path, .. } => path.lookup(),
            SourceLocationKey::Generated => "<generated>",
        }
    }
}

/// An absolute source location describing both the file and position (span)
/// with that file.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Location {
    pub source_location: SourceLocationKey,

    /// Relative position with the file
    pub span: Span,
}

impl Location {
    pub fn generated() -> Self {
        Location {
            source_location: SourceLocationKey::generated(),
            span: Span::todo_generated(),
        }
    }

    pub fn new(source_location: SourceLocationKey, span: Span) -> Self {
        Location {
            source_location,
            span,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.source_location {
            SourceLocationKey::Embedded {
                path,
                start_index,
                len,
            } => {
                let span = self.span;
                let file_path = path.lookup();
                let file_contents = std::fs::read_to_string(&file_path).expect("file should exist");

                let path_text = &file_contents[start_index..start_index + len];
                let text_with_carats = text_with_carats(path_text, span);

                write!(f, "{}\n{}", file_path, text_with_carats)
            }
            SourceLocationKey::Generated => write!(f, "<generated>"),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct WithLocation<T> {
    pub location: Location,
    pub item: T,
}

impl<T: fmt::Display> fmt::Display for WithLocation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{}", self.item, self.location)
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
}
