# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

First doc of the series parsing-plan.md orders, written against parsing-standards.md. This doc lands `ItemCursor` / `ChunkStream` (`new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `expected`, `text`, `token_text`, `end_span`), `Chunk::stream`, `boundary_comma`, `ChunkedLevel::len`, `parse_singleton`, `parse_iso_literal`, `ParseError`, `UnparsedLiteral`, and `entrypoint Type.field`. `field` and `pointer` are identifiers that return `UnsupportedDeclarationType`; parse-fields.md and parse-pointers.md replace those arms.

## The grammar

```
entrypoint <Identifier> . <Identifier>
```

The root level is one chunk. `parse_singleton` uses `Expectation::EndOfDeclaration`. Empty level: `EmptyLiteral` at the root span. Extra chunk: `MultipleDeclarations` at that chunk's span. Boundary comma: `Expected(EndOfDeclaration, Token(Comma))` at the comma.

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

Any other root parse is `UnparsedLiteral`: one reason and the entire root level.

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
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

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

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntrypointKeyword;

pub type EntrypointDeclarationPath<'a> = PositionResolutionPath<&'a EntrypointDeclaration, ()>;

pub type UnparsedLiteralPath<'a> = PositionResolutionPath<&'a UnparsedLiteral, ()>;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> = PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl IsoLiteralParse {
    pub fn errors(&self) -> Vec<WithSpan<ParseError>> {
        match self {
            IsoLiteralParse::Entrypoint(_) => vec![],
            IsoLiteralParse::Unparsed(unparsed) => unparsed.reason.wrap_vec(),
        }
    }
}
```

## The parser

The root is borrowed until the end: on `Err` it moves into `UnparsedLiteral`; on `Ok` it is dropped.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    let parse = parse_singleton(
        root.reference(),
        text,
        || WithSpan::new(ParseError::EmptyLiteral, location),
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        parse_declaration,
    )
    .unwrap_or_else(|reason| {
        IsoLiteralParse::Unparsed(UnparsedLiteral { reason, level: root })
    });
    WithSpan::new(parse, location)
}

fn parse_declaration(cursor: &mut ItemCursor<'_>) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        text if text == "entrypoint" => {
            IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, cursor)?).wrap_ok()
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
        ).wrap_err(),
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

`parse_declaration` returns after the last identifier. `parse_singleton` matches `len()` first: empty is `EmptyLiteral`, two or more is `MultipleDeclarations` on the second chunk (the first is not parsed), one chunk is `parse_declaration` then `require_end` then `boundary_comma`. `entrypoint\nQuery.foo` is two chunks, so `MultipleDeclarations` at `Query.foo`.

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

pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    empty: impl FnOnce() -> WithSpan<ParseError>,
    extra: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>) -> Result<T, WithSpan<ParseError>>,
) -> Result<T, WithSpan<ParseError>> {
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
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
    UnparsedLiteral(UnparsedLiteralPath<'a>),
}
```

`UnparsedLiteralPath` ends at `()`, so `UnparsedLiteral` is unboxed. `Root` is the parent a caller passes when resolving a bare chunk tree.

## Generated code

The novel shape is `UnparsedLiteral`'s field: `parent_variant` constructs an unboxed variant. The enum delegation, struct descent with a fallthrough leaf, and fieldless-marker impls follow chunk.rs.

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for UnparsedLiteral {
    type Parent<'a> = ();
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.level.location.contains(position) {
            let new_parent = <ChunkedLevel as ::resolve_position::ResolvePosition>::Parent::UnparsedLiteral(self.path(parent).to());
            return self.level.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::UnparsedLiteral(self.path(parent).to());
    }
}
```

The level's span is the whole literal, so the fallthrough leaf is the derive's required shape.

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

    fn parsed(text: &str) -> WithSpan<IsoLiteralParse> {
        let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        parse
    }

    fn parsed_with_errors(
        text: &str,
    ) -> (WithSpan<IsoLiteralParse>, Vec<BracketError>, Vec<CommaWithoutItem>) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(brackets.reference());
        (parse_iso_literal(text, tree), bracket_errors, comma_errors)
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
        match parse.item.reference() {
            IsoLiteralParse::Entrypoint(declaration) => declaration,
            parse => panic!("expected an entrypoint declaration, got {parse:?}"),
        }
    }

    fn assert_unparsed(text: &str, reason: ParseError, reason_span: Span) {
        let parse = parsed(text);
        match parse.item.reference() {
            IsoLiteralParse::Unparsed(unparsed) => {
                assert_eq!(unparsed.reason.item, reason, "for literal {text:?}");
                assert_eq!(unparsed.reason.location, reason_span, "for literal {text:?}");
                assert_eq!(parse.item.errors(), unparsed.reason.wrap_vec());
            }
            parse => panic!("expected an unparsed literal for {text:?}, got {parse:?}"),
        }
    }

    #[test]
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let parse = parsed(text);
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(declaration.entrypoint_keyword.location, span_of(text, "entrypoint"));
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
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
        ] {
            let parse = parsed(text);
            let declaration = as_entrypoint(parse.reference());
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
        ] {
            let (parse, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(comma_errors.len(), comma_error_count, "for literal {text:?}");
            let declaration = as_entrypoint(parse.reference());
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
        match parse.item.reference() {
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
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(declaration.client_field_name.location, span_of(text, "foo"), "for literal {text:?}");
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_final_comma_after_the_declaration_is_an_error() {
        for text in [
            "entrypoint Query.foo,",
            "\nentrypoint Query.foo,\n",
        ] {
            assert_unparsed(
                text,
                expected(EndOfDeclaration, Found::Token(Comma)),
                span_of(text, ","),
            );
        }
    }

    #[test]
    fn a_comma_before_a_second_declaration_is_multiple_declarations() {
        let text = "entrypoint Query.foo, field User.name";
        assert_unparsed(
            text,
            ParseError::MultipleDeclarations,
            span_of(text, "field User.name"),
        );
    }

    #[test]
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        assert_unparsed(text, ParseError::MultipleDeclarations, span_of(text, "field User.name"));
    }

    #[test]
    fn two_chunks_are_multiple_declarations_without_parsing_the_first() {
        let text = "entrypoint\nQuery.foo";
        assert_unparsed(
            text,
            ParseError::MultipleDeclarations,
            span_of(text, "Query.foo"),
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
                let interior_level = match token.parent.parent.parent.reference() {
                    ChunkedLevelParent::Interior(group) => group,
                    parent => panic!("expected an interior level, got {parent:?}"),
                };
                match interior_level.parent.parent.parent.reference() {
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
                match token.parent.parent.parent.reference() {
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

1. chunk_stream.rs, `Chunk::stream`, `boundary_comma`, `parse_singleton`, parse_error.rs, parse_iso_literal.rs, the lib.rs registrations, the `IsographResolutionNode` and `ChunkedLevelParent` changes, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
