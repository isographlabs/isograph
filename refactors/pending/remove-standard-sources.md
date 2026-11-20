# Remove `StandardSources` (upstream isograph)

This doc plans an upstream isograph PR. It follows upstream #1003 (`9d72f6fc`, a missed key lookup is a dependency) and #1024 (`9b81e0b6`, iso literal paths tracked separately from file contents). Upstream paths are written `isograph/<path>:<line>` and were read at `9b81e0b6`.

i2 has no `StandardSources`. i2's counterpart is `SchemaPathMap`, and this change does not apply to it (section "i2").

## What we are building

`IsographDatabase` loses its `#[tracked] standard_sources` field. A `SchemaSource` is still `db.set` by the watcher and at startup, keyed by its `RelativePathToSourceFile`. Every reader computes the source's id from a path with `SchemaSource::id_for_key(&path)` and reads it with `db.try_get`. The list of schema paths comes from the `CompilerConfig` singleton (`schema` and `schema_extensions`), which every reader already depends on through `get_isograph_config`.

The config names exact paths, not globs (`isograph/crates/isograph_config/src/compilation_options.rs:24-27`), so the config already holds the set of expected schema paths. `StandardSources` duplicates it as `SourceId`s, and since #1003 a `SourceId<SchemaSource>` is computable from the path.

Dependencies a reader records after this change:

