# Working with pico

pico is incremental memoization. The compiler state is a pico database. Facts from outside the process are sources. Everything derived from those facts is a memo.

A memo is a deterministic function of its arguments and of the sources and memos it reads while running. pico records those reads. When a source changes, pico re-invokes a memo only if a recorded read changed. If the new value `==` the old value, dependents do not re-invoke.

The parser is not in pico. `parse_iso_literal` is a function over `&str`. Memoization sits above the parser: which files exist, which literals were extracted, which text was parsed.

The building blocks are `Database`, `Source`, `Memo`, and `Key`.

## Database

```rust
#[derive(Default, Db)]
struct IsographState {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
}
```

`IsographState` is the database. `handle` writes sources into it. Memos read from it. Tests construct the same type, intern sources the same way `handle` does, and call memos.

You cannot `set` or `remove` a source while a memo is running.

## Source

A source is an input fact. Disk contents and open editor buffers are sources. The config is a singleton source.

```rust
#[derive(Clone, PartialEq, Eq, Source)]
struct DiskFile {
    #[key]
    path: PathBuf,
    contents: String,
}

#[derive(Clone, PartialEq, Eq, Source)]
struct OpenFile {
    #[key]
    path: PathBuf,
    contents: String,
}
```

The `#[key]` field is the identity of the source. `TypeId` plus that field is the storage key. Two `DiskFile` values with the same path are the same source. `db.set` of a `DiskFile` with that path replaces the previous one.

If the new value `==` the old value, the epoch does not advance and dependents do not re-invoke.

`db.get(source_id)` returns the current value and records a read. `db.get` of a removed source panics. Presence is a separate fact: the tracked map of paths that currently have a `DiskFile`.

A path may have a `DiskFile`, an `OpenFile`, both, or neither. Artifact generation reads `DiskFile`. The LSP reads `OpenFile` when that path has one, otherwise `DiskFile`. Those are different memos. One overlay used by both, as isograph's `read_iso_literals_source` does, makes artifact generation depend on editor buffers.

A singleton has no key field. There is at most one. `CompilerConfig` and the working directory are singletons.

## Memo

```rust
#[memo]
fn extract_iso_literals_from_file_content(
    db: &IsographState,
    path: PathBuf,
) -> Option<Vec<IsoLiteralExtraction>>;
```

The first argument is `&Database`. The rest are the key. pico hashes the function identity plus those arguments. That tuple is the cache slot.

The body must be a pure function of `db` reads and the arguments. No filesystem, no clock, no LSP. Reading the world is `handle` interning a source.

What the body reads becomes a dependency. `db.get(source_id)` is a dependency on that source. Calling another `#[memo]` function is a dependency on that memo.

pico changes the written return type `T` to `&T`. The value lives in the database. `T` must be `PartialEq` so pico can compare a re-invocation to the stored value.

`#[memo(raw)]` returns `MemoRef<T>` instead of `&T`. Pass that `MemoRef` as an argument to another memo when the identity of the result is the key, not a copy of the value.

A memo that does not return every time it is called with the same key is a bug. Cycles panic.

## Key

A memo is looked up by the arguments you pass. There is no query that searches the database for "the parse of the literal under the cursor." The caller computes the arguments it already has, and calls the memo with those arguments.

Choose keys the call site has. Put the expensive work behind those keys. A coordinate the caller does not have is not a key of that work.

Arguments that are `SourceId<T>` or `MemoRef<T>` are identities already. Everything else (`PathBuf`, `usize`, `String`, interned names) is hashed and interned as a param.

## Iso literals

File text is a source. Extracting the literals in a file is a memo. Mapping a row and column onto an index in that list is a memo. Indexing one literal out of the list is a memo. Parsing that literal is a memo.

```rust
#[derive(Clone, PartialEq, Eq)]
struct IsoLiteralExtraction {
    iso_literal_text: String,
    iso_literal_start_index: usize,
    context: LiteralContext,
}

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
struct LineChar {
    line: u32,
    character: u32,
}

#[memo]
fn extract_iso_literals_from_file_content(
    db: &IsographState,
    path: PathBuf,
) -> Option<Vec<IsoLiteralExtraction>> {
    let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
    let contents = db.get(source_id).contents.as_str();
    Some(extract_iso_literals(contents))
}

#[memo]
fn iso_literal_index(
    db: &IsographState,
    path: PathBuf,
    line_char: LineChar,
) -> Option<usize> {
    let extractions = extract_iso_literals_from_file_content(db, path.clone())?;
    let source_id = db.get_disk_file_map().tracked().0.get(&path).copied()?;
    let content = db.get(source_id).contents.as_str();
    find_iso_literal_index(line_char, content, extractions)
}

#[memo]
fn iso_literal_extraction(
    db: &IsographState,
    path: PathBuf,
    index: usize,
) -> Option<IsoLiteralExtraction> {
    extract_iso_literals_from_file_content(db, path)?
        .get(index)
        .cloned()
}

#[memo]
fn parsed_iso_literal(db: &IsographState, iso_literal_text: String) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}

#[memo]
fn parsed_iso_literal_in_file(
    db: &IsographState,
    path: PathBuf,
    index: usize,
) -> Option<ParsedIsoLiteral> {
    let extraction = iso_literal_extraction(db, path, index)?;
    parsed_iso_literal(db, extraction.iso_literal_text.clone()).wrap_some()
}
```

