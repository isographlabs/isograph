# Semantic tokens

The grammar stage records a semantic token as it commits each token or bracket. `ItemCursor::peek` does not take a `SemanticToken`. `CursorPeek::commit` takes the `SemanticToken` the call site is consuming and records it. `consume_token_if` / `require_token` / `consume_group_if` / `require_group` take that token and pass it to `commit`. The tree does not mention tokens; they are a sibling of the tree, not a field on it.

One shippable change. The collector becomes a type parameter so the parse can be constructed with a noop or a non-noop.

## What a token is

A `SemanticToken` is a role. `require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)` records `Keyword`. The same identifier kind records `Type`, `FieldName`, `ObjectKey`, or `GraphQLTypeName` at the call sites that consume those roles.

```rust
// from crates/isograph_parser/src/semantic_token.rs
use span::{Span, WithSpan};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SemanticToken {
    Keyword,
    Type,
    FieldName,
    ObjectKey,
    GraphQLTypeName,
    DirectiveName,
    Variable,
    Argument,
    Integer,
    String,
    BooleanOrNull,
    Period,
    Colon,
    Equals,
    Parenthesis,
    Brace,
    Content,
    Bracket,
    Error,
}
```

Grammar consume names the role. Unparsed extra, extra chunks, and the matcher's cut are classified later by `leftover_token`, a free function only leftover fill-in calls. extra and extra_chunks are leftover-semantic-tokens.md. The matcher's cut stays this doc's leftover fill-in.

Call sites, by variant:

- `Keyword`: `entrypoint`, `field`, `to`.
- `Type`: `Query` / `User` in `Type.fieldName`.
- `FieldName`: `foo` in `Query.foo`, a selection name, and a selection alias. The first identifier of `alias: name` is consumed before the colon is visible; `SafePeekable` has one item of lookahead, so that identifier is `FieldName` on both arms.
- `ObjectKey`: an object-literal key.
- `GraphQLTypeName`: a type annotation's name, `!`, and a type-list `[]` the grammar consumes (`Foo`, `Foo!`, `[Foo]`). This variant goes away when type annotations leave the language.
- `DirectiveName`: `@` and the directive identifier the grammar consumes.
- `Variable`: `$` and the variable identifier the grammar consumes.
- `Argument`: an argument name.
- `Integer`: an integer literal the grammar consumes. Leftover fill-in reuses this for an unparsed integer.
- `String`: a string or block string the grammar consumes, including a description. Leftover fill-in reuses this for an unparsed string.
- `BooleanOrNull`: `true` / `false` / `null`.
- `Period`: `.` in `Type.fieldName`.
- `Colon`: alias, argument, type-annotation, and object-entry colons the grammar consumes.
- `Equals`: a variable default.
- `Parenthesis`: `(` and `)` the grammar consumes.
- `Brace`: `{` and `}` the grammar consumes.
- `Content`: leftover fill-in only. Identifiers, `@`, `!`, `$`, `.`, `:`, `=`, `,`, and the other non-bracket content kinds that no consume covered.
- `Bracket`: list-value `[` and `]` the grammar consumes, and leftover `(`, `)`, `{`, `}`, `[`, `]` that no consume covered. Type-list brackets stay `GraphQLTypeName`.
- `Error`: leftover fill-in only. An `Error` token.

Open and close of one `BracketKind` share one token. `consume_group_if(kind, token, parse_inside)` records `token` on the open, runs `parse_inside`, and records the same `token` on the close when `parse_inside` returns.

No `line_behavior` / `indent_change` on this type. Upstream puts both on `IsographSemanticToken` and the formatter walks that vec. Formatter metadata is a later change on this same type.

Tokens stay off the tree. Upstream stores `semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>` on each declaration (`entrypoint_declaration.rs`, `client_selectable_declaration.rs`). We do not.

## Origin: how isograph records during parse

Extracted from `crates/isograph_lang_parser/src/peekable_lexer.rs`. Every advance takes a legend constant and pushes it.

