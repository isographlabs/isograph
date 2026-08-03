# parser_lang_types: Span without Location

The new parser stack gets its own lang-types crate, `parser_lang_types`, copied from `common_lang_types` but holding only what parsing needs: `Span` and `WithSpan`. `common_lang_types` itself is untouched; it continues to serve the kept swc chain, and no new crate imports it.

The changes here land before `resilient-parser.md`'s Change 1, which consumes this crate.

## The location model

- A parse operates within one literal, so a `Span` (byte offsets relative to the literal) is the only location a parse tree node can require. No type in `parser_lang_types` or `isograph_parser` names a file.
- File identity is held once, at the top level, beside the literal: the extraction stage (its own doc) hands the parser a `&str` and keeps `(file, offset of the literal in the file)` next to the result. A file-absolute span is computed at that level as `span.with_offset(offset_in_file)`, at the moment a consumer (a diagnostic, an LSP response) needs one — never stored in the tree.
- This is a fact about parsing, not a global rule: stages that relate multiple files to each other own their own file-qualified location types when they arrive, and those types live with those stages, not in `parser_lang_types`.
- Upstream's `Location` / `EmbeddedLocation` / `TextSource` / `WithLocation` family is not copied. Its shape — every located item carrying the interned file path plus an `Option<Span>` for the literal's position — is the per-node version of what the top level already knows once.

## No pico in the parser

The parser does not use pico. The explicit assumption is that parsing a literal is trivially cheap — a single pass over a few hundred bytes — and not worth memoizing. Memoization applies above the parser (which files changed, which literals were extracted), where pico already lives; `isograph_parser` and `parser_lang_types` are plain functions over `&str` with no database parameter, no interning, and no dependency on pico. This keeps every parser function directly callable from a test with nothing constructed around it.

## Change 1: the `parser_lang_types` crate

`crates/parser_lang_types/Cargo.toml`:

```toml
[package]
name = "parser_lang_types"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]

[lints]
workspace = true
```

`crates/parser_lang_types/src/lib.rs` is `common_lang_types/src/span.rs` copied, with three departures: the `prelude` dependency is dropped (`and_then` returns `Ok(...)` directly), the `Location` conversions are dropped (`to_with_location`, `to_with_embedded_location`), and the generated-span escape hatches are dropped (`todo_generated`, `with_generated_span` — every span the parser produces comes from text it just scanned, so a made-up span is unrepresentable rather than discouraged).

```rust
use std::{fmt, ops::Range};

/// A range of byte offsets into one isograph literal.
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
```

The `u32::try_from(...).unwrap()` calls in `from_usize` are copied as-is; a literal longer than `u32::MAX` bytes is out of scope for this doc.

## Change 2: `resolve_position` moves onto `parser_lang_types`

`resolve_position` uses only `Span` and `WithSpan`, so it repoints and nothing else about it changes.

`crates/resolve_position/Cargo.toml`, before:

```toml
[dependencies]
common_lang_types = { path = "../common_lang_types" }
```

After:

```toml
[dependencies]
parser_lang_types = { path = "../parser_lang_types" }
```

`crates/resolve_position/src/lib.rs`, before:

```rust
use common_lang_types::Span;
```

After:

```rust
use parser_lang_types::Span;
```

The test module's `use common_lang_types::{Span, WithSpan};` becomes `use parser_lang_types::{Span, WithSpan};`.

## Change 3: the parser and tests crates depend on `parser_lang_types`

`crates/isograph_parser/Cargo.toml`, before:

```toml
[dependencies]
common_lang_types = { path = "../common_lang_types" }
resolve_position = { path = "../resolve_position" }
```

After:

```toml
[dependencies]
parser_lang_types = { path = "../parser_lang_types" }
resolve_position = { path = "../resolve_position" }
```

`crates/tests/Cargo.toml` changes the same way. Every `use common_lang_types::...` in `resilient-parser.md`'s snippets reads `use parser_lang_types::...`; that doc is written against this one.
