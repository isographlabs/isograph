# Format iso literals

Requires iso-format-lsp.md. Independent of lsp-diagnostics.md. Uses open-file contents when lsp-open-files.md change 3 has landed; until then, disk, same as tokens.

iso-format-lsp.md's `textDocument/formatting` and `textDocument/willSaveWaitUntil` handlers return `[]`. This slice records a printable stream at consume, prints from that stream, and those handlers return the edits.

A list group (`{` of a selection set or object, `(` of arguments or variables, `[` of a value list) indents: line break after the opener, line break before the closer. Each successful chunk ends with a comma and a line break. Empty groups stay `{}` `()` `[]`. Type-list `[Foo!]` does not indent: those brackets are `GraphQLTypeName`, glued, including `!`. A description occupies its own line, so `{` after a description is on the next line.

```
user(
  arg: {
    b: true,
  },
  c: false,
)
```

```
field Pet.fullName {
  firstName,
  lastName,
}
```

```
field Pet.fullName
"a pet's name"
{
  firstName,
  lastName,
}
```

```
field Query.user(
  $id: ID,
  $x: Int,
) {
  user(
    id: $id,
    extra: {
      b: true,
    },
  ) {
    id,
    name,
  }
}
```

## What the user does

```
export const Home = iso(`field Query.HomeRoute{id  name}`)
```

Save. The interior is:

```
field Query.HomeRoute {
  id,
  name,
}
```

The `iso(\`` and closing backtick stay. A parsed literal in the same file as a failed parse is still formatted.

`willSaveWaitUntil` (advertised in iso-format-lsp.md) returns those edits. `textDocument/formatting` returns the same edits.

## Recording

Most important first.

`ParsedIsoLiteral.tokens` is `Vec<WithOptionalSpan<IsographSemanticToken>>`. Consume has `Some(span)` because the highlighter needs file positions and the printer needs the source lexeme (`fullName`, `Query`, `{`). `Comma` has `location: None`: it is not in the source. `parse_each_chunk` after a successful item always records that `Comma` (after the last item too). Empty level: no comma. Failed item: no comma. List source commas stay on `trailing_separator` and are not leftover-filled. `leftover_token` is unchanged.

Depth is a counter over list-group open and close. `ResolvePosition` does not enter `None`. The highlighter skips `Comma`.

Leftover remaining contents always `report_error` (`parse_stream`). A failed item does too. `parsed.errors.is_empty()` means a full valid parse: no leftover extra, no failed item. `format_iso_literals` calls `format_recorded` only then. The printer never sees leftover tokens.

`require_token` / `require_group` signatures do not change. `commit` pushes `WithGenericLocation::new(token, span.wrap_some())`.

```rust
// from crates/isograph_parser/src/chunk.rs
    pub(crate) fn parse_each_chunk<P>(
        &self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>> {
        self.0
            .iter()
            .map(|chunk| {
                let slot = parse_one_chunk(
                    chunk,
                    parent.stream_chunk(&chunk.item),
                    leftover,
                    &parse_item,
                );
                if slot.item.item.is_some() {
                    parent.tokens().push(WithGenericLocation::new(
                        IsographSemanticToken::Comma,
                        None,
                    ));
                }
                slot
            })
            .collect()
    }
```

```rust
// from crates/isograph_parser/src/semantic_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IsographSemanticToken {
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
    Description,
    At,
    Exclamation,
    Comma,
}
```

- `Description`: the declaration description. Own line. `String` stays a string value (`name: "x"`).
- `At`: `@`. Space before, glued to `DirectiveName`. `DirectiveName` is the identifier only. `FieldName` is already the selectable name.
- `Exclamation`: `!` after a type. Glued. Today `peek.advance()` records nothing, so `[Foo!]` would drop `!`.
- `Comma`: recorded only by `parse_each_chunk`. Prints `,` and a line break.

```rust
cursor.require_token(
    NonBracketTokenKind::Period,
    IsographSemanticToken::Period,
)
cursor.require_group(
    BracketKind::Brace,
    IsographSemanticToken::Brace,
    parse_inside,
)
cursor.require_token(
    NonBracketTokenKind::Exclamation,
    IsographSemanticToken::Exclamation,
)
```



## Printer

