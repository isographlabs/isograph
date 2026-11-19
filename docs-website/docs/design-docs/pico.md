# Working with pico

pico is incremental memoization. The compiler state is a pico database. Facts from outside the process are sources. Everything derived from those facts is a memo.

A memo is a deterministic function of its arguments and of the sources and memos it reads while running. pico records those reads. When a source changes, pico re-invokes a memo only if a recorded read changed. If the new value `==` the old value, dependents do not re-invoke.

The parser is not in pico. `parse_iso_literal` is a function over `&str`. Memoization sits above the parser: which files exist, which literals were extracted, which text was parsed.

The original pico writeup used hover as the example:

```text
hover(fileA, row: 1, column: 1) -> parse(fileA) -> file_text(fileA)
hover(fileA, row: 1, column: 2) -> parse(fileA) -> file_text(fileA)
```

Two hovers reuse one parse. The cursor function may run twice. Parse does not. In i2 the parse is keyed by the literal text, not the file, and the cursor function is a plain function, not a memo (see Structuring functions and memos).

The building blocks are `Database`, `Source`, `Memo`, and `Key`.

## Database

```rust
#[derive(Debug, Db)]
struct IsographDb<TProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    #[tracked]
    source_path_map: SourcePathMap,
    #[tracked]
    schema_path_map: SchemaPathMap,
    phantom_data: PhantomData<TProfile>,
}
```

`IsographDb` is the database. `handle` writes sources into it. Memos read from it. Tests construct the same type, intern a `RelativePathToSourceFile`, `insert_disk_file`, and call memos. `handle` converts an event `PathBuf` first, then the same insert.

Memos take `&IsographDb<TProfile>`. `set` and `remove` take `&mut` and panic if a memo is on the stack. Derived data is a return value. A `&mut Entity` that you write selectables into cannot sit behind a memo. isograph had to delete `server_object_entity_mut` before those reads could be memos.

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

`db.set` stores the value in a `SourceNode` (`time_updated` plus `Option<Box<dyn DynEq>>`) in a `HashMap<Key, SourceNode>` and returns `SourceId<T>`. Inserting a new key and replacing a value with an unequal one each advance the epoch. That id is `Copy`: a `Key` plus `PhantomData<T>`. It is a type-level proof that a `T` exists in the database. `db.get(id)` takes that proof, looks up the `Key`, downcasts the node to `T`, records a read, and returns `&T`. There is no `Option`. A `SourceId<T>` whose source is missing is a false proof. pico panics. That panic is allowed.

The type system does not stop you from holding a `SourceId` after `remove`. `remove` keeps the key's `SourceNode`, `take`s its value, and stamps it with an incremented epoch. Dependents that read that source see it as changed (`source_node_changed_since` compares the node's newer `time_updated`). The next `get` of the stale id panics. A memo that returns a `SourceId` after the source was removed is handing out a false proof.

A path is not a proof. Looking up by path can miss and return `None`. `#[derive(Source)]` gives `DiskFile::id_for_key(&path)`, the `SourceId<DiskFile>` that `db.set` returns for a `DiskFile` with that path, computed whether or not one is set. `db.try_get(id)` returns `Option<&DiskFile>`. Call sites that have a path and not a `SourceId` intern a `RelativePathToSourceFile` and call `try_get(DiskFile::id_for_key(&path))`.

`get_singleton<T>()` returns `Option<&T>`. A singleton has no `SourceId` you hold as a proof of presence. You ask by type. The config may not have been set.

A path may have a `DiskFile`, one `OpenFile` per client that has it open, both, or neither. A read of file text through `file_contents(db, client, path)` reads client `c`'s `OpenFile` if present, otherwise `DiskFile`. The compiler is another client, `IsographState.compiler_client`.

A singleton has no key field. There is at most one. `CompilerConfig` and `CurrentWorkingDirectory` are singletons. `CurrentWorkingDirectory` is the parent directory of the config file, interned at process start. `handle` reads it to convert an event `PathBuf`. Memos do not. They receive `RelativePathToSourceFile` as an argument.

## Memo

```rust
#[memo]
pub fn extract_iso_literals<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<TProfile::HostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    TProfile::HostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}
```

