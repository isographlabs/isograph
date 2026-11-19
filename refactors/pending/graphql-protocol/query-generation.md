# GraphQL protocol: query generation

This is the second of two files; `entrypoints.md` in this directory specifies the protocol data, `NetworkProtocol`'s methods, the fetch plan, and validation. This file specifies how the core turns an entrypoint into a merged selection map, how the GraphQL protocol builds an operation document from that map and prints it, where the text goes, and which functions are memos. The ordered change list for both files is at the end.

Every entrypoint gets a query. The core walks the entrypoint field's selection set, inlines the selection sets of the `field` declarations it selects, and merges the result into one map keyed by selectable name and arguments. That walk is protocol-agnostic: a protocol selection is a node holding the protocol's data, which the core does not inspect. The protocol builds an `OperationDocument` that adds the fields normalization needs (`id`, `__typename`) and prints it with one printer. Entities declared by `entity` in source files contribute nothing to a query.

`normalization-ast.md` in this directory specifies the normalization AST, built from the same `OperationDocument`.

## End-user experience

The daemon generates, for `entrypoint Query.PetDetailRoute` in the pet demo, three files under `src/__isograph/Query/PetDetailRoute/`:

```text
entrypoint.ts
query_text.ts
query_text.graphql
```

`query_text.graphql` is exactly the operation text:

```graphql
query PetDetailRoute($id: ID!) {
  pet____id___v_id: pet(id: $id) {
    id: id
    best_friend_relationship: best_friend_relationship {
      best_friend: best_friend {
        id: id
        firstName: firstName
        lastName: lastName
      }
      picture_together: picture_together
    }
    firstName: firstName
    lastName: lastName
    nickname: nickname
    stats: stats {
      cuteness: cuteness
      energy: energy
      hunger: hunger
      intelligence: intelligence
      sociability: sociability
      weight: weight
    }
  }
}
```

followed by one newline. `query_text.ts` default-exports the same text as a string. `entrypoint.ts`'s default export gains `queryText`, imported from `./query_text`.

What the text shows:

- `PetDetailRoute`'s own selections are the `pet(id: $id)` field. `fullName`, `PetStatsCard`, `PetBestFriendCard`, `bestFriend`, and `owner` are `field` declarations; their selection sets are inlined (`firstName`, `lastName`, `nickname`, `stats { .. }`, `best_friend_relationship { .. }`, `id`).
- `friendName: firstName` in `PetBestFriendCard` prints as `firstName: firstName`. A reader alias is the reader's name for a value; it does not reach the query.
- Every field prints with its normalization alias: `pet(id: $id)` as `pet____id___v_id: pet(id: $id)`, and a field with no arguments under its own name (`firstName: firstName`).
- `PetCheckinsCard(limit: 5) @loadable(lazyLoadArtifact: true)` contributes nothing. Its query is the loadable field's own, a later doc.
- `bestFriend { fullName }` contributes `bestFriend`'s own selection set (`best_friend_relationship { best_friend { id } }`), not `fullName`. The selections under a `field ... to` pointer are fetched by refetching the target, a later doc. `owner { OwnerBadge }` likewise contributes `owner`'s `{ id }`; `OwnerBadge` is on the source entity `Owner`.
- `id: id` comes first under every type with `id: ID!`, whether or not anything selected it. `Pet` is an object, so there is no `__typename`.
- Entries are in alphabetical order after `id`.

`entrypoint Pet.PetSummaryCard`, through `Node`:

```graphql
query PetSummaryCard($id: ID!) {
  node____id___v_id: node(id: $id) {
    __typename: __typename
    id: id
    ... on Pet {
      __typename: __typename
      id: id
      age: age
      firstName: firstName
      lastName: lastName
      nickname: nickname
      picture: picture
      species: species
      tagline: tagline
    }
  }
}
```

`entrypoint Mutation.SetTagline`:

```graphql
mutation SetTagline($id: ID!, $tagline: String!) {
  set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {
    pet: pet {
      id: id
      tagline: tagline
    }
  }
}
```

A query that cannot be generated is an error of that entrypoint only. Selecting `nope` in `field Pet.PetStatsCard` fails `PetDetailRoute`'s query (it inlines `PetStatsCard`) and leaves `HomeRoute`'s, `SpeciesRoute`'s, and `SetTagline`'s queries unchanged. While any query fails, the daemon logs the entrypoint and the error and writes no artifacts, as it does for a literal with a parse error (artifact-generation.md); open decision 4.

## The merged selection map

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The server selections of one selection set after inlining `field` declarations, keyed
/// by selectable name and arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelectionMap<TSelectableData, TEntityData>(
    pub BTreeMap<MergeKey, MergedSelection<TSelectableData, TEntityData>>,
);

/// One protocol selectable selected with one set of argument values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelection<TSelectableData, TEntityData> {
    pub selectable: TSelectableData,
    pub target: MergedTarget<TEntityData>,
    /// `None` when no merged selection of this key wrote a selection set.
    pub selection_set: Option<MergedSelectionMap<TSelectableData, TEntityData>>,
}

/// The entity a selection's selectable points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedTarget<TEntityData> {
    pub name: EntityName,
    pub data: TEntityData,
}

/// What identifies a merged selection within its map: two selections with the same key
/// are one selection in the query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergeKey {
    pub name: SelectableName,
    pub arguments: MergedArguments,
}

/// Arguments sorted by name, no name twice.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedArguments(Vec<MergedArgument>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedArgument {
    pub name: ArgumentName,
    pub value: MergedValue,
}

/// An argument value with every variable of an inlined field replaced by the value its
/// selection passed. A remaining `Variable` names a variable of the field whose map this is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MergedValue {
    Variable(VariableName),
    String(StringLiteralValue),
    Integer(i64),
    Boolean(Boolean),
    Null,
    Object(MergedObject),
}

/// Object entries sorted by name, no name twice.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedObject(Vec<MergedObjectEntry>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedObjectEntry {
    pub name: ValueKeyName,
    pub value: MergedValue,
}

pub type ProfileSelectionMap<TProfile> = MergedSelectionMap<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableData,
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityData,
>;
```

`MergedArguments` and `MergedObject` have private fields and one constructor each that sorts and rejects a repeated name, so a sorted, unique list is the only one that exists:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl MergedArguments {
    pub fn new(arguments: Vec<MergedArgument>) -> Result<Self, ArgumentName>;
    pub fn iter(&self) -> impl Iterator<Item = &MergedArgument>;
    pub fn is_empty(&self) -> bool;
}

impl MergedObject {
    pub fn new(entries: Vec<MergedObjectEntry>) -> Result<Self, ValueKeyName>;
    pub fn iter(&self) -> impl Iterator<Item = &MergedObjectEntry>;
}
```

`Err` is the repeated name. `MergedArguments::default()` is the empty list, which is sorted and unique; the printer's added `id` and `__typename` use it. `Boolean` is `isograph_parser::Boolean`, which gains `PartialOrd, Ord, Hash` derives. Ordering of interned names compares their text (`intern::string::StringId`'s `Ord`), so map order is alphabetical and independent of intern order.

Upstream's origin is `MergedSelectionMap`, `MergedServerSelection`, and `NormalizationKey` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs`). Deltas:

