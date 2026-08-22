# Interned path as memo param and DiskFile key

Requires filesystem-events.md (landed), extract-iso-literals-from-file.md (landed), literal-id.md (landed), and `docs-website/docs/design-docs/pico.md`. pico.md: the inner model does not know about the filesystem; a `DiskFile` is a source in the database; `PathBuf` on a source key, a tracked map, or a memo argument is a leak. The code still uses `PathBuf`. This file is the type, the conversion at `handle`, and the call-site rewrite.

Origin of the interned path: isograph `RelativePathToSourceFile` (`string_key_newtype!` in `crates/common_lang_types/src/string_key_types.rs`). Origin of converting an absolute path: isograph `relative_path_from_absolute_and_working_directory`. Origin of using that interned path as a source key and a memo param: isograph `IsoLiteralsSource.relative_path` and `extract_iso_literals_from_file_content(db, relative_path_to_source_file)`. Delta: the base directory is the config file's parent, not process CWD; `DiskChanged.path` stays an absolute `PathBuf` (event-model.md); `handle` converts; `IsographState` never takes a `PathBuf`; compiler tests intern a relative string and never go through that conversion.

i2 extract-iso-literals-from-file.md chose `PathBuf` instead of `RelativePathToSourceFile`. pico hashes that `PathBuf` as an owned memo param and clones it out of the param store on every execute. `RelativePathToSourceFile` is `Copy`. `LiteralId` becomes `Copy`.

`parsed_iso_literal` still takes `String`. Out of scope.

One shippable change.

## What the user does

No CLI change. `isograph send` still writes `DiskChanged` with an absolute `path`. After `handle`, that file's identity in the database is the path relative to the directory that contains the config file.

Tests of extract and of `insert_disk_file` intern `"src/a.ts"`. They do not intern `/tmp/proj/src/a.ts`.

## Types

Most important first.

```rust
// from crates/common_lang_types/src/string_key_types.rs
string_key_newtype!(RelativePathToSourceFile);
string_key_newtype!(CurrentWorkingDirectory);
```

No new interned type. `RelativePathToSourceFile` is Copy, Hash, Eq, Display, `AsRef<Path>`. Construction from a relative UTF-8 string is `"src/a.ts".intern().to()`. pico.md names this type.

`CurrentWorkingDirectory` is a pico singleton. This change intern it as the parent directory of the config file, not `std::env::current_dir()`. isograph interned process CWD. i2 `source_files` globs are already relative to the config file's directory. The singleton's value is that same directory. The type name stays. `serve` intern it with `db.set`. Memos do not read it. `handle` reads it to convert an event `PathBuf`.

`DiskChanged.path` stays `PathBuf`. event-model.md: the event path is absolute and canonical. A relative path is resolved by the source that built the event, never by `handle`. `handle` does the other conversion: absolute OS path to interned project-relative identity. After that conversion, `insert_disk_file` takes only `RelativePathToSourceFile`. The compiler crate does not mention `PathBuf` in production APIs.

```rust
// from crates/isograph_compiler/src/database.rs
use std::collections::HashMap;
use std::marker::PhantomData;

use common_lang_types::RelativePathToSourceFile;
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};

use crate::HostLanguage;

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<RelativePathToSourceFile, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: RelativePathToSourceFile,
    pub contents: String,
}
```

`path` is Copy. `insert_disk_file` does not clone it.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub path: RelativePathToSourceFile,
    pub index: usize,
}
```

`LineChar` is unchanged.

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
```

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<LiteralId>;

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>>;
```

`iso_literal_extraction` still takes `LiteralId`. `literal_id.path` is Copy, so the body does not `.clone()` the path when it calls extract.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>>;
```

Lookup is `get(&path)`. No `path.clone()` to call another memo.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
pub fn file_literals<'a>(
    db: &'a IsographState<TypeScriptHostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<FileLiteral<'a>>>;
```

Who calls `file_literals`: tests in this crate. lsp-parse-diagnostics.md later.

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
    }
}
```

`Default` does not intern `CurrentWorkingDirectory`. Extract memos do not read that singleton. Extract tests and `insert_disk_file` tests keep `IsographState::default()` and intern a relative string themselves.

```rust
// from crates/isograph_cli/src/state.rs
use std::path::{Path, PathBuf};

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Intern;
use pico::Database;

pub(crate) fn intern_config_directory(
    state: &mut IsographState<impl HostLanguage>,
    config_path: &Path,
) {
    let directory = config_path
        .parent()
        .expect("a config file path has a parent directory");
    let interned: CurrentWorkingDirectory = directory
        .to_str()
        .expect("the config directory is UTF-8")
        .intern()
        .to();
    state.set(interned);
}

fn relative_path_to_source_file(
    state: &IsographState<impl HostLanguage>,
    absolute: &PathBuf,
) -> RelativePathToSourceFile {
    let cwd = *state
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("CurrentWorkingDirectory is interned from the config path before DiskChanged");
    relative_path_from_absolute_and_working_directory(cwd, absolute)
}

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    change: DiskChanged,
) {
    let path = relative_path_to_source_file(state, &change.path);
    match change.presence {
        Presence::Present(contents) => {
            state.insert_disk_file(path, contents);
        }
        Presence::Absent => {
            state.remove_disk_file(path);
        }
    }
}
```

