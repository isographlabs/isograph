# Semantic tokens for the iso literals in a DiskFile

Requires extract-iso-literals-from-file.md, memoized-parse-iso-literal.md, lsp-semantic-token-encoding.md (landed), and `docs-website/docs/design-docs/pico.md`. Extract finds the literals. Parse records `ParsedIsoLiteral.tokens` with spans relative to the literal text. This file offsets those tokens to file coordinates, concatenates them in extract order, and encodes them with `lsp_semantic_tokens`. File-absolute spans do not live on the parse memo (pico.md).

Origin of the pipeline: isograph `crates/isograph_lsp/src/semantic_tokens.rs` `get_semantic_tokens` / `concatenate_and_absolutize_relative_tokens`. Origin of encoding: landed `lsp_semantic_tokens`. Delta: pico memos over `DiskFile` instead of `Uri` + LSP state; no `TextSource`; no multiline split here (`lsp_semantic_tokens` already splits); `with_offset` on each relative span.

The LSP adapter (event-model.md, not written) will call this on `semanticTokens/full`. This slice does not start the adapter. Tests intern a `DiskFile` and assert tokens.

One shippable change, two functions: file-absolute tokens in the compiler, encoding in `isograph_lsp`.

## What the user does

No editor highlighting until the adapter. Tests intern

```
export const Home = iso(`entrypoint Query.HomeRoute`)
```

and assert that `entrypoint` is `Keyword` at its file offset, then that `lsp_semantic_tokens` encodes that vec against the file text.

## Types

Most important first.

```text
iso_literal_semantic_tokens_in_file(path)
  -> parsed_iso_literals_in_file(path)

parsed_iso_literals_in_file(path)
  -> extract_iso_literals_from_file_content(path)
  + parsed_iso_literal(text) for each extraction
```

Origin of the concat memo: isograph `get_semantic_tokens` without the URI / `uri_is_project_file` checks. Delta: `Option` if there is no `DiskFile`; tokens from `ParsedIsoLiteral.tokens`; offset is `extraction.iso_literal_start_index as u32`. Parse is the text-keyed memo, not a cursor.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use isograph_parser::{IsographSemanticToken, ParsedIsoLiteral};
use pico_macros::memo;
use span::{WithSpan, WithSpanPostfix};

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
) -> Option<Vec<ParsedIsoLiteral>> {
    let extractions = extract_iso_literals_from_file_content::<THostLanguage>(db, path)?;
    extractions
        .iter()
        .map(|extraction| parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone())
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn iso_literal_semantic_tokens_in_file<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
) -> Option<Vec<WithSpan<IsographSemanticToken>>> {
    let extractions = extract_iso_literals_from_file_content::<THostLanguage>(db, path.clone())?;
    let parsed_literals = parsed_iso_literals_in_file::<THostLanguage>(db, path)?;
    let mut tokens = Vec::new();
    for (extraction, parsed) in extractions.iter().zip(parsed_literals.iter()) {
        let offset = extraction.iso_literal_start_index as u32;
        tokens.extend(parsed.tokens.iter().map(|token| {
            token.item.with_span(token.location.with_offset(offset))
        }));
    }
    tokens.wrap_some()
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso literals, or literals whose parse produced no tokens.

A parse with errors still has leftover tokens. Use them.

Tokens from different literals do not overlap: they sit inside disjoint backtick spans. `lsp_semantic_tokens` asserts that. JS between literals has no iso tokens.

`path.clone()` is the intern param of extract and of `parsed_iso_literals_in_file`. The inner parse intern is the literal text.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
use std::path::Path;

use isograph_compiler::{
    HostLanguage, IsographState, iso_literal_semantic_tokens_in_file,
};
use pico::Database;

use crate::lsp_semantic_tokens;

pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: &Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let tokens = iso_literal_semantic_tokens_in_file::<THostLanguage>(db, path.to_owned())?;
    let source_id = db.get_disk_file_map().tracked().0.get(path).copied()?;
    let page_content = db.get(source_id).contents.reference();
    lsp_semantic_tokens(tokens, page_content).wrap_some()
}
```

`isograph_lsp` depends on `isograph_compiler` and `pico`. `lib.rs` gains `mod file_semantic_tokens` and re-exports `lsp_semantic_tokens_for_file`.

The adapter later: `semanticTokens/full` for a URI maps to this path, then this function. Not this doc.

## Tests

Compiler tests in `isograph_extract_typescript` `memo_tests` (needs `TypeScriptHostLanguage` and interned files):

- No `DiskFile`: `iso_literal_semantic_tokens_in_file` is `None`.
- File with no `iso`: `Some` of empty vec.
- `iso(\`entrypoint Query.HomeRoute\`)`. Let `start` be the byte index of `entrypoint` in the file. The first token is `Keyword` at `Span` covering `entrypoint` in file coordinates (`start..start+"entrypoint".len()`). `Query` is `Type`. The `.` is `Period`. `HomeRoute` is `FieldName`.
- Two literals in one file. Tokens of the second start at or after the second extraction's `iso_literal_start_index`. Tokens are sorted by `location.start`.
- Prefixing the file with `const x = 1;\n` (second `Present` of the same path) moves the keyword span by that prefix length. The parse memo of the same iso text is reused; the concat memo is not. isograph `memoized_parse_iso_literal` takes `text_source` and comments that moving the literal breaks memoization because of that param. i2 `parsed_iso_literal` is keyed on `iso_literal_text` only. isograph `get_semantic_tokens` (issue 548) cannot reuse file-absolute encoded tokens after typing before the literal; that is this concat memo.
- Appending `"\nconst y = 1;\n"` after the same one-literal file: keyword span is unchanged. Extract's `IsoLiteralExtraction` Eq-equals (same text, same `iso_literal_start_index`, same context). pico re-invokes extract, backdates it, and does not re-invoke concat or parse.

Count reuse with test-only memos in `isograph_extract_typescript` `memo_tests`. They are not production. pico's own tests increment an `AtomicUsize` in the memo body.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    static PARSE_BODY: AtomicUsize = AtomicUsize::new(0);
    static CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);

    #[memo]
    fn counted_parse(db: &IsographState, iso_literal_text: String) {
        PARSE_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = parsed_iso_literal(db, iso_literal_text);
    }

    #[memo]
    fn counted_concat(db: &IsographState, path: PathBuf) {
        CONCAT_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = iso_literal_semantic_tokens_in_file::<TypeScriptHostLanguage>(db, path);
    }
```

Reset both atomics to 0 at the start of each of these two tests. Intern the one-literal file. Call `counted_parse` with that literal text and `counted_concat` with that path. Both counters are 1.

Append test: second `Present` with the suffix. Call both again. Both counters stay 1.

Prefix test: second `Present` with the prefix (a fresh intern of the original, or the append test's sibling: do not share a `db` across these two tests). Call both again. `PARSE_BODY` stays 1. `CONCAT_BODY` is 2.

LSP tests in `crates/isograph_lsp` `file_semantic_tokens.rs`:

- Intern the one-literal file. `lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>` is `Some`. The first encoded token has `delta_line` 0 and `token_type` the legend index of keyword. `length` is the UTF-16 length of `entrypoint`.
- Empty file (present, no iso): `Some` of empty vec.

`isograph_lsp` tests intern a `DiskFile` with `db.set` plus the tracked map, same as `handle`. They depend on `isograph_extract_typescript` as a dev-dependency for `TypeScriptHostLanguage`.

`expect` names the fixture the test interned.

## Call sites

- LSP adapter `semanticTokens/full` -> `lsp_semantic_tokens_for_file`.
- Tests as above.
