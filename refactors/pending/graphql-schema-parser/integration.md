# GraphQL schema parser: integration

Requires parsing-standards.md, ast.md, and every change in order.md. This file is how `graphql_network_protocol` reads schema files. That crate is created by the CompilationProfile prefactor; the changes here land after it. When this doc and that prefactor's code disagree (a type name, where `IsographState` keeps config), stop and fix this doc first.

The goal is isolation: many schema files, any of them broken in places, and a lookup succeeds whenever the definitions on its own path parse and agree. A lookup uses `?` only on the parse results of the name it asks for.

## End-user experience

The config's `schema` globs name the schema files. A user can split one type across files: `type User` in one file and `extend type User` in others, in any order. A lookup of `User` sees the definition and every extension. A syntax error inside `type User` fails every lookup that reaches `User`, and no other lookup. Two definitions of `User` (in one file or across files) and an `extend type User` with no `type User` are errors of `User` only. An unreadable or deleted schema file contributes nothing. A field whose type names an entity that does not exist fails only when something resolves that field's type. Adding or removing a schema file changes lookups without a restart. An open, edited schema buffer is what that client's lookups read. Errors are not surfaced to the user yet (parsing-standards.md, Later).

## Config

```rust
// from crates/isograph_config/src/compilation_options.rs
    /// Glob patterns relative to the config file's directory: the GraphQL schema files.
    #[serde(default)]
    pub schema: Vec<String>,
```

A field of the config struct that holds `source_files`, with the same glob semantics. `#[serde(default)]` keeps configs without it valid (no schema files). The daemon parses it with `SourceGlobs::parse` (`crates/isograph_cli/src/watch.rs`) as it parses `source_files`, and the watcher watches both lists.

## Schema paths

```rust
// from crates/isograph_compiler/src/database.rs
/// Every schema file path on disk, and every open buffer of a schema file path.
#[derive(Debug, Default)]
pub struct SchemaPathMap {
    pub disk: BTreeSet<RelativePathToSourceFile>,
    pub open: BTreeSet<OpenFileKey>,
}

// on IsographDb
    #[tracked]
    schema_path_map: SchemaPathMap,

impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn insert_schema_disk_path(&mut self, path: RelativePathToSourceFile);
    pub fn remove_schema_disk_path(&mut self, path: RelativePathToSourceFile);
    pub fn insert_schema_open_path(&mut self, key: OpenFileKey);
    pub fn remove_schema_open_path(&mut self, key: OpenFileKey);
}
```

Each method reads `untracked()` first and writes `tracked()` only when the set changes, the pattern of `set_open_file`. `handle` calls them beside `insert_disk_file`, `remove_disk_file`, `set_open_file`, and `remove_open_file` when the path matches the `schema` globs; `remove_open_files_of_client` also removes that client's open schema keys. A TypeScript file change does not write `schema_path_map`.

```rust
// from crates/graphql_network_protocol/src/schema_files.rs
/// Every schema file `client` reads: disk paths and this client's open paths, sorted.
#[memo]
pub fn schema_file_paths<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let map = db.get_schema_path_map().tracked();
    map.disk
        .iter()
        .copied()
        .chain(map.open.iter().filter(|key| key.client == client).map(|key| key.path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
```

The memo iterates the map, so it uses `tracked()` (AGENTS.md, pico tracked maps) and re-runs when any schema path is added or removed; it backdates when the list is `==`.

Known gap: pico records no dependency when a memo reads a tracked map whose counter was never written (refactors/pending/lsp-open-files.md, Known gap; the fix is refactors/pending/pico-missing-key-dependency.md). On a database where `schema_path_map` was never written, `schema_file_paths` reads the empty map and does not see the first `insert_schema_disk_path`. Until that pico fix is in i2, tests write `schema_path_map` before the first read, with a comment naming this gap, as the `file_contents` tests do.

## Parsed files

```rust
// from crates/graphql_network_protocol/src/schema_files.rs
/// The parse of one schema file as `client` reads it; `None` when the file has no text.
#[memo]
pub fn parsed_schema_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<ParsedSchema> {
    file_contents(db, client, path)
        .as_ref()
        .map(|text| graphql_schema_parser::parse_schema(text))
}
```

