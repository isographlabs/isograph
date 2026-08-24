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

`ParsedIsoLiteral.tokens` is `Vec<WithOptionalSpan<IsographSemanticToken>>`. Consume has `Some(span)` because the highlighter needs file positions and the printer needs the source lexeme (`fullName`, `Query`, `{`). `Comma` has `location: None`: it is not in the source. `parse_each_chunk` after a successful item always records that `Comma` (after the last item too). Empty level: no comma. Failed item: no comma. Source commas are ignored: `leftover_token` on `Comma` is `None`, same as `LineBreak`.

Depth is a counter over list-group open and close. `ResolvePosition` does not enter `None`. The highlighter skips `Comma`. The handler does not call `format_recorded` when there are errors.

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

Roles that would format two ways are split on `IsographSemanticToken`:

```rust
// from crates/isograph_parser/src/semantic_token.rs
    Description,
    At,
    Exclamation,
    Comma,
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

`NoSemanticTokens::record` ignores commas.

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
        let Some(lexeme) = lexeme(text, token) else {
            continue;
        };
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
    token
        .location
        .map(|span| &text[span.as_usize_range()])
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

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>>
```

If request dispatch already passes `LspClientId`, this function takes it and reads `lsp_file_contents`. If not, it reads `DiskFile` the way `semantic_tokens_response` does.

For each extraction in the file: if parse succeeded, one `TextEdit` whose range is the iso interior (extraction start and length, converted the way tokens already convert) and whose `new_text` is `format_recorded`. Failed parse: no edit for that extraction. No iso literals: empty vec. Missing file: `null`.

```rust
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

Add `Description`, `At`, `Exclamation`, `Comma` to `IsographSemanticToken`.

Before:

```rust
// from crates/isograph_parser/src/semantic_token.rs
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => IsographSemanticToken::Content.wrap_some(),
```

After: same, and `NonBracketTokenKind::Comma` is `None` (match it with `LineBreak | EndOfFile | Comma`). Today `Comma` falls through to `Content`.

#### `crates/isograph_parser/src/chunk_stream.rs`

Every `Vec<WithSpan<IsographSemanticToken>>` (ItemCursor, CursorPeek, constructors, `tokens()`, `take_item`) becomes `Vec<WithOptionalSpan<IsographSemanticToken>>`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn record(&mut self, token: IsographSemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        tokens.push(token.with_span(span));
```

After both:

```rust
        self.tokens
            .push(WithGenericLocation::new(token, span.wrap_some()));
```

(`commit` uses the second.)

#### `crates/isograph_parser/src/chunk.rs`

