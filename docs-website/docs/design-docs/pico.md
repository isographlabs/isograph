# Working with pico

pico is incremental memoization. The compiler state is a pico database. Facts from outside the process are sources. Everything derived from those facts is a memo.

A memo is a deterministic function of its arguments and of the sources and memos it reads while running. pico records those reads. When a source changes, pico re-invokes a memo only if a recorded read changed. If the new value `==` the old value, dependents do not re-invoke.

The parser is not in pico. `parse_iso_literal` is a function over `&str`. Memoization sits above the parser: which files exist, which literals were extracted, which text was parsed.

The original pico writeup used hover as the example:

```text
hover(fileA, row: 1, column: 1) -> parse(fileA) -> file_text(fileA)
hover(fileA, row: 1, column: 2) -> parse(fileA) -> file_text(fileA)
```

Two hovers reuse one parse. The cursor memo may run twice. Parse does not.

The building blocks are `Database`, `Source`, `Memo`, and `Key`.

## Database

```rust
#[derive(Debug, Db)]
struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<THostLanguage>,
}
```

`IsographDb` is the database. `handle` writes sources into it. Memos read from it. Tests construct the same type, intern a `RelativePathToSourceFile`, `insert_disk_file`, and call memos. `handle` converts an event `PathBuf` first, then the same insert.

Memos take `&IsographDb<THostLanguage>`. `set` and `remove` take `&mut` and panic if a memo is on the stack. Derived data is a return value. A `&mut Entity` that you write selectables into cannot sit behind a memo. isograph had to delete `server_object_entity_mut` before those reads could be memos.

## Source

A source is an input fact. Disk contents and open editor buffers are sources. The config is a singleton source.

The inner model does not know about the filesystem. A `DiskFile` is a row in the database, not a path the compiler opens. The watcher, `isograph send`, and the editor adapter observe the OS and intern facts. Memos read those facts. They do not open a path, they do not call `PathBuf`, they do not know where the config file lives.

`PathBuf` is an OS path. It belongs on events and in the outer process. A `PathBuf` on a source key, a tracked map, or a memo argument is a leak of the filesystem into the inner model.

The identity of a source file is `RelativePathToSourceFile`: `string_key_newtype!`, an interned UTF-8 string, `Copy`. The string is the path relative to the directory that contains the config file. `"src/a.ts"` and `"src/./a.ts"` are two keys. Construction from a relative string is `"src/a.ts".intern().to()`. `handle` is the ingest seam: it converts `DiskFileChanged.path` or `FolderRemoved.path` (`PathBuf`, absolute) with `relative_path_from_absolute_and_working_directory` against the interned config directory, then `set`s or removes `DiskFile`s. Compiler tests intern `"src/a.ts"` and call `insert_disk_file`. They never construct a `PathBuf`.

```rust
string_key_newtype!(RelativePathToSourceFile);

#[derive(Clone, PartialEq, Eq, Source)]
struct DiskFile {
    #[key]
    path: RelativePathToSourceFile,
    contents: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct OpenFileKey {
    client: ClientId,
    path: RelativePathToSourceFile,
}

#[derive(Clone, PartialEq, Eq, Source)]
struct OpenFile {
    #[key]
    key: OpenFileKey,
    contents: String,
}
```

The `#[key]` field is the identity of the source. Under the hood that identity is a `Key`: the hash of `TypeId<T>` plus the `#[key]` field. Two `DiskFile` values with the same path are the same source. `db.set` of a `DiskFile` with that path replaces the previous one.

If the new value `==` the old value, the epoch does not advance and dependents do not re-invoke.

`db.set` stores the value in a `SourceNode` (`time_updated` plus `Box<dyn DynEq>`), records `Key -> index` in a map, and returns `SourceId<T>`. That id is `Copy`: a `Key` plus `PhantomData<T>`. It is a type-level proof that a `T` exists in the database. `db.get(id)` takes that proof, looks up the `Key`, downcasts the node to `T`, records a read, and returns `&T`. There is no `Option`. A `SourceId<T>` whose source is missing is a false proof. pico panics. That panic is allowed.

