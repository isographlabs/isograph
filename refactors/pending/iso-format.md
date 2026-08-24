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

The tokens vec is enough to print, plus chunk boundaries. Depth is not recorded; the printer counts list-group open and close. Source commas are chunk separators and are not consume tokens. `parse_each_chunk` after a successful item pushes a spanless `Comma` (after the last item too). Empty level: no comma. Failed item: no comma.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum Recorded {
    Token(WithSpan<IsographSemanticToken>),
    Comma,
}
```

`commit` and leftover fill-in push `Token`. The highlighter reads `.item` on `Token` and skips `Comma`. List-item commas live on `Chunk::trailing_separator` and are not leftover-filled (`parse_each_chunk`'s leftover is `Expectation::Separator`). The printer sees those as `Recorded::Comma` only. `Comma` has no span. `ResolvePosition` does not enter it. The handler does not call `format_recorded` when there are errors.

`require_token` / `require_group` signatures do not change.

Roles that would format two ways are split on `IsographSemanticToken`:

```rust
// from crates/isograph_parser/src/semantic_token.rs
    Description,
    At,
    Exclamation,
```

- `Description`: the declaration description. Own line. `String` stays a string value (`name: "x"`).
- `At`: `@`. Space before, glued to `DirectiveName`. `DirectiveName` is the identifier only. `FieldName` is already the selectable name.
- `Exclamation`: `!` after a type. Glued. Today `peek.advance()` records nothing, so `[Foo!]` would drop `!`.

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
pub fn format_recorded(text: &str, tokens: &[Recorded]) -> String
```

Each role prints its lexeme. List-group `{` `(` `[` (`Brace` / `Parenthesis` / `Bracket`) indent in and break; the matching close indents out and breaks. Type-list `[` `]` are `GraphQLTypeName` and do not indent. `Comma` prints `,` and a line break. Open immediately followed by its close is `{}` `()` `[]`. `Description` is its own line. Space after `Keyword` / `Colon` / `Equals`. Space before `Keyword` / `Equals` / `Brace` / `At`. Everything else glues: `Query.x`, `user(`, `$id`, `@loadable`, `[Foo!]`, `id:`.

```rust
// from crates/isograph_parser/src/format.rs
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
```

Width is 2 spaces. A parse failure of a literal is not passed to `format_recorded`.

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

### Change 1: `Recorded`, commas after list items, `Description` / `At` / `Exclamation`

`ParsedIsoLiteral.tokens` is `Vec<Recorded>`. `commit` and leftover fill-in push `Token`. `parse_each_chunk` pushes `Comma` after every successful item, including the last. Description consume is `Description`. `@` is `At`, the identifier is `DirectiveName`. `!` is `Exclamation`.

Tests: a `field Query.x { id, name }` parse records `Keyword`, `Type`, `Period`, `FieldName`, `Brace`, `FieldName`, `Comma`, `FieldName`, `Comma`, `Brace`. `field Query.x { id }` records `Comma` after `id`. `field Query.x {}` has no `Comma`. `user(a: 1, b: 2)` records `Comma` after each argument; both parens are `Parenthesis`. `[Foo!]` records `GraphQLTypeName`, `GraphQLTypeName`, `Exclamation`, `GraphQLTypeName`. Failed `require_token` records nothing. `field Pet.x "desc" { id }` records `Description`. `id @loadable` records `At` then `DirectiveName`. Compile `NoSemanticTokens` still compiles.

Highlighting tests read `.item` on `Token` and skip `Comma`. iso-format-lsp.md tests still get `[]`.

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

- grammar consume → `commit(token)` → `Recorded::Token`
- leftover fill-in → `Recorded::Token`
- `parse_each_chunk` after each successful item → `Recorded::Comma`
- description → `commit(Description)`
- `@` → `commit(At)`; directive identifier → `commit(DirectiveName)`
- `!` after a type → `commit(Exclamation)`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `Token.item`

## Docs this change amends

iso-format-lsp.md: the two handlers return the edits from `format_iso_literals`.

semantic-tokens.md: `Description`, `At`, `Exclamation`. `DirectiveName` is the identifier only. `String` is a value. `!` is recorded. Formatter metadata: list groups are `Brace` / `Parenthesis` / `Bracket`; type-list `[` is `GraphQLTypeName`; chunk commas are `Recorded::Comma`. This file.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

iso-format-later.md: packing a group onto one line when it fits the width.

comments.md: leftover fill-in is still `Recorded::Token`. Descriptions are `Description`.