pico rewrites the return to `&T`. First argument is `&Database`. There is no `&self`. `HostLanguage::extract_iso_literals_from_text(contents: &str)` is a plain associated function, not a memo; the memo is generic over the profile and calls the profile's host language.

The first argument is `&Database`. The rest are the key, at most eight of them. pico hashes the function identity (the signature text at expansion) plus those arguments. That tuple (`DerivedNodeDescriptor`) identifies the memo's derived node. A type parameter is not an argument. `IsographDb<TProfile>` is a different database type per profile, same as isograph `IsographDatabase<TCompilationProfile>`.

The body must be a pure function of `db` reads and the arguments. No filesystem, no clock, no LSP. Reading the world is `handle` interning a source.

What the body reads becomes a dependency. `db.get(source_id)` is a dependency on that source. Calling another `#[memo]` function from inside the body is a dependency on that memo. That is the edge pico records. A `MemoRef` passed in as an argument is a key. Reading its contents with `lookup` does not record a dependency; `lookup_tracked` does. isograph shipped a test where a parent took a `MemoRef`, used `lookup`, and kept a stale value when the pointed-to node changed.

pico changes the written return type `T` to `&T`. The value lives in the database. The caller does not clone it. `T` must be `PartialEq` so pico can compare a re-invocation to the stored value.

The result is owned. A memo cannot return a `&str` into a `DiskFile` or an `OpenFile`. The source can be replaced; the stored result has to survive that. `IsoLiteralExtraction.iso_literal_text` is a `String`. isograph made `IsoLiteralExtraction` owned for this reason.

`#[memo(raw)]` returns `MemoRef<T>` instead of `&T`. Pass that `MemoRef` as an argument to another memo when the identity of the result is the key, not a copy of the value.

A memo that does not return every time it is called with the same key is a bug. Cycles panic.

A memo that returns the whole project schema is one derived node. A change to one file invalidates it. `client_selectable_declaration(db, parent, name)` is keyed by the name the caller already has. isograph inlined a memo that returned a `Schema`.

## Key

A memo is looked up by the arguments you pass. There is no query that searches the database for "the parse of the literal under the cursor." The caller computes the arguments it already has, and calls the memo with those arguments.

Public APIs take one of:

- `client` and `path`
- `client`, `path`, and a position
- `EntityName`
- `EntityName` and `SelectableName`

The APIs that read file text take `client`: semantic tokens and file diagnostics take `client` and `path`; hover, goto definition, and completion take `client`, `path`, and a position. The hover chain takes `lsp_types::Position`. Entity lookup takes `EntityName`. Selectable lookup takes `EntityName` and `SelectableName`.

