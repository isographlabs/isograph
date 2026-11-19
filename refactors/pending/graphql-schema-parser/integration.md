# GraphQL schema parser: integration

Requires parsing-standards.md, ast.md, and every change in order.md. This file is how `graphql_network_protocol` reads schema files and how the core lookups (`entity_named`, `selectable_named`, `selectables_for_entity` in `crates/isograph_compiler/src/schema.rs`) merge what the schema defines with what source files declare. The protocol data the core carries (`EntityOrigin`, `SelectableOrigin`, `GraphQLEntityData`, `GraphQLSelectableData`) is `refactors/pending/graphql-protocol/entrypoints.md`; its changes 2 and 3 (query-generation.md, Changes) land with this doc's changes 2 and 3.

The goal is isolation: many schema files, any of them broken in places, and a lookup succeeds whenever the definitions on its own path parse and agree. A lookup uses `?` only on the parse results of the name it asks for.

## End-user experience

The config's `schema` globs name the schema files. A user can split one type across files: `type User` in one file and `extend type User` in others, in any order. A lookup of `User` sees the definition and every extension.

```graphql
# schema/schema.graphql
type Query {
  pet(id: ID!): Pet
  feed: [FeedItem!]!
}
interface Node {
  id: ID!
}
type Pet implements Node {
  "The pet's given name."
  name: String!
}
union FeedItem = Pet | Owner
scalar ID
```

```graphql
# schema/extensions.graphql
extend type Pet {
  nickname: String
}
type Owner implements Node {
  id: ID!
}
```

```ts
iso(`field Query.HomeRoute { pet(id: $id) { name nickname } feed { asPet { name } } }`);
```

- Hover on `pet` shows `field **Query.pet**: **Pet**`; go-to-definition jumps to `pet` in `schema/schema.graphql`. Hover on `nickname` shows `field **Pet.nickname**: **String**`; go-to-definition jumps to `extensions.graphql`. Hover on `name` shows the description under the heading.
- Hover on `asPet` shows `field **FeedItem.asPet**: **Pet**`; go-to-definition jumps to `Pet` in `type Pet`, the target type's definition.
- Completion inside `pet(id: $id) { }` lists `name` and `nickname` with detail `String`; inside `feed { }` it lists `asPet` and `asOwner`. On `Node` it lists `id`, `asPet`, and `asOwner`.
- Validation: `field Pet.x { name }` has no diagnostic (`Pet` comes from the schema). Selecting `nope` under `pet` is `` Field `Pet.nope` is not defined. `` on `nope`. `entity Pet` beside `type Pet` is `` Entity `Pet` is defined more than once. `` on the `entity Pet` name, and every lookup of `Pet` fails.

Error rules, each an error of that name only:

- A syntax error inside `type User` or `extend type User` fails every lookup that reaches `User` (`User` itself, `User.x`, and `asUser` on a union containing `User`), and no other lookup.
- Two definitions of `User` (in one file or across files), an `extend type User` with no `type User`, and an `extend input User` on `type User` are errors of `User` and of its selectables.
- Two fields `id` on `User` across the definition and extensions are an error of `User.id` only.
- A field named `asPet` on a union or interface that also has the refinement `asPet` is an error of that selectable only.
- An unreadable or deleted schema file contributes nothing. A field whose type names an entity that does not exist fails only when something resolves that field's target.

Adding or removing a schema file changes lookups without a restart. An open, edited schema buffer is what that client's lookups read. Schema errors are not shown as diagnostics yet (parsing-standards.md, Later); they reach the user as a failed hover, go-to-definition, or completion, and validation does not report a selection whose lookup fails with a schema error.

Entities are object, interface, union, enum, scalar, and input object types. `ID`, `String`, `Int`, `Float`, and `Boolean` are entities only when a schema file defines them; nothing reads a built-in scalar yet. Selectables are fields of objects and interfaces, and refinements. Input object fields are not selectables: input objects are never selection targets, and argument validation, their only reader, will add them (likely as a `GraphQLSelectableData::InputField` variant).

