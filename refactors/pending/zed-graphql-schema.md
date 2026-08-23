# Zed extension: GraphQL schema highlighting

Requires zed-and-vscode-extensions.md (the `zed-extension/` crate, `language_server_command` for `isograph lsp`). Independent of lsp-sessions.md, lsp-open-files.md, vscode-config-discovery.md.

Iso literals in JS/TS are LSP semantic tokens (zed-and-vscode-extensions.md). GraphQL schema files are not. They are a file type (`.graphql`) the Isograph daemon reads as schema / schema-extension sources. Zed colors a file type with tree-sitter: a language directory plus a grammar registered in `extension.toml`. This slice is that language. It does not spawn a GraphQL language server. It does not color `graphql` / `gql` tagged templates inside JS/TS (an extension cannot add `injections.scm` to Zed's built-in TypeScript).

Origin of the language directory and queries: `11bit/zed-extension-graphql` `languages/graphql/`. Origin of the grammar: `11bit/tree-sitter-graphql` at the commit that extension pins. Origin of schema-only scope in the compiler: isograph `crates/graphql_schema_parser` (SDL, not executable operations). Delta from that GraphQL extension: no `[language_servers.graphql]`, no `language_ids` for JS/TS/Vue/Astro/Svelte, no graphql-language-service. Queries live in our extension; the grammar is a git pin, not a vendored copy. Capture names that are not in Zed's highlight list are mapped (below).

Upstream isograph `tree-sitter/` is a grammar for the iso language (`entrypoint` / `field` / `pointer`), file type `.isograph`. It is not this. It cannot color `iso(\`...\`)` inside `.ts`. Do not register it here.

Conflict, do not implement the registration until it is resolved. `11bit/zed-extension-graphql` (Zed extension id `graphql`) already uses:

- language `name = "GraphQL"`
- `[grammars.graphql]`
- `path_suffixes = ["graphql", "gql", "graphqls"]`

Those are the names and suffixes a schema highlighter for `schema.graphql` wants. Shipping them in the Isograph extension steals those binds. A different `name` / grammar id / suffix list either fails to open `schema.graphql` or still collides on the suffix. Zed has no `extensionDependencies`. This file specifies the queries, the grammar pin, and the tests. It does not pick a winner for the three binds.

Two shippable changes after the binds are decided. Change 1 is the language. Change 2 is a parse/highlight test that does not run Zed.

## What the user does

Install Dev Extension on `zed-extension/`. Open an Isograph project's `schema.graphql` (the path `isograph.config.json` names as schema).

```
type Pet {
  id: ID!
  name: String
}
```

`type` is a keyword. `Pet` is a type. `id` and `name` are properties. `#` comments toggle as line comments. `{` / `}` match.

No GraphQL language server starts. No completion, validation, or hover from graphql-language-service. Iso literals in `.ts` are still `isograph lsp` semantic tokens.

## Change 1: language directory and grammar pin

`zed-extension/` already has `extension.toml`, `Cargo.toml`, `src/lib.rs`. This change adds `languages/graphql/` and a `[grammars.*]` entry. `language_server_command` is unchanged. Opening a `.graphql` file does not spawn `isograph lsp` (`languages` on `[language_servers.isograph]` stays the four JS/TS names).

### Types

Most important first.

The language metadata. `name`, `grammar`, and `path_suffixes` are the conflict. Copied from origin so the rest of the file is the query set that grammar expects. Do not land this `config.toml` until the binds are resolved.

```toml
# from zed-extension/languages/graphql/config.toml
# origin: 11bit/zed-extension-graphql languages/graphql/config.toml
name = "GraphQL"
grammar = "graphql"
path_suffixes = ["graphql", "gql", "graphqls"]
line_comments = ["# "]
autoclose_before = "}])"
brackets = [
    { start = "{", end = "}", close = true, newline = true },
    { start = "(", end = ")", close = true, newline = true },
    { start = "[", end = "]", close = true, newline = true },
    { start = "\"", end = "\"", close = true, newline = true },
]
prettier_parser_name = "graphql"
```

Delta from origin: none on this file. `opt_into_language_servers` is omitted; Isograph's server is not a GraphQL server.

```toml
# from zed-extension/extension.toml
[grammars.graphql]
repository = "https://github.com/11bit/tree-sitter-graphql"
rev = "951bde9fb3145b5f676204231e35f8b21d21f7b3"
```

Origin: that extension's `[grammars.graphql]` (`commit` in their toml; Zed's field is `rev`). Same repo, same sha. Local iteration can use a `file://` URL instead of the github URL; ship the github pin.