- `parse_graphql_schema` depends on the `CompilerConfig` singleton key, on the main schema's `SchemaSource` key, and on one `parse_schema_extensions_file(db, path)` memo per configured extension path.
- `parse_schema_extensions_file(db, path)` depends on that one `SchemaSource` key.
- A `db.set` or `db.remove` of one extension re-invokes only that extension's `parse_schema_extensions_file`. `parse_graphql_schema` re-invokes only if that memo's value is `!=`. Today every extension write calls `get_standard_sources_mut().tracked()` (`isograph/crates/isograph_compiler/src/source_files.rs:128-131`), which bumps the counter that `parse_graphql_schema` reads (`isograph/crates/graphql_network_protocol/src/read_schema.rs:30-34`), so `parse_graphql_schema` re-invokes on every extension write.
- A missing key records a dependency at the initial epoch (#1003, `isograph/crates/pico/src/database.rs:137-147`). A later `db.set` of that path advances the epoch and re-invokes the reader.

## End-user experience

`isograph compile`, `isograph watch`, and the LSP behave as today, with two differences:

- In watch mode and the LSP, an edit to one schema extension re-parses that extension only. Today it also re-runs `parse_graphql_schema`'s loop over every extension (each re-parse is a memo hit).
- Under open decision 1 (b), goto definition on a server field, an entity, or a client field name defined in a schema extension or an iso literal lands at the right position. Today all five sites compute the position against the main schema's text (`isograph/crates/isograph_lsp/src/goto_definition.rs:89`, `:136`, `:186`, `:205`, `:238`).

A missing schema file behaves as today (open decision 2): at startup `create_config` panics on `canonicalize` (`isograph/crates/isograph_config/src/compilation_options.rs:194-205`); after a remove, `update_sources` returns "Schema not found. Cannot proceed without a schema" and `handle_watch_command` (`isograph/crates/isograph_compiler/src/watch.rs:58`) and the LSP loop (`isograph/crates/isograph_lsp/src/server.rs:197`) return it with `?`. A removed extension is dropped silently (open decision 3).

## Changes

### 1. `isograph_schema`: drop `StandardSources`, add keyed accessors

Before (`isograph/crates/isograph_schema/src/isograph_database.rs:19-94`):

```rust
// external isograph/crates/isograph_schema/src/isograph_database.rs
#[derive(Default, Debug, Db)]
pub struct IsographDatabase<TCompilationProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    iso_literals: HashSet<RelativePathToSourceFile>,
    #[tracked]
    standard_sources: StandardSources,
    phantom_data: PhantomData<TCompilationProfile>,
}

#[derive(Debug, Clone, Default)]
pub struct StandardSources {
    // TODO we should just store this as a singleton, and not have it in standard sources.
    // Or perhaps, we store the schema source directly here.
    pub schema_source_id: SourceId<SchemaSource>,
    pub schema_extension_sources: BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
}

    pub fn get_schema_source(&self) -> &SchemaSource {
        self.get(self.standard_sources.schema_source_id)
    }

    pub fn remove_schema_extension(
        &mut self,
        relative_path: RelativePathToSourceFile,
    ) -> Option<SourceId<SchemaSource>> {
        self.get_standard_sources_mut()
            .tracked()
            .schema_extension_sources
            .remove(&relative_path)
            .inspect(|&source_id| self.remove(source_id))
    }
```

After:

```rust
// external isograph/crates/isograph_schema/src/isograph_database.rs
#[derive(Default, Debug, Db)]
pub struct IsographDatabase<TCompilationProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    iso_literals: HashSet<RelativePathToSourceFile>,
    phantom_data: PhantomData<TCompilationProfile>,
}

    pub fn get_schema_source(
        &self,
        relative_path: RelativePathToSourceFile,
    ) -> Option<&SchemaSource> {
        self.try_get(SchemaSource::id_for_key(&relative_path))
    }

    pub fn insert_schema_source(&mut self, relative_path: RelativePathToSourceFile, content: String) {
        self.set(SchemaSource {
            relative_path,
            content,
            text_source: TextSource {
                relative_path_to_source_file: relative_path,
                span: None,
            },
        });
    }

    pub fn remove_schema_source(&mut self, relative_path: RelativePathToSourceFile) {
        self.remove(SchemaSource::id_for_key(&relative_path));
    }
```

`SchemaSource` (`:29-36`) is unchanged. The names follow #1024's `get_iso_literal` / `insert_iso_literal` / `remove_iso_literal` (`:96-124`). `remove_schema_source` returns nothing: `remove_schema_extension`'s `Option<SourceId<SchemaSource>>` has no reader (`source_files.rs:107`, `:115` discard it). `pico::remove_source` on a key with no value is a no-op (`isograph/crates/pico/src/database.rs:333-339`). `BTreeMap` leaves the `use` at `:2`; `SourceId` stays for `remove_iso_literal` (`:116`).

`file_text_at_location`, before (`:232-266`, the schema part):

```rust
// external isograph/crates/isograph_schema/src/isograph_database.rs
    let standard_sources = db.get_standard_sources();
    let standard_sources = standard_sources.tracked();
    let config = db.get_isograph_config();

    // Then schema source
    if relative_path == config.schema.relative_path {
        return db
            .get(standard_sources.schema_source_id)
            .content
            .clone()
            .wrap_some();
    }

    // Is it a schema extension? Maybe! Try that.
    return db
        .standard_sources
        .schema_extension_sources
        .get(&relative_path)
        .map(|extension_source| db.get(extension_source.dereference()).content.clone());
}
```

After:

```rust
// external isograph/crates/isograph_schema/src/isograph_database.rs
    db.get_schema_source(relative_path)
        .map(|schema_source| schema_source.content.clone())
}
```

The only writer of `SchemaSource` is `read_schema` (change 2), called for the configured paths only, so a `SchemaSource` at `relative_path` exists exactly when the path is a configured schema path with a stored file. The config comparison is redundant. The memo now depends on the editor buffer key, the iso literal key, and the schema key of `relative_path`, not on the config or the whole `StandardSources` counter.

### 2. `isograph_compiler`: write and remove `SchemaSource` by path

`initialize_sources`, before (`isograph/crates/isograph_compiler/src/source_files.rs:23-34`):

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
pub fn initialize_sources<TCompilationProfile: CompilationProfile>(
    db: &mut IsographDatabase<TCompilationProfile>,
) -> LocationFreeDiagnosticResult<()> {
    let schema = db.get_isograph_config().schema.clone();
    let schema_source_id = read_schema(db, &schema)?;
    let schema_extension_sources = read_schema_extensions(db)?;
    *db.get_standard_sources_mut().tracked() = StandardSources {
        schema_source_id,
        schema_extension_sources,
    };
    read_iso_literals_from_project_root(db)
}
```

After:

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
pub fn initialize_sources<TCompilationProfile: CompilationProfile>(
    db: &mut IsographDatabase<TCompilationProfile>,
) -> LocationFreeDiagnosticResult<()> {
    let config = db.get_isograph_config().clone();
    std::iter::once(&config.schema)
        .chain(config.schema_extensions.iter())
        .try_for_each(|schema_path| read_schema(db, schema_path))?;
    read_iso_literals_from_project_root(db)
}
```

`read_schema_extensions` (`:259-269`) is deleted. Reads stay in config order and the first error returns, as today.

`read_schema`, before (`:206-221`):

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
fn read_schema<TCompilationProfile: CompilationProfile>(
    db: &mut IsographDatabase<TCompilationProfile>,
    schema_path: &AbsolutePathAndRelativePath,
) -> LocationFreeDiagnosticResult<SourceId<SchemaSource>> {
    let content = read_schema_file(&schema_path.absolute_path)?;
    let text_source = TextSource {
        relative_path_to_source_file: schema_path.relative_path,
        span: None,
    };
    db.set(SchemaSource {
        relative_path: schema_path.relative_path,
        content,
        text_source,
    })
    .wrap_ok()
}
```

After:

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
fn read_schema<TCompilationProfile: CompilationProfile>(
    db: &mut IsographDatabase<TCompilationProfile>,
    schema_path: &AbsolutePathAndRelativePath,
) -> LocationFreeDiagnosticResult<()> {
    let content = read_schema_file(&schema_path.absolute_path)?;
    db.insert_schema_source(schema_path.relative_path, content);
    Ok(())
}
```

`handle_update_schema`, before (`:59-84`):

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
    let schema = db.get_isograph_config().schema.clone();
    match event_kind {
        SourceEventKind::CreateOrModify(_) => {
            db.get_standard_sources_mut().tracked().schema_source_id = read_schema(db, &schema)?;
        }
        SourceEventKind::Rename((_, target_path)) => {
            if schema.absolute_path != *target_path {
                db.remove(db.get_standard_sources().untracked().schema_source_id);
                db.get_standard_sources_mut().tracked().schema_source_id = SourceId::default();

                return schema_not_found_diagnostic().wrap_err();
            }
        }
        SourceEventKind::Remove(_) => {
            db.remove(db.get_standard_sources().untracked().schema_source_id);
            db.get_standard_sources_mut().tracked().schema_source_id = SourceId::default();

            return schema_not_found_diagnostic().wrap_err();
        }
    }
    Ok(())
```

After:

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
    let schema = db.get_isograph_config().schema.clone();
    match event_kind {
        SourceEventKind::CreateOrModify(_) => {
            read_schema(db, &schema)?;
        }
        SourceEventKind::Rename((_, target_path)) => {
            if schema.absolute_path != *target_path {
                db.remove_schema_source(schema.relative_path);
                return schema_not_found_diagnostic().wrap_err();
            }
        }
        SourceEventKind::Remove(_) => {
            db.remove_schema_source(schema.relative_path);
            return schema_not_found_diagnostic().wrap_err();
        }
    }
    Ok(())
```

The `SourceId::default()` placeholder goes away. Today a later `get_schema_source()` with that placeholder panics in `Storage::get` (`isograph/crates/pico/src/database.rs:126-131`); the `?` at `watch.rs:58` and `server.rs:197` exits before any compile, so the panic is not reached.

`handle_update_schema_extensions` (`:86-119`) calls `db.remove_schema_source(interned_file_path)` at `:107` and `:115` in place of `db.remove_schema_extension(interned_file_path)`. `create_or_update_schema_extension`, before (`:121-133`):

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
    let absolute_and_relative =
        absolute_and_relative_paths(db.get_current_working_directory(), path.to_path_buf());
    let schema_id = read_schema(db, &absolute_and_relative)?;
    db.get_standard_sources_mut()
        .tracked()
        .schema_extension_sources
        .insert(absolute_and_relative.relative_path, schema_id);
    Ok(())
```

After:

```rust
// external isograph/crates/isograph_compiler/src/source_files.rs
    let absolute_and_relative =
        absolute_and_relative_paths(db.get_current_working_directory(), path.to_path_buf());
    read_schema(db, &absolute_and_relative)
```

`use` lines: `BTreeMap` (`:2`), `SourceId` (`:14`), `SchemaSource`, and `StandardSources` (`:13`) leave; `TextSource` (`:8`) leaves with the `TextSource` construction, which moves to `insert_schema_source`.

The watcher (`isograph/crates/isograph_compiler/src/watch.rs:222-245`, `:286-293`) is unchanged. It already watches each configured schema path by exact path, `NonRecursive`, including paths outside `project_root` (`demos/pet-demo/isograph.config.json`: `project_root` `./src/components`, `schema` `./backend/schema.graphql`). `read_schema` already reads the schema files specially, outside `read_iso_literals_from_folder`; that does not change. Config changes rebuild `CompilerState` with a new database (`watch.rs:44-55`, `server.rs:174-192`), so the config singleton never changes inside one database.

### 3. `graphql_network_protocol`: read schema files by path

Before (`isograph/crates/graphql_network_protocol/src/read_schema.rs:11-58`):

```rust
// external isograph/crates/graphql_network_protocol/src/read_schema.rs
#[memo]
// TODO add error recovery and the non_fatal_diagnostics vec, i.e. do not return
// a Result
pub fn parse_graphql_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<(
    GraphQLTypeSystemDocument,
    BTreeMap<RelativePathToSourceFile, MemoRef<GraphQLTypeSystemExtensionDocument>>,
)> {
    // TODO use db.get_standard_sources()
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get_schema_source();

    let schema = parse_schema(content, *text_source)?;

    let mut schema_extensions = BTreeMap::new();
    for (relative_path, schema_extension_source_id) in db
        .get_standard_sources()
        .tracked()
        .schema_extension_sources
        .iter()
    {
        let extensions_document = parse_schema_extensions_file(db, *schema_extension_source_id)
            .to_owned()
            .note_todo("Do not clone. Use a MemoRef.")?;
        schema_extensions.insert(*relative_path, extensions_document);
    }

    (schema, schema_extensions).wrap_ok()
}

#[memo]
pub fn parse_schema_extensions_file<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    schema_extension_source_id: SourceId<SchemaSource>,
) -> DiagnosticResult<MemoRef<GraphQLTypeSystemExtensionDocument>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_extension_source_id);
    let schema_extensions = parse_schema_extensions(content, *text_source)?;

    schema_extensions.interned_value(db).wrap_ok()
}
```

After:

```rust
// external isograph/crates/graphql_network_protocol/src/read_schema.rs
#[memo]
// TODO add error recovery and the non_fatal_diagnostics vec, i.e. do not return
// a Result
pub fn parse_graphql_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<(
    GraphQLTypeSystemDocument,
    BTreeMap<RelativePathToSourceFile, MemoRef<GraphQLTypeSystemExtensionDocument>>,
)> {
    let config = db.get_isograph_config();
    let SchemaSource {
        content,
        text_source,
        ..
    } = db
        .get_schema_source(config.schema.relative_path)
        .ok_or_else(schema_not_found_diagnostic)?;

    let schema = parse_schema(content, *text_source)?;

    let schema_extensions = config
        .schema_extensions
        .iter()
        .filter_map(|schema_extension| {
            parse_schema_extensions_file(db, schema_extension.relative_path)
                .to_owned()
                .note_todo("Do not clone. Use a MemoRef.")
                .map(|extensions_document| {
                    extensions_document.map(|extensions_document| {
                        (schema_extension.relative_path, extensions_document)
                    })
                })
        })
        .collect::<DiagnosticResult<BTreeMap<_, _>>>()?;

    (schema, schema_extensions).wrap_ok()
}

