# Semantic tokens for the iso literals in a DiskFile

Requires extract-iso-literals-from-file.md (landed), memoized-parse-iso-literal.md (landed), literal-id.md (landed), lsp-semantic-token-encoding.md (landed), and lsp-semantic-tokens-offset.md (landed). Extract finds the literals. `parsed_iso_literal` records `ParsedIsoLiteral.tokens` with spans relative to the literal text. `locations_of_iso_literals_in_file` is those literals’ `IsoLiteralStartIndex` values in extract order. `lsp_semantic_tokens_for_file` passes the file text and every `(*start_index, parsed.tokens.as_slice())` in extract order to one `lsp_semantic_tokens` call. It does not encode one extraction. File-absolute `WithSpan<IsographSemanticToken>` is not interned and not exported. Parse stays keyed on the literal text.

File-level highlighting is keyed on `path`. It does not go through `LineChar`. Cursor APIs convert `path` and `LineChar` to `LiteralId`. This file’s parse intern is `parsed_iso_literal(text)` via `parsed_iso_literals_in_file`.

Origin of the pipeline: isograph `get_semantic_tokens`. Origin of encoding: landed `lsp_semantic_tokens`. Delta: pico memos over `DiskFile`; relative tokens stay relative; one encoder call for the file; no concat intern.

The LSP adapter (event-model.md, not written) will call this on `semanticTokens/full`. This slice does not start the adapter. `OpenFile` is not implemented yet; tests intern a `DiskFile` and assert encoded tokens. Encoding is not a memo. Memoizing encoded tokens is semantic-tokens-line-offset.md.

One shippable change: two compiler memos and `lsp_semantic_tokens_for_file`. pico.md already has this graph and the un-memoized `lsp_semantic_tokens_for_file`. Amend that file only if the graph drifted.

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
  + locations_of_iso_literals_in_file(path)
  + DiskFile contents
  -> lsp_semantic_tokens(page_content, (offset, tokens) per literal)

parsed_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
  + parsed_iso_literal(text) for each extraction

locations_of_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
```

`parsed_iso_literals_in_file` is the trees. It does not store `iso_literal_start_index`. A prepend leaves that vec `==` (same texts, relative spans). `locations_of_iso_literals_in_file` is those start indices in extract order. A prepend makes it `!=`. `lsp_semantic_tokens_for_file` reads both and the file text. If it only read the parsed vec, encoded `delta_line` / `delta_start` would not move.

`locations_of_iso_literals_in_file` does not store the literal text or `LiteralContext`. Text is the parse intern. Context is host embedding, not highlighting. Path intern params are `RelativePathToSourceFile`. Locations is `Vec<IsoLiteralStartIndex>`.

pico lookup of an `Option` memo is `&Option<T>`. Callers write `.as_ref()?`.

The two memos go in `crates/isograph_compiler/src/iso_literals.rs`, next to `parsed_iso_literal`. Add `IsoLiteralStartIndex` to the existing `host_language` import. `parsed_iso_literal` is already in this module.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<ParsedIsoLiteral>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone())
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn locations_of_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralStartIndex>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| extraction.iso_literal_start_index)
        .collect::<Vec<_>>()
        .wrap_some()
}

```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso literals.

A parse with errors still has leftover tokens. Use them.

Tokens from different literals do not overlap: they sit inside disjoint backtick spans. `lsp_semantic_tokens` asserts that. JS between literals has no iso tokens.

`path` is `RelativePathToSourceFile` (interned, `Copy`). It is the intern param of `parsed_iso_literals_in_file` and of `locations_of_iso_literals_in_file`. The inner parse intern is the literal text. `parsed_iso_literal` already exists.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, LiteralId, iso_literal_extraction, literal_id_at_location,
    locations_of_iso_literals_in_file, parsed_iso_literal, parsed_iso_literals_in_file,
};
```

The existing `pub use` already names `LineChar`, `LiteralId`, `iso_literal_extraction`, `literal_id_at_location`, and `parsed_iso_literal`. Add the two new names.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{
    HostLanguage, IsographState, locations_of_iso_literals_in_file, parsed_iso_literals_in_file,
};
use pico::Database;
use prelude::Postfix;

use crate::lsp_semantic_tokens;

pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let parsed_literals = parsed_iso_literals_in_file(db, path).as_ref()?;
    let locations = locations_of_iso_literals_in_file(db, path).as_ref()?;
    let source_id = db.get_disk_file_map().untracked().0.get(&path).copied()?;
    let page_content = db.get(source_id).contents.reference();
    lsp_semantic_tokens(
        page_content,
        parsed_literals.iter().zip(locations.iter()).map(|(parsed, start_index)| {
            (*start_index, parsed.tokens.as_slice())
        }),
    )
    .wrap_some()
}
```