A refinement `asT`: on a union `U`, for each member type `T` of the definition and its extensions that is an object type; on an interface `I`, for each object type `T` whose definition or any `extend type T implements I` lists `I`. No refinement targets an interface, so an interface implementing `I` gives none. Its target is `T` and its definition is `T`'s definition name. A union member or implementor whose own lookup fails makes `asT` fail with that error.

## Config and schema paths

```rust
// from crates/isograph_config/src/compilation_options.rs
    /// Glob patterns relative to the config file's directory: the GraphQL schema files.
    #[serde(default)]
    pub schema: Vec<String>,
```

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
```

The config field, the watcher on both glob lists, `SchemaPathMap`, and its writers (`insert_disk_file`, `set_open_file`, `remove_disk_file`, `remove_disk_files_from_path`, `remove_open_file`, `remove_open_files_of_client`) are in i2 (`refactors/past/source-and-schema-path-maps.md`). An applied `DiskChanged` arms the diagnostics debounce, so a schema edit on disk republishes the open source files' validation diagnostics.

## Schema files

```rust
// from crates/graphql_network_protocol/src/schema_files.rs
/// Every schema file `client` reads: disk paths and this client's open paths, sorted.
#[memo]
pub fn schema_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let view = db.get_schema_path_map();
    let map = view.tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.open
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The parse of one schema file as `client` reads it; `None` when the file has no text.
#[memo]
pub fn parsed_schema_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<ParsedSchema> {
    file_contents(db, client, path)
        .as_deref()
        .map(graphql_schema_parser::parse_schema)
}

/// Every parsed entry of every schema file `client` reads, with its file.
pub(crate) fn schema_entries<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> impl Iterator<Item = SchemaEntry<'_>> {
    schema_file_paths(db, client).iter().flat_map(move |path| {
        parsed_schema_file(db, client, path.dereference())
            .iter()
            .flat_map(|parsed| parsed.definitions.iter())
            .map(move |entry| SchemaEntry {
                path: path.dereference(),
                entry,
            })
    })
}

/// One top-level entry of a schema file.
pub(crate) struct SchemaEntry<'db> {
    pub path: RelativePathToSourceFile,
    pub entry: &'db Result<WithSpan<TypeSystemDefinitionOrExtension>, WithSpan<DefinitionError>>,
}
```

`schema_file_paths` is `source_file_paths` (`crates/isograph_compiler/src/database.rs`) over `schema_path_map`. It iterates the map, so it reads `tracked()` (AGENTS.md, pico tracked maps). `file_contents` is the client's open buffer, otherwise disk. Each keystroke in an open schema file re-invokes `parsed_schema_file` for that client and path and reparses the whole file. The parse runs once per client per file text version.

Known gap: pico records no dependency when a memo reads a tracked map whose counter was never written (refactors/pending/pico-missing-key-dependency.md). In the daemon the boot scan writes the map before the first request; tests insert their schema files before the first lookup.

## Entities in the schema

```rust
// from crates/graphql_network_protocol/src/schema_lookup.rs
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
    #[error("{0}")]
    Syntax(EntitySyntaxError),
    #[error("{0}")]
    DuplicateDefinition(DuplicateDefinition),
    #[error("{0}")]
    ExtensionWithoutDefinition(ExtensionWithoutDefinition),
    #[error("{0}")]
    ExtensionKindMismatch(ExtensionKindMismatch),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("A definition or extension of `{name}` has a syntax error: {}", .failed.item.item.error.item)]
