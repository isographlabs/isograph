# Extract iso literals from a DiskFile

Requires filesystem-events.md (landed), extract-iso-literals.md (landed), and `docs-website/docs/design-docs/pico.md`. `HostLanguage::extract_iso_literals` is the pico memo over a `DiskFile`. Origin of a memoized trait method: isograph `CompilationProfile::deprecated_parse_type_system_documents` / `parse_nested_data_model_schema` in `crates/graphql_network_protocol/src/graphql_network_protocol.rs`. The trait writes `&T`. The impl writes `T` with `#[memo]`. First argument is `&Database`. There is no `&self`.

Origin of the extract work: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` `extract_iso_literals_from_file_content` and `IsoLiteralExtraction`. Delta: the memo is `HostLanguage::extract_iso_literals(db, path)`, not a free function; `PathBuf` instead of `RelativePathToSourceFile`; `THostLanguage::LiteralContext` instead of four ad-hoc fields; extract does not parse (isograph extract also does not parse; i2's TypeScript implementor currently does, and this doc stops that); missing `DiskFile` is `None`, not a panic.

This slice is all extractions in a file, and the extraction at a file and `LineChar`. Parsed AST at that location is memoized-parse-iso-literal.md. Semantic tokens for a file are file-semantic-tokens.md.

Four shippable changes: stop parsing inside extract, move the pico types to `isograph_compiler`, the extract-all memo, then the extraction at a row and column.

## What the user does

No user-facing change. Tests intern a `DiskFile` and assert the extracted literals: text, span, export name, call shape, and associated function. A row and column inside a literal text is `Some(extraction)`; a row and column in the JS around it is `None`.

## Change 1: extract does not parse

`TypeScriptHostLanguage::extract_iso_literals` currently calls `parse_iso_literal` and pushes `IsoLiteralError::Parse` / host errors that need the parse tree (`MissingExport`, `MissingAssociatedFunction`) onto `WithErrors.errors`.

isograph extract is regex only. Parse is a later memo. Host embedding checks that need the parse tree (`MissingExport` for a field) run after parse (memoized-parse-iso-literal.md).

Before:

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals<'a>(&self, source: &'a str) -> ExtractedIsoLiterals<'a, Self>;
}

pub type ExtractedIsoLiterals<'a, THostLanguage> = Vec<
    WithErrors<
        WithSpan<(&'a str, <THostLanguage as HostLanguage>::LiteralContext)>,
        Vec<WithSpan<IsoLiteralError<THostLanguage>>>,
    >,
>;
```

After:

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>>;
}
```

`WithErrors` and `IsoLiteralError` stay in this crate. They are used after parse. `extract_iso_literals` does not return them. The `ExtractedIsoLiterals` type alias is deleted.

`TypeScriptHostLanguage::extract_iso_literals` drops the `parse_iso_literal` call, the `item_of` helper, and the host-error pushes. It maps each regex capture to `WithSpan<(iso_literal_text, TypeScriptLiteralContext)>`. Commented captures still return `None`. Empty backticks still skip (`captures.name("literal")` fails).

`isograph_extract_typescript` no longer depends on `parse_iso_literal`, `IsoLiteralItem`, `IsoLiteralParse`, or `IsoLiteralError` for extract.

Tests that stay in `isograph_extract_typescript` (the ones that assert text, span, `const_export_name`, `IsoCall`, `AssociatedJsFunction`, skip comments, two literals, nested iso): rewrite them off `WithErrors`. They call `extract_iso_literals` and assert the `Vec<WithSpan<(...)>>`.

Tests that move to memoized-parse-iso-literal.md:

- `tagged_template_is_missing_parentheses`
- `incomplete_entrypoint_is_a_parse_error`
- `entrypoint_without_export_is_valid`
- `field_without_export_is_missing_export`
- `field_without_associated_function_is_missing_associated_function`
- `exported_field_with_associated_function_is_valid`
- `tagged_template_field_reports_parentheses_and_export_and_associated`
- `host_error_span_is_the_extraction_span`
- `valid_extraction_has_no_errors`

`tagged_template` (the extraction test that asserts `IsoCall::TaggedTemplate`) stays.

## Change 2: the database lives in `isograph_compiler`

filesystem-events.md puts `IsographState`, `DiskFile`, and `DiskFileMap` in `crates/isograph_cli/src/state.rs`. Memos over those sources cannot live in `isograph_compiler` if the compiler would depend on the CLI.

Move the pico types to `crates/isograph_compiler/src/database.rs`. `handle` stays in the CLI as a free function. That is the isograph split: `IsographDatabase` in `isograph_schema`, the event loop outside.

Origin of the move: filesystem-events.md `state.rs`. Delta: crate `isograph_compiler`; `handle` is no longer a method.

```rust
// from crates/isograph_compiler/src/database.rs
use std::collections::HashMap;
use std::path::PathBuf;

