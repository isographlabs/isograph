use std::ops::Range;

use intern::string_key::Intern;
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    // Whole file. A Span plus `cursor.text()` at the call site copies 16 fewer bytes per consume.
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn text(self) -> &'a str {
        &self.text[self.location.as_usize_range()]
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn exclude_ends(self, n: u32) -> TokenText<'a> {
        TokenText {
            location: Span::new(self.location.start + n, self.location.end - n),
            text: self.text,
        }
    }

    #[expect(dead_code)]
    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T> {
        self.text().intern().to::<T>().with_span(self.location)
    }

    /// A byte range inside `text()` as a span in the file.
    pub(crate) fn sub_span(self, range: Range<usize>) -> Span {
        Span::new(
            self.location.start + range.start as u32,
            self.location.start + range.end as u32,
        )
    }
}

#[cfg(test)]
impl<'a> TokenText<'a> {
    pub(crate) fn new(location: Span, text: &'a str) -> Self {
        TokenText { location, text }
    }
}