`extract_iso_literals` (the host regex) is a plain function over `&str`. `parse_iso_literal` is a plain function over `&str`. `find_iso_literal_index` is a plain function over a cursor, file text, and the extract vec. The memos call them.

`None` from extract is no `DiskFile`. `Some(vec![])` is a present file with no literals. `None` from `iso_literal_index` is no file, or a cursor that is not inside any literal text (the JS around the literals, including `iso(`). `None` from `iso_literal_extraction` is no file, or `index` past the last extraction.

The parse memo is keyed on the literal text, not on the file, not on the index, not on the span in the file. Two files with the same iso text share a parse. Prefixing the file with JavaScript re-invokes extract (the start index moved) and does not re-invoke parse (the text did not).

`ParsedIsoLiteral` stores spans relative to the literal text. File-absolute spans are applied by a later memo that already has `iso_literal_start_index`. A file-absolute span in the parse result would make parse `!=` after a prepend, and dependents of parse would re-invoke even though the tree is the same.

isograph passes a file-absolute `TextSource` into the parse memo. Moving the literal around the file changes that argument, so the parse cache does not hit.

## Keys you already have

The compiler walks a file's extractions in order. It has `path` and `index`. File plus index is a key it can pass without searching. `iso_literal_extraction` and `parsed_iso_literal_in_file` take that pair.

The LSP has `path` and a cursor. `LineChar` is that cursor: `line` is a 0-based count of `\n`, `character` is bytes since the last `\n`. `iso_literal_index` is the memo whose arguments are those coordinates. Its result is `Option<usize>`. Hover, goto definition, and completion call that, then call `parsed_iso_literal_in_file` with the index.

Hover fires once per cursor. Each `(path, LineChar)` is its own memo slot, so moving the mouse across a literal executes `iso_literal_index` for every character. That is expected. The body is a walk over a handful of spans. The stored value is an index, or `None`.

That early step exists so the expensive memos can reuse. Every character inside the first literal yields `Some(0)`. `parsed_iso_literal_in_file(path, 0)` is one slot. pico hits it. Parse is the expensive work.

`iso_literal_index` does not return `IsoLiteralExtraction`. If it did, each cursor slot would store a clone of `iso_literal_text`. Hovering would copy the literal string once per character into the database. The index is `Copy`. The string lives on the extract-all memo and on the one `iso_literal_extraction(path, index)` slot.

The principle: the keys of each memo are coordinates the caller already has. A caller that has a cursor calls `iso_literal_index`. A caller that has an index calls `iso_literal_extraction`. A caller that has the literal text calls `parsed_iso_literal`. The high-cardinality memo (one slot per cursor) returns a small key. The large values sit behind that key.

After parse, the identity of a selectable is `(EntityName, SelectableName)`, which is in the AST. Schema memos are keyed on those names, not on files.

```rust
#[memo]
fn client_selectable_declaration(
    db: &IsographState,
    parent: EntityName,
    name: SelectableName,
) -> Option<SelectableDeclaration>
```

`User.Avatar` is that pair. Goto definition of a selection named `Avatar` already has the parent entity from resolve and the name from the token. That call site has names, not a file index.

## Equality and backdating

pico re-invokes a memo when a dependency's `time_updated` is newer than the last time this memo was verified. After re-invoke, it compares the new value to the stored value with `==`.

If they are equal, pico keeps the old `time_updated`. Dependents see no change and do not re-invoke. That is backdating.

```text
syntax highlighting  ->  parsed literals  ->  extract  ->  DiskFile
```

Typing JavaScript after the last iso literal re-invokes extract. If the `Vec<IsoLiteralExtraction>` is `==` (same texts, same start indices, same context), extract is backdated. Parse and syntax highlighting do not re-invoke.