`file_contents` (`crates/isograph_compiler/src/database.rs`) is the client's open buffer, otherwise disk. Each keystroke in an open schema file re-invokes `parsed_schema_file` for that client and path and reparses the whole file (cheap, as iso reparses a whole literal). Lookups that read it re-run and backdate when their result is `==`. The parse runs once per client per file text version: two clients reading the same disk file parse it twice, once per memo key.

## Entities

```rust
// from crates/graphql_network_protocol/src/entity_named.rs
/// A definition or extension and the schema file it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InSchemaFile<T> {
    pub path: RelativePathToSourceFile,
    pub item: WithSpan<T>,
}

/// Every schema source of one named entity: its one definition and its extensions, in
/// schema file path order then source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaEntity {
    pub definition: InSchemaFile<TypeDefinition>,
    pub extensions: Vec<InSchemaFile<TypeExtension>>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntityLookupError {
    #[error("A definition or extension of this type has a syntax error.")]
    Syntax(InSchemaFile<DefinitionError>),
    #[error("This type is defined more than once.")]
    DuplicateDefinition(DuplicateDefinition),
    #[error("This type is extended but never defined.")]
    ExtensionWithoutDefinition(InSchemaFile<TypeExtension>),
    #[error("This extension's kind does not match the type's definition.")]
    ExtensionKindMismatch(InSchemaFile<TypeExtension>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateDefinition {
    pub first: InSchemaFile<TypeDefinition>,
    pub second: InSchemaFile<TypeDefinition>,
}

/// The named entity `name` as `client` reads the schema; `Ok(None)` when no schema file
/// defines or extends it.
#[memo]
pub fn entity_named<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<SchemaEntity>, EntityLookupError>
```

`InSchemaFile` uses `WithSpan<T>`'s span as written by the parser: the definition's span in that file. For a `DefinitionError` it is the failed definition's span (ast.md).

`entity_named` walks `schema_file_paths(db, client)` and, per path, `parsed_schema_file(db, client, path)`; a `None` file is skipped. From each file's entries it keeps only the ones of `name`:

- `Ok` with `Definition(Type(definition))` whose name is `name`: a definition.
- `Ok` with `Extension(Type(extension))` whose name is `name`: an extension.
- `Err` whose `DefinitionError.name` is `Some(DefinitionName::Entity(name))`: `Err(EntityLookupError::Syntax(..))`, returned for the first such entry.
- Every other entry (other names, schema definitions and extensions, directive definitions, unnamed failures) is not read.

Then, in this order: no definition and no extension is `Ok(None)`; extensions and no definition is `ExtensionWithoutDefinition` of the first extension; two or more definitions is `DuplicateDefinition` of the first two; an extension of a different kind than the definition (spec 3.4.3: an object extension extends an object, and so on) is `ExtensionKindMismatch` of the first such extension; otherwise `Ok(Some(SchemaEntity))`. Extensions apply in any order: `SchemaEntity.extensions` keeps them all, and nothing reads their order for meaning.

Each `entity_named` reads every schema file's parse, so a keystroke in any schema file re-runs every `entity_named` the client has asked for; each re-run is a scan of already-parsed entries and backdates when its result is `==`. A broken definition of another name, an unreadable file, a schema definition with an error, or a duplicate of another name does not change the result.

## Selectables

```rust
// from crates/graphql_network_protocol/src/entity_named.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SelectableLookupError {
    #[error("{0}")]
    Entity(#[from] EntityLookupError),
    #[error("This field is defined more than once on its type.")]
    DuplicateField(DuplicateField),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateField {
    pub first: InSchemaFile<FieldDefinition>,
    pub second: InSchemaFile<FieldDefinition>,
}

/// The field `selectable` of the entity `entity`; `Ok(None)` when the entity does not exist
/// or has no such field.
#[memo]
pub fn selectable_named<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<InSchemaFile<FieldDefinition>>, SelectableLookupError>
```