/// The parse of the schema extension at `relative_path`; `None` when no file is stored there.
#[memo]
pub fn parse_schema_extensions_file<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    relative_path: RelativePathToSourceFile,
) -> Option<DiagnosticResult<MemoRef<GraphQLTypeSystemExtensionDocument>>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get_schema_source(relative_path)?;
    parse_schema_extensions(content, *text_source)
        .map(|schema_extensions| schema_extensions.interned_value(db))
        .wrap_some()
}

fn schema_not_found_diagnostic() -> Diagnostic {
    LocationFreeDiagnostic::from(
        "Schema not found. Cannot proceed without a schema".to_string(),
    )
    .into()
}
```

`Diagnostic: From<LocationFreeDiagnostic>` is `isograph/crates/common_lang_types/src/diagnostic.rs:134`. The message is the one `isograph_compiler`'s `schema_not_found_diagnostic` returns (`source_files.rs:291-295`); `graphql_network_protocol` does not depend on `isograph_compiler`, so the function is written in both crates (open decision 2). The `BTreeMap` keyed by path keeps today's order and today's de-duplication of a path listed twice. `.collect::<DiagnosticResult<BTreeMap<_, _>>>()` stops at the first extension parse error, as the `?` in today's loop does. `SourceId` leaves the `use` at `:7`.

Callers of `parse_graphql_schema` (`isograph/crates/graphql_network_protocol/src/parse_type_system_document.rs:48`, `isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:220`) are unchanged: the return type is unchanged. `parse_schema_extensions_file` has no other caller.

### 4. `isograph_lsp`: goto definition reads the location's file

Under open decision 1 (b). Before (`isograph/crates/isograph_lsp/src/goto_definition.rs:78-92`, and the same shape at `:125-139`):

```rust
// external isograph/crates/isograph_lsp/src/goto_definition.rs
                        DefinitionLocation::Server(server_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name.item,
                                server_selectable.lookup(db).name.item,
                            )
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    db.get_schema_source().content.reference(),
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
```

After:

```rust
// external isograph/crates/isograph_lsp/src/goto_definition.rs
                        DefinitionLocation::Server(server_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name.item,
                                server_selectable.lookup(db).name.item,
                            )
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    db.get_schema_source(
                                        location.text_source.relative_path_to_source_file,
                                    )?
                                    .content
                                    .reference(),
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
```

`goto_entity_definition`, before (`:231-243`):

```rust
// external isograph/crates/isograph_lsp/src/goto_definition.rs
    GotoDefinitionResponse::Scalar(
        isograph_location_to_lsp_location(db, location, &db.get_schema_source().content)
            .ok_or(LSPRuntimeError::ExpectedError)?,
    )