use pico::{SourceId, Storage};
use pico_macros::{Db, Source};

#[derive(Default, Debug, Db)]
pub struct IsographState {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: PathBuf,
    pub contents: String,
}
```

`handle` stays the writer. Tests intern a `DiskFile` the same way `handle` does: `db.set` plus insert into the tracked map. A `#[cfg(test)]` helper in the tests module is fine. No production method.

```rust
// from crates/isograph_cli/src/state.rs
use isograph_compiler::IsographState;
use prelude::Postfix;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, IsographEvent, Presence};

pub fn handle(state: &mut IsographState, event: IsographEvent) -> Vec<IsographEffect> {
    match event {
        IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
        IsographEvent::DiskChanged(change) => {
            handle_disk_changed(state, change);
            Vec::new()
        }
    }
}

fn handle_disk_changed(state: &mut IsographState, change: DiskChanged) {
    match change.presence {
        Presence::Present(contents) => {
            let source_id = state.set(DiskFile {
                path: change.path.clone(),
                contents,
            });
            state
                .get_disk_file_map_mut()
                .tracked()
                .0
                .insert(change.path, source_id);
        }
        Presence::Absent => {
            if let Some(source_id) = state
                .get_disk_file_map_mut()
                .tracked()
                .0
                .remove(&change.path)
            {
                state.remove(source_id);
            }
        }
    }
}
```

`run_event_loop` calls `handle(&mut state, event)`. Tests that currently write `state.handle(...)` write `handle(&mut state, ...)`.

`isograph_compiler` depends on `pico`, `pico_macros`, `prelude`. `isograph_cli` depends on `isograph_compiler` and drops direct `pico` / `pico_macros` unless something else in the crate needs them.

```rust
// from crates/isograph_compiler/src/lib.rs
mod database;
mod host_language;

pub use database::{DiskFile, DiskFileMap, IsographState};
pub use host_language::*;
```

### Tests

The existing `state.rs` tests. `state.handle(...)` becomes `handle(&mut state, ...)`. `disk_file` still reads the tracked map. Same assertions.

`run_event_loop` tests construct `IsographState::default()` as today.

## Change 3: `HostLanguage::extract_iso_literals` is the memo

Callers intern a `DiskFile` the same way `handle` does. There is no free function `extract_iso_literals_from_file_content`. Callers write `THostLanguage::extract_iso_literals(db, path)`.

Origin of `IsoLiteralExtraction`: isograph `IsoLiteralExtraction`. Delta: `context` instead of the four fields. `iso_literal_start_index` is the byte offset of the literal text in the file, same name as isograph. A cursor looks up an extraction by `LineChar`, not by vec position.

```rust
// from crates/isograph_compiler/src/host_language.rs
use std::path::PathBuf;

use crate::IsographState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoLiteralExtraction<THostLanguage: HostLanguage> {
    pub iso_literal_text: String,
    pub iso_literal_start_index: usize,
    pub context: THostLanguage::LiteralContext,
}

impl<THostLanguage: HostLanguage> IsoLiteralExtraction<THostLanguage> {
    pub fn span(&self) -> span::Span {
        span::Span::from_usize(
            self.iso_literal_start_index,
            self.iso_literal_start_index + self.iso_literal_text.len(),
        )
    }
}

pub trait HostLanguage: Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + std::fmt::Debug + 'static;

    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
}
```

`iso_literal_start_index` offsets semantic tokens to file coordinates (file-semantic-tokens.md). `LiteralContext` is stored and compared. `TypeScriptLiteralContext` already is `Copy`.

The change 1 method (`&self`, `&str`, borrowed `WithSpan`) is deleted. `ExtractedIsoLiterals` is already gone.

`isograph_extract_typescript` depends on `pico` and `pico_macros`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use std::path::PathBuf;

use isograph_compiler::{HostLanguage, IsoLiteralExtraction, IsographState};
use pico::Database;
use pico_macros::memo;
use prelude::Postfix;

