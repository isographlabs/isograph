# Extract iso literals from a DiskFile

Requires filesystem-events.md (landed), extract-iso-literals.md (landed), and `docs-website/docs/design-docs/pico.md`. `HostLanguage::extract_iso_literals` finds iso literals in a `&str`. This file puts that behind a pico memo over a `DiskFile`. The memos and keys are the iso-literal instance of that pico model.

Origin of the memo: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` `extract_iso_literals_from_file_content` and `IsoLiteralExtraction`. Delta: `PathBuf` instead of `RelativePathToSourceFile`; `THostLanguage::LiteralContext` instead of four ad-hoc fields (`const_export_name`, `has_associated_js_function`, `iso_function_called_with_paren` as bools); extract does not parse (isograph extract also does not parse; i2's TypeScript implementor currently does, and this doc stops that); missing `DiskFile` is `None`, not a panic.

Five shippable changes: stop parsing inside extract, move the pico types to `isograph_compiler`, the extract-all memo, row and column to an optional vec index, then the extraction at that index.

## What the user does

No user-facing change. Tests intern a `DiskFile` and assert the extracted literals: text, span, export name, call shape, and associated function. A row and column inside a literal text is `Some(index)`; a row and column in the JS around it is `None`.

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

## Change 3: the memo

`iso_literals.rs` in `isograph_compiler`. Callers intern a `DiskFile` the same way `handle` does.

```rust
// from crates/isograph_compiler/src/lib.rs
mod database;
mod host_language;
mod iso_literals;

pub use database::{DiskFile, DiskFileMap, IsographState};
pub use host_language::*;
pub use iso_literals::{IsoLiteralExtraction, extract_iso_literals_from_file_content};
```

### The extraction

Origin: isograph `IsoLiteralExtraction`. Delta: `context` instead of the four fields. `iso_literal_start_index` is the byte offset of the literal text in the file, same name as isograph. The nth literal in a file is the vec index, an argument to `parsed_iso_literal_in_file`, not a field.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
use std::path::PathBuf;

use pico_macros::memo;
use prelude::Postfix;
use span::WithSpan;

use crate::host_language::HostLanguage;
use crate::IsographState;
use pico::Database;

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
```

`iso_literal_index` takes the file and a row and column (`LineChar`) and returns `Option<usize>`. `iso_literal_extraction` takes that index. `iso_literal_start_index` offsets semantic tokens to file coordinates (file-semantic-tokens.md).

`HostLanguage::LiteralContext` gains `Clone + PartialEq + Eq + Debug + 'static` so the extraction can be stored and compared. `TypeScriptLiteralContext` already is `Copy`.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Sized + Default + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + std::fmt::Debug + 'static;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>>;
}
```

`TypeScriptHostLanguage` already implements `Default`.

### The memo

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn extract_iso_literals_from_file_content<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
    let contents = db.get(source_id).contents.reference();
    THostLanguage::default()
        .extract_iso_literals(contents)
        .into_iter()
        .map(|extracted| {
            let (iso_literal_text, context) = extracted.item;
            IsoLiteralExtraction {
                iso_literal_text: iso_literal_text.to_owned(),
                iso_literal_start_index: extracted.location.start as usize,
                context,
            }
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no `DiskFile` for that path. `Some(vec![])` is a present file with no literals.

`db.get(source_id)` records the file. pico.md's extract snippet looks up the path with `tracked()`. That sees a later `Present` of a path that was `None`. It also re-invokes this memo when any other path is inserted or removed. pico.md Tracked maps: a memo already keyed by `path` that then `db.get`s that source should use `untracked()` so an unrelated file does not re-invoke; a `None` that never `db.get`s cannot be purely `untracked`. This memo follows the extract snippet (`tracked()`).

`path` is interned as an owned `PathBuf` param. Callers pass `path.clone()` when they still need the path.

pico lookup returns `&Option<Vec<IsoLiteralExtraction<THostLanguage>>>`.

### Tests

Tests in `crates/isograph_extract_typescript/src/lib.rs` under a `memo_tests` module, using `TypeScriptHostLanguage`. That crate already depends on the compiler. `use pico::Database` in the tests module for `db.get`. Do not add a test-only `HostLanguage` to the compiler crate.

- No `DiskFile` for the path: `extract_iso_literals_from_file_content::<TypeScriptHostLanguage>` is `None`.
- Present file, no `iso`: `Some` of empty vec.
- One exported field: vec len 1, `iso_literal_text` is the interior, `iso_literal_start_index` is `source.find(text)`, `const_export_name` is `Some`, `IsoCall::FunctionCall`, `AssociatedJsFunction::Present`.
- Two literals: vec len 2, start indices match `find`.
- Second `Present` on the same path with different contents replaces: extract sees the new literals, not the old.
- `Absent` then extract: `None`.

`expect` names the fixture the test interned.

## Change 4: row and column to an index

Origin of `LineChar` and the walk: isograph `crates/isograph_lsp/src/hover.rs` `get_iso_literal_extraction_from_text_position_params` / `find_iso_literal_extraction_under_cursor`. Origin of `delta_line_delta_start`: isograph `crates/isograph_lsp/src/semantic_tokens.rs`. Delta: the memo returns `Option<usize>`, not `(IsoLiteralExtraction, u32)`. pico.md: the caller that has a cursor calls this; the caller that has an index calls `iso_literal_extraction`. Hover is one slot per `LineChar`. Returning the extraction would clone `iso_literal_text` into each of those slots. The index is `Copy`. The string lives on extract-all and on the one `iso_literal_extraction(path, index)` slot. Every character inside the first literal yields `Some(0)`, so `parsed_iso_literal_in_file(path, 0)` is one slot. `LineChar` lives in `iso_literals.rs` so the compiler intern does not take `lsp_types::Position` (that type is not `Hash`).

`line` is 0-based count of `\n`. `character` is bytes since the last `\n`, same as isograph `delta_line_delta_start`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct LineChar {
    pub line: u32,
    pub character: u32,
}

#[memo]
pub fn iso_literal_index<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
    line_char: LineChar,
) -> Option<usize> {
    let extractions =
        extract_iso_literals_from_file_content::<THostLanguage>(db, path.clone())?;
    let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
    let content = db.get(source_id).contents.reference();
    find_iso_literal_index(line_char, content, extractions)
}
```