- Upstream's map has variants per kind (`ScalarField`, `LinkedField`, `ClientObjectSelectable`, `InlineFragment`) and keys `Discriminator` and `Id`. The core here has one node kind; scalar versus linked versus refinement is the protocol's data, and `__typename` and `id` are added by the protocol's completion, not stored. Upstream panics when two kinds meet at one key; here one key is one selectable, so they cannot.
- Upstream stores client pointers in the map for normalization. Client pointer refetch is a later doc; nothing here records them.
- Arguments are sorted by name, so `f(a: 1, b: 2)` and `f(b: 2, a: 1)` merge. Upstream keeps source order and does not merge them.
- `MergedValue` has no `List`, `Float`, or `Enum`. The iso grammar has no float or enum literal; a list argument is an error (open decision 5).

### Merging

Two maps merge by key. For a key in both, the selection sets merge: `None` and `None` is `None`, `Some` and `None` is the `Some`, `Some` and `Some` merge recursively. `selectable` and `target` are taken from the first: both came from `selectable_named` and `entity_named` of the same parent entity and name in the same database revision, so they are equal.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl<TSelectableData, TEntityData> MergedSelectionMap<TSelectableData, TEntityData> {
    pub fn empty() -> Self;

    /// `self` with every selection of `other` merged in.
    pub fn merge(self, other: Self) -> Result<Self, AliasCollision>;
}
```

The only conflict merging can find is two different keys in one map whose normalization aliases are equal: `f(s: "a b")` and `f(s: "a_b")` both alias to `f____s___s_a_b`, and a field named `f____s___s_a` selected with no arguments aliases to the same text as `f(s: "a")`. That is `AliasCollision`, an error of the query. Different arguments to one field are two keys, not a conflict; a reader alias does not take part. Open decision 1 is the alternative where they are.

### Normalization aliases

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The name a selection's value is stored under in the response and in the runtime store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizationAlias(pub String);

impl MergeKey {
    /// The field name, then one chunk per argument: the field name alone when the key has no
    /// arguments.
    pub fn normalization_alias(&self) -> NormalizationAlias;
}
```

Every field in a generated query prints with its normalization alias, including a field with no arguments and the `id` and `__typename` the document adds; a field with no arguments aliases to its own name (`firstName: firstName`).

Upstream aliases only fields with arguments. `MergedScalarFieldSelection::normalization_alias` and `MergedLinkedFieldSelection::normalization_alias` return `None` when `arguments` is empty (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:124-131` and `:159-166`), and `write_selections_for_query_text` writes `alias: ` only for `Some` (`isograph/crates/graphql_network_protocol/src/query_text.rs:90-92` and `:100-102`). The user's direction is to alias every field automatically, and this doc writes that: every field gets the deterministic alias above. Upstream shows no concrete problem with it. The runtime reads a field's value from the response under the field name followed by the same per-argument chunks (`getNetworkResponseKey`, `isograph/libs/isograph-react/src/core/cache.ts:538-552`, with `____`, `___`, `__` at `:594-596`), which for a field with no arguments is the field name, so `firstName: firstName` puts the value where the runtime reads it. GraphQL accepts an alias equal to the field name, `__typename` included (spec 2.7). The cost is query text length.

The format is upstream's `get_aliased_mutation_field_name` and `to_alias_str_chunk` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs`, `isograph/crates/isograph_lang_types/src/declarations/selection_argument.rs`): the name, then per argument `____`, the argument name, `___`, and the value chunk. Chunks: `v_<name>` for a variable, `l_<n>` for an integer, `l_true` and `l_false`, `l_null`, `s_<text>` with every character outside `[A-Za-z0-9_]` replaced by `_`, and `o_<entries>_c` with each entry `<name>__<chunk>` joined by `_`. The alias lives in the core because the runtime's normalization keys and the query must agree, whatever the protocol.

## Source fields and variables

A `field` declaration's selection set becomes a map once, keyed by the field, with its own variables left as `MergedValue::Variable`. A selection of the field substitutes the values it passed and merges the result into its parent's map.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The merged selection map of a `field` declaration's selection set, and the variables it
/// declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSelectionMap<TSelectableData, TEntityData> {
    pub variables: Vec<FieldVariable>,
    pub selections: MergedSelectionMap<TSelectableData, TEntityData>,
}

/// `$name: Type = default` on a `field` declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldVariable {
    pub name: VariableName,
    pub variable_type: VariableType,
    /// A default contains no variables.
    pub default_value: Option<MergedValue>,
}

/// A variable's type: GraphQL's type grammar, which iso type annotations also write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableType {
    NonNull(NonNullVariableType),
    Nullable(NonNullVariableType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NonNullVariableType {
    Named(EntityName),
    List(Box<VariableType>),
}

pub type ProfileFieldSelectionMap<TProfile> = FieldSelectionMap<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableData,
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityData,
>;
```

The iso `TypeAnnotation` converts: `Named(n)` is `NonNull(Named(n))`, `List(l)` is `NonNull(List(l.inner))`, `Union([w, Null])` is `Nullable(..)` of `w`'s conversion. A list whose `inner` is `None` did not parse and is `UnparsedVariableType`.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// What each variable of an inlined field is replaced by.
#[derive(Clone, Debug, PartialEq, Eq)]
enum VariableValue {
    Value(MergedValue),
    /// The selection passed no argument and the variable has no default.
    Unset,
}

/// `field_map` with each variable replaced by its value in `context`.
fn substitute<TSelectableData: Clone, TEntityData: Clone>(
    field_map: &MergedSelectionMap<TSelectableData, TEntityData>,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedSelectionMap<TSelectableData, TEntityData>, AliasCollision>;
```

The context of a selection `G(a: v)` of a field `G` with variables `$a`, `$b: Int = 3`, `$c: Int` is `{ a: Value(v), b: Value(3), c: Unset }`. `v` is already a `MergedValue` of the parent's map: a parent variable stays a `Variable` naming the parent's variable, and is substituted when the parent is inlined in turn. Upstream's origin is `VariableContext::child_variable_context` (`isograph/crates/isograph_schema/src/variable_context.rs`).

An argument whose value is a top-level `Unset` variable is removed from the key. Inside an object, `Unset` is `Null`. Delta: upstream writes `null` for both, which overrides a server-side default (`checkins(skip: null)` where the schema says `skip: Int = 0`); omitting the argument keeps the default (open decision 2). Substitution re-keys the map; two keys that become equal merge, and the result is checked for alias collisions.

A variable that is not declared by the field whose selection set uses it is `UndefinedVariable`, reported when that field's map is built, so every `Variable` in a field's map names one of its `variables`.

### The walk

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The merged selection map of the `field` declaration `entity.selectable` as `client`
/// reads the source and schema.
#[memo]
pub fn field_selection_map<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<ProfileFieldSelectionMap<TProfile>, SelectionMapError>
```

The body:

1. `inline_cycle(db, client, key)`: a cycle reachable from this field is `InlineCycle`, returned before any recursive call.
2. `source_field_declaration(db, client, entity, selectable)`: the declaration's parse.
3. Its `variable_definitions` become `FieldVariable`s, in source order.
4. Its selection set, when present, is `merge_selection_set(db, client, declaration_context, entity, selection_set)`; absent, the map is empty.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// A `field` declaration in a source file, borrowed from its literal's parse.
pub struct SourceFieldDeclaration<'db> {
    pub path: RelativePathToSourceFile,
    /// The byte offset of the literal in its file; spans in `declaration` are relative to it.
    pub literal_start: u32,
    pub declaration: &'db SelectableDeclaration,
}

/// The declaration of `entity.selectable`, found from the selectable's `definition`.
pub fn source_field_declaration<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<SourceFieldDeclaration<'_>, SelectionMapError>;
```

It calls `selectable_named(db, client, entity, selectable)`; `Err` is `Lookup`, `SelectableKind::Server` is `NotASourceField`. The selectable's `definition` is a path and the file span of the declaration's name. The function reads `parsed_iso_literals_in_file(db, client, path)` and takes the literal whose span contains that span, then its item, which is a `SelectableDeclaration`. A literal that no longer contains it is `DefinitionMoved`: an invariant between two memo results of one revision, which cannot fail while both are current; `LSPRuntimeError::UnexpectedError` is not available in a memo, so it is an error variant here.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// What a selection set's errors are located against: the declaration it is in.
#[derive(Clone, Copy)]
struct DeclarationContext<'a> {
    path: RelativePathToSourceFile,
    literal_start: u32,
    variables: &'a [FieldVariable],
}