```rust
// from crates/isograph_lang_parser/src/peekable_lexer.rs (upstream)
pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    end_index_of_last_parsed_token: u32,
    offset: u32,
    semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>,
    pub text_source: TextSource,
}

impl<'source> PeekableLexer<'source> {
    fn parse_token(
        &mut self,
        isograph_semantic_token: IsographSemanticToken,
    ) -> WithEmbeddedLocation<IsographLangTokenKind> {
        let kind = self
            .lexer
            .next()
            .unwrap_or(IsographLangTokenKind::EndOfFile);
        self.end_index_of_last_parsed_token = self.current.span.end;
        let span = self.lexer_span();
        let parsed_token = std::mem::replace(&mut self.current, kind.with_span(span))
            .to_with_embedded_location(self.text_source);
        self.semantic_tokens
            .push(isograph_semantic_token.with_location(parsed_token.location));
        parsed_token
    }

    pub fn parse_token_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithEmbeddedLocation<IsographLangTokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            self.parse_token(isograph_semantic_token).wrap_ok()
        } else {
            parse_token_kind_diagnostic(expected_kind, found.item, found.location).wrap_err()
        }
    }
}
```

Call sites pass the class:

```rust
// from crates/isograph_lang_parser/src/parse_iso_literal.rs (upstream)
let parent_type = tokens
    .parse_string_key_type(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_SERVER_OBJECT_TYPE,
    )?;
let dot = tokens
    .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;
```

Delta from that extract:

- `ItemCursor::peek` does not take a `SemanticToken`. `CursorPeek::commit` takes the `SemanticToken` and records. `require_token` / `consume_token_if` take the kind and the token and pass the token to `commit`. `require_group` / `consume_group_if` take the `BracketKind`, the token, and `parse_inside`.
- Open and close share one token. Upstream splits `ST_OPEN_PAREN` / `ST_CLOSE_PAREN` (and the brace pair) for formatter metadata.
- One variant per role. Upstream's `ST_DIRECTIVE_AT` / `ST_DIRECTIVE` are both `DirectiveName`; `ST_VARIABLE_DOLLAR_DECLARATION` / `ST_VARIABLE_DOLLAR_USAGE` / `ST_VARIABLE` are `Variable`; `ST_KEYWORD_USE` / `ST_KEYWORD_DECLARATION` / `ST_TO` are `Keyword`; `ST_SERVER_OBJECT_TYPE` is `Type`; `ST_TYPE_ANNOTATION` and `!` are `GraphQLTypeName`; `ST_CLIENT_SELECTABLE_NAME` / `ST_SELECTION_NAME_OR_ALIAS` / `ST_SELECTION_NAME_OR_ALIAS_POST_COLON` are `FieldName`; `ST_OBJECT_LITERAL_KEY` is `ObjectKey`; `ST_STRING_LITERAL` covers string and block string.
- The constructor does not push a dummy token and pop it. Upstream `PeekableLexer::new` does `parse_token(ST_COMMENT)` then `semantic_tokens.pop()`.
- A failed `require_token` records nothing. A successful consume that a later `?` discards stays recorded. There is no corrective pop.
- The declaration types do not grow a `semantic_tokens` field.
- Recording uses `with_span`. `with_location` stays on `WithLocationPostfix`; the parser has only `Span`s, so every call site is `with_span`.

## 1. Construct with a noop or a non-noop

The collector becomes a type parameter. Two implementors, by design; this seam is the whole of what the trait exists for. The parse is constructed with one or the other.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub trait SemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span);
}

pub struct CollectedSemanticTokens(pub Vec<WithSpan<SemanticToken>>);

pub struct NoSemanticTokens;

impl CollectedSemanticTokens {
    pub fn new() -> CollectedSemanticTokens {
        CollectedSemanticTokens(Vec::new())
    }
}

impl NoSemanticTokens {
    pub fn new() -> NoSemanticTokens {
        NoSemanticTokens
    }
}

