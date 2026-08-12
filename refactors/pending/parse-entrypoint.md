# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

Second doc of the series parsing-plan.md orders, atop the invariant one-comma-per-boundary.md lands: every boundary holds at most one comma, and a doubled comma surfaces as an empty chunk. This doc lands `parse_iso_literal`, the root-level rules, keyword dispatch, `ParseError`, the whole-literal failure fallback with its resolution path, and the complete `entrypoint Type.field` declaration. `field` and `pointer` are recognized keywords that dispatch to a temporary error variant; parse-fields.md and parse-pointers.md replace it.

## The grammar this doc accepts

A literal parses when its root level holds exactly one chunk with contents and that chunk is:

```
entrypoint <Identifier> . <Identifier>
```

with nothing after the second identifier. Leading and trailing separators around the declaration are insignificant, a single comma included; the normal literal style parses:

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

Everything else produces an `UnparsedLiteral` holding one reason and the entire root level.

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
    UnmatchedOpen(BracketKind),
    UnmatchedClose(BracketKind),
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
            ChunkContentItem::UnmatchedOpen(open) => Found::UnmatchedOpen(open.0),
            ChunkContentItem::UnmatchedClose(close) => Found::UnmatchedClose(close.0),
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
            Found::UnmatchedOpen(kind) => {
                write!(f, "an unmatched {}", opening_bracket_text(*kind))
            }
            Found::UnmatchedClose(kind) => {
                write!(f, "an unmatched {}", closing_bracket_text(*kind))
            }
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

fn closing_bracket_text(kind: BracketKind) -> &'static str {
    match kind {
        BracketKind::Parenthesis => "')'",
        BracketKind::Brace => "'}'",
        BracketKind::Bracket => "']'",
    }
}
```

Contextual suggestions (directive migration on a found `@`, and the like) are the rendering stage's concern, keyed off the `(expected, found)` pair; this crate carries only the structural facts.

## New module: parse_iso_literal.rs

The types, most important first. Span conventions follow the crate: the enum is wrapped once at the top, variant payloads are bare, every struct field carries its own span, and a declaration's tight span is derivable from its fields' spans, the way a chunk's is.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkContentItem, ChunkedLevel, Expectation, Found, IsographResolutionNode,
    NonBracketTokenKind, ParseError,
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

/// The declaration chunk parses before the extra-chunk check so that an incomplete
/// declaration split across a line break reports its own precise error, and only a
/// complete declaration followed by more content reports `MultipleDeclarations`.
fn try_parse(text: &str, root: &WithSpan<ChunkedLevel>) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let (declaration, extra) = declaration_chunk(root)?;
    let parse = parse_declaration_chunk(text, declaration)?;
    if let Some(extra) = extra {
        return Err(WithSpan::new(ParseError::MultipleDeclarations, extra));
    }
    Ok(parse)
}

/// The root level's one chunk with contents, plus the joined span of any further
/// contentful chunks. A first chunk with no contents holds the literal's leading
/// separators; an empty chunk anywhere else is a doubled comma
/// (one-comma-per-boundary.md), and its span is that comma plus the line breaks it
/// absorbed.
fn declaration_chunk(
    root: &WithSpan<ChunkedLevel>,
) -> Result<(&WithSpan<Chunk>, Option<Span>), WithSpan<ParseError>> {
    let mut declaration = None;
    let mut extra = None;
    for (index, chunk) in root.item.0.iter().enumerate() {
        if chunk.item.contents.is_empty() {
            if index == 0 {
                continue;
            }
            let expected = match declaration {
                None => Expectation::DeclarationKeyword,
                Some(_) => Expectation::EndOfDeclaration,
            };
            return Err(WithSpan::new(
                ParseError::expected(expected, Found::Token(NonBracketTokenKind::Comma)),
                chunk.location,
            ));
        }
        match declaration {
            None => declaration = Some(chunk),
            Some(_) => {
                extra = Some(match extra {
                    None => chunk.location,
                    Some(span) => Span::join(span, chunk.location),
                });
            }
        }
    }
    let Some(declaration) = declaration else {
        return Err(WithSpan::new(ParseError::EmptyLiteral, root.location));
    };
    Ok((declaration, extra))
}

type ChunkContents<'a> = SafePeekable<std::slice::Iter<'a, WithSpan<ChunkContentItem>>>;

fn parse_declaration_chunk(
    text: &str,
    chunk: &WithSpan<Chunk>,
) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let mut items = chunk.item.contents.iter().safe_peekable();
    let keyword = expect_token(
        &mut items,
        NonBracketTokenKind::Identifier,
        Expectation::DeclarationKeyword,
        chunk.location.start,
    )?;
    match token_text(text, keyword) {
        "entrypoint" => Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, &mut items)?)),
        "field" | "pointer" => Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword)),
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
    items: &mut ChunkContents<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = expect_token(
        items,
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
        keyword.end,
    )?;
    let dot = expect_token(
        items,
        NonBracketTokenKind::Period,
        Expectation::Token(NonBracketTokenKind::Period),
        parent_type.end,
    )?;
    let client_field_name = expect_token(
        items,
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
        dot.end,
    )?;
    expect_chunk_end(items, Expectation::EndOfDeclaration)?;
    Ok(EntrypointDeclaration {
        entrypoint_keyword: WithSpan::new(EntrypointKeyword, keyword),
        parent_type: WithSpan::new(EntityName, parent_type),
        dot: WithSpan::new(Dot, dot),
        client_field_name: WithSpan::new(ClientFieldName, client_field_name),
    })
}

/// The next item's span when it is a non-bracket token of `kind`; the error otherwise,
/// on the found item, unconsumed, or empty at `missing_at` when the chunk ran out.
fn expect_token(
    items: &mut ChunkContents<'_>,
    kind: NonBracketTokenKind,
    expected: Expectation,
    missing_at: u32,
) -> Result<Span, WithSpan<ParseError>> {
    let Some(peek) = items.peek() else {
        return Err(WithSpan::new(
            ParseError::expected(expected, Found::EndOfChunk),
            Span::new(missing_at, missing_at),
        ));
    };
    let item = peek.view();
    match &item.item {
        ChunkContentItem::NonBracket(token) if token.0 == kind => {
            let span = item.location;
            peek.commit();
            Ok(span)
        }
        other => Err(WithSpan::new(
            ParseError::expected(expected, Found::from(other)),
            item.location,
        )),
    }
}

/// The chunk must have no items left; the first leftover is the error.
fn expect_chunk_end(
    items: &mut ChunkContents<'_>,
    expected: Expectation,
) -> Result<(), WithSpan<ParseError>> {
    let Some(peek) = items.peek() else {
        return Ok(());
    };
    let item = peek.view();
    Err(WithSpan::new(
        ParseError::expected(expected, Found::from(&item.item)),
        item.location,
    ))
}

/// The literal text a span covers. The parser reads it only to recognize keywords.
fn token_text(text: &str, span: Span) -> &str {
    &text[span.as_usize_range()]
}
```