`selectable_named` is `entity_named(db, client, entity).clone()?`, then the fields named `selectable` in the definition and in every object or interface extension (`ObjectTypeDefinition.fields`, `InterfaceTypeDefinition.fields`, and the extensions' `fields`; scalars, unions, enums, and input objects have no output fields). One field is `Ok(Some(..))`; two or more are `DuplicateField` of the first two; none is `Ok(None)`. The field's type is not resolved here: a caller that needs the target calls `entity_named` with the type annotation's name, so an unknown target type fails only the lookups that resolve that field.

Whole-schema checks (every root operation type exists, every interface a type implements is an interface, every union member is an object, no two definitions of any name) are not lookups. They belong to a later compile or diagnostics pass over every name, and do not gate `entity_named`.

## Changes

### 1. `schema` config field and `SchemaPathMap`

`CompilationOptions.schema`, `SchemaPathMap`, the four `IsographDb` methods, the `handle` calls, and the watcher watching `schema` globs. Tests (`crates/isograph_compiler/src/database.rs`, and the daemon's handle tests where `handle` is tested today): inserting a schema disk path twice writes once (a memo reading the map re-runs once); removing an absent path does not write; a TypeScript file event does not change `schema_path_map`; `remove_open_files_of_client` removes that client's open schema keys and no other client's.

### 2. `schema_file_paths` and `parsed_schema_file`

Tests (`crates/graphql_network_protocol`, writing `schema_path_map` first per Known gap): disk paths `b.graphql`, `a.graphql` list as `[a, b]`; client 1 opening `c.graphql` lists `[a, b, c]` for client 1 and `[a, b]` for client 2; a path open and on disk lists once; adding a schema file adds it to the list and removing it drops it; `parsed_schema_file` of a path with no text is `None`; of an open buffer it parses the buffer text, not disk.

### 3. `entity_named` and `selectable_named`

Tests:

- `type User { id: ID! }` in `a.graphql`, `extend type User { name: String }` in `b.graphql`: `entity_named(User)` has the definition from `a` and one extension from `b`; `selectable_named(User, name)` is the field from `b`.
- The extension in `a.graphql` and the definition in `b.graphql`: the same result (extensions in any order, any file).
- `type User` in `a.graphql` and `type User` in `b.graphql`: `DuplicateDefinition` with first from `a`, second from `b`; `entity_named(Query)` for `type Query { a: A }` in `a.graphql` is `Ok`.
- `type User { a: A } type User { b: B }` in one file: `DuplicateDefinition`.
- `extend type User { a: A }` with no definition anywhere: `ExtensionWithoutDefinition`; `entity_named(Other)` is unaffected.
- `type User { a: A }` and `extend input User { b: B }`: `ExtensionKindMismatch`.
- `type User { a: ! }` in `a.graphql` and `type Query { user: User }` in `b.graphql`: `entity_named(User)` is `Syntax` with path `a`; `entity_named(Query)` is `Ok`; `selectable_named(Query, user)` is `Ok(Some(..))` (the target is not resolved).
- A broken definition of another name in the same file as `type User` (`type Broken { a: ! } type User { id: ID! }`): `entity_named(User)` is `Ok`.
- `type Query { a: Missing }`: `selectable_named(Query, a)` is `Ok(Some(..))`; `entity_named(Missing)` is `Ok(None)`.
- `entity_named(Nobody)` is `Ok(None)`; `selectable_named(User, nope)` is `Ok(None)`.
- `type User { id: ID! }` and `extend type User { id: ID }`: `selectable_named(User, id)` is `DuplicateField`; `selectable_named(User, other)` for a third field is `Ok(Some(..))`.
- Adding `b.graphql` with `extend type User { name: String }` after a first `entity_named(User)` call: the next call has the extension; removing the file drops it.
- A schema path whose file was deleted from disk (path still listed, no text): `entity_named` of names in other files is `Ok`.
- An open buffer of `a.graphql` in client 1 that breaks `type User`: client 1's `entity_named(User)` is `Syntax`; client 2's is `Ok`.
