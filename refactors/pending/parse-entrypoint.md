# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

First doc of the series parsing-plan.md orders, written against parsing-standards.md. This doc lands `ItemCursor` / `ChunkStream`, `Chunk::stream`, `parse_singleton` (including `boundary_comma`), `parse_iso_literal`, the root-level rules, keyword dispatch, `ParseError`, the whole-literal failure fallback with its resolution path, and the complete `entrypoint Type.field` declaration. `field` and `pointer` are recognized keywords that dispatch to a temporary error variant; parse-fields.md and parse-pointers.md replace it.

## The grammar this doc accepts

A literal parses when its root level holds exactly one chunk and that chunk is:

```
entrypoint <Identifier> . <Identifier>
```

with nothing after the second identifier. The earlier passes decide what reaches this stage: leading line breaks were captured by the literal's start, a comma no item precedes was dropped by chunking with a `CommaWithoutItem` error beside the tree, and an unmatched bracket cut its level's tail at the matcher. So `,entrypoint Query.foo` parses at this stage — the comma was chunking's error — and `entrypoint Query.foo)` parses at this stage — the `)` was the matcher's, and the cut removed it. `parse_singleton` rejects a comma in the declaration chunk's boundary: `entrypoint Query.foo,` is `Expected(EndOfDeclaration, Token(Comma))` at the comma. The normal literal style parses:

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

Everything else produces an `UnparsedLiteral` holding one reason and the entire root level.

## New module: chunk_stream.rs

The `ItemCursor` / `ChunkStream` subset this grammar uses. Later docs add methods to the same impl. `ItemCursor` and `ChunkStream` are `pub(crate)` and are not re-exported. This doc lands `require_token`, `token_text`, `end_span`, `text`, `missing`, `new`, `cursor`, and `require_end`.

`new` takes `NonEmpty<WithSpan<ChunkContentItem>>` (chunk contents) and `&str`.

## Changes to chunk.rs

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    pub(crate) fn stream<'a>(&'a self, text: &'a str) -> ChunkStream<'a> {
        ChunkStream::new(&self.contents, text)
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
    end_expectation: Expectation,
) -> Result<T, WithSpan<ParseError>> {
    let mut chunks = level.item.0.iter();
    let Some(chunk) = chunks.next() else {
        return Err(empty());
    };
    let mut stream = chunk.item.stream(text);
    let item = parse(stream.cursor())?;
    stream.require_end(end_expectation)?;
    if let Some(comma) = chunk.item.boundary_comma() {
        return Err(WithSpan::new(
            ParseError::expected(end_expectation, Found::Token(NonBracketTokenKind::Comma)),
            comma,
        ));
    }
    if let Some(more) = chunks.next() {
        return Err(extra(more));
    }
    Ok(item)
}
```

`ChunkedLevel`'s vec becomes a private field of the `chunk` module, so only `parse_singleton` (and later `parse_items`) iterates it.

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
    #[cfg(test)]
    pub(crate) fn chunks(&self) -> &[WithSpan<Chunk>] {
        &self.0
    }
}
```

`parse_singleton` is in that module. Tests outside `chunk.rs` that read the vec use `chunks()`.

`parse_level_items`, `LevelSlot`, and `contents_span` wait for parse-fields.md.

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

`Expectation` is the one global enum parsing-standards.md locks. Later docs add variants. Contextual suggestions (directive migration on a found `@`, and the like) are the rendering stage's concern, keyed off the `(expected, found)` pair; this crate carries only the structural facts.

## New module: parse_iso_literal.rs

The types, most important first. Span conventions follow the crate: the enum is wrapped once at the top, variant payloads are bare, every struct field carries its own span, and a declaration's tight span is derivable from its fields' spans, the way a chunk's is.

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

fn try_parse(
    text: &str,
    root: &WithSpan<ChunkedLevel>,
) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    parse_singleton(
        root,
        text,
        || WithSpan::new(ParseError::EmptyLiteral, root.location),
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        parse_declaration,
        Expectation::EndOfDeclaration,
    )
}

fn parse_declaration(cursor: &mut ItemCursor<'_>) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let keyword = cursor.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::DeclarationKeyword,
    )?;
    match cursor.token_text(keyword) {
        text if text == "entrypoint" => {
            Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, cursor)?))
        }
        text if text == "field" || text == "pointer" => {
            Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword))
        }
        _ => Err(WithSpan::new(
            ParseError::expected(
                Expectation::DeclarationKeyword,
                Found::Token(NonBracketTokenKind::Identifier),
            ),
            keyword,
        )),
    }
}

fn parse_entrypoint(
    keyword: Span,
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    let dot = cursor.require_token(
        NonBracketTokenKind::Period,
        Expectation::Token(NonBracketTokenKind::Period),
    )?;
    let client_field_name = cursor.require_token(
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
```

`parse_declaration` returns after the last item of the declaration. `parse_singleton` calls `require_end`, checks `boundary_comma`, and returns `Err` on a second chunk or an empty level. An incomplete declaration split across a line break reports its own error; only a complete declaration followed by more content reports `MultipleDeclarations`.
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

After (`chunk_stream` has no re-export; `ItemCursor` and `ChunkStream` are `pub(crate)`):

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
    fn a_comma_before_a_second_declaration_reports_the_comma() {
        let text = "entrypoint Query.foo, field User.name";
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::Token(Comma)),
            span_of(text, ","),
        );
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

1. chunk_stream.rs (`ItemCursor` / `ChunkStream` subset), `Chunk::stream`, `boundary_comma`, `parse_singleton`, parse_error.rs, parse_iso_literal.rs, the lib.rs registrations, the `IsographResolutionNode` and `ChunkedLevelParent` changes, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
