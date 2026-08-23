# Format iso literals

Requires iso-format-lsp.md. Independent of lsp-diagnostics.md. Uses open-file contents when lsp-open-files.md change 3 has landed; until then, disk, same as tokens.

iso-format-lsp.md's `textDocument/formatting` and `textDocument/willSaveWaitUntil` handlers return `[]`. This slice records layout at consume, prints from that stream, and those handlers return the edits.

A line break plus indent is `{`. `}` starts a new line and dedents. A description occupies its own line (`before` and `after` are `Line`), so `{` after a description is on the next line. Every list (selection set, argument list, variable list, object literal, value list) has a comma between adjacent items, with a space after the comma.

```
user(arg: {
  b: true
}, c: false)
```

```
field Pet.fullName {
  firstName, lastName
}
```

```
field Pet.fullName
"a pet's name"
{
  firstName, lastName
}
```

```
field Query.user($id: ID, $x: Int) {
  user(id: $id, extra: {
    b: true
  }) {
    id, name
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
  id, name
}
```

The `iso(\`` and closing backtick stay. A parsed literal in the same file as a failed parse is still formatted.

`willSaveWaitUntil` (advertised in iso-format-lsp.md) returns those edits. `textDocument/formatting` returns the same edits.

## Layout

Most important first.

```rust
// from crates/isograph_parser/src/layout.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Gap {
    None,
    Space,
    Line,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Indent {
    None,
    In,
    Out,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Layout {
    pub before: Gap,
    pub after: Gap,
    pub indent: Indent,
}
```

`Gap` orders `None < Space < Line`. The gap between two adjacent recorded tokens is `max(prev.after, next.before)`.

`Indent::In` applies after the token (open `{` of a selection set or object literal). `Indent::Out` applies before the token (the matching `}`). `(` / `)` / type-list `[` / `]` use `Indent::None`.

```rust
impl Layout {
    pub const GLUE: Layout = Layout {
        before: Gap::None,
        after: Gap::None,
        indent: Indent::None,
    };
    pub const SPACE: Layout = Layout {
        before: Gap::Space,
        after: Gap::Space,
        indent: Indent::None,
    };
    pub const SPACE_AFTER: Layout = Layout {
        before: Gap::None,
        after: Gap::Space,
        indent: Indent::None,
    };
    pub const OWN_LINE: Layout = Layout {
        before: Gap::Line,
        after: Gap::Line,
        indent: Indent::None,
    };
    pub const OPEN_BRACE: Layout = Layout {
        before: Gap::Space,
        after: Gap::Line,
        indent: Indent::In,
    };
    pub const CLOSE_BRACE: Layout = Layout {
        before: Gap::Line,
        after: Gap::None,
        indent: Indent::Out,
    };
    pub const AT: Layout = Layout {
        before: Gap::Space,
        after: Gap::None,
        indent: Indent::None,
    };
    pub const COMMA: Layout = Layout {
        before: Gap::None,
        after: Gap::Space,
        indent: Indent::None,
    };
}
```

`OPEN_BRACE` / `CLOSE_BRACE` are `{` / `}` of a selection set or object literal. Type-list `[String]` and `(` `)` are `GLUE`.

An `In` token immediately followed by the matching `Out` with nothing recorded between them uses `Gap::None` between those two: `{}`. Consume sites pass `OPEN_BRACE` / `CLOSE_BRACE` either way.

## Recording

`commit` takes the highlight role and the layout.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum Recorded {
    Token(RecordedToken),
    Comma(Layout),
}

pub struct RecordedToken {
    pub span: Span,
    pub token: IsographSemanticToken,
    pub layout: Option<Layout>,
}
```

`RecordedToken.layout` `Some` is consume. `None` is leftover fill-in. The printer walks consume tokens and `Comma`. The highlighter walks `Recorded::Token`. `Comma` has no span. `ResolvePosition` does not enter it.

`parse_each_chunk` after a successfully parsed item that is not the last item of the level records `Recorded::Comma(Layout::COMMA)`. A failed item has no comma. A trailing source comma is leftover fill-in.

```rust
cursor.require_token(
    NonBracketTokenKind::Period,
    IsographSemanticToken::Period,
    Layout::GLUE,
)
cursor.require_group(
    BracketKind::Brace,
    IsographSemanticToken::Brace,
    Layout::OPEN_BRACE,
    Layout::CLOSE_BRACE,
    parse_inside,
)
```

`require_group` / `consume_group_if` take two layouts. Open and close share one highlight role. `RecordGroupClose` stores the close layout. Parenthesis groups and type-list brackets pass `Layout::GLUE`, `Layout::GLUE`.

`!` is consumed with `IsographSemanticToken::Content` and `Layout::GLUE`.

`NoSemanticTokens::record` ignores layout and commas.

## Consume sites

- Declaration keyword (`entrypoint`, `field`): `SPACE_AFTER`
- Type in `Type.name`: `GLUE`
- `.`: `GLUE`
- Selectable name in `Type.name`: `GLUE`
- `to`: `SPACE`
- Type-annotation identifier: `GLUE`
- Type-list `[` / `]`: `GLUE`
- `!`: `GLUE`, `Content`
- Description string / block string: `OWN_LINE`
- Selection name or alias (first identifier of a selection): `GLUE`
- `:` of `alias: name`, of `name: Type`, of `name: value`: `SPACE_AFTER`
- Name after `alias:`: `GLUE`
- Selection-set `{` / `}`: `OPEN_BRACE` / `CLOSE_BRACE`
- Object literal `{` / `}`: `OPEN_BRACE` / `CLOSE_BRACE`
- Argument list `(` / `)`: `GLUE` / `GLUE`
- Variable declaration list `(` / `)`: `GLUE` / `GLUE`
- Value list `[` / `]`: `GLUE` / `GLUE`
- Argument name / object key: `GLUE`
- `$` and the variable identifier, declaration or usage: `GLUE`
- `=`: `SPACE`
- Integer / string / boolean / null as values: `GLUE`
- `@`: `AT`
- Directive identifier: `GLUE`
- Comma between list items: `Recorded::Comma(Layout::COMMA)`, from `parse_each_chunk`

`max(description.after=Line, brace.before=Space)` is `Line`.

Tests in `chunk_stream.rs` that call `require_token` / `consume_group_if` pass `Layout::GLUE` unless they assert layout.

## Printer

```rust
// from crates/isograph_parser/src/format.rs
pub fn format_recorded(text: &str, tokens: &[Recorded]) -> String
```

Walk `Recorded::Token` whose `layout` is `Some`, and `Recorded::Comma`. A comma's text is `","`.

```
indent = 0
last_after = Gap::None
output empty