```

After:

```rust
// external isograph/crates/isograph_lsp/src/goto_definition.rs
    let schema_source = db
        .get_schema_source(location.text_source.relative_path_to_source_file)
        .ok_or(LSPRuntimeError::ExpectedError)?;
    GotoDefinitionResponse::Scalar(
        isograph_location_to_lsp_location(db, location, &schema_source.content)
            .ok_or(LSPRuntimeError::ExpectedError)?,
    )
```

The `ClientScalarSelectableNameWrapper` and `ClientObjectSelectableNameWrapper` arms (`:181-190`, `:200-209`) resolve a client field or pointer, whose location is in an iso literal file. They read the text the way the `DefinitionLocation::Client` arms do (`:95-111`):

```rust
// external isograph/crates/isograph_lsp/src/goto_definition.rs
                selectable_definition_location(db, parent_type_name.0, wrapper.inner.0)
                    .and_then(|location| {
                        isograph_location_to_lsp_location(
                            db,
                            location,
                            read_iso_literals_source_from_relative_path(
                                db,
                                location.text_source.relative_path_to_source_file,
                            )
                            .as_ref()?
                            .content
                            .reference(),
                        )
                    })
                    .map(lsp_location_to_scalar_response)
```

(and the same with `object_wrapper_path.inner.0` at `:200-209`).

## Who calls what, after

- `initialize_sources`, `handle_update_schema`, `create_or_update_schema_extension` call `read_schema`, which calls `insert_schema_source`.
- `handle_update_schema`, `handle_update_schema_extensions` call `remove_schema_source`.
- `parse_graphql_schema` calls `get_isograph_config`, `get_schema_source(config.schema.relative_path)`, and `parse_schema_extensions_file(db, path)` for each `config.schema_extensions` path.
- `parse_schema_extensions_file` calls `get_schema_source(path)`.
- `file_text_at_location` calls `get_schema_source(relative_path)` after the editor buffer and iso literal lookups.
- `on_goto_definition_impl` and `goto_entity_definition` call `get_schema_source(location path)` or `read_iso_literals_source_from_relative_path(location path)`.

Nothing calls `get_standard_sources`, `get_standard_sources_mut`, or `remove_schema_extension`; the `Db` derive stops generating the first two.

## What could block the proposal

Nothing blocks it upstream. Each requirement was checked:

- The expected paths are in the config. `CompilerConfig.schema` and `CompilerConfig.schema_extensions` are exact canonicalized paths (`compilation_options.rs:24-27`, `:194-219`), not globs.
- The ids are computable. `SchemaSource` has `#[key] relative_path` (`isograph_database.rs:29-36`), and `id_for_key` hashes `TypeId::of::<SchemaSource>()` plus the key (`isograph/crates/pico_macros/src/source_macro.rs:44-53`), so a schema file and an iso literal at one path do not collide.
- The contents still reach the database. `read_schema` is the only reader of schema files, and the watcher posts every configured schema path by exact match (`watch.rs:222-245`). The proposal removes the bookkeeping, not the read.
- A schema path outside `project_root` is watched explicitly (`watch.rs:286-293`).
- Nothing else lives in `StandardSources`: `schema_source_id` and `schema_extension_sources` are its only fields. Iso literal paths moved out in #1024; the config is a singleton.

