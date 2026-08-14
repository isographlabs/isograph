# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

First doc of the series parsing-plan.md orders, written against parsing-standards.md and the landed prefactors (refactors/past/cut-at-unmatched.md, no-empty-chunks.md, one-comma-per-boundary.md, non-empty-vec-iter.md, private-chunk-fields.md): no unmatched-bracket state and no empty chunk reach this stage, and `Chunk`'s fields are already private. This doc lands `ChunkStream`'s required-token core, `Chunk::stream`, `parse_iso_literal`, the root-level rules, keyword dispatch, `ParseError`, `token_text`, the whole-literal failure fallback with its resolution path, and the complete `entrypoint Type.field` declaration. `field` and `pointer` are recognized keywords that dispatch to a temporary error variant; parse-fields.md and parse-pointers.md replace it.

## The grammar this doc accepts

A literal parses when its root level holds exactly one chunk and that chunk is:

```
entrypoint <Identifier> . <Identifier>
```

with nothing after the second identifier. The earlier passes decide what reaches this stage: leading line breaks were captured by the literal's start, a comma no item precedes was dropped by chunking with a `CommaWithoutItem` error beside the tree (refactors/past/no-empty-chunks.md), and an unmatched bracket cut its level's tail at the matcher (refactors/past/cut-at-unmatched.md). So `,entrypoint Query.foo` parses at this stage — the comma was chunking's error — and `entrypoint Query.foo)` parses at this stage — the `)` was the matcher's, and the cut removed it. A trailing boundary after the declaration is insignificant in this doc, comma included; no-final-comma.md rejects the comma there once every declaration form exists. The normal literal style parses:

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

Everything else produces an `UnparsedLiteral` holding one reason and the entire root level.

## New module: chunk_stream.rs

The enforcement structure parsing-standards.md specifies, at the subset this doc's grammar needs; later docs extend the impl block. The module has no re-export: `ChunkStream` is `pub(crate)` and never crosses the crate boundary.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use non_empty_vec::NonEmptyVec;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{ChunkContentItem, Expectation, Found, NonBracketTokenKind, ParseError};

/// The only reader of a chunk's contents. No rewind and no raw peek exist: a committed
/// item is committed, and a decision is made on at most the next item.
pub(crate) struct ChunkStream<'a> {
    items: SafePeekable<non_empty_vec::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last accepted item (the chunk's start before any): where an
    /// `Expected(_, EndOfChunk)` error points.
    previous_end: u32,
}

impl<'a> ChunkStream<'a> {
    /// Only `Chunk::stream` constructs one, so a stream always reads a whole chunk.
    pub(crate) fn new(contents: &'a NonEmptyVec<WithSpan<ChunkContentItem>>) -> Self {
        ChunkStream {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
        }
    }

    /// The next item's span when it is a non-bracket token of `kind`; the error
    /// otherwise, on the found item, unconsumed, or empty at `previous_end` when the
    /// chunk ran out.
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>> {
        let Some(peek) = self.items.peek() else {
            return Err(WithSpan::new(
                ParseError::expected(expected, Found::EndOfChunk),
                Span::new(self.previous_end, self.previous_end),
            ));
        };
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                Ok(item.location)
            }
            other => Err(WithSpan::new(
                ParseError::expected(expected, Found::from(other)),
                item.location,
            )),
        }
    }

    /// Nothing further may exist. The first leftover item is the error.
    pub(crate) fn require_end(
        &mut self,
        expected: Expectation,
    ) -> Result<(), WithSpan<ParseError>> {
        match self.items.peek() {
            None => Ok(()),
            Some(peek) => {
                let item = *peek.view();
                Err(WithSpan::new(
                    ParseError::expected(expected, Found::from(&item.item)),
                    item.location,
                ))
            }
        }
    }
}
```

`Chunk` gains the constructor's one call site, on its narrowed surface:

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// The stream a parser reads this chunk through.
    pub(crate) fn stream(&self) -> ChunkStream<'_> {
        ChunkStream::new(&self.contents)
    }
}
```

