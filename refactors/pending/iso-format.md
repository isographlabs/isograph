# Format iso literals

Requires iso-format-lsp.md. Independent of lsp-diagnostics.md. Uses open-file contents when lsp-open-files.md change 3 has landed; until then, disk, same as tokens.

iso-format-lsp.md advertises `textDocument/formatting` and `textDocument/willSaveWaitUntil` and answers both with `[]`. This slice records layout at consume, prints from that stream, and puts those edits in the existing handlers.

Save then reformats every successfully parsed iso interior in the file. JS/TS around the literals is not touched. Prettier (or vtsls) stays the TypeScript document formatter. There is no `isograph.autoformatIsoLiterals` setting.

Spacing is recorded at consume, next to the semantic token, not stuffed into `SemanticToken`. The highlighter ignores layout. The printer ignores highlight role. Leftover fill-in is highlight-only and is not printed.

## What the user does

A TypeScript file contains messy interiors:

```
export const Home = iso(`field Query.HomeRoute{id  name}`)
```

Save. The interior becomes:

```
field Query.HomeRoute {
  id, name
}
```

The `iso(\`` and closing backtick do not move. A second literal in the same file that failed to parse is left as-is. A literal that parsed is formatted even if another one failed.

VS Code: this happens on save because iso-format-lsp.md advertised `willSaveWaitUntil` and this slice returns edits. The user does not turn on a setting. `editor.formatOnSave` can stay aimed at Prettier; that run does not have to see our edits.

Zed: we already advertise the capability. Zed's `format_on_save` for TypeScript is the language formatter (Prettier / vtsls), not every language server on that buffer. If Zed does not send `willSaveWaitUntil` to a secondary server, save will not reformat iso interiors in Zed until that is true. Explicit `textDocument/formatting` that reaches us still edits iso spans only.

## Why isograph's formatter is more than we need

isograph records one `IsographSemanticToken` per consume. That struct is the LSP highlight index plus `LineBehavior` plus `IndentChange`. Highlight and layout are the same constant (`ST_OPEN_BRACE` vs `ST_CLOSE_BRACE`, `ST_KEYWORD_USE` vs `ST_KEYWORD_DECLARATION`, `ST_SELECTION_NAME_OR_ALIAS` vs `ST_SELECTION_NAME_OR_ALIAS_POST_COLON`) because a role and a layout that happen to coincide were fused.

`LineBehavior` is five variants. Three of them wrap structs that exist only to hold `SpaceBefore` / `SpaceAfter`, which are `bool` newtypes with `Deref`. `Remove` means "do not emit this token" (commas). `IsOwnLine` is "starts a line and ends a line." Helpers `starts_new_line`, `ends_line`, `has_space_after`, `has_space_before`, `should_keep` re-derive a lattice the variants already knew.

The printer then combines adjacent tokens two different ways: a newline if either side wants one (`ends_line` OR `starts_new_line`); a space only if both sides want one (`space_after` AND `space_before`). The comment in `format.rs` says the least spacing wins; newlines do not follow that rule.

`IndentChange::Same` is the do-nothing arm of a three-way enum that is otherwise a direction.

i2 already split the highlight role out (`SemanticToken` is `Keyword` / `Brace` / …). Open and close of a group share one role. That is the right split. Layout is a second argument at consume, not a field on `SemanticToken`. semantic-tokens.md's "Formatter metadata" later-change (put `line_behavior` / `indent_change` on `SemanticToken`) is not this work.

This slice does not delete commas. It inserts a comma between list items even when the source used a newline and no comma. Leftover source commas are not printed; the printer's commas are the recorded ones.

## v1 rules

A line break plus indent happens only for `{`. `}` dedents and starts a new line so the close is not glued to the last item. Nothing else breaks: not `(`, not selection names, not arguments.

Exception: a description is always on its own line (`before` and `after` are `Line`). Combined with `{`'s `before: Space`, `max(Line, Space)` is `Line`, so a description is never followed by `{` on the same line.

Commas: one comma between adjacent items of every list (selection set, argument list, variable list, object literal, value list). Space after the comma. No trailing comma. `parse_each_chunk` records that comma; it is not leftover source text.

```
user(arg: {
  b: true
}, c: false)
```

`(` does not break. `{` does. The comma after `}` is the argument-list comma between `arg: { ... }` and `c: false`. Ugly, and v1.

