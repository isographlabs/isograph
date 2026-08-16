# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

First doc of the series parsing-plan.md orders, written against parsing-standards.md. This doc lands `ItemCursor` / `ChunkStream` (`new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `expected`, `text`, `token_text`, `end_span`), `Chunk::stream`, `boundary_comma`, `ChunkedLevel::len`, `parse_singleton`, `parse_iso_literal`, `ParseError`, and `entrypoint Type.field`. `field` and `pointer` are identifiers that return `UnsupportedDeclarationType`; parse-fields.md and parse-pointers.md replace those arms.

## The grammar

```
entrypoint <Identifier> . <Identifier>
```

The root is a one-item context, not a list. Chunk 0 goes through `parse_one_item` (`Complete` / `Both` / `Failed`). Remaining chunks are `ExtraChunks` plus `MultipleDeclarations` on the first extra chunk. A boundary comma is a tokenless diagnostic. Empty is `EmptyLiteral` and no first slot. `item()` is `Some` when chunk 0 is `Complete` or `Both`.

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

A failed first chunk is `RootSlot::Failed` (that chunk’s items). Extra chunks still sit in `ExtraChunks`.

## Types

Most important first. The wrapping `WithSpan` on `IsoLiteralParse` is the whole literal.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::chunk_stream::ItemCursor;
use crate::{
    ChunkedLevel, Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError,
    parse_singleton,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub struct IsoLiteralParse {
    // resolve-position-generic-slot.md: Option<WithSpan<LevelSlot<IsoLiteralItem>>>.
    #[resolve_field]
    pub first: Option<WithSpan<RootSlot>>,
    #[resolve_field]
    pub extra: Option<ExtraChunks>,
}

/// Derived stand-in for `LevelSlot<IsoLiteralItem>`. resolve-position-generic-slot.md.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum RootSlot {
    Complete(#[resolve_field(parent_variant = Complete)] WithSpan<IsoLiteralItem>),
    Both(BothRoot),
    Failed(#[resolve_field(parent_variant = Failed)] Failed),
}

/// Derived stand-in for `Both<IsoLiteralItem>`. resolve-position-generic-slot.md.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BothRoot {
    #[resolve_field(parent_variant = Both)]
    pub item: WithSpan<IsoLiteralItem>,
    #[resolve_field(parent_variant = Both)]
    pub failed: WithSpan<Failed>,
}

#[derive(Debug)]
pub enum FailedParent<'a> {
    Both(BothRootPath<'a>),
    Failed(IsoLiteralParsePath<'a>),
}

#[derive(Debug)]
pub enum IsoLiteralItemParent<'a> {
    Complete(IsoLiteralParsePath<'a>),
    Both(BothRootPath<'a>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralItemPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    pub entrypoint_keyword: WithSpan<EntrypointKeyword>,
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName;

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntrypointKeyword;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;

pub type RootSlotPath<'a> = PositionResolutionPath<&'a RootSlot, IsoLiteralParsePath<'a>>;

pub type BothRootPath<'a> = PositionResolutionPath<&'a BothRoot, IsoLiteralParsePath<'a>>;

pub type IsoLiteralItemPath<'a> = PositionResolutionPath<&'a IsoLiteralItem, IsoLiteralItemParent<'a>>;

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralItemPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

pub type FailedPath<'a> = PositionResolutionPath<&'a Failed, FailedParent<'a>>;

pub type UnparsedChunkItemsPath<'a> = PositionResolutionPath<&'a UnparsedChunkItems, FailedPath<'a>>;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> = PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl IsoLiteralParse {
    pub fn item(&self) -> Option<&EntrypointDeclaration> {
        let first = self.first.as_ref()?;
        let item = match first.item.reference() {
            RootSlot::Complete(item) => item.item.reference(),
            RootSlot::Both(both) => both.item.item.reference(),
            RootSlot::Failed(_) => return None,
        };
        match item {
            IsoLiteralItem::Entrypoint(declaration) => declaration.wrap_some(),
        }
    }
}

// resolve-position-generic-slot.md: this From is gone.
impl From<LevelSlot<IsoLiteralItem>> for RootSlot {
    fn from(slot: LevelSlot<IsoLiteralItem>) -> Self {
        match slot {
            LevelSlot::Complete(item) => RootSlot::Complete(item),
            LevelSlot::Both(both) => RootSlot::Both(BothRoot {
                item: both.item,
                failed: both.failed,
            }),
            LevelSlot::Failed(failed) => RootSlot::Failed(failed),
        }
    }
}
```

## The parser

The root is borrowed until the end. A failed first chunk clones that chunk's items into `Failed`. Extra chunks after the first are moved into `ExtraChunks`. On `Complete` with no extra the root `ChunkedLevel` is dropped. Diagnostics go through `push_error`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    push_error: impl FnMut(WithSpan<ParseError>),
) -> WithSpan<IsoLiteralParse> {
    /* parsing-standards.md */
}

fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        text if text == "entrypoint" => {
            IsoLiteralItem::Entrypoint(parse_entrypoint(keyword, cursor)?).wrap_ok()
        }
        text if text == "field" || text == "pointer" => {
            WithSpan::new(ParseError::UnsupportedDeclarationType, keyword).wrap_err()
        }
        _ => WithSpan::new(
            ParseError::expected(
                Expectation::DeclarationKeyword,
                Found::Token(NonBracketTokenKind::Identifier),
            ),
            keyword,
        )
        .wrap_err(),
    }
}

fn parse_entrypoint(
    keyword: Span,
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    EntrypointDeclaration {
        entrypoint_keyword: WithSpan::new(EntrypointKeyword, keyword),
        parent_type: WithSpan::new(EntityName, parent_type),
        client_field_name: WithSpan::new(ClientFieldName, client_field_name),
    }.wrap_ok()
}
```

`parse_iso_literal` wraps `parse_iso_literal_item`: `parse_singleton`, then `IsoLiteralParse` from `Singleton`. Artifact generation requires that `push_error` was never called (and the earlier-stage lists empty). `parse_iso_literal_item` is the keyword dispatch. After `entrypoint` it calls `parse_entrypoint`. Empty is `first: None` and `EmptyLiteral` through `push_error`. A failed first chunk is `Failed` plus that chunk’s items; extra chunks still sit in `ExtraChunks`. `entrypoint Query.foo\nfield User.name` is `Complete` plus `ExtraChunks` and `push_error(MultipleDeclarations)`. `entrypoint Query.foo bar` is `Both` (declaration plus leftover items) and `push_error(Expected(EndOfDeclaration, Identifier))`. `entrypoint\nQuery.foo` is `Failed` on `entrypoint` plus `ExtraChunks` for `Query.foo`. `entrypoint Query.foo,` is `Complete` plus a tokenless comma diagnostic through `push_error`.

## `ItemCursor` and `ChunkStream`

Extracted from parsing-standards.md. Delta: this impl is `new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `expected`, `text`, `token_text`, `end_span`. Group methods and `spanning` land in parse-fields.md. `ItemCursor` and `ChunkStream` are `pub(crate)` and are not re-exported.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use nonempty::NonEmpty;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{ChunkContentItem, Expectation, Found, NonBracketTokenKind, ParseError};

/// Sequential reader of one chunk. Parameter of a parse function.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last item this cursor advanced past (the chunk's start before
    /// any). An `Expected(_, EndOfChunk)` error uses this offset.
    previous_end: u32,
    text: &'a str,
}