impl SemanticTokens for CollectedSemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.0.push(token.with_span(span));
    }
}

impl SemanticTokens for NoSemanticTokens {
    fn record(&mut self, _token: SemanticToken, _span: Span) {}
}
```

`record` takes `(SemanticToken, Span)`, not a pre-built `WithSpan`. `CollectedSemanticTokens` constructs the `WithSpan` inside `record`. `NoSemanticTokens::record` is empty, so that monomorphization does not build a `WithSpan`.

### Cursor and helpers

`ItemCursor` / `ChunkStream` gain `TTokens`. The bound lives on the `impl`, not the struct. Every `&mut Vec<WithSpan<SemanticToken>>` becomes `&mut TTokens`. Grammar functions become generic; their bodies do not mention `TTokens`. Consume signatures stay `(kind, token)`. `peek` still takes no `SemanticToken`. `CursorPeek::commit` still takes the token and records through `TTokens::record`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
}

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a, TTokens> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut TTokens,
}

pub(crate) struct ChunkStream<'a, TTokens>(ItemCursor<'a, TTokens>);

pub(crate) struct CursorPeek<'c, 'a, TTokens> {
    peek: Peek<'c, &'a WithSpan<ChunkContentItem>>,
    previous_end: &'c mut u32,
    tokens: &'c mut TTokens,
}

impl<'a, TTokens: SemanticTokens> ItemCursor<'a, TTokens> {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.record(token, span);
    }
}

impl<'c, 'a, TTokens: SemanticTokens> CursorPeek<'c, 'a, TTokens> {
    pub(crate) fn commit(self, token: SemanticToken) -> &'a WithSpan<ChunkContentItem> {
        let item = self.peek.commit();
        *self.previous_end = item.location.end;
        let span = match item.item.reference() {
            ChunkContentItem::NonBracket(_) => item.location,
            ChunkContentItem::Group(group) => group.opening.location,
        };
        self.tokens.record(token, span);
        item
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint<TTokens: SemanticTokens>(
    cursor: &mut ItemCursor<'_, TTokens>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

`parse_iso_literal_item`, `parse_selection`, `parse_value`, and the rest of the grammar functions are the same substitution.

`Chunk::stream`, `parse_chunk`, `parse_chunk_item`, `parse_singleton`, `parse_chunk_item_list`, and `parse_group_*` are generic over `TTokens: SemanticTokens` and take `&mut TTokens`. Closures receive `&mut ItemCursor<'_, TTokens>`.

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P, TTokens: SemanticTokens>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut TTokens,
    parse_item: impl FnOnce(&mut ItemCursor<'a, TTokens>) -> Result<P, WithSpan<ParseError>>,
) -> (
    ChunkStream<'a, TTokens>,
    Result<WithSpan<P>, WithSpan<ParseError>>,
) {
    let mut stream = chunk.item.stream(text, tokens);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}
```

### Entry point

`parse_iso_literal` is generic. The caller constructs the collector.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal<TTokens: SemanticTokens>(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut TTokens,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    singleton.with_span(location).wrap_some()
}
```

Non-noop:

```rust
let mut tokens = CollectedSemanticTokens::new();
let tree = parse_iso_literal(text, root, push_error, &mut tokens);
// tokens.0 is the vec
```

Noop:

```rust
let tree = parse_iso_literal(text, root, push_error, &mut NoSemanticTokens::new());
```

The tree is the same either way.

### Tests

The existing recording facts still hold against `CollectedSemanticTokens`. Added:

- `parse_iso_literal` with `NoSemanticTokens::new()` returns the same tree as with `CollectedSemanticTokens::new()`.
- `stream_of` is generic. Existing consume tests that do not assert tokens pass `&mut NoSemanticTokens::new()`. Collecting tests pass `&mut CollectedSemanticTokens::new()` and assert on `.0`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a, TTokens: SemanticTokens>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut TTokens,
    ) -> ChunkStream<'a, TTokens> {
        first_chunk(tree).stream(text, tokens)
    }
