# Extract iso literals from a DiskFile

Requires filesystem-events.md and extract-iso-literals.md (landed). `HostLanguage::extract_iso_literals` finds iso literals in a `&str`. This file puts that behind a pico memo over a `DiskFile`.

Origin of the memo: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` `extract_iso_literals_from_file_content` and `IsoLiteralExtraction`. Delta: `PathBuf` instead of `RelativePathToSourceFile`; `THostLanguage::LiteralContext` instead of four ad-hoc fields (`const_export_name`, `has_associated_js_function`, `iso_function_called_with_paren` as bools); extract does not parse (isograph extract also does not parse; i2's TypeScript implementor currently does, and this doc stops that); missing `DiskFile` is `None`, not a panic.

Two shippable changes: stop parsing inside extract, then the memo.

## What the user does

No user-facing change. Tests intern a `DiskFile` and assert the extracted literals: text, span, export name, call shape, associated function, and index.

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

## Change 2: the database lives in `isograph_compiler`, then the memo

filesystem-events.md puts `IsographState`, `DiskFile`, `DiskFileMap`, `insert_disk_file`, and `remove_disk_file` in `crates/isograph_cli/src/state.rs`. Memos over those sources cannot live in `isograph_compiler` if the compiler would depend on the CLI.

Move the pico types to `crates/isograph_compiler/src/database.rs`. `handle` stays in the CLI as a free function. That is the isograph split: `IsographDatabase` in `isograph_schema`, the event loop outside.

Origin of the move: filesystem-events.md `state.rs`. Delta: crate `isograph_compiler`; `handle` is no longer a method.

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

    pub fn remove_disk_file(&mut self, path: &Path) -> Option<SourceId<DiskFile>> {
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .remove(path)
            .inspect(|&source_id| self.remove(source_id))
    }
}
```

`insert_disk_file` / `remove_disk_file` are `pub`. The CLI `handle` is the production writer. Compiler tests intern files through these methods.

```rust
// from crates/isograph_cli/src/state.rs
use isograph_compiler::IsographState;
use prelude::Postfix;

use crate::effect::{IsographEffect, LogDiskChanged};
use crate::event::{DiskChanged, IsographEvent, Presence};

pub fn handle(state: &mut IsographState, event: IsographEvent) -> Vec<IsographEffect> {
    match event {
        IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
        IsographEvent::DiskChanged(change) => handle_disk_changed(state, change),
    }
}

fn handle_disk_changed(state: &mut IsographState, change: DiskChanged) -> Vec<IsographEffect> {
    let path = change.path;
    match change.presence {
        Presence::Present(present) => {
            state.insert_disk_file(path.clone(), present.contents);
            IsographEffect::LogDiskPresent(LogDiskChanged {
                path,
                file_count: state.get_disk_file_map().untracked().0.len(),
            })
            .wrap_vec()
        }
        Presence::Absent => {
            state.remove_disk_file(path.reference());
            IsographEffect::LogDiskAbsent(LogDiskChanged {
                path,
                file_count: state.get_disk_file_map().untracked().0.len(),
            })
            .wrap_vec()
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
mod iso_literals;

pub use database::{DiskFile, DiskFileMap, IsographState};
pub use host_language::*;
pub use iso_literals::{IsoLiteralExtraction, extract_iso_literals_from_file_content};
```

### The extraction

Origin: isograph `IsoLiteralExtraction`. Delta: `context` instead of the four fields; `index` is the 0-based position in this file; `iso_literal_start_index` is the byte offset of the literal text in the file, same name as isograph.

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
    pub index: usize,
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

`index` is `enumerate()` over the extract vec. `parsed_iso_literal_in_file` (memoized-parse-iso-literal.md) takes that index. `iso_literal_start_index` offsets semantic tokens to file coordinates (file-semantic-tokens.md). Both are used. They are not the same number.

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
        .enumerate()
        .map(|(index, extracted)| {
            let (iso_literal_text, context) = extracted.item;
            IsoLiteralExtraction {
                index,
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

The memo reads the tracked map, then `db.get(source_id)`, so a contents change invalidates. Adding or removing any path also invalidates (tracked map). isograph does the same for `get_iso_literal`.

`path` is interned as an owned `PathBuf` param. Callers pass `path.clone()` when they still need the path.

pico lookup returns `&Option<Vec<IsoLiteralExtraction<THostLanguage>>>`.

## Tests

Tests in `crates/isograph_extract_typescript/src/lib.rs` under a `memo_tests` module, using `TypeScriptHostLanguage`. That crate already depends on the compiler. `use pico::Database` in the tests module for `db.get`. Do not add a test-only `HostLanguage` to the compiler crate.

- No `DiskFile` for the path: `extract_iso_literals_from_file_content::<TypeScriptHostLanguage>` is `None`.
- Present file, no `iso`: `Some` of empty vec.
- One exported field: vec len 1, `index == 0`, `iso_literal_text` is the interior, `iso_literal_start_index` is `source.find(text)`, `const_export_name` is `Some`, `IsoCall::FunctionCall`, `AssociatedJsFunction::Present`.
- Two literals: indices 0 and 1, start indices match `find`.
- Second `Present` on the same path with different contents replaces: extract sees the new literals, not the old.
- `Absent` then extract: `None`.

`expect` names the fixture the test interned.

Tests of the trait change (no parse inside extract) are the rewritten extract tests in change 1: `iso(\`entrypoint\`)` extracts text even though it is a parse error; `iso\`...\`` extracts with `IsoCall::TaggedTemplate` and no errors on the extract result (there is no error vec).

## Call sites

- `handle` -> `insert_disk_file` / `remove_disk_file`.
- memoized-parse-iso-literal.md -> `extract_iso_literals_from_file_content`.
- file-semantic-tokens.md -> the same memo.
