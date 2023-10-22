use std::fmt;

// Invariant: end >= start
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

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Span::from_usize(range.start, range.end)
    }
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Span { start, end }
    }

    pub fn todo_generated() -> Self {
        // Calling this indicates we have no actual span, which is indicative of
        // poor modeling.
        Span::new(0, 0)
    }

    pub fn with_offset(self, offset: u32) -> Self {
        Self::new(self.start + offset, self.end + offset)
    }

    pub fn from_usize(start: usize, end: usize) -> Self {
        Self::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap())
    }

    pub fn as_usize(self) -> (usize, usize) {
        (self.start as usize, self.end as usize)
    }

    pub fn join(left: Span, right: Span) -> Self {
        Span::new(left.start, right.end)
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
}

impl<T, E> WithSpan<Result<T, E>> {
    pub fn transpose(self) -> Result<WithSpan<T>, E> {
        Ok(WithSpan::new(self.item?, self.span))
    }
}

impl<T: fmt::Display> fmt::Display for WithSpan<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}