/// The merged selection map of `selection_set`, a selection set on `parent`.
fn merge_selection_set<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    parent: EntityName,
    selection_set: &SelectionSet,
) -> Result<ProfileSelectionMap<TProfile>, SelectionMapError>;

/// What one selection on `parent` adds to its selection set's map.
fn selection_contribution<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    parent: EntityName,
    selection: &Selection,
) -> Result<ProfileSelectionMap<TProfile>, SelectionMapError>;
```

`merge_selection_set` maps every slot to `selection_contribution` and folds with `merge`, stopping at the first error. A slot with no item is `UnparsedSelection`. `selection_contribution`:

- Looks up `selectable_named(db, client, parent, selection.name)`; `Err` is `Lookup`.
- Converts `selection.arguments` to `MergedArguments` against `context.variables`: a variable not declared is `UndefinedVariable`, a repeated argument `DuplicateArgument`, a repeated object key `DuplicateObjectKey`, a list `ListArgument`, a slot with no item `UnparsedArgument`.
- `SelectableKind::Server`: looks up `entity_named(db, client, target_entity())`. `Err` is `Lookup`; `EntityOrigin::Source` is `TargetIsSourceEntity`. The nested selection set, when present, is `merge_selection_set` on the target. The result is a map of one entry.
- `SelectableKind::ClientField` or `ClientPointer`, with a directive named `loadable` on the selection: the empty map.
- `SelectableKind::ClientField` or `ClientPointer` otherwise: `field_selection_map(db, client, parent, selection.name)`, then `substitute` with the context built from the converted arguments and that map's `variables`. An argument the field does not declare is `UnknownFieldArgument`. The selection's own nested selection set is not read.

The recursion through `field_selection_map` is a memo call, so pico records the dependency on the inlined field.

### Cycles

`field Pet.a { b }` and `field Pet.b { a }` inline each other forever, and a memo that reaches itself panics in pico. Before recursing, `field_selection_map` checks for a cycle through a memo that does not recurse.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The `field` declarations whose selection sets `entity.selectable`'s selection set
/// inlines directly: source selections, not `@loadable`, at any depth under protocol
/// selections. A selection whose lookup fails is not listed.
#[memo]
pub fn inlined_fields<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> BTreeSet<SelectableKey>;

/// A path of inlined fields from `start` that returns to a field on the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineCycle(pub Vec<SelectableKey>);

/// The first cycle of inlined fields reachable from `start`, by depth-first search over
/// `inlined_fields`.
fn inline_cycle<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    start: SelectableKey,
) -> Option<InlineCycle>;
```

`inline_cycle` keeps an explicit stack (a worklist over a graph, which the iterator traits do not express). If no cycle is reachable from `F`, no field reachable from `F` is on a cycle, so the recursive `field_selection_map` calls under `F` all terminate. A field that reaches a cycle fails with `InlineCycle`; fields that do not reach it are unaffected.

`SelectableKey` is validation-diagnostics.md's `{ entity, selectable }`; it gains `PartialOrd, Ord, Hash`.