The literal text as a parse intern is how those public functions share a parse. A vec index of a literal in a file is not a public argument. Public APIs take `path` and a position. The first step converts that pair to `LiteralId` (the client, the path, and the 0-based index in that client's extract vec): `iso_literal_at_lsp_position`. Parse is interned on the literal text, not on `LiteralId`.

Arguments that are `SourceId<T>` or `MemoRef<T>` are identities already. They are `Copy`. pico never clones them. `RelativePathToSourceFile` is an interned string and `Copy`. Everything else (`String`, interned names) is hashed and interned as a param. A `PathBuf` param is the filesystem leak: pico would hash and clone an OS path on every execute.

An interned owned param is cloned into the param store the first time that value is seen, and cloned out of the param store on every execute of the memo body. A borrowed param (`&T`) is cloned once when interned, not on execute. pico's own tests pin this.

A memo result `T` is stored once in that derived node and returned as `&T`.

A value read from the database is not an argument to another function that has `db`. The argument is the identity the caller already has (`client` and `path`, `LiteralId`, `EntityName`). The inner function reads again.

Source text is `file_contents(db, client, path)`. A function that needs the file takes `client` and `path` and calls `file_contents`. `file_contents` is the only reader of `DiskFile.contents` and `OpenFile.contents`. The caller does not read the file and pass `contents`. Hover is `client`, `path`, and a position. Semantic tokens for a file is `client` and `path`. Neither takes the file text. `line_index` and `extract_iso_literals` read the file. `text_through_last_iso_literal` reads the file and stores bytes from the start through the last iso interior. `lsp_semantic_tokens_for_file` reads that memo, not the full file text. A helper they call that also takes `db` takes `client` and `path` and reads the file itself.

File text as a `String` memo argument is an interned owned param: a new intern per unique file, cloned on every execute. `path` is `Copy`. `db.get` records the source on the memo that uses the text. If the parent reads the file and passes the string, the child's key is the whole file (every edit is a new derived node) and the child does not record a dependency on `file_contents`.

The same for a memo result. The caller has the key. It calls the memo. It does not clone `&T` out of the database to pass as an argument. `iso_literal_extraction` takes `LiteralId` and reads extract. `iso_literal_at_lsp_position` returns `LiteralId`, not a cloned parse tree.

`parsed_iso_literal(iso_literal_text: String)` is keyed on the literal interior so two files share a parse. That string is the intern key, not file text forwarded from a parent. File text is never a memo argument.

A function with no `db` is not in pico. `parse_iso_literal` takes `&str`. `LineIndex::new` and `lsp_semantic_tokens` take `&str`. The memo that has `client` and `path` reads the file and calls that function.

## Iso literals

File text is a source. Extracting the literals in a file is a memo. The `LiteralId` at a position is a plain function. The extraction at a `LiteralId` is a memo. Parsing the extraction's text is a memo. There is no memo keyed on a position.

```text
extract_iso_literals(client, path)
  -> file_contents(client, path)
  + TProfile::HostLanguage::extract_iso_literals_from_text(&str)

iso_literal_at_lsp_position(client, path, Position)
  -> byte_offset_at_lsp_position(client, path, Position)
    -> line_index(client, path)
      -> file_contents(client, path)
  + extract_iso_literals(client, path)
  + the extraction whose interior contains the offset

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

fn iso_literal_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Span)> {
    let offset = byte_offset_at_lsp_position(db, client, path, position)?;
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .enumerate()
        .find_map(|(index, extraction)| {
            let within = offset.checked_sub(extraction.iso_literal_start_index.0 as u32)?;
            let character = extraction
                .iso_literal_text
                .get(within as usize..)?
                .chars()
                .next()?;
            (
                LiteralId {
                    client,
                    path,
                    index,
                },
                Span::new(within, within + character.len_utf8() as u32),
            )
                .wrap_some()
        })
}

#[memo]
fn iso_literal_extraction<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<TProfile::HostLanguage>> {
    let extractions = extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}

#[memo]
fn parsed_iso_literal(db: &IsographDb, iso_literal_text: String) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}
```

`HostLanguage::extract_iso_literals_from_text` is not a memo. The regex lives in its body. The memo `extract_iso_literals` reads `file_contents` and calls it. `parse_iso_literal` is a plain function over `&str` in the parser crate. `iso_literal_at_lsp_position` is a plain function over a position; it returns the vec index of the literal whose interior contains that position, and the span of the char under it relative to the interior. isograph moved `parse_iso_literal` out of the database crate so the parser would not know about `IsographDatabase`.

`None` from extract is no text for that client (no buffer and no `DiskFile`). `Some(vec![])` is text with no literals. `LiteralId` carries the client because its `index` is a position in that client's text. `None` from `iso_literal_at_lsp_position` is no file, a position past its line's end or past the end of the file, or a position that is not inside any literal text (the JS around the literals, including `iso(` and the closing backtick). `None` from `iso_literal_extraction` is no file, or `index` past the extract vec.

The parse memo is keyed on the literal text, not on the file, not on the span in the file, not on `LiteralId`. Two files with the same iso text share a parse.

Callers of `iso_literal_at_lsp_position` pass `path` and a position. The first work is turning that pair into a `LiteralId`. Nothing is stored per position. `iso_literal_extraction` of that `LiteralId` is one derived node per literal in the file. Callers short-circuit at `parsed_iso_literal(text)`: same string, parse does not re-run.

Work keyed by a position (finding the literal, hover, completion) is a plain function of the position, not a memo (Structuring functions and memos). It reads the `LiteralId`, then the extraction and parse.

Inserting a newline at the top of a one-line file moves the literal to the next line. `line_index` re-invokes and is `!=`. The `LiteralId` at the new position has the same bits (`index: 0`). `iso_literal_extraction` of that id is the same intern. `parsed_iso_literal` of that text does not re-invoke.

Bytes added on an earlier line, no extra newline, cursor on a later line: same position, extract is `!=` (`iso_literal_start_index` moved), and `iso_literal_at_lsp_position` returns an `==` `LiteralId` (`path` and `index` unchanged). `iso_literal_extraction` of that id re-invokes and is `!=`. `parsed_iso_literal` of that text does not re-invoke.

`ParsedIsoLiteral` stores spans relative to the literal text. Encoding takes each literal's `iso_literal_start_index` as an offset into the file. A file-absolute span in the parse result would make parse `!=` after a prepend, and dependents of parse would re-invoke even though the tree is the same.

isograph passes a file-absolute `TextSource` into the parse memo. Moving the literal around the file changes that argument, so the parse cache does not hit.

## Public keys and intern

A public function takes a public key. Hover is `client`, `path`, and a position. Semantic tokens for a file are `client` and `path`. An entity is `EntityName`. A selectable is `EntityName` and `SelectableName`.

```rust
// a plain function, not a memo
fn hover_for_position(
    db: &IsographDb,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover>;

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

Inside the hover chain, `iso_literal_at_lsp_position` takes `client`, `path`, and `lsp_types::Position` and turns them into a `LiteralId`. `parsed_iso_literal_by_id` takes the id and reads `iso_literal_extraction` and `parsed_iso_literal`. Semantic tokens for the file calls `lsp_semantic_tokens_for_file(client, path)`, which reads `parsed_iso_literals_in_file` and encodes with each extraction's start index.

Hover fires once per position. The functions keyed by position are plain functions, so moving the caret across a literal creates no derived node per character. `iso_literal_extraction` of a `LiteralId` is one derived node. Every character inside the same literal yields the same text, so `parsed_iso_literal` is one derived node.

After parse, resolve produces names. Goto definition of a selection named `Avatar` already has the parent entity and the name from the token. It calls `flattened_selectable_named(db, User, Avatar)`. That call site has names, not a file and cursor.

## Structuring functions and memos

These rules decide what each function takes, what it calls, and which functions are memos. They apply to every memo and every LSP handler.

### One step per function

Each function asks the layer below for exactly the one thing it needs, keyed the same way that layer is keyed, and makes one step. A function does not read text, extract literals, parse, and resolve in one body.

The hover chain follows.

```text
hover_for_position(db, client, path, position)
  -> resolved_position_at_lsp_position(db, client, path, position)
    -> ast_at_lsp_position(db, client, path, position)
      -> iso_literal_at_lsp_position(db, client, path, position)
        -> byte_offset_at_lsp_position(db, client, path, position)
          -> line_index(db, client, path)
            -> file_contents(db, client, path)
        -> extract_iso_literals(db, client, path)
          -> file_contents(db, client, path)
      -> parsed_iso_literal_by_id(db, literal_id)
        -> iso_literal_extraction(db, literal_id)
          -> extract_iso_literals(db, client, path)
        -> parsed_iso_literal(db, iso_literal_text)
```

`hover_for_position` receives a resolved node and writes its variant name. It does not receive file text, a byte offset, or an extraction. `line_index` and `extract_iso_literals` are the only functions in the chain that read text, and both read it through `file_contents`. Every function above them receives offsets, a `LiteralId`, a parse, or a resolved node.

```rust
// from crates/isograph_lsp/src/hover.rs
pub fn hover_for_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover> {
    let node = resolved_position_at_lsp_position(db, client, path, position)?;
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: node.to_string(),
        }),
        range: None,
    }
    .wrap_some()
}
```

A body that calls `file_contents`, then `TProfile::HostLanguage::extract_iso_literals_from_text`, then `parse_iso_literal`, then resolves, makes four steps in one function. Split it along the chain above.

### File text is read only through `file_contents`

```rust
// from crates/isograph_compiler/src/database.rs
#[memo]
pub fn file_contents<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String>
```

Nothing else reads `DiskFile.contents` or `OpenFile.contents`. `file_contents` does not read `disk_file_map` or `open_file_map`; it calls `try_get` of `DiskFile::id_for_key` and `OpenFile::id_for_key`. The readers of `disk_file_map`, `open_file_map`, `source_path_map`, and `schema_path_map` are the writers (`insert_disk_file`, `remove_disk_file`, `remove_disk_files_from_path`, `set_open_file`, `remove_open_file`, `remove_open_files_of_client`), and uses that do not read text, such as listing the paths that exist (`open_source_paths_of_client`). `source_path_map` and `schema_path_map` hold paths and `OpenFileKey`s, not `SourceId`s: a reader of a listed file's text calls `file_contents` for each listed path. `insert_disk_file` and `set_open_file` take the path's `FileMembership` (source file, schema file, both, or neither), which `handle` computes from the config's globs. The writers touch the two path maps only when membership changes, because `MutView::tracked` bumps the counter on every call. A function that lists paths and needs each file's text calls `file_contents` for each path.

### The client is an input

Every read of file text is keyed by `ClientId`, because two clients can hold different buffers for one path. Every LSP handler receives the client as its own parameter.

```rust
// from crates/isograph_cli/src/lsp_request_dispatch.rs
handler: fn(&TState, ClientId, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>

// from crates/isograph_cli/src/lsp_notification_dispatch.rs
handler: fn(&mut TState, ClientId, TNotification::Params) -> Vec<IsographEffect>

// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    params: lsp_types::SemanticTokensParams,
) -> LSPRuntimeResult<...>
```

The session loop in `lsp_socket.rs` tags each incoming message with the `ClientId` `handle` assigned to its connection when it processed that connection's `Started`. `LspRequest`, `LspNotification`, and `LspResponse` carry that `client` across the event channel only. `LSPRequestDispatch` and `LSPNotificationDispatch` take it off the payload and pass it to the handler.

A handler's output never contains a `ClientId`. `TRequest::Result` is the LSP result type. `dispatch_lsp_request` routes the reply by the client the request came in on: `live_sessions.reply(client, id)` returns `BeforeCompute`, `before_compute` returns `AfterCompute`, and `after_compute` sends on that client's `LiveSession.sender`.

### Memos are keyed by the narrowest input that determines the result

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn parsed_iso_literal<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    iso_literal_text: String,
) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}
```