## Open decisions

1. The five `get_schema_source()` call sites in `goto_definition.rs`. `get_schema_source` loses its zero-argument form, so each site must name a path.
   - (a) Upstream behavior: pass `db.get_isograph_config().schema.relative_path`. Every position is still computed against the main schema's text, including for definitions in extensions and, at `:186` and `:205`, for client fields in iso literal files.
   - (b) Pass the location's path (change 4). A server field or entity in an extension gets its own file's text; the two client wrapper arms read the iso literal source as the `DefinitionLocation::Client` arms already do.
   - Held: (b). Stated reason for the deviation: under (a) the span of a location in file X is converted to a line and column against the text of file Y, which gives a wrong position or none; (b) uses the pattern the same function already uses for client locations.
2. A removed main schema.
   - (a) Upstream behavior: `handle_update_schema` removes the source and returns "Schema not found"; watch mode and the LSP exit with it. `parse_graphql_schema` returns the same message when `get_schema_source` is `None`, a branch no current flow reaches.
   - (b) `handle_update_schema` removes the source and returns `Ok(())`; the compile reports "Schema not found" from `parse_graphql_schema`, and a later create re-invokes it through the missing-key dependency. This also needs the watcher to see the re-create: `watch.rs:290-292` watches the file itself with `RecursiveMode::NonRecursive`, and an inotify watch on a deleted file is dropped, so on Linux the parent directory must be watched instead.
   - Held: (a). (b) changes the watcher and the error flow, which this refactor does not need; it is a follow-up that this change makes possible.
