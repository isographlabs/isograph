# Format iso literals on save

Requires lsp-tokens.md (landed) and the consume-time `SemanticToken` recording in the parser. Independent of lsp-diagnostics.md. Uses open-file contents when lsp-open-files.md change 3 has landed; until then, disk.

Save reformats every successfully parsed iso interior in the file. JS/TS around the literals is not touched. Prettier (or vtsls) stays the TypeScript document formatter. There is no `isograph.autoformatIsoLiterals` setting.

Spacing is recorded at consume, next to the semantic token, not stuffed into `SemanticToken`. The highlighter ignores layout. The printer ignores highlight role. Leftover fill-in is highlight-only and is not printed.

## What the user does

A TypeScript file contains messy interiors:

```
export const Home = iso(`field Query.HomeRoute{id  name}`)
```

Save. The interior becomes:

```
field Query.HomeRoute {
  id
  name
}
```

The `iso(\`` and closing backtick do not move. A second literal in the same file that failed to parse is left as-is. A literal that parsed is formatted even if another one failed.

VS Code: this happens on save because the server advertised `willSaveWaitUntil`. The user does not turn on a setting. `editor.formatOnSave` can stay aimed at Prettier; that run does not have to see our edits.

Zed: we advertise the same capability. Zed's `format_on_save` for TypeScript is the language formatter (Prettier / vtsls), not every language server on that buffer. If Zed does not send `willSaveWaitUntil` to a secondary server, save will not reformat iso interiors in Zed until that is true. The printer and the `textDocument/formatting` arm still exist; explicit format requests that reach us still edit iso spans only.

## Why isograph's formatter is more than we need

isograph records one `IsographSemanticToken` per consume. That struct is the LSP highlight index plus `LineBehavior` plus `IndentChange`. Highlight and layout are the same constant (`ST_OPEN_BRACE` vs `ST_CLOSE_BRACE`, `ST_KEYWORD_USE` vs `ST_KEYWORD_DECLARATION`, `ST_SELECTION_NAME_OR_ALIAS` vs `ST_SELECTION_NAME_OR_ALIAS_POST_COLON`) because a role and a layout that happen to coincide were fused.

`LineBehavior` is five variants. Three of them wrap structs that exist only to hold `SpaceBefore` / `SpaceAfter`, which are `bool` newtypes with `Deref`. `Remove` means "do not emit this token" (commas). `IsOwnLine` is "starts a line and ends a line." Helpers `starts_new_line`, `ends_line`, `has_space_after`, `has_space_before`, `should_keep` re-derive a lattice the variants already knew.

The printer then combines adjacent tokens two different ways: a newline if either side wants one (`ends_line` OR `starts_new_line`); a space only if both sides want one (`space_after` AND `space_before`). The comment in `format.rs` says the least spacing wins; newlines do not follow that rule.

`IndentChange::Same` is the do-nothing arm of a three-way enum that is otherwise a direction.

i2 already split the highlight role out (`SemanticToken` is `Keyword` / `Brace` / …). Open and close of a group share one role. That is the right split. Layout is a second argument at consume, not a field on `SemanticToken`. semantic-tokens.md's "Formatter metadata" later-change (put `line_behavior` / `indent_change` on `SemanticToken`) is not this work.

We do not have `Remove`. Selection sets are newline-separated; leftover commas are not in the parse-time stream the printer walks.

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

`Gap` is a lattice: `None < Space < Line`. The gap between two adjacent recorded tokens is `max(prev.after, next.before)`. One combining rule. No AND for spaces and OR for newlines.

`Indent::In` applies after the token (open `{` / `(` / `[` that owns a list). `Indent::Out` applies before the token (the matching close). A token is not both.

No `bool`. No `SpaceBefore(true)`.

