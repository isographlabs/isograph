# Semantic tokens for the iso literals in a DiskFile

Requires extract-iso-literals-from-file.md (landed), memoized-parse-iso-literal.md (landed), literal-id.md (landed), lsp-semantic-token-encoding.md (landed), and lsp-semantic-tokens-offset.md (landed). Extract finds the literals. `parsed_iso_literal` records `ParsedIsoLiteral` with spans relative to the literal text. `parsed_iso_literals_in_file` is those extractions plus those trees, in extract order. `lsp_semantic_tokens_for_file` passes the file text and every `(extraction.iso_literal_start_index, parsed.tokens.as_slice())` in extract order to one `lsp_semantic_tokens` call. It does not encode one extraction. File-absolute `WithSpan<IsographSemanticToken>` is not interned and not exported. Parse stays keyed on the literal text.

Every document-scoped LSP method that walks iso interiors needs the AST and the extraction (start index, and later host context). There is no intern of trees without extractions, and no intern of start indices without trees. Cursor APIs convert `path` and `LineChar` to `LiteralId`. This file’s parse intern is `parsed_iso_literal(text)` via `parsed_iso_literals_in_file`.

Origin of the pipeline: isograph `get_semantic_tokens`. Origin of encoding: landed `lsp_semantic_tokens`. Delta: pico memo over `DiskFile`; each item is extraction plus tree; relative tokens stay relative; one encoder call for the file; no concat intern.

The LSP adapter (event-model.md, not written) will call this on `semanticTokens/full`. This slice does not start the adapter. `OpenFile` is not implemented yet; tests intern a `DiskFile` and assert encoded tokens. Encoding is not a memo. Memoizing encoded tokens is semantic-tokens-line-offset.md.

One shippable change: `IsographState::disk_file`, one compiler memo, and `lsp_semantic_tokens_for_file`. This change amends `docs-website/docs/design-docs/pico.md`.

## What the user does

No editor highlighting until the adapter. Tests intern

```
export const Home = iso(`entrypoint Query.HomeRoute`)
```

and assert that `lsp_semantic_tokens_for_file` encodes `entrypoint` as keyword at its file column.

## Types

Most important first.

```text
lsp_semantic_tokens_for_file(path)
  -> parsed_iso_literals_in_file(path)
  + DiskFile contents
  -> lsp_semantic_tokens(page_content, (offset, tokens) per literal)

parsed_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
  + parsed_iso_literal(text) for each extraction
```

`parsed_iso_literals_in_file` is `Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>>`. A prepend makes the vec `!=` (`iso_literal_start_index` moved). The tree stays relative; `parsed_iso_literal` of the same text is `==`. A context-only rename of the export (`Home` to `Page`, same length) makes the vec `!=` (`LiteralContext`). The tree is `==`. `iso_literal_start_index` is `==`. Highlighting reads start index and tokens, not context. Document-scoped methods that need the export name read context from the same intern.

This is not `file_literals`. `file_literals` is TypeScript, borrowed, and includes file-absolute host and parse errors. Diagnostics stay on that function.

pico lookup of an `Option` memo is `&Option<T>`. Callers write `.as_ref()?`.

