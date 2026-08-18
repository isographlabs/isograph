# parse-entrypoint: the grammar stage's skeleton, and entrypoint declarations

First grammar feature, written against parsing-standards.md. The shared surface lands first. This doc lands `parse_iso_literal` and `entrypoint Type.field`. `field` and `pointer` are identifiers that return `UnsupportedDeclarationType`; parse-fields.md and parse-pointers.md replace those arms.

## The grammar

```
entrypoint <Identifier> . <Identifier>
```

The root is a one-item context, not a list. Chunk 0 goes through `parse_one_item` into `Slot<IsoLiteralItem, UnparsedChunkItems>`. Remaining chunks are `IsoLiteralParse.extra_chunks` plus `MultipleDeclarations` on the first extra chunk. A boundary comma is a tokenless diagnostic. Empty is `EmptyLiteral` and `None` from `parse_iso_literal`. `Slot.item` is `Some` when the form parsed.

```
iso(`
  entrypoint Query.PetDetailRoute
`)
```

A failed first chunk is `item: None` plus that chunk’s items in `Slot.extra_tokens`. Extra chunks sit in `IsoLiteralParse.extra_chunks`.

## Types

Most important first. The wrapping `WithSpan` on `IsoLiteralParse` is the whole literal (`root.location`). `Singleton.item` is `WithSpan<Slot<...>>`, the `parse_one_item` attempt. `Slot` / `Singleton` are the optimistic tree and impl `ResolvePosition`. `IsoLiteralParse` is the alias. Empty is `None`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::chunk_stream::ItemCursor;
use crate::{
    ChunkedLevel, Expectation, Found, ExtraChunks, IsographResolutionNode, NonBracketTokenKind,
    ParseError, Singleton, Slot, UnparsedChunkItems, parse_singleton,
};

pub type IsoLiteralParse = Singleton<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;

pub type SlotPath<'a> = PositionResolutionPath<
    &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
    IsoLiteralParsePath<'a>,
>;

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName(common_lang_types::EntityName);

impl From<intern::string_key::StringKey> for EntityName {
    fn from(key: intern::string_key::StringKey) -> Self {
        EntityName(key.to())
    }
}

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName(SelectableName);

impl From<intern::string_key::StringKey> for ClientFieldName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ClientFieldName(key.to())
    }
}

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, SlotPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> = PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

## The parser

The root is borrowed until the end. A failed first chunk clones that chunk's items into `Slot.extra_tokens`. Extra chunks after the first are cloned into `IsoLiteralParse.extra_chunks`. On a parsed first slot with no extra the root `ChunkedLevel` is dropped. Diagnostics go through `push_error`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    push_error: impl FnMut(WithSpan<ParseError>),
) -> Option<WithSpan<IsoLiteralParse>> {
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
            IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok()
        }
        text if text == "field" || text == "pointer" => {
            ParseError::UnsupportedDeclarationType.with_span(keyword).wrap_err()
        }
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
}

