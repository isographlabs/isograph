use std::{error::Error, fmt};

use intern::{string_key::Intern, Lookup};

use crate::{text_with_carats::text_with_carats, SourceFileName, Span, WithSpan};

/// A source, which consists of a filename, and an optional span
/// indicating the subset of the file which corresponds to the
/// source.
///
/// TODO consider whether to replace the span with an index,
/// as this will probably mean that sources are more reusable
/// during watch mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextSource {
    pub path: SourceFileName,
    pub span: Option<Span>,
}

impl TextSource {
    pub fn todo_generated() -> TextSource {
        TextSource {
            path: "generated-file".intern().into(),
            span: None,
        }
    }
}

impl TextSource {
    pub fn read_to_string(&self) -> (&str, String) {
        // TODO maybe intern these or somehow avoid reading a bajillion times.
        // This is especially important for when we display many errors.
        let file_path = self.path.lookup();
        let file_contents = std::fs::read_to_string(&file_path).expect("file should exist");
        if let Some(span) = self.span {
            // TODO we're cloning here unnecessarily, I think!
            (file_path, file_contents[span.as_usize_range()].to_string())
        } else {
            (file_path, file_contents)
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Location {
    Embedded {
        text_source: TextSource,
        /// The span is relative to the Source's span, not to the
        /// entire source file.
        span: Span,
    },
    Generated,
}

impl Location {
    pub fn generated() -> Self {
        Location::Generated
    }

    pub fn new(text_source: TextSource, span: Span) -> Self {
        Location::Embedded { text_source, span }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Embedded { text_source, span } => {
                let (file_path, read_out_text) = text_source.read_to_string();
                let text_with_carats = text_with_carats(&read_out_text, *span);

                write!(f, "{}\n{}", file_path, text_with_carats)
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

    /// This method should not be called. It exists because in some places,
    /// we have locations where we want spans, which needs to be fixed with
    /// refactoring.
    pub fn hack_to_with_span(self) -> WithSpan<T> {
        let span = match self.location {
            Location::Embedded { span, .. } => span,
            Location::Generated => Span::todo_generated(),
        };
        WithSpan {
            item: self.item,
            span,
        }
    }
}
