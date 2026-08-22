# Semantic tokens for the iso literals in a DiskFile

Requires extract-iso-literals-from-file.md (landed), memoized-parse-iso-literal.md (landed), literal-id.md (landed), and lsp-semantic-token-encoding.md (landed). Extract finds the literals. `parsed_iso_literal` records `ParsedIsoLiteral.tokens` with spans relative to the literal text. This file offsets those tokens to file coordinates, concatenates them in extract order, and encodes them with `lsp_semantic_tokens`. File-absolute spans do not live on the parse memo: `parsed_iso_literal` is keyed on the literal text, and relative spans keep that result `==` after a prepend.

File-level tokens are keyed on `path`. They do not go through `LineChar`. Cursor APIs convert `path` and `LineChar` to `LiteralId` (the file plus the 0-based extract index). Extraction and parse hang off that id, so there is not a clone of the tree or the extraction per caret. This file's parse intern is `parsed_iso_literal(text)` via `parsed_iso_literals_in_file`.

Origin of the pipeline: isograph `crates/isograph_lsp/src/semantic_tokens.rs` `get_semantic_tokens` / `concatenate_and_absolutize_relative_tokens`. Origin of encoding: landed `lsp_semantic_tokens`. Origin of the offset map: that concat, and the encoding tests' `rebased`. Delta: pico memos over `DiskFile` instead of `Uri` + LSP state; no `TextSource`; no multiline split here (`lsp_semantic_tokens` already splits); `with_offset` on each relative span; start indices in a memo that is not the parse vec.

The LSP adapter (event-model.md, not written) will call this on `semanticTokens/full`. This slice does not start the adapter. `OpenFile` is not implemented yet; tests intern a `DiskFile` and assert tokens. Encoding is not a memo. Memoizing encoded tokens, and converting relative tokens to file-absolute with a line offset instead of `with_offset` on each span, is semantic-tokens-line-offset.md.

One shippable change: three compiler memos and `lsp_semantic_tokens_for_file`. This change amends `docs-website/docs/design-docs/pico.md`.

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
  + locations_of_iso_literals_in_file(path)

parsed_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
  + parsed_iso_literal(text) for each extraction

locations_of_iso_literals_in_file(path)
  -> THostLanguage::extract_iso_literals(path)