## New module: parse_error.rs

```rust
// from crates/isograph_parser/src/parse_error.rs
use std::fmt;

use crate::{BracketKind, ChunkContentItem, NonBracketTokenKind};

/// Why a region failed to parse, positioned by a wrapping `WithSpan` that covers the
/// offending item, or is empty at the position where a missing item was expected.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    /// Temporary: parse-fields.md and parse-pointers.md remove this variant.
    UnsupportedDeclarationType,
}

/// What the grammar wanted at a position and what sat there instead.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

/// What the grammar wanted at the error's position.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    /// A specific token, e.g. an identifier or a `.`.
    Token(NonBracketTokenKind),
    /// `entrypoint`, `field`, or `pointer`, as the declaration's first token.
    DeclarationKeyword,
    /// Nothing further: the declaration is complete.
    EndOfDeclaration,
}

/// What sat at the error's position.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Token(NonBracketTokenKind),
    /// A matched group, named by its opening bracket.
    Group(BracketKind),
    /// The chunk ended; there was nothing at the position.
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

parsing-standards.md brackets whether `Expectation` stays one global enum, becomes per-logical-group enums, or the error becomes a rendered `Diagnostic`; the listing above is the global-enum candidate, and this doc's review decides the bracket. Contextual suggestions (directive migration on a found `@`, and the like) are the rendering stage's concern, keyed off the `(expected, found)` pair; this crate carries only the structural facts.

## New module: parse_iso_literal.rs

The types, most important first. Span conventions follow the crate: the enum is wrapped once at the top, variant payloads are bare, every struct field carries its own span, and a declaration's tight span is derivable from its fields' spans, the way a chunk's is.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::chunk_stream::ChunkStream;
use crate::{
    Chunk, ChunkedLevel, Expectation, Found, IsographResolutionNode, NonBracketTokenKind,
    ParseError,
};

/// The parse of one literal. The wrapping `WithSpan`'s span is the whole literal.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralParse {
    Entrypoint(EntrypointDeclaration),
    Unparsed(UnparsedLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    pub entrypoint_keyword: WithSpan<EntrypointKeyword>,
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    pub dot: WithSpan<Dot>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

/// A literal that failed to parse: the reason, and the whole root level for positions to
/// resolve against.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedLiteral {
    pub reason: WithSpan<ParseError>,
    #[resolve_field(parent_variant = UnparsedLiteral)]
    pub level: WithSpan<ChunkedLevel>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName;

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`.
/// Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName;

/// The `entrypoint` keyword. Positions on it answer the declaration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntrypointKeyword;

/// The `.` between the type and the field name. Positions on it answer the declaration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Dot;

pub type EntrypointDeclarationPath<'a> = PositionResolutionPath<&'a EntrypointDeclaration, ()>;

pub type UnparsedLiteralPath<'a> = PositionResolutionPath<&'a UnparsedLiteral, ()>;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> = PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

`EntityName` and `ClientFieldName` will gain more parents when field and pointer declarations arrive; parse-fields.md converts their parent aliases to enums then, the same evolution the chunk types went through.