### Errors

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// Why a field's merged selection map could not be built: the error, and where it is.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct SelectionMapError {
    pub location: DefinitionLocation,
    pub kind: SelectionMapErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SelectionMapErrorKind {
    #[error("{0}")]
    Lookup(Diagnostic),
    #[error("`{}.{}` comes from the schema and has no selection set to inline.", .0.entity, .0.selectable)]
    NotASourceField(SelectableKey),
    #[error("The declaration of `{}.{}` is not where its definition says.", .0.entity, .0.selectable)]
    DefinitionMoved(SelectableKey),
    #[error("This selection did not parse.")]
    UnparsedSelection,
    #[error("This argument did not parse.")]
    UnparsedArgument,
    #[error("This variable's type did not parse.")]
    UnparsedVariableType,
    #[error("Variable `${0}` is not declared by this field.")]
    UndefinedVariable(VariableName),
    #[error("A default value cannot use the variable `${0}`.")]
    VariableInDefault(VariableName),
    #[error("Argument `{0}` is passed more than once.")]
    DuplicateArgument(ArgumentName),
    #[error("Key `{0}` appears more than once in this object.")]
    DuplicateObjectKey(ValueKeyName),
    #[error("List arguments are not supported in queries yet.")]
    ListArgument,
    #[error("`{0}` declares no variable `${1}`.", .0.field, .0.argument)]
    UnknownFieldArgument(UnknownFieldArgument),
    #[error("`{0}` is declared in source files and cannot be the type of a schema field.")]
    TargetIsSourceEntity(EntityName),
    #[error("These fields inline each other: {}.", .0.display())]
    InlineCycle(InlineCycle),
    #[error("{0}")]
    AliasCollision(AliasCollision),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownFieldArgument {
    pub field: SelectableKey,
    pub argument: ArgumentName,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{}` and `{}` have the same normalization alias.", .first.name, .second.name)]
pub struct AliasCollision {
    pub first: MergeKey,
    pub second: MergeKey,
}
```

`SelectableKey` implements `Display` as `Entity.selectable` (`derive_more::Display`), and `InlineCycle::display` joins the keys with ` -> `. `location` is the offending node's span in its file: the selection name, the argument, the variable, or for `InlineCycle` the name of the field that starts the walk. A field that inlines a broken field returns the inlined field's error unchanged, so the location is where the mistake is.

## The operation and the protocol's printer

```rust
// from crates/isograph_compiler/src/query.rs
/// What the core hands the protocol to print one entrypoint's query.
pub struct EntrypointOperation<'a, TProtocol: NetworkProtocol> {
    /// The operation name: the entrypoint's selectable name.
    pub name: SelectableName,
    pub fetch_plan: &'a TProtocol::FetchPlan,
    /// The entrypoint field's variables that the selections use, in declaration order.
    pub variables: Vec<&'a FieldVariable>,
    pub selections: &'a MergedSelectionMap<TProtocol::SelectableData, TProtocol::EntityData>,
}

/// The text of one entrypoint's query, and the file name the protocol gives a file holding
/// exactly that text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryArtifact {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
}
```

`QueryText` and `ArtifactFileName` are the existing `common_lang_types` types. Only used variables are declared, because GraphQL rejects an operation that declares a variable it does not use (spec 5.8.4); upstream's `get_used_variable_definitions` does the same. A variable is used when a `MergedValue::Variable` of that name is in `selections`, at any depth or inside an object.

```rust
// from crates/isograph_compiler/src/query.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointQueryError<TNotFetchable: std::fmt::Display, TQueryError: std::fmt::Display> {
    #[error("{0}")]
    Lookup(Diagnostic),
    #[error("{0}")]
    NotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    SelectionMap(SelectionMapError),
    #[error("{0}")]
    Protocol(TQueryError),
}

pub type ProfileEntrypointQueryError<TProfile> = EntrypointQueryError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::QueryError,
>;

/// The query of `entrypoint entity.selectable` as `client` reads the source and schema.
#[memo]
pub fn query_for_entrypoint<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<QueryArtifact, ProfileEntrypointQueryError<TProfile>> {
    let entity_value = entity_named(db, client, entity)
        .clone()
        .map_err(EntrypointQueryError::Lookup)?;
    let selectable_value = selectable_named(db, client, entity, selectable)
        .clone()
        .map_err(EntrypointQueryError::Lookup)?;
    let fetch_plan = entrypoint_fetch_plan(db, client, &entity_value, &selectable_value)
        .map_err(EntrypointQueryError::NotFetchable)?;
    let field = field_selection_map(db, client, entity, selectable)
        .as_ref()
        .map_err(|error| EntrypointQueryError::SelectionMap(error.clone()))?;
    let used = field.selections.used_variables();
    TProfile::NetworkProtocol::query_artifact(&EntrypointOperation {
        name: selectable,
        fetch_plan: &fetch_plan,
        variables: field
            .variables
            .iter()
            .filter(|variable| used.contains(&variable.name))
            .collect(),
        selections: &field.selections,
    })
    .map_err(EntrypointQueryError::Protocol)
}
```

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl<TSelectableData, TEntityData> MergedSelectionMap<TSelectableData, TEntityData> {
    /// Every variable name in the arguments of this map, at any depth.
    pub fn used_variables(&self) -> BTreeSet<VariableName>;
}
```

The operation name is the selectable name, upstream's (`isograph/crates/artifact_content/src/entrypoint_artifact.rs`, `query_name = entrypoint.name`); open decision 3.

### GraphQL printing

`GraphQLNetworkProtocol::query_artifact` is a pure function of the operation: every fact it needs (leaf or composite, abstract, has `id`) is in the `GraphQLEntityData` the core stored on each node and in the fetch plan. It reads no database.

Query text is produced in two steps. `operation_document` builds an `OperationDocument` from the merged selection map and the fetch plan, adding `id`, `__typename`, the `node` field, and the refinement. `print_operation_document` prints that document; it is the only function that writes GraphQL text, and the generation logic concatenates no strings.

Upstream has no structure between the merged map and the string. `generate_query_text` (`isograph/crates/graphql_network_protocol/src/query_text.rs:13`) pushes the header onto one `String` (`:23`), and `write_selections_for_query_text` (`:70`) recurses over the `MergedSelectionMap` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:33`, a `BTreeMap<NormalizationKey, MergedServerSelection>`) with `push_str` and `format!`, writing aliases at `:89-92` and `:99-102`. Arguments are strings from `get_serialized_arguments_for_query_text` (`:135`), values from `serialize_non_constant_value_for_graphql` (`:158`), and variable definitions from `write_variables_to_string` (`:38`). `id` and `__typename` are inserted into the map during the merge (`select_typename_and_id_fields_in_merged_selection`, `create_merged_selection_set.rs:1414`), not by the printer.

```rust
// from crates/graphql_network_protocol/src/operation_document.rs
/// A GraphQL executable document holding one operation (spec 2.3), with `id`, `__typename`,
/// and the `node` field already added: what `print_operation_document` prints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationDocument {
    pub operation_type: OperationType,
    /// The entrypoint's selectable name (open decision 3).
    pub name: SelectableName,
    /// The fetch plan's arguments, then the entrypoint field's used variables.
    pub variable_definitions: Vec<FieldVariable>,
    pub selection_set: OperationSelectionSet,
}

/// A selection set; GraphQL rejects an empty one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationSelectionSet(pub NonEmpty<OperationSelection>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationSelection {
    Field(OperationField),
    InlineFragment(OperationInlineFragment),
}

/// `alias: name(arguments) { selections }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationField {
    pub alias: NormalizationAlias,
    pub name: SelectableName,
    pub arguments: MergedArguments,
    /// `None` when the field's target is a scalar or enum.
    pub selection_set: Option<OperationSelectionSet>,
}

/// `... on type_condition { selections }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationInlineFragment {
    pub type_condition: EntityName,
    pub selection_set: OperationSelectionSet,
}
```

`OperationType` is the parser's; `FieldVariable`, `MergedArguments`, `NormalizationAlias`, and `MergedValue` are the core's. A fetch argument becomes a `FieldVariable` with the argument's name and `variable_type` and no default. `graphql_network_protocol` gains the `nonempty` workspace dependency.

```rust
// from crates/graphql_network_protocol/src/query_text.rs
pub const QUERY_TEXT_FILE_NAME: &str = "query_text.graphql";