Same vec type on `stream`, `record_leftover_item`, `record_leftover_chunk`, `record_leftover_extra`, `record_leftover_span`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
        && tokens.iter().all(|recorded| recorded.location != span)
    {
        tokens.push(role.with_span(span));
```

After:

```rust
        && tokens
            .iter()
            .all(|recorded| recorded.location != span.wrap_some())
    {
        tokens.push(WithGenericLocation::new(role, span.wrap_some()));
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

`ParsedIsoLiteral.tokens` and `parse_chunked_iso_literal`'s `tokens` param: `Vec<WithOptionalSpan<IsographSemanticToken>>`.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        IsographSemanticToken::String,
```

in `consume_description` (both string and block string). After: `IsographSemanticToken::Description`.

#### `crates/isograph_parser/src/directives.rs`

Before:

```rust
// from crates/isograph_parser/src/directives.rs
            while let Some(at) = cursor.consume_token_if(
                NonBracketTokenKind::At,
                IsographSemanticToken::DirectiveName,
            ) {
```

After: `IsographSemanticToken::At`. The identifier `require_token` stays `DirectiveName`.

#### `crates/isograph_parser/src/variables.rs`

Before:

```rust
// from crates/isograph_parser/src/variables.rs
        peek.advance();
```

After: replace `peek.advance()` with

```rust
        cursor.consume_token_if(
            NonBracketTokenKind::Exclamation,
            IsographSemanticToken::Exclamation,
        );
```

and keep the `if` on that `Option` (drop the separate `peek` match).

#### `crates/isograph_parser/src/assert_semantic_tokens.rs`

`actual` is `&[WithOptionalSpan<IsographSemanticToken>]`. Skip `Comma`. Compare `location` to `span.wrap_some()`. `displayed` skips `Comma` and does not read `None`.

#### `crates/isograph_parser/src/lib.rs`

`IsographSemanticToken` variants and `ParsedIsoLiteral.tokens` type. `mod format` and `pub use format::format_recorded` land in Change 2.

#### Tests that read `.location` as `Span`

`crates/isograph_extract_typescript/src/lib.rs` `one_entrypoint_is_one_pair`: `tokens[0].location` is `Span::from_usize(0, 10).wrap_some()`. Same for any other `.location` compared to a `Span`.

Directive and description highlighting tests that expect `(DirectiveName, "@")` become `(At, "@")`. Description strings become `Description`. `!` tests that expected leftover `Content` for a consumed `!` become `Exclamation`.

`parse_each_chunk_parses_one_identifier_per_chunk` and other `assert_semantic_tokens` callers: helper skips `Comma`, so expected lists stay as they are except `@` / description / `!` as above.

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

New file. The `format_recorded` body in Printer, exactly.

#### `crates/isograph_parser/src/lib.rs`

```rust
mod format;
pub use format::format_recorded;
```

#### `crates/isograph_lsp/src/semantic_tokens.rs`

`lsp_semantic_tokens` / `lsp_semantic_tokens_with` / `encode` take `&[WithOptionalSpan<IsographSemanticToken>]`.

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
        for token in tokens {
            let token = token.item.with_span(token.location.with_offset(offset));
```

After:

```rust
        for token in tokens {
            if token.item == IsographSemanticToken::Comma {
                continue;
            }
            let Some(location) = token.location else {
                continue;
            };
            let token = token.item.with_span(location.with_offset(offset));
```

`lsp_type_index`: `Description => STRING`, `At => DECORATOR`, `Exclamation => OPERATOR`. `Comma` is not reached.

Tests that pass `&[WithSpan<...>]` to `encode` construct `WithGenericLocation::new(role, span.wrap_some())` instead of `role.with_span(span)`.

#### `crates/isograph_cli/src/adapter.rs`

Before: `formatting_response` / `will_save_wait_until_response` return `Vec::new().wrap_some()`.

After: the handler bodies in Handlers, plus:

```rust
// from crates/isograph_cli/src/adapter.rs
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

`advance_to` and `column` stay `fn`. Export `lsp_range_for_byte_range` from `crates/isograph_lsp/src/lib.rs`.

#### `crates/isograph_cli/Cargo.toml`

Add `isograph_parser = { path = "../isograph_parser" }`.

Printer tests (string equality) in `format.rs`:

- `field Query.x{id}` → `field Query.x {\n  id,\n}`
- `field Query.x { id, name }` and `field Query.x { id name }` and `field Query.x { id, name, }` → `field Query.x {\n  id,\n  name,\n}`
- `field Query.x {}` → `field Query.x {}`
- `field Query.x { user() }` → `field Query.x {\n  user(),\n}`
- `field Query.x { foo(x: [1, 2]) }` → `field Query.x {\n  foo(\n    x: [\n      1,\n      2,\n    ],\n  ),\n}`
- `field Query.x($y: [Foo!]) { id }` → `field Query.x(\n  $y: [Foo!],\n) {\n  id,\n}`
- `entrypoint Query.HomeRoute` → itself
- `field Pet.x to Image { id }` → `field Pet.x to Image {\n  id,\n}`
- `field Query.x { user(arg: { b: true }, c: false) }` → `field Query.x {\n  user(\n    arg: {\n      b: true,\n    },\n    c: false,\n  ),\n}`
- `field Query.user($id:ID,$x:Int){id}` → `field Query.user(\n  $id: ID,\n  $x: Int,\n) {\n  id,\n}`
- `field Pet.x\n\"desc\"\n{id}` → `field Pet.x\n\"desc\"\n{\n  id,\n}`
- `field Query.x { id @loadable }`: space before `@`
- `Query.x` keeps the dot glued
- leftover-only tokens do not appear as extra words
- a lone close still emits (indent stays at 0)

`lsp_socket.rs` listen_and_reply:

- `textDocument/formatting` after intern/open of a file with `iso(\`field Query.x{id}\`)`: one edit, `new_text` is the formatted interior, range covers that interior only
- a file with one good literal and one `iso(\`not a declaration\`)`: one edit, the broken interior unchanged
- missing URI: `result` is JSON `null`
- `willSaveWaitUntil` for the interned file: same edits as formatting

## Call sites

- grammar consume → `commit(token)` → `WithGenericLocation::new(token, span.wrap_some())`
- leftover fill-in → same, `Some(span)`
- `parse_each_chunk` after each successful item → `WithGenericLocation::new(IsographSemanticToken::Comma, None)`
- description → `commit(Description)`
- `@` → `commit(At)`; directive identifier → `commit(DirectiveName)`
- `!` after a type → `commit(Exclamation)`
- `leftover_token` on `Comma` → `None`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `.item`, skip `Comma`

## Docs this change amends

iso-format-lsp.md: the two handlers return the edits from `format_iso_literals`.

semantic-tokens.md: `Description`, `At`, `Exclamation`, `Comma`. `DirectiveName` is the identifier only. `String` is a value. `!` is recorded. `leftover_token` on `Comma` is `None`. Tokens vec is `Vec<WithOptionalSpan<IsographSemanticToken>>`. This file.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

iso-format-later.md: packing a group onto one line when it fits the width.

comments.md: tokens vec is `WithOptionalSpan`. Descriptions are `Description`. Chunk commas are `Comma` with `None`.