Typing JavaScript before a literal changes `iso_literal_start_index`. Extract is `!=`. The file-absolute token memo re-invokes. `parsed_iso_literal` of the same text does not.

`==` decides whether dependents re-invoke. A memo result should be equal when the downstream work should be skipped. Spans that move with the file do not belong on a value whose dependents should survive a prepend. Presence of a diagnostic does belong, because diagnostics are the output.

`db.set` of a source with `==` contents does not advance the epoch. Re-saving an unchanged file does not re-invoke anything.

## Tracked maps

```rust
struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);
```

The map is which paths currently have a `DiskFile`. `db.set` of a `DiskFile` does not update the map. `handle` does both: intern the source, then insert into the map. Remove is the reverse.

`#[tracked]` on the field gives `get_disk_file_map()` / `get_disk_file_map_mut()`.

`tracked()` records a dependency on the whole map. Inserting or removing any path invalidates every memo that used `tracked()`. A memo that iterates every file (compile, "all client declarations") uses `tracked()`.

`untracked()` does not record the map. It is correct when the memo is already keyed by `path`, looks up that one entry, and then `db.get(source_id)` (which tracks the source). Adding an unrelated file must not re-invoke a per-file extract. Iterating `untracked()` is wrong: a newly inserted path is not seen.

Absence is the remaining case. A memo keyed by `path` that last time returned `None` did not `db.get` a source. If it also did not track the map, a later `Present` of that path is not seen and the memo stays `None`. Per-file memos that can miss a file therefore cannot be purely untracked. The intended dependency of a per-file memo is that file, not the set of all files.

## Intern

`db.intern_value(t)` stores `t` and returns `MemoRef<T>` whose identity is the hash of `t`. The same value interned twice is the same `MemoRef`. Looking up the `MemoRef` in a parent memo is a dependency on that interned node. If a producer re-interns an equal value, the `MemoRef` identity is unchanged and the parent does not re-invoke.

Pass a `MemoRef` as a memo argument when later work is "this declaration," not "whatever currently lives at this name." isograph interns a parsed field declaration and keys `add_client_scalar_selectable_to_entity` on `MemoRef<ClientFieldDeclaration>`.

`db.intern_ref(&t)` is the same identity idea for a value that already lives in another memo's result, without cloning it into a new allocation as the identity. The `MemoRef` still hashes the value, not the address.

## Parser and extract

The parser takes `&str`. Tests call `parse_iso_literal` with a string. Host extract takes `&str`. Tests call it with a string.

A caller that needs a parse tree calls the parse memo with the key it has. pico returns `&T`. The caller does not clone a whole file's AST out of the database to hand to another memo.

## Example

A file on disk:

```text
export const Avatar = iso(`
  field User.Avatar {
    name
  }
`)(function Avatar() { return null })
```

`handle` receives `DiskChanged` with those contents. It `set`s a `DiskFile` keyed by the path and inserts that `SourceId` into `disk_file_map`.

`extract_iso_literals_from_file_content(db, path)` reads that source, runs the host extract, and stores a one-element vec. Index `0` is the `field User.Avatar` literal. `iso_literal_text` is the interior. `iso_literal_start_index` is the byte offset of that interior in the file.

`parsed_iso_literal_in_file(db, path, 0)` loads extraction `0` and calls `parsed_iso_literal` with that text. The parse tree is the selectable declaration. Semantic tokens on that tree are relative to the interior.

The LSP asks for hover at a cursor on `name`. The adapter has `path` and a `LineChar`. `iso_literal_index` returns `Some(0)` and stores that `usize`. Moving the cursor along `name` executes `iso_literal_index` again with a new `LineChar`; it still returns `Some(0)`. `parsed_iso_literal_in_file(db, path, 0)` is one slot and does not re-invoke. Resolve uses the offset of that cursor within the literal. Schema hover for `User.name` is keyed by `(User, name)`, which resolve already has.

The user types `const x = 1;` at the top of the file. `handle` sets a new `DiskFile`. Extract re-invokes: same text, new `iso_literal_start_index`. `parsed_iso_literal` of that text does not re-invoke. File-absolute token offsets do.

The user types the same `field User.Avatar { name }` into a second file. `parsed_iso_literal` of that text is one slot. Both files' `parsed_iso_literal_in_file` entries share it.

The user adds a second iso literal at the top of the first file. Index `0` is now a different extraction. `parsed_iso_literal_in_file(db, path, 0)` is a different text. Index `1` is the old Avatar literal. Callers that still have "the first literal in the file" as their meaning of index `0` are walking the vec, so they enumerate again. Callers that meant Avatar by name are not using index `0` as the identity of Avatar; they are using `(User, Avatar)`.