pub(crate) fn query_artifact(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> Result<QueryArtifact, GraphQLQueryError> {
    let document = operation_document(operation)?;
    QueryArtifact {
        text: print_operation_document(document.reference()),
        file_name: QUERY_TEXT_FILE_NAME.intern().to::<ArtifactFileName>().wrap_some(),
    }
    .wrap_ok()
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLQueryError {
    #[error("Variable `${0}` is declared by the entrypoint and passed to `node`.")]
    VariableCollision(VariableName),
    #[error("`{0}` returns a scalar or enum and takes no selection set.")]
    SelectionSetOnLeaf(SelectableName),
    #[error("`{0}` returns an object, interface, or union and needs a selection set.")]
    MissingSelectionSet(SelectableName),
    #[error("`{0}` returns an input object, which cannot be selected.")]
    InputObjectTarget(SelectableName),
}
```

```rust
// from crates/graphql_network_protocol/src/operation_document.rs
/// The document `operation` sends.
pub(crate) fn operation_document(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> Result<OperationDocument, GraphQLQueryError>;

/// Where a selection set sits, which decides whether it selects `__typename`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionSetPosition {
    /// Under a field, or the operation's top level.
    Field,
    /// Under `... on T`.
    InlineFragment,
}

/// `selections`, on `target`, with `id` and `__typename` added.
fn completed_selection_set(
    target: &MergedTarget<GraphQLEntityData>,
    position: SelectionSetPosition,
    selections: &GraphQLSelectionMap,
) -> Result<OperationSelectionSet, GraphQLQueryError>;

/// One merged selection as a field or an inline fragment.
fn operation_selection(
    key: &MergeKey,
    selection: &GraphQLMergedSelection,
) -> Result<OperationSelection, GraphQLQueryError>;

pub type GraphQLSelectionMap = MergedSelectionMap<GraphQLSelectableData, GraphQLEntityData>;
pub type GraphQLMergedSelection = MergedSelection<GraphQLSelectableData, GraphQLEntityData>;
```

`operation_document`:

- `Root(root)`: `operation_type` is `root.operation_type`, `variable_definitions` are the operation's variables, and `selection_set` is `completed_selection_set(root.entity, Field, selections)`.
- `Node(fetch)`: `operation_type` is `Query`; `variable_definitions` are `fetch.arguments` then the operation's variables, and a name in both is `VariableCollision`. `selection_set` is one `OperationField` with name `node`, arguments `<argument>: $<argument>` for each fetch argument, alias the `normalization_alias` of that key, and selection set `completed_selection_set(fetch.field_target, Field, ..)` over the entity's selections (`Exact`), or over a map holding only an inline fragment on the entity whose selection set is `completed_selection_set(entity, InlineFragment, selections)` (`InlineFragment`).

`completed_selection_set` produces, in order:

1. `__typename` when the target is an `Interface` or `Union`, or when `position` is `InlineFragment`.
2. `id` when the target is an `Object` or `Interface` with `IdField::Present`, or when the map selected `id` with no arguments. That entry is not produced again in step 3.
3. Each `Field` entry of the map in key order, through `operation_selection`.
4. Each `Refinement` entry in key order, through `operation_selection`.
5. If steps 1 to 4 produced nothing, `__typename`, because GraphQL rejects an empty selection set (upstream prints `__typename` for an empty map, `query_text.rs:81-84`). `NonEmpty::from_vec` returning `None` is this case.

Every `OperationField`'s alias is `MergeKey::normalization_alias` of its key; the added `id` and `__typename` use a key with `MergedArguments::default()`, so they alias to their own names. `operation_selection` of a `Field` entry: a `Leaf` target with a selection set is `SelectionSetOnLeaf`; an `Object`, `Interface`, or `Union` target with none is `MissingSelectionSet`; an `InputObject` target is `InputObjectTarget`; a composite target's selection set is `completed_selection_set` at position `Field`. A `Refinement` entry is an `OperationInlineFragment` on its target with `completed_selection_set` at position `InlineFragment`.

`GraphQLRootFetch.entity` and `FetchRefinement::InlineFragment` carry the entity's `MergedTarget<GraphQLEntityData>` (entrypoints.md), so step 2 under the root type and under `... on Pet` reads that type's `IdField`.

```rust
// from crates/graphql_network_protocol/src/print_operation.rs
/// The text of `document`: one selection per line, two spaces of indentation per depth, one
/// newline between lines and none at the end.
pub(crate) fn print_operation_document(document: &OperationDocument) -> QueryText;

/// The lines of `selection_set`'s selections at `depth`, without its braces.
fn print_selection_set(selection_set: &OperationSelectionSet, depth: usize) -> Vec<String>;

/// `(name: value, ...)`, or the empty string when there are none.
fn print_arguments(arguments: &MergedArguments) -> String;

fn print_value(value: &MergedValue) -> String;

fn print_variable_definition(variable: &FieldVariable) -> String;

fn print_variable_type(variable_type: &VariableType) -> String;
```

`print_selection_set` returns a `Vec` because each nested selection set's lines are spliced between its opening and closing lines, and a recursive `impl Iterator` is not a nameable type.

The header is `<operation_type> <name>(<variable definitions>) {`, with the parentheses omitted when there are none, then the top-level selection set at depth 1, then `}`. A variable definition prints `$name: Type`, then ` = <default>` when it has one. `VariableType` prints `Nullable(t)` as `t`'s text without `!`, `NonNull(t)` with `!`, `List` in brackets. A field prints `<alias>: <name><arguments>`, then ` {`, its selection set at `depth + 1`, and `}` on its own line when it has one. An inline fragment prints `... on <type_condition> {`, its selection set at `depth + 1`, and `}`.

Values print as GraphQL: `$name`, integers in decimal, `true`, `false`, `null`, `{ key: value, key: value }`, and strings in double quotes with `"`, `\`, and the control characters U+0000 to U+001F escaped (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, and `\u00XX` for the rest; spec 2.9.4). Upstream writes the string's text between quotes without escaping (`query_text.rs:164`, whose comment notes the bug).

`query_artifact` returns `file_name: Some(QUERY_TEXT_FILE_NAME)`. Upstream writes no raw query file; its query text lives only in `query_text.ts` (`export default 'query HomeRoute {\...'`), so the name is new. Upstream separates selections with `,` and builds a TypeScript string with `\` line continuations; here the protocol returns plain text, and the host language encodes it as a string literal.

Upstream's origin for the completion rules is `select_typename_and_id_fields_in_merged_selection` and `selection_map_wrapped` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1414`, `:1477`) and `GraphQLNetworkProtocol::wrap_merged_selection_map` (`isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:305`) for the `node` wrapping. Deltas: `id` and `__typename` are added when the document is built, from entity data, rather than inserted into the map during the merge; every field is aliased (Normalization aliases); the text comes from one printer over `OperationDocument`.

### Config: which types get an identity field

No doc owns GraphQL config yet, and `crates/isograph_config` has no GraphQL section (`IsographProjectConfig` has `source_files`, `schema`, `output_directory`). This section documents the option that decides `completed_selection_set` step 2. v1 implements only the default, `auto`, which is the rule step 2 states (a type with a field `id: ID!`, `IdField::Present`, entrypoints.md). v1 does not add the field to `IsographProjectConfig`; the other two variants are documented and not implemented.

```json
{
  "source_files": ["src/**"],
  "schema": ["schema/**"],
  "graphql": {
    "identity_field_selection": "auto"
  }
}
```

```rust
// from crates/isograph_config/src/compilation_options.rs
/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    // `json_schema`, `source_files`, `schema`, `output_directory` unchanged
    /// Options the GraphQL network protocol reads.
    #[serde(default)]
    pub graphql: GraphQLConfig,
}

/// The `graphql` section of the config.
#[derive(Deserialize, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct GraphQLConfig {
    /// Which types the query selects an identity field on, whether or not a selection asked
    /// for it.
    pub identity_field_selection: IdentityFieldSelection,
}

/// Which types the query selects an identity field on.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityFieldSelection {
    /// `id` on every object and interface type with a field `id: ID!`. v1 implements only this.
    #[default]
    Auto,
    /// `id` only on types that are `Node` or implement `Node`. Documented; not implemented in v1.
    NodeOnly,
    /// The key field or fields only on types marked with a key directive in the schema,
    /// `type Pet @key(fields: "id")`, and no identity field on any other type. Documented; not
    /// implemented in v1.
    KeyDirective,
}
```

Naming follows `isograph_config`: snake_case serde field names (`source_files`, `output_directory`), and upstream's config enums are `rename_all = "snake_case"` or `"lowercase"` (`isograph/crates/isograph_config/src/compilation_options.rs:252`, `:265`), so the JSON values are `"auto"`, `"node_only"`, `"key_directive"`. `GraphQLConfig` keeps a named field because serde reads the JSON key from it. The `graphql` key holds options of one protocol; `isograph_config` already names `schema`, which only a schema-reading protocol uses.

`@key(fields: "...")` is the Apollo Federation directive spelling: `fields` is a selection of the key fields, so `@key(fields: "id")` and `@key(fields: "owner_id name")` both name a key. The directive's definition (`directive @key(fields: String!) on OBJECT | INTERFACE`) comes from the user's schema.

What each variant changes when it is implemented:

- `Auto` and `NodeOnly` decide only `IdField` in `graphql_entity` (entrypoints.md): `NodeOnly` is `Present` when the type has `id: ID!` and is `Node` or lists `Node` in `interfaces`. `completed_selection_set` step 2 is unchanged.
- `KeyDirective` changes `IdField::Present` to carry the key fields, because the key may be several fields or a field not named `id`; step 2 selects each of them.
- `NodeOnly` and `KeyDirective` select no identity field on some types that have an `id`. Upstream's runtime keys a record by `dataToNormalize.id` whenever the response holds one (`getDataIdOfNetworkResponse`, `isograph/libs/isograph-react/src/core/cache.ts:623-626`), so a user selection of `id` on such a type would still key the record. Under these variants the normalization AST must say which fields are the identity, rather than the runtime reading `id`; normalization-ast.md does not have that yet.
- An `id` that a selection asks for with no arguments is still selected, by step 2's second clause, under every variant.

The option reaches `graphql_entity` as a database input read by the protocol's memos; how config values enter the database is not specified in i2 yet, and is the first thing a change implementing a non-default variant specifies.

## Where the query text goes

The protocol produces the text and the file name; the host language writes the files. The artifact work replaces artifact-generation.md's string contents with structured values printed by a generic TypeScript printer: an `ArtifactFile` holds an `ArtifactValue`, which has `String`, object, and `Import(ArtifactImport { path, export })` variants, and the entrypoint artifact's value today is `{ kind: "Entrypoint", name }`. This section is written against those names; when the landed code differs, stop and fix this section first.

Generation reads queries only through its input. artifact-generation.md's input is `BTreeSet<ArtifactSource>`, from the per-file memo, with `ArtifactSource::Entrypoint(EntityNameAndSelectableName)`. The compile step pairs each entrypoint with its query before calling the host language:

```rust
// from crates/isograph_compiler/src/artifacts.rs
/// What one iso literal contributes to generation, with an entrypoint's query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactInput {
    Entity(EntityName),
    Field(EntityNameAndSelectableName),
    Entrypoint(EntrypointArtifactInput),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointArtifactInput {
    pub entrypoint: EntityNameAndSelectableName,
    pub query: QueryArtifact,
}
```

```rust
// from crates/isograph_compiler/src/host_language.rs
    /// Every artifact of a project whose iso literals are `inputs`, in `ArtifactSource` order.
    fn generate_artifacts(inputs: &[ArtifactInput]) -> Vec<Artifact>;
```

Before: `fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<Artifact>;`.

```rust
// from crates/isograph_cli/src/compile.rs
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("The query of `entrypoint {entrypoint}` could not be generated: {error}")]
pub(crate) struct EntrypointQueryFailed<TError: std::fmt::Display> {
    pub entrypoint: EntityNameAndSelectableName,
    pub error: TError,
}

/// Each source with its entrypoint's query as `input.client` reads the project. `Err` is
/// the first entrypoint, in source order, whose query failed.
fn artifact_inputs<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
    sources: BTreeSet<ArtifactSource>,
) -> Result<Vec<ArtifactInput>, EntrypointQueryFailed<ProfileEntrypointQueryError<TProfile>>> {
    sources
        .into_iter()
        .map(|source| match source {
            ArtifactSource::Entity(name) => ArtifactInput::Entity(name).wrap_ok(),
            ArtifactSource::Field(field) => ArtifactInput::Field(field).wrap_ok(),
            ArtifactSource::Entrypoint(entrypoint) => query_for_entrypoint(
                input.db,
                input.client,
                entrypoint.parent_entity_name,
                entrypoint.selectable_name,
            )
            .clone()
            .map(|query| {
                ArtifactInput::Entrypoint(EntrypointArtifactInput { entrypoint, query })
            })
            .map_err(|error| EntrypointQueryFailed { entrypoint, error }),
        })
        .collect()
}
```

`generate_artifacts` in `compile.rs` (artifact-generation.md) gains one step between `project_artifact_sources` and the host call:

```rust
// from crates/isograph_cli/src/compile.rs
    let Some(inputs) = artifact_inputs(state.compile_input().reference(), sources)
        .map_err(|error| tracing::warn!(%error, "artifacts not written"))
        .ok()
    else {
        return no_effects();
    };
    let artifacts = TProfile::HostLanguage::generate_artifacts(inputs.as_slice());
```

`TypeScriptHostLanguage::generate_artifacts`, per `ArtifactInput::Entrypoint`, writes into `<Parent>/<name>/`:

- `query_text.ts`: an `ArtifactFile` whose value is a default export of `ArtifactValue::String(query.text)`.
- `entrypoint.ts`: the entrypoint value gains the property `queryText` whose value is `ArtifactValue::Import(ArtifactImport { path: "./query_text", export: default })`. Before: `{ kind: "Entrypoint", name }`; after: `{ kind: "Entrypoint", name, queryText }`.
- When `query.file_name` is `Some(file_name)`: an `Artifact` at `<Parent>/<name>/<file_name>` whose contents are `query.text` and one newline, with no printer.

`iso.ts` is unchanged: it imports `entrypoint.ts`, whose default export's type now includes `queryText: string`.

The TypeScript host never inspects the text or the file name beyond writing them; another protocol's text (SQL, a persisted id) goes through the same path.

## Memos and invalidation

```text
query_for_entrypoint(client, E, f)
  -> entity_named(client, E), selectable_named(client, E, f)
  -> entrypoint_fetch_plan -> NetworkProtocol::fetch_plan -> graphql_fetch_plan(client, E)
       -> graphql_entity(client, E), root_operation_types(client), graphql_selectable(client, Query, node)
  -> field_selection_map(client, E, f)
       -> inlined_fields(client, ..) (via inline_cycle)
       -> parsed_iso_literals_in_file(client, path of E.f)
       -> selectable_named / entity_named of every selection and target
       -> field_selection_map(client, P, g) of every inlined field g
  -> NetworkProtocol::query_artifact (plain, no db)
```

Each memo is keyed by the narrowest identity that determines it: `field_selection_map` by the field, `query_for_entrypoint` by the entrypoint, `graphql_fetch_plan` by the entity, `root_operation_types` by the client only. The client is a key because every one reads file text through `file_contents`.

- Editing `field Pet.PetStatsCard`'s selection set re-runs its literal's parse, `field_selection_map(Pet, PetStatsCard)`, and through it the maps of fields that inline it and the queries of entrypoints that reach it (`PetDetailRoute`). `HomeRoute`'s `field_selection_map` reads only `HomeRoute` and `PetSummaryCard` and the fields they inline, so it is not re-run.
- An edit that does not change a field's map backdates there: renaming a reader alias, reordering selections, editing the resolver function body, or adding a selection already present.
- A `field_selection_map` does not depend on how its field was selected. Two selections of `PetCheckinsCard` with different arguments share one map and substitute separately.
- Editing an unrelated schema type re-runs the protocol's per-name lookups (integration.md), which backdate; no query re-runs.
- Every derived node is retained for the process lifetime (pico.md, Garbage collection). There is one `field_selection_map` per field reached and one `query_for_entrypoint` per entrypoint, per client; the compile reads as `compiler_client`, so the LSP clients add nodes only if an LSP feature reads queries.

### Error isolation

A query fails only on errors inside what it reads. A broken field fails every `field_selection_map` that inlines it, transitively, and every query that reaches one of those; no other memo reads it. A broken `Query.node` fails the fetch plan of every type fetched through `node`, and so only entrypoints whose parent is one of those types. A broken schema definition fails `root_operation_types`, which every fetch plan reads, so every entrypoint fails: the root types decide every operation's kind. A syntax error in `type Query` fails every `node` fetch plan and every `Query.*` field lookup, so entrypoints that read `Query` fail and `Mutation.SetTagline` does not.

## Tests

Facts about generated text, not snapshots. A helper in the test module returns the lines of a query text, and assertions name lines and their relative order (`assert!(lines contains "    tagline")`, "the `id` line precedes the `age` line", "no line contains `friendName`").

`crates/graphql_network_protocol/src/query_text.rs`, building `EntrypointOperation` values by hand (no database):

- `Root(Query)` with no variables and one `Field` entry `pets` targeting `Pet` (object, id present) whose map has `tagline` (leaf): text is `query HomeRoute {`, `  pets: pets {`, `    id: id`, `    tagline: tagline`, `  }`, `}`.
- `Root(Mutation)` header starts `mutation`; `Root(Subscription)` starts `subscription`.
- A variable `$limit: Int = 3` used in the map prints `query X($limit: Int = 3) {`; `[ID!]!` prints `[ID!]!`; a nullable list of nullable prints `[ID]`.
- An entry targeting a `Union` prints `__typename: __typename` first in its block; an `Interface` with id prints `__typename: __typename` then `id: id`; a `Refinement` entry prints `... on BlogItem {` after every `Field` entry of its block, with `__typename: __typename` first inside it.
- A composite entry with `Some(empty)` prints one `__typename: __typename` line in its block.
- Every line that is not the header, `}`, or an `... on` line has the form `<alias>: <name>`, where the alias is the key's `normalization_alias`.
- A leaf entry with a selection set is `SelectionSetOnLeaf`; an object entry with `None` is `MissingSelectionSet`; an input object target is `InputObjectTarget`.
- `Node` with `InlineFragment(Pet)` and one entrypoint variable `$id` is `VariableCollision(id)`; without it, the header is `query X($id: ID!) {` and the second line is `  node____id___v_id: node(id: $id) {`.
- A string argument `a"b\c` followed by a newline prints `"a\"b\\c\n"`.
- An object argument `{ id: $id, tagline: $tagline }` prints in key order with `{ ` and ` }`.
- No line ends in whitespace; the text does not end in a newline.
- `operation_document` of `Root` with an empty map has a selection set of exactly one `__typename` field aliased `__typename`; `print_operation_document` of a hand-built `OperationDocument` with one field `a` aliased `b` prints `  b: a`.

`crates/isograph_compiler/src/merged_selection.rs` (test protocol with `()` data where a case needs no protocol fields; otherwise the GraphQL profile in the next group):

- `MergedArguments::new` sorts `b, a` to `a, b`, and returns `Err(a)` for `a, a`.
- `normalization_alias` of `pet(id: $id)` is `pet____id___v_id`; of `checkins(limit: 5, skip: 0)` is `checkins____limit___l_5____skip___l_0`; of `set_pet_tagline(input: { id: $id, tagline: $tagline })` is `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c`; of a key `firstName` with no arguments is `firstName`.
- Merging `f(s: "a b")` and `f(s: "a_b")` into one map is `AliasCollision`; so is merging `f(s: "a")` with a no-argument selection of a field named `f____s___s_a`.

`crates/graphql_network_protocol/tests` or the crate's test module (GraphQL profile, TypeScript host, the pet demo's schema and source files inserted before the first lookup):