```rust
// from crates/isograph_parser/src/format.rs
pub fn format_recorded(
    text: &str,
    tokens: &[WithOptionalSpan<IsographSemanticToken>],
) -> String {
    let mut out = String::new();
    let mut indent = 0_usize;
    let mut last: Option<IsographSemanticToken> = None;
    for (i, token) in tokens.iter().enumerate() {
        if token.item == IsographSemanticToken::Comma {
            out.push(',');
            last = IsographSemanticToken::Comma.wrap_some();
            continue;
        }
        let span = token.location.expect("only Comma is recorded with location None");
        let lexeme = &text[span.as_usize_range()];
        let role = token.item;
        let delta = list_indent(role, lexeme);
        let empty_open = delta > 0
            && tokens
                .get(i + 1)
                .and_then(|next| lexeme(text, next))
                .is_some_and(|next_lexeme| list_indent(tokens[i + 1].item, next_lexeme) < 0);
        if delta < 0 {
            indent = indent.saturating_sub(1);
        }
        apply_gap(&mut out, gap(last, role, lexeme), indent);
        out.push_str(lexeme);
        if delta > 0 && !empty_open {
            indent += 1;
            apply_gap(&mut out, Gap::Line, indent);
        }
        last = role.wrap_some();
    }
    out
}

fn lexeme<'a>(text: &'a str, token: &WithOptionalSpan<IsographSemanticToken>) -> Option<&'a str> {
    token.location.map(|span| &text[span.as_usize_range()])
}

enum Gap {
    None,
    Space,
    Line,
}

fn list_indent(role: IsographSemanticToken, lexeme: &str) -> i8 {
    match (role, lexeme) {
        (IsographSemanticToken::Brace, "{")
        | (IsographSemanticToken::Parenthesis, "(")
        | (IsographSemanticToken::Bracket, "[") => 1,
        (IsographSemanticToken::Brace, "}")
        | (IsographSemanticToken::Parenthesis, ")")
        | (IsographSemanticToken::Bracket, "]") => -1,
        _ => 0,
    }
}

fn gap(last: Option<IsographSemanticToken>, role: IsographSemanticToken, lexeme: &str) -> Gap {
    match last {
        None => Gap::None,
        Some(IsographSemanticToken::Comma) => Gap::Line,
        Some(prev) => {
            if role == IsographSemanticToken::Description
                || prev == IsographSemanticToken::Description
            {
                Gap::Line
            } else if space_after(prev) || space_before(role, lexeme) {
                Gap::Space
            } else {
                Gap::None
            }
        }
    }
}

fn space_after(role: IsographSemanticToken) -> bool {
    matches!(
        role,
        IsographSemanticToken::Keyword
            | IsographSemanticToken::Colon
            | IsographSemanticToken::Equals
    )
}

fn space_before(role: IsographSemanticToken, lexeme: &str) -> bool {
    matches!(
        role,
        IsographSemanticToken::Keyword
            | IsographSemanticToken::Equals
            | IsographSemanticToken::At
    ) || lexeme == "{"
}

fn apply_gap(out: &mut String, gap: Gap, indent: usize) {
    match gap {
        Gap::None => {}
        Gap::Space => out.push(' '),
        Gap::Line => {
            out.push('\n');
            for _ in 0..indent {
                out.push_str("  ");
            }
        }
    }
}
```

After a list open the printer already wrote a line break, so `last` is the open bracket and the next item glues onto that new line. Type-list `[` `]` are `GraphQLTypeName`; `list_indent` is 0. Width is 2 spaces. A parse failure of a literal is not passed to `format_recorded`.

## Target shape

```
entrypoint Query.HomeRoute

field Pet.fullName {
  firstName,
  lastName,
}

field Query.user(
  $id: ID,
  $x: Int,
) {
  user(
    id: $id,
    extra: {
      b: true,
    },
    c: false,
  ) {
    id,
  }
}

field Pet.Avatar to Image {
  url,
}

field Pet.fullName
"a pet's name"
{
  firstName,
  lastName,
}
```

## Handlers

If request dispatch already passes `LspClientId`, `format_iso_literals` takes it and reads `lsp_file_contents`. If not, it reads `DiskFile` the way `semantic_tokens_response` does.

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}
```

Missing file is `Ok(None)` JSON `null`.

## Changes

Two independently shippable changes.

### Change 1: `WithOptionalSpan`, `Comma` after list items, `Description` / `At` / `Exclamation`

#### `crates/isograph_parser/src/semantic_token.rs`

Before:

```rust
// from crates/isograph_parser/src/semantic_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IsographSemanticToken {
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