The memo goes in `crates/isograph_compiler/src/iso_literals.rs`, next to `parsed_iso_literal`. `IsoLiteralExtraction` and `parsed_iso_literal` are already in this module.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| {
            (
                extraction.clone(),
                parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone(),
            )
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso literals.

A parse with errors still has leftover tokens. Use them.

Tokens from different literals do not overlap: they sit inside disjoint backtick spans. `lsp_semantic_tokens` asserts that. JS between literals has no iso tokens.

`path` is `RelativePathToSourceFile` (interned, `Copy`). It is the intern param of `parsed_iso_literals_in_file`. The inner parse intern is the literal text. `parsed_iso_literal` already exists.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, LiteralId, iso_literal_extraction, literal_id_at_location, parsed_iso_literal,
    parsed_iso_literals_in_file,
};
```

The existing `pub use` already names `LineChar`, `LiteralId`, `iso_literal_extraction`, `literal_id_at_location`, and `parsed_iso_literal`. Add `parsed_iso_literals_in_file`.

`literal_id_at_location` and this function both look up the `DiskFile` after another memo has already returned `Some` for this path. Promote the test-only `disk_file` helper in `database.rs` onto `IsographState`. A miss does not `tracked()` the map: the caller already subscribed via extract / `parsed_iso_literals_in_file`. Extract is the first lookup of a path and on miss must `tracked()` the map. It does not call `disk_file`.

```rust
// from crates/isograph_compiler/src/database.rs
pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
    let source_id = self.get_disk_file_map().untracked().0.get(&path).copied()?;
    self.get(source_id).wrap_some()
}
```

Add it next to `insert_disk_file` / `remove_disk_file`. Add `use prelude::Postfix` to that module. Delete the test-only `fn disk_file`. Those tests call `state.disk_file(path)`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
// in literal_id_at_location
let file_content = db.disk_file(path)?.contents.reference();
```

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{HostLanguage, IsographState, parsed_iso_literals_in_file};
use prelude::Postfix;

use crate::lsp_semantic_tokens;

pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let literals = parsed_iso_literals_in_file(db, path).as_ref()?;
    let page_content = db.disk_file(path)?.contents.reference();
    lsp_semantic_tokens(
        page_content,
        literals.iter().map(|(extraction, parsed)| {
            (extraction.iso_literal_start_index, parsed.tokens.as_slice())
        }),
    )
    .wrap_some()
}
```

`isograph_lsp` already depends on `isograph_compiler`. This slice adds `common_lang_types` (`RelativePathToSourceFile`). It does not add `pico`. Dev-dependencies: `intern` (`.intern()`) and `isograph_extract_typescript` (`TypeScriptHostLanguage`).

```toml
# from crates/isograph_lsp/Cargo.toml
[package]
name = "isograph_lsp"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
common_lang_types = { path = "../common_lang_types" }
isograph_compiler = { path = "../isograph_compiler" }
isograph_parser = { path = "../isograph_parser" }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
span = { path = "../span" }

[dev-dependencies]
intern = { path = "../../relay-crates/intern" }
isograph_extract_typescript = { path = "../isograph_extract_typescript" }

[lints]
workspace = true
```

```rust
// from crates/isograph_lsp/src/lib.rs
mod file_semantic_tokens;
mod semantic_tokens;