The errors, derived from the tree as everywhere else in the crate:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl IsoLiteralParse {
    /// Every error the pass produced. A parsed declaration has none; an unparsed
    /// literal has exactly its reason.
    pub fn errors(&self) -> Vec<WithSpan<ParseError>> {
        match self {
            IsoLiteralParse::Entrypoint(_) => vec![],
            IsoLiteralParse::Unparsed(unparsed) => vec![unparsed.reason],
        }
    }
}
```

## The parser

Validation runs by reference; the root moves into the output exactly once, at the end, into `UnparsedLiteral` on failure or dropped on success (a parsed declaration copies only spans out of it).

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    match try_parse(text, &root) {
        Ok(parse) => WithSpan::new(parse, location),
        Err(reason) => WithSpan::new(
            IsoLiteralParse::Unparsed(UnparsedLiteral { reason, level: root }),
            location,
        ),
    }
}

/// The declaration chunk parses before the extra-chunk check, so an incomplete
/// declaration split across a line break reports its own precise error, and only a
/// complete declaration followed by more content reports `MultipleDeclarations`.
fn try_parse(
    text: &str,
    root: &WithSpan<ChunkedLevel>,
) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let (declaration, extra) = declaration_chunk(root)?;
    let parse = parse_declaration_chunk(text, declaration)?;
    if let Some(extra) = extra {
        return Err(WithSpan::new(ParseError::MultipleDeclarations, extra));
    }
    Ok(parse)
}

/// The root's first chunk, which is the declaration, plus the first extra chunk's span
/// when more exist. The literal's leading line breaks were captured before any chunk
/// existed and no empty chunk exists (refactors/past/no-empty-chunks.md), so the
/// declaration can sit nowhere else.
fn declaration_chunk(
    root: &WithSpan<ChunkedLevel>,
) -> Result<(&WithSpan<Chunk>, Option<Span>), WithSpan<ParseError>> {
    let mut chunks = root.item.0.iter();
    let Some(declaration) = chunks.next() else {
        return Err(WithSpan::new(ParseError::EmptyLiteral, root.location));
    };
    let extra = chunks.next().map(|chunk| chunk.location);
    Ok((declaration, extra))
}

/// The one-item walker's per-chunk half: the keyword dispatch, and the end check after
/// the parsed production. The item parser stops when its production ends; exhaustion is
/// checked here (parsing-standards.md, Level walks).
fn parse_declaration_chunk(
    text: &str,
    chunk: &WithSpan<Chunk>,
) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let mut stream = chunk.item.stream();
    let keyword = stream.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::DeclarationKeyword,
    )?;
    let parse = match token_text(text, keyword) {
        "entrypoint" => IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, &mut stream)?),
        "field" | "pointer" => {
            return Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword));
        }
        _ => {
            return Err(WithSpan::new(
                ParseError::expected(
                    Expectation::DeclarationKeyword,
                    Found::Token(NonBracketTokenKind::Identifier),
                ),
                keyword,
            ));
        }
    };
    stream.require_end(Expectation::EndOfDeclaration)?;
    Ok(parse)
}

fn parse_entrypoint(
    keyword: Span,
    stream: &mut ChunkStream<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = stream.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    let dot = stream.require_token(
        NonBracketTokenKind::Period,
        Expectation::Token(NonBracketTokenKind::Period),
    )?;
    let client_field_name = stream.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    Ok(EntrypointDeclaration {
        entrypoint_keyword: WithSpan::new(EntrypointKeyword, keyword),
        parent_type: WithSpan::new(EntityName, parent_type),
        dot: WithSpan::new(Dot, dot),
        client_field_name: WithSpan::new(ClientFieldName, client_field_name),
    })
}

/// The literal text a span covers. The parser reads it only to recognize keywords.
fn token_text(text: &str, span: Span) -> &str {
    &text[span.as_usize_range()]
}
```

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

After (chunk_stream has no re-export; `ChunkStream` is `pub(crate)`):

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

## The resolution surface