- `HomeRoute`: the first line is `query HomeRoute {`; the `pets: pets {` block contains `id: id`, `age: age`, `firstName: firstName`, `lastName: lastName`, `nickname: nickname`, `picture: picture`, `species: species`, `tagline: tagline`, each once; no line contains `PetSummaryCard`, `fullName`, or `__typename`.
- `PetDetailRoute`: the first line is `query PetDetailRoute($id: ID!) {`; a line is `  pet____id___v_id: pet(id: $id) {`; exactly one line is `    best_friend_relationship: best_friend_relationship {`, and its block contains `picture_together: picture_together` and a `best_friend: best_friend {` block with `id: id`, `firstName: firstName`, `lastName: lastName`; no line contains `checkins`, `friendName`, `OwnerBadge`, or `displayName`.
- `SpeciesRoute`: the header is `query SpeciesRoute($species: PetSpecies!) {`; lines include `  petsBySpecies____species___v_species: petsBySpecies(species: $species) {`, `    favorite_phrase: favorite_phrase`, `  viewer: viewer {`, and `  node____id___s_1: node(id: "1") {`, whose block's first two lines are `__typename: __typename` and `id: id`.
- `SetTagline`: the header is `mutation SetTagline($id: ID!, $tagline: String!) {`; the second line is the `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {` line.
- `entrypoint Pet.PetSummaryCard` with a `field`: the second line is `  node____id___v_id: node(id: $id) {`, a later line is `    ... on Pet {`.
- Removing `@loadable` from `PetCheckinsCard(limit: 5)` in `PetDetailRoute` adds the line `    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {`. A second selection `PetCheckinsCard` with no arguments adds `checkins____limit___l_3____skip___l_0` (the default); both blocks are present.
- `field Pet.Opt($x: Int) { checkins(limit: $x) { id } }` selected as `Opt` with no argument prints `checkins: checkins {` with no arguments.
- An entrypoint field declaring `$unused: Int` that nothing reads has no `$unused` in its header.
- A union refinement: `field Query.Feed { viewer { newsfeed(limit: 2, skip: 0) { asBlogItem { title } } } }` prints a `newsfeed____limit___l_2____skip___l_0: newsfeed(limit: 2, skip: 0) {` block whose first line is `__typename` and which contains `... on BlogItem {` with `title`.
- Isolation: adding `nope` to `field Pet.PetStatsCard` makes `query_for_entrypoint(Query, PetDetailRoute)` an `Err` whose `SelectionMap` location is the `nope` span in `src/PetStatsCard.ts`; `query_for_entrypoint(Query, HomeRoute)` is `==` to its value before the edit.
- Backdating, by value: renaming `friendName:` to `buddyName:` leaves `field_selection_map(Pet, PetBestFriendCard)` and `query_for_entrypoint(Query, PetDetailRoute)` `==` to their values before the edit.
- Cycle: `field Pet.a { b }`, `field Pet.b { a }`, `field Query.Loop { pets { a } }`, `entrypoint Query.Loop`: `query_for_entrypoint(Query, Loop)` is `Err` with `InlineCycle` and does not panic; `query_for_entrypoint(Query, HomeRoute)` is `Ok`. A self-cycle `field Pet.s { s }` is `InlineCycle` of one key.
- `field Pet.x { checkins(limit: $nope) { id } }` is `UndefinedVariable(nope)`; `pet(id: 1, id: 2)` is `DuplicateArgument(id)`; `PetCheckinsCard(other: 1)` is `UnknownFieldArgument`.
- `entrypoint Owner.OwnerBadge` is `NotFetchable(SourceEntity)`; `entrypoint PetStats.Card` is `NotFetchable(Protocol(NoFetchRoot))`.
- A schema field `owner: Owner` where `Owner` is only an `entity` declaration: selecting `owner { .. }` is `TargetIsSourceEntity(Owner)`.

