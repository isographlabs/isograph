# parser_lang_types: Span without Location

The new parser stack gets its own lang-types crate, `parser_lang_types`, holding `Span` and `WithSpan`, which move out of `common_lang_types`. `common_lang_types` keeps the `Location` family and everything else, and depends on `parser_lang_types` for the `Span` its location and carat-printing types embed. The swc plugin itself uses `swc_common`'s `Span` and never imports these types, so the kept chain changes only inside `common_lang_types`. No new crate imports `common_lang_types`.

The changes here land before `resilient-parser.md`'s Change 1, which consumes this crate.

## The location model

- A parse operates within one literal, so a `Span` (byte offsets relative to the literal) is the only location a parse tree node can require. No type in `parser_lang_types` or `isograph_parser` names a file.
- File identity is held once, at the top level, beside the literal: the extraction stage (its own doc) hands the parser a `&str` and keeps `(file, offset of the literal in the file)` next to the result. A file-absolute span is computed at that level as `span.with_offset(offset_in_file)`, at the moment a consumer (a diagnostic, an LSP response) needs one — never stored in the tree.
- This is a fact about parsing, not a global rule: stages that relate multiple files to each other own their own file-qualified location types when they arrive, and those types live with those stages, not in `parser_lang_types`.
- The `Location` / `EmbeddedLocation` / `TextSource` / `WithLocation` family stays in `common_lang_types` and is never imported by the parser stack. Its shape — every located item carrying the interned file path plus an `Option<Span>` for the literal's position — is the per-node version of what the top level already knows once.

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

`crates/parser_lang_types/src/lib.rs` is `common_lang_types/src/span.rs` moved, with three deletions, each of something with no callers in this repo:

- the `prelude` dependency (`and_then` returns `Ok(...)` directly);
- the `Location` conversions (`to_with_location`, `to_with_embedded_location`);
- the generated-span escape hatch `todo_generated`, whose only caller was `EmbeddedLocation::todo_generated`, itself uncalled — every span the parser produces comes from text it just scanned, so a made-up span is unrepresentable rather than discouraged. (Change 2 deletes that caller.)

```rust
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
```

The `u32::try_from(...).unwrap()` calls in `from_usize` are copied as-is; a literal longer than `u32::MAX` bytes is out of scope for this doc.

## Change 2: `common_lang_types` moves off its own span

`crates/common_lang_types/src/span.rs` is deleted. The crate depends on `parser_lang_types` for the `Span` that `location.rs` and `text_with_carats.rs` embed, and does not re-export it: once Change 3 lands, nothing consumes `common_lang_types::Span`.

`crates/common_lang_types/Cargo.toml`, `[dependencies]`, before:

```toml
intern = { path = "../../relay-crates/intern" }
string_key_newtype = { path = "../string_key_newtype" }
```

After:

```toml
intern = { path = "../../relay-crates/intern" }
parser_lang_types = { path = "../parser_lang_types" }
string_key_newtype = { path = "../string_key_newtype" }
```

`src/lib.rs`, before:

```rust
mod span;
...
pub use span::*;
```

After: both lines deleted.

`src/location.rs`: the import repoints, and `EmbeddedLocation::todo_generated` goes, taking `GENERATED_FILE_DO_NOT_PRINT` (its only reason to exist) and the now-unused `lazy_static` import with it. Before:

```rust
use intern::string_key::{Intern, Lookup};
use lazy_static::lazy_static;
use prelude::Postfix;
use std::path::PathBuf;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile, Span};
```

```rust
lazy_static! {
    // This is a horrible hack! If this is printed, we presumably blow up.
    pub static ref GENERATED_FILE_DO_NOT_PRINT: TextSource = TextSource {
        relative_path_to_source_file: "generated".intern().into(),
        span: None,
    };
}
```

```rust
impl EmbeddedLocation {
    /// This function will give us an embedded location that will probably cause
    /// a panic if printed! It's use is indicative that we need to refactor somehow.
    pub fn todo_generated() -> EmbeddedLocation {
        EmbeddedLocation::new(*GENERATED_FILE_DO_NOT_PRINT, Span::todo_generated())
    }
}
```

After: the import block reads

```rust
use intern::string_key::{Intern, Lookup};
use prelude::Postfix;
use std::path::PathBuf;

use parser_lang_types::Span;

use crate::{CurrentWorkingDirectory, RelativePathToSourceFile};
```

and the `lazy_static` block and the `todo_generated` impl block are deleted. (`Intern` stays: `relative_path_from_absolute_and_working_directory` interns.)

`src/text_with_carats.rs`, before:

```rust
use crate::Span;
```

After:

```rust
use parser_lang_types::Span;
```

## Change 3: `resolve_position` moves onto `parser_lang_types`

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

## Change 4: the parser and tests crates depend on `parser_lang_types`

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

`crates/tests/Cargo.toml` changes the same way. Every `use parser_lang_types::...` in `resilient-parser.md`'s snippets reads against this crate; that doc is written against this one.