pub(crate) fn leftover_token(kind: SplitToken) -> Option<IsographSemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            IsographSemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => IsographSemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => {
            IsographSemanticToken::Error.wrap_some()
        }
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => IsographSemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => IsographSemanticToken::Bracket.wrap_some(),
    }
}
```

After: the enum in Recording. `leftover_token` is unchanged. Delta: four variants on the enum.

#### `crates/isograph_parser/src/chunk_stream.rs`

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    tokens: &'a mut Vec<WithSpan<IsographSemanticToken>>,
```

on `ItemCursor`, `CursorPeek`, `ChunkStream::new`, `from_nonempty`, `from_iter`, `tokens()`, `take_item`.

After: `Vec<WithOptionalSpan<IsographSemanticToken>>` on each of those.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn record(&mut self, token: IsographSemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn record(&mut self, token: IsographSemanticToken, span: Span) {
        self.tokens
            .push(WithGenericLocation::new(token, span.wrap_some()));
    }
```

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn commit(self, token: IsographSemanticToken) -> &'a WithSpan<ChunkContentItem> {
        let (tokens, item) = self.take_item();
        let span = match item.item.reference() {
            ChunkContentItem::NonBracket(_) => item.location,
            ChunkContentItem::Group(group) => group.opening.location,
        };
        tokens.push(token.with_span(span));
        item
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn commit(self, token: IsographSemanticToken) -> &'a WithSpan<ChunkContentItem> {
        let (tokens, item) = self.take_item();
        let span = match item.item.reference() {
            ChunkContentItem::NonBracket(_) => item.location,
            ChunkContentItem::Group(group) => group.opening.location,
        };
        tokens.push(WithGenericLocation::new(token, span.wrap_some()));
        item
    }
```

#### `crates/isograph_parser/src/chunk.rs`

`Chunk::stream`, `record_leftover_item`, `record_leftover_chunk`, `record_leftover_extra`, `record_leftover_span`: `Vec<WithSpan<IsographSemanticToken>>` becomes `Vec<WithOptionalSpan<IsographSemanticToken>>`. Bodies of `record_leftover_item` / `record_leftover_chunk` / `record_leftover_extra` otherwise unchanged.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn record_leftover_span(
    tokens: &mut Vec<WithSpan<IsographSemanticToken>>,
    role: Option<IsographSemanticToken>,
    span: Span,
) {
    if let Some(role) = role
        && tokens.iter().all(|recorded| recorded.location != span)
    {
        tokens.push(role.with_span(span));
    }
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn record_leftover_span(
    tokens: &mut Vec<WithOptionalSpan<IsographSemanticToken>>,
    role: Option<IsographSemanticToken>,
    span: Span,
) {
    if let Some(role) = role
        && tokens
            .iter()
            .all(|recorded| recorded.location != span.wrap_some())
    {
        tokens.push(WithGenericLocation::new(role, span.wrap_some()));
    }
}
```

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    pub(crate) fn parse_each_chunk<P>(
        &self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>> {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_chunk(
                    chunk,
                    parent.stream_chunk(&chunk.item),
                    leftover,
                    &parse_item,
                )
            })
            .collect()
    }
```

After: the function in Recording. Delta: bind `slot`, push `Comma` with `None` when `slot.item.item.is_some()`, return `slot`.

#### `crates/isograph_parser/src/parse_iso_literal.rs`

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub tokens: Vec<WithSpan<IsographSemanticToken>>,
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub tokens: Vec<WithOptionalSpan<IsographSemanticToken>>,
}
```

`parse_chunked_iso_literal`'s `tokens` parameter is the same vec type.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    if let Some(span) = cursor.consume_token_if(
        NonBracketTokenKind::StringLiteral,
        IsographSemanticToken::String,
    ) {
        return Description(span.exclude_ends(1).interned().item)
            .with_span(span.location)
            .wrap_some();
    }
    cursor
        .consume_token_if(
            NonBracketTokenKind::BlockStringLiteral,
            IsographSemanticToken::String,
        )
        .map(|span| {
            Description(intern_block_string_value(span.exclude_ends(3).text()))
                .with_span(span.location)
        })
}
```

After: both `IsographSemanticToken::String` are `IsographSemanticToken::Description`. Body otherwise unchanged.

#### `crates/isograph_parser/src/directives.rs`

Before:

```rust
// from crates/isograph_parser/src/directives.rs
            while let Some(at) = cursor.consume_token_if(
                NonBracketTokenKind::At,
                IsographSemanticToken::DirectiveName,
            ) {
                directives.push(parse_directive_after_at(cursor, at.location)?);
            }
```

After:

```rust
// from crates/isograph_parser/src/directives.rs
            while let Some(at) = cursor.consume_token_if(
                NonBracketTokenKind::At,
                IsographSemanticToken::At,
            ) {
                directives.push(parse_directive_after_at(cursor, at.location)?);
            }
```

`parse_directive_after_at` still `require_token(..., IsographSemanticToken::DirectiveName)`.

#### `crates/isograph_parser/src/variables.rs`

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<AstError>> {
    let core = parse_named_or_list(cursor)?;
    if let Some(peek) = cursor.peek()
        && matches!(
            peek.view().item.reference(),
            ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Exclamation))
        )
    {
        peek.advance();
        return core
            .item
            .into_type_annotation()
            .with_span(core.location)
            .wrap_ok();
    }
    let location = core.location;
    let written = WithGenericLocation::new(core.item.into_union_variant(), location.wrap_some());
    let null = WithGenericLocation::new(UnionVariant::Null(NullTypeAnnotation), None);
    TypeAnnotation::Union(UnionTypeAnnotation(vec![written, null]))
        .with_span(location)
        .wrap_ok()
}
```

After:

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<AstError>> {
    let core = parse_named_or_list(cursor)?;
    if cursor
        .consume_token_if(
            NonBracketTokenKind::Exclamation,
            IsographSemanticToken::Exclamation,
        )
        .is_some()
    {
        return core
            .item
            .into_type_annotation()
            .with_span(core.location)
            .wrap_ok();
    }
    let location = core.location;
    let written = WithGenericLocation::new(core.item.into_union_variant(), location.wrap_some());
    let null = WithGenericLocation::new(UnionVariant::Null(NullTypeAnnotation), None);
    TypeAnnotation::Union(UnionTypeAnnotation(vec![written, null]))
        .with_span(location)
        .wrap_ok()
}
```