`crates/isograph_cli/src/compile.rs` (handle seam, artifact directory set, GraphQL profile):

- The compile fire after the pet demo's files returns a `Replace` containing `Query/HomeRoute/query_text.ts`, `Query/HomeRoute/query_text.graphql`, and `Query/HomeRoute/entrypoint.ts`; the `.graphql` artifact's contents are the query text and one newline; `query_text.ts` contains the text's first line; `entrypoint.ts` contains `queryText` and an import from `./query_text`.
- Editing `PetStatsCard`'s selection set returns an `Update` whose writes are `Query/PetDetailRoute/query_text.ts` and `Query/PetDetailRoute/query_text.graphql`, and nothing else (`entrypoint.ts` imports the text and does not change).
- A failing query returns no effects; fixing it returns the write.

## Changes

Each change is one commit. Prerequisites, landed first in their own docs: source-backed-lookups.md (landed), validation-diagnostics.md, integration.md changes 1 to 3, compiler-errors.md changes 1 to 3, artifact-generation.md changes 1 to 4 and the structured `ArtifactValue` printer.

### 1. `DefinitionName::Schema` (graphql_schema_parser)

entrypoints.md, Prefactor. Independent of every other change.

### 2. Protocol data types and origins (isograph_compiler)

Status: landed with integration.md change 2. `NetworkProtocol` gained `EntityData` and `SelectableData` here and `LookupError` in change 3; `FetchPlan`, `NotFetchable`, and `QueryError` land with changes 4, 5, and 7, which read them.