fn parse_entrypoint(
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
        parent_type: cursor
            .token_text(parent_type)
            .intern()
            .to::<EntityName>()
            .with_span(parent_type),
        client_field_name: cursor
            .token_text(client_field_name)
            .intern()
            .to::<ClientFieldName>()
            .with_span(client_field_name),
    }.wrap_ok()
}
```

`parse_iso_literal` wraps `parse_iso_literal_item`: `parse_singleton` is the tree. Artifact generation requires that `push_error` was never called (and the earlier-stage lists empty). Resolve walks the optimistic tree only. `parse_iso_literal_item` is the keyword dispatch. After `entrypoint` it calls `parse_entrypoint`. Empty is `None` plus `EmptyLiteral` through `push_error`. A failed first chunk is `item: None` plus that chunk’s items; extra chunks still sit in `IsoLiteralParse.extra_chunks`. `entrypoint Query.foo\nfield User.name` is a parsed first slot plus `IsoLiteralParse.extra_chunks` and `push_error(MultipleDeclarations)`. `entrypoint Query.foo bar` is `item: Some` plus leftover items and `push_error(Expected(EndOfDeclaration, Identifier))`. `entrypoint Foo.$ asdf` is `item: None`: the error is at `$`, `extra_tokens` is the whole chunk. `entrypoint\nQuery.foo` is `item: None` on `entrypoint` plus `IsoLiteralParse.extra_chunks` for `Query.foo`. `entrypoint Query.foo,` is `item: Some` plus a tokenless comma diagnostic through `push_error`.

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
    Separator,
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
            Expectation::Separator => write!(f, "a comma or line break"),
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
    IsoLiteralParsePath, NonBracketTokenPath, OpenBracketPath, SlotPath,
    UnparsedChunkItemsPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    Singleton(IsoLiteralParsePath<'a>),
    Slot(SlotPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
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

`ChunkedLevelParent` stays. A bare chunk tree still resolves with `Root`. Group interiors still use `Interior`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkedLevelPath<'a>>;

pub struct ChunkedLevel(#[resolve_field] pub Vec<WithSpan<Chunk>>);

#[resolve_position(parent_type = ChunkedLevelPath<'a>, resolved_node = IsographResolutionNode<'a>)]
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

#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}

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

pub type ChunkedLevelPath<'a> = PositionResolutionPath<&'a ChunkedLevel, ChunkedLevelParent<'a>>;

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkParent<'a>>;

pub type ChunkedGroupPath<'a> = PositionResolutionPath<&'a ChunkedGroup, ChunkContentItemParent<'a>>;

pub type NonBracketTokenPath<'a> =
    PositionResolutionPath<&'a NonBracketToken, ChunkContentItemParent<'a>>;

pub struct ChunkedLevel(#[resolve_field(parent_variant = Level)] Vec<WithSpan<Chunk>>);

#[resolve_position(parent_type = ChunkParent<'a>, resolved_node = IsographResolutionNode<'a>)]
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

#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);
```

Extra root chunks use `parent_variant = Extra`. Leftover and failed items use `parent_variant = Unparsed`. `ChunkContentItem` is transparent, so `NonBracketToken` and `ChunkedGroup` parent at `ChunkContentItemParent`.

