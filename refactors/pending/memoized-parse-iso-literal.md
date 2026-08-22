# Memoized parse of an extracted iso literal

Requires extract-iso-literals-from-file.md. Extract returns text, span, and context. This file parses one extraction.

Origin of the memo: isograph `memoized_parse_iso_literal`. Origin of the lookup-by-index function: isograph `parse_iso_literals_in_file_content` walking `extract_iso_literals_from_file_content` by vec position. Delta: parse is keyed on the literal text only, not on `TextSource` or the file path (isograph's TODO: passing `text_source` breaks memoization when the literal moves); i2 `parse_iso_literal` already takes `&str` only; host embedding errors that need the parse tree run after parse, in `host_errors_for_extraction`.

`parsed_iso_literal_in_file` takes the file and a 0-based index into that file's extract vec. That index is not the pico cache key. The cache key is `iso_literal_text`. Two files with the same literal text share a parse. Editing JavaScript around a literal re-extracts and reuses the parse.

Two shippable changes: the text-keyed parse memo, then the file+index accessor and host errors.

## What the user does

No user-facing change. Tests intern a `DiskFile`, extract, parse index 0, and assert the tree (or parse errors). A field without an export reports `MissingExport` from `host_errors_for_extraction`.

## Change 1: `parsed_iso_literal`

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use isograph_parser::{ParsedIsoLiteral, parse_iso_literal};

#[memo]
pub fn parsed_iso_literal(db: &IsographState, iso_literal_text: String) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}
```

`db` is the pico database. The body does not read sources. Changing any `DiskFile` does not invalidate this memo unless the interned `iso_literal_text` param is different.

pico lookup returns `&ParsedIsoLiteral`. `ParsedIsoLiteral` does not need `Clone`.

`isograph_compiler` already depends on `isograph_parser`.

### Tests

In `crates/isograph_extract_typescript` `memo_tests`, or in `crates/isograph_compiler` if these tests do not need a host (they do not). Put them in `crates/isograph_compiler/src/iso_literals.rs` under `#[cfg(test)]`.

- `parsed_iso_literal(&db, "entrypoint Query.HomeRoute".to_owned())` has `errors` empty and `item` `Some` whose item is `IsoLiteralItem::Entrypoint`.
- `"entrypoint"` has a parse error (incomplete). `item` may still be `Some` (resilient parse). `errors` is not empty.
- `""` is `AstError::EmptyLiteral` as today.
- Calling twice with the same text returns a pointer to the same stored value (address equality is not required; asserting the tree twice is enough). A third call after interning an unrelated `DiskFile` still matches.

Do not add a production function only the tests call.

## Change 2: parse at an index, and host embedding errors

```rust
// from crates/isograph_compiler/src/iso_literals.rs
pub fn parsed_iso_literal_in_file<'a, THostLanguage: HostLanguage>(
    db: &'a IsographState,
    path: &Path,
    index: usize,
) -> Option<&'a ParsedIsoLiteral> {
    let extractions = extract_iso_literals_from_file_content::<THostLanguage>(db, path.to_owned())?;
    let extraction = extractions.get(index)?;
    parsed_iso_literal(db, extraction.iso_literal_text.clone()).wrap_some()
}
```

Not a memo. The extract memo and the text-keyed parse memo are the caches. `path.to_owned()` is the intern param of extract. `iso_literal_text.clone()` is the intern param of parse.

`None` is no `DiskFile`, or `index` past the last extraction.

isograph walks every extraction and parses all of them (`parse_iso_literals_in_file_content`). That helper, if a later doc needs it:

```rust
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: &Path,
) -> Option<Vec<&ParsedIsoLiteral>> {
    let extractions = extract_iso_literals_from_file_content::<THostLanguage>(db, path.to_owned())?;
    extractions
        .iter()
        .map(|extraction| parsed_iso_literal(db, extraction.iso_literal_text.clone()))
        .collect::<Vec<_>>()
        .wrap_some()
}
```

Do not add `parsed_iso_literals_in_file` unless a caller in this slice needs the whole vec. file-semantic-tokens.md is that caller; put the helper in that doc if it wants it, or have it loop `index`. This doc ships `parsed_iso_literal_in_file` (one index).

### Host errors after parse

Origin: isograph `process_iso_literal_extraction` (paren check, then parse, then associated-function check for fields). Origin of the error types: extract-iso-literals.md `TypeScriptHostError`. Delta: takes an extraction plus `&ParsedIsoLiteral` instead of parsing inside extract.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use isograph_compiler::{IsoLiteralError, WithErrors};
use isograph_parser::{IsoLiteralItem, ParsedIsoLiteral};
use span::WithSpanPostfix;