`NetworkProtocol`'s six associated types; `Entity<TEntityData>`, `EntityOrigin`, `Selectable<TSelectableData>`, `SelectableKind`, `ClientPointerTarget`, `ServerSelectable`, `Selectable::target_entity()`, artifact-set-and-types.md change 1's `Wrapper`, the `Profile*` aliases; `SourceSelectableDefinition`; readers of `target_entity` switched to the method; the test protocol's types. Lookups still read source files only; every entity is `EntityOrigin::Source`. Depends on source-backed-lookups.md. Tests: the existing lookup, hover, and validation tests pass unchanged in their facts.

### 3. Protocol lookups in the core (isograph_compiler, graphql_network_protocol)

Status: landed as integration.md change 3, which owns it (open decision 7). Deltas from the text below: `NetworkProtocol::selectables` (completion) is a fifth method; `GraphQLCompositeData` has only `interfaces`, with `id_field` arriving in change 7; a refinement's definition is the target type's definition name.

`NetworkProtocol::entity` and `::selectable`; `ProtocolEntity`, `ProtocolSelectable`; `GraphQLEntityData`, `GraphQLCompositeData`, `IdField`, `GraphQLSelectableData`; `graphql_entity`, `graphql_selectable` (refinements per the integration step); `entity_named` and `selectable_named` merging source and protocol definitions; validation's "defined" checks counting both. Depends on 2 and integration.md 1 to 3. This is the schema integration step that source-backed-lookups.md's Later section names; if a schema-integration doc specifies it first, that doc's version replaces this change and the data types above move into it (open decision 7).

### 4. Root operation types and fetch plans (graphql_network_protocol)

`RootOperationTypes`, `root_operation_types`; `GraphQLFetchPlan`, `GraphQLRootFetch`, `GraphQLNodeFetch`, `GraphQLFetchArgument`, `FetchRefinement`, `GraphQLNotFetchable`, `NodeFieldError`, `NodeFieldTypeMismatch`, `graphql_fetch_plan`, `node_fetch`; `NetworkProtocol::fetch_plan`; `VariableType` and `NonNullVariableType` (in isograph_compiler, needed by `GraphQLFetchArgument`); `MergedTarget`. `OperationType` gains `strum::Display`. Depends on 1 and 3.

### 5. Entrypoint validation (isograph_compiler)

`EntrypointNotFetchableError`, `ValidationError<TNotFetchable>`, `entrypoint_fetch_plan`, `entrypoint_declaration_error`'s new check. Depends on 4 and validation-diagnostics.md.

### 6. Merged selection maps (isograph_compiler)

`MergedSelectionMap`, `MergedSelection`, `MergeKey`, `MergedArguments` (with `Default`), `MergedArgument`, `MergedValue`, `MergedObject`, `MergedObjectEntry`, `NormalizationAlias`, `merge`, `used_variables`; `FieldSelectionMap`, `FieldVariable`, `VariableValue`, `substitute`; `SourceFieldDeclaration`, `source_field_declaration`; `inlined_fields`, `InlineCycle`, `inline_cycle`; `field_selection_map`, `merge_selection_set`, `selection_contribution`; `SelectionMapError`, `SelectionMapErrorKind`, `UnknownFieldArgument`, `AliasCollision`. `isograph_parser::Boolean` and `SelectableKey` gain the `Ord` and `Hash` derives. Depends on 3.

### 7. Operations and GraphQL printing (isograph_compiler, graphql_network_protocol)

`EntrypointOperation`, `QueryArtifact`, `EntrypointQueryError`, `query_for_entrypoint`; `NetworkProtocol::query_artifact`; `GraphQLQueryError`, `QUERY_TEXT_FILE_NAME`, the GraphQL `query_artifact`; `OperationDocument`, `OperationSelectionSet`, `OperationSelection`, `OperationField`, `OperationInlineFragment`, `SelectionSetPosition`, `operation_document`, `completed_selection_set`, `operation_selection`, `GraphQLSelectionMap`, `GraphQLMergedSelection`; `print_operation_document` and its helpers; the `nonempty` dependency of `graphql_network_protocol`. Depends on 4, 5, and 6.

### 8. Query artifacts (isograph_compiler, isograph_extract_typescript, isograph_cli)

`ArtifactInput`, `EntrypointArtifactInput`, `generate_artifacts` taking `&[ArtifactInput]`, `artifact_inputs`, `EntrypointQueryFailed`, the TypeScript host writing `query_text.ts`, the raw file, and `queryText` on the entrypoint value. The test hosts ignore the new input. Depends on 7 and the artifact-generation work.

### 9. Pet demo output

The committed `demos/pet-demo/src/__isograph/` gains the query files for `HomeRoute`, `PetDetailRoute`, `SpeciesRoute`, and `SetTagline`, generated by the daemon. Depends on 8.

## Open decisions

1. Merge key. Options: selectable name and arguments, with every field printed under its normalization alias (upstream keys the same way and aliases only fields with arguments); the response name (reader alias or field name), where one response name with two argument sets is a conflict error, GraphQL's own field-merging rule. Consequences: the first lets two components select `checkins(limit: 3)` and `checkins(limit: 5)` on the same pet, and its only conflicts are alias collisions and duplicate arguments; the second makes composition of independently written fields fail. Held: the first.
2. A variable with no argument and no default. Options: omit the argument; print `null` (upstream). Consequences: `null` overrides a schema default and is an error for a non-null argument; omitting keeps the server's default and changes the normalization alias, which the runtime computes from the same key. Held: omit at the top level, `null` inside an object.
3. Operation name. Options: the selectable name (upstream: `query HomeRoute`); `Entity__name` (`query Query__HomeRoute`). Consequences: two entrypoints `Query.Foo` and `Pet.Foo` send the same operation name, which servers that key persisted operations or metrics by name conflate. Held: the selectable name, upstream's.
4. Writing when a query fails. Options: no artifacts written (artifact-generation.md's rule for a parse error); everything written except the failing entrypoint's files, which keep their previous contents. Consequences: the first keeps the generated folder consistent and blocks all output for one broken field; the second mixes a stale query with new readers. Held: the first.
5. List arguments. Options: an error in query generation; extend the normalization alias with a list chunk. Consequences: the alias format is shared with the runtime, which upstream never defined for lists (it panics); defining one is a runtime decision. Held: an error until the runtime design fixes the chunk.
6. `entrypoint Mutation.set_pet_tagline` on a schema field. Options: not allowed (held); allowed with the field's whole response selected; allowed with a selection set written in the entrypoint literal. Consequences: the grammar has no selection set on `entrypoint`, so the third is a grammar change. Held: not allowed; every root field is reachable through a `field Mutation.X { set_pet_tagline(..) { .. } }` declaration.
7. Ownership of change 3. The protocol lookups in `entity_named` and `selectable_named` are the schema integration step. Options: this doc owns it as written; a schema-integration doc owns it and this doc's change 3 is deleted. Held: this doc owns it unless that doc exists when implementation starts.
8. `ValidationError` carrying the protocol's reason. Options: generic over `TNotFetchable` (held); a `String` message from the protocol. Consequences: generic keeps the reason's structure for a later code action at the cost of a type parameter through the diagnostics path; a string discards it.
9. Fetch argument and entrypoint variable with the same name. Options: `VariableCollision` error (held); one operation variable when the types are equal. Consequences: unifying lets `field Pet.X($id: ID!)` read the id passed to `node`, a coupling between the fetch path and the field that the field cannot see.
10. Interfaces as entrypoint parents. Options: objects and interfaces (held; `ICheckin implements Node` is fetchable as `node(id:) { ... on ICheckin }`); objects only (upstream computes fetchability for interfaces too). Unions are never fetchable: they implement no interface and have no fields.