Existing `chunk.rs` resolve tests that walk `token.parent` or `open.parent.parent` change. `ChunkSeparator.parent` is still a `ChunkPath`; `.inner` is still the chunk.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => {
                assert_eq!(open.parent.inner.closing.item.0, Brace);
                assert_eq!(render_chunk(text, open.parent.parent.inner), "foo { bar }");
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
```

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert!(matches!(
                    token.parent.parent.parent,
                    ChunkedLevelParent::Interior(_)
                ));
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
```

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert_eq!(render_chunk(text, token.parent.inner), "bar,");
                match token.parent.parent.parent.reference() {
                    ChunkedLevelParent::Interior(group) => {
                        assert_eq!(render_chunk(text, group.parent.inner), "foo { bar, baz }");
                    }
                    parent => panic!("expected an interior level, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => {
                assert_eq!(
                    render_chunk(text, open.parent.parent.inner),
                    "foo { bar, baz }"
                );
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => {
                assert_eq!(open.parent.inner.closing.item.0, Brace);
                match open.parent.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => {
                        assert_eq!(render_chunk(text, chunk.inner), "foo { bar }");
                    }
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
```

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                match token.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => match chunk.parent.reference() {
                        ChunkParent::Level(level) => {
                            assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
                        }
                        parent => panic!("expected a level parent, got {parent:?}"),
                    },
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
```

```rust
// from crates/isograph_parser/src/chunk.rs
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                match token.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => {
                        assert_eq!(render_chunk(text, chunk.inner), "bar,");
                        match chunk.parent.reference() {
                            ChunkParent::Level(level) => match level.parent.reference() {
                                ChunkedLevelParent::Interior(group) => {
                                    assert_eq!(
                                        render_chunk(text, group.parent.inner),
                                        "foo { bar, baz }"
                                    );
                                }
                                parent => panic!("expected an interior level, got {parent:?}"),
                            },
                            parent => panic!("expected a level parent, got {parent:?}"),
                        }
                    }
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => {
                match open.parent.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => {
                        assert_eq!(render_chunk(text, chunk.inner), "foo { bar, baz }");
                    }
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
```

## Generated code

`Slot<IsoLiteralItem, UnparsedChunkItems>` tries `item` then `extra_tokens`; leftover gap is `Slot`. `Singleton<...>` tries `item` then `extra_chunks`; leftover gap is `Singleton`. `UnparsedChunkItems` iterates the `NonEmpty`. `ExtraChunks` iterates the `NonEmpty`. The enum delegation, struct descent, and fieldless-marker impls follow chunk.rs. Resolve walks the optimistic tree only.

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
    use NonBracketTokenKind::{At, Comma, Dollar, Identifier, IntegerLiteral, Period};

    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (
            parse.expect("the fixture is not an empty literal"),
            errors,
        )
    }

    fn parsed_with_errors(
        text: &str,
    ) -> (
        Option<WithSpan<IsoLiteralParse>>,
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

    fn first_slot(
        parse: &WithSpan<IsoLiteralParse>,
    ) -> &Slot<IsoLiteralItem, UnparsedChunkItems> {
        parse.item.item.item.reference()
    }

    fn parsed_item(parse: &WithSpan<IsoLiteralParse>) -> Option<&IsoLiteralItem> {
        first_slot(parse).item.as_ref().map(|wrapped| wrapped.item.reference())
    }

    fn as_entrypoint(parse: &WithSpan<IsoLiteralParse>) -> &EntrypointDeclaration {
        let item = parsed_item(parse).expect("the fixture's literal parsed an item");
        match item {
            IsoLiteralItem::Entrypoint(declaration) => declaration,
        }
    }

    fn assert_no_declaration(text: &str, reason: ParseError, reason_span: Span) {
        let (parse, errors) = parsed(text);
        assert!(
            parsed_item(parse.reference()).is_none(),
            "for literal {text:?}",
        );
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
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            assert!(parse.is_none(), "for literal {text:?}");
            assert_eq!(
                errors,
                ParseError::EmptyLiteral
                    .with_span(Span::from_usize(0, text.len()))
                    .wrap_vec(),
                "for literal {text:?}",
            );
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
        assert!(parse.is_none());
        assert_eq!(
            errors,
            ParseError::EmptyLiteral
                .with_span(Span::from_usize(0, text.len()))
                .wrap_vec(),
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
                expected(EndOfDeclaration, Found::Token(Comma))
                    .with_span(span_of(text, ","))
                    .wrap_vec(),
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
                expected(EndOfDeclaration, Found::Token(Comma)).with_span(span_of(text, ",")),
                ParseError::MultipleDeclarations.with_span(span_of(text, "field User.name")),
            ],
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(text);
        assert_eq!(as_entrypoint(parse.reference()).client_field_name.location, span_of(text, "foo"));
        assert_eq!(
            errors,
            ParseError::MultipleDeclarations
                .with_span(span_of(text, "field User.name"))
                .wrap_vec(),
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_failed_first_chunk_is_reported_even_when_a_second_exists() {
        let text = "entrypoint\nQuery.foo";
        let keyword_end = span_of(text, "entrypoint").end;
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(keyword_end, keyword_end)
        }));
        assert!(errors.iter().any(|error| error.item == ParseError::MultipleDeclarations));
        assert!(parse.item.extra_chunks.as_ref().is_some());
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
    fn a_failed_form_keeps_the_whole_chunk_as_remaining() {
        let text = "entrypoint Foo.$ asdf";
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        match first_slot(parse.reference()).extra_tokens.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(items) => {
                assert_eq!(items.0.first().location, span_of(text, "entrypoint"));
                assert_eq!(items.0.last().location, span_of(text, "asdf"));
            }
            None => panic!("expected remaining items covering the whole chunk"),
        }
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
                && error.location == span_of(text, "$")
        }));
    }

    #[test]
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert!(parsed_item(parse.reference()).is_some());
        assert!(first_slot(parse.reference()).extra_tokens.as_ref().is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_selection_set_on_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo { bar }";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Group(BracketKind::Brace))
                .with_span(span_of(text, "{ bar }"))
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
            expected(EndOfDeclaration, Found::Token(At))
                .with_span(span_of(text, "@"))
                .wrap_vec(),
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

1. parse_iso_literal.rs, the `ParseError` variants this form adds, the `IsographResolutionNode`, `ChunkParent`, and `ChunkContentItemParent` changes this form needs, the existing `chunk.rs` parent walks, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