/// Sequential reader of one chunk, plus `require_end`.
pub(crate) struct ChunkStream<'a> {
    cursor: ItemCursor<'a>,
}

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(contents: &'a NonEmpty<WithSpan<ChunkContentItem>>, text: &'a str) -> Self {
        ChunkStream {
            cursor: ItemCursor {
                previous_end: contents.first().location.start,
                items: contents.iter().safe_peekable(),
                text,
            },
        }
    }

    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a> {
        &mut self.cursor
    }

    pub(crate) fn require_end(&mut self) -> Result<(), ()> {
        self.cursor.items.peek().map_or(().wrap_ok(), |_| ().wrap_err())
    }
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                item.location.wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError> {
        match self.items.peek() {
            None => WithSpan::new(
                ParseError::expected(expected, Found::EndOfChunk),
                self.end_span(),
            ),
            Some(peek) => {
                let item = *peek.view();
                WithSpan::new(
                    ParseError::expected(expected, Found::from(item.item.reference())),
                    item.location,
                )
            }
        }
    }

    pub(crate) fn require_token(&mut self, kind: NonBracketTokenKind) -> Result<Span, ()> {
        self.consume_token_if(kind).ok_or(())
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
    }

    fn end_span(&self) -> Span {
        Span::new(self.previous_end, self.previous_end)
    }
}
```

## Changes to chunk.rs

Extracted from parsing-standards.md. Delta: none on these items.

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    pub(crate) fn stream<'a>(&'a self, text: &'a str) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text)
    }

    pub fn boundary_comma(&self) -> Option<Span> {
        let separator = self.trailing_separator.as_ref()?;
        separator
            .0
            .iter()
            .find(|token| token.item == SeparatorToken::Comma)
            .map(|token| token.location)
    }
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    empty: impl FnOnce() -> WithSpan<ParseError>,
    extra: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<T>
where
    F: FnMut(WithSpan<ParseError>),
{
    /* parsing-standards.md */
}
```

`ChunkedLevel`'s vec becomes a private field. `len` is the chunk count. Tests call `chunks()`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel(#[resolve_field] pub Vec<WithSpan<Chunk>>);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel(#[resolve_field] Vec<WithSpan<Chunk>>);

impl ChunkedLevel {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub(crate) fn chunks(&self) -> &[WithSpan<Chunk>] {
        self.0.reference()
    }
}
```

## `ParseError`

```rust
// from crates/isograph_parser/src/parse_error.rs
use std::fmt;

use crate::{BracketKind, ChunkContentItem, NonBracketTokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    UnsupportedDeclarationType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(NonBracketTokenKind),
    DeclarationKeyword,
    EndOfDeclaration,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Token(NonBracketTokenKind),
    Group(BracketKind),
    EndOfChunk,
}

impl ParseError {
    pub fn expected(expected: Expectation, found: Found) -> Self {
        ParseError::Expected(ExpectedFound { expected, found })
    }
}

impl From<&ChunkContentItem> for Found {
    fn from(item: &ChunkContentItem) -> Self {
        match item {
            ChunkContentItem::NonBracket(token) => Found::Token(token.0),
            ChunkContentItem::Group(group) => Found::Group(group.opening.item.0),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Expected(expected_found) => expected_found.fmt(f),
            ParseError::EmptyLiteral => {
                write!(f, "Expected a declaration. An isograph literal cannot be empty.")
            }
            ParseError::MultipleDeclarations => {
                write!(f, "Expected nothing after the declaration. Each literal holds exactly one declaration.")
            }
            ParseError::UnsupportedDeclarationType => {
                write!(f, "This declaration type is not supported yet.")
            }
        }
    }
}

impl fmt::Display for ExpectedFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected {}, found {}.", self.expected, self.found)
    }
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expectation::Token(kind) => kind.fmt(f),
            Expectation::DeclarationKeyword => {
                write!(f, "one of `entrypoint`, `field`, or `pointer`")
            }
            Expectation::EndOfDeclaration => write!(f, "the end of the declaration"),
        }
    }
}

impl fmt::Display for Found {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Found::Token(kind) => kind.fmt(f),
            Found::Group(kind) => write!(f, "a group opened by {}", opening_bracket_text(*kind)),
            Found::EndOfChunk => write!(f, "nothing more"),
        }
    }
}

fn opening_bracket_text(kind: BracketKind) -> &'static str {
    match kind {
        BracketKind::Parenthesis => "'('",
        BracketKind::Brace => "'{'",
        BracketKind::Bracket => "'['",
    }
}
```

Suggestions (a found `@`, and the like) are produced by the rendering stage from the `(expected, found)` pair.

## lib.rs

Before:

```rust
// from crates/isograph_parser/src/lib.rs
mod chunk;
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod token_kind;
mod tokenize;

