# Encode relative tokens, then offset to the file

Requires file-semantic-tokens.md. That slice offsets each `WithSpan` to file coordinates and encodes the concat against the file on every call. This slice encodes each literal against its own text (relative deltas), then a path-keyed memo translates those streams into document deltas using the line and column of each `iso_literal_start_index`.

Prepend JS before a literal: relative encode of that text Eq-equals; the path memo's encoded tokens have a new `delta_line`. Append after the last literal: locations Eq-equals, relative encodings Eq-equals, the path memo's encoded vec Eq-equals. file-semantic-tokens.md's concat of file-absolute `WithSpan` stays for compiler assertions of file coordinates. Encoding no longer reads it.

Origin of relative-then-absolute: isograph issue 548 (`get_semantic_tokens` cannot reuse encoded positions after typing before the literal; the suggested fix is relative LSP tokens plus an offset at send time). Origin of encoding: landed `lsp_semantic_tokens`. Origin of start indices: `locations_of_iso_literals_in_file`. Delta: `lsp_semantic_tokens` on the literal text; path-keyed stitch; no zero-length placeholder token.

One shippable change: a text-keyed encode memo and `lsp_semantic_tokens_for_file` becomes a `#[memo]` on `path`.

## What the user does

No editor highlighting until the adapter. Tests intern

```
export const Home = iso(`entrypoint Query.HomeRoute`)
```

Relative encode of `entrypoint Query.HomeRoute` has first token `delta_line` 0, `delta_start` 0, `length` 11, keyword. `lsp_semantic_tokens_for_file` of that path has first token `delta_line` 0, `delta_start` the UTF-16 column of `entrypoint` on the line (`export const Home = iso(\``), `length` 11, keyword. Prefix `"const x = 1;\n"`: relative encode of that text Eq-equals; first file token `delta_line` is 1, `delta_start` is unchanged.

## Types

```text
encoded_iso_literal_semantic_tokens(text)
  -> parsed_iso_literal(text)
  -> lsp_semantic_tokens(tokens, text)

lsp_semantic_tokens_for_file(path)
  -> THostLanguage::extract_iso_literals(path)
  + encoded_iso_literal_semantic_tokens(text) for each extraction
  + locations_of_iso_literals_in_file(path)
  + DiskFile contents
```

The file memo is keyed on `path`. It reads contents to turn each start index into a line and column. It does not go through `LineChar` as a public argument.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
use std::path::{Path, PathBuf};

use isograph_compiler::{
    HostLanguage, IsographState, locations_of_iso_literals_in_file, parsed_iso_literal,
};
use pico::Database;
use pico_macros::memo;
use prelude::Postfix;

use crate::lsp_semantic_tokens;

#[memo]
pub fn encoded_iso_literal_semantic_tokens<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    iso_literal_text: String,
) -> Vec<lsp_types::SemanticToken> {
    let parsed = parsed_iso_literal(db, iso_literal_text.clone());
    lsp_semantic_tokens(parsed.tokens.reference(), iso_literal_text.as_str())
}