impl HostLanguage for TypeScriptHostLanguage {
    type LiteralContext = TypeScriptLiteralContext;
    type Error = TypeScriptHostError;

    #[memo]
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
        let contents = db.get(source_id).contents.reference();
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                IsoLiteralExtraction {
                    iso_literal_text: literal.as_str().to_owned(),
                    iso_literal_start_index: literal.start(),
                    context: TypeScriptLiteralContext {
                        const_export_name: captures
                            .name("export_name")
                            .map(|m| m.as_str().intern().to()),
                        call: match captures.name("open_paren") {
                            Some(_) => IsoCall::FunctionCall,
                            None => IsoCall::TaggedTemplate,
                        },
                        associated_js_function: match captures.name("associated") {
                            Some(_) => AssociatedJsFunction::Present,
                            None => AssociatedJsFunction::Absent,
                        },
                    },
                }
                .wrap_some()
            })
            .collect::<Vec<_>>()
            .wrap_some()
    }
}
```

`None` is no `DiskFile` for that path. `Some(vec![])` is a present file with no literals.

`db.get(source_id)` records the file. pico.md's extract snippet looks up the path with `tracked()`. That sees a later `Present` of a path that was `None`. It also re-invokes this memo when any other path is inserted or removed. pico.md Tracked maps: a memo already keyed by `path` that then `db.get`s that source should use `untracked()` so an unrelated file does not re-invoke; a `None` that never `db.get`s cannot be purely `untracked`. This memo follows the extract snippet (`tracked()`).

`path` is interned as an owned `PathBuf` param. Callers pass `path.clone()` when they still need the path.

pico lookup is the trait return: `&Option<Vec<IsoLiteralExtraction<Self>>>`.

The change 1 tests that stayed in this crate (text, span, export name, `IsoCall`, `AssociatedJsFunction`, skip comments, two literals, nested iso) intern a `DiskFile` and call `TypeScriptHostLanguage::extract_iso_literals`.

### Tests

Tests in `crates/isograph_extract_typescript/src/lib.rs` under a `memo_tests` module. `use pico::Database` in the tests module for `db.get`. Do not add a test-only `HostLanguage` to the compiler crate.

- No `DiskFile` for the path: `TypeScriptHostLanguage::extract_iso_literals` is `None`.
- Present file, no `iso`: `Some` of empty vec.
- One exported field: vec len 1, `iso_literal_text` is the interior, `iso_literal_start_index` is `source.find(text)`, `const_export_name` is `Some`, `IsoCall::FunctionCall`, `AssociatedJsFunction::Present`.
- Two literals: vec len 2, start indices match `find`.
- Second `Present` on the same path with different contents replaces: extract sees the new literals, not the old.
- `Absent` then extract: `None`.

`expect` names the fixture the test interned.

## Change 4: the extraction at a file and `LineChar`

```text
iso_literal_extraction(path, LineChar)
  -> THostLanguage::extract_iso_literals(path)
  + find_iso_literal_extraction(LineChar, file text, extract vec)
```

Origin of `LineChar` and the walk: isograph `crates/isograph_lsp/src/hover.rs` `get_iso_literal_extraction_from_text_position_params` / `find_iso_literal_extraction_under_cursor`. Origin of `delta_line_delta_start`: isograph `crates/isograph_lsp/src/semantic_tokens.rs`. Delta: the memo returns `Option<IsoLiteralExtraction>`, not `(IsoLiteralExtraction, u32)`. It does not return an offset into the literal (`get_index_of_line_char`). There is no vec index on the signature. `LineChar` lives in `iso_literals.rs` so the compiler intern does not take `lsp_types::Position` (that type is not `Hash`).

`line` is 0-based count of `\n`. `character` is bytes since the last `\n`, same as isograph `delta_line_delta_start`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use std::path::PathBuf;

use pico_macros::memo;
use prelude::Postfix;

use crate::host_language::{HostLanguage, IsoLiteralExtraction};
use crate::IsographState;
use pico::Database;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct LineChar {
    pub line: u32,
    pub character: u32,
}

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
    line_char: LineChar,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = THostLanguage::extract_iso_literals(db, path.clone())?;
    let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
    let content = db.get(source_id).contents.reference();
    find_iso_literal_extraction(line_char, content, extractions).cloned()
}
```