for each printable:
    if layout.indent is Out: indent = indent - 1, not below 0
    if this token is Out and the previous emitted token was In
        with no printable between them:
        gap = Gap::None
    else:
        gap = max(last_after, layout.before)
    if gap is Space: push ' '
    if gap is Line: push '\n' then "  " repeated indent times
    push text (source slice, or "," for Comma)
    last_after = layout.after
    if layout.indent is In: indent = indent + 1

if last_after is Line: push '\n'
```

Matching In: previous emitted token had `Indent::In` and this token has `Indent::Out`.

Width is 2 spaces.

A parse failure of a literal is not passed to `format_recorded`.

## Target shape

```
entrypoint Query.HomeRoute

field Pet.fullName {
  firstName, lastName
}

field Query.user($id: ID, $x: Int) {
  user(id: $id, extra: {
    b: true
  }, c: false) {
    id
  }
}

field Pet.Avatar to Image {
  url
}

field Pet.fullName
"a pet's name"
{
  firstName, lastName
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

### Change 1: `Layout` at consume, commas between list items

`layout.rs`, `Recorded`, `commit` / `require_token` / `require_group` take layout, leftover `layout: None`. `parse_each_chunk` records `Recorded::Comma(Layout::COMMA)` between successful items. Catalog above at every consume.

Tests: a `field Query.x { id, name }` parse records `Keyword SPACE_AFTER`, `Type GLUE`, `Period GLUE`, `FieldName GLUE`, `Brace OPEN_BRACE`, `FieldName GLUE`, `Comma COMMA`, `FieldName GLUE`, `Brace CLOSE_BRACE`. `field Query.x { id }` has no `Comma`. `user(a: 1, b: 2)` records `Comma` between the two arguments; both parens are `GLUE`. Failed `require_token` records nothing. `!` is recorded. A description records `OWN_LINE`. Compile `NoSemanticTokens` still compiles.

Highlighting tests read `.token` on `Recorded::Token` and skip `Recorded::Comma`. iso-format-lsp.md tests still get `[]`.

### Change 2: `format_recorded` and the handler body

Printer tests (string equality):

- `field Query.x{id}` → `field Query.x {\n  id\n}`
- `field Query.x { id, name }` and `field Query.x { id name }` and `field Query.x { id, name, }` → `field Query.x {\n  id, name\n}`
- `field Query.x {}` → `field Query.x {}`
- `entrypoint Query.HomeRoute` → itself
- `field Pet.x to Image { id }` → `field Pet.x to Image {\n  id\n}`
- `user(arg: { b: true }, c: false)` → `user(arg: {\n  b: true\n}, c: false)`
- `field Query.user($id:ID,$x:Int){id}` → `field Query.user($id: ID, $x: Int) {\n  id\n}`
- `field Pet.x\n\"desc\"\n{id}` → `field Pet.x\n\"desc\"\n{\n  id\n}`
- `id @loadable` inside a set: space before `@`
- `Query.x` keeps the dot glued
- leftover-only tokens do not appear as extra words
- a lone close still emits (indent stays at 0)

`lsp_socket.rs` listen_and_reply:

- `textDocument/formatting` after intern/open of a file with `iso(\`field Query.x{id}\`)`: one edit, `new_text` is the formatted interior, range covers that interior only
- a file with one good literal and one `iso(\`not a declaration\`)`: one edit, the broken interior unchanged
- missing URI: `result` is JSON `null`
- `willSaveWaitUntil` for the interned file: same edits as formatting

## Call sites

- grammar consume → `commit(token, layout)`
- `parse_each_chunk` between items → `Recorded::Comma`
- leftover fill-in → `Recorded::Token { layout: None, ... }`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `Recorded::Token.token`

## Docs this change amends

iso-format-lsp.md: the two handlers return the edits from `format_iso_literals`.

semantic-tokens.md "Formatter metadata": consume passes a `Layout` next to the highlight role. This file.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

iso-format-later.md: next shapes for `(` lists, width, and trailing commas.
