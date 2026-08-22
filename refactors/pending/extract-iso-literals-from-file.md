# Extract iso literals from a DiskFile

Requires filesystem-events.md (landed), extract-iso-literals.md (landed), and `docs-website/docs/design-docs/pico.md`. `HostLanguage::extract_iso_literals` is the pico memo over a `DiskFile`. Origin of a memoized trait method: isograph `CompilationProfile::deprecated_parse_type_system_documents` / `parse_nested_data_model_schema` in `crates/graphql_network_protocol/src/graphql_network_protocol.rs`. The trait writes `&T`. The impl writes `T` with `#[memo]`. First argument is `&Database`. There is no `&self`.

Origin of the extract work: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` `extract_iso_literals_from_file_content` and `IsoLiteralExtraction`. Delta: the memo is `HostLanguage::extract_iso_literals(db, path)`, not a free function; `PathBuf` instead of `RelativePathToSourceFile`; `THostLanguage::LiteralContext` instead of four ad-hoc fields; extract does not parse (isograph extract also does not parse; i2's TypeScript implementor currently does, and this doc stops that); missing `DiskFile` is `None`, not a panic.

This slice is all extractions in a file, and the extraction at a file and `LineChar`. Parsed AST at that location is memoized-parse-iso-literal.md. Semantic tokens for a file are file-semantic-tokens.md.

Four shippable changes: stop parsing inside extract, move the pico types to `isograph_compiler`, the extract-all memo, then the extraction at a row and column.

## What the user does

No user-facing change. Tests of the host regex pass a string and assert the extracted literals: text, span, export name, call shape, and associated function. Tests of the memos intern a `DiskFile`. A row and column inside a literal text is `Some(extraction)`; a row and column in the JS around it is `None`.

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

```rust
// from crates/isograph_extract_typescript/src/lib.rs
impl HostLanguage for TypeScriptHostLanguage {
    type LiteralContext = TypeScriptLiteralContext;
    type Error = TypeScriptHostError;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(source)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                let span = Span::from_usize(literal.start(), literal.end());
                let context = TypeScriptLiteralContext {
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
                };
                (literal.as_str(), context).with_span(span).wrap_some()
            })
            .collect()
    }
}
```

`item_of` is deleted. Commented captures still return `None`. Empty backticks still skip (`captures.name("literal")` fails).

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

Origin of the move: filesystem-events.md `state.rs`. Delta: crate `isograph_compiler`; `handle` is no longer a method; `set` plus the map insert is `insert_disk_file` / `remove_disk_file` on `IsographState` (origin isograph `insert_iso_literal` / `remove_iso_literal`), so the CLI does not name pico.

```rust
// from crates/isograph_compiler/src/database.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pico::{Database, SourceId, Storage};
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

impl IsographState {
    pub fn insert_disk_file(&mut self, path: PathBuf, contents: String) {
        let source_id = self.set(DiskFile {
            path: path.clone(),
            contents,
        });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: &Path) {
        if let Some(source_id) = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .remove(path)
        {
            self.remove(source_id);
        }
    }
}
```

`handle` stays the writer. It calls `insert_disk_file` / `remove_disk_file`. Tests intern a `DiskFile` the same way: `insert_disk_file`.

```rust
// from crates/isograph_cli/src/state.rs
use prelude::Postfix;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, IsographEvent, Presence};

pub use isograph_compiler::IsographState;

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
            state.insert_disk_file(change.path, contents);
        }
        Presence::Absent => {
            state.remove_disk_file(&change.path);
        }
    }
}
```

`run_event_loop` calls `handle(&mut state, event)`. Tests that currently write `state.handle(...)` write `handle(&mut state, ...)`.

`daemon.rs` keeps `IsographState` from `state` (re-exported) and adds `handle`.

```rust
// from crates/isograph_cli/src/daemon.rs
use crate::state::{handle, IsographState};
```

```rust
// from crates/isograph_cli/src/daemon.rs
        let effects = handle(&mut state, event);
```

```toml
# from crates/isograph_compiler/Cargo.toml
pico = { path = "../pico" }
pico_macros = { path = "../pico_macros" }
prelude = { path = "../prelude" }
```

`isograph_parser`, `span`, `thiserror` stay.

```toml
# from crates/isograph_cli/Cargo.toml
isograph_compiler = { path = "../isograph_compiler" }
```

Drop `pico` and `pico_macros` from `[dependencies]`. `pico` moves to `[dev-dependencies]`: the `disk_file` helper and the map-len assertions use `Database::get` and `View::untracked`. Production CLI code does not.

```toml
# from crates/isograph_cli/Cargo.toml
[dev-dependencies]
pico = { path = "../pico" }
```

```rust
// from crates/isograph_compiler/src/lib.rs
mod database;
mod host_language;