pub use file_semantic_tokens::lsp_semantic_tokens_for_file;
pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};
```

This function is not a memo. Every call re-encodes. Parsed list `Some` means this path has a `DiskFile`; `disk_file` is `untracked` the same way `literal_id_at_location` looks up a path it already resolved. A miss is `None`.

The adapter later: `semanticTokens/full` for a URI maps to this path, then this function. Not this doc. Encoded-token reuse after a prepend is semantic-tokens-line-offset.md.

## Tests

Compiler tests in `isograph_extract_typescript` `memo_tests` (needs `TypeScriptHostLanguage` and interned files). Intern with the existing `intern_file` / `intern_path`. Path is `intern_path("src/a.ts")`. Add `parsed_iso_literals_in_file` to that module’s `isograph_compiler` import.

- No `DiskFile`: `parsed_iso_literals_in_file` is `None`.
- File with no `iso`: `Some` of empty vec.
- `iso(\`entrypoint Query.HomeRoute\`)`. One pair. The extraction Eq-equals extract `[0]`. The tree has empty parse errors, item variant `IsoLiteralItem::Entrypoint(_)`. The first parse token is `Keyword` at a relative `Span` covering `entrypoint` (`0..10` if the interior has no leading whitespace). `Query` is `Type`. The `.` is `Period`. `HomeRoute` is `FieldName`.
- `iso(\`entrypoint\`)`. Parse errors are non-empty. The first parse token is still `Keyword`.
- `iso(\`\`)`. Extract's regex requires a non-empty interior (`[^`]+`). `Some` of empty vec, same as a file with no `iso`.
- `iso(\`\n\`)`. Extract len 1. Parse errors contain `EmptyLiteral`. Parse tokens are empty (leftover `LineBreak` is not a token).
- `iso(\` \`)`. Spaces are skipped by the tokenizer, not leftover. Extract len 1. Parse errors contain `EmptyLiteral`. Parse tokens are empty.
- Two literals in one file. Two pairs. Each extraction Eq-equals the corresponding extract item.
- Multiline: intern `iso(\`\nfield User.Avatar {\n  name\n}\n\`)`. Parse has `Keyword` on `field` and `FieldName` on `name`. `name`'s relative `location.start` is greater than `field`'s.
- Prefixing the file with `const x = 1;\n` (second `intern_file` of the same path): the vec is `!=`. `iso_literal_start_index` is `IsoLiteralStartIndex` of the old start plus that prefix's byte length. The tree Eq-equals the pre-prefix tree. `parsed_iso_literal` of that text Eq-equals the pre-prefix tree. isograph `memoized_parse_iso_literal` takes `text_source` and comments that moving the literal breaks memoization because of that param. i2 `parsed_iso_literal` is keyed on `iso_literal_text` only. Encoded-token reuse after a prepend is semantic-tokens-line-offset.md.
- Prefixing with `const x = "😀";\n`. `iso_literal_start_index` is a byte offset (`contents.find("entrypoint")`).
- Appending `"\nconst y = 1;\n"` after the same one-literal file: the vec Eq-equals the pre-append vec. Extract's `IsoLiteralExtraction` Eq-equals (same text, same `iso_literal_start_index`, same context).
- Context-only: intern `export const Home = iso(\`entrypoint Query.HomeRoute\`)`, then `export const Page = iso(\`entrypoint Query.HomeRoute\`)` (`Home` and `Page` are the same length, so `iso_literal_start_index` is unchanged). Extract is `!=` (`const_export_name`). The vec is `!=`. `iso_literal_start_index` Eq-equals. The tree Eq-equals.
- Intern the one-literal file, the memo is `Some`, `remove_disk_file`, it is `None`.

LSP tests in `crates/isograph_lsp` `file_semantic_tokens.rs`. Offset arithmetic (prefix, two interiors, emoji) is lsp-semantic-tokens-offset.md. These tests intern a `DiskFile` and call `lsp_semantic_tokens_for_file`.

- Intern the one-literal file. `lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>` is `Some`. The first encoded token has `delta_line` 0, `token_type` the legend index of keyword (`15`), `length` 10, and `delta_start` the UTF-16 column of `entrypoint` on that line (the UTF-16 length of `export const Home = iso(\``).
- Two `iso` interiors interned in one file. `Some`. Keyword tokens are both interiors’ `entrypoint`, in extract order.
- Empty file (present, no iso): `Some` of empty vec.
- Intern `iso(\` \`)`: `Some` of empty vec.
- Intern then `remove_disk_file`: `None`.
- No `DiskFile`: `None`.

`isograph_lsp` tests intern with `insert_disk_file` and `"src/a.ts".intern().to()`, same construction as `memo_tests` `intern_path`. They depend on `isograph_extract_typescript` as a dev-dependency for `TypeScriptHostLanguage`, and `intern` for `.intern()`.

`expect` names the fixture the test interned.

## Call sites

- e2e-semantic-tokens.md: `isograph semantic-tokens` -> `lsp_semantic_tokens_for_file`.
- LSP adapter `semanticTokens/full` -> `lsp_semantic_tokens_for_file`. Later document-scoped methods (`documentSymbol`, `foldingRange`, `textDocument/formatting`) read `parsed_iso_literals_in_file`.
- Tests as above.