The parse depends only on the literal's text, so the literal's text is the only key. The parse is not keyed by client, path, `LiteralId`, or file offset. Every span in `ParsedIsoLiteral` is relative to `iso_literal_text`. A caller that needs file positions adds the literal's `iso_literal_start_index`.

```rust
// from crates/isograph_compiler/src/host_language.rs (IsoLiteralExtraction::span, which format_iso_literals calls)
span::Span::from_usize(
    self.iso_literal_start_index.0,
    self.iso_literal_start_index.0 + self.iso_literal_text.len(),
)

// from crates/isograph_lsp/src/file_semantic_tokens.rs
literals.iter().map(|(extraction, parsed)| {
    (extraction.iso_literal_start_index, parsed.tokens.as_slice())
})
```

Diagnostics (planned, `refactors/pending/lsp-diagnostics.md`) shift each parse error's span by the same offset.

Typing above a literal changes `iso_literal_start_index` and not `iso_literal_text`, so `parsed_iso_literal` does not re-invoke. Identical literal text in two files, or in one path as two clients read it, is one parse.

### Memos return owned values; borrows are plain functions

pico stores a memo's owned `T` and returns `&T`. A memo cannot return a value that borrows from another memo's result. A function whose result borrows from a memo result is a plain function over memo outputs. `IsographResolutionNode<'a>` holds paths such as `IsoLiteralParsePath<'a>` (`PositionResolutionPath<&'a IsoLiteralParse, ()>`), which borrow the `IsoLiteralParse` inside the result of `parsed_iso_literal`. `resolved_position_at_lsp_position` returns an `IsographResolutionNode` and is therefore a plain function.