```

Origin of the concat memo: isograph `get_semantic_tokens` without the URI / `uri_is_project_file` checks. Delta: `Option` if there is no `DiskFile`; tokens from `ParsedIsoLiteral.tokens`; offset is `locations_of_iso_literals_in_file[i] as u32`, that value being extract's `iso_literal_start_index`. Parse is the text-keyed memo, not a cursor.

`parsed_iso_literals_in_file` is the trees. It does not store `iso_literal_start_index`. A prepend leaves that vec `==` (same texts, relative spans). `locations_of_iso_literals_in_file` is those start indices in extract order. A prepend makes it `!=`. Concat reads both. If it only read the parsed vec, it would not re-offset.

`locations_of_iso_literals_in_file` does not store the literal text or `LiteralContext`. Text is the parse intern. Context is host embedding, not highlighting.

pico lookup of an `Option` memo is `&Option<T>`. Callers write `.as_ref()?`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use isograph_parser::{IsographSemanticToken, ParsedIsoLiteral};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::IsographState;
use crate::host_language::HostLanguage;

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
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
pub fn locations_of_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
) -> Option<Vec<usize>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| extraction.iso_literal_start_index)
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn iso_literal_semantic_tokens_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
) -> Option<Vec<WithSpan<IsographSemanticToken>>> {
    let parsed_literals = parsed_iso_literals_in_file(db, path.clone()).as_ref()?;
    let locations = locations_of_iso_literals_in_file(db, path).as_ref()?;
    parsed_literals
        .iter()
        .zip(locations.iter())
        .flat_map(|(parsed, start_index)| {
            let offset = *start_index as u32;
            parsed.tokens.iter().map(move |token| {
                token.item.with_span(token.location.with_offset(offset))
            })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso literals, or (for concat) literals whose parse produced no tokens.

A parse with errors still has leftover tokens. Use them.

Tokens from different literals do not overlap: they sit inside disjoint backtick spans. `lsp_semantic_tokens` asserts that. JS between literals has no iso tokens.

`path.clone()` is the intern param of `parsed_iso_literals_in_file` and of `locations_of_iso_literals_in_file`. The inner parse intern is the literal text. `parsed_iso_literal` already exists. Concat does not call extract.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, LiteralId, iso_literal_extraction, iso_literal_semantic_tokens_in_file,
    literal_id_at_location, locations_of_iso_literals_in_file, parsed_iso_literal,
    parsed_iso_literals_in_file,
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

This function is not a memo. Every call re-encodes. Concat `Some` means this path has a `DiskFile`; the map lookup is `untracked` the same way `literal_id_at_location` looks up a path it already resolved. A miss is `None`. `lsp_semantic_tokens` takes `&[WithSpan<IsographSemanticToken>]`; pico lookup of concat is `&Vec<_>`.

The adapter later: `semanticTokens/full` for a URI maps to this path, then this function. Not this doc. Encoded-token reuse after a prepend is semantic-tokens-line-offset.md.

## Tests

Compiler tests in `isograph_extract_typescript` `memo_tests` (needs `TypeScriptHostLanguage` and interned files). Intern with `intern_file` (`insert_disk_file`).

- No `DiskFile`: `parsed_iso_literals_in_file`, `locations_of_iso_literals_in_file`, and `iso_literal_semantic_tokens_in_file` are `None`.
- File with no `iso`: all three are `Some` of empty vec.
- `iso(\`entrypoint Query.HomeRoute\`)`. Let `start` be the byte index of `entrypoint` in the file. `locations_of_iso_literals_in_file` is one element, the extraction's `iso_literal_start_index`. `parsed_iso_literals_in_file` is one tree, empty parse errors, item variant `IsoLiteralItem::Entrypoint(_)`. The first concat token is `Keyword` at `Span` covering `entrypoint` in file coordinates (`start..start+"entrypoint".len()`). `Query` is `Type`. The `.` is `Period`. `HomeRoute` is `FieldName`.
- `iso(\`entrypoint\`)`. Parse errors are non-empty. The first token is still `Keyword` at the file offset of `entrypoint`.
- `iso(\`\`)`. Extract's regex requires a non-empty interior (`[^`]+`). The three file memos are `Some` of empty vec, same as a file with no `iso`.
- `iso(\`\n\`)`. Extract len 1. Parse errors contain `EmptyLiteral`. Concat is `Some` of empty vec (leftover `LineBreak` is not a token).
- Two literals in one file. `locations_of_iso_literals_in_file` has two start indices, matching the two extractions. Tokens of the second start at or after the second start index. Tokens are sorted by `location.start`.
- Multiline: intern `iso(\`\nfield User.Avatar {\n  name\n}\n\`)`. Concat has `Keyword` at the file offset of `field` and `FieldName` at the file offset of `name`. `name`'s `location.start` is greater than `field`'s.
- Prefixing the file with `const x = 1;\n` (second `intern_file` of the same path): `parsed_iso_literals_in_file` Eq-equals the pre-prefix vec. `locations_of_iso_literals_in_file[0]` is the old start plus that prefix's byte length. The keyword span moves by that length. `parsed_iso_literal` of the same iso text does not re-invoke; concat does. isograph `memoized_parse_iso_literal` takes `text_source` and comments that moving the literal breaks memoization because of that param. i2 `parsed_iso_literal` is keyed on `iso_literal_text` only. File-absolute `WithSpan` tokens cannot be reused after typing before the literal; that is this concat memo. Encoded-token reuse is semantic-tokens-line-offset.md.
- Prefixing with `const x = "😀";\n`. `iso_literal_start_index` and the keyword span start are byte offsets (`contents.find("entrypoint")`). Encoding `delta_start` is UTF-16 of the prefix of that line up to `entrypoint`.
- Appending `"\nconst y = 1;\n"` after the same one-literal file: `locations_of_iso_literals_in_file` Eq-equals the pre-append vec. Keyword span is unchanged. Extract's `IsoLiteralExtraction` Eq-equals (same text, same `iso_literal_start_index`, same context). pico re-invokes extract, backdates it, and does not re-invoke locations, parsed-in-file, concat, or parse.
- Context-only: intern `export const Home = iso(\`entrypoint Query.HomeRoute\`)`, then `export const Page = iso(\`entrypoint Query.HomeRoute\`)` (`Home` and `Page` are the same length, so `iso_literal_start_index` is unchanged). Extract is `!=` (`const_export_name`). `locations_of_iso_literals_in_file` Eq-equals. `parsed_iso_literals_in_file` Eq-equals. Concat does not re-invoke. That is why locations is a list of start indices and not extract: host context is not highlighting.
- Intern the one-literal file, concat is `Some`, `remove_disk_file`, concat is `None`.

Count reuse with test-only memos in `isograph_extract_typescript` `memo_tests`. They are not production. pico's own tests increment an `AtomicUsize` in the memo body. Each of these two tests has its own atomics and counted memos.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    static APPEND_PARSE_BODY: AtomicUsize = AtomicUsize::new(0);
    static APPEND_CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);
    static APPEND_PARSED_IN_FILE_BODY: AtomicUsize = AtomicUsize::new(0);
    static APPEND_LOCATIONS_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_PARSE_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_PARSED_IN_FILE_BODY: AtomicUsize = AtomicUsize::new(0);
    static PREFIX_LOCATIONS_BODY: AtomicUsize = AtomicUsize::new(0);
    static CONTEXT_CONCAT_BODY: AtomicUsize = AtomicUsize::new(0);

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
    fn counted_parsed_in_file_append(
        db: &IsographState<TypeScriptHostLanguage>,
        path: PathBuf,
    ) {
        APPEND_PARSED_IN_FILE_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = parsed_iso_literals_in_file(db, path);
    }

    #[memo]
    fn counted_locations_append(db: &IsographState<TypeScriptHostLanguage>, path: PathBuf) {
        APPEND_LOCATIONS_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = locations_of_iso_literals_in_file(db, path);
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

    #[memo]
    fn counted_parsed_in_file_prefix(
        db: &IsographState<TypeScriptHostLanguage>,
        path: PathBuf,
    ) {
        PREFIX_PARSED_IN_FILE_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = parsed_iso_literals_in_file(db, path);
    }

    #[memo]
    fn counted_locations_prefix(db: &IsographState<TypeScriptHostLanguage>, path: PathBuf) {
        PREFIX_LOCATIONS_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = locations_of_iso_literals_in_file(db, path);
    }

    #[memo]
    fn counted_concat_context(db: &IsographState<TypeScriptHostLanguage>, path: PathBuf) {
        CONTEXT_CONCAT_BODY.fetch_add(1, Ordering::SeqCst);
        let _ = iso_literal_semantic_tokens_in_file(db, path);
    }
```

Append test: intern the one-literal file. Call the four append counted memos. All counters are 1. Second `intern_file` with the suffix. Call all four again. All four stay 1.

Prefix test: intern the one-literal file in a fresh `db`. Call the four prefix counted memos. All counters are 1. Second `intern_file` with the prefix. Call all four again. `PREFIX_PARSE_BODY` stays 1. `PREFIX_PARSED_IN_FILE_BODY` stays 1 (`parsed_iso_literals_in_file` re-invokes because extract `!=`, result `==`, backdates; the wrapper depends on that `time_updated`). `PREFIX_LOCATIONS_BODY` is 2. `PREFIX_CONCAT_BODY` is 2.

Context-only test: intern `export const Home = iso(\`entrypoint Query.HomeRoute\`)` in a fresh `db`. Call `counted_concat_context`. Counter is 1. Second `intern_file` with `Page` in place of `Home`. Call again. `CONTEXT_CONCAT_BODY` stays 1.

LSP tests in `crates/isograph_lsp` `file_semantic_tokens.rs`:

- Intern the one-literal file. `lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>` is `Some`. The first encoded token has `delta_line` 0, `token_type` the legend index of keyword (`15`), `length` the UTF-16 length of `entrypoint` (`11`), and `delta_start` the UTF-16 column of `entrypoint` on that line (the UTF-16 length of `export const Home = iso(\``).
- Multiline fixture as above. The encoded token for `name` has `delta_line` greater than 0.
- Emoji prefix on the previous line (`const x = "😀";\n` plus the fixture). First token `delta_line` 1, `length` 11, `delta_start` equals the unprefixed fixture.
- Emoji prefix on the same line (`const x = "😀"; ` plus the fixture). First token `delta_line` 0, `length` 11, `delta_start` is UTF-16 of that prefix plus `export const Home = iso(\``.
- Empty file (present, no iso): `Some` of empty vec.
- No `DiskFile`: `None`.

`isograph_lsp` tests intern a `DiskFile` with `insert_disk_file`, same as `handle` and `memo_tests` `intern_file`. They depend on `isograph_extract_typescript` as a dev-dependency for `TypeScriptHostLanguage`.

`expect` names the fixture the test interned.

## Call sites

- LSP adapter `semanticTokens/full` -> `lsp_semantic_tokens_for_file`.
- Tests as above.