#[memo]
pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let extractions = THostLanguage::extract_iso_literals(db, path.clone()).as_ref()?;
    let locations = locations_of_iso_literals_in_file(db, path.clone()).as_ref()?;
    let source_id = db.get_disk_file_map().untracked().0.get(&path).copied()?;
    let page_content = db.get(source_id).contents.reference();
    let mut file_line = 0u32;
    let mut file_col = 0u32;
    let mut out = Vec::new();
    for (extraction, start_index) in extractions.iter().zip(locations.iter()) {
        let (start_line, start_col) = line_and_column(page_content, *start_index);
        let relative = encoded_iso_literal_semantic_tokens(
            db,
            extraction.iso_literal_text.clone(),
        );
        offset_relative_tokens(
            relative,
            start_line,
            start_col,
            &mut file_line,
            &mut file_col,
            |token| out.push(token),
        );
    }
    out.wrap_some()
}
```

`line_and_column(text, byte_index)` is the `(line, column)` of that byte: `line` is a 0-based count of `\n` before it, `column` is UTF-16 of `text[line_start..byte_index]` where `line_start` is the index after the last `\n` (0 if none). Relative `delta_start` is already UTF-16. Adding them is the same unit. A non-ASCII prefix on the same line as the literal is a test of this conversion. A non-ASCII prefix on a previous line (`"😀";\n` plus the fixture) only bumps `delta_line`.

`offset_relative_tokens` decodes the relative stream to `(rel_line, rel_col)` starts, adds `(start_line, start_col)` (`rel_line == 0` adds `start_col` to `rel_col`; otherwise the column is `rel_col`), and emits LSP deltas from the previous file cursor `(file_line, file_col)`. First token of the file: previous cursor is `(0, 0)`.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
fn offset_relative_tokens(
    relative: &[lsp_types::SemanticToken],
    start_line: u32,
    start_col: u32,
    file_line: &mut u32,
    file_col: &mut u32,
    mut emit: impl FnMut(lsp_types::SemanticToken),
) {
    let mut rel_line = 0u32;
    let mut rel_col = 0u32;
    for token in relative {
        if token.delta_line == 0 {
            rel_col += token.delta_start;
        } else {
            rel_line += token.delta_line;
            rel_col = token.delta_start;
        }
        let abs_line = start_line + rel_line;
        let abs_col = if rel_line == 0 {
            start_col + rel_col
        } else {
            rel_col
        };
        let delta_line = abs_line - *file_line;
        let delta_start = if delta_line == 0 {
            abs_col - *file_col
        } else {
            abs_col
        };
        emit(lsp_types::SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: token.token_modifiers_bitset,
        });
        *file_line = abs_line;
        *file_col = abs_col;
    }
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso tokens.

`zip` of extract and locations is the same index space as file-semantic-tokens.md.

This function is a memo. The map lookup is `untracked`: extract returned `Some`, so this path has a `DiskFile`. Same keyed-by-path lookup as `literal_id_at_location`. On miss, `tracked()` the map (pico tracked-field miss). Extract already did that miss path; concat `Some` in the current slice implied the file exists. Here extract is read first; a miss is `None` from extract.

`iso_literal_semantic_tokens_in_file` is unchanged. `lsp_semantic_tokens_for_file` no longer calls it.

## Tests

Relative encode (`encoded_iso_literal_semantic_tokens`) in `isograph_lsp` tests, intern not required (text intern only):

- `entrypoint Query.HomeRoute`: first token `delta_line` 0, `delta_start` 0, `length` 11, `token_type` 15.
- `entrypoint`: parse errors non-empty. First token still keyword, `delta_start` 0, `length` 11.

File memo, intern with `insert_disk_file`:

- One-literal fixture. First token `delta_line` 0, `delta_start` UTF-16 of `export const Home = iso(\``, `length` 11, `token_type` 15.
- Prefix `"const x = 1;\n"`: first token `delta_line` 1, `delta_start` equals the pre-prefix `delta_start`. Relative encode of the same iso text Eq-equals.
- Append `"\nconst y = 1;\n"`: encoded vec Eq-equals the pre-append vec. Relative encode Eq-equals.
- Context-only `Home` -> `Page` (same length): relative encode Eq-equals. File memo encoded vec Eq-equals.
- Two literals. Tokens of the second have `delta_line` / `delta_start` that place them at the second literal's `entrypoint` (or first token) in the file. Sorted in extract order.
- Multiline `iso(\`\nfield User.Avatar {\n  name\n}\n\`)`. Relative encode of that text: `name` has `delta_line` greater than 0. File memo: `name`'s `delta_line` is the file line of `name` minus the file line of the previous token.
- Emoji prefix on the previous line: `const x = "😀";\n` plus the one-literal fixture. First token `delta_line` 1, `delta_start` equals the unprefixed fixture's `delta_start`. Relative encode Eq-equals.
- Emoji prefix on the same line: `const x = "😀"; ` plus the one-literal fixture (no extra newline). First token `delta_line` 0, `delta_start` is UTF-16 of `const x = "😀"; export const Home = iso(\``. Relative encode Eq-equals.
- Empty file: `Some` of empty vec.
- No `DiskFile`: `None`.
- Intern then `remove_disk_file`: `None`.

`expect` names the fixture the test interned.

## Call sites

- `lsp_semantic_tokens_for_file` as today: e2e-semantic-tokens.md, later adapter `semanticTokens/full`.
- Tests as above.

e2e-semantic-tokens.md's prepend assertion (`delta_line` 1, `delta_start` unchanged) stays. That file's claim that concat is the 548 fix updates: relative encode is the intern that survives a prepend; the path memo supplies the line offset.

Amend `docs-website/docs/design-docs/pico.md` when this lands: syntax highlighting is the path-keyed encoded memo, not the `WithSpan` concat.