Choose memo boundaries deliberately. Each distinct set of memo arguments creates one derived node: a `DerivedNode` (the inner function and the result), its interned params, its recorded dependencies, and a `DerivedNodeRevision` (`time_verified` and `time_updated`). Derived nodes are retained until `run_garbage_collection`, which i2 never calls. A memo keyed by cursor position creates a derived node for every position the cursor visits, for the life of the process. Do not memoize per position unless there is a stated reason. In the hover chain, the functions that take `position` are plain functions, and `line_index(db, client, path)` is keyed by client and path.

### Keyed reads depend on their key, present or absent

A read of one key records a dependency on that key whether the key is present or absent, so a later insert or removal of that key re-invokes the reader. `tracked()` is for iterating the key set.

A keyed lookup is `db.try_get(T::id_for_key(&key))`. It does not read a tracked map.

```rust
// from crates/isograph_compiler/src/database.rs
match db.try_get(OpenFile::id_for_key(&OpenFileKey { client, path })) {
    Some(open_file) => open_file.contents.clone().wrap_some(),
    None => db
        .try_get(DiskFile::id_for_key(&path))
        .map(|disk_file| disk_file.contents.clone()),
}
```

`try_get` records a dependency on the key's `SourceNode` at that node's `time_updated`. A key that was never set records the initial epoch. `set` of that key inserts the node with an incremented epoch, and `remove` keeps the node, `take`s its value, and increments the epoch, so either one re-invokes the reader. A set or remove of another key does not.