pub use database::{DiskFile, DiskFileMap, IsographState};
pub use host_language::*;
```

### Tests

The existing `state.rs` tests. `state.handle(...)` becomes `handle(&mut state, ...)`. `DiskFile` is `use isograph_compiler::DiskFile`. `disk_file` still reads the tracked map (`use pico::Database` in the tests module). Same assertions.

`run_event_loop` tests construct `IsographState::default()` as today.

## Change 3: `HostLanguage::extract_iso_literals` is the memo

Callers intern a `DiskFile` with `insert_disk_file`. There is no free function `extract_iso_literals_from_file_content`. Callers write `THostLanguage::extract_iso_literals(db, path)`.

Origin of `IsoLiteralExtraction`: isograph `IsoLiteralExtraction`. Delta: `context` instead of the four fields. `iso_literal_start_index` is the byte offset of the literal text in the file, same name as isograph. A cursor looks up an extraction by `LineChar`, not by vec position.

```rust
// from crates/isograph_compiler/src/host_language.rs
use std::path::PathBuf;

use span::WithSpan;

use crate::IsographState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoLiteralExtraction<THostLanguage: HostLanguage> {
    pub iso_literal_text: String,
    pub iso_literal_start_index: usize,
    pub context: THostLanguage::LiteralContext,
}

pub trait HostLanguage: Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + std::fmt::Debug + 'static;

    fn extract_iso_literals_from_source<'a>(
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>>;

    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
}
```

`iso_literal_start_index` offsets semantic tokens to file coordinates (file-semantic-tokens.md). `LiteralContext` is stored and compared. `TypeScriptLiteralContext` already is `Copy`.

The change 1 method is `extract_iso_literals_from_source`. It loses `&self`. The regex body is unchanged. `extract_iso_literals(db, path)` is the memo: look up the `DiskFile`, call `from_source`, own the captures. `ExtractedIsoLiterals` is already gone.

`isograph_extract_typescript` depends on `pico`, `pico_macros`, and `tracing`. `#[memo]` expands to `::pico::` and `::tracing::debug_span!`. Origin `graphql_network_protocol` has the same three deps.

```toml
# from crates/isograph_extract_typescript/Cargo.toml
pico = { path = "../pico" }
pico_macros = { path = "../pico_macros" }
tracing = { workspace = true }
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use std::path::PathBuf;

use isograph_compiler::{HostLanguage, IsoLiteralExtraction, IsographState};
use pico::Database;
use pico_macros::memo;
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

impl HostLanguage for TypeScriptHostLanguage {
    type LiteralContext = TypeScriptLiteralContext;
    type Error = TypeScriptHostError;

    fn extract_iso_literals_from_source<'a>(
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(source)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                let span = Span::from_usize(literal.start(), literal.end());
                let context = TypeScriptLiteralContext {
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
                };
                (literal.as_str(), context).with_span(span).wrap_some()
            })
            .collect()
    }

    #[memo]
    fn extract_iso_literals(
        db: &IsographState,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
        let contents = db.get(source_id).contents.reference();
        Self::extract_iso_literals_from_source(contents)
            .into_iter()
            .map(|extracted| IsoLiteralExtraction {
                iso_literal_text: extracted.item.0.to_owned(),
                iso_literal_start_index: extracted.location.start as usize,
                context: extracted.item.1,
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

The change 1 tests that stayed in this crate (text, span, export name, `IsoCall`, `AssociatedJsFunction`, skip comments, two literals, nested iso) keep passing a string. They call `TypeScriptHostLanguage::extract_iso_literals_from_source`.

### Tests

Tests in `crates/isograph_extract_typescript/src/lib.rs` under a `memo_tests` module. Do not add a test-only `HostLanguage` to the compiler crate.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
fn intern_file(db: &mut IsographState, path: PathBuf, contents: &str) {
    db.insert_disk_file(path, contents.to_owned());
}
```

Absent is `db.remove_disk_file(&path)`. Intern present files with `intern_file`.

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

Origin of `LineChar` and the walk: isograph `crates/isograph_lsp/src/hover.rs` `get_iso_literal_extraction_from_text_position_params` / `find_iso_literal_extraction_under_cursor`. Origin of the substring measure: isograph `crates/isograph_lsp/src/semantic_tokens.rs` `delta_line_delta_start`. Delta: the memo returns `Option<IsoLiteralExtraction>`, not `(IsoLiteralExtraction, u32)`. It does not return an offset into the literal (`get_index_of_line_char`). There is no vec index on the signature. `LineChar` lives in `iso_literals.rs` so the compiler intern does not take `lsp_types::Position` (that type is not `Hash`). Name is `line_and_byte`: `line` is a count of `\n` bytes, `character` is bytes after the last `\n`. Origin mixed `chars().enumerate()` with `text.len()`.

