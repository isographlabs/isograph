# Semantic tokens for the iso literals in a DiskFile

Requires extract-iso-literals-from-file.md (landed), memoized-parse-iso-literal.md (landed), and lsp-semantic-token-encoding.md (landed). Extract finds the literals. `parsed_iso_literal` records `ParsedIsoLiteral.tokens` with spans relative to the literal text. This file offsets those tokens to file coordinates, concatenates them in extract order, and encodes them with `lsp_semantic_tokens`. File-absolute spans do not live on the parse memo: `parsed_iso_literal` is keyed on the literal text, and relative spans keep that result `==` after a prepend.

Origin of the pipeline: isograph `crates/isograph_lsp/src/semantic_tokens.rs` `get_semantic_tokens` / `concatenate_and_absolutize_relative_tokens`. Origin of encoding: landed `lsp_semantic_tokens`. Origin of the offset map: that concat, and the encoding tests' `rebased`. Delta: pico memos over `DiskFile` instead of `Uri` + LSP state; no `TextSource`; no multiline split here (`lsp_semantic_tokens` already splits); `with_offset` on each relative span.

The LSP adapter (event-model.md, not written) will call this on `semanticTokens/full`. This slice does not start the adapter. Tests intern a `DiskFile` and assert tokens.

One shippable change: two compiler memos and `lsp_semantic_tokens_for_file`.

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
  -> THostLanguage::extract_iso_literals(path)
  + parsed_iso_literals_in_file(path)

parsed_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
  + parsed_iso_literal(text) for each extraction
```

Origin of the concat memo: isograph `get_semantic_tokens` without the URI / `uri_is_project_file` checks. Delta: `Option` if there is no `DiskFile`; tokens from `ParsedIsoLiteral.tokens`; offset is `extraction.iso_literal_start_index as u32`. Parse is the text-keyed memo, not a cursor.

pico lookup of an `Option` memo is `&Option<T>`. Callers write `.as_ref()?`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use isograph_parser::{IsographSemanticToken, ParsedIsoLiteral, parse_iso_literal};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::IsographState;
use crate::host_language::{HostLanguage, IsoLiteralExtraction};

#[memo]
fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
) -> Option<Vec<ParsedIsoLiteral>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone())
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn iso_literal_semantic_tokens_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
) -> Option<Vec<WithSpan<IsographSemanticToken>>> {
    let extractions = THostLanguage::extract_iso_literals(db, path.clone()).as_ref()?;
    let parsed_literals = parsed_iso_literals_in_file(db, path).as_ref()?;
    extractions
        .iter()
        .zip(parsed_literals.iter())
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index as u32;
            parsed.tokens.iter().map(move |token| {
                token.item.with_span(token.location.with_offset(offset))
            })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso literals, or literals whose parse produced no tokens.

A parse with errors still has leftover tokens. Use them.

Tokens from different literals do not overlap: they sit inside disjoint backtick spans. `lsp_semantic_tokens` asserts that. JS between literals has no iso tokens.

`iso_literal_semantic_tokens_in_file` reads extract for `iso_literal_start_index`. `parsed_iso_literals_in_file` is `==` after a prepend (relative spans), so concat would not re-invoke if it only read that memo.

`path.clone()` is the intern param of extract and of `parsed_iso_literals_in_file`. The inner parse intern is the literal text. `parsed_iso_literal` already exists.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, iso_literal_extraction, iso_literal_semantic_tokens_in_file,
    iso_literal_text_at_location, parsed_iso_literal, parsed_iso_literal_at_location,
};
```

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
use std::path::Path;

use isograph_compiler::{
    HostLanguage, IsographState, iso_literal_semantic_tokens_in_file,
};
use pico::Database;
use prelude::Postfix;

use crate::lsp_semantic_tokens;

pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: &Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let tokens = iso_literal_semantic_tokens_in_file(db, path.to_owned()).as_ref()?;
    let source_id = db.get_disk_file_map().untracked().0.get(path).copied()?;
    let page_content = db.get(source_id).contents.reference();
    lsp_semantic_tokens(tokens, page_content).wrap_some()
}
```

`isograph_lsp` depends on `isograph_compiler` and `pico`. `lib.rs` gains `mod file_semantic_tokens` and re-exports `lsp_semantic_tokens_for_file`.

The map lookup is `untracked`: concat returned `Some`, so this path has a `DiskFile`. Same keyed-by-path lookup as `iso_literal_extraction`. `lsp_semantic_tokens` takes `&[WithSpan<IsographSemanticToken>]`; pico lookup of concat is `&Vec<_>`.

The adapter later: `semanticTokens/full` for a URI maps to this path, then this function. Not this doc.

## Tests

Compiler tests in `isograph_extract_typescript` `memo_tests` (needs `TypeScriptHostLanguage` and interned files). Intern with `intern_file` (`insert_disk_file`).

- No `DiskFile`: `iso_literal_semantic_tokens_in_file` is `None`.
- File with no `iso`: `Some` of empty vec.
- `iso(\`entrypoint Query.HomeRoute\`)`. Let `start` be the byte index of `entrypoint` in the file. The first token is `Keyword` at `Span` covering `entrypoint` in file coordinates (`start..start+"entrypoint".len()`). `Query` is `Type`. The `.` is `Period`. `HomeRoute` is `FieldName`.
- `iso(\`entrypoint\`)`. Parse errors are non-empty. The first token is still `Keyword` at the file offset of `entrypoint`.
- Two literals in one file. Tokens of the second start at or after the second extraction's `iso_literal_start_index`. Tokens are sorted by `location.start`.
- Prefixing the file with `const x = 1;\n` (second `intern_file` of the same path) moves the keyword span by that prefix length. `parsed_iso_literal` of the same iso text does not re-invoke; concat does. isograph `memoized_parse_iso_literal` takes `text_source` and comments that moving the literal breaks memoization because of that param. i2 `parsed_iso_literal` is keyed on `iso_literal_text` only. isograph `get_semantic_tokens` (issue 548) cannot reuse file-absolute encoded tokens after typing before the literal; that is this concat memo.
- Appending `"\nconst y = 1;\n"` after the same one-literal file: keyword span is unchanged. Extract's `IsoLiteralExtraction` Eq-equals (same text, same `iso_literal_start_index`, same context). pico re-invokes extract, backdates it, and does not re-invoke concat or parse.

Count reuse with test-only memos in `isograph_extract_typescript` `memo_tests`. They are not production. pico's own tests increment an `AtomicUsize` in the memo body. Each of these two tests has its own atomics and counted memos.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    static APPEND_PARSE_BODY: AtomicUsize = AtomicUsize::new(0);
    static APPEND_CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_PARSE_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);

    #[memo]
    fn counted_parse_append(db: &IsographState<TypeScriptHostLanguage>, iso_literal_text: String) {
        APPEND_PARSE_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = parsed_iso_literal(db, iso_literal_text);
    }

    #[memo]
    fn counted_concat_append(db: &IsographState<TypeScriptHostLanguage>, path: PathBuf) {
        APPEND_CONCAT_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = iso_literal_semantic_tokens_in_file(db, path);
    }

    #[memo]
    fn counted_parse_prefix(db: &IsographState<TypeScriptHostLanguage>, iso_literal_text: String) {
        PREFIX_PARSE_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = parsed_iso_literal(db, iso_literal_text);
    }

    #[memo]
    fn counted_concat_prefix(db: &IsographState<TypeScriptHostLanguage>, path: PathBuf) {
        PREFIX_CONCAT_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = iso_literal_semantic_tokens_in_file(db, path);
    }
```

Append test: intern the one-literal file. Call `counted_parse_append` with that literal text and `counted_concat_append` with that path. Both counters are 1. Second `intern_file` with the suffix. Call both again. Both counters stay 1.

Prefix test: intern the one-literal file in a fresh `db`. Call `counted_parse_prefix` and `counted_concat_prefix`. Both counters are 1. Second `intern_file` with the prefix. Call both again. `PREFIX_PARSE_BODY` stays 1. `PREFIX_CONCAT_BODY` is 2.

LSP tests in `crates/isograph_lsp` `file_semantic_tokens.rs`:

- Intern the one-literal file. `lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>` is `Some`. The first encoded token has `delta_line` 0 and `token_type` the legend index of keyword. `length` is the UTF-16 length of `entrypoint`.
- Empty file (present, no iso): `Some` of empty vec.
- No `DiskFile`: `None`.

`isograph_lsp` tests intern a `DiskFile` with `insert_disk_file`, same as `handle` and `memo_tests` `intern_file`. They depend on `isograph_extract_typescript` as a dev-dependency for `TypeScriptHostLanguage`.

`expect` names the fixture the test interned.

## Call sites

- LSP adapter `semanticTokens/full` -> `lsp_semantic_tokens_for_file`.
- Tests as above.