`src/lib.rs` does not mention the grammar. Zed compiles tree-sitter C to wasm with wasi-sdk when it installs the extension. The extension wasm (`language_server_command`) is a different wasm.

### Queries

Copied from `11bit/zed-extension-graphql` `languages/graphql/`. Node names match `11bit/tree-sitter-graphql` at that sha. Delta: captures that are not in Zed's highlight list (`zed.dev/docs/extensions/languages`, Syntax highlighting) are replaced with ones that are.

`@float` is not in that list. `(float_value) @float` becomes `@number`.

`@parameter` is not in that list. Zed has `@variable.parameter`. Argument names and variable definitions use that.

Everything else is verbatim.

```scheme
; from zed-extension/languages/graphql/highlights.scm
; origin: 11bit/zed-extension-graphql languages/graphql/highlights.scm
; Types
;------

(scalar_type_definition
  (name) @type)

(object_type_definition
  (name) @type)

(interface_type_definition
  (name) @type)

(union_type_definition
  (name) @type)

(enum_type_definition
  (name) @type)

(input_object_type_definition
  (name) @type)

(directive_definition
  (name) @type)

(directive_definition
  "@" @type)

(scalar_type_extension
  (name) @type)

(object_type_extension
  (name) @type)

(interface_type_extension
  (name) @type)

(union_type_extension
  (name) @type)

(enum_type_extension
  (name) @type)

(input_object_type_extension
  (name) @type)

(named_type
  (name) @type)

(directive) @type

; Properties
;-----------

(field
  (name) @property)

(field
  (alias
    (name) @property))

(field_definition
  (name) @property)

(object_value
  (object_field
    (name) @property))

(enum_value
  (name) @property)

(input_fields_definition
  (input_value_definition
    (name) @property))

; Variable Definitions and Arguments
;-----------------------------------

(operation_definition
  (name) @variable)

(fragment_name
  (name) @variable)

(argument
  (name) @variable.parameter)

(arguments_definition
  (input_value_definition
    (name) @variable.parameter))

(variable_definition
  (variable) @variable.parameter)

(argument
  (value
    (variable) @variable))

; Constants
;----------

(string_value) @string

(int_value) @number

(float_value) @number

(boolean_value) @boolean

; Literals
;---------

(description) @comment

(comment) @comment

(directive_location
  (executable_directive_location) @type.builtin)

(directive_location
  (type_system_directive_location) @type.builtin)

; Keywords
;----------

[
  "query"
  "mutation"
  "subscription"
  "fragment"
  "scalar"
  "type"
  "interface"
  "union"
  "enum"
  "input"
  "extend"
  "directive"
  "schema"
  "on"
  "repeatable"
  "implements"
] @keyword

; Punctuation
;------------

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

"=" @operator

"|" @punctuation.delimiter
"&" @punctuation.delimiter
":" @punctuation.delimiter

"..." @punctuation.special
"!" @punctuation.special
```

The grammar parses executable documents as well as SDL. The query file therefore has `query` / `mutation` / `fragment` / `operation_definition`. We do not add a second grammar. A `.graphql` operation file that is not a schema still highlights. The compiler still only parses SDL (`graphql_schema_parser`).

```scheme
; from zed-extension/languages/graphql/brackets.scm
; origin: 11bit/zed-extension-graphql languages/graphql/brackets.scm
("(" @open ")" @close)
("{" @open "}" @close)
("[" @open "]" @close)
```

```scheme
; from zed-extension/languages/graphql/indents.scm
; origin: 11bit/zed-extension-graphql languages/graphql/indents.scm
(_ "{" "}" @end) @indent
(_ "(" ")" @end) @indent
(_ "[" "]" @end) @indent
```