`line` is 0-based count of `\n`. `character` is bytes since the last `\n`.

`#[memo]` hashes the signature text once at expansion, then interned params. `THostLanguage` is not a param. All monomorphizations of `iso_literal_extraction` share one slot. isograph avoided this: `IsographDatabase<TCompilationProfile>` is a different database type per profile. i2 `IsographState` is not generic (pico.md). One host per process (pluggable-compiler: the binary names the implementor). A second host in the same `IsographState` looking up the same `(path, LineChar)` downcasts the stored value and panics. The TypeScript `extract_iso_literals` impl is a different expanded function, so it does not share that slot.

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

`None` is no `DiskFile`, or a position that is not inside any literal text (JS around the literals, including `iso(` and the closing backtick). pico lookup returns `&Option<IsoLiteralExtraction<THostLanguage>>`.

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
        let (intermediate_line, intermediate_char) = line_and_byte(intermediate_content);

        let start_line_count = last_iteration_end_line_count + intermediate_line;
        let start_char_count = if intermediate_line > 0 {
            intermediate_char
        } else {
            last_iteration_end_char_count + intermediate_char
        };

        let iso_content = &content[iso_literal_start_index..iso_literal_end_index];
        let (iso_line, iso_char) = line_and_byte(iso_content);

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
        || (target.line == end_line_count && target.character >= end_char_count)
    {
        return false;
    }

    true
}

fn line_and_byte(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }
    (line_break_count, text.len() as u32 - last_line_break_index)
}
```

Origin of `find_iso_literal_extraction`: isograph `find_iso_literal_extraction_under_cursor`. Delta: returns `Option<&IsoLiteralExtraction>`, not `(IsoLiteralExtraction, u32)`. Exclusive of the end. Origin `position_in_range` treats `character == end_char_count` as inside (the caret one past the last interior byte, on the closing backtick). This one treats that caret as outside, same as `iso(` and the rest of the JS around the literal. Start stays inclusive. The walk is a running line/char count; that is a state machine, so this is a loop.

The slices are this file's text at indices this file's regex produced (`captures_iter` is left to right, non-overlapping). User input cannot hand `find` a vec from another file. A panic here is a broken caller.

Origin of `position_in_range`: copy from isograph `hover.rs`. Origin of `line_and_byte`: isograph `delta_line_delta_start`, byte walk. `lsp-semantic-token-encoding.md` keeps the origin function under the origin name. Do not depend on `isograph_lsp`. `position_in_range` is yes or no: the cursor is inside the span.

`isograph_compiler` gains `tracing`. `#[memo]` on `iso_literal_extraction` expands to `::tracing::debug_span!`.

```toml
# from crates/isograph_compiler/Cargo.toml
tracing = { workspace = true }
```

```rust
// add to crates/isograph_compiler/src/lib.rs
mod iso_literals;

pub use iso_literals::{LineChar, iso_literal_extraction};
```

### Tests

Same `memo_tests` module. Intern with `intern_file`. One-line fixtures: `line` is 0, `character` is the byte index.

- No `DiskFile`: `iso_literal_extraction::<TypeScriptHostLanguage>(db, path, LineChar { line: 0, character: 0 })` is `None`.
- Present file, no `iso`: any `LineChar` is `None`.
- `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. `character` is `contents.find("entrypoint")`. The function is `Some`; `iso_literal_text` and `iso_literal_start_index` match `TypeScriptHostLanguage::extract_iso_literals` `[0]`. `character` 0 (`e` of `export`) is `None`. The `LineChar` of the first byte of the interior is `Some`. The `LineChar` of the last byte of the interior is `Some`. One past that last byte is `None`.
- Two literals on one line. A `character` inside the second literal text is the second extraction. A `character` between the two backtick spans is `None`.
- `iso(\`\nentrypoint Query.HomeRoute\n\`)`. `LineChar { line: 1, character: 0 }` (`e` of `entrypoint`) is `Some`. `{ line: 0, character: 0 }` (`i` of `iso`) is `None`. `{ line: 2, character: 0 }` (the closing backtick) is `None`.
- Two literals, the second starting on a later line. A `LineChar` inside the second interior is the second extraction. A `LineChar` on the JS line between them is `None`.

## Call sites

Change 2: `run_event_loop` -> `handle(&mut state, event)`. Tests intern a `DiskFile` with `insert_disk_file`.

Change 3: `iso_literal_extraction` -> `THostLanguage::extract_iso_literals`. file-semantic-tokens.md reads the vec for offsets.

Change 4: memoized-parse-iso-literal.md -> `iso_literal_extraction`.