pub struct EntitySyntaxError {
    pub name: EntityName,
    pub failed: InSchemaFile<DefinitionError>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Type `{name}` is defined more than once in the schema.")]
pub struct DuplicateDefinition {
    pub name: EntityName,
    pub first: InSchemaFile<TypeDefinition>,
    pub second: InSchemaFile<TypeDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Type `{name}` is extended but never defined.")]
pub struct ExtensionWithoutDefinition {
    pub name: EntityName,
    pub extension: InSchemaFile<TypeExtension>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("An extension of `{name}` is not the kind of type `{name}` is defined as.")]
pub struct ExtensionKindMismatch {
    pub name: EntityName,
    pub extension: InSchemaFile<TypeExtension>,
}

/// The named entity `name` as `client` reads the schema; `Ok(None)` when no schema file
/// defines or extends it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<SchemaEntity>, EntityLookupError>
```

`InSchemaFile` uses `WithSpan<T>`'s span as the parser wrote it: the entry's span in that file. For a `DefinitionError` it is the failed definition's span (ast.md).

`entity_named` walks `schema_entries(db, client)` and keeps only the entries of `name`:

- `Ok` with `Definition(Type(definition))` whose name is `name`: a definition.
- `Ok` with `Extension(Type(extension))` whose name is `name`: an extension.
- `Err` whose `DefinitionError.name` is `Some(DefinitionName::Entity(name))`: `Syntax`, returned for the first such entry.
- Every other entry (other names, schema definitions and extensions, directive definitions, unnamed failures) is not read.

Then, in this order: no definition and no extension is `Ok(None)`; extensions and no definition is `ExtensionWithoutDefinition` of the first extension; two or more definitions is `DuplicateDefinition` of the first two; an extension of a different kind than the definition (spec 3.4.3) is `ExtensionKindMismatch` of the first such extension; otherwise `Ok(Some(SchemaEntity))`. Extensions apply in any order.

```rust
// from crates/graphql_network_protocol/src/schema_lookup.rs
/// The kind of a type definition or extension (spec 3.4).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
}

pub(crate) fn type_definition_name(definition: &TypeDefinition) -> WithSpan<EntityNameWrapper>;
pub(crate) fn type_definition_kind(definition: &TypeDefinition) -> TypeKind;
pub(crate) fn type_extension_name(extension: &TypeExtension) -> WithSpan<EntityNameWrapper>;
pub(crate) fn type_extension_kind(extension: &TypeExtension) -> TypeKind;
```

Each is a match over the six variants. They are plain functions because `TypeDefinition` and `TypeExtension` live in `graphql_schema_parser`, which no other crate depends on for behavior.

## Fields in the schema

```rust
// from crates/graphql_network_protocol/src/schema_lookup.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum SelectableLookupError {
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("{0}")]
    DuplicateField(DuplicateField),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{entity}.{selectable}` is defined more than once in the schema.")]
pub struct DuplicateField {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub first: InSchemaFile<FieldDefinition>,
    pub second: InSchemaFile<FieldDefinition>,
}

/// The field `selectable` of the entity `entity` in the schema; `Ok(None)` when the entity
/// does not exist in the schema or has no such field.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<InSchemaFile<FieldDefinition>>, SelectableLookupError>

/// Every field of `schema_entity`: the definition's then each extension's, with its file.
pub(crate) fn schema_entity_fields(
    schema_entity: &SchemaEntity,
) -> impl Iterator<Item = InSchemaFile<FieldDefinition>> + '_
```

`selectable_named` is `entity_named(db, client, entity).clone()?`, then the fields named `selectable` among `schema_entity_fields`: `ObjectTypeDefinition.fields`, `InterfaceTypeDefinition.fields`, and the object and interface extensions' `fields`. Scalars, unions, enums, and input objects contribute none. One field is `Ok(Some(..))`; two or more are `DuplicateField` of the first two; none is `Ok(None)`. The field's type is not resolved here.

Whole-schema checks (every root operation type exists, every interface a type implements is an interface, every union member is an object) are not lookups. They belong to a later pass over every name and do not gate `entity_named`.

## Protocol entities and selectables

These memos turn the schema lookups into the core's `ProtocolEntity` and `ProtocolSelectable` (entrypoints.md, Protocol data on entities and selectables).

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// What the GraphQL protocol records about a type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLEntityData {
    /// A scalar or enum: selected with no selection set.
    Leaf,
    /// An input object: an argument type, never a selection target.
    InputObject,
    Object(GraphQLCompositeData),
    Interface(GraphQLCompositeData),
    Union,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLCompositeData {
    /// The interfaces this type declares it implements, in definition then extension order.
    pub interfaces: Vec<EntityName>,
}