```scheme
; from zed-extension/languages/graphql/outline.scm
; origin: 11bit/zed-extension-graphql languages/graphql/outline.scm
; Operations (named only)
(operation_definition
    (operation_type) @context
    (name) @name) @item

; Fragments
(fragment_definition
    "fragment" @context
    (fragment_name (name) @name)) @item

; Type system definitions
(object_type_definition
    "type" @context
    (name) @name) @item

(interface_type_definition
    "interface" @context
    (name) @name) @item

(enum_type_definition
    "enum" @context
    (name) @name) @item

(union_type_definition
    "union" @context
    (name) @name) @item

(input_object_type_definition
    "input" @context
    (name) @name) @item

(scalar_type_definition
    "scalar" @context
    (name) @name) @item

(directive_definition
    "directive" @context
    (name) @name) @item

; Type extensions
(object_type_extension
    "extend" @context
    "type" @context
    (name) @name) @item

(interface_type_extension
    "extend" @context
    "interface" @context
    (name) @name) @item

(enum_type_extension
    "extend" @context
    "enum" @context
    (name) @name) @item

(union_type_extension
    "extend" @context
    "union" @context
    (name) @name) @item

(input_object_type_extension
    "extend" @context
    "input" @context
    (name) @name) @item

(scalar_type_extension
    "extend" @context
    "scalar" @context
    (name) @name) @item
```

No `injections.scm`. No `overrides.scm`. No `textobjects.scm`. No `redactions.scm`. No `runnables.scm`. No `semantic_token_rules.json`.

### README

`zed-extension/README.md` (from zed-and-vscode-extensions.md change 1) gains: schema files are tree-sitter, not `isograph lsp`. Iso literals still need `semantic_tokens` combined or full on TypeScript/TSX.

### Tests

No production function only tests call.

`cargo check --target wasm32-wasip2 --manifest-path zed-extension/Cargo.toml` stays green. The grammar is not that crate.

Handle tests of `didOpen` on a `.ts` file are unchanged. A `.graphql` URI is not a JS/TS document; `isograph lsp` does not advertise this language.

## Change 2: parse and highlight a schema fixture without Zed

Clone the pinned grammar, build it with tree-sitter CLI, run our queries against a fixture. Assert facts about captures. No checked-in expected-output file. No Zed. No pixel.

Fixture, inputs only:

```graphql
# from zed-extension/tests/schema.graphql
"""Pet in the store."""
type Pet {
  id: ID!
  name: String
}

enum Status {
  AVAILABLE
}

directive @oneOf on INPUT_OBJECT
```

`type`, `enum`, `directive`, `on` are `@keyword`. `Pet`, `Status`, `ID`, `String`, `INPUT_OBJECT` are `@type`. `id`, `name`, `AVAILABLE` are `@property`. The description is `@comment`. `!` is `@punctuation.special`.

```yaml
# from .github/workflows/ci.yml
  zed-graphql-grammar:
    name: zed graphql grammar
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install tree-sitter CLI
        run: cargo install tree-sitter-cli --locked
      - name: Fetch grammar
        run: |
          git clone https://github.com/11bit/tree-sitter-graphql grammar
          git -C grammar checkout 951bde9fb3145b5f676204231e35f8b21d21f7b3
      - name: Build grammar
        working-directory: grammar
        run: tree-sitter generate && tree-sitter build
      - name: Parse fixture
        run: tree-sitter parse --quiet zed-extension/tests/schema.graphql
        working-directory: grammar
      - name: Highlight fixture
        run: tree-sitter query --captures ../zed-extension/languages/graphql/highlights.scm ../zed-extension/tests/schema.graphql
        working-directory: grammar
```

The highlight step's stdout is the captures. The job fails if `type` is missing as `@keyword` or `Pet` as `@type`. Implement that as a small assertion in the workflow (python or a `crates/tests` binary that reads the query CLI output). Do not commit the CLI dump.

`all-checks-passed.needs` appends `zed-graphql-grammar`.

A parse error on the fixture fails `tree-sitter parse --quiet` (non-zero). That is the degenerate case: empty file, and a file that is only `type` with no name, if we add those as extra fixtures. Empty file: parse succeeds with an empty document (assert no error node). `type {` with no close: error node present; the test that expects clean SDL is the well-formed fixture.

### Call sites

- Zed loads `languages/graphql/config.toml` for matching suffixes
- Zed clones `[grammars.graphql].repository` at `rev`, compiles parser.c
- Zed runs `highlights.scm` / `brackets.scm` / `indents.scm` / `outline.scm` on the tree
- `isograph lsp` is not started for this language

### Docs this change amends

`refactors/pending/zed-and-vscode-extensions.md`: the extension does define one language, GraphQL schema files, this file. A language directory for `.iso` is still not added. Iso-literal coloring stays LSP semantic tokens.

`refactors/pending/event-model.md` item 25 stays zed-and-vscode-extensions.md. This file is after that.

## What this is not

Not the compiler's `graphql_schema_parser`. Not hover or diagnostics on schema files. Not GraphQL tagged templates. Not registering `tree-sitter-isograph` from upstream `tree-sitter/`.