`IsographResolutionNode` gains the grammar-stage leaves and keeps the chunk-stage ones, which remain reachable through `UnparsedLiteral`. Before:

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
    CloseBracketPath, EntityNamePath, EntrypointDeclarationPath, NonBracketTokenPath,
    OpenBracketPath, UnparsedLiteralPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    UnparsedLiteral(UnparsedLiteralPath<'a>),
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

`ChunkedLevelParent` gains the variant `UnparsedLiteral`'s `#[resolve_field]` constructs, so an ancestry walk from inside a failed literal reaches the grammar tree. `Root` stays: it is the parent a caller passes when resolving a bare chunk tree, as the chunk tests do. Before:

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
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
    UnparsedLiteral(UnparsedLiteralPath<'a>),
}
```

No box on the new variant: `UnparsedLiteralPath` ends at `()`, so there is no type cycle to break, unlike `Interior`.

## Generated code

Per AGENTS.md, expansions whose shape landed code already shows are not written out: the enum delegation, the struct descent with a fallthrough leaf, and the fieldless-marker impls all appear in chunk.rs's derives. The one novel shape is `UnparsedLiteral`'s field, whose `parent_variant` constructs an unboxed variant:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for UnparsedLiteral {
    type Parent<'a> = ();
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.level.location.contains(position) {
            let new_parent = <ChunkedLevel as ::resolve_position::ResolvePosition>::Parent::UnparsedLiteral(self.path(parent).into());
            return self.level.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::UnparsedLiteral(self.path(parent).into());
    }
}
```

The level's span is the whole literal, so the fallthrough leaf is unreachable in practice but exists as the derive's shape requires.

## Changes from the pre-standards draft

This doc revises the earlier parse-entrypoint.md in place; the types, `errors()`, `token_text`, the resolution surface, and most tests are carried verbatim. The deltas, each forced by a landed prefactor or by parsing-standards.md:

- The free functions `expect_token` and `expect_chunk_end`, the `ChunkContents` alias, and the `missing_at` parameter are replaced by `ChunkStream`'s `require_token` and `require_end`: `previous_end` is tracked once in the stream instead of threaded through every call.
- `require_end` moves out of `parse_entrypoint` into `parse_declaration_chunk`, after the dispatch: the item parser stops when its production ends, and the walker owns exhaustion.
- `Found` loses `UnmatchedOpen` and `UnmatchedClose`, and `closing_bracket_text` dies with them: the cut means no unmatched state reaches a parser.
- `declaration_chunk` loses its empty-chunk arms and the `empty_chunk_comma_span` helper: no empty chunk exists. The intro's description of a leading comma as "an empty chunk whose boundary starts with that comma" described chunking before no-empty-chunks.md and is gone with it.
- The extra-chunks span becomes the first extra chunk's span; the old draft joined every extra chunk's span, and `Span::join` in a parser is banned.
- The tests `a_comma_before_the_declaration_is_an_error`, `doubled_commas_before_the_declaration_report_the_first`, and `a_doubled_comma_after_the_declaration_is_an_error_on_the_empty_chunk` are replaced by division-of-labor tests: those commas are chunking's errors now, and the declaration parses. `an_unmatched_bracket_after_an_entrypoint_is_leftover` becomes a cut test the same way. `parsed` asserts the earlier passes were clean; `parsed_with_errors` exposes their error vecs for the tests that split responsibility between stages.

## Tests

In-file, in the pattern of chunk.rs: `span_of` anchors, structural assertions, no snapshots, degenerate cases included.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[cfg(test)]
mod tests {
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::{
        chunk, match_brackets, tokenize, BracketError, BracketKind, ChunkedLevelParent,
        CommaWithoutItem,
    };
    use Expectation::{DeclarationKeyword, EndOfDeclaration};
    use NonBracketTokenKind::{At, Comma, Identifier, IntegerLiteral, Period};

    /// The pipeline for literals whose earlier passes are clean; the assertion catches a
    /// test accidentally relying on a bracket or comma mistake.
    fn parsed(text: &str) -> WithSpan<IsoLiteralParse> {
        let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        parse
    }

    /// The whole pipeline with the earlier passes' error vecs, for the tests that split
    /// responsibility between stages.
    fn parsed_with_errors(
        text: &str,
    ) -> (WithSpan<IsoLiteralParse>, Vec<BracketError>, Vec<CommaWithoutItem>) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(&brackets);
        (parse_iso_literal(text, tree), bracket_errors, comma_errors)
    }

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
    }

    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
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
        match &parse.item {
            IsoLiteralParse::Entrypoint(declaration) => declaration,
            parse => panic!("expected an entrypoint declaration, got {parse:?}"),
        }
    }

    /// The literal must be unparsed for `reason` at `reason_span`, and `errors()` must
    /// report exactly that reason.
    fn assert_unparsed(text: &str, reason: ParseError, reason_span: Span) {
        let parse = parsed(text);
        match &parse.item {
            IsoLiteralParse::Unparsed(unparsed) => {
                assert_eq!(unparsed.reason.item, reason, "for literal {text:?}");
                assert_eq!(unparsed.reason.location, reason_span, "for literal {text:?}");
                assert_eq!(parse.item.errors(), vec![unparsed.reason]);
            }
            parse => panic!("expected an unparsed literal for {text:?}, got {parse:?}"),
        }
    }

    #[test]
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let parse = parsed(text);
        let declaration = as_entrypoint(&parse);
        assert_eq!(declaration.entrypoint_keyword.location, span_of(text, "entrypoint"));
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.dot.location, span_of(text, "."));
        assert_eq!(declaration.client_field_name.location, span_of(text, "foo"));
        assert_eq!(parse.item.errors(), vec![]);
        assert_eq!(parse.location, Span::from_usize(0, text.len()));
    }

    #[test]
    fn surrounding_line_breaks_and_interior_spaces_are_insignificant() {
        for text in [
            "\n  entrypoint Query.foo\n",
            "\n\nentrypoint Query.foo",
            "entrypoint Query . foo",
            "entrypoint Query.foo,",
            "\nentrypoint Query.foo,\n",
        ] {
            let parse = parsed(text);
            let declaration = as_entrypoint(&parse);
            assert_eq!(declaration.parent_type.location, span_of(text, "Query"), "for literal {text:?}");
            assert_eq!(declaration.client_field_name.location, span_of(text, "foo"), "for literal {text:?}");
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn empty_and_whitespace_only_literals_are_empty_literal_errors() {
        for text in ["", "   ", "\n\n"] {
            assert_unparsed(text, ParseError::EmptyLiteral, Span::from_usize(0, text.len()));
        }
    }

    #[test]
    fn comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses() {
        for (text, comma_error_count) in [
            (",entrypoint Query.foo", 1),
            (",,entrypoint Query.foo", 2),
            ("entrypoint Query.foo,,", 1),
        ] {
            let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(comma_errors.len(), comma_error_count, "for literal {text:?}");
            let declaration = as_entrypoint(&parse);
            assert_eq!(declaration.parent_type.location, span_of(text, "Query"), "for literal {text:?}");
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_lone_comma_is_chunkings_error_and_an_empty_literal() {
        let text = ",";
        let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        match &parse.item {
            IsoLiteralParse::Unparsed(unparsed) => {
                assert_eq!(unparsed.reason.item, ParseError::EmptyLiteral);
                assert_eq!(unparsed.reason.location, Span::from_usize(0, text.len()));
            }
            parse => panic!("expected an unparsed literal, got {parse:?}"),
        }
    }

    #[test]
    fn the_cut_removes_an_unmatched_bracket_and_the_declaration_parses() {
        for text in ["entrypoint Query.foo)", "entrypoint Query.foo ("] {
            let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert_eq!(bracket_errors.len(), 1, "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            let declaration = as_entrypoint(&parse);
            assert_eq!(declaration.client_field_name.location, span_of(text, "foo"), "for literal {text:?}");
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_second_contentful_chunk_after_a_complete_declaration_is_an_error() {
        let text = "entrypoint Query.foo\nfield User.name";
        assert_unparsed(text, ParseError::MultipleDeclarations, span_of(text, "field User.name"));
    }

    #[test]
    fn an_incomplete_declaration_reports_its_own_error_before_the_extra_chunk() {
        let text = "entrypoint\nQuery.foo";
        let keyword_end = span_of(text, "entrypoint").end;
        assert_unparsed(
            text,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(keyword_end, keyword_end),
        );
    }

    #[test]
    fn an_unknown_keyword_is_an_error_at_the_keyword() {
        let text = "fieldd Query.foo { bar }";
        assert_unparsed(
            text,
            expected(DeclarationKeyword, Found::Token(Identifier)),
            span_of(text, "fieldd"),
        );
    }

    #[test]
    fn a_literal_opening_with_a_group_expects_a_keyword() {
        let text = "{ bar }";
        assert_unparsed(
            text,
            expected(DeclarationKeyword, Found::Group(BracketKind::Brace)),
            span_of(text, "{ bar }"),
        );
    }

    #[test]
    fn field_and_pointer_declarations_do_not_parse_yet() {
        let field = "field Query.foo { bar }";
        assert_unparsed(field, ParseError::UnsupportedDeclarationType, span_of(field, "field"));
        let pointer = "pointer Query.foo to Bar { id }";
        assert_unparsed(pointer, ParseError::UnsupportedDeclarationType, span_of(pointer, "pointer"));
    }

    #[test]
    fn each_missing_entrypoint_part_reports_at_its_position() {
        let bare = "entrypoint";
        let keyword_end = span_of(bare, "entrypoint").end;
        assert_unparsed(
            bare,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(keyword_end, keyword_end),
        );

        let numeric = "entrypoint 42.foo";
        assert_unparsed(
            numeric,
            expected(token(Identifier), Found::Token(IntegerLiteral)),
            span_of(numeric, "42"),
        );

        let dotless = "entrypoint Query foo";
        assert_unparsed(
            dotless,
            expected(token(Period), Found::Token(Identifier)),
            span_of(dotless, "foo"),
        );

        let nameless = "entrypoint Query.";
        let dot_end = span_of(nameless, ".").end;
        assert_unparsed(
            nameless,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(dot_end, dot_end),
        );
    }

    #[test]
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::Token(Identifier)),
            span_of(text, "bar"),
        );
    }

    #[test]
    fn a_selection_set_on_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo { bar }";
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::Group(BracketKind::Brace)),
            span_of(text, "{ bar }"),
        );
    }

    #[test]
    fn a_directive_is_an_ordinary_unexpected_token() {
        let text = "entrypoint Query.foo @lazy";
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::Token(At)),
            span_of(text, "@"),
        );
    }

    #[test]
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let parse = parsed(text);
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
    fn positions_inside_an_unparsed_literal_resolve_through_the_chunk_tree() {
        let text = "fieldd Query.foo { bar }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                let interior_level = match &token.parent.parent.parent {
                    ChunkedLevelParent::Interior(group) => group,
                    parent => panic!("expected an interior level, got {parent:?}"),
                };
                match &interior_level.parent.parent.parent {
                    ChunkedLevelParent::UnparsedLiteral(unparsed) => {
                        assert_eq!(
                            unparsed.inner.reason.item,
                            expected(DeclarationKeyword, Found::Token(Identifier))
                        );
                    }
                    parent => panic!("expected the unparsed literal at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn the_unrecognized_keyword_itself_resolves_under_the_unparsed_literal() {
        let text = "fieldd Query.foo { bar }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "fieldd")) {
            IsographResolutionNode::NonBracketToken(token) => {
                match &token.parent.parent.parent {
                    ChunkedLevelParent::UnparsedLiteral(_) => {}
                    parent => panic!("expected the unparsed literal at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }
}
```

## Landing checklist

1. chunk_stream.rs, `Chunk::stream`, parse_error.rs, parse_iso_literal.rs, the lib.rs registrations, the `IsographResolutionNode` and `ChunkedLevelParent` changes, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. The bracketed `Expectation` question in parsing-standards.md is resolved by this doc's review, and the standards' Errors section is amended to record the decision.
3. Move this doc to refactors/past.