`find_iso_literal_index` is the isograph walk with `enumerate`. It returns the index of the first extraction whose line/character range contains `line_char`. Inclusive of both ends, same as isograph `position_in_range`. It does not return an offset into the literal.

`None` is no `DiskFile`, or a position that is not inside any literal text (JS around the literals, including `iso(`).

pico lookup returns `&Option<usize>`.

Copy `delta_line_delta_start` and `position_in_range` into `iso_literals.rs`. Do not depend on `isograph_lsp`.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    IsoLiteralExtraction, LineChar, extract_iso_literals_from_file_content, iso_literal_index,
};
```

### Tests

Same `memo_tests` module. One-line fixtures: `line` is 0, `character` is the byte index.

- No `DiskFile`: `iso_literal_index::<TypeScriptHostLanguage>(db, path, LineChar { line: 0, character: 0 })` is `None`.
- `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. `character` is `contents.find("entrypoint")`. The memo is `Some(0)`. `character` 0 (`e` of `export`) is `None`.
- Two literals on one line. A `character` inside the second literal text is `Some(1)`. A `character` between the two backtick spans is `None`.

## Change 5: the extraction at an index

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState,
    path: PathBuf,
    index: usize,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    extract_iso_literals_from_file_content::<THostLanguage>(db, path)?
        .get(index)
        .cloned()
}
```

`None` is no `DiskFile`, or `index` past the last extraction. pico lookup returns `&Option<IsoLiteralExtraction<THostLanguage>>`.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use iso_literals::{
    IsoLiteralExtraction, LineChar, extract_iso_literals_from_file_content, iso_literal_extraction,
    iso_literal_index,
};
```

### Tests

Same `memo_tests` module. Intern the same fixtures as change 3.

- No `DiskFile`: `iso_literal_extraction::<TypeScriptHostLanguage>(db, path, 0)` is `None`.
- Present file, no `iso`: index 0 is `None`.
- One exported field: index 0 is `Some`, same text and `iso_literal_start_index` as `extract_iso_literals_from_file_content` `[0]`. Index 1 is `None`.
- Two literals: index 0 and 1 match the vec; index 2 is `None`.
- The `Some(0)` from change 4's `entrypoint` `LineChar` is the same extraction as `iso_literal_extraction(..., 0)`.

## Call sites

Change 2: `run_event_loop` -> `handle(&mut state, event)`. Tests intern a `DiskFile` the same way `handle` does: `db.set` plus insert into the tracked map.

Change 3: `iso_literal_index` and `iso_literal_extraction` -> `extract_iso_literals_from_file_content`. file-semantic-tokens.md reads the vec for offsets.

Change 4: memoized-parse-iso-literal.md tests pick a `LineChar` and call `iso_literal_index`.

Change 5: memoized-parse-iso-literal.md -> `iso_literal_extraction`.