The type system does not stop you from holding a `SourceId` after `remove`. `remove` drops the map entry, `take`s the slot, and increments the epoch. Dependents that read that source see it as changed (`source_node_changed_since` is true when the node is gone). The next `get` of the stale id panics. A memo that returns a `SourceId` after the source was removed is handing out a false proof.

A path is not a proof. Looking up by path can miss and return `None`. The tracked map is `HashMap<RelativePathToSourceFile, SourceId<DiskFile>>`: `Option<SourceId<_>>` from the map, then `get` of the id. Call sites that have a path and not a `SourceId` intern a `RelativePathToSourceFile` and go through the map.

`get_singleton<T>()` returns `Option<&T>`. A singleton has no `SourceId` you hold as a proof of presence. You ask by type. The config may not have been set.

A path may have a `DiskFile`, one `OpenFile` per client that has it open, both, or neither. A read of file text through `file_contents(db, client, path)` reads client `c`'s `OpenFile` if present, otherwise `DiskFile`. The file-based compiler will be another client with its own `ClientId`.

A singleton has no key field. There is at most one. `CompilerConfig` and `CurrentWorkingDirectory` are singletons. `CurrentWorkingDirectory` is the parent directory of the config file, interned at process start. `handle` reads it to convert an event `PathBuf`. Memos do not. They receive `RelativePathToSourceFile` as an argument.

## Memo

```rust
#[memo]
pub fn extract_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    THostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}
```

pico rewrites the return to `&T`. First argument is `&Database`. There is no `&self`. `HostLanguage::extract_iso_literals_from_text(contents: &str)` is a plain associated function, not a memo; the memo is generic over the host.

The first argument is `&Database`. The rest are the key, at most eight of them. pico hashes the function identity (the signature text at expansion) plus those arguments. That tuple is the cache slot. A type parameter is not an argument. `IsographDb<THostLanguage>` is a different database type per host, same as isograph `IsographDatabase<TCompilationProfile>`.

The body must be a pure function of `db` reads and the arguments. No filesystem, no clock, no LSP. Reading the world is `handle` interning a source.

What the body reads becomes a dependency. `db.get(source_id)` is a dependency on that source. Calling another `#[memo]` function from inside the body is a dependency on that memo. That is the edge pico records. A `MemoRef` passed in as an argument is a key. Reading its contents with `lookup` does not record a dependency; `lookup_tracked` does. isograph shipped a test where a parent took a `MemoRef`, used `lookup`, and kept a stale value when the pointed-to node changed.

pico changes the written return type `T` to `&T`. The value lives in the database. The caller does not clone it. `T` must be `PartialEq` so pico can compare a re-invocation to the stored value.

The result is owned. A memo cannot return a `&str` into a `DiskFile` or an `OpenFile`. The source can be replaced; the stored result has to survive that. `IsoLiteralExtraction.iso_literal_text` is a `String`. isograph made `IsoLiteralExtraction` owned for this reason.

`#[memo(raw)]` returns `MemoRef<T>` instead of `&T`. Pass that `MemoRef` as an argument to another memo when the identity of the result is the key, not a copy of the value.

A memo that does not return every time it is called with the same key is a bug. Cycles panic.

A memo that returns the whole project schema is one slot. A change to one file invalidates it. `client_selectable_declaration(db, parent, name)` is keyed by the name the caller already has. isograph inlined a memo that returned a `Schema`.

## Key

A memo is looked up by the arguments you pass. There is no query that searches the database for "the parse of the literal under the cursor." The caller computes the arguments it already has, and calls the memo with those arguments.

Public APIs take one of:

- `client` and `path`
- `client`, `path`, and `LineChar`
- `EntityName`
- `EntityName` and `SelectableName`