#### `crates/isograph_parser/src/assert_semantic_tokens.rs`

Before:

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
pub(crate) fn assert_semantic_tokens(
    text: &str,
    actual: &[WithSpan<IsographSemanticToken>],
    expected: &[(IsographSemanticToken, &str)],
) {
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let span = loop {
            let offset = text[search_from..]
                .find(pattern)
                .expect("the expected lexeme occurs as a recorded token after the previous token");
            let start = search_from + offset;
            let end = start + pattern.len();
            let span = Span::from_usize(start, end);
            search_from = end;
            if actual.iter().any(|token| token.location == span) {
                break span;
            }
        };
        expected_tokens.push(role.with_span(span));
    }
    assert_eq!(
        actual,
        expected_tokens.as_slice(),
        "for literal {text:?}, actual {:?}, expected {:?}",
        displayed(text, actual),
        displayed(text, &expected_tokens),
    );
}

fn displayed(
    text: &str,
    tokens: &[WithSpan<IsographSemanticToken>],
) -> Vec<(IsographSemanticToken, String)> {
    tokens
        .iter()
        .map(|token| (token.item, text[token.location.as_usize_range()].to_owned()))
        .collect()
}
```

After:

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
pub(crate) fn assert_semantic_tokens(
    text: &str,
    actual: &[WithOptionalSpan<IsographSemanticToken>],
    expected: &[(IsographSemanticToken, &str)],
) {
    let actual: Vec<_> = actual
        .iter()
        .copied()
        .filter(|token| token.item != IsographSemanticToken::Comma)
        .collect();
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let span = loop {
            let offset = text[search_from..]
                .find(pattern)
                .expect("the expected lexeme occurs as a recorded token after the previous token");
            let start = search_from + offset;
            let end = start + pattern.len();
            let span = Span::from_usize(start, end);
            search_from = end;
            if actual
                .iter()
                .any(|token| token.location == span.wrap_some())
            {
                break span;
            }
        };
        expected_tokens.push(WithGenericLocation::new(role, span.wrap_some()));
    }
    assert_eq!(
        actual.as_slice(),
        expected_tokens.as_slice(),
        "for literal {text:?}, actual {:?}, expected {:?}",
        displayed(text, actual.as_slice()),
        displayed(text, &expected_tokens),
    );
}

fn displayed(
    text: &str,
    tokens: &[WithOptionalSpan<IsographSemanticToken>],
) -> Vec<(IsographSemanticToken, String)> {
    tokens
        .iter()
        .filter(|token| token.item != IsographSemanticToken::Comma)
        .map(|token| {
            let span = token.location.expect(
                "assert_semantic_tokens skips Comma; remaining tokens have spans",
            );
            (token.item, text[span.as_usize_range()].to_owned())
        })
        .collect()
}
```

