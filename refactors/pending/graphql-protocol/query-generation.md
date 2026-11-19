# GraphQL protocol: query generation

This is the second of two files; `entrypoints.md` in this directory specifies the protocol data, `NetworkProtocol`'s lookup and fetch plan methods, and validation. This file specifies how the core turns an entrypoint into a merged selection map and then a completed selection map, what the protocol tells the core to do so, how the GraphQL protocol prints the completed map, where the text goes, and which functions are memos. The ordered change list for both files is at the end.

Every entrypoint gets a query. The core walks the entrypoint field's selection set, inlines the selection sets of the `field` declarations it selects, and merges the result into one map keyed by selectable name and arguments. The walk is protocol-agnostic: the core reads the protocol's data only through `NetworkProtocol::entity_shape` and `::selectable_shape`, which state as core values whether a target is a leaf or composite and whether a selectable is a refinement. The core then completes the map: it adds the discriminator and identity selections the protocol names, as protocol-neutral `Discriminator` and `Identity` selections, and turns refinements into conditionals. The protocol prints the completed map; the core builds the normalization AST from the same map (`normalization-ast.md` in this directory). Entities declared by `entity` in source files contribute nothing to a query, and neither do client pointers in v0.

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
    __typename: __typename
    id: id
    best_friend_relationship: best_friend_relationship {
      __typename: __typename
      best_friend: best_friend {
        __typename: __typename
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
      __typename: __typename
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

- `PetDetailRoute`'s own selections are the `pet(id: $id)` field. `fullName`, `PetStatsCard`, and `PetBestFriendCard` are `field` declarations without `to`; their selection sets are inlined (`firstName`, `lastName`, `nickname`, `stats { .. }`, `best_friend_relationship { .. }`, `id`).
- `friendName: firstName` in `PetBestFriendCard` prints as `firstName: firstName`. A reader alias is the reader's name for a value; it does not reach the query.
- Every field prints with its normalization alias: `pet(id: $id)` as `pet____id___v_id: pet(id: $id)`, and a field with no arguments under its own name (`firstName: firstName`).
- `PetCheckinsCard(limit: 5) @loadable(lazyLoadArtifact: true)` contributes nothing. Its query is the loadable field's own, a later doc.
- `bestFriend { fullName }` and `owner { OwnerBadge }` contribute nothing. `bestFriend` and `owner` are client pointers (`field Pet.bestFriend to Pet`, `field Pet.owner to Owner!`), which v0 query generation omits with their nested selection sets (Client pointers, below). The `best_friend_relationship` block comes from `PetBestFriendCard`.
- `__typename: __typename` comes first in every selection set under a selection, and `id: id` next under every type with `id: ID!`, whether or not anything selected them. The top level has neither.
- Entries are in alphabetical order after those two.

`entrypoint Pet.PetSummaryCard`, through `Node`:

```graphql
query PetSummaryCard($id: ID!) {
  node____id___v_id: node(id: $id) {
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
    __typename: __typename
    pet: pet {
      __typename: __typename
      id: id
      tagline: tagline
    }
  }
}
```

A query that cannot be generated is an error of that entrypoint only. Selecting `nope` in `field Pet.PetStatsCard` fails `PetDetailRoute`'s query (it inlines `PetStatsCard`) and leaves `HomeRoute`'s, `SpeciesRoute`'s, and `SetTagline`'s queries unchanged. While any query fails, the daemon logs the entrypoint and the error and writes no artifacts, as it does for a literal with a parse error (artifact-generation.md); open decision 4.

## What the protocol tells the core

The core walks selections without matching on the protocol's data. Four facts about that data change the walk's result: whether a selection's target entity is a leaf or has selectables, whether a selectable is a refinement, the discriminator the protocol selects in every selection set on a composite entity, and the identity it selects on each entity. The protocol states them as core values, through `NetworkProtocol` methods (entrypoints.md, Protocol data on entities and selectables):

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // associated types and the lookup and fetch plan methods unchanged

    /// What the core reads about an entity with `data`: whether a selection of it has
    /// selections, and for one that does, the identity selection the protocol adds to every
    /// selection set on it.
    fn entity_shape(data: &Self::EntityData) -> EntityShape;

    /// What a selection of a selectable with `data` is in a merged selection map.
    fn selectable_shape(data: &Self::SelectableData) -> SelectableShape;

    /// The selectable the protocol selects in every selection set on a composite entity,
    /// whose value names the response object's concrete entity.
    fn discriminator() -> SelectableName;
}
```

```rust
// from crates/isograph_compiler/src/protocol_shape.rs
/// What the core reads about an entity the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityShape {
    /// A selection of it has no selections (GraphQL scalars and enums).
    Leaf,
    /// Never the target of a selection (GraphQL input objects).
    Input,
    /// A selection of it selects its selectables; the protocol adds the discriminator and
    /// this identity selection to every selection set on it.
    Composite(IdentitySelection),
}

/// Whether the protocol selects the identity, the selectable whose value identifies the
/// response object's record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentitySelection {
    Select(SelectableName),
    Omit,
}

/// What a selection of a protocol selectable is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectableShape {
    /// A selection of the selectable's target entity.
    Direct,
    /// A conditional: its selections apply when the response object is the selectable's
    /// target entity, and select on that entity.
    Refinement,
}
```

The core decides nothing about which identity an entity has; it inserts the selections the protocol names, as the neutral `Discriminator` and `Identity` variants below, and builds the normalization AST from the same map. The discriminator is one selectable per protocol, not one per entity, because the runtime reads one discriminator response key per artifact (normalization-ast.md open decision 2); `discriminator()`'s contract is the rule "every selection set on a composite entity selects it", which is GraphQL's rule below. A protocol whose rule names a discriminator per entity would move it into `EntityShape::Composite` and carry the key per composite target in the AST.

`IdentitySelection` carries the name, so a later identity rule that names a different selectable per entity (`IdentityFieldSelection::KeyDirective`, Config below) changes only the protocol's `entity_shape`. `KeyDirective` with several key selectables changes `IdentitySelection::Select` to carry a non-empty list.

### GraphQL's shapes

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
const TYPENAME: &str = "__typename";
const ID: &str = "id";

/// An object, interface, or union is composite; `id` is its identity where `IdField` is
/// `Present`.
pub(crate) fn graphql_entity_shape(data: &GraphQLEntityData) -> EntityShape {
    match data {
        GraphQLEntityData::Leaf => EntityShape::Leaf,
        GraphQLEntityData::InputObject => EntityShape::Input,
        GraphQLEntityData::Object(composite) | GraphQLEntityData::Interface(composite) => {
            EntityShape::Composite(identity_selection(composite.id_field))
        }
        GraphQLEntityData::Union => EntityShape::Composite(IdentitySelection::Omit),
    }
}

fn identity_selection(id_field: IdField) -> IdentitySelection {
    match id_field {
        IdField::Present => IdentitySelection::Select(ID.intern().to()),
        IdField::Absent => IdentitySelection::Omit,
    }
}

pub(crate) fn graphql_selectable_shape(data: &GraphQLSelectableData) -> SelectableShape {
    match data {
        GraphQLSelectableData::Field => SelectableShape::Direct,
        GraphQLSelectableData::Refinement => SelectableShape::Refinement,
    }
}
```

`GraphQLNetworkProtocol::entity_shape` and `::selectable_shape` call these; `::discriminator` is `TYPENAME.intern().to()`. `IdField` is decided by `IdentityFieldSelection::Auto` (Config, below). `GraphQLCompositeData` gains `id_field: IdField` (entrypoints.md) in this change, its first reader.

GraphQL selects `__typename` in every selection set under a selection of an object, interface, or union, and inside every refinement. Consequences:

- Every record is self-describing: its response object holds its concrete entity's name.
- The normalizer always takes a record's entity from the response's discriminator selection. The normalization AST has no concrete entity on a selection's target, and no node for the discriminator: the normalizer reads it from every composite response object without being told (normalization-ast.md, the asymmetry its tests account for).
- A composite selection always selects at least `__typename`, so no selection set under a selection is empty, and upstream's filler (`__typename` for an empty map, `query_text.rs:81-84`) has no counterpart.
- Each response object carries one more key, which costs bytes per response.