3. A configured extension with no stored file.
   - (a) Upstream behavior: skip it (today `remove_schema_extension` drops it and returns nothing, `source_files.rs:107`, `:115`).
   - (b) Report a diagnostic naming the path.
   - Held: (a).
4. The shape of `parse_schema_extensions_file`.
   - (a) Take `RelativePathToSourceFile` and call `try_get` inside, returning `Option<DiagnosticResult<..>>` (change 3).
   - (b) `parse_graphql_schema` calls `get_schema_source(path)` to filter, then passes a path or `SourceId` to a memo that calls `db.get`. That memo panics if pico re-executes it after the source is removed. Today it is not re-executed first only because `any_dependency_changed` checks dependencies in recorded order and stops at the first change (`isograph/crates/pico/src/execute_memoized_function.rs:183-195`), and the `try_get` in the caller was recorded before the call.
   - Held: (a). It does not depend on pico's verification order. It mixes `Option` and `Result` levels, which AGENTS.md allows for `Option<Result<T, E>>`.
5. A test that counts re-invocations, as #1003 (`crates/graphql_network_protocol/tests/editor_buffer.rs`) and #1024 (`crates/graphql_network_protocol/tests/iso_literal_paths.rs`) each added.
   - (a) Upstream precedent: add `crates/graphql_network_protocol/tests/schema_paths.rs`, asserting that setting extension A does not re-invoke the parse of extension B, and that setting a configured extension path that had no file re-invokes `parse_graphql_schema`. It needs a `CompilerConfig` singleton in the test database.
   - (b) No test.
   - Held: (b), per the instruction not to add tests. (a) is the upstream PRs' pattern.