### No `Default` for missing data

A missing key is an absent state, and readers depend on that absence. A read does not insert a default value for a key that is not there, and a type does not implement `Default` to stand in for "not written yet". `file_contents` returns `None` when there is no buffer and no `DiskFile`; it does not insert an empty `DiskFile`. Change 2 of `refactors/pending/pico-missing-key-dependency.md` removes `Default` from the `#[tracked]` counter for this reason.

### Outbound I/O state lives on `IsographState`

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

State that exists to perform outbound I/O (connections, reply channels, outstanding request ids, timer guards, and the planned last-published diagnostics per client) is a field on `IsographState`, not a pico source. Memos take `&IsographDb` and never read it. `IsographState` holds the database, so no memo can reach `IsographState`. Request handlers receive `&state.db`. Data that memos read (file text, open buffers) lives in the database, keyed by client.

### One line model

Every conversion between an LSP position and a byte offset goes through one `line_index`, which counts columns in UTF-16 code units and treats `\n`, `\r\n`, and `\r` as line breaks. Semantic tokens, formatting, hover, and diagnostics share it.

`LineIndex` (`crates/isograph_lsp/src/line_index.rs`) is owned. `line_index(db, client, path)` is a memo over `file_contents`, and hover converts positions through `byte_offset_at_lsp_position` over it. Semantic tokens and formatting build a `LineIndex` per call from `text_through_last_iso_literal`, so typing after the last literal does not re-invoke them.

## Equality and backdating

Each source has a `time_updated` epoch. Each derived node's `DerivedNodeRevision` has `time_verified` (last epoch we checked it) and `time_updated` (last epoch its value actually changed). The database epoch increments when a source is set at a new key, set to a different value, or removed.

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

A memo that reads one key does not use the map. It calls `db.try_get(T::id_for_key(&key))` (Keyed reads depend on their key, present or absent). Adding an unrelated file does not re-invoke a per-file `file_contents`. Iterating `untracked()` is wrong: a newly inserted path is not seen. `file_contents` is the one per-path lookup of text; every other memo that needs text calls it.

`open_file_map` is keyed by `OpenFileKey { client, path }`, the `#[key]` of `OpenFile`. `file_contents` reads `OpenFile::id_for_key` of that key, then `DiskFile::id_for_key` of the path on a miss. A keystroke re-invokes only the derived node that read that `OpenFile`. An open or close re-invokes only the `file_contents` derived nodes of that client and path. The maps exist for the writers and memos that iterate them (`remove_open_files_of_client`, `remove_disk_files_from_path`, `source_file_paths`), so `handle` still inserts into and removes from them.

A tracked read of a map whose counter was never written records a dependency on the counter's key at the initial epoch. The first counter write inserts that key and advances the epoch. On a fresh database, a `source_file_paths` call before the first insert sees that later insert.

## Intern

`db.intern_value(t)` stores `t` and returns `MemoRef<T>` whose identity is the hash of `t`. The same value interned twice is the same `MemoRef`. `MemoRef` is `Copy`. Looking up the `MemoRef` in a parent memo with `lookup_tracked` is a dependency on that interned node. `lookup` reads the value and does not record a dependency. If a producer re-interns an equal value, the `MemoRef` identity is unchanged and a parent keyed on that `MemoRef` does not re-invoke.