`intern_config_directory` who calls it: `serve` (the daemon's `config_path` is already canonical from `discover::config_path`). Handle tests that send `DiskChanged`.

`relative_path_to_source_file` who calls it: `handle_disk_changed` only. It is not a method on `IsographState`. The compiler crate does not convert OS paths.

```rust
// from crates/isograph_cli/src/daemon.rs
    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
```

`run_event_loop` tests that only send `HelloWorld` / `Quit` keep `IsographState::default()`.

```toml
# from crates/isograph_compiler/Cargo.toml
common_lang_types = { path = "../common_lang_types" }

[dev-dependencies]
intern = { path = "../../relay-crates/intern" }
```

```toml
# from crates/isograph_cli/Cargo.toml
common_lang_types = { path = "../common_lang_types" }
intern = { path = "../../relay-crates/intern" }
pico = { path = "../pico" }
```

`isograph_compiler` tests intern `"src/a.ts"`. Production compiler code takes `RelativePathToSourceFile` as a parameter. It does not intern strings. `isograph_cli` intern the config directory and converts event paths.

## Expect and panic

`relative_path_from_absolute_and_working_directory` already `expect`s that `pathdiff` returns a path and that the remainder is UTF-8. This change calls that function. It does not add a new `expect` there.

`intern_config_directory` `expect`s a parent directory. A config path from `discover::config_path` is a canonical file path. `Path::parent` of a file path is `Some`. The type system does not distinguish a file path from `/`.

`intern_config_directory` `expect`s UTF-8. `string_key_newtype!` intern is a UTF-8 `StringKey`. A non-UTF-8 config directory cannot be that singleton.

`relative_path_to_source_file` `expect`s the singleton. `serve` intern it before the event loop recvs. A `DiskChanged` on `IsographState::default()` is a missed intern. The type system does not require a singleton to have been `set`.

isograph `get_current_working_directory` is the same `expect`.

## Normalization

Intern is the UTF-8 string. `"src/a.ts"` and `"src/./a.ts"` are two keys. isograph did not normalize. This change does not.

## pico.md

Already the inner model: no filesystem, `PathBuf` is a smell on sources and memos, identity is `RelativePathToSourceFile`, `handle` converts at ingest, `LiteralId` is `Copy`, `CurrentWorkingDirectory` is the config file's parent and memos do not read it. This change makes the code match.

## Tests

### `database.rs`

`intern_path` in the tests module is `"src/a.ts".intern().to()`. Every current `PathBuf::from("/tmp/proj/src/a.ts")` intern becomes that. `disk_file` takes `RelativePathToSourceFile`. `TestHostLanguage::extract_iso_literals` takes `RelativePathToSourceFile`.

Existing insert/remove/replace/two-paths/empty-string tests, same assertions, interned keys.

New:

- `""` interned is a map key. Insert, lookup, remove.
- `"src/a.ts"` and `"src/./a.ts"` interned are two map entries.

### `isograph_extract_typescript`

Both `intern_file` helpers take `RelativePathToSourceFile`. `one_literal` returns that type. `text_at_line_char` / `parsed_at_line_char` take it. `file_literals(&db, path)`: `path` is Copy, no `&path`.

Every fixture path `/tmp/proj/src/a.ts` becomes `"src/a.ts".intern().to()`. `/tmp/proj/src/b.ts` / `other.ts` become `"src/b.ts"` / `"src/other.ts"`. Same assertions.

### `state.rs`

Handle tests that send `DiskChanged` call `intern_config_directory` with `Path::new("/tmp/proj/isograph.config.json")` before `handle`.

- Present of `/tmp/proj/src/a.ts` inserts `"src/a.ts"`. Assert contents through the map. Effects stay empty.
- Same config, Present of `/tmp/other/a.ts` inserts `"../other/a.ts"`.
- Absent of `/tmp/proj/src/a.ts` after Present removes `"src/a.ts"`. Effects stay empty.
- `handle` of `DiskChanged` on `IsographState::default()` panics (`#[should_panic]`). The singleton is missing.

external.rs event-frame tests stay `PathBuf`. They do not intern.

### `daemon.rs`

`serve` calls `intern_config_directory`. Event-loop unit tests that do not send `DiskChanged` stay `default()`.

### e2e `cli.rs`

Send `DiskChanged` of `/tmp/proj/src/a.ts` against a temp-dir config still exits 0. The interned key is the pathdiff remainder, not `src/a.ts`. This crate does not assert that key.

## Later docs

file-semantic-tokens.md, semantic-tokens-line-offset.md, and e2e-semantic-tokens.md already wrote `RelativePath`. That is `RelativePathToSourceFile`. e2e-semantic-tokens.md "same string `DiskChanged.path` interned" is wrong: `handle` diffs against the config directory first. Those docs update when they are implemented.