pub fn host_errors_for_extraction(
    extraction: &isograph_compiler::IsoLiteralExtraction<TypeScriptHostLanguage>,
    parsed: &ParsedIsoLiteral,
) -> Vec<span::WithSpan<IsoLiteralError<TypeScriptHostLanguage>>> {
    let span = extraction.span();
    let mut errors = Vec::new();
    if let IsoCall::TaggedTemplate = extraction.context.call {
        errors.push(
            IsoLiteralError::Host(TypeScriptHostError::MissingParentheses).with_span(span),
        );
    }
    let parsed_item = parsed.item.as_ref().and_then(item_of);
    if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
        if extraction.context.const_export_name.is_none() {
            errors.push(
                IsoLiteralError::Host(TypeScriptHostError::MissingExport {
                    suggested_name: selectable.name.item,
                })
                .with_span(span),
            );
        }
        if let AssociatedJsFunction::Absent = extraction.context.associated_js_function {
            errors.push(
                IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction)
                    .with_span(span),
            );
        }
    }
    errors
}

fn item_of(parse: &span::WithSpan<isograph_parser::IsoLiteralParse>) -> Option<&IsoLiteralItem> {
    parse
        .item
        .item
        .item
        .item
        .as_ref()
        .map(|item| item.item.reference())
}
```

`item_of` moves here from extract. Parse errors live on `ParsedIsoLiteral.errors`. File-absolute parse error spans are `error.location.with_offset(extraction.span().start)`.

A convenience that one file's diagnostics will want (lsp-parse-diagnostics.md). Define it here so that doc does not invent `file_literals`:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
pub struct FileLiteral<'a> {
    pub extraction: &'a isograph_compiler::IsoLiteralExtraction<TypeScriptHostLanguage>,
    pub parsed: &'a ParsedIsoLiteral,
    pub errors: Vec<span::WithSpan<IsoLiteralError<TypeScriptHostLanguage>>>,
}

pub fn file_literals<'a>(
    db: &'a IsographState,
    path: &Path,
) -> Option<Vec<FileLiteral<'a>>> {
    let extractions = extract_iso_literals_from_file_content::<TypeScriptHostLanguage>(
        db,
        path.to_owned(),
    )?;
    extractions
        .iter()
        .map(|extraction| {
            let parsed = parsed_iso_literal(db, extraction.iso_literal_text.clone());
            let mut errors = host_errors_for_extraction(extraction, parsed);
            for error in &parsed.errors {
                errors.push(
                    IsoLiteralError::Parse(error.item.clone())
                        .with_span(error.location.with_offset(extraction.span().start)),
                );
            }
            FileLiteral {
                extraction,
                parsed,
                errors,
            }
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`ParseError` must be `Clone` for that push. If it is not, take `error.item` by reference in `IsoLiteralError::Parse` or clone via existing derives. Check `ParseError`. If it is not `Clone`, add `Clone` as part of this change (a derive that matches an established pattern needs only the attribute).

lsp-parse-diagnostics.md currently takes `host` and `source: &str` and calls `file_literals(host, source)`. After this doc it takes `db` and `path`, or it keeps a `&str` entry for tests that do not intern a `DiskFile`. That doc updates when implemented. This doc ships `file_literals` on `db` + `path`.

`FileLiteral` is TypeScript-shaped because host errors are TypeScript. A generic version is `extraction + parsed + Vec<WithSpan<IsoLiteralError<T>>>`. Put the generic pieces (`parsed_iso_literal_in_file`) in `isograph_compiler`. Put `host_errors_for_extraction` and `file_literals` in `isograph_extract_typescript`.

### Tests

Move the extract-typescript error tests listed in extract-iso-literals-from-file.md change 1 here.

- Intern `iso(\`entrypoint Query.HomeRoute\`)`. `parsed_iso_literal_in_file::<TypeScriptHostLanguage>(db, path, 0)` is `Some` with empty parse errors. `host_errors_for_extraction` is empty.
- Intern `iso(\`entrypoint\`)`. Parse errors non-empty. Index 0 exists.
- Intern `iso\`entrypoint Query.HomeRoute\``. Extract context is `TaggedTemplate`. `host_errors_for_extraction` is `MissingParentheses` at the extraction span.
- Intern `iso(\`field Pet.fullName { id }\`)(`. `MissingExport`.
- Intern `export const fullName = iso(\`field Pet.fullName { id }\`)`. `MissingAssociatedFunction`.
- Intern `export const fullName = iso(\`field Pet.fullName { id }\`)(`. Host errors empty.
- Intern two literals, index 1 is the second. Index 2 is `None`.
- No `DiskFile`: `parsed_iso_literal_in_file` is `None`.
- Same literal text in two files (two paths): `parsed_iso_literal` of that text is one memo; both indices return trees that match.

`file_literals` of a file with `iso(\`entrypoint\`)`: one `FileLiteral`, `errors` contains a `Parse` at a span whose start is at least the extraction start.

## Call sites

- file-semantic-tokens.md -> `parsed_iso_literal` / extract + index.
- lsp-parse-diagnostics.md -> `file_literals`.