Pass a `MemoRef` as a memo argument when later work is "this declaration," not "whatever currently lives at this name." isograph interns a parsed field declaration and keys `add_client_scalar_selectable_to_entity` on `MemoRef<ClientFieldDeclaration>`.

`db.intern_ref(&t)` is the same identity idea for a value that already lives in another memo's result, without cloning it into a new allocation as the identity. The `MemoRef` hashes the value, not the address. If the same value is later interned from a new allocation (the producer re-ran), pico rewrites the pointer so lookup still works after garbage collection, and leaves `time_updated` at the first intern of that value so dependents can reuse.

`intern_value` and `intern_ref` of the same bits are different identities. pico wraps one of them before hashing so a value and a reference to that value do not collide.

## Garbage collection

Sources are not garbage collected. `handle` inserts and removes them.

Derived nodes accumulate until `run_garbage_collection`. i2 never calls `run_garbage_collection`, so every derived node created lives for the life of the process. A call of `run_garbage_collection` keeps the last 10_000 top-level memo calls (an LRU) and everything reachable from them, plus anything `retain`ed. A top-level call is a memo invoked when no other memo is on the stack. `iso_literal_extraction` of a `LiteralId` and `parsed_iso_literal` of a text stay if a recent top-level call still reaches them (a later hover in the same literal, or compile). If nothing reaches them, the next call recomputes them.

`retain` marks a top-level call so the LRU will not drop it. Compile can retain the validation memo. `RetainedQuery` panics if dropped without `clear_retain` or `never_garbage_collect`.

## Parser and extract

The parser takes `&str`. Tests call `parse_iso_literal` with a string. Extract is a memo that reads `file_contents` and calls `HostLanguage::extract_iso_literals_from_text`. Tests call `extract_iso_literals_from_text` with a string, or intern `"src/a.ts"`, `insert_disk_file` or `set_open_file`, and call the memo with a `ClientId`. They do not pass a `PathBuf`.

A caller that needs a parse tree calls the parse memo with the key it has. pico returns `&T`. The caller does not clone a whole file's AST out of the database to hand to another memo. That is the same as source text: the inner function has the key and reads; the parent does not pass a value it already read. `iso_literal_at_lsp_position` does not clone the AST either. It returns `LiteralId`.

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

`iso_literal_at_lsp_position` at a position on `name` returns `LiteralId { client, path, index: 0 }`. `iso_literal_extraction` of that id is the one extraction. `parsed_iso_literal` of its text is the selectable declaration. Semantic tokens on that tree are relative to the interior.

The LSP asks for hover at a position on `name`. The handler calls `hover_for_position(db, client, path, position)`. That is the public key. Inside, `iso_literal_at_lsp_position` converts the position to a byte offset through `line_index` and returns the id. Moving the caret along `name` runs the plain functions again with a new position; the `LiteralId` bits are the same. `iso_literal_extraction` of that id is one derived node. `parsed_iso_literal` of that text is one derived node and does not re-invoke. Resolve uses the offset of that position within the literal. Schema hover for `User.name` calls `flattened_selectable_named(db, User, name)`.

The user types `const x = 1; ` at the start of the first line (no extra newline). `handle` sets a new `DiskFile`. Extract re-invokes: same text, new `iso_literal_start_index`. The position of `name` is unchanged. `iso_literal_at_lsp_position` returns an `==` `LiteralId` (`index: 0`). `iso_literal_extraction` of that id re-invokes and is `!=`. `parsed_iso_literal` of that text does not re-invoke. Encoded file tokens change.

The user inserts a newline at the top of the file. The position of `name` moves to the next line. The `LiteralId` at the new position is still `{ client, path, index: 0 }`. `parsed_iso_literal` of that text does not re-invoke. Encoded file tokens change.

The user types the same `field User.Avatar { name }` into a second file. `parsed_iso_literal` of that text is one derived node. The two files have different `LiteralId` paths and share the parse intern.

The user adds a second iso literal at the top of the first file. Extract's vec has two elements. A position on Avatar still finds Avatar. Callers that meant Avatar by name are using `(User, Avatar)`.