The APIs that read file text take `client`: semantic tokens and file diagnostics take `client` and `path`; hover, goto definition, and completion take `client`, `path`, and `LineChar`. Entity lookup takes `EntityName`. Selectable lookup takes `EntityName` and `SelectableName`.

The literal text as a parse intern is how those public functions share a parse. A vec index of a literal in a file is not a public argument. Public APIs take `path` and `LineChar`. The first intern converts that pair to `LiteralId` (the client, the path, and the 0-based index in that client's extract vec). Parse is interned on the literal text, not on `LiteralId`.

Arguments that are `SourceId<T>` or `MemoRef<T>` are identities already. They are `Copy`. pico never clones them. `RelativePathToSourceFile` is an interned string and `Copy`. Everything else (`String`, interned names) is hashed and interned as a param. A `PathBuf` param is the filesystem leak: pico would hash and clone an OS path on every execute.

An interned owned param is cloned into the param store the first time that value is seen, and cloned out of the param store on every execute of the memo body. A borrowed param (`&T`) is cloned once when interned, not on execute. pico's own tests pin this.

A memo result `T` is stored once in that slot and returned as `&T`.

A value read from the database is not an argument to another function that has `db`. The argument is the identity the caller already has (`client` and `path`, `LiteralId`, `EntityName`). The inner function reads again.

Source text is `file_contents(db, client, path)`. A function that needs the file takes `client` and `path` and calls `file_contents`. `file_contents` is the only reader of `DiskFile.contents` and `OpenFile.contents`. The caller does not read the file and pass `contents`. Hover is `client`, `path`, and `LineChar`. Semantic tokens for a file is `client` and `path`. Neither takes the file text. `literal_id_at_location` reads the file. `text_through_last_iso_literal` reads the file and stores bytes from the start through the last iso interior. `lsp_semantic_tokens_for_file` reads that memo, not the full file text. A helper they call that also takes `db` takes `client` and `path` and reads the file itself.

File text as a `String` memo argument is an interned owned param: a new intern per unique file, cloned on every execute. `path` is `Copy`. `db.get` records the source on the memo that uses the text. If the parent reads the file and passes the string, the child's key is the whole file (every edit is a new slot) and the child does not record a dependency on `file_contents`.

The same for a memo result. The caller has the key. It calls the memo. It does not clone `&T` out of the database to pass as an argument. `iso_literal_extraction` takes `LiteralId` and reads extract. A `(path, LineChar)` memo stores `LiteralId`, not a cloned parse tree.

`parsed_iso_literal(iso_literal_text: String)` is keyed on the literal interior so two files share a parse. That string is the intern key, not file text forwarded from a parent. File text is never a memo argument.

A function with no `db` is not in pico. `parse_iso_literal` takes `&str`. `find_iso_literal_index` takes a `LineChar`, file text, and the extract vec. `lsp_semantic_tokens` takes `&str`. The memo that has `client` and `path` reads the file and calls that function.

## Iso literals

File text is a source. Extracting the literals in a file is a memo. The `LiteralId` at a `LineChar` is a memo. The extraction at a `LiteralId` is a memo. Parsing the extraction's text is a memo. There is no parse-tree memo keyed on `LineChar`.

```text
extract_iso_literals(client, path)
  -> file_contents(client, path)
  + THostLanguage::extract_iso_literals_from_text(&str)

literal_id_at_location(client, path, LineChar)
  -> extract_iso_literals(client, path)
  + find_iso_literal_index(LineChar, file_contents(client, path), extract vec)

iso_literal_extraction(LiteralId)
  -> extract_iso_literals(client, path)
  + vec[index]

parsed_iso_literal(text)
  -> parse_iso_literal(&str)
```

Semantic tokens for a file (file-semantic-tokens.md):

```text
lsp_semantic_tokens_for_file(client, path)
  -> parsed_iso_literals_in_file(client, path)
  + text_through_last_iso_literal(client, path)

parsed_iso_literals_in_file(client, path)
  -> extract_iso_literals(client, path)
  + parsed_iso_literal(text) for each extraction

text_through_last_iso_literal(client, path)
  -> extract_iso_literals(client, path)
  + file_contents(client, path)
```

Formatting and `willSaveWaitUntil` (`format_iso_literals(db, client, path)`) read the same two memos.

Each item is `(IsoLiteralExtraction, ParsedIsoLiteral)`. Document-scoped LSP methods need both. There is no intern of trees without extractions.

```rust
struct IsoLiteralStartIndex(pub usize);

#[derive(Clone, PartialEq, Eq)]
struct IsoLiteralExtraction {
    iso_literal_text: String,
    iso_literal_start_index: IsoLiteralStartIndex,
    context: LiteralContext,
}

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
struct LineChar {
    line: u32,
    character: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct LiteralId {
    client: ClientId,
    path: RelativePathToSourceFile,
    index: usize,
}

impl HostLanguage for TypeScriptHostLanguage {
    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                IsoLiteralExtraction {
                    iso_literal_text: literal.as_str().to_owned(),
                    iso_literal_start_index: IsoLiteralStartIndex(literal.start()),
                    context: TypeScriptLiteralContext { /* export_name, call, associated */ },
                }
                .wrap_some()
            })
            .collect()
    }
}

#[memo]
fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<LiteralId> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    let file_content = file_contents(db, client, path).as_ref()?;
    let index = find_iso_literal_index(line_char, file_content, extractions)?;
    LiteralId {
        client,
        path,
        index,
    }
    .wrap_some()
}

#[memo]
fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}

#[memo]
fn parsed_iso_literal(db: &IsographDb, iso_literal_text: String) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}
```

`HostLanguage::extract_iso_literals_from_text` is not a memo. The regex lives in its body. The memo `extract_iso_literals` reads `file_contents` and calls it. `parse_iso_literal` is a plain function over `&str` in the parser crate. `find_iso_literal_index` is a plain function over a `LineChar`, file text, and the extract vec; it returns the vec index. isograph moved `parse_iso_literal` out of the database crate so the parser would not know about `IsographDatabase`.

`None` from extract is no text for that client (no buffer and no `DiskFile`). `Some(vec![])` is text with no literals. `LiteralId` carries the client because its `index` is a position in that client's text. `None` from `literal_id_at_location` is no file, or a `LineChar` that is not inside any literal text (the JS around the literals, including `iso(` and the closing backtick). `None` from `iso_literal_extraction` is no file, or `index` past the extract vec.

The parse memo is keyed on the literal text, not on the file, not on the span in the file, not on `LiteralId`. Two files with the same iso text share a parse.

Public callers pass `path` and `LineChar`. Those memos re-invoke. The first work is turning that pair into a `LiteralId`. The stored value per `LineChar` is a path and a vec index, not an extraction and not a parse tree. `iso_literal_extraction` of that `LiteralId` is one slot per literal in the file. They short-circuit at `parsed_iso_literal(text)`: same string, parse does not re-run.

A `(path, LineChar)` memo must not store a parse tree, an extraction, or other per-literal data that is the same at every position inside that literal. Store `LiteralId`. Work that can differ at every `LineChar` (hover, completion) is a later memo on `(path, LineChar)` whose body reads `LiteralId` and then the extraction and parse.

Inserting a newline at the top of a one-line file is a new `LineChar` and a new `literal_id_at_location` slot. The `LiteralId` bits are the same (`index: 0`). `iso_literal_extraction` of that id is the same intern. `parsed_iso_literal` of that text does not re-invoke.

Bytes added on an earlier line, no extra newline, cursor on a later line: same `LineChar`, extract is `!=` (`iso_literal_start_index` moved), `literal_id_at_location` re-invokes, `LiteralId` is `==` (`path` and `index` unchanged) and backdates. `iso_literal_extraction` of that id re-invokes and is `!=`. `parsed_iso_literal` of that text does not re-invoke.

`ParsedIsoLiteral` stores spans relative to the literal text. Encoding takes each literal's `iso_literal_start_index` as an offset into the file. A file-absolute span in the parse result would make parse `!=` after a prepend, and dependents of parse would re-invoke even though the tree is the same.

isograph passes a file-absolute `TextSource` into the parse memo. Moving the literal around the file changes that argument, so the parse cache does not hit.

## Public keys and intern

A public function takes a public key. Hover is `client`, `path`, and `LineChar`. Semantic tokens for a file are `client` and `path`. An entity is `EntityName`. A selectable is `EntityName` and `SelectableName`.

```rust
#[memo]
fn hover(
    db: &IsographDb,
    client: ClientId,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<Hover>;

#[memo]
fn lsp_semantic_tokens_for_file(
    db: &IsographDb,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>>;

#[memo]
fn flattened_entity_named(
    db: &IsographDb,
    entity_name: EntityName,
) -> Option<MemoRef<Entity>>;

#[memo]
fn flattened_selectable_named(
    db: &IsographDb,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Option<MemoRef<Selectable>>;
```

`LineChar` is the cursor: `line` is a 0-based count of `\n`, `character` is bytes since the last `\n`. The adapter has that pair.

Inside hover, `literal_id_at_location` takes `client`, `path`, and `LineChar` and turns them into a `LiteralId`. `iso_literal_extraction` takes the id. `parsed_iso_literal` takes the extraction's text. Semantic tokens for the file calls `lsp_semantic_tokens_for_file(client, path)`, which reads `parsed_iso_literals_in_file` and encodes with each extraction's start index.

Hover fires once per `LineChar`. Each `(client, path, LineChar)` is its own `literal_id_at_location` slot, so moving the caret across a literal executes that intern for every character. That is expected. The stored value is a `ClientId`, a `RelativePathToSourceFile`, and a vec index. `iso_literal_extraction` of that `LiteralId` is one slot. Every character inside the same literal yields the same text, so `parsed_iso_literal` is one slot.

After parse, resolve produces names. Goto definition of a selection named `Avatar` already has the parent entity and the name from the token. It calls `flattened_selectable_named(db, User, Avatar)`. That call site has names, not a file and cursor.

## Equality and backdating

Each source has a `time_updated` epoch. Each memo slot has `time_verified` (last epoch we checked it) and `time_updated` (last epoch its value actually changed). The database epoch increments when a source is set to a different value.

pico re-invokes a memo when a dependency's `time_updated` is newer than this memo's last `time_verified`. After re-invoke, it compares the new value to the stored value with `==`.

If they are equal, pico keeps the old `time_updated`. Dependents see no change and do not re-invoke. That is backdating.

```text
syntax highlighting  ->  lsp_semantic_tokens_for_file  ->  parsed_iso_literals_in_file + text_through_last_iso_literal  ->  extract  ->  file_contents  ->  OpenFile or DiskFile
```

Typing JavaScript after the last iso literal re-invokes extract. If the `Vec<IsoLiteralExtraction>` is `==` (same texts, same start indices, same context), extract is backdated. `parsed_iso_literals_in_file` does not re-invoke. `parsed_iso_literal` of the same text does not. `text_through_last_iso_literal` re-invokes because the file text changed, then `==` and backdates. Encoded tokens do not re-invoke.

Typing JavaScript before a literal changes `iso_literal_start_index`. Extract is `!=`. `parsed_iso_literals_in_file` is `!=` (the extraction moved). Encoded file tokens change (`delta_line` / `delta_start`). The tree is relative and `==`. `parsed_iso_literal` of the same text does not re-invoke.

If this memo was already verified in the current epoch, pico returns the stored value without walking dependencies. Two LSP requests in the same epoch (hover and semantic tokens, no edit between them) share extract and parse this way.

`==` decides whether dependents re-invoke. A memo result should be equal when the downstream work should be skipped. `parsed_iso_literal` stores spans relative to the literal text, so a prepend does not make parse `!=`. `parsed_iso_literals_in_file` includes the extraction, so a prepend makes it `!=` and file-scoped dependents re-invoke. Presence of a diagnostic does belong, because diagnostics are the output.

`db.set` of a source with `==` contents does not advance the epoch. Re-saving an unchanged file does not re-invoke anything.

## Tracked maps

```rust
struct DiskFileMap(pub HashMap<RelativePathToSourceFile, SourceId<DiskFile>>);
```

The map is which paths currently have a `DiskFile`. `db.set` of a `DiskFile` does not update the map. `handle` does both: intern the source, then insert into the map. Remove is the reverse.

`#[tracked]` on the field gives `get_disk_file_map()` / `get_disk_file_map_mut()`.

`tracked()` records a dependency on the whole map. Inserting or removing any path invalidates every memo that used `tracked()`. A memo that iterates every file (compile, "all client declarations") uses `tracked()`.

`untracked()` does not record the map. It is correct when the memo is already keyed by `path`, looks up that one `SourceId`, and then `db.get(source_id)` (which tracks the source). Adding an unrelated file must not re-invoke a per-file `file_contents`. Iterating `untracked()` is wrong: a newly inserted path is not seen. `file_contents` is the one per-path lookup of text; every other memo that needs text calls it.

A lookup that misses did not `db.get`. If it also did not `tracked()` the map, a later `Present` of that path is not seen and the memo stays `None`. isograph hit this: an autofix created a file, `get_iso_literal` ran untracked, returned `None`, and the next request reused that `None` and panicked. On miss, `tracked()` the map so a later insert of that key is seen. The intended dependency of a per-file memo that hits is that file, not the set of all files.

`open_file_map` is keyed by `OpenFileKey { client, path }`. `file_contents` looks up untracked and `db.get`s the hit; on a miss it reads the map tracked. A write of a key already present does not write the map, so a keystroke re-invokes only the slot that read that `OpenFile`. Any open or close re-invokes every open-miss slot of every client; each backdates when its text is `==`.

A tracked read of a map whose counter was never written records no dependency (`refactors/pending/pico-missing-key-dependency.md`). On a fresh database, a `file_contents` miss before the first `set_open_file` does not see that later open.

## Intern

`db.intern_value(t)` stores `t` and returns `MemoRef<T>` whose identity is the hash of `t`. The same value interned twice is the same `MemoRef`. `MemoRef` is `Copy`. Looking up the `MemoRef` in a parent memo with `lookup_tracked` is a dependency on that interned node. `lookup` reads the value and does not record a dependency. If a producer re-interns an equal value, the `MemoRef` identity is unchanged and a parent keyed on that `MemoRef` does not re-invoke.

Pass a `MemoRef` as a memo argument when later work is "this declaration," not "whatever currently lives at this name." isograph interns a parsed field declaration and keys `add_client_scalar_selectable_to_entity` on `MemoRef<ClientFieldDeclaration>`.

`db.intern_ref(&t)` is the same identity idea for a value that already lives in another memo's result, without cloning it into a new allocation as the identity. The `MemoRef` hashes the value, not the address. If the same value is later interned from a new allocation (the producer re-ran), pico rewrites the pointer so lookup still works after garbage collection, and leaves `time_updated` at the first intern of that value so dependents can reuse.

`intern_value` and `intern_ref` of the same bits are different identities. pico wraps one of them before hashing so a value and a reference to that value do not collide.

## Garbage collection

Sources are not garbage collected. `handle` inserts and removes them.

Memo slots accumulate until `run_garbage_collection`. That keeps the last 10_000 top-level memo calls (an LRU) and everything reachable from them, plus anything `retain`ed. A top-level call is a memo invoked when no other memo is on the stack. LSP hover is top-level. Old `(path, LineChar)` slots drop. `iso_literal_extraction` of a `LiteralId` and `parsed_iso_literal` of a text stay if a recent top-level call still reaches them (a later hover in the same literal, or compile). If nothing reaches them, the next call recomputes them.

`retain` marks a top-level call so the LRU will not drop it. Compile can retain the validation memo. `RetainedQuery` panics if dropped without `clear_retain` or `never_garbage_collect`.

## Parser and extract

The parser takes `&str`. Tests call `parse_iso_literal` with a string. Extract is a memo that reads `file_contents` and calls `HostLanguage::extract_iso_literals_from_text`. Tests call `extract_iso_literals_from_text` with a string, or intern `"src/a.ts"`, `insert_disk_file` or `set_open_file`, and call the memo with a `ClientId`. They do not pass a `PathBuf`.

A caller that needs a parse tree calls the parse memo with the key it has. pico returns `&T`. The caller does not clone a whole file's AST out of the database to hand to another memo. That is the same as source text: the inner function has the key and reads; the parent does not pass a value it already read. A `(path, LineChar)` memo does not clone the AST into that slot either. It stores `LiteralId`.

## Example

A file on disk:

```text
export const Avatar = iso(`
  field User.Avatar {
    name
  }
`)(function Avatar() { return null })
```

`handle` receives `DiskChanged` with those contents and an absolute `PathBuf`. It converts that path to `RelativePathToSourceFile` against the interned config directory, `set`s a `DiskFile` keyed by that interned path, and inserts the `SourceId` into `disk_file_map`. Extract and hover take the interned path. They never see the `PathBuf`.

`extract_iso_literals(db, client, path)` reads `file_contents(db, client, path)` (that `DiskFile`, for a client with no buffer), runs the host regex, and stores a one-element vec. `iso_literal_text` is the interior of `field User.Avatar`. `iso_literal_start_index` is the byte offset of that interior in the file.

`literal_id_at_location` at a `LineChar` on `name` returns `LiteralId { client, path, index: 0 }`. `iso_literal_extraction` of that id is the one extraction. `parsed_iso_literal` of its text is the selectable declaration. Semantic tokens on that tree are relative to the interior.

The LSP asks for hover at a `LineChar` on `name`. The adapter calls `hover(db, client, path, line_char)`. That is the public key. Inside, `literal_id_at_location` returns the id. Moving the caret along `name` executes `literal_id_at_location` again with a new `LineChar`; the `LiteralId` bits are the same. `iso_literal_extraction` of that id is one slot. `parsed_iso_literal` of that text is one slot and does not re-invoke. Resolve uses the offset of that `LineChar` within the literal. Schema hover for `User.name` calls `flattened_selectable_named(db, User, name)`.

The user types `const x = 1; ` at the start of the first line (no extra newline). `handle` sets a new `DiskFile`. Extract re-invokes: same text, new `iso_literal_start_index`. The `LineChar` of `name` is unchanged. `literal_id_at_location` re-invokes, `LiteralId` is `==` (`index: 0`) and backdates. `iso_literal_extraction` of that id re-invokes and is `!=`. `parsed_iso_literal` of that text does not re-invoke. Encoded file tokens change.

The user inserts a newline at the top of the file. The `LineChar` of `name` moves. That is a new `literal_id_at_location` slot. The `LiteralId` is still `{ client, path, index: 0 }`. `parsed_iso_literal` of that text does not re-invoke. Encoded file tokens change.

The user types the same `field User.Avatar { name }` into a second file. `parsed_iso_literal` of that text is one slot. The two files have different `LiteralId` paths and share the parse intern.

The user adds a second iso literal at the top of the first file. Extract's vec has two elements. A `LineChar` on Avatar still finds Avatar. Callers that meant Avatar by name are using `(User, Avatar)`.
