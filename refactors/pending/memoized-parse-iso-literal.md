# Memoized parse of an extracted iso literal

Requires extract-iso-literals-from-file.md and `docs-website/docs/design-docs/pico.md`. Extract-all and `iso_literal_extraction(path, LineChar)` are memos. This file parses the literal at that location.

```text
parsed_iso_literal_at_location(path, LineChar)
  -> iso_literal_text_at_location(path, LineChar)
  + parsed_iso_literal(text)

iso_literal_text_at_location(path, LineChar)
  -> iso_literal_extraction(path, LineChar)

parsed_iso_literal(text)
  -> parse_iso_literal(&str)
```

Origin of the parse memo: isograph `memoized_parse_iso_literal`. Origin of looking up one literal from a file and cursor: isograph `get_iso_literal_extraction_from_text_position_params`. Delta: parse is keyed on the literal text only, not on `TextSource` or the file path (isograph's TODO: passing `text_source` breaks memoization when the literal moves); i2 `parse_iso_literal` already takes `&str` only; host embedding errors that need the parse tree run after parse, in `host_errors_for_extraction`.

`iso_literal_text_at_location` is the backdate seam. Prefixing the file changes `iso_literal_start_index`, so `iso_literal_extraction` is `!=`. The text string is `==`, so `iso_literal_text_at_location` backdates. `parsed_iso_literal_at_location` depends on the text memo and does not re-invoke. `parsed_iso_literal` of that text does not re-run. Two files with the same literal text share `parsed_iso_literal`.

Semantic tokens for a file are file-semantic-tokens.md: parse every extraction's text, not a cursor.

Three shippable changes: the text-keyed parse memo, the text and tree at a location, then host embedding errors.

## What the user does

No user-facing change. Tests intern a `DiskFile`, take a row and column inside the literal (`LineChar`), call `parsed_iso_literal_at_location`, and assert the tree (or parse errors). A field without an export reports `MissingExport` from `host_errors_for_extraction`.

## Change 1: `parsed_iso_literal`

`ParsedIsoLiteral` currently derives `Debug`. pico re-invoke compares with `==`, and `parsed_iso_literal_at_location` stores `Option<ParsedIsoLiteral>`. Add `Clone, PartialEq, Eq` to `ParsedIsoLiteral`. Add `Clone` to every type that field contains that does not have it (`Slot`, `IsoLiteralItem`, `EntrypointDeclaration`, `SelectableDeclaration`, and so on). A derive that matches an established pattern needs only the attribute change.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use isograph_parser::{ParsedIsoLiteral, parse_iso_literal};

#[memo]
pub fn parsed_iso_literal(db: &IsographState, iso_literal_text: String) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}
```

`db` is the pico database. The body does not read sources. Changing any `DiskFile` does not invalidate this memo unless the interned `iso_literal_text` param is different.

pico lookup returns `&ParsedIsoLiteral`.

`isograph_compiler` already depends on `isograph_parser`.

### Tests

In `crates/isograph_compiler/src/iso_literals.rs` under `#[cfg(test)]`. These tests do not need a host.

- `parsed_iso_literal(&db, "entrypoint Query.HomeRoute".to_owned())` has `errors` empty and `item` `Some` whose item is `IsoLiteralItem::Entrypoint`.
- `"entrypoint"` has a parse error (incomplete). `item` may still be `Some` (resilient parse). `errors` is not empty.
- `""` is `AstError::EmptyLiteral` as today.
- Calling twice with the same text returns a pointer to the same stored value (address equality is not required; asserting the tree twice is enough). A third call after interning an unrelated `DiskFile` still matches.

Do not add a production function only the tests call.

## Change 2: text and tree at a file and `LineChar`

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn iso_literal_text_at_location<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
    line_char: LineChar,
) -> Option<String> {
    iso_literal_extraction::<THostLanguage>(db, path, line_char)?
        .iso_literal_text
        .clone()
        .wrap_some()
}

#[memo]
pub fn parsed_iso_literal_at_location<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
    line_char: LineChar,
) -> Option<ParsedIsoLiteral> {
    let text = iso_literal_text_at_location::<THostLanguage>(db, path, line_char)?;
    parsed_iso_literal(db, text.clone())
        .clone()
        .wrap_some()
}
```

`None` is no `DiskFile`, or a position that is not inside any literal text. pico lookup of the text memo returns `&Option<String>`. pico lookup of the tree memo returns `&Option<ParsedIsoLiteral>`.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    LineChar, iso_literal_extraction, iso_literal_text_at_location, parsed_iso_literal,
    parsed_iso_literal_at_location,
};
```

Do not add a memo that parses every extraction in a file. file-semantic-tokens.md does that.

### Tests

Same `memo_tests` module as extract, using `TypeScriptHostLanguage`. One-line fixtures: `line` is 0, `character` is the byte index.