The helper's own tests construct `WithGenericLocation::new(role, span.wrap_some())` instead of `role.with_span(span)`.

#### `crates/isograph_parser/src/lib.rs`

Change 2 adds `mod format` and `pub use format::format_recorded`.

#### `crates/isograph_extract_typescript/src/lib.rs`

Before:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
        assert_eq!(literals[0].1.tokens[0].location, Span::from_usize(0, 10));
```

After:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
        assert_eq!(
            literals[0].1.tokens[0].location,
            Span::from_usize(0, 10).wrap_some()
        );
```

Every other `token.location` compared to a `Span` gets `.wrap_some()` on the expected `Span`.

Highlighting `assert_semantic_tokens` expected lists: `(IsographSemanticToken::DirectiveName, "@")` becomes `(IsographSemanticToken::At, "@")`. Description strings become `Description`. A consumed `!` that expected `Content` becomes `Exclamation`. The helper skips `Comma`, so list-item expected lists do not mention `Comma`.

Facts:

- `field Query.x { id, name }` records `Keyword`, `Type`, `Period`, `FieldName`, `Brace`, `FieldName`, `Comma`, `FieldName`, `Comma`, `Brace`. `Comma` has `location: None`.
- `field Query.x { id }` records `Comma` after `id`.
- `field Query.x {}` has no `Comma`.
- `user(a: 1, b: 2)` records `Comma` after each argument.
- `[Foo!]` records `GraphQLTypeName`, `GraphQLTypeName`, `Exclamation`, `GraphQLTypeName`.
- Failed `require_token` records nothing.
- `field Pet.x "desc" { id }` records `Description`.
- `id @loadable` records `At` then `DirectiveName`.
- iso-format-lsp.md tests still get `[]`.

### Change 2: `format_recorded` and the handler body

#### `crates/isograph_parser/src/format.rs`

New file. The `format_recorded` body in Printer, exactly, plus the tests below.

#### `crates/isograph_parser/src/lib.rs`

Before:

```rust
// from crates/isograph_parser/src/lib.rs
mod arguments;
mod chunk;
mod chunk_stream;
mod directives;
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod parse_error;
mod parse_iso_literal;
mod selections;
mod semantic_token;
mod string_value;
mod token_kind;
mod tokenize;
mod variables;
```

After: `mod format;` after `mod directives;`. `pub use format::format_recorded;` next to `pub use semantic_token::IsographSemanticToken`.

