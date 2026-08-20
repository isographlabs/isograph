use prelude::Postfix;
use std::{fmt, ops::Range};

/// A range of byte offsets into source text. The parser stack uses these relative to one
/// literal (see the location model in refactors/past/parser-lang-types.md).
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

    /// Creates a new Span starting at left.end and ending at right.start: the gap
    /// between the two, the counterpart of [`Span::join`].
    pub fn between(left: Span, right: Span) -> Self {
        Span::new(left.end, right.start)
    }

    pub fn contains(&self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

/// One item plus whatever locates it: a `Span` here, `common_lang_types`' location family
/// in the kept chain, `()` for nothing.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WithGenericLocation<TItem, TLocation> {
    pub item: TItem,
    pub location: TLocation,
}

/// An item located by a `Span`.
pub type WithSpan<T> = WithGenericLocation<T, Span>;

/// An item that may have no source span.
pub type WithOptionalSpan<T> = WithGenericLocation<T, Option<Span>>;

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
            item: self.item.reference(),
        }
    }

    pub fn drop_location(self) -> WithGenericLocation<T, ()> {
        self.map_location(|_| ())
    }

    pub fn item(self) -> T {
        self.item
    }
}

impl<TItem: fmt::Display> fmt::Display for WithGenericLocation<TItem, ()> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
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