Delta from upstream: upstream adds `__typename` only under abstract types (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1419-1427`) and inside inline fragments, and its AST names `concreteType` for object targets. Later: selecting `__typename` only where the compiler does not know the concrete entity, with the concrete entity back in the AST (normalization-ast.md, Later), is an efficiency change.

The test protocol defines no entity, so the core never calls its shape methods with data: its `entity_shape` returns `EntityShape::Leaf`, its `selectable_shape` returns `SelectableShape::Direct`, and its `discriminator` returns `"__typename"`.

## The merged selection map

The walk produces a `MergedSelectionMap`: the selections of one selection set after inlining `field` declarations, without the discriminator and identity. `complete` (The operation, below) turns the entrypoint's map into a `CompletedSelectionMap`, which adds them and is what the protocol prints and the core builds the normalization AST from. Completion runs after every merge, so each selection set receives the protocol's additions once.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The server selections of one selection set after inlining `field` declarations, keyed by
/// what identifies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelectionMap(BTreeMap<MergeKey, MergedSelection>);

/// What identifies a merged selection within its map: two selections with the same key are
/// one selection in the query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MergeKey {
    Selectable(SelectionKey),
    Conditional(Condition),
}

/// One selectable selected with one set of argument values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectionKey {
    pub name: SelectableName,
    pub arguments: MergedArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergedSelection {
    /// A selection of a protocol selectable.
    Selectable(MergedSelectableSelection),
    /// The selections of a refinement.
    Conditional(MergedConditional),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelectableSelection {
    pub key: SelectionKey,
    pub target: MergedSelectionTarget,
}

/// What a selectable selection's target entity is, which decides whether it has selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergedSelectionTarget {
    Leaf,
    Composite(MergedCompositeTarget),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedCompositeTarget {
    /// The identity the protocol adds to a selection set on the target entity.
    pub identity: IdentitySelection,
    pub selections: MergedSelectionMap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedConditional {
    pub condition: Condition,
    /// The identity the protocol adds to a selection set on the condition's entity, which
    /// the selections select on.
    pub identity: IdentitySelection,
    pub selections: MergedSelectionMap,
}

/// When a conditional's selections apply to the response object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Condition {
    /// A refinement `asT`: the response object's discriminator selection equals `T`'s name.
    SelectionEquals(EntityName),
}
```

`MergedSelectionMap`'s field is private. Its only constructors are `empty`, `single`, `merge`, and `substitute`, and each stores a selection under `selection.key()`, so a key and its selection always agree:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl MergedSelection {
    /// `Selectable(s)` is `MergeKey::Selectable(s.key)`; `Conditional(c)` is
    /// `MergeKey::Conditional(c.condition)`.
    pub fn key(&self) -> MergeKey;
}

impl MergedSelectionMap {
    pub fn empty() -> Self;

    /// The map holding only `selection`.
    pub fn single(selection: MergedSelection) -> Self;

    /// `self` with every selection of `other` merged in.
    pub fn merge(self, other: Self) -> Result<Self, MergeError>;

    /// The selections in key order.
    pub fn iter(&self) -> impl Iterator<Item = &MergedSelection>;

    /// Every variable name in the arguments of this map, at any depth.
    pub fn used_variables(&self) -> BTreeSet<VariableName>;
}
```

The completed map:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// A merged selection map with the discriminator and identity selections its protocol adds
/// at every depth below the operation's top level. Never empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedSelectionMap(BTreeMap<CompletedKey, CompletedSelection>);

/// The derived order is query order: the discriminator, the identity, selectable selections
/// by name and arguments, then conditionals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompletedKey {
    Discriminator,
    Identity,
    Selectable(SelectionKey),
    Conditional(Condition),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletedSelection {
    /// The selection of the protocol's `discriminator()`, with no arguments.
    Discriminator,
    /// The selection of the identity selectable this name names, with no arguments.
    Identity(SelectableName),
    Selectable(CompletedSelectableSelection),
    Conditional(CompletedConditional),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedSelectableSelection {
    pub key: SelectionKey,
    pub target: CompletedSelectionTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletedSelectionTarget {
    Leaf,
    Composite(CompletedSelectionMap),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedConditional {
    pub condition: Condition,
    pub selections: CompletedSelectionMap,
}

impl CompletedSelection {
    pub fn key(&self) -> CompletedKey;
}

impl CompletedSelectionMap {
    /// The selections in key order.
    pub fn iter(&self) -> impl Iterator<Item = &CompletedSelection>;
}
```

The field is private; `complete`, `top_level`, and `fetch_selection` (The operation) are the only constructors, which keeps a key and its selection in agreement and the map non-empty.

`MergedArguments`, `MergedArgument`, `MergedValue`, `MergedObject`, and `MergedObjectEntry` are unchanged:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
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

`Err` is the repeated name. `MergedArguments::default()` is the empty list, which is sorted and unique; the discriminator and identity selections use it. `Boolean` is `isograph_parser::Boolean`, which gains `PartialOrd, Ord, Hash` derives. Ordering of interned names compares their text (`intern::string::StringId`'s `Ord`), so map order is alphabetical and independent of intern order.

Delta from the stash of change 6 (`git stash show -p stash@{0}` in the `i2` clone, "WIP change 6"): its `MergedSelectionMap<TSelectableData, TEntityData>` and `MergedSelection<TSelectableData, TEntityData> { selectable, target: MergedTarget<TEntityData>, selection_set: Option<..> }` become the non-generic types above; its struct `MergeKey { name, arguments }` is `SelectionKey`, and `MergeKey` is the enum; `ProfileSelectionMap` and `ProfileFieldSelectionMap` are deleted, because no type is generic over the protocol any more. The map stores no protocol data: the walk reads `entity_shape` and `selectable_shape` once per selection and keeps only their values. The landed `MergedTarget<TEntityData>` (`crates/isograph_compiler/src/merged_selection.rs`) is deleted in change 7, together with the fetch plan payloads that read it (The fetch plan).

Before (landed, and the stash's head of the file):

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The entity a selection's selectable points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedTarget<TEntityData> {
    pub name: EntityName,
    pub data: TEntityData,
}

/// One protocol selectable selected with one set of argument values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelection<TSelectableData, TEntityData> {
    pub selectable: TSelectableData,
    pub target: MergedTarget<TEntityData>,
    /// `None` when no merged selection of this key wrote a selection set.
    pub selection_set: Option<MergedSelectionMap<TSelectableData, TEntityData>>,
}
```

After: the types above. `VariableType` and `NonNullVariableType` stay where they landed.

Upstream's origin is `MergedSelectionMap`, `MergedServerSelection`, and `NormalizationKey` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs`). Deltas:

- Upstream's `NormalizationKey` has `Discriminator`, `Id`, `ServerField`, `ClientPointer`, and `InlineFragment`; its merged selections are `ScalarField`, `LinkedField`, `ClientObjectSelectable`, and `InlineFragment`. i2's `CompletedKey` and `CompletedSelection` have the same roles under protocol-neutral names (`Discriminator`, `Identity`, `Selectable`, `Conditional`), and leaf versus composite is `CompletedSelectionTarget`. Upstream inserts `__typename` and `id` by name in the core (`select_typename_and_id_fields_in_merged_selection`, `:1414-1463`); i2's core inserts the protocol's `discriminator()` and the identity `entity_shape` names. Upstream panics when two kinds meet at one key; here a key and its selection agree by construction, and two selections at one key with different targets are `MergeError::SelectionMismatch`.
- Upstream merges a client pointer's reader selections into the parent and records a `ClientObjectSelectable` entry. v0 omits client pointers (Client pointers, below).
- Arguments are sorted by name, so `f(a: 1, b: 2)` and `f(b: 2, a: 1)` merge. Upstream keeps source order and does not merge them.
- `MergedValue` has no `List`, `Float`, or `Enum`. The iso grammar has no float or enum literal; a list argument is an error (open decision 5).

### Merging

Two maps merge by key. For a key in both:

- `Selectable` with `Leaf` and `Leaf` is `Leaf`. `Composite` and `Composite` with equal `identity` merge their `selections` recursively. Any other pair is `SelectionMismatch`.
- `Conditional` and `Conditional` with equal `identity` merge their `selections`; different `identity` is `SelectionMismatch`.