`isograph_lsp` already depends on `isograph_compiler`. This slice adds `pico` (`Database`) and `common_lang_types` (`RelativePathToSourceFile`). Dev-dependencies: `intern` (`.intern()`) and `isograph_extract_typescript` (`TypeScriptHostLanguage`).

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
pico = { path = "../pico" }
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

This function is not a memo. Every call re-encodes. Parsed list `Some` means this path has a `DiskFile`; the map lookup is `untracked` the same way `literal_id_at_location` looks up a path it already resolved. A miss is `None`.

The adapter later: `semanticTokens/full` for a URI maps to this path, then this function. Not this doc. Encoded-token reuse after a prepend is semantic-tokens-line-offset.md.

## Tests

Compiler tests in `isograph_extract_typescript` `memo_tests` (needs `TypeScriptHostLanguage` and interned files). Intern with the existing `intern_file` / `intern_path`. Path is `intern_path("src/a.ts")`. Add `locations_of_iso_literals_in_file` and `parsed_iso_literals_in_file` to that module’s `isograph_compiler` import.

- No `DiskFile`: `parsed_iso_literals_in_file` and `locations_of_iso_literals_in_file` are `None`.
- File with no `iso`: both are `Some` of empty vec.
- `iso(\`entrypoint Query.HomeRoute\`)`. Let `start` be the byte index of `entrypoint` in the file. `locations_of_iso_literals_in_file` is one element, the extraction's `iso_literal_start_index`. `parsed_iso_literals_in_file` is one tree, empty parse errors, item variant `IsoLiteralItem::Entrypoint(_)`. The first parse token is `Keyword` at a relative `Span` covering `entrypoint` (`0..10` if the interior has no leading whitespace). `Query` is `Type`. The `.` is `Period`. `HomeRoute` is `FieldName`.
- `iso(\`entrypoint\`)`. Parse errors are non-empty. The first parse token is still `Keyword`.
- `iso(\`\`)`. Extract's regex requires a non-empty interior (`[^`]+`). Both file memos are `Some` of empty vec, same as a file with no `iso`.
- `iso(\`\n\`)`. Extract len 1. Parse errors contain `EmptyLiteral`. Parse tokens are empty (leftover `LineBreak` is not a token).
- `iso(\` \`)`. Spaces are skipped by the tokenizer, not leftover. Extract len 1. Parse errors contain `EmptyLiteral`. Parse tokens are empty.
- Two literals in one file. `locations_of_iso_literals_in_file` has two start indices, matching the two extractions.
- Multiline: intern `iso(\`\nfield User.Avatar {\n  name\n}\n\`)`. Parse has `Keyword` on `field` and `FieldName` on `name`. `name`'s relative `location.start` is greater than `field`'s.
- Prefixing the file with `const x = 1;\n` (second `intern_file` of the same path): `parsed_iso_literals_in_file` Eq-equals the pre-prefix vec. `locations_of_iso_literals_in_file[0]` is `IsoLiteralStartIndex` of the old start plus that prefix's byte length. isograph `memoized_parse_iso_literal` takes `text_source` and comments that moving the literal breaks memoization because of that param. i2 `parsed_iso_literal` is keyed on `iso_literal_text` only. Encoded-token reuse after a prepend is semantic-tokens-line-offset.md.
- Prefixing with `const x = "😀";\n`. `iso_literal_start_index` is a byte offset (`contents.find("entrypoint")`).
- Appending `"\nconst y = 1;\n"` after the same one-literal file: `locations_of_iso_literals_in_file` Eq-equals the pre-append vec. Extract's `IsoLiteralExtraction` Eq-equals (same text, same `iso_literal_start_index`, same context).
- Context-only: intern `export const Home = iso(\`entrypoint Query.HomeRoute\`)`, then `export const Page = iso(\`entrypoint Query.HomeRoute\`)` (`Home` and `Page` are the same length, so `iso_literal_start_index` is unchanged). Extract is `!=` (`const_export_name`). `locations_of_iso_literals_in_file` Eq-equals. `parsed_iso_literals_in_file` Eq-equals. That is why locations is a list of start indices and not extract: host context is not highlighting.
- Intern the one-literal file, both memos are `Some`, `remove_disk_file`, both are `None`.

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
- LSP adapter `semanticTokens/full` -> `lsp_semantic_tokens_for_file`.
- Tests as above.