```

### Docs this change amends

- parsing-standards.md: `ItemCursor` / `ChunkStream` gain `TTokens`. List helpers are generic over `TTokens: SemanticTokens`. Grammar functions take `ItemCursor<'_, TTokens>`. Catalog replaces the vec with `SemanticTokens`, `CollectedSemanticTokens::new`, `NoSemanticTokens::new`.
- parse-entrypoint.md: `parse_iso_literal` is generic over `TTokens`.
- parse-arguments.md, parse-selection-sets.md, parse-fields.md, parse-variables.md, parse-descriptions.md, optional-to.md: each `parse_*` gains `TTokens: SemanticTokens`.
- spanless-parsing.md: the cheap pass is `NoSemanticTokens::new()` plus `NoSpan`.

## Later changes

Each is independently shippable.

### Drop `GraphQLTypeName`

When type annotations leave the language, `GraphQLTypeName` leaves this enum. The type-annotation call sites go with it.

### Leftover fill-in

A walk over `tokenize(text)` that emits `leftover_token` for every token whose span is not already in the collected vec, in source order. Those spans were never passed to `commit`: the matcher's cut. `Slot.extra`, `Singleton.extra_chunks`, and separator commas are leftover-semantic-tokens.md. The collected vec stays sorted by span. This is the LSP layer, not the parser's consume path. It runs against `CollectedSemanticTokens`.

`leftover_token` is a free function leftover fill-in calls. leftover-semantic-tokens.md's extra walk calls it too. It picks one of four format buckets, or `Error`.

```rust
// from crates/isograph_parser/src/semantic_token.rs
// Only leftover fill-in: extra, extra_chunks, the matcher's cut.
pub(crate) fn leftover_token(kind: SplitToken) -> Option<SemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            SemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => SemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => SemanticToken::Error.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => SemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => SemanticToken::Bracket.wrap_some(),
    }
}
```

`Content` is identifiers, `@`, `!`, `$`, `.`, `:`, `=`, `,`. `Bracket` is any leftover `(`, `)`, `{`, `}`, `[`, `]`. A leftover `@lazy` is `Content` at `@` and `Content` at `lazy`. A leftover `$` is `Content`. A leftover `!` is `Content`. A leftover `[` is `Bracket`.

Facts:

- `leftover_token` on `Identifier`, `At`, `Exclamation`, `Dollar`, `Period`, `Colon`, `Equals`, and `Comma` is `Content`.
- `leftover_token` on `IntegerLiteral` is `Integer`. On `StringLiteral` and `BlockStringLiteral` is `String`. On `Error` is `Error`.
- `leftover_token` on each `BracketToken` is `Bracket`.
- `leftover_token` on `LineBreak` and `EndOfFile` is `None`.
- extra / extra_chunks after-fill-in facts are leftover-semantic-tokens.md.

### Formatter metadata

`SemanticToken` gains `line_behavior` and `indent_change`, the fields on upstream `IsographSemanticToken`. The formatter walks `Vec<WithSpan<SemanticToken>>` the way `crates/isograph_lsp/src/format.rs` walks the upstream vec.

### Cheap pass

spanless-parsing.md's `TSpan = NoSpan` parse is constructed with `NoSemanticTokens::new()`. The compile path:

```rust
let tree = parse_iso_literal::<NoSpan>(
    text,
    root,
    push_error,
    &mut NoSemanticTokens::new(),
);
if /* push_error was called, or tree.has_errors() */ {
    let mut tokens = CollectedSemanticTokens::new();
    let spanned = parse_iso_literal::<Span>(text, root, push_error, &mut tokens);
    report(spanned);
}
```

The LSP path always constructs with `CollectedSemanticTokens::new()` and `TSpan = Span`. One function, two instantiations of each parameter, same consume-time recording. The two trees cannot disagree about structure.

`TSpan` is the tree's parameter. `TTokens` is the cursor's. They are chosen together at the entry point by which values are constructed. `require_token` names neither parameter.