Both selections of one key came from `selectable_named` and `entity_named` of the same parent entity and name in the same database revision, so their targets and `identity` are equal and `SelectionMismatch` is not reached; it is an error variant, as `DefinitionMoved` is, because a memo cannot return `LSPRuntimeError::UnexpectedError`.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MergeError {
    #[error("{0}")]
    AliasCollision(AliasCollision),
    #[error("Two selections with one key have different targets.")]
    SelectionMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{}` and `{}` have the same normalization alias.", .first.name, .second.name)]
pub struct AliasCollision {
    pub first: SelectionKey,
    pub second: SelectionKey,
}
```

The only other conflict merging can find is two different selectable keys in one map whose normalization aliases are equal: `f(s: "a b")` and `f(s: "a_b")` both alias to `f____s___s_a_b`, and a field named `f____s___s_a` selected with no arguments aliases to the same text as `f(s: "a")`. That is `AliasCollision`, an error of the query. Different arguments to one field are two keys, not a conflict; a reader alias does not take part. Open decision 1 is the alternative where they are. Conditionals have no alias.

### Normalization aliases

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The name a selection's value is stored under in the response and in the runtime store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub struct NormalizationAlias(pub String);

impl SelectionKey {
    /// The selectable name, then one chunk per argument: the name alone when the key has no
    /// arguments.
    pub fn normalization_alias(&self) -> NormalizationAlias;
}
```

Every selection in a generated query prints with its normalization alias, including a selection with no arguments and the discriminator and identity selections, whose key is their name with `MergedArguments::default()`; a selection with no arguments aliases to its own name (`firstName: firstName`).

Upstream aliases only fields with arguments. `MergedScalarFieldSelection::normalization_alias` and `MergedLinkedFieldSelection::normalization_alias` return `None` when `arguments` is empty (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:124-131` and `:159-166`), and `write_selections_for_query_text` writes `alias: ` only for `Some` (`isograph/crates/graphql_network_protocol/src/query_text.rs:90-92` and `:100-102`). The user's direction is to alias every field automatically, and this doc writes that: every field gets the deterministic alias above. Upstream shows no concrete problem with it. The runtime reads a field's value from the response under the field name followed by the same per-argument chunks (`getNetworkResponseKey`, `isograph/libs/isograph-react/src/core/cache.ts:538-552`, with `____`, `___`, `__` at `:594-596`), which for a field with no arguments is the field name, so `firstName: firstName` puts the value where the runtime reads it. GraphQL accepts an alias equal to the field name, `__typename` included (spec 2.7). The cost is query text length.

The format is upstream's `get_aliased_mutation_field_name` and `to_alias_str_chunk` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs`, `isograph/crates/isograph_lang_types/src/declarations/selection_argument.rs`): the name, then per argument `____`, the argument name, `___`, and the value chunk. Chunks: `v_<name>` for a variable, `l_<n>` for an integer, `l_true` and `l_false`, `l_null`, `s_<text>` with every character outside `[A-Za-z0-9_]` replaced by `_`, and `o_<entries>_c` with each entry `<name>__<chunk>` joined by `_`. The alias lives in the core because the runtime's normalization keys and the query must agree, whatever the protocol.

## Source fields and variables