```
field Pet.fullName {
  firstName, lastName
}
```

Both names stay on the indented line. `{` broke once. The comma is between items.

```
field Pet.fullName
"a pet's name"
{
  firstName, lastName
}
```

Description owns its line. `{` is not on that line.

```
field Query.user($id: ID, $x: Int) {
  user(id: $id, extra: {
    b: true
  }) {
    id, name
  }
}
```

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

`Gap` is a lattice: `None < Space < Line`. The gap between two adjacent recorded tokens is `max(prev.after, next.before)`. One combining rule.

`Indent::In` applies after the token (open `{` of a selection set or object literal). `Indent::Out` applies before the token (the matching `}`). A token is not both. `(` / `)` / type-list `[` / `]` use `Indent::None`.

No `bool`. No `SpaceBefore(true)`.

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

`OPEN_BRACE` / `CLOSE_BRACE` are only `{` / `}` of a selection set or object literal. Type-list `[String]` and grouping `(` `)` are `GLUE`.

Empty `{}`: an `In` token immediately followed by the matching `Out` with nothing recorded between them (no items, no commas) uses `Gap::None` between those two, so `{}` stays one pair. That rule lives in the printer. Consume sites always pass `OPEN_BRACE` / `CLOSE_BRACE`.

## Recording

`commit` takes the highlight role and the layout. Leftover fill-in does not.

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

`RecordedToken.layout` `Some` is consume, `None` is leftover fill-in. The printer walks consume tokens and `Comma`. The highlighter walks `Recorded::Token` only. `Comma` has no span: it is not in the source. `ResolvePosition` does not enter it.

`parse_each_chunk` after a successfully parsed item that is not the last item of the level records `Recorded::Comma(Layout::COMMA)`. A failed item does not get a comma. A trailing source comma is leftover, not this.

Before:

```rust
cursor.require_token(NonBracketTokenKind::Period, IsographSemanticToken::Period)
cursor.require_group(BracketKind::Brace, IsographSemanticToken::Brace, parse_inside)
```

After:

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

`require_group` / `consume_group_if` take two layouts. Open and close keep one highlight role. `RecordGroupClose` stores the close layout. Parenthesis groups and type-list brackets pass `Layout::GLUE`, `Layout::GLUE`.

`peek.advance()` on `!` today records nothing. The printer would drop bangs. `!` is consumed with `IsographSemanticToken::Content` and `Layout::GLUE`. Leftover `!` is already `Content`. Parsed `!` becomes the same highlight. Resolve still does not enter `!`; this does not add a resolve leaf.

`NoSemanticTokens::record` ignores layout and commas. Compile stays a noop collector.

## Consume sites

Every grammar consume passes a layout. Catalog:

- Declaration keyword (`entrypoint`, `field`): `SPACE_AFTER`
- Type in `Type.name`: `GLUE` (space comes from the keyword's `after`)
- `.`: `GLUE`
- Selectable name in `Type.name`: `GLUE`
- `to`: `SPACE`
- Type-annotation identifier: `GLUE`
- Type-list `[` / `]`: `GLUE`, `Indent::None`
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
- Comma between list items: `Recorded::Comma(Layout::COMMA)`, from `parse_each_chunk`, not from source

`max(description.after=Line, brace.before=Space)` is `Line`, so `{` after a description starts a new line.

Tests in `chunk_stream.rs` that call `require_token` / `consume_group_if` pass `Layout::GLUE` unless they assert layout.

## Printer

Pure function. No pico. No LSP.

```rust
// from crates/isograph_parser/src/format.rs
pub fn format_recorded(text: &str, tokens: &[Recorded]) -> String
```

Walk `Recorded::Token` whose `layout` is `Some`, and `Recorded::Comma`. Skip leftover (`layout: None`). A comma's text is `","`.

```
indent = 0
last_after = Gap::None
output empty

for each printable:
    if layout.indent is Out: indent = indent - 1 (do not go below 0)
    if this token is Out and the previous emitted token was the matching In
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

"Matching In" is: previous emitted token had `Indent::In` and this token has `Indent::Out`. Used only for empty `{}`. Do not look at highlight role.

Width is 2 spaces. `FormattingOptions.tabSize` / `insertSpaces` from LSP are ignored. No printer config. No printed-width.

Parse failure of a literal: the caller does not invoke `format_recorded` for that extraction. No edit.

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

`field Pet.x { id }` becomes `field Pet.x {\n  id\n}`. `field Pet.x {}` stays `field Pet.x {}`. `entrypoint Query.HomeRoute` stays one line. `user(arg: { b: true }, c: false)` becomes `user(arg: {\n  b: true\n}, c: false)`.

## Handlers

iso-format-lsp.md's `formatting_response` and `will_save_wait_until_response` ignore state and return `[]`. This slice gives both the same body: `format_iso_literals`.

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>>
```

If request dispatch already passes `LspClientId`, this function takes it and reads `lsp_file_contents`. If not, it reads `DiskFile` the way `semantic_tokens_response` does today.

For each extraction in the file: if parse succeeded, one `TextEdit` whose range is the iso interior (extraction start and length, converted the way tokens already convert) and whose `new_text` is `format_recorded`. Failed parse: no edit for that extraction. No iso literals: empty vec, not `null`. Missing file: `null` (same as tokens). That last case is the only change from iso-format-lsp.md's always-`[]` for a never-interned URI.

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

`ExpectedError` is not used. Missing file is `Ok(None)` JSON `null`, same as tokens.

## Changes

Two independently shippable changes. Prefactor first. The LSP methods already exist.

### Change 1: `Layout` at consume, commas between list items

`layout.rs`, `Recorded`, `commit` / `require_token` / `require_group` take layout, leftover still `layout: None`. `parse_each_chunk` records `Recorded::Comma(Layout::COMMA)` between successful items. Catalog above at every consume.

Tests: a `field Query.x { id, name }` parse records `Keyword SPACE_AFTER`, `Type GLUE`, `Period GLUE`, `FieldName GLUE`, `Brace OPEN_BRACE`, `FieldName GLUE`, `Comma COMMA`, `FieldName GLUE`, `Brace CLOSE_BRACE`. `field Query.x { id }` has no `Comma`. `user(a: 1, b: 2)` records `Comma` between the two arguments; both parens are `GLUE`. Failed `require_token` records nothing, including no layout. `!` is recorded. A description records `OWN_LINE`. Compile `NoSemanticTokens` still compiles.

Highlighting tests stay green: they read `.token` on `Recorded::Token`, not `.layout`, and skip `Recorded::Comma`. iso-format-lsp.md tests still get `[]`.

### Change 2: `format_recorded` and the handler body

The function above, then `format_iso_literals` in the two existing handlers.

Printer tests (string equality, no snapshot files):

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
- leftover-only tokens in the vec do not appear as extra words
- `indent` never prints a negative repeat (a lone close still emits)

Do not call LSP in the printer tests.

`lsp_socket.rs` listen_and_reply, replacing the empty-result facts from iso-format-lsp.md:

- `textDocument/formatting` after intern/open of a file with `iso(\`field Query.x{id}\`)`: one edit, `new_text` is the formatted interior, range covers that interior only
- a file with one good literal and one `iso(\`not a declaration\`)`: one edit, the broken interior unchanged
- missing URI: `result` is JSON `null`
- `willSaveWaitUntil` for the interned file: same edits as formatting

e2e: save is not an LSP method we send from `cli.rs`. The willSaveWaitUntil request in the socket test is the save path.

## Call sites

- grammar consume → `commit(token, layout)`
- `parse_each_chunk` between items → `Recorded::Comma`
- leftover fill-in → `Recorded::Token { layout: None, ... }`
- `formatting_response` / `will_save_wait_until_response` → `format_iso_literals` → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save (already sent after iso-format-lsp.md)
- highlighter → `Recorded::Token.token` (unchanged encoding)

## Docs this change amends

iso-format-lsp.md: the two handlers stop returning `[]` for a present file.

semantic-tokens.md "Formatter metadata": do not add `line_behavior` / `indent_change` to `SemanticToken`. This file.

vscode-extension.md: the setting stays gone. On-save format is the server capability, not a client hook.

zed-and-vscode-extensions.md: format is iso interiors on `willSaveWaitUntil` / `formatting`. Not a second highlighter.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.

Broken `(` lists, width, one-item-per-line nested lists, and trailing commas on broken lists are iso-format-later.md.