#### `crates/isograph_lsp/src/semantic_tokens.rs`

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
pub fn lsp_semantic_tokens<'a>(
    page_content: &str,
    literals: impl IntoIterator<Item = (IsoLiteralStartIndex, &'a [WithSpan<IsographSemanticToken>])>,
) -> Vec<lsp_types::SemanticToken> {
```

After: the slice type is `&'a [WithOptionalSpan<IsographSemanticToken>]`. Same on `lsp_semantic_tokens_with` and `encode`.

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
        for token in tokens {
            let token = token.item.with_span(token.location.with_offset(offset));
            cursor.check_span(token.location, previous_token_end);
            previous_token_end = token.location.end;
            emit_pieces(token, &mut cursor, &mut last_start, &utf16_len, |t| {
                encoded.push(t)
            });
        }
```

After:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
        for token in tokens {
            if token.item == IsographSemanticToken::Comma {
                continue;
            }
            let Some(location) = token.location else {
                continue;
            };
            let token = token.item.with_span(location.with_offset(offset));
            cursor.check_span(token.location, previous_token_end);
            previous_token_end = token.location.end;
            emit_pieces(token, &mut cursor, &mut last_start, &utf16_len, |t| {
                encoded.push(t)
            });
        }
```

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
fn lsp_type_index(token: IsographSemanticToken) -> u32 {
    match token {
        IsographSemanticToken::Keyword => KEYWORD,
        IsographSemanticToken::Type => CLASS,
        IsographSemanticToken::FieldName => METHOD,
        IsographSemanticToken::ObjectKey | IsographSemanticToken::Argument => TYPE_PARAMETER,
        IsographSemanticToken::GraphQLTypeName => TYPE,
        IsographSemanticToken::DirectiveName => DECORATOR,
        IsographSemanticToken::Variable => FUNCTION,
        IsographSemanticToken::BooleanOrNull => VARIABLE,
        IsographSemanticToken::Integer => NUMBER,
        IsographSemanticToken::String => STRING,
        IsographSemanticToken::Period
        | IsographSemanticToken::Colon
        | IsographSemanticToken::Equals
        | IsographSemanticToken::Parenthesis
        | IsographSemanticToken::Brace
        | IsographSemanticToken::Content
        | IsographSemanticToken::Bracket => OPERATOR,
        IsographSemanticToken::Error => COMMENT,
    }
}
```

After:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
fn lsp_type_index(token: IsographSemanticToken) -> u32 {
    match token {
        IsographSemanticToken::Keyword => KEYWORD,
        IsographSemanticToken::Type => CLASS,
        IsographSemanticToken::FieldName => METHOD,
        IsographSemanticToken::ObjectKey | IsographSemanticToken::Argument => TYPE_PARAMETER,
        IsographSemanticToken::GraphQLTypeName => TYPE,
        IsographSemanticToken::DirectiveName | IsographSemanticToken::At => DECORATOR,
        IsographSemanticToken::Variable => FUNCTION,
        IsographSemanticToken::BooleanOrNull => VARIABLE,
        IsographSemanticToken::Integer => NUMBER,
        IsographSemanticToken::String | IsographSemanticToken::Description => STRING,
        IsographSemanticToken::Period
        | IsographSemanticToken::Colon
        | IsographSemanticToken::Equals
        | IsographSemanticToken::Parenthesis
        | IsographSemanticToken::Brace
        | IsographSemanticToken::Content
        | IsographSemanticToken::Bracket
        | IsographSemanticToken::Exclamation
        | IsographSemanticToken::Comma => OPERATOR,
        IsographSemanticToken::Error => COMMENT,
    }
}
```

`Comma` is skipped in the loop, so the panic is a programming error. Tests that pass `&[WithSpan<...>]` to `encode` construct `WithGenericLocation::new(role, span.wrap_some())` instead of `role.with_span(span)`.

Need exhaustive match - panic on Comma is ugly. Could use `unreachable` - banned almost. Better skip so match doesn't need Comma if we filter first... but exhaustiveness requires all variants. Use `Comma => OPERATOR` and never hit it, or:

```
IsographSemanticToken::Comma => COMMENT,
```

No, skip in loop. For exhaustiveness:

```
IsographSemanticToken::Comma => unreachable!("lsp_semantic_tokens skips Comma"),
```

User banned unreachable almost always. Filter before lsp_type_index so we could use `_` ... no that's worse.

Keep the skip in the loop AND `Comma => OPERATOR` as dead. Cleaner: `Comma => OPERATOR` with a comment that the loop never calls it. Exhaustive without panic.

I'll change to `| IsographSemanticToken::Comma` on OPERATOR... no that might get used if skip fails.

I'll use the skip and `Comma => OPERATOR` grouped with Content. If skip fails, a comma would highlight as operator - bad but no panic.

Actually the loop continues on Comma so lsp_type_index never sees it. I'll put Comma with the skip and not in lsp_type_index if we... rustc requires exhaustive.

I'll group `Comma` with a compile-time unused: still need the arm.

`IsographSemanticToken::Comma => OPERATOR` plus skip. Fine. Remove the panic I just wrote.

Let me fix that panic in the after snippet.

#### `crates/isograph_cli/src/adapter.rs`

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: isograph_compiler::HostLanguage>(
    _state: &isograph_compiler::IsographState<THostLanguage>,
    _params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    Vec::new().wrap_some().wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: isograph_compiler::HostLanguage>(
    _state: &isograph_compiler::IsographState<THostLanguage>,
    _params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    Vec::new().wrap_some().wrap_ok()
}
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>> {
    let absolute = file_path(uri)?;
    let cwd = *state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(cwd, &absolute);
    let literals = isograph_compiler::parsed_iso_literals_in_file(state, path).as_ref()?;
    let page_content = isograph_compiler::text_through_last_iso_literal(state, path).as_ref()?;
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            let start = extraction.iso_literal_start_index.0 as u32;
            let end = start + extraction.iso_literal_text.len() as u32;
            lsp_types::TextEdit {
                range: isograph_lsp::lsp_range_for_byte_range(page_content, start, end),
                new_text: isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ),
            }
            .wrap_some()
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

Missing file: `file_path` or cwd or the literals memo is `None` → `format_iso_literals` returns `None` → JSON `null`. No iso literals: `Some(vec![])`.

#### `crates/isograph_lsp/src/semantic_tokens.rs` (range helper)

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
pub fn lsp_range_for_byte_range(page_content: &str, start: u32, end: u32) -> lsp_types::Range {
    let index = LineIndex::new(page_content);
    let utf16_len = |text: &str| {
        if page_content.is_ascii() {
            text.len() as u32
        } else {
            text.encode_utf16().count() as u32
        }
    };
    let mut cursor = index.cursor();
    cursor.advance_to(start);
    let start_pos = lsp_types::Position {
        line: cursor.break_index as u32,
        character: cursor.column(start, &utf16_len),
    };
    cursor.advance_to(end);
    let end_pos = lsp_types::Position {
        line: cursor.break_index as u32,
        character: cursor.column(end, &utf16_len),
    };
    lsp_types::Range {
        start: start_pos,
        end: end_pos,
    }
}
```

`advance_to` and `column` stay `fn` on `LineCursor`. They are in the same module.

Before:

```rust
// from crates/isograph_lsp/src/lib.rs
pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};
```

After:

```rust
// from crates/isograph_lsp/src/lib.rs
pub use semantic_tokens::{lsp_range_for_byte_range, lsp_semantic_tokens, semantic_token_legend};
```

#### `crates/isograph_cli/Cargo.toml`

Before: no `isograph_parser` dependency.

After:

```toml
# from crates/isograph_cli/Cargo.toml
isograph_parser = { path = "../isograph_parser" }
```

next to `isograph_lsp`.

#### `crates/isograph_parser/src/format.rs` tests

```rust
// from crates/isograph_parser/src/format.rs
#[cfg(test)]
mod tests {
    use super::format_recorded;
    use crate::parse_iso_literal;

    fn formatted(text: &str) -> String {
        let parsed = parse_iso_literal(text);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        format_recorded(text, parsed.tokens.as_slice())
    }

    #[test]
    fn one_selection_breaks_and_trailing_comma() {
        assert_eq!(formatted("field Query.x{id}"), "field Query.x {\n  id,\n}");
    }

    #[test]
    fn two_selections_one_per_line_with_commas() {
        assert_eq!(
            formatted("field Query.x { id, name }"),
            "field Query.x {\n  id,\n  name,\n}"
        );
        assert_eq!(
            formatted("field Query.x { id name }"),
            "field Query.x {\n  id,\n  name,\n}"
        );
        assert_eq!(
            formatted("field Query.x { id, name, }"),
            "field Query.x {\n  id,\n  name,\n}"
        );
    }

    #[test]
    fn empty_braces_stay_empty() {
        assert_eq!(formatted("field Query.x {}"), "field Query.x {}");
    }

    #[test]
    fn empty_parens_stay_empty() {
        assert_eq!(
            formatted("field Query.x { user() }"),
            "field Query.x {\n  user(),\n}"
        );
    }

    #[test]
    fn value_list_breaks() {
        assert_eq!(
            formatted("field Query.x { foo(x: [1, 2]) }"),
            "field Query.x {\n  foo(\n    x: [\n      1,\n      2,\n    ],\n  ),\n}"
        );
    }

    #[test]
    fn type_list_stays_glued() {
        assert_eq!(
            formatted("field Query.x($y: [Foo!]) { id }"),
            "field Query.x(\n  $y: [Foo!],\n) {\n  id,\n}"
        );
    }

    #[test]
    fn entrypoint_is_unchanged() {
        assert_eq!(
            formatted("entrypoint Query.HomeRoute"),
            "entrypoint Query.HomeRoute"
        );
    }

    #[test]
    fn to_has_spaces() {
        assert_eq!(
            formatted("field Pet.x to Image { id }"),
            "field Pet.x to Image {\n  id,\n}"
        );
    }

    #[test]
    fn nested_object_and_args_break() {
        assert_eq!(
            formatted("field Query.x { user(arg: { b: true }, c: false) }"),
            "field Query.x {\n  user(\n    arg: {\n      b: true,\n    },\n    c: false,\n  ),\n}"
        );
    }

    #[test]
    fn variable_list_breaks() {
        assert_eq!(
            formatted("field Query.user($id:ID,$x:Int){id}"),
            "field Query.user(\n  $id: ID,\n  $x: Int,\n) {\n  id,\n}"
        );
    }

    #[test]
    fn description_is_on_its_own_line() {
        assert_eq!(
            formatted("field Pet.x\n\"desc\"\n{id}"),
            "field Pet.x\n\"desc\"\n{\n  id,\n}"
        );
    }

    #[test]
    fn at_has_a_space_before() {
        let out = formatted("field Query.x { id @loadable }");
        assert!(out.contains("id @loadable"), "{out}");
    }

    #[test]
    fn dot_stays_glued() {
        assert!(formatted("field Query.x { id }").contains("Query.x"));
    }

    #[test]
    fn failed_parse_is_not_formatted() {
        let parsed = parse_iso_literal("not a declaration");
        assert!(!parsed.errors.is_empty());
    }
}
```

Leftover remaining contents report an error, so this covers leftover too. The handler skips `format_recorded` when `errors` is non-empty.

#### `crates/isograph_cli/src/lsp_socket.rs`

```rust
// from crates/isograph_cli/src/lsp_socket.rs
    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_of_an_interned_iso_literal_edits_the_interior() {
        let (port, _event_rx) = listen_and_reply().await;
        intern_iso_field(port).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        let edits: Vec<lsp_types::TextEdit> =
            serde_json::from_value(response.result.expect("edits")).expect("Vec<TextEdit>");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "field Query.x {\n  id,\n}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_skips_a_failed_literal_and_edits_the_good_one() {
        let (port, _event_rx) = listen_and_reply().await;
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/x.ts"),
                presence: Presence::Present(
                    "iso(`field Query.x{id}`) iso(`not a declaration`)".to_owned(),
                ),
            })),
        )
        .expect("notify DiskChanged");
        tokio::time::sleep(SETTLE).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        let edits: Vec<lsp_types::TextEdit> =
            serde_json::from_value(response.result.expect("edits")).expect("Vec<TextEdit>");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "field Query.x {\n  id,\n}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_of_a_never_interned_uri_is_null() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/never.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        assert!(
            response.result.is_none()
                || response.result.as_ref() == serde_json::Value::Null.reference().wrap_some(),
            "{response:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn will_save_wait_until_matches_formatting() {
        let (port, _event_rx) = listen_and_reply().await;
        intern_iso_field(port).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let formatted = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        let will_save = will_save_wait_until(
            &mut writer,
            &mut reader,
            3,
            will_save_wait_until_params("file:///tmp/proj/src/x.ts"),
        );
        assert_eq!(formatted.result, will_save.result);
    }
```

`intern_iso_field` already presents `iso(\`field Query.x{id}\`)`. The existing tests that expect `[]` from formatting are deleted.

## Call sites

- grammar consume → `commit(token)` → `WithGenericLocation::new(token, span.wrap_some())`
- leftover fill-in → same, `Some(span)`
- `parse_each_chunk` after each successful item → `WithGenericLocation::new(IsographSemanticToken::Comma, None)`
- description → `commit(Description)`
- `@` → `commit(At)`; directive identifier → `commit(DirectiveName)`
- `!` after a type → `commit(Exclamation)`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `.item`, skip `Comma`

## Docs this change amends

iso-format-lsp.md: the two handlers return the edits from `format_iso_literals`.

semantic-tokens.md: `Description`, `At`, `Exclamation`, `Comma`. `DirectiveName` is the identifier only. `String` is a value. `!` is recorded. Tokens vec is `Vec<WithOptionalSpan<IsographSemanticToken>>`. `leftover_token` is unchanged. This file.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

iso-format-later.md: packing a group onto one line when it fits the width.

comments.md: tokens vec is `WithOptionalSpan`. Descriptions are `Description`. Chunk commas are `Comma` with `None`.