A `field` declaration's selection set becomes a map once, keyed by the field, with its own variables left as `MergedValue::Variable`. A selection of the field substitutes the values it passed and merges the result into its parent's map.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The merged selection map of a `field` declaration's selection set, and the variables it
/// declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSelectionMap {
    pub variables: Vec<FieldVariable>,
    pub selections: MergedSelectionMap,
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
fn substitute(
    field_map: &MergedSelectionMap,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedSelectionMap, SelectionMapErrorKind>;
```

The context of a selection `G(a: v)` of a field `G` with variables `$a`, `$b: Int = 3`, `$c: Int` is `{ a: Value(v), b: Value(3), c: Unset }`. `v` is already a `MergedValue` of the parent's map: a parent variable stays a `Variable` naming the parent's variable, and is substituted when the parent is inlined in turn. Upstream's origin is `VariableContext::child_variable_context` (`isograph/crates/isograph_schema/src/variable_context.rs`).

An argument whose value is a top-level `Unset` variable is removed from the key. Inside an object, `Unset` is `Null`. Delta: upstream writes `null` for both, which overrides a server-side default (`checkins(skip: null)` where the schema says `skip: Int = 0`); omitting the argument keeps the default (open decision 2). Substitution re-keys `Selectable` entries at every depth, inside composite targets and conditionals; two keys that become equal merge, and the result is checked for alias collisions. `Err` is `UndefinedVariable` or `Merge`, as in the stash.

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
) -> Result<FieldSelectionMap, SelectionMapError>
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
) -> Result<MergedSelectionMap, SelectionMapError>;

/// What one selection on `parent` adds to its selection set's map.
fn selection_contribution<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    parent: EntityName,
    selection: &Selection,
) -> Result<MergedSelectionMap, SelectionMapError>;
```

`merge_selection_set` maps every slot to `selection_contribution` and folds with `merge`, stopping at the first error. A slot with no item is `UnparsedSelection`. `selection_contribution`:

- Looks up `selectable_named(db, client, parent, selection.name)`; `Err` is `Lookup`.
- Converts `selection.arguments` to `MergedArguments` against `context.variables`: a variable not declared is `UndefinedVariable`, a repeated argument `DuplicateArgument`, a repeated object key `DuplicateObjectKey`, a list `ListArgument`, a slot with no item `UnparsedArgument`.
- `SelectableKind::Server(server)`: the target entity is the first inner entity of `server.target` (none is `NoTargetEntity`); `entity_named(db, client, target)`'s `Err` is `Lookup`, and `EntityOrigin::Source` is `TargetIsSourceEntity`. The target's data gives `entity_shape`, and `server.data` gives `selectable_shape`:
  - `Direct` on a `Leaf` target: a selection set on the selection is `SelectionSetOnLeaf`; otherwise one entry `Selectable(MergedSelectableSelection { key, target: Leaf })`.
  - `Direct` on a `Composite(identity)` target: no selection set is `MissingSelectionSet`; otherwise one entry `Selectable { key, target: Composite(MergedCompositeTarget { identity, selections }) }`, with `selections` the nested selection set's `merge_selection_set` on the target.
  - `Refinement` on a `Composite(identity)` target: arguments on the selection are `ArgumentsOnRefinement`, no selection set is `MissingSelectionSet`; otherwise one entry `Conditional(MergedConditional { condition: Condition::SelectionEquals(target), identity, selections })`.
  - `Refinement` on a `Leaf` target is `RefinementToNonComposite(target)`; GraphQL's refinements target only object types (`refinement_to`), so GraphQL does not reach it.
  - Either shape on an `Input` target is `InputTarget(target)`.
- `SelectableKind::ClientField` with a directive named `loadable` on the selection: the empty map.
- `SelectableKind::ClientField` otherwise: `field_selection_map(db, client, parent, selection.name)`, then `substitute` with the context built from the converted arguments and that map's `variables`. An argument the field does not declare is `UnknownFieldArgument`. The selection's own nested selection set is not read.
- `SelectableKind::ClientPointer`: the empty map, with or without `@loadable`. Its own selection set and the selection's nested selection set are not read (Client pointers, below).

`key` is `SelectionKey { name: selection.name, arguments }` with the converted arguments. Each shape error is located at the selection's name.

### Client pointers

v0 query generation omits client pointers (`field X.y to T`, `SelectableKind::ClientPointer`). The merge skips a client pointer selection and its nested selection set entirely, so neither the query text nor the normalization AST contains it or anything selected through it, and the two still match by construction. There is no error and no panic. `inlined_fields` does not list client pointers, so a pointer takes no part in cycle detection.

The data a client pointer's selections need is not fetched, so reading through a pointer finds it missing until pointers are supported. In the pet demo, `PetDetailRoute` selects `bestFriend` and `owner`, both pointers; its generated query has neither, nor `bestFriend`'s own `best_friend_relationship { best_friend { id } }` or `owner`'s `{ id }`.

Later: upstream merges the pointer's reader selections into the parent (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1291-1349`, `:1241-1265`) and records a `ClientObjectSelectable` entry, which both printers skip. Supporting pointers is a later doc, with the refetch of the target.

The recursion through `field_selection_map` is a memo call, so pico records the dependency on the inlined field.

### Cycles

`field Pet.a { b }` and `field Pet.b { a }` inline each other forever, and a memo that reaches itself panics in pico. Before recursing, `field_selection_map` checks for a cycle through a memo that does not recurse.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The `field` declarations whose selection sets `entity.selectable`'s selection set
/// inlines directly: client field selections, not `@loadable`, at any depth under protocol
/// selections. A selection whose lookup fails is not listed, and neither is a client pointer.
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
    /// `None` when the error is the lookup of the field itself, which has no one definition.
    pub location: Option<DefinitionLocation>,
    pub kind: SelectionMapErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SelectionMapErrorKind {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("`{0}` comes from the schema and has no selection set to inline.")]
    NotASourceField(SelectableKey),
    #[error("The declaration of `{0}` is not where its definition says.")]
    DefinitionMoved(SelectableKey),
    #[error("This selection did not parse.")]
    UnparsedSelection,
    #[error("This argument did not parse.")]
    UnparsedArgument,
    #[error("This variable declaration did not parse.")]
    UnparsedVariable,
    #[error("This variable's type did not parse.")]
    UnparsedVariableType,
    #[error("A variable's type is a named type, a list, or one of those or `null`.")]
    UnsupportedVariableType,
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
    #[error("`{}` declares no variable `${}`.", .0.field, .0.argument)]
    UnknownFieldArgument(UnknownFieldArgument),
    #[error("`{0}` has no entity to select into.")]
    NoTargetEntity(SelectableKey),
    #[error("`{0}` is declared in source files and cannot be the type of a schema field.")]
    TargetIsSourceEntity(EntityName),
    #[error("`{0}` returns a leaf and takes no selection set.")]
    SelectionSetOnLeaf(SelectableKey),
    #[error("`{0}` returns an entity with selectables and needs a selection set.")]
    MissingSelectionSet(SelectableKey),
    #[error("`{0}` is an input type, which cannot be selected.")]
    InputTarget(EntityName),
    #[error("The refinement `{0}` takes no arguments.")]
    ArgumentsOnRefinement(SelectableKey),
    #[error("A refinement cannot target the leaf `{0}`.")]
    RefinementToNonComposite(EntityName),
    #[error("These fields inline each other: {}.", .0.display())]
    InlineCycle(InlineCycle),
    #[error("{0}")]
    Merge(MergeError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownFieldArgument {
    pub field: SelectableKey,
    pub argument: ArgumentName,
}
```

The enum is the stash's (`git stash show -p stash@{0}`), whose deltas from the earlier text of this doc stay: `location` is optional, and `UnparsedVariable`, `UnsupportedVariableType`, and `NoTargetEntity` exist. Deltas from the stash: `SelectionSetOnLeaf`, `MissingSelectionSet`, `InputTarget`, `ArgumentsOnRefinement`, and `RefinementToNonComposite` are new; `AliasCollision(AliasCollision)` becomes `Merge(MergeError)`, which also carries `SelectionMismatch`.

`SelectableKey` implements `Display` as `Entity.selectable` (`derive_more::Display`), and `InlineCycle::display` joins the keys with ` -> `. `location` is the offending node's span in its file: the selection name, the argument, the variable, or for `InlineCycle` the name of the field that starts the walk. A field that inlines a broken field returns the inlined field's error unchanged, so the location is where the mistake is.

## The operation

`query_for_entrypoint` builds one operation's completed selection map in the core, builds the normalization AST from it (normalization-ast.md), and hands the same map to the protocol, which prints the text. The text and the AST are produced from one value.

### Where the entrypoint's selections sit

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// Where a fetch plan puts an entrypoint's selections in its operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchPosition<'a> {
    /// At the operation's top level: the entrypoint's parent is the operation's root.
    TopLevel,
    /// Under one top-level selection, whose arguments are operation variables of the same
    /// names.
    Selection(&'a FetchSelection),
}

/// The top-level selection a fetch goes through: GraphQL's `node(id: $id)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchSelection {
    pub selectable: SelectableName,
    /// In definition order.
    pub arguments: Vec<FetchArgument>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchArgument {
    pub name: InputValueName,
    /// The argument's type, made nullable when the argument has a default, so the caller
    /// may omit it (spec 5.8.5, All Variable Usages Are Allowed).
    pub variable_type: VariableType,
}
```

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // as above
    /// Where `plan` puts an entrypoint's selections.
    fn fetch_position(plan: &Self::FetchPlan) -> FetchPosition<'_>;
}
```

`FetchArgument` is the landed `GraphQLFetchArgument` (`crates/graphql_network_protocol/src/fetch_plan.rs`) moved to the core and renamed; its fields are unchanged. `InputValueName` converts to `ArgumentName` and `VariableName` with `.to()` (`string_key_one_way_conversion!` in `crates/common_lang_types/src/string_key_types.rs`). The test protocol's `fetch_position` returns `TopLevel`; its `fetch_plan` never returns `Ok`.

`fetch_position` maps the protocol's plan to the one fact the core reads. The alternative, a core `FetchPlan<TFetchData> { position, data }`, would let a GraphQL `Root` plan carry a `Selection` position; the method derives the position from the plan, so that state does not exist.

### The fetch plan

Completion in the core changes the landed `GraphQLFetchPlan` (change 4). Its `MergedTarget<GraphQLEntityData>` payloads existed so the GraphQL operation builder could add `id` and `__typename` under the root type, under `node`, and inside `... on T`. The core now adds them from `discriminator()` and `entity_shape`, and the `node` selection's selections are the entity's own (below), so no payload carries entity data. The `node` selection and its arguments become the core `FetchSelection`, which `fetch_position` returns. The refinement stays GraphQL data: it prints `... on T` and has no normalization AST node.

Before (landed):

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
pub enum GraphQLFetchPlan {
    Root(GraphQLRootFetch),
    Node(GraphQLNodeFetch),
}

pub struct GraphQLRootFetch {
    pub operation_type: OperationType,
    /// The root type, whose data decides the top-level `id` and `__typename`.
    pub entity: MergedTarget<GraphQLEntityData>,
}

pub struct GraphQLNodeFetch {
    /// `node`'s arguments, each passed as the operation variable of the same name.
    pub arguments: Vec<GraphQLFetchArgument>,
    /// The type `node` returns, which the operation builder completes like any selection
    /// target.
    pub field_target: MergedTarget<GraphQLEntityData>,
    pub refinement: FetchRefinement,
}

pub struct GraphQLFetchArgument {
    pub name: InputValueName,
    pub variable_type: VariableType,
}

pub enum FetchRefinement {
    Exact,
    InlineFragment(MergedTarget<GraphQLEntityData>),
}
```

After:

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How the GraphQL protocol fetches an entity that parents an entrypoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLFetchPlan {
    /// The entity is the root type of this operation type: its selections are the
    /// operation's.
    Root(OperationType),
    /// A `query` that selects the query root's `node` field, which returns the entity or an
    /// interface or union containing it.
    Node(GraphQLNodeFetch),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLNodeFetch {
    /// `node` and its arguments.
    pub selection: FetchSelection,
    pub refinement: FetchRefinement,
}

/// How the query text puts the entity's selections under `node`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchRefinement {
    /// `node` returns the entity itself.
    Exact,
    /// `node` returns an interface or union containing the entity, an object type: the text
    /// prints `... on Entity { }` around the entity's selections.
    InlineFragment(EntityName),
}
```

`GraphQLRootFetch` has one field once `entity` is gone, so it is replaced by its `OperationType` (AGENTS.md, a one-field struct is a newtype, and the variant then holds the payload). `GraphQLFetchArgument` is replaced by the core `FetchArgument`. `graphql_fetch_plan` and `node_fetch` change only in what they store:

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// The fetch of the object type `entity`, which declares `object`, through the `node` field
/// of the query root `query_root`.
fn node_fetch<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    query_root: EntityName,
    entity: EntityName,
    object: &GraphQLCompositeData,
) -> Result<GraphQLNodeFetch, NodeFieldError>;

/// Whether the interface or union `returns`, whose data is `returns_data`, contains the
/// object type `entity`, which declares `object`.
fn contains<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    returns: EntityName,
    returns_data: &GraphQLEntityData,
    entity: EntityName,
    object: &GraphQLCompositeData,
) -> bool;
```

`graphql_fetch_plan` step 1 binds the `Object`'s `GraphQLCompositeData` (entrypoints.md, objects only) and passes it to `node_fetch`. `node_fetch` still reads `graphql_entity(T)` of `node`'s return type, for `contains` and for `Target` and `UnknownTarget`, and no longer stores it. `contains`'s `Interface` arm is `object.interfaces.contains(&returns)`, with no match on the entity's data, because the entity is an object. `GraphQLNetworkProtocol::fetch_position` is `TopLevel` for `Root(_)` and `Selection(fetch.selection.reference())` for `Node(fetch)`.

### The `node` selection's selections

`node(id:)` returns the entity whose id is `$id`, so the entity is known by construction. The `node` selection's selections are the entity's own completed selections, and nothing is selected beside them: no `__typename` and no `id` under `node` outside the fragment, and no condition in the normalization AST. The GraphQL text still prints `... on T` around them, because GraphQL requires it to select `Pet`'s fields on a `Node`-typed field (spec 5.3.1). The text has this line and the AST does not: normalization-ast.md, Tests, states the asymmetry. `FetchRefinement::InlineFragment` is text only; conditionals come only from refinements `asT`.

For `entrypoint Pet.PetSummaryCard`:

```graphql
query PetSummaryCard($id: ID!) {
  node____id___v_id: node(id: $id) {
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

Inside the fragment, `__typename` is the discriminator every selection set under a selection gets, and `id` is `Pet`'s identity.

### Completion

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// `map`, a selection set under a selection of an entity whose identity is `identity`, with
/// the discriminator, the identity, and every composite target and conditional below it
/// completed.
pub fn complete<TProtocol: NetworkProtocol>(
    map: &MergedSelectionMap,
    identity: IdentitySelection,
) -> Result<CompletedSelectionMap, AddedSelectionCollision>;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{}` has the normalization alias of a selection the protocol adds.", .0.name)]
pub struct AddedSelectionCollision(pub SelectionKey);
```

`complete`, in order:

1. Each `Selectable` entry: `Leaf` is `CompletedSelectionTarget::Leaf`; `Composite(target)` is `Composite(complete::<TProtocol>(target.selections, target.identity)?)`.
2. Each `Conditional` entry: `CompletedConditional { condition, selections: complete::<TProtocol>(c.selections, c.identity)? }`.
3. A `Leaf` selectable entry with no arguments named `TProtocol::discriminator()` becomes `Discriminator`; one named `identity`'s `Select` name becomes `Identity(name)`. The selection a user wrote and the one the protocol adds are one selection.
4. `Discriminator` is added. `IdentitySelection::Select(i)` adds `Identity(i)`.
5. A remaining `Selectable` key whose normalization alias equals the discriminator's or the identity's name is `AddedSelectionCollision(key)`. Step 3 has absorbed the leaf selection with no arguments; what remains is a composite selectable named like one of them, which GraphQL excludes (`__typename` is not a schema field, and `IdField::Present` requires `id: ID!`).

Step 4 makes the map non-empty and puts the discriminator before every conditional in the same selection set, which the condition reads. Under `IdentitySelection::Omit` (a type whose `id` is not `ID!`), a selection of `id` stays a `Selectable` entry, in name order.

Completion runs once, on the entrypoint's whole map, after all inlining and substitution, so a selection set merged from several selections receives the protocol's additions once. A `field` declaration's map is not completed: it is merged into its selection's parent and completed with it.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl CompletedSelectionMap {
    /// The operation's top-level map under a `TopLevel` fetch: every selection completed, and
    /// nothing added at the top level itself.
    pub fn top_level<TProtocol: NetworkProtocol>(
        map: &MergedSelectionMap,
    ) -> Result<Self, TopLevelError>;

    /// The operation's top-level map under a fetch selection: that one selection.
    pub fn fetch_selection(key: SelectionKey, selections: CompletedSelectionMap) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TopLevelError {
    #[error("The entrypoint selects nothing the server returns.")]
    Empty,
    #[error("A refinement cannot be selected at the top level of an operation.")]
    Conditional,
    #[error("{0}")]
    AddedSelectionCollision(AddedSelectionCollision),
}
```

The operation's top level is not under a selection, so it gets no discriminator and no identity. GraphQL forbids an introspection field as a subscription's root field (spec 5.2.3.1), so `__typename` at the top level of `subscription` would be invalid, and the root record's entity is the root type, known without a discriminator. `top_level` completes each `Selectable` entry as `complete`'s step 1 does. A `Conditional` entry is `Conditional`, because its condition would read a discriminator the top level does not select; a GraphQL root type is an object type (spec 3.3.1) and has no refinements, so GraphQL does not reach it. An empty map is `Empty`: GraphQL rejects an empty selection set, and the top level has no discriminator to fill it. `entrypoint Query.X` whose only selection is `@loadable` is `Empty`.

### The operation's selections and variables

```rust
// from crates/isograph_compiler/src/query.rs
/// What the core hands the protocol to print one entrypoint's query.
pub struct EntrypointOperation<'a, TProtocol: NetworkProtocol> {
    /// The operation name: the entrypoint's selectable name.
    pub name: SelectableName,
    pub fetch_plan: &'a TProtocol::FetchPlan,
    /// The fetch selection's arguments, then the entrypoint field's variables that the
    /// selections use, in declaration order.
    pub variables: &'a [FieldVariable],
    pub selections: &'a CompletedSelectionMap,
}

/// The text the protocol prints for one operation, and its name for a file holding exactly
/// that text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolQueryText {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
}

/// One entrypoint's query: its text, the protocol's file name for the text, and the
/// normalization AST of its response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryArtifact {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
    pub normalization_ast: NormalizationAst,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OperationError {
    #[error("Variable `${0}` is declared by the entrypoint and is an argument of the fetch.")]
    VariableCollision(VariableName),
    #[error("The fetch passes argument `{0}` more than once.")]
    DuplicateFetchArgument(ArgumentName),
    #[error("{0}")]
    TopLevel(TopLevelError),
    #[error("{0}")]
    Completion(AddedSelectionCollision),
}
```

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // as above
    /// The text of `operation`'s query, and the protocol's name for a file holding exactly
    /// that text.
    fn query_text(operation: &EntrypointOperation<'_, Self>) -> ProtocolQueryText;
}
```

`query_text` is infallible: the core finds every error of an operation (`OperationError`, and `SelectionSetOnLeaf`, `MissingSelectionSet`, `InputTarget` in the walk, located at the selection). `NetworkProtocol` has no `QueryError` associated type, because it would be `Infallible`. The test protocol's `query_text` returns an empty `QueryText` and `file_name: None`; its `fetch_plan` never returns `Ok`, so it is not reached.

```rust
// from crates/isograph_compiler/src/query.rs
/// The fetch selection's arguments as variables with no default, then the entrypoint
/// field's variables that its selections use. A name in both is `VariableCollision`.
fn operation_variables(
    position: FetchPosition<'_>,
    field: &FieldSelectionMap,
) -> Result<Vec<FieldVariable>, OperationError>;

/// The operation's top-level selections: the entrypoint field's map at the top level, or
/// completed under the fetch selection when there is one.
fn operation_selections<TProtocol: NetworkProtocol>(
    identity: IdentitySelection,
    position: FetchPosition<'_>,
    selections: &MergedSelectionMap,
) -> Result<CompletedSelectionMap, OperationError>;
```

`TopLevel` is `CompletedSelectionMap::top_level::<TProtocol>(selections)` (`Err` is `OperationError::TopLevel`). `Selection(fetch)` is `CompletedSelectionMap::fetch_selection(key, complete::<TProtocol>(selections, identity)?)` (`Err` is `Completion`), with `key` `SelectionKey { name: fetch.selectable, arguments }` and `arguments` `MergedArguments::new` of `<argument>: $<argument>` for each fetch argument (`Err` is `DuplicateFetchArgument`). The entity's selections are under a selection, so they get the discriminator and the entity's identity.

The operation's top level under a fetch selection holds only the fetch selection.

```rust
// from crates/isograph_compiler/src/query.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointQueryError<TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    Lookup(Diagnostic),
    #[error("{0}")]
    NotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    SelectionMap(SelectionMapError),
    #[error("{0}")]
    Operation(OperationError),
    #[error("{0}")]
    IntegerOutOfRange(IntegerOutOfRange),
}

pub type ProfileEntrypointQueryError<TProfile> = EntrypointQueryError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
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
    let fetch = entrypoint_fetch_plan(db, client, &entity_value, &selectable_value)
        .map_err(EntrypointQueryError::NotFetchable)?;
    let field = field_selection_map(db, client, entity, selectable)
        .as_ref()
        .map_err(|error| EntrypointQueryError::SelectionMap(error.clone()))?;
    let position = TProfile::NetworkProtocol::fetch_position(fetch.plan.reference());
    let variables =
        operation_variables(position, field).map_err(EntrypointQueryError::Operation)?;
    let selections = operation_selections::<TProfile::NetworkProtocol>(
        fetch.identity,
        position,
        field.selections.reference(),
    )
    .map_err(EntrypointQueryError::Operation)?;
    let normalization_ast = normalization_ast(
        selections.reference(),
        SelectionKey {
            name: TProfile::NetworkProtocol::discriminator(),
            arguments: MergedArguments::default(),
        }
        .normalization_alias(),
    )
    .map_err(EntrypointQueryError::IntegerOutOfRange)?;
    let ProtocolQueryText { text, file_name } =
        TProfile::NetworkProtocol::query_text(&EntrypointOperation {
            name: selectable,
            fetch_plan: fetch.plan.reference(),
            variables: variables.as_slice(),
            selections: selections.reference(),
        });
    QueryArtifact {
        text,
        file_name,
        normalization_ast,
    }
    .wrap_ok()
}
```

`normalization_ast`, `NormalizationAst`, and `IntegerOutOfRange` are normalization-ast.md's, and land with its change 2: until then `QueryArtifact` is `{ text, file_name }` and `query_for_entrypoint` has no `normalization_ast` step. The AST carries the discriminator's response key, so the runtime names no GraphQL selectable. Only used variables are declared, because GraphQL rejects an operation that declares a variable it does not use (spec 5.8.4); upstream's `get_used_variable_definitions` does the same. A variable is used when a `MergedValue::Variable` of that name is in `selections`, at any depth or inside an object. The operation name is the selectable name, upstream's (`isograph/crates/artifact_content/src/entrypoint_artifact.rs`, `query_name = entrypoint.name`); open decision 3.

### The entrypoint's parent shape

Under a fetch selection, `query_for_entrypoint` completes the entity's map with the entity's `IdentitySelection`, which `entrypoint_fetch_plan` returns with the plan. A parent whose shape is not `Composite` has nothing to select and is reported where validation reports the other reasons.

Before (landed change 5):

```rust
// from crates/isograph_compiler/src/entrypoint.rs
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    if let EntityOrigin::Source = entity.origin {
        return EntrypointNotFetchableError::SourceEntity(entity.name).wrap_err();
    }
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntrypointNotFetchableError::Protocol)
}
```

After:

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// The protocol's plan for fetching an entrypoint's parent, and the parent's identity
/// selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointFetch<TFetchPlan> {
    pub plan: TFetchPlan,
    pub identity: IdentitySelection,
}

pub type ProfileEntrypointFetch<TProfile> = EntrypointFetch<ProfileFetchPlan<TProfile>>;

/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileEntrypointFetch<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    let data = match entity.origin.reference() {
        EntityOrigin::Source => {
            return EntrypointNotFetchableError::SourceEntity(entity.name).wrap_err();
        }
        EntityOrigin::Protocol(data) => data,
    };
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    let identity = match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(identity) => identity,
        EntityShape::Leaf | EntityShape::Input => {
            return EntrypointNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    };
    let plan = TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntrypointNotFetchableError::Protocol)?;
    EntrypointFetch { plan, identity }.wrap_ok()
}
```

`EntrypointNotFetchableError` gains:

```rust
// from crates/isograph_compiler/src/validation.rs
    #[error("`{0}` has no selectables, so it cannot parent an entrypoint.")]
    NotComposite(EntityName),