Associated values, so consume sites are not struct literals. Names are the layout, not a new vocabulary of token kinds.

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
    pub const LINE_BEFORE: Layout = Layout {
        before: Gap::Line,
        after: Gap::None,
        indent: Indent::None,
    };
    pub const OPEN_LIST: Layout = Layout {
        before: Gap::Space,
        after: Gap::Line,
        indent: Indent::In,
    };
    pub const CLOSE_LIST: Layout = Layout {
        before: Gap::Line,
        after: Gap::None,
        indent: Indent::Out,
    };
    pub const OPEN_ARGS: Layout = Layout {
        before: Gap::None,
        after: Gap::Line,
        indent: Indent::In,
    };
    pub const CLOSE_ARGS: Layout = Layout {
        before: Gap::Line,
        after: Gap::None,
        indent: Indent::Out,
    };
}
```

`OPEN_LIST` is `{` of a selection set or object literal (space before, break after). `OPEN_ARGS` is `(` of arguments or variable definitions (glued to the name, break after). Brackets of a type list `[String]` are `GLUE` on open and close, `Indent::None`.

Empty lists would otherwise print as `{` / newline / `}`. The printer special-cases an `In` token immediately followed by the matching `Out` token with nothing recorded between them: the gap between those two is `Gap::None`, so `{}` and `()` stay one token pair. That rule lives in the printer. Consume sites always pass `OPEN_LIST` / `CLOSE_LIST` (or the args pair). They do not know emptiness yet.

## Recording

`commit` takes the highlight role and the layout. Leftover fill-in does not.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub struct Recorded {
    pub span: Span,
    pub token: IsographSemanticToken,
    pub layout: Option<Layout>,
}
```

`Some(layout)` is a consume. `None` is leftover fill-in. The printer walks `Some` only. The highlighter walks every `token`. `Option` here is "this span was produced by consume, or it was filled in later", not a yes/no flag on one concept.

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
    Layout::OPEN_LIST,
    Layout::CLOSE_LIST,
    parse_inside,
)
```

`require_group` / `consume_group_if` take two layouts. Open and close keep one highlight role (`Brace`, `Parenthesis`, `GraphQLTypeName` for type-list brackets). `RecordGroupClose` stores the close layout.

`peek.advance()` on `!` today records nothing. The printer would drop bangs. `!` is consumed with `IsographSemanticToken::Content` and `Layout::GLUE`. Leftover `!` is already `Content`. Parsed `!` becomes the same highlight. Resolve still does not enter `!`; this does not add a resolve leaf.

`NoSemanticTokens::record` ignores layout. Compile stays a noop collector.

## Consume sites

Every grammar consume passes a layout. Catalog:

- Declaration keyword (`entrypoint`, `field`): `SPACE_AFTER`
- Type in `Type.name`: `GLUE` (space comes from the keyword's `after`)
- `.`: `GLUE`
- Selectable name in `Type.name`: `GLUE`
- `to`: `SPACE`
- Type-annotation identifier: `GLUE`
- Type-list `[` / `]`: `GLUE`, `Indent::None` (not a list that breaks)
- `!`: `GLUE`, `Content`
- Description string / block string: `LINE_BEFORE` (after directives, before the selection set)
- Selection name or alias (first identifier of a selection): `LINE_BEFORE`
- `:` of `alias: name`, of `name: Type`, of `name: value`: `SPACE_AFTER` (glued to the left, space after)

- Name after `alias:`: `GLUE` (space comes from colon `after`)
- Selection-set `{` / `}`: `OPEN_LIST` / `CLOSE_LIST`
- Argument list `(` / `)`: `OPEN_ARGS` / `CLOSE_ARGS`
- Variable declaration list `(` / `)`: `OPEN_ARGS` / `CLOSE_ARGS`
- Object literal `{` / `}`: `OPEN_LIST` / `CLOSE_LIST`
- Value list `[` / `]`: `OPEN_LIST` / `CLOSE_LIST`
- Argument name / object key: `LINE_BEFORE`
- `$` of a variable: declaration in a `(` list is `LINE_BEFORE`; usage in a value is `GLUE` (space comes from colon or from `SPACE` of a sibling). Two consume sites already exist (`parse_variable_name` vs value). Declaration: `LINE_BEFORE` on `$`, `GLUE` on the identifier. Usage: `GLUE` on both.
- `=`: `SPACE`
- Integer / string / boolean / null as values: `GLUE` (spaces come from colon / equals)
- `@`: `before: Space, after: None`:

```rust
    pub const AT: Layout = Layout {
        before: Gap::Space,
        after: Gap::None,
        indent: Indent::None,
    };
```

- Directive identifier: `GLUE`

Directive on a selection that already broke (`LINE_BEFORE` on the name): `max(name.after=None, at.before=Space)` is a space: `name @loadable`. Directive after `Type.name` on a field declaration: `max(GLUE.after, AT.before)` is a space: `field Pet.x @loadable {`.

Tests in `chunk_stream.rs` that call `require_token` / `consume_group_if` pass `Layout::GLUE` unless they assert layout.

## Printer

Pure function. No pico. No LSP.

```rust
// from crates/isograph_parser/src/format.rs
pub fn format_recorded(text: &str, tokens: &[Recorded]) -> String
```

Walk `tokens` whose `layout` is `Some`. Skip leftover (`None`).

```
indent = 0
last_after = Gap::None
output empty

for each parsed token:
    if layout.indent is Out: indent = indent - 1 (do not go below 0)
    if this token is Out and the previous emitted token was the matching In
        with no parsed token between them:
        gap = Gap::None
    else:
        gap = max(last_after, layout.before)
    if gap is Space: push ' '
    if gap is Line: push '\n' then "  " repeated indent times
    push text[span]
    last_after = layout.after
    if layout.indent is In: indent = indent + 1

if last_after is Line: push '\n'
```

"Matching In" is: previous emitted token had `Indent::In` and this token has `Indent::Out`. Used only for the empty `{}` / `()` glue. Do not look at highlight role.

Width is 2 spaces. `FormattingOptions.tabSize` / `insertSpaces` from LSP are ignored. No printer config.

Parse failure of a literal: the caller does not invoke `format_recorded` for that extraction. No edit.

## Target shape

```
entrypoint Query.HomeRoute

field Pet.fullName {
  firstName
  lastName
}

field Query.user(
  $id: ID
) {
  user(id: $id) {
    id
  }
}

field Pet.Avatar to Image {
  url
}
```

One-line `field Pet.x { id }` becomes the multiline form. `field Pet.x {}` stays `field Pet.x {}` (empty-list glue). `entrypoint Query.HomeRoute` stays one line.

## On save

Origin of the request: `textDocument/formatting` and `textDocument/willSaveWaitUntil`. Origin of the client hook: isograph `vscode-extension/src/languageClient.ts` `onWillSaveTextDocument` gated on `autoformatIsoLiterals`. Delta: the server advertises `willSaveWaitUntil`. The client does not send formatting itself. The setting does not exist.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
                lsp_types::TextDocumentSyncOptions {
                    open_close: true.wrap_some(),
                    change: lsp_types::TextDocumentSyncKind::FULL.wrap_some(),
                    will_save_wait_until: true.wrap_some(),
                    ..Default::default()
                },
            )
            .wrap_some(),
            document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

`will_save_wait_until: true` and `OneOf::Left(true)` are `lsp_types` protocol fields, not domain bools.

Both requests call one function:

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>>
```

For each extraction in the file (open then disk, same path as tokens): if parse succeeded, one `TextEdit` whose range is the iso interior (extraction start and length, converted the way tokens already convert) and whose `new_text` is `format_recorded`. Failed parse: no edit for that extraction. No iso literals: empty vec, not `null`. Missing file: `null` (same as tokens).

`textDocument/formatting` and `textDocument/willSaveWaitUntil` are `.on_request_sync` arms next to semantic tokens. They take `LspClientId` once request dispatch has it.

vscode-extension.md dropped the will-save handler and the setting. They stay dropped. vscode-languageclient already sends `willSaveWaitUntil` when the server advertises it. Do not re-add `autoformatIsoLiterals`.

## Changes

Three independently shippable changes. Prefactor first.

### Change 1: `Layout` at consume

`layout.rs`, `Recorded`, `commit` / `require_token` / `require_group` take layout, leftover still `layout: None`. Catalog above at every consume. Tests: a `field Query.x { id }` parse records `Keyword SPACE_AFTER`, `Type GLUE`, `Period GLUE`, `FieldName GLUE`, `Brace OPEN_LIST`, `FieldName LINE_BEFORE`, `Brace CLOSE_LIST`. Failed `require_token` records nothing, including no layout. `!` is recorded. Compile `NoSemanticTokens` still compiles.

Highlighting tests stay green: they read `.token`, not `.layout`.

### Change 2: `format_recorded`

The function above. Tests (string equality, no snapshot files):

- `field Query.x{id}` → `field Query.x {\n  id\n}`
- `field Query.x { id }` → the same
- `field Query.x {}` → `field Query.x {}`
- `entrypoint Query.HomeRoute` → itself
- `field Pet.x to Image { id }` → `field Pet.x to Image {\n  id\n}`
- `field Query.user($id:ID){user(id:$id){id}}` → the multiline target shape
- `id @loadable` inside a set: space before `@`
- `Query.x` keeps the dot glued
- leftover-only tokens in the vec do not appear as extra words
- `indent` never prints a negative repeat (a lone close still emits)

Do not call LSP.

### Change 3: LSP format and save

Advertise capabilities. Two request arms. `format_iso_literals` as above. Tests in `lsp_socket.rs` listen_and_reply:

- `textDocument/formatting` after intern/open of a file with `iso(\`field Query.x{id}\`)`: one edit, `new_text` is the formatted interior, range covers that interior only
- a file with one good literal and one `iso(\`not a declaration\`)`: one edit, the broken interior unchanged
- missing URI: `null`
- `willSaveWaitUntil` with the same params shape as the spec (`TextDocumentIdentifier`): same edits as formatting

e2e: save is not an LSP method we send from `cli.rs`. The willSaveWaitUntil request in the socket test is the save path.

## Call sites

- grammar consume → `commit(token, layout)`
- leftover fill-in → `Recorded { layout: None, ... }`
- `textDocument/formatting` / `willSaveWaitUntil` → extractions → `format_recorded` → `TextEdit`s
- vscode-languageclient → `willSaveWaitUntil` on save
- highlighter → `Recorded.token` (unchanged encoding)

## Docs this change amends

semantic-tokens.md "Formatter metadata": do not add `line_behavior` / `indent_change` to `SemanticToken`. This file.

vscode-extension.md: the setting stays gone. On-save format is the server capability, not a client hook.

zed-and-vscode-extensions.md: format is iso interiors on `willSaveWaitUntil` / `formatting`. Not a second highlighter.

lsp-open-files.md: format reads `lsp_file_contents` once that exists.