- No `DiskFile`: `iso_literal_text_at_location` and `parsed_iso_literal_at_location` are `None`.
- Intern `iso(\`entrypoint Query.HomeRoute\`)`. `character` is `contents.find("entrypoint")`. `iso_literal_text_at_location` is `Some` of that interior. `parsed_iso_literal_at_location` is `Some` with empty parse errors and `IsoLiteralItem::Entrypoint`. `character` 0 (`e` of `export`) is `None` for both.
- Intern `iso(\`entrypoint\`)`. `parsed_iso_literal_at_location` at the interior has parse errors non-empty.
- Intern two literals. A `character` inside the second literal text is the second tree. A `character` between the two backtick spans is `None`.
- Same literal text in two files (two paths): `parsed_iso_literal` of that text is one memo. Both locations return trees that match.

## Change 3: host embedding errors

Origin: isograph `process_iso_literal_extraction` (paren check, then parse, then associated-function check for fields). Origin of the error types: extract-iso-literals.md `TypeScriptHostError`. Delta: takes an extraction plus `&ParsedIsoLiteral` instead of parsing inside extract.

First reader of `IsoLiteralExtraction::span`. extract-iso-literals-from-file.md does not define it.

```rust
// from crates/isograph_compiler/src/host_language.rs
impl<THostLanguage: HostLanguage> IsoLiteralExtraction<THostLanguage> {
    pub fn span(&self) -> span::Span {
        span::Span::from_usize(
            self.iso_literal_start_index,
            self.iso_literal_start_index + self.iso_literal_text.len(),
        )
    }
}
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use isograph_compiler::IsoLiteralError;
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
    parse.item.item.as_ref().map(|item| item.item.reference())
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
    let extractions = TypeScriptHostLanguage::extract_iso_literals(db, path.to_owned())?;
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

`ParseError` is already `Clone`.

lsp-parse-diagnostics.md currently takes `host` and `source: &str` and calls `file_literals(host, source)`. After this doc it takes `db` and `path`, or it keeps a `&str` entry for tests that do not intern a `DiskFile`. That doc updates when implemented. This doc ships `file_literals` on `db` + `path`.

`FileLiteral` is TypeScript-shaped because host errors are TypeScript. A generic version is `extraction + parsed + Vec<WithSpan<IsoLiteralError<T>>>`. Put the generic pieces (`parsed_iso_literal`, `parsed_iso_literal_at_location`) in `isograph_compiler`. Put `host_errors_for_extraction` and `file_literals` in `isograph_extract_typescript`.

### Tests

Same `memo_tests` intern as extract-iso-literals-from-file.md (`intern_file`). One-line fixtures: `line` is 0, `character` is the byte index of the interior (`contents.find` of the iso text). `extraction` is `TypeScriptHostLanguage::extract_iso_literals(db, path)` `[0]`. `parsed` is `parsed_iso_literal_at_location` at that `LineChar`.

These nine are the extract-typescript error tests extract-iso-literals-from-file.md deletes. Write them again here.

- `tagged_template_is_missing_parentheses`. Intern `iso\`entrypoint Query.HomeRoute\``. `host_errors_for_extraction` is `IsoLiteralError::Host(TypeScriptHostError::MissingParentheses).wrap_vec()`.
- `incomplete_entrypoint_is_a_parse_error`. Intern `iso(\`entrypoint\`)`. `parsed.errors` is not empty. At least one `IsoLiteralError::Parse` in `file_literals` of that path.
- `entrypoint_without_export_is_valid`. Intern `iso(\`entrypoint Query.HomeRoute\`)`. `host_errors_for_extraction` is empty.
- `field_without_export_is_missing_export`. Intern `iso(\`field Pet.fullName { id }\`)(`. `host_errors_for_extraction` is one error, `IsoLiteralError::Host(TypeScriptHostError::MissingExport { .. })`.
- `field_without_associated_function_is_missing_associated_function`. Intern `export const fullName = iso(\`field Pet.fullName { id }\`)`. `host_errors_for_extraction` is `IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction).wrap_vec()`.
- `exported_field_with_associated_function_is_valid`. Intern `export const fullName = iso(\`field Pet.fullName { id }\`)(`. `host_errors_for_extraction` is empty.
- `tagged_template_field_reports_parentheses_and_export_and_associated`. Intern `iso\`field Pet.fullName { id }\``. `host_errors_for_extraction` is three errors: `MissingParentheses`, `MissingExport { .. }`, `MissingAssociatedFunction`, in that order.
- `host_error_span_is_the_extraction_span`. Intern `iso\`entrypoint Query.HomeRoute\``. `host_errors_for_extraction` has one error. Its `location` is `extraction.span()`.
- `valid_extraction_has_no_errors`. Intern `iso(\`entrypoint Query.HomeRoute\`)`. `parsed.errors` is empty. `host_errors_for_extraction` is empty.

Also:

- No `DiskFile`: `parsed_iso_literal_at_location` is `None`.
- `file_literals` of a file with `iso(\`entrypoint\`)`: one `FileLiteral`, `errors` contains a `Parse` at a span whose start is at least the extraction start.

## Call sites

- Tests in this file: intern a `DiskFile`, `parsed_iso_literal_at_location` at a `LineChar`, assert the AST.
- file-semantic-tokens.md -> `THostLanguage::extract_iso_literals` and `parsed_iso_literal` for each extraction's text.
- lsp-parse-diagnostics.md -> `file_literals`.