```

`entrypoint_declaration_error` locates `NotComposite` at `parent_type`, with `SourceEntity` and `Protocol`. The origin `match` binds the data the `if let` used to discard, so the shape is read from the same value the origin check ruled on. For GraphQL, `entrypoint String.X` is now `NotComposite` where it was `Protocol(NotObjectOrInterface)`; `graphql_fetch_plan(String)` itself is still `NotObject` (entrypoints.md).

### GraphQL printing

`GraphQLNetworkProtocol::query_text` is a pure function of the operation: every fact it prints is in the completed map and the fetch plan. It reads no database. It prints the map directly; there is no GraphQL document type between the map and the text, and the printer is the only function that writes GraphQL text.

Upstream has no structure between the merged map and the string. `generate_query_text` (`isograph/crates/graphql_network_protocol/src/query_text.rs:13`) pushes the header onto one `String` (`:23`), and `write_selections_for_query_text` (`:70`) recurses over the `MergedSelectionMap` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:33`, a `BTreeMap<NormalizationKey, MergedServerSelection>`) with `push_str` and `format!`, writing aliases at `:89-92` and `:99-102`. Arguments are strings from `get_serialized_arguments_for_query_text` (`:135`), values from `serialize_non_constant_value_for_graphql` (`:158`), and variable definitions from `write_variables_to_string` (`:38`). Like upstream, i2's printer walks the core's map; unlike upstream, the map is protocol-neutral and GraphQL's names arrive in it as values.