/// What the GraphQL protocol records about a selectable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphQLSelectableData {
    /// A field of the schema, printed as `name(arguments)`.
    Field,
    /// `asUser` on a union or interface: printed as `... on User`, where `User` is the
    /// selectable's target.
    Refinement,
}

/// The GraphQL type `name` as `client` reads the schema, with the data the protocol records.
#[memo]
pub fn graphql_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, EntityLookupError>
```

`graphql_entity` maps `entity_named`'s `Ok(Some(schema_entity))`: `definition` is the definition's path and name span; the variant follows `type_definition_kind` (`Scalar` and `Enum` are `Leaf`); `interfaces` is the definition's `interfaces` then each object or interface extension's `interfaces`. `GraphQLCompositeData` gains `id_field` with query-generation.md change 7 (the printer reads it) and `fetchable_directives` with change 4 (the fetch plan reads it).

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum GraphQLSelectableLookupError {
    #[error("{0}")]
    Selectable(SelectableLookupError),
    #[error("{0}")]
    #[from(skip)]
    RefinementTarget(RefinementTargetError),
    #[error("{0}")]
    #[from(skip)]
    RefinementCollision(RefinementCollision),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{entity}.{selectable}` refines to `{target}`, whose lookup failed: {error}")]
pub struct RefinementTargetError {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub target: EntityName,
    pub error: EntityLookupError,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{entity}.{selectable}` is defined in the schema and is also a refinement.")]
pub struct RefinementCollision {
    pub entity: EntityName,
    pub selectable: SelectableName,
    pub field: InSchemaFile<FieldDefinition>,
}

/// The selectable `selectable` of the GraphQL type `entity`: a field of the type, or a
/// refinement `as<Type>` of a union or interface.
#[memo]
pub fn graphql_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLSelectableLookupError>

/// Every selectable of the GraphQL type `entity`: its fields, then its refinements whose
/// target resolves. A name may repeat. Empty when the type's lookup fails.
#[memo]
pub fn graphql_selectables<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Vec<(SelectableName, ProtocolSelectable<GraphQLSelectableData>)>

/// Every object type whose definition or an object extension lists `interface` in
/// `implements`, sorted, each once.
#[memo]
pub fn object_types_implementing<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    interface: EntityName,
) -> Vec<EntityName>
```

A field becomes `ProtocolSelectable { description, target_entity, definition, data: Field }`: `target_entity` is the named type under the type annotation's list and non-null wrappers, `definition` is the path and the field name's span.

`graphql_selectable` computes the field (`selectable_named`) and the refinement, then: both is `RefinementCollision`; one is that one; neither is `Ok(None)`. The refinement of `selectable` on `entity`: `selectable` must be `as` followed by a name `T`. On a union (`entity_named(entity)` is a `Union` definition), `T` must be a member of the definition or a union extension. On an interface, `graphql_entity(T)` must be `Object` whose `interfaces` contain `entity`. Then `graphql_entity(T)`: `Err` is `RefinementTarget`; `Ok(Some)` of an `Object` is the refinement `ProtocolSelectable { description: None, target_entity: T, definition: T's definition, data: Refinement }`; anything else is no refinement.

`graphql_selectables` lists `schema_entity_fields` of the entity, then the refinements of each union member (deduplicated) or each of `object_types_implementing(entity)`, keeping the ones whose `graphql_entity` is `Ok(Some)` of an `Object`. A collision or a duplicate field shows as a repeated name; the core keeps names defined exactly once.

`object_types_implementing` walks `schema_entries` and keeps the names of `Ok` object definitions and object extensions whose `interfaces` contain `interface`. It reads every schema file, so a keystroke in any schema file re-runs it; it backdates when the list is `==`.

```rust
// from crates/graphql_network_protocol/src/lib.rs
pub struct GraphQLNetworkProtocol;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum GraphQLLookupError {
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("{0}")]
    Selectable(GraphQLSelectableLookupError),
}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityData = GraphQLEntityData;
    type SelectableData = GraphQLSelectableData;
    type LookupError = GraphQLLookupError;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql" | "graphqls") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }

    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        name: EntityName,
    ) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, GraphQLLookupError> {
        graphql_entity(db, client, name).clone().map_err(GraphQLLookupError::from)
    }

    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLLookupError> {
        graphql_selectable(db, client, entity, selectable)
            .clone()
            .map_err(GraphQLLookupError::from)
    }

    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<GraphQLSelectableData>)> {
        graphql_selectables(db, client, entity).clone()
    }
}
```

## The core lookups

`NetworkProtocol` gains `EntityData`, `SelectableData`, `LookupError`, `entity`, `selectable` (entrypoints.md), and `selectables`, the completion list:

```rust
// from crates/isograph_compiler/src/host_language.rs
    /// Every selectable the protocol defines on `entity`, by name. A name may repeat;
    /// `selectables_for_entity` keeps names defined exactly once.
    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<Self::SelectableData>)>;
```

`FetchPlan`, `NotFetchable`, `QueryError`, `fetch_plan`, and `query_artifact` land with the query-generation changes that read them (4, 5, 7).

The core counts definitions in one place:

```rust
// from crates/isograph_compiler/src/schema.rs
/// What the source files and the protocol define under one name.
pub(crate) enum Definitions<T, TLookupError> {
    Undefined,
    Unique(T),
    /// More than one definition among the source declarations and the protocol.
    Duplicated,
    /// The protocol's lookup failed and no source declaration has the name.
    ProtocolLookupFailed(TLookupError),
}

pub(crate) fn entity_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> Definitions<ProfileEntity<TProfile>, ProfileLookupError<TProfile>>

pub(crate) fn selectable_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Definitions<ProfileSelectable<TProfile>, ProfileLookupError<TProfile>>

pub type ProfileLookupError<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::LookupError;
```

The source definitions (`source_entity_definitions`, `source_selectable_definitions`) chained with the protocol's `Ok(Some)` are counted: zero is `Undefined`, one is `Unique`, more is `Duplicated`. A protocol `Err` counts as one definition: with no source definition it is `ProtocolLookupFailed`, with one or more it is `Duplicated`.

`entity_named` and `selectable_named` match on it: `Unique` is `Ok`; `Undefined` is `` Entity `X` is not defined. `` / `` Field `X.y` is not defined. ``; `Duplicated` is `` Entity `X` is defined more than once. `` / `` Field `X.y` is defined more than once. ``; `ProtocolLookupFailed(error)` is `Diagnostic::new(error.to_string(), None)`.

`selectables_for_entity` is `entity_named(..)?`, then the source selectables of the entity chained with `NetworkProtocol::selectables`, grouped by name, keeping names with exactly one.

Validation reads `entity_definitions` and `selectable_definitions`:

- `UnknownEntity` and `UnknownSelectable` are `Undefined`.
- `DuplicateDefinition` is `Duplicated`, at the source declaration's name. `entity Pet` beside `type Pet` is a duplicate at `entity Pet`.
- A selection whose lookup is `ProtocolLookupFailed` or `Duplicated` reports nothing at the selection and does not descend. A selection of a `Unique` selectable descends into its target when the target's `entity_definitions` is not `Undefined`, as today.

## Changes

### 1. Config, watcher, and `SchemaPathMap`

Status: landed.

### 2. Protocol data types and origins

query-generation.md change 2: `Entity<TEntityData>`, `EntityOrigin`, `Selectable<TSelectableData>`, `SelectableOrigin`, `SourceSelectableOrigin`, `ProtocolSelectableOrigin`, `Selectable::target_entity()`, `ProfileEntity`, `ProfileSelectable`, `SourceSelectableDefinition`; `NetworkProtocol::EntityData`, `SelectableData`. Lookups still read source files only. Tests: the existing lookup, hover, completion, and validation tests pass unchanged in their facts.

### 3. Schema lookups and protocol lookups in the core

`schema_file_paths`, `parsed_schema_file`, `schema_entries`, `entity_named`, `selectable_named` (protocol crate), `graphql_entity`, `graphql_selectable`, `graphql_selectables`, `object_types_implementing`, the error types, `GraphQLLookupError`; `NetworkProtocol::LookupError`, `entity`, `selectable`, `selectables`; `ProtocolEntity`, `ProtocolSelectable`; `Definitions`, `entity_definitions`, `selectable_definitions`; the core lookups and validation counting both. Merges query-generation.md change 3.

Tests in `crates/graphql_network_protocol` (GraphQL profile with `TypeScriptHostLanguage`, schema files inserted before the first lookup):

- `schema_file_paths`: disk `b.graphql`, `a.graphql` lists `[a, b]`; client 1 opening `c.graphql` lists `[a, b, c]` for client 1 and `[a, b]` for client 2; removing `b.graphql` drops it.
- `parsed_schema_file` of an open buffer parses the buffer text, not disk; of a path with no text is `None`.
- `type User { id: ID! }` in `a.graphql`, `extend type User { name: String }` in `b.graphql`: `entity_named(User)` has the definition from `a` and one extension from `b`; `selectable_named(User, name)` is the field from `b`. With the extension in `a.graphql` and the definition in `b.graphql`: the same.
- `type User` in `a.graphql` and in `b.graphql`: `DuplicateDefinition`, first from `a`; `entity_named(Query)` is `Ok`. Two definitions in one file: `DuplicateDefinition`.
- `extend type User { a: A }` alone: `ExtensionWithoutDefinition`; `type User { a: A }` with `extend input User { b: B }`: `ExtensionKindMismatch`.
- `type User { a: ! }` in `a.graphql` and `type Query { user: User }` in `b.graphql`: `entity_named(User)` is `Syntax` with path `a`; `entity_named(Query)` is `Ok`; `selectable_named(Query, user)` is `Ok(Some)`. `type Broken { a: ! } type User { id: ID! }` in one file: `entity_named(User)` is `Ok`.
- `type User { id: ID! }` and `extend type User { id: ID }`: `selectable_named(User, id)` is `DuplicateField`; `selectable_named(User, other)` for a third field is `Ok(Some)`.
- An open buffer of `a.graphql` in client 1 that breaks `type User`: client 1's `entity_named(User)` is `Syntax`; client 2's is `Ok`.
- `graphql_entity`: `scalar ID` and `enum E { A }` are `Leaf`; `input I { a: Int }` is `InputObject`; `union U = A` is `Union`; `type T implements I1` with `extend type T implements I2` is `Object` with `interfaces == [I1, I2]`; `interface I` is `Interface`.
- `graphql_selectable`: `Query.pet` with `pet: [Pet!]!` has target `Pet`, data `Field`, and the definition span of `pet`; `union U = A | B` with `extend union U = C`, objects `A`, `B`, `C`: `asA`, `asC` are `Refinement` with targets `A`, `C` and definitions at their type names; `asD` is `Ok(None)`. `interface Node`, `type Pet implements Node`, `extend type Owner implements Node` in another file, `interface Named implements Node`: `Node.asPet` and `Node.asOwner` are refinements, `Node.asNamed` is `Ok(None)`. A union member `A` whose definition has a syntax error: `U.asA` is `RefinementTarget`, `U.asB` is `Ok(Some)`. `interface I { asPet: Pet }` with `type Pet implements I`: `I.asPet` is `RefinementCollision`, `I.other` unaffected.
- `graphql_selectables(Node)` lists the fields then `asOwner`, `asPet`; of a broken type is empty.

Tests in `crates/isograph_cli/src/state.rs` (handle seam, GraphQL profile, a schema file `schema/schema.graphql` and a source file `src/x.ts`): hover on a schema field and on a refinement; go-to-definition to the field's and the refinement target's definitions in the schema file; completion inside a selection set on a schema type lists the fields and refinements; validation publishes `` Field `Pet.nope` is not defined. `` for an unknown selection and nothing for known ones; editing the schema file on disk (a `DiskChanged`) to add `nope` clears that diagnostic on the next debounce fire.