An empty declaration chunk cannot reach `parse_declaration_chunk` (`declaration_chunk` filters empties), but no code relies on that: `expect_token`'s ran-out arm answers `Expected(DeclarationKeyword, EndOfChunk)` at the chunk's start.

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

The derive on the enum delegates each variant to its payload with the parent passed through:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for IsoLiteralParse {
    type Parent<'a> = ();
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        match self {
            IsoLiteralParse::Entrypoint(inner) => inner.resolve(parent, position),
            IsoLiteralParse::Unparsed(inner) => inner.resolve(parent, position),
        }
    }
}
```

The declaration descends into its two name fields and answers itself everywhere else, so the keyword, the dot, and interior whitespace resolve to the declaration:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for EntrypointDeclaration {
    type Parent<'a> = ();
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.parent_type.location.contains(position) {
            let new_parent = self.path(parent);
            return self.parent_type.item.resolve(new_parent, position);
        }
        if self.client_field_name.location.contains(position) {
            let new_parent = self.path(parent);
            return self.client_field_name.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::EntrypointDeclaration(self.path(parent).into());
    }
}
```

The unparsed literal wraps its path in the new parent variant and descends into the level; the level's span is the whole literal, so the fallthrough leaf is unreachable in practice but exists as the derive's shape requires:

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

The two name types are fieldless, so their impls are the fallthrough alone:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for EntityName {
    type Parent<'a> = EntrypointDeclarationPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::EntityName(self.path(parent).into());
    }
}
```

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ClientFieldName {
    type Parent<'a> = EntrypointDeclarationPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::ClientFieldName(self.path(parent).into());
    }
}
```

## Tests

In-file, in the pattern of chunk.rs: `span_of` anchors, structural assertions, no snapshots, degenerate cases included.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[cfg(test)]
mod tests {
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::{chunk, match_brackets, tokenize, BracketKind, ChunkedLevelParent};
    use Expectation::{DeclarationKeyword, EndOfDeclaration};
    use NonBracketTokenKind::{At, Comma, Identifier, IntegerLiteral, Period};

    fn parsed(text: &str) -> WithSpan<IsoLiteralParse> {
        let tree = chunk(&match_brackets(tokenize(text), text.len() as u32));
        parse_iso_literal(text, tree)
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
    fn surrounding_separators_and_interior_spaces_are_insignificant() {
        for text in [
            "\n  entrypoint Query.foo\n",
            "\n\nentrypoint Query.foo",
            "entrypoint Query . foo",
            "entrypoint Query.foo,",
            ",entrypoint Query.foo",
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
        for text in ["", "   ", "\n\n", ","] {
            assert_unparsed(text, ParseError::EmptyLiteral, Span::from_usize(0, text.len()));
        }
    }

    #[test]
    fn a_doubled_comma_after_the_declaration_is_an_error_on_the_empty_chunk() {
        let text = "entrypoint Query.foo,,";
        let commas = span_of(text, ",,");
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::Token(Comma)),
            Span::new(commas.start + 1, commas.end),
        );
    }

    #[test]
    fn a_doubled_comma_before_the_declaration_is_an_error_on_the_empty_chunk() {
        let text = ",,entrypoint Query.foo";
        let commas = span_of(text, ",,");
        assert_unparsed(
            text,
            expected(DeclarationKeyword, Found::Token(Comma)),
            Span::new(commas.start + 1, commas.end),
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
    fn an_unmatched_bracket_after_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo)";
        assert_unparsed(
            text,
            expected(EndOfDeclaration, Found::UnmatchedClose(BracketKind::Parenthesis)),
            span_of(text, ")"),
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

1. parse_error.rs, parse_iso_literal.rs, the lib.rs registrations, the `IsographResolutionNode` and `ChunkedLevelParent` changes, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