```rust
// from crates/graphql_network_protocol/src/query_text.rs
pub const QUERY_TEXT_FILE_NAME: &str = "query_text.graphql";

pub(crate) fn query_text(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> ProtocolQueryText {
    ProtocolQueryText {
        text: print_operation(operation),
        file_name: QUERY_TEXT_FILE_NAME.intern().to::<ArtifactFileName>().wrap_some(),
    }
}
```

```rust
// from crates/graphql_network_protocol/src/print_operation.rs
/// The text of `operation`: one selection per line, two spaces of indentation per depth, one
/// newline between lines and none at the end.
pub(crate) fn print_operation(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> QueryText;

/// `query`, `mutation`, or `subscription`: the root's operation type, or `query` for `node`.
fn operation_type(plan: &GraphQLFetchPlan) -> OperationType;

/// The lines of the operation's top-level selections. Under `Node` with
/// `InlineFragment(T)`, each composite selection's selections are printed inside
/// `... on T { }`.
fn print_top_level(plan: &GraphQLFetchPlan, selections: &CompletedSelectionMap) -> Vec<String>;

/// The lines of `selections` at `depth`, without braces.
fn print_selection_set(selections: &CompletedSelectionMap, depth: usize) -> Vec<String>;

/// The lines of one selection at `depth`.
fn print_selection(selection: &CompletedSelection, depth: usize) -> Vec<String>;

/// `<alias>: <name><arguments>`.
fn print_selection_head(key: &SelectionKey) -> String;

/// `(name: value, ...)`, or the empty string when there are none.
fn print_arguments(arguments: &MergedArguments) -> String;

fn print_value(value: &MergedValue) -> String;

fn print_variable_definition(variable: &FieldVariable) -> String;

fn print_variable_type(variable_type: &VariableType) -> String;
```

`print_selection_set` and `print_selection` return a `Vec` because each nested selection set's lines are spliced between its opening and closing lines, and a recursive `impl Iterator` is not a nameable type.

The header is `<operation_type> <name>(<variable definitions>) {`, with the parentheses omitted when there are none, then `print_top_level` at depth 1, then `}`. `print_top_level` is `print_selection_set(selections, 1)` for `Root(_)` and `Node` with `Exact`. For `Node` with `InlineFragment(T)` it prints each top-level selection as `print_selection` does, except that a `Selectable` with a `Composite` target prints its head and ` {`, then `... on T {` at depth 2, its selections at depth 3, `}` at depth 2, and `}` at depth 1. The core gives that map one selection, the `node` selection. A variable definition prints `$name: Type`, then ` = <default>` when it has one. `VariableType` prints `Nullable(t)` as `t`'s text without `!`, `NonNull(t)` with `!`, `List` in brackets.

`print_selection`:

- `Discriminator` prints `__typename: __typename`; `Identity(name)` prints `<name>: <name>`.
- `Selectable(selection)` prints `print_selection_head(selection.key)`; with a `Composite` target, then ` {`, its selections at `depth + 1`, and `}` on its own line. The alias is `selection.key.normalization_alias()`.
- `Conditional(conditional)` with `SelectionEquals(entity)` prints `... on <entity> {`, its selections at `depth + 1`, and `}`.