`None` is no `DiskFile`, or a position that is not inside any literal text (JS around the literals, including `iso(`). pico lookup returns `&Option<IsoLiteralExtraction<THostLanguage>>`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
fn find_iso_literal_extraction<'a, THostLanguage: HostLanguage>(
    target_line_char: LineChar,
    content: &str,
    extracted_items: &'a [IsoLiteralExtraction<THostLanguage>],
) -> Option<&'a IsoLiteralExtraction<THostLanguage>> {
    let mut last_iteration_end_line_count = 0;
    let mut last_iteration_end_char_count = 0;
    let mut max_prev_span_end = 0;
    for extract_item in extracted_items {
        let iso_literal_start_index = extract_item.iso_literal_start_index;
        let iso_literal_end_index = iso_literal_start_index + extract_item.iso_literal_text.len();

        let intermediate_content = &content[max_prev_span_end..iso_literal_start_index];
        let (intermediate_line, intermediate_char) = delta_line_delta_start(intermediate_content);

        let start_line_count = last_iteration_end_line_count + intermediate_line;
        let start_char_count = if intermediate_line > 0 {
            intermediate_char
        } else {
            last_iteration_end_char_count + intermediate_char
        };

        let iso_content = &content[iso_literal_start_index..iso_literal_end_index];
        let (iso_line, iso_char) = delta_line_delta_start(iso_content);

        let end_line_count = start_line_count + iso_line;
        let end_char_count = if iso_line > 0 {
            iso_char
        } else {
            start_char_count + iso_char
        };

        if position_in_range(
            (start_line_count, start_char_count),
            (end_line_count, end_char_count),
            target_line_char,
        ) {
            return extract_item.wrap_some();
        }

        last_iteration_end_line_count = end_line_count;
        last_iteration_end_char_count = end_char_count;
        max_prev_span_end = iso_literal_end_index;
    }

    None
}

fn position_in_range(start: (u32, u32), end: (u32, u32), target: LineChar) -> bool {
    let (start_line_count, start_char_count) = start;
    let (end_line_count, end_char_count) = end;

    if target.line < start_line_count
        || (target.line == start_line_count && target.character < start_char_count)
        || target.line > end_line_count
        || (target.line == end_line_count && target.character > end_char_count)
    {
        return false;
    }

    true
}

fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, char) in text.chars().enumerate() {
        if char == '\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }
    (line_break_count, text.len() as u32 - last_line_break_index)
}
```

Origin of `find_iso_literal_extraction`: isograph `find_iso_literal_extraction_under_cursor`. Delta: returns `Option<&IsoLiteralExtraction>`, not `(IsoLiteralExtraction, u32)`. Inclusive of both ends, same as isograph `position_in_range`. The walk is a running line/char count; that is a state machine, so this is a loop.

Origin of `position_in_range` and `delta_line_delta_start`: copy from isograph `hover.rs` / `semantic_tokens.rs`. `delta_line_delta_start` is the same function as `lsp-semantic-token-encoding.md`. Do not depend on `isograph_lsp`. `position_in_range` is yes or no: the cursor is inside the span.

```rust
// from crates/isograph_compiler/src/lib.rs
mod iso_literals;

pub use host_language::IsoLiteralExtraction;
pub use iso_literals::{LineChar, iso_literal_extraction};
```

### Tests

Same `memo_tests` module. One-line fixtures: `line` is 0, `character` is the byte index.

- No `DiskFile`: `iso_literal_extraction::<TypeScriptHostLanguage>(db, path, LineChar { line: 0, character: 0 })` is `None`.
- Present file, no `iso`: any `LineChar` is `None`.
- `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. `character` is `contents.find("entrypoint")`. The function is `Some`; `iso_literal_text` and `iso_literal_start_index` match `TypeScriptHostLanguage::extract_iso_literals` `[0]`. `character` 0 (`e` of `export`) is `None`. The `LineChar` of the first byte of the interior is `Some`. The `LineChar` of the last byte of the interior is `Some`. One past that last byte is `None`.
- Two literals on one line. A `character` inside the second literal text is the second extraction. A `character` between the two backtick spans is `None`.

## Call sites

Change 2: `run_event_loop` -> `handle(&mut state, event)`. Tests intern a `DiskFile` the same way `handle` does: `db.set` plus insert into the tracked map.

Change 3: `iso_literal_extraction` -> `THostLanguage::extract_iso_literals`. file-semantic-tokens.md reads the vec for offsets.

Change 4: memoized-parse-iso-literal.md -> `iso_literal_extraction`.