pub use chunk::*;
pub use isograph_resolution_node::*;
pub use matched_brackets::*;
pub use non_bracket_token::*;
pub use token_kind::*;
pub use tokenize::*;
```

After:

```rust
// from crates/isograph_parser/src/lib.rs
mod chunk;
mod chunk_stream;
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod parse_error;
mod parse_iso_literal;
mod token_kind;
mod tokenize;

pub use chunk::*;
pub use isograph_resolution_node::*;
pub use matched_brackets::*;
pub use non_bracket_token::*;
pub use parse_error::*;
pub use parse_iso_literal::*;
pub use token_kind::*;
pub use tokenize::*;
```

## Resolution

Before:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    ChunkPath, ChunkSeparatorPath, ChunkedGroupPath, ChunkedLevelPath, CloseBracketPath,
    NonBracketTokenPath, OpenBracketPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the chunk tree's.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
```

After:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    ChunkPath, ChunkSeparatorPath, ChunkedGroupPath, ChunkedLevelPath, ClientFieldNamePath,
    CloseBracketPath, EntityNamePath, EntrypointDeclarationPath, ExtraChunksPath,
    FailedPath, IsoLiteralItemPath, IsoLiteralParsePath, NonBracketTokenPath, OpenBracketPath,
    RootSlotPath, UnparsedChunkItemsPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    IsoLiteralParse(IsoLiteralParsePath<'a>),
    RootSlot(RootSlotPath<'a>),
    IsoLiteralItem(IsoLiteralItemPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    Failed(FailedPath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
    ExtraChunks(ExtraChunksPath<'a>),
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
```

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkParent<'a> {
    Level(ChunkedLevelPath<'a>),
    Extra(ExtraChunksPath<'a>),
}

#[derive(Debug)]
pub enum ChunkContentItemParent<'a> {
    Chunk(ChunkPath<'a>),
    Unparsed(UnparsedChunkItemsPath<'a>),
}
```

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field(parent_variant = Chunk)]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}
```

`Root` remains the parent a caller passes when resolving a bare chunk tree. `FailedParent` is `Both` or `Failed`. Extra root chunks use `parent_variant = Extra`. Leftover and failed items use `parent_variant = Unparsed`.

## Generated code

`UnparsedChunkItems` iterates `items`. `BothRoot` tries `item` then `failed`. `Failed` descends into its `UnparsedChunkItems`. `ExtraChunks` iterates `chunks`. The enum delegation, struct descent, and fieldless-marker impls follow chunk.rs.

## Tests

In-file, in the pattern of chunk.rs: `span_of` anchors, structural assertions, no snapshots, degenerate cases included.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[cfg(test)]
mod tests {
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::{
        chunk, match_brackets, tokenize, BracketError, BracketKind, CommaWithoutItem,
    };
    use Expectation::{DeclarationKeyword, EndOfDeclaration};
    use NonBracketTokenKind::{At, Comma, Identifier, IntegerLiteral, Period};

    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (parse, errors)
    }

    fn parsed_with_errors(
        text: &str,
    ) -> (
        WithSpan<IsoLiteralParse>,
        Vec<WithSpan<ParseError>>,
        Vec<BracketError>,
        Vec<CommaWithoutItem>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let parse = parse_iso_literal(text, tree, |error| errors.push(error));
        (parse, errors, bracket_errors, comma_errors)
    }

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
    }

    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    fn as_entrypoint(parse: &WithSpan<IsoLiteralParse>) -> &EntrypointDeclaration {
        parse
            .item
            .item()
            .expect("the fixture's literal is an entrypoint")
    }

    fn assert_no_declaration(text: &str, reason: ParseError, reason_span: Span) {
        let (parse, errors) = parsed(text);
        assert!(parse.item.item().is_none(), "for literal {text:?}");
        assert!(
            errors.iter().any(|error| error.item == reason && error.location == reason_span),
            "for literal {text:?}, errors were {errors:?}",
        );
    }

    #[test]
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let (parse, errors) = parsed(text);
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(declaration.entrypoint_keyword.location, span_of(text, "entrypoint"));
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "foo"));
        assert_eq!(errors, vec![]);
        assert_eq!(parse.location, Span::from_usize(0, text.len()));
    }

    #[test]
    fn surrounding_line_breaks_and_interior_spaces_are_insignificant() {
        for text in [
            "\n  entrypoint Query.foo\n",
            "\n\nentrypoint Query.foo",
            "entrypoint Query . foo",
        ] {
            let (parse, errors) = parsed(text);
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(declaration.parent_type.location, span_of(text, "Query"), "for literal {text:?}");
            assert_eq!(declaration.client_field_name.location, span_of(text, "foo"), "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn empty_and_whitespace_only_literals_are_empty_literal_errors() {
        for text in ["", "   ", "\n\n"] {
            assert_no_declaration(text, ParseError::EmptyLiteral, Span::from_usize(0, text.len()));
        }
    }

    #[test]
    fn comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses() {
        for (text, comma_error_count) in [
            (",entrypoint Query.foo", 1),
            (",,entrypoint Query.foo", 2),
        ] {
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(comma_errors.len(), comma_error_count, "for literal {text:?}");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(declaration.parent_type.location, span_of(text, "Query"), "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_lone_comma_is_chunkings_error_and_an_empty_literal() {
        let text = ",";
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        assert!(parse.item.item().is_none());
        assert_eq!(
            errors,
            WithSpan::new(ParseError::EmptyLiteral, Span::from_usize(0, text.len())).wrap_vec(),
        );
    }

    #[test]
    fn the_cut_removes_an_unmatched_bracket_and_the_declaration_parses() {
        for text in ["entrypoint Query.foo)", "entrypoint Query.foo ("] {
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert_eq!(bracket_errors.len(), 1, "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(declaration.client_field_name.location, span_of(text, "foo"), "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_final_comma_after_the_declaration_is_an_error() {
        for text in [
            "entrypoint Query.foo,",
            "\nentrypoint Query.foo,\n",
        ] {
            let (parse, errors) = parsed(text);
            as_entrypoint(parse.reference());
            assert_eq!(
                errors,
                WithSpan::new(expected(EndOfDeclaration, Found::Token(Comma)), span_of(text, ",")).wrap_vec(),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_comma_before_a_second_declaration_is_the_boundary_comma() {
        let text = "entrypoint Query.foo, field User.name";
        let (parse, errors) = parsed(text);
        assert_eq!(as_entrypoint(parse.reference()).client_field_name.location, span_of(text, "foo"));
        assert_eq!(
            errors,
            vec![
                WithSpan::new(expected(EndOfDeclaration, Found::Token(Comma)), span_of(text, ",")),
                WithSpan::new(ParseError::MultipleDeclarations, span_of(text, "field User.name")),
            ],
        );
        assert!(parse.item.extra.as_ref().is_some());
    }

    #[test]
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(text);
        assert_eq!(as_entrypoint(parse.reference()).client_field_name.location, span_of(text, "foo"));
        assert_eq!(
            errors,
            WithSpan::new(ParseError::MultipleDeclarations, span_of(text, "field User.name")).wrap_vec(),
        );
        assert!(parse.item.extra.as_ref().is_some());
    }

    #[test]
    fn a_failed_first_chunk_is_reported_even_when_a_second_exists() {
        let text = "entrypoint\nQuery.foo";
        let keyword_end = span_of(text, "entrypoint").end;
        let (parse, errors) = parsed(text);
        assert!(parse.item.item().is_none());
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(keyword_end, keyword_end)
        }));
        assert!(errors.iter().any(|error| error.item == ParseError::MultipleDeclarations));
        assert!(parse.item.extra.as_ref().is_some());
    }

    #[test]
    fn an_unknown_keyword_is_an_error_at_the_keyword() {
        let text = "fieldd Query.foo { bar }";
        assert_no_declaration(
            text,
            expected(DeclarationKeyword, Found::Token(Identifier)),
            span_of(text, "fieldd"),
        );
    }

    #[test]
    fn a_literal_opening_with_a_group_expects_a_keyword() {
        let text = "{ bar }";
        assert_no_declaration(
            text,
            expected(DeclarationKeyword, Found::Group(BracketKind::Brace)),
            span_of(text, "{ bar }"),
        );
    }

    #[test]
    fn field_and_pointer_declarations_do_not_parse_yet() {
        let field = "field Query.foo { bar }";
        assert_no_declaration(field, ParseError::UnsupportedDeclarationType, span_of(field, "field"));
        let pointer = "pointer Query.foo to Bar { id }";
        assert_no_declaration(pointer, ParseError::UnsupportedDeclarationType, span_of(pointer, "pointer"));
    }

    #[test]
    fn each_missing_entrypoint_part_reports_at_its_position() {
        let bare = "entrypoint";
        let keyword_end = span_of(bare, "entrypoint").end;
        assert_no_declaration(
            bare,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(keyword_end, keyword_end),
        );

        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(token(Identifier), Found::Token(IntegerLiteral)),
            span_of(numeric, "42"),
        );

        let dotless = "entrypoint Query foo";
        assert_no_declaration(
            dotless,
            expected(token(Period), Found::Token(Identifier)),
            span_of(dotless, "foo"),
        );

        let nameless = "entrypoint Query.";
        let dot_end = span_of(nameless, ".").end;
        assert_no_declaration(
            nameless,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(dot_end, dot_end),
        );
    }

    #[test]
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        match parse.item.first.as_ref().map(|slot| slot.item.reference()) {
            Some(RootSlot::Both(_)) => {}
            other => panic!("expected Both, got {other:?}"),
        }
        assert_eq!(
            errors,
            WithSpan::new(expected(EndOfDeclaration, Found::Token(Identifier)), span_of(text, "bar")).wrap_vec(),
        );
    }

    #[test]
    fn a_selection_set_on_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo { bar }";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            WithSpan::new(
                expected(EndOfDeclaration, Found::Group(BracketKind::Brace)),
                span_of(text, "{ bar }"),
            )
            .wrap_vec(),
        );
    }

    #[test]
    fn a_directive_is_an_ordinary_unexpected_token() {
        let text = "entrypoint Query.foo @lazy";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            WithSpan::new(expected(EndOfDeclaration, Found::Token(At)), span_of(text, "@")).wrap_vec(),
        );
    }

    #[test]
    fn leftover_after_an_entrypoint_resolves_to_the_leftover_token() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
    }

    #[test]
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Query")) {
            IsographResolutionNode::EntityName(name) => {
                assert_eq!(name.parent.inner.client_field_name.location, span_of(text, "foo"));
            }
            node => panic!("expected the entity name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::ClientFieldName(_) => {}
            node => panic!("expected the client field name leaf, got {node:?}"),
        }
        for span in [
            span_of(text, "entrypoint"),
            span_of(text, "."),
            Span::new(span_of(text, "entrypoint").end, span_of(text, "Query").start),
        ] {
            match parse.resolve((), span) {
                IsographResolutionNode::EntrypointDeclaration(_) => {}
                node => panic!("expected the declaration leaf at {span}, got {node:?}"),
            }
        }
    }

    #[test]
    fn positions_inside_a_failed_first_chunk_resolve_through_the_cloned_chunk() {
        let text = "fieldd Query.foo { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn the_unrecognized_keyword_resolves_as_a_token_in_the_failed_chunk() {
        let text = "fieldd Query.foo { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "fieldd")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }
}
```

## Landing checklist

1. chunk_stream.rs, `Chunk::stream`, `remaining_contents`, `boundary_comma`, `parse_one_item`, `parse_singleton`, parse_error.rs, parse_iso_literal.rs, the lib.rs registrations, the `IsographResolutionNode`, `ChunkParent`, and `ChunkContentItemParent` changes, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