Values print as GraphQL: `$name`, integers in decimal, `true`, `false`, `null`, `{ key: value, key: value }`, and strings in double quotes with `"`, `\`, and the control characters U+0000 to U+001F escaped (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, and `\u00XX` for the rest; spec 2.9.4). Upstream writes the string's text between quotes without escaping (`query_text.rs:164`, whose comment notes the bug).

`query_text` returns `file_name: Some(QUERY_TEXT_FILE_NAME)`. Upstream writes no raw query file; its query text lives only in `query_text.ts` (`export default 'query HomeRoute {\...'`), so the name is new. Upstream separates selections with `,` and builds a TypeScript string with `\` line continuations; here the protocol returns plain text, and the host language encodes it as a string literal.

Upstream's origin for the completion rules is `select_typename_and_id_fields_in_merged_selection` and `selection_map_wrapped` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1414`, `:1477`) and `GraphQLNetworkProtocol::wrap_merged_selection_map` (`isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:305`) for the `node` wrapping. Deltas: the names are the protocol's `discriminator()` and `entity_shape`, applied by the core's `complete`, rather than names hardcoded in the core; the discriminator is selected in every selection set under a selection, and none at the operation's top level; the `node` selection holds the entity's selections with no `__typename`, `id`, or conditional beside them; every selection is aliased (Normalization aliases); the text comes from one printer over the completed map.

### Config: which types get an identity field

No doc owns GraphQL config yet, and `crates/isograph_config` has no GraphQL section (`IsographProjectConfig` has `source_files`, `schema`, `output_directory`). This section documents the option that decides GraphQL's `IdField`, which `graphql_entity_shape` turns into `IdentitySelection`. v1 implements only the default, `auto` (a type with a field `id: ID!` is `IdField::Present`, entrypoints.md). v1 does not add the field to `IsographProjectConfig`; the other two variants are documented and not implemented.

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

- `Auto` and `NodeOnly` decide only `IdField` in `graphql_entity` (entrypoints.md): `NodeOnly` is `Present` when the type has `id: ID!` and is `Node` or lists `Node` in `interfaces`. `graphql_entity_shape` and the core's `complete` are unchanged.
- `KeyDirective` changes `IdField::Present` to carry the key fields, because the key may be several fields or a field not named `id`; `IdentitySelection::Select` then carries each of them, and `complete` selects each.
- `NodeOnly` and `KeyDirective` select no identity field on some types that have an `id`. Upstream's runtime keys a record by `dataToNormalize.id` whenever the response holds one (`getDataIdOfNetworkResponse`, `isograph/libs/isograph-react/src/core/cache.ts:623-626`), so a user selection of `id` on such a type would still key the record. Under these variants the normalization AST must say which fields are the identity, rather than the runtime reading `id`; normalization-ast.md does not have that yet.
- An `id` that a selection asks for with no arguments is still selected under every variant: as `Identity` where the identity is `id`, otherwise as an ordinary selection.

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
  -> complete, normalization_ast, NetworkProtocol::query_text (plain, no db)
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

`crates/graphql_network_protocol/src/print_operation.rs`, building `EntrypointOperation` values by hand from `complete` of hand-built `MergedSelectionMap`s (no database):

- `Root(Query)` with no variables and one `Selectable` entry `pets` targeting an entity with identity `Select(id)` whose map has `tagline` (leaf): text is `query HomeRoute {`, `  pets: pets {`, `    __typename: __typename`, `    id: id`, `    tagline: tagline`, `  }`, `}`.
- `Root(Mutation)` header starts `mutation`; `Root(Subscription)` starts `subscription`; no top-level line is `__typename: __typename`.
- A variable `$limit: Int = 3` used in the map prints `query X($limit: Int = 3) {`; `[ID!]!` prints `[ID!]!`; a nullable list of nullable prints `[ID]`.
- An entry whose identity is `Omit` prints `__typename: __typename` first in its block and no `id: id`; one with `Select(id)` prints `__typename: __typename` then `id: id`; a `Conditional` entry `SelectionEquals(BlogItem)` prints `... on BlogItem {` after every `Selectable` entry of its block, with `__typename: __typename` first inside it.
- A composite entry whose map is empty and whose identity is `Omit` prints one `__typename: __typename` line in its block.
- Every line that is not the header, `}`, or an `... on` line has the form `<alias>: <name>`, where the alias is the key's `normalization_alias`.
- `Node` with `InlineFragment(Pet)`: the header is `query X($id: ID!) {`, the second line is `  node____id___v_id: node(id: $id) {`, the third is `    ... on Pet {`, and the fourth is `      __typename: __typename`. With `Exact`, the third line is `    __typename: __typename`.
- A string argument `a"b\c` followed by a newline prints `"a\"b\\c\n"`.
- An object argument `{ id: $id, tagline: $tagline }` prints in key order with `{ ` and ` }`.
- No line ends in whitespace; the text does not end in a newline.

`crates/isograph_compiler/src/query.rs` and `merged_selection.rs`, the core's operation rules (GraphQL profile in the next group where a case needs a schema):

- `complete` of a map holding a leaf `id` with no arguments under `Select(id)` has one `Identity(id)` entry and no `Selectable` entry `id`; under `Omit`, the `Selectable` entry `id` remains, in name order.
- `complete` of a map holding a composite selection named `id` with no arguments under `Select(id)` is `AddedSelectionCollision`.
- `top_level` of an empty map is `Empty`; of a map with a `Conditional` entry is `Conditional`; of a map with `pets` has no `Discriminator` entry at its own level and one under `pets`.
- `operation_variables` of `Selection` with fetch argument `id` and a used entrypoint variable `$id` is `VariableCollision(id)`.
- `fetch_selection` under `Selection` with fetch arguments `id`, `id` is `DuplicateFetchArgument(id)`.

`crates/isograph_compiler/src/merged_selection.rs` (test protocol with `()` data where a case needs no protocol fields; otherwise the GraphQL profile in the next group):

- `MergedArguments::new` sorts `b, a` to `a, b`, and returns `Err(a)` for `a, a`.
- `normalization_alias` of `pet(id: $id)` is `pet____id___v_id`; of `checkins(limit: 5, skip: 0)` is `checkins____limit___l_5____skip___l_0`; of `set_pet_tagline(input: { id: $id, tagline: $tagline })` is `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c`; of a key `firstName` with no arguments is `firstName`.
- Merging `f(s: "a b")` and `f(s: "a_b")` into one map is `AliasCollision`; so is merging `f(s: "a")` with a no-argument selection of a field named `f____s___s_a`.
- Merging a `Leaf` and a `Composite` entry at one key is `SelectionMismatch`.

`crates/graphql_network_protocol/tests` or the crate's test module (GraphQL profile, TypeScript host, the pet demo's schema and source files inserted before the first lookup):

- `HomeRoute`: the first line is `query HomeRoute {`; the `pets: pets {` block contains `id: id`, `age: age`, `firstName: firstName`, `lastName: lastName`, `nickname: nickname`, `picture: picture`, `species: species`, `tagline: tagline`, each once; the block's first line is `__typename: __typename`; no line contains `PetSummaryCard` or `fullName`; the top level has no `__typename` line.
- `PetDetailRoute`: the first line is `query PetDetailRoute($id: ID!) {`; a line is `  pet____id___v_id: pet(id: $id) {`; exactly one line is `    best_friend_relationship: best_friend_relationship {`, and its block contains `picture_together: picture_together` and a `best_friend: best_friend {` block with `id: id`, `firstName: firstName`, `lastName: lastName`; no line contains `checkins`, `friendName`, `owner`, `OwnerBadge`, or `displayName` (`owner` and `bestFriend` are client pointers, omitted).
- `SpeciesRoute`: the header is `query SpeciesRoute($species: PetSpecies!) {`; lines include `  petsBySpecies____species___v_species: petsBySpecies(species: $species) {`, `    favorite_phrase: favorite_phrase`, `  viewer: viewer {`, and `  node____id___s_1: node(id: "1") {`, whose block's first two lines are `__typename: __typename` and `id: id`.
- `SetTagline`: the header is `mutation SetTagline($id: ID!, $tagline: String!) {`; the second line is the `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {` line.
- `entrypoint Pet.PetSummaryCard` with a `field`: the second line is `  node____id___v_id: node(id: $id) {`, the third is `    ... on Pet {`; no line at depth 2 is `__typename` or `id`.
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
- `age { x }` on the leaf `age` is `SelectionSetOnLeaf` located at `age`; `stats` with no selection set is `MissingSelectionSet` located at `stats`; `asBlogItem(x: 1) { title }` is `ArgumentsOnRefinement`.
- A client pointer: `field Pet.p to Pet { stats { weight } }` selected as `p { nickname }` adds no line; `field Pet.loop to Pet { loop }` selected anywhere is not `InlineCycle`.
- `field String.X` with `entrypoint String.X` is `NotFetchable(NotComposite(String))`.

