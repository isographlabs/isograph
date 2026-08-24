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

The tokens vec on `ParsedIsoLiteral` is `Vec<Recorded>`. `commit` still takes only the highlight role. `require_token` / `require_group` signatures do not change.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum Recorded {
    Consume(WithSpan<IsographSemanticToken>),
    Leftover(WithSpan<IsographSemanticToken>),
    Comma,
}
```

`commit` pushes `Consume`. Leftover fill-in pushes `Leftover`. `parse_each_chunk` after a successfully parsed item pushes `Comma`, including after the last item of the level. A failed item has no comma. An empty level has no comma. A trailing source comma is leftover fill-in.

The highlighter walks `Consume` and `Leftover`, reads `.item`, skips `Comma`. The printer walks `Consume` and `Comma`. `Comma` has no span. `ResolvePosition` does not enter it.

`!` after a type is `peek.advance()` today and is not recorded. Consume it with `IsographSemanticToken::Content` so it prints, glued, in `[Foo!]`.

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
```

`NoSemanticTokens::record` ignores commas.

## Printer

```rust
// from crates/isograph_parser/src/format.rs
pub fn format_recorded(text: &str, tokens: &[Recorded]) -> String
```

A list-group open is a `Consume` whose token is `Brace` / `Parenthesis` / `Bracket` and whose source char is `{` / `(` / `[`. A list-group close is the same roles with `}` / `)` / `]`. Type-list `[` `]` are `GraphQLTypeName` and are not list-group brackets.

A description is a `Consume` `String` whose previous `Consume` is not `Colon` and not `Equals`.

```
indent = 0
pending_line = false
open_awaiting_close = false
last: Option<WithSpan<IsographSemanticToken>>
output empty

for each Consume token or Comma:
    if Comma:
        push ','
        pending_line = true
        open_awaiting_close = false
        continue
    if list-group close: indent = indent - 1, not below 0
    empty = list-group close and open_awaiting_close
    if empty:
        gap is none
    else if pending_line or list-group close or this token is a description or last is a description:
        push '\n' then "  " repeated indent times
    else if last is Some(prev) and space_between(prev, this, text):
        push ' '
    push source slice
    last = this token
    if list-group open:
        indent = indent + 1
        pending_line = true
        open_awaiting_close = true
    else:
        pending_line = false
        open_awaiting_close = false
```

`space_between(prev, next, text)` is true when `space_after(prev)` or `space_before(next, text)`:

- `space_after`: `Keyword`, `Colon`, `Equals`
- `space_before`: `Keyword`, `Equals`, `Brace`, and `DirectiveName` whose source text is `@`

Everything else glues: `Query.x`, `user(`, `$id`, `@loadable`, `[Foo!]`, `id:`.

Width is 2 spaces.

A parse failure of a literal is not passed to `format_recorded`.

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

### Change 1: `Recorded`, commas after list items, record `!`

`ParsedIsoLiteral.tokens` is `Vec<Recorded>`. `commit` pushes `Consume`. Leftover fill-in pushes `Leftover`. `parse_each_chunk` pushes `Comma` after every successful item, including the last. Consume `!` with `Content`.

Tests: a `field Query.x { id, name }` parse records `Keyword`, `Type`, `Period`, `FieldName`, `Brace`, `FieldName`, `Comma`, `FieldName`, `Comma`, `Brace`. `field Query.x { id }` records `Comma` after `id`. `field Query.x {}` has no `Comma`. `user(a: 1, b: 2)` records `Comma` after each argument; both parens are `Parenthesis`. `[Foo!]` records `GraphQLTypeName`, `GraphQLTypeName`, `Content`, `GraphQLTypeName`. Failed `require_token` records nothing. A description is `String`. Compile `NoSemanticTokens` still compiles.

Highlighting tests read `.item` on `Consume` and `Leftover` and skip `Comma`. iso-format-lsp.md tests still get `[]`.

### Change 2: `format_recorded` and the handler body

Printer tests (string equality):

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

- grammar consume → `commit(token)` → `Recorded::Consume`
- leftover fill-in → `Recorded::Leftover`
- `parse_each_chunk` after each successful item → `Recorded::Comma`
- `!` after a type → `commit(Content)`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `.item` on `Consume` and `Leftover`

## Docs this change amends

iso-format-lsp.md: the two handlers return the edits from `format_iso_literals`.

semantic-tokens.md "Formatter metadata": no `Layout` on consume. List groups are `Brace` / `Parenthesis` / `Bracket`. Type-list `[` is `GraphQLTypeName`. Commas are `Recorded::Comma`. This file.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

iso-format-later.md: packing a group onto one line when it fits the width.

comments.md: leftover is `Recorded::Leftover`, not `layout: None`. Descriptions are own-line in the printer, not a consume `Layout`.