6. The schema-path rename case. `handle_update_schema` does nothing for `Rename((_, target))` when `target` is the schema path (`source_files.rs:67-75`), so an editor that saves by writing a temporary file and renaming it onto the schema leaves the old text stored. This exists today and this change keeps it. Options: (a) keep, (b) call `read_schema` in that branch. Held: (a), upstream behavior; (b) is a separate fix.

## i2

i2 has no `StandardSources` (`refactors/past/filesystem-events.md:30` dropped it when `IsographDb` was created). Schema text lives in `DiskFile` and `EditorBuffer` and is read only through `file_contents`, which already does `try_get(EditorBuffer::id_for_key(..))` then `try_get(DiskFile::id_for_key(&path))` (`crates/isograph_compiler/src/database.rs:124-136`). The proposal's per-key read is already i2's read.

What i2 keeps is the list of schema paths, `#[tracked] schema_path_map: SchemaPathMap` (`crates/isograph_compiler/src/database.rs:26-27`, `:62-66`), read by `schema_file_paths(db, client)` (`:161-178`). It is written by `insert_disk_file`, `remove_disk_file`, `remove_disk_files_from_path` (`:343-415`), `set_editor_buffer`, `remove_editor_buffer`, `remove_editor_buffers_of_client` (`:187-317`), from a `FileMembership` that `ProjectGlobs::membership` computes (`crates/isograph_cli/src/watch.rs:250-259`, called from `apply_disk_changed`, `crates/isograph_cli/src/state.rs:366-389`). Its readers are `schema_entries` (`crates/graphql_network_protocol/src/schema_files.rs:33-46`), `compile` (`crates/isograph_cli/src/compile.rs:197`), and `client_diagnostics` through `editor_buffer_schema_paths_of_client` (`crates/isograph_cli/src/diagnostics.rs:111`).

i2's `schema` config is a glob list (`crates/isograph_config/src/compilation_options.rs:16-18`, `refactors/past/schema-files.md`), not a list of paths. A glob does not name the paths it matches, so a memo cannot iterate expected paths from the config; the set of matching paths is learned from the watcher's scan and from opened buffers, which is what `schema_path_map` records. Applying the proposal in i2 needs i2 to adopt upstream's `schema` plus `schema_extensions` exact paths (the difference is recorded at `refactors/pending/upstream-parity.md:146`), a config singleton in `IsographDb` that a memo reads (i2 has only `ProtocolOptions`, `database.rs:93-121`), and watches on configured paths outside the config directory (i2 watches only the config directory, `crates/isograph_cli/src/watch.rs:121-129`). That is a separate i2 decision and is not part of this doc.