`crates/isograph_cli/src/compile.rs` (handle seam, artifact directory set, GraphQL profile):

- The compile fire after the pet demo's files returns a `Replace` containing `Query/HomeRoute/query_text.ts`, `Query/HomeRoute/query_text.graphql`, and `Query/HomeRoute/entrypoint.ts`; the `.graphql` artifact's contents are the query text and one newline; `query_text.ts` contains the text's first line; `entrypoint.ts` contains `queryText` and an import from `./query_text`.
- Editing `PetStatsCard`'s selection set returns an `Update` whose writes are `Query/PetDetailRoute/query_text.ts` and `Query/PetDetailRoute/query_text.graphql`, and nothing else (`entrypoint.ts` imports the text and does not change).
- A failing query returns no effects; fixing it returns the write.

## Changes

Each change is one commit. Prerequisites, landed first in their own docs: source-backed-lookups.md (landed), validation-diagnostics.md, integration.md changes 1 to 3, compiler-errors.md changes 1 to 3, artifact-generation.md changes 1 to 4 and the structured `ArtifactValue` printer.

### 1. `DefinitionName::Schema` (graphql_schema_parser)

entrypoints.md, Prefactor. Independent of every other change.

### 2. Protocol data types and origins (isograph_compiler)

Status: landed with integration.md change 2. `NetworkProtocol` gained `EntityData` and `SelectableData` here and `LookupError` in change 3; `FetchPlan` and `NotFetchable` landed with changes 4 and 5. `NetworkProtocol` has no `QueryError` (The operation).

`NetworkProtocol`'s six associated types; `Entity<TEntityData>`, `EntityOrigin`, `Selectable<TSelectableData>`, `SelectableKind`, `ClientPointerTarget`, `ServerSelectable`, `Selectable::target_entity()`, artifact-set-and-types.md change 1's `Wrapper`, the `Profile*` aliases; `SourceSelectableDefinition`; readers of `target_entity` switched to the method; the test protocol's types. Lookups still read source files only; every entity is `EntityOrigin::Source`. Depends on source-backed-lookups.md. Tests: the existing lookup, hover, and validation tests pass unchanged in their facts.

### 3. Protocol lookups in the core (isograph_compiler, graphql_network_protocol)

Status: landed as integration.md change 3, which owns it (open decision 7). Deltas from the text below: `NetworkProtocol::selectables` (completion) is a fifth method; `GraphQLCompositeData` has only `interfaces`, with `id_field` arriving in change 7; a refinement's definition is the target type's definition name.

`NetworkProtocol::entity` and `::selectable`; `ProtocolEntity`, `ProtocolSelectable`; `GraphQLEntityData`, `GraphQLCompositeData`, `IdField`, `GraphQLSelectableData`; `graphql_entity`, `graphql_selectable` (refinements per the integration step); `entity_named` and `selectable_named` merging source and protocol definitions; validation's "defined" checks counting both. Depends on 2 and integration.md 1 to 3. This is the schema integration step that source-backed-lookups.md's Later section names; if a schema-integration doc specifies it first, that doc's version replaces this change and the data types above move into it (open decision 7).

### 4. Root operation types and fetch plans (graphql_network_protocol)

Status: landed (`103a23b9`). `RootOperationTypes`, `root_operation_types`; `GraphQLFetchPlan`, `GraphQLRootFetch`, `GraphQLNodeFetch`, `GraphQLFetchArgument`, `FetchRefinement`, `GraphQLNotFetchable`, `NodeFieldError`, `NodeFieldTypeMismatch`, `graphql_fetch_plan`, `node_fetch`; `NetworkProtocol::fetch_plan`; `VariableType` and `NonNullVariableType`; `MergedTarget`. Change 7 revises the fetch plan types and accepts only object types.

### 5. Entrypoint validation (isograph_compiler)

Status: landed (`7e38d5c4`). `EntrypointNotFetchableError`, `ValidationError<TNotFetchable>`, `entrypoint_fetch_plan`, `entrypoint_declaration_error`'s new check. Change 7 adds `NotComposite` and `EntrypointFetch`.

### 6. Protocol shapes and merged selection maps (isograph_compiler, graphql_network_protocol)

The stash `stash@{0}` ("WIP change 6") is the starting point; its deltas are stated in The merged selection map and Errors.

- `crates/isograph_compiler/src/protocol_shape.rs`: `EntityShape`, `IdentitySelection`, `SelectableShape`.
- `NetworkProtocol::entity_shape`, `::selectable_shape`, `::discriminator`; the GraphQL implementations `graphql_entity_shape`, `identity_selection`, `graphql_selectable_shape`, `TYPENAME`, `ID`; `GraphQLCompositeData.id_field` and `IdField` (entrypoints.md) with `graphql_entity` computing it; the test protocols' implementations (`crates/isograph_compiler`, `crates/isograph_cli`, `crates/isograph_lsp` test profiles).
- `MergedSelectionMap`, `MergeKey`, `SelectionKey`, `MergedSelection`, `MergedSelectableSelection`, `MergedSelectionTarget`, `MergedCompositeTarget`, `MergedConditional`, `Condition`, `MergedSelection::key`, `MergedSelectionMap::{empty, single, merge, iter, used_variables}`, `MergeError`, `AliasCollision`; `MergedArguments` (with `Default`), `MergedArgument`, `MergedValue`, `MergedObject`, `MergedObjectEntry`, `NormalizationAlias`, `SelectionKey::normalization_alias`.
- `FieldSelectionMap`, `FieldVariable`, `VariableValue`, `substitute`; `SourceFieldDeclaration`, `source_field_declaration`; `inlined_fields` (client fields only), `InlineCycle`, `inline_cycle`; `field_selection_map`, `merge_selection_set`, `selection_contribution` with the shape errors and the client pointer omission; `SelectionMapError`, `SelectionMapErrorKind`, `UnknownFieldArgument`.
- `isograph_parser::Boolean` and `SelectableKey` gain the `Ord` and `Hash` derives.
- The `crates/isograph_compiler/src/merged_selection.rs` tests and the walk tests of the GraphQL group above that do not read query text.

The landed `MergedTarget` stays until change 7. Depends on 3 and 5.

### 7. Completion, operations, and GraphQL printing (isograph_compiler, graphql_network_protocol)

- Core: `CompletedSelectionMap`, `CompletedKey`, `CompletedSelection`, `CompletedSelectableSelection`, `CompletedSelectionTarget`, `CompletedConditional`, `CompletedSelection::key`, `CompletedSelectionMap::{iter, top_level, fetch_selection}`, `complete`, `AddedSelectionCollision`, `TopLevelError`.
- Core: `FetchPosition`, `FetchSelection`, `FetchArgument`, `NetworkProtocol::fetch_position`; `EntrypointFetch`, `ProfileEntrypointFetch`, `entrypoint_fetch_plan` returning it, `EntrypointNotFetchableError::NotComposite` and its location in `entrypoint_declaration_error`.
- Core: `EntrypointOperation`, `ProtocolQueryText`, `QueryArtifact { text, file_name }`, `OperationError`, `EntrypointQueryError` without `IntegerOutOfRange`, `operation_variables`, `operation_selections`, `query_for_entrypoint` without the `normalization_ast` step; `NetworkProtocol::query_text`; the test protocols' `fetch_position` and `query_text`. `MergedTarget` is deleted.
- GraphQL: `GraphQLFetchPlan::Root(OperationType)`, `GraphQLNodeFetch { selection, refinement }`, `FetchRefinement::InlineFragment(EntityName)`; `GraphQLRootFetch` and `GraphQLFetchArgument` deleted; `node_fetch` and `contains` taking the object's `GraphQLCompositeData`; objects only (`NotObject`), entrypoints.md, Delta from the landed fetch plan; `GraphQLNetworkProtocol::fetch_position`.
- GraphQL: `QUERY_TEXT_FILE_NAME`, `query_text`, `print_operation`, `operation_type`, `print_top_level`, `print_selection_set`, `print_selection`, `print_selection_head`, `print_arguments`, `print_value`, `print_variable_definition`, `print_variable_type`; `GraphQLNetworkProtocol::query_text`.
- Tests: the printer tests, the core operation-rule tests, the profile tests that read query text, and the fetch plan and validation test updates.

Depends on 4, 5, and 6.

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
