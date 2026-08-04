use std::{fmt, ops::Range};

/// A range of byte offsets into source text. The parser stack uses these relative to one
/// literal (see the location model in parser-lang-types.md).
/// Invariant: end >= start.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start, self.end)
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Span::from_usize(range.start, range.end)
    }
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        debug_assert!(
            start <= end,
            "span.start ({start}) should be less than or \
            equal to span.end ({end})"
        );
        Span { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Rebase a literal-relative span to a file-absolute one, at the top level that
    /// holds the literal's offset in its file.
    pub fn with_offset(self, offset: u32) -> Self {
        Self::new(self.start + offset, self.end + offset)
    }

    pub fn from_usize(start: usize, end: usize) -> Self {
        Self::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap())
    }

    pub fn as_usize(self) -> (usize, usize) {
        (self.start as usize, self.end as usize)
    }

    /// Creates a new Span starting at left.start and ending at right.end
    pub fn join(left: Span, right: Span) -> Self {
        Span::new(left.start, right.end)
    }

    pub fn as_usize_range(&self) -> Range<usize> {
        (self.start as usize)..(self.end as usize)
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn span_between(&self, other: Span) -> Span {
        Span {
            start: self.end,
            end: other.start,
        }
    }

    pub fn contains(&self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WithSpan<T> {
    pub item: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn new(item: T, span: Span) -> Self {
        WithSpan { item, span }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> WithSpan<U> {
        WithSpan::new(map(self.item), self.span)
    }

    pub fn and_then<U, E>(self, map: impl FnOnce(T) -> Result<U, E>) -> Result<WithSpan<U>, E> {
        Ok(WithSpan::new(map(self.item)?, self.span))
    }

    pub fn as_ref(&self) -> WithSpan<&T> {
        WithSpan {
            item: &self.item,
            span: self.span,
        }
    }
}

pub trait WithSpanPostfix
where
    Self: Sized,
{
    fn with_span(self, span: Span) -> WithSpan<Self> {
        WithSpan::new(self, span)
    }
}

impl<T> WithSpanPostfix for T {}
