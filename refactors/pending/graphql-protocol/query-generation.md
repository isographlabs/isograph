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
    __typename
    id
    best_friend_relationship {
      __typename
      best_friend {
        __typename
        id
        firstName
        lastName
      }
      picture_together
    }
    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {
      __typename
      id
      location
      time
    }
    firstName
    lastName
    nickname
    stats {
      __typename
      cuteness
      energy
      hunger
      intelligence
      sociability
      weight
    }
  }
}
```

followed by one newline. `query_text.ts` default-exports the same text as a string. `entrypoint.ts`'s default export gains `queryText`, imported from `./query_text`.

The text above is the `expanded` query text format, which the pet demo's `isograph.config.json` sets:

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src",
  "graphql": {
    "query_text_format": "expanded"
  }
}
```

A config without `graphql.query_text_format` gets `collapsed`, one line with a space only between two tokens that would otherwise lex as one. `HomeRoute` in `collapsed`:

```graphql
query HomeRoute{pets{__typename id age firstName lastName nickname picture species tagline}}
```

and `SetTagline`:

```graphql
mutation SetTagline($id:ID!$tagline:String!){set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c:set_pet_tagline(input:{id:$id tagline:$tagline}){__typename pet{__typename id tagline}}}
```

Both `query_text.ts` and `query_text.graphql` hold the configured format (Config: query text format, below).

What the text shows:

- `PetDetailRoute`'s own selections are the `pet(id: $id)` field. `fullName`, `PetStatsCard`, `PetCheckinsCard`, and `PetBestFriendCard` are `field` declarations without `to`; their selection sets are inlined (`firstName`, `lastName`, `nickname`, `stats { .. }`, `checkins(..) { .. }`, `best_friend_relationship { .. }`, `id`).
- `friendName: firstName` in `PetBestFriendCard` prints as `firstName`. A reader alias is the reader's name for a value; it does not reach the query.
- A field with arguments prints with its normalization alias: `pet(id: $id)` as `pet____id___v_id: pet(id: $id)`. A field with no arguments prints its name (`firstName`), as upstream does.
- `PetCheckinsCard(limit: 5)` inlines `checkins(limit: $limit, skip: 0)` with `$limit` replaced by `5`.
- `bestFriend { fullName }` and `owner { OwnerBadge }` contribute nothing. `bestFriend` and `owner` are client pointers (`field Pet.bestFriend to Pet`, `field Pet.owner to Owner!`), which v0 query generation omits with their nested selection sets (Client pointers, below). The `best_friend_relationship` block comes from `PetBestFriendCard`.
- `__typename` comes first in every selection set under a selection, and `id` next under every type with `id: ID!`, whether or not anything selected them. The top level has neither.
- Entries are in alphabetical order after those two.

`entrypoint Pet.PetSummaryCard` has no query: v1 entrypoints are on root operation types only (entrypoints.md), and it is a validation error on `Pet`.

`entrypoint Mutation.SetTagline`:

```graphql
mutation SetTagline($id: ID!, $tagline: String!) {
  set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {
    __typename
    pet {
      __typename
      id
      tagline
    }
  }
}
```

A query that cannot be generated is an error of that entrypoint only. Selecting `nope` in `field Pet.PetStatsCard` fails `PetDetailRoute`'s query (it inlines `PetStatsCard`) and leaves `HomeRoute`'s, `SpeciesRoute`'s, and `SetTagline`'s queries unchanged. While any query fails, the daemon logs the entrypoint and the error and writes no artifacts, as it does for a literal with a parse error (artifact-generation.md); open decision 4.

## What the protocol tells the core

The core walks selections without matching on the protocol's data. Six facts about that data change the result: whether a selection's target entity is a leaf or has selectables, whether that entity is concrete or abstract, whether a selectable is a refinement, the discriminator the protocol selects in every selection set on a composite entity, the identity it selects on each entity, and which entities are root entities. The protocol states them as core values, through `NetworkProtocol` methods (entrypoints.md, Protocol data on entities and selectables). The first five depend only on one entity's or selectable's data and are read in the walk; the root entities depend on the schema definition and arrive with the fetch plan (Root entities, below).

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // associated types and the lookup and fetch plan methods unchanged

    /// What the core reads about an entity with `data`: whether a selection of it has
    /// selections, and for one that does, whether the entity is concrete and the identity
    /// selection the protocol adds to every selection set on it.
    fn entity_shape(data: &Self::EntityData) -> EntityShape;

    /// What a selection of a selectable with `data` is in a merged selection map.
    fn selectable_shape(data: &Self::SelectableData) -> SelectableShape;

    /// The selectable the protocol selects in every selection set on a composite entity,
    /// whose value names the response object's concrete entity.
    fn discriminator() -> SelectableName;

    /// The root entities of the operation `plan` describes.
    fn root_entities(plan: &Self::FetchPlan) -> RootEntities;
}
```

Before (landed): `entity_shape`, `selectable_shape`, and `discriminator`, with `EntityShape::Composite(IdentitySelection)`. After: `root_entities` is new, and `EntityShape::Composite` holds a `CompositeShape` (change 10).

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
    /// the shape's identity selection to every selection set on it.
    Composite(CompositeShape),
}

/// What the core reads about a composite entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeShape {
    pub concreteness: Concreteness,
    pub identity: IdentitySelection,
}

/// Whether the entity is the concrete entity of every response object of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Concreteness {
    /// Every response object of it is of this entity (GraphQL object types).
    Concrete,
    /// A response object of it is of one of several entities, which its discriminator names
    /// (GraphQL interfaces and unions).
    Abstract,
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

The core decides nothing about which identity an entity has; it inserts the selections the protocol names, as the neutral `Discriminator` and `Identity` variants below, and builds the normalization AST from the same map. The discriminator is one selectable per protocol, not one per entity, because the runtime reads one discriminator response key per artifact (normalization-ast.md, The discriminator); `discriminator()`'s contract is the rule "every selection set on a composite entity selects it", which is GraphQL's rule below. A protocol whose rule names a discriminator per entity would move it into `EntityShape::Composite` and carry the key per composite target in the AST.

`IdentitySelection` carries the name, so a later identity rule that names a different selectable per entity (`IdentityFieldSelection::KeyDirective`, Config below) changes only the protocol's `entity_shape`. `KeyDirective` with several key selectables changes `IdentitySelection::Select` to carry a non-empty list.

`Concreteness` is upstream's `ConcreteTargetEntityName` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:155`, `:820`), which the normalization AST prints as `concreteType` (`isograph/crates/artifact_content/src/normalization_ast_text.rs:68`) and the normalizer reads as `astNode.concreteType ?? response.__typename` (`isograph/libs/isograph-react/src/core/cache.ts:296`, `:323`, `:413`, `:607`). The completed map and the normalization AST carry the target's entity name when the target is `Concrete` (`RecordEntity`, Completion).

### Root entities

A root entity has exactly one record, at a fixed id (`FixedRecordId::root()`, Completion). Every selection whose target entity, or conditional whose entity, is a root entity is normalized onto that record, at any depth, as the operation's top level is. The compiler decides this when it generates the query; the runtime compares no discriminator value and holds no list of root entities.

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// The root entities of one operation. Each has one record, at `FixedRecordId::root()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootEntities {
    /// The root entity whose record holds the operation's top-level selections.
    pub operation: EntityName,
    /// The other root entities.
    pub others: BTreeSet<EntityName>,
}

impl RootEntities {
    /// `Fixed(FixedRecordId::root())` when `entity` is `operation` or one of `others`;
    /// otherwise `identity`'s `Select(name)` as `Select(name)` and `Omit` as `Omit`.
    pub fn identity_of(&self, entity: EntityName, identity: IdentitySelection) -> CompletedIdentity;
}
```

`root_entities` reads the plan, not the database: the plan exists only when `fetch_plan` returned `Ok`, so the root operation types it was decided against were readable, and no lookup is left to fail. Reading the root operation types in `entity_shape` instead would make every `field_selection_map` depend on the schema definition, and a broken `schema {}` would fail every field map, which entrypoints.md keeps out of entity data for the same reason (GraphQL entity data). The test protocol's `root_entities` returns `RootEntities { operation: "Query".intern().to(), others: BTreeSet::new() }`; its `fetch_plan` never returns `Ok`, so it is not reached.

GraphQL's root entities are the schema's root operation types (`GraphQLFetchPlan.roots`, The fetch plan). `schema { query: RootQuery }` makes `RootQuery` the root entity; the name `Query` is not special.

Delta from upstream: upstream's normalizer compares `__typename === 'Query'` and returns `ROOT_ID` (`isograph/libs/isograph-react/src/core/cache.ts:614-618`, under the comment `// TODO do not hard code this value here`). i2 decides it from the schema's root operation types when the query is generated.

### GraphQL's shapes

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// The selectable GraphQL selects in every selection set on a composite type.
pub(crate) const TYPENAME: &str = "__typename";
const ID: &str = "id";

/// An object, interface, or union is composite; an object is concrete; `id` is the identity
/// where `IdField` is `Present`.
pub(crate) fn graphql_entity_shape(data: &GraphQLEntityData) -> EntityShape {
    match data {
        GraphQLEntityData::Scalar(_) | GraphQLEntityData::Enum => EntityShape::Leaf,
        GraphQLEntityData::InputObject => EntityShape::Input,
        GraphQLEntityData::Object(composite) => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Concrete,
            identity: identity_selection(composite.id_field),
        }),
        GraphQLEntityData::Interface(composite) => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: identity_selection(composite.id_field),
        }),
        GraphQLEntityData::Union => EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: IdentitySelection::Omit,
        }),
    }
}

fn identity_selection(id_field: IdField) -> IdentitySelection {
    match id_field {
        IdField::Present => IdentitySelection::Select(ID.intern().to()),
        IdField::Absent => IdentitySelection::Omit,
    }
}

/// A field or input field is a direct selection; a refinement is a conditional.
pub(crate) fn graphql_selectable_shape(data: &GraphQLSelectableData) -> SelectableShape {
    match data {
        GraphQLSelectableData::Field | GraphQLSelectableData::InputField => {
            SelectableShape::Direct
        }
        GraphQLSelectableData::Refinement => SelectableShape::Refinement,
    }
}
```

Delta from entrypoints.md's `GraphQLEntityData`: the landed enum has `Scalar(EntityName)` and `Enum` where entrypoints.md has `Leaf`, and `GraphQLSelectableData` has `InputField`, both from the artifact-types work. `Scalar` and `Enum` are `Leaf`. `InputField` is `Direct`: an input field is a selectable of an input object, and a selection on an input object is never reached, because a selection whose target is an input object is `InputTarget` and an input object cannot parent an entrypoint (`NotComposite`). `id_field` is computed by `id_field(db, client, name)` in `protocol_data.rs`, which `graphql_entity` calls once per composite type.

`GraphQLNetworkProtocol::entity_shape` and `::selectable_shape` call these; `::discriminator` is `TYPENAME.intern().to()`. `IdField` is decided by `IdentityFieldSelection::Auto` (Config, below). `GraphQLCompositeData` gains `id_field: IdField` (entrypoints.md) in this change, its first reader.

GraphQL selects `__typename` in every selection set under a selection of an object, interface, or union, and inside every refinement. Consequences:

- Every record is self-describing: its response object holds its concrete entity's name.
- The normalizer takes a record's entity from the AST when the target is `Concrete` (`RecordEntity::Known`) and from the response's discriminator selection when it is `Abstract` (`RecordEntity::Discriminator`). The normalization AST has no node for the discriminator: the normalizer reads it from a composite response object without being told (normalization-ast.md, the asymmetry its tests account for).
- A composite selection always selects at least `__typename`, so no selection set under a selection is empty, and upstream's filler (`__typename` for an empty map, `query_text.rs:81-84`) has no counterpart.
- Each response object carries one more key, which costs bytes per response.

Delta from upstream: upstream adds `__typename` only under abstract types (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1419-1427`) and inside inline fragments. Both name the concrete entity of object targets in the AST (`concreteType`; i2's `RecordEntity::Known`). Later: selecting `__typename` only under `Abstract` targets and inside refinements (normalization-ast.md, Later) is an efficiency change.

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
    /// The target entity, which completion compares with the root entities and names in
    /// `RecordEntity::Known`.
    pub entity: EntityName,
    /// The target entity's concreteness and the identity the protocol adds to a selection
    /// set on it.
    pub shape: CompositeShape,
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
    Composite(CompletedCompositeTarget),
}

/// A completed selection set on a composite target entity, and how its response objects'
/// records are found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedCompositeTarget {
    pub entity: RecordEntity,
    pub identity: CompletedIdentity,
    pub selections: CompletedSelectionMap,
}

/// The conditional's records are of the condition's entity, so it carries no `RecordEntity`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedConditional {
    pub condition: Condition,
    pub identity: CompletedIdentity,
    pub selections: CompletedSelectionMap,
}

/// Which entity a response object's record is of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordEntity {
    /// The target entity is `Concrete`: every response object is of this entity.
    Known(EntityName),
    /// The target entity is `Abstract`: the response object's discriminator names its entity.
    Discriminator,
}

/// How a response object's record id is found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletedIdentity {
    /// The value of the identity selection, which the map holds as `Identity(name)`.
    Select(SelectableName),
    /// The entity has no identity selection; the normalizer derives the record id from the
    /// parent record and the path.
    Omit,
    /// The entity is a root entity: every response object of it is the record with this id.
    Fixed(FixedRecordId),
}

/// A record id the compiler assigns.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub struct FixedRecordId(pub String);

impl FixedRecordId {
    /// `"__ROOT"`: the id of a root entity's one record, upstream's `ROOT_ID`
    /// (`isograph/libs/isograph-react/src/core/IsographEnvironment.ts:163`).
    pub fn root() -> Self;
}

impl CompletedSelection {
    pub fn key(&self) -> CompletedKey;
}

impl CompletedSelectionMap {
    /// The selections in key order.
    pub fn iter(&self) -> impl Iterator<Item = &CompletedSelection>;
}
```

The field is private; `complete` and `top_level` (The operation) are the only constructors, which keeps a key and its selection in agreement and the map non-empty. `CompletedCompositeTarget` and `CompletedConditional` are built only by `complete` and `top_level`, which write `identity` and the map's `Identity(name)` entry from one `CompletedIdentity`, so `Select(name)` holds exactly when the map holds `Identity(name)`.

`RecordEntity` and `CompletedIdentity` are two independent facts. A root entity's selection is `Known(Query)` with `Fixed(FixedRecordId::root())`; an object with `id: ID!` is `Known(Pet)` with `Select(id)`; a union is `Discriminator` with `Omit`. `CompletedIdentity` is wider than `IdentitySelection` by `Fixed`: `entity_shape` cannot return `Fixed`, because rootness is not entity data (Root entities).

Before (landed change 7): `CompletedSelectionTarget::Composite(CompletedSelectionMap)`, `CompletedConditional { condition, selections }`, and `MergedCompositeTarget { identity, selections }`. After: the types above (change 10).

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

- `Selectable` with `Leaf` and `Leaf` is `Leaf`. `Composite` and `Composite` with equal `entity` and `shape` merge their `selections` recursively. Any other pair is `SelectionMismatch`.
- `Conditional` and `Conditional` with equal `identity` merge their `selections`; different `identity` is `SelectionMismatch`.

Both selections of one key came from `selectable_named` and `entity_named` of the same parent entity and name in the same database revision, so their targets, `shape`, and `identity` are equal and `SelectionMismatch` is not reached; it is an error variant, as `DefinitionMoved` is, because a memo cannot return `LSPRuntimeError::UnexpectedError`.

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

The only other conflict merging can find is two different selectable keys in one map whose normalization aliases are equal: a field named `f____s___s_a` selected with no arguments aliases to the same text as `f(s: "a")`. Two different argument sets of one field never have equal aliases. That is `AliasCollision`, an error of the query. Different arguments to one field are two keys, not a conflict; a reader alias does not take part. Open decision 1 is the alternative where they are. Conditionals have no alias.

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

Change 10 adds the response key the normalization AST carries, which states whether the query prints an alias:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// The key of a selection's value in the response object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResponseKey {
    /// The selection has no arguments: the query prints no alias, and the key is the
    /// selectable name.
    SelectableName,
    /// The selection has arguments: the query prints this alias.
    Alias(NormalizationAlias),
}

impl SelectionKey {
    /// `SelectableName` when the key has no arguments, otherwise `Alias` of
    /// `normalization_alias`.
    pub fn response_key(&self) -> ResponseKey;
}
```

`print_selection_head` prints `<alias>: ` exactly for `ResponseKey::Alias`, so the printer and the AST read one decision. `ResponseKey` is an enum rather than `Option<NormalizationAlias>` so the case with no alias names what the key is.

A selection with arguments prints with its normalization alias (`pet____id___v_id: pet(id: $id)`); a selection without arguments prints its bare name (`firstName`), and so do the discriminator and identity selections. `normalization_alias` is defined for every key, and for a key with no arguments it is the selectable name, the response key under which GraphQL returns the value. Collision detection compares these response keys for every selectable key, with or without arguments, so a field literally named `f____s___s_a` still collides with `f(s: "a")`.

This is upstream's rule: `MergedScalarFieldSelection::normalization_alias` and `MergedLinkedFieldSelection::normalization_alias` return `None` when `arguments` is empty (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:124-131` and `:159-166`), and `write_selections_for_query_text` writes `alias: ` only for `Some` (`isograph/crates/graphql_network_protocol/src/query_text.rs:90-92` and `:100-102`). The runtime reads a field's value from the response under the field name followed by the same per-argument chunks (`getNetworkResponseKey`, `isograph/libs/isograph-react/src/core/cache.ts:538-552`, with `____`, `___`, `__` at `:594-596`), which for a field with no arguments is the field name.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// `text` with each ASCII letter and digit kept and every other character, `_` included,
/// written as `_<its code point in decimal>_`. The result matches `[_0-9A-Za-z]*`, and an `_`
/// that opens an escape is followed by a digit, so the text ends before the first `_` outside an
/// escape that is followed by a letter or another `_`.
fn alias_text(text: &str) -> String;
```

The layout is upstream's `get_aliased_mutation_field_name` and `to_alias_str_chunk` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs`, `isograph/crates/isograph_lang_types/src/declarations/selection_argument.rs`): the selectable name, then per argument `____`, the argument name, `___`, and the value chunk. Chunks: `v_<name>` for a variable, `l_<n>` for an integer `n` of at least 0, `l_n<n>` for the integer `-n` (`l_n5` for `-5`), `l_true` and `l_false`, `l_null`, `s_<text>` for a string, `o_<entries>_c` with each entry `<key>__<chunk>` joined by `_`, and `a_<chunks>_c` joined by `_`. Every argument name, variable name, object key, and string is written by `alias_text`; the selectable name is written as is, because it is the response key of the same field selected without arguments.

Delta from upstream: upstream writes `l_-5` for `-5`, replaces each character of a string outside `[A-Za-z0-9_]` with `_`, and writes names as they are. `-` is not a GraphQL name character, so upstream's alias of a negative integer argument makes the query invalid; `"a b"` and `"a_b"` alias the same; and a variable named `x_v_y` in a list aliases as the two variables `x` and `y` (`a_v_x_v_y_c`). i2's chunks are injective: after `l_`, a digit starts a non-negative integer, `n` and a digit a negative one, and `n` and `u` is `null`; `alias_text` keeps only letters and digits and escapes every `_`, so a text cannot contain a separator and two texts never encode the same; and after a chunk inside an object, `_c` followed by a letter, or by `__` and a letter, starts an entry whose key starts with `c`, and `_c` followed by anything else closes the object.

The alias lives in the core because the query and the response key in the normalization AST must agree, whatever the protocol. The runtime's store keys are its own: `storeKey` writes `____<name>___<JSON of the value>` (`libs/isograph-runtime/src/storeKey.ts`) and never reads the alias.

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
/// `field_map` with each variable replaced by its value in `context`.
fn substitute(
    field_map: &MergedSelectionMap,
    context: &BTreeMap<VariableName, MergedValue>,
) -> Result<MergedSelectionMap, SelectionMapErrorKind>;
```

The context of a selection `G(a: v)` of a field `G` with variables `$a` and `$b: Int = 3` is `{ a: v, b: 3 }`. A variable that has no default and that the selection does not pass is `MissingFieldArgument` (below): there are no unset variables, so every variable of a client field has a value wherever the field is read (store.md, open decision 11, settled). A passed `null` is a value. `v` is already a `MergedValue` of the parent's map: a parent variable stays a `Variable` naming the parent's variable, and is substituted when the parent is inlined in turn. Upstream's origin is `VariableContext::child_variable_context` (`isograph/crates/isograph_schema/src/variable_context.rs`).

Delta from upstream: upstream writes `null` for an unpassed variable without a default (`VariableContext::child_variable_context`); i2 rejects the selection. Substitution re-keys `Selectable` entries at every depth, inside composite targets and conditionals; two keys that become equal merge, and the result is checked for alias collisions. `Err` is `UndefinedVariable` or `Merge`, as in the stash.

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

1. `source_field_declaration(db, client, entity, selectable)`: the declaration's parse.
2. `inline_cycle(db, client, key)`: a cycle reachable from this field is `InlineCycle`, located at the declaration's name, returned before any recursive call. The declaration is read first because the location is its name; a field whose declaration fails has no inlined fields, so the order changes no result.
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
  - `Direct` on a `Composite(shape)` target: no selection set is `MissingSelectionSet`; otherwise one entry `Selectable { key, target: Composite(MergedCompositeTarget { entity: target, shape, selections }) }`, with `selections` the nested selection set's `merge_selection_set` on the target.
  - `Refinement` on a `Composite(shape)` target: arguments on the selection are `ArgumentsOnRefinement`, no selection set is `MissingSelectionSet`; otherwise one entry `Conditional(MergedConditional { condition: Condition::SelectionEquals(target), identity: shape.identity, selections })`. `shape.concreteness` is not kept: the conditional's records are of `target`, the entity the condition names.
  - `Refinement` on a `Leaf` target is `RefinementToNonComposite(target)`; GraphQL's refinements target only object types (`refinement_to`), so GraphQL does not reach it.
  - Either shape on an `Input` target is `InputTarget(target)`.
- `SelectableKind::ClientField`: `field_selection_map(db, client, parent, selection.name)`, then `substitute` with the context built from the converted arguments and that map's `variables`. An argument the field does not declare is `UnknownFieldArgument`. A variable with no default that the selection does not pass is `MissingFieldArgument`, located at the selection's name; the first such variable, in declaration order, is reported. The selection's own nested selection set is not read.
- `SelectableKind::ClientPointer(_)`: the empty map. Its own selection set and the selection's nested selection set are not read (Client pointers, below).

`key` is `SelectionKey { name: selection.name, arguments }` with the converted arguments. Each shape error is located at the selection's name. The `Server` arm is the private `server_contribution` and the `ClientField` arm the private `client_field_contribution`, each taking the context, the `SelectableKey`, the selection, and the converted arguments; a substitution error is located at the selection's name.

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
/// inlines directly: client field selections at any depth under protocol selections. A
/// selection whose lookup fails is not listed, and neither is a client pointer.
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
    #[error("{0}")]
    MissingFieldArgument(MissingFieldArgument),
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

```rust
// from crates/isograph_compiler/src/validation.rs
/// A selection of `field` that passes no value for `variable`, which has no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{field}` declares `${variable}` with no default, and this selection does not pass it.")]
pub struct MissingFieldArgument {
    pub field: SelectableKey,
    pub variable: VariableName,
}
```

`MissingFieldArgument` is also `ValidationError::MissingFieldArgument`, so the editor shows it: `validate_selection` reports one per variable of a selected `field` declaration (client field or client pointer) that has no default and that the selection does not pass, located at the selection's name, reading the declaration through `field_declaration` (`missing_field_arguments`, private).

`SelectableKey` implements `Display` as `Entity.selectable` (`derive_more::Display`), and `InlineCycle::display` joins the keys with ` -> `. `location` is the offending node's span in its file: the selection name, the argument, the variable, or for `InlineCycle` the name of the field that starts the walk. A field that inlines a broken field returns the inlined field's error unchanged, so the location is where the mistake is.

## The operation

`query_for_entrypoint` builds one operation's completed selection map in the core, builds the normalization AST from it (normalization-ast.md), and hands the same map to the protocol, which prints the text. The text and the AST are produced from one value.

An entrypoint's parent is a root operation type (entrypoints.md), so the entrypoint field's selections are the operation's top-level selections. There is no fetch selection between the operation and the entrypoint's selections.

### The fetch plan

Completion in the core changes the `GraphQLFetchPlan` that the Node removal left (entrypoints.md, Delta from the landed fetch plan). Its `MergedTarget<GraphQLEntityData>` payload existed so the GraphQL operation builder could add `id` and `__typename` under the root type. The core now adds the discriminator and the identity from `discriminator()` and `entity_shape`, and adds neither at the top level (Completion), so no payload carries entity data.

Before:

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
pub struct GraphQLFetchPlan {
    pub operation_type: OperationType,
    /// The root type, whose data decides the top-level `id` and `__typename`.
    pub entity: MergedTarget<GraphQLEntityData>,
}
```

After:

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How the GraphQL protocol fetches an entity that parents an entrypoint: the entity is the
/// root type of this operation type, and its selections are the operation's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchPlan(pub OperationType);
```

The struct has one field once `entity` is gone, so it is a newtype (AGENTS.md). `graphql_fetch_plan` checks the root first and then that the root is an object type (`NotObject`, replacing `NotObjectOrInterface`); entrypoints.md has the body.

Change 10 gives the plan the root entities, which the core reads through `root_entities` (Root entities):

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How the GraphQL protocol fetches an entity that parents an entrypoint: the entity is the
/// root type of `operation_type`, and its selections are the operation's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchPlan {
    pub operation_type: OperationType,
    /// The schema's root operation types, with the entity as `operation`.
    pub roots: RootEntities,
}
```

```rust
// from crates/graphql_network_protocol/src/root_operation_types.rs
impl RootOperationTypes {
    /// `operation` as the operation's root, and every other root operation type the schema
    /// names (`query`, then `mutation` and `subscription` when present) as `others`.
    pub fn root_entities(&self, operation: EntityName) -> RootEntities;
}
```

`graphql_fetch_plan`'s last step (entrypoints.md) is `GraphQLFetchPlan { operation_type, roots: roots.root_entities(entity) }` where it was `GraphQLFetchPlan(operation_type)`. `root_entities`'s body collects `[self.query.wrap_some(), self.mutation, self.subscription].into_iter().flatten().filter(|root| *root != operation)` as `others`; the spec requires the three root types to be distinct types (spec 3.3.1). `GraphQLNetworkProtocol::root_entities(plan)` is `plan.roots.clone()`. The printer reads `plan.operation_type` where it read `plan.0`. The memo already reads `root_operation_types`, so it re-runs exactly when it did.

### Completion

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// `map`, a selection set under a selection of an entity whose record id is found by
/// `identity`, with the discriminator, the identity, and every composite target and
/// conditional below it completed against `roots`.
pub fn complete<TProtocol: NetworkProtocol>(
    map: &MergedSelectionMap,
    identity: &CompletedIdentity,
    roots: &RootEntities,
) -> Result<CompletedSelectionMap, AddedSelectionCollision>;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{}` has the normalization alias of a selection the protocol adds.", .0.name)]
pub struct AddedSelectionCollision(pub SelectionKey);
```

`complete`, in order:

Before (landed change 7): `complete(map: &MergedSelectionMap, identity: IdentitySelection)`; after: the signature above (change 10).

1. Each `Selectable` entry: `Leaf` is `CompletedSelectionTarget::Leaf`. `Composite(target)` is `Composite(CompletedCompositeTarget { entity, identity, selections })` with `entity` `RecordEntity::Known(target.entity)` when `target.shape.concreteness` is `Concrete` and `RecordEntity::Discriminator` when it is `Abstract`, `identity` `roots.identity_of(target.entity, target.shape.identity)`, and `selections` `complete::<TProtocol>(target.selections, &identity, roots)?`.
2. Each `Conditional` entry `c` with `c.condition` `SelectionEquals(entity)`: `CompletedConditional { condition: c.condition, identity, selections: complete::<TProtocol>(c.selections, &identity, roots)? }` with `identity` `roots.identity_of(entity, c.identity)`.
3. A `Leaf` selectable entry with no arguments named `TProtocol::discriminator()` becomes `Discriminator`; when `identity` is `Select(name)`, one named `name` becomes `Identity(name)`. The selection a user wrote and the one the protocol adds are one selection.
4. `Discriminator` is added. `CompletedIdentity::Select(name)` adds `Identity(name)`; `Omit` and `Fixed` add nothing.
5. A remaining `Selectable` key whose normalization alias equals the discriminator's or the identity's name is `AddedSelectionCollision(key)`. Step 3 has absorbed the leaf selection with no arguments; what remains is a composite selectable named like one of them, which GraphQL excludes (`__typename` is not a schema field, and `IdField::Present` requires `id: ID!`).

Step 4 makes the map non-empty and puts the discriminator before every conditional in the same selection set, which the condition reads. The steps run per selection in the private `complete_selection` (steps 1 to 3 and 5 for one entry) and `complete_target` (a selectable's target, which `top_level` also calls); `CompletedSelectionMap::from_selections`, private, keys a list of completed selections by `CompletedSelection::key`, and is how `complete` and `top_level` build the map. Under `Omit` (a type whose `id` is not `ID!`) and under `Fixed` (a root entity, whose record id is known), a selection of `id` stays a `Selectable` entry, in name order. The discriminator is selected under a `Known` entity too: the query selects it in every selection set under a selection (GraphQL's shapes), and the normalizer reads it only under `Discriminator`.

For `field Query.NestedQuery { greeting query { tags } }` on `type Query { greeting: String query: Query! tags: [String] }`, the `query` entry's target is `CompletedCompositeTarget { entity: Known(Query), identity: Fixed(FixedRecordId::root()), selections }`, whose map holds `Discriminator` and `tags` and no `Identity`, even when `Query` has `id: ID!`.

Completion runs once, on the entrypoint's whole map, after all inlining and substitution, so a selection set merged from several selections receives the protocol's additions once. A `field` declaration's map is not completed: it is merged into its selection's parent and completed with it.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl CompletedSelectionMap {
    /// The operation's top-level map: every selection completed against `roots`, and nothing
    /// added at the top level itself.
    pub fn top_level<TProtocol: NetworkProtocol>(
        map: &MergedSelectionMap,
        roots: &RootEntities,
    ) -> Result<Self, TopLevelError>;
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

The operation's top level is not under a selection, so it gets no discriminator and no identity. GraphQL forbids an introspection field as a subscription's root field (spec 5.2.3.1), so `__typename` at the top level of `subscription` would be invalid, and the root record is `roots.operation` at `FixedRecordId::root()`, known without a discriminator or an identity (normalization-ast.md, `NormalizationAst.root`). `top_level` completes each `Selectable` entry as `complete`'s step 1 does. A `Conditional` entry is `Conditional`, because its condition would read a discriminator the top level does not select; a GraphQL root type is an object type (spec 3.3.1) and has no refinements, so GraphQL does not reach it. An empty map is `Empty`: GraphQL rejects an empty selection set, and the top level has no discriminator to fill it. `entrypoint Query.X` on a `field Query.X` with no selection set, or whose selections are only client pointers, is `Empty`.

### The operation's selections and variables

```rust
// from crates/isograph_compiler/src/query.rs
/// What the core hands the protocol to print one entrypoint's query.
pub struct EntrypointOperation<'a, TProtocol: NetworkProtocol> {
    /// The operation name: the entrypoint's selectable name.
    pub name: SelectableName,
    pub fetch_plan: &'a TProtocol::FetchPlan,
    /// The entrypoint field's variables that the selections use, in declaration order.
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

`common_lang_types::QueryText` gains `Clone` and `Hash` derives, because `QueryArtifact` is a memo value and derives `Clone`.

`query_text` is infallible: the core finds every error of an operation (`TopLevelError`, and `SelectionSetOnLeaf`, `MissingSelectionSet`, `InputTarget` in the walk, located at the selection). `NetworkProtocol` has no `QueryError` associated type, because it would be `Infallible`. The test protocol's `query_text` returns an empty `QueryText` and `file_name: None`; its `fetch_plan` never returns `Ok`, so it is not reached.

```rust
// from crates/isograph_compiler/src/query.rs
/// The entrypoint field's variables that its selections use, in declaration order.
fn operation_variables(field: &FieldSelectionMap) -> Vec<FieldVariable>;
```

```rust
// from crates/isograph_compiler/src/query.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointQueryError<TNotFetchable: std::fmt::Display> {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("{0}")]
    NotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    SelectionMap(SelectionMapError),
    #[error("{0}")]
    TopLevel(TopLevelError),
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
    let fetch_plan = entrypoint_fetch_plan(
        db,
        client,
        entity_value.reference(),
        selectable_value.reference(),
    )
    .map_err(EntrypointQueryError::NotFetchable)?;
    let field = field_selection_map(db, client, entity, selectable)
        .as_ref()
        .map_err(|error| EntrypointQueryError::SelectionMap(error.clone()))?;
    let roots = TProfile::NetworkProtocol::root_entities(fetch_plan.reference());
    let variables = operation_variables(field);
    let selections = CompletedSelectionMap::top_level::<TProfile::NetworkProtocol>(
        field.selections.reference(),
        roots.reference(),
    )
    .map_err(EntrypointQueryError::TopLevel)?;
    let normalization_ast = normalization_ast(
        selections.reference(),
        TProfile::NetworkProtocol::discriminator(),
        roots.operation,
    )
    .map_err(EntrypointQueryError::IntegerOutOfRange)?;
    let ProtocolQueryText { text, file_name } =
        TProfile::NetworkProtocol::query_text(&EntrypointOperation {
            name: selectable,
            fetch_plan: fetch_plan.reference(),
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

`normalization_ast`, `NormalizationAst`, and `IntegerOutOfRange` are normalization-ast.md's, and land with its change 2: until then `QueryArtifact` is `{ text, file_name }` and `query_for_entrypoint` has no `normalization_ast` step. The AST carries the discriminator's response key and the root record, so the runtime names no GraphQL selectable or type. Only used variables are declared, because GraphQL rejects an operation that declares a variable it does not use (spec 5.8.4); upstream's `get_used_variable_definitions` does the same. A variable is used when a `MergedValue::Variable` of that name is in `selections`, at any depth or inside an object. The operation name is the selectable name, upstream's (`isograph/crates/artifact_content/src/entrypoint_artifact.rs`, `query_name = entrypoint.name`); open decision 3.

### The entrypoint's parent shape

A parent whose shape is not `Composite` has nothing to select and is reported where validation reports the other reasons.

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
/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
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
    match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(_) => {}
        EntityShape::Leaf | EntityShape::Input => {
            return EntrypointNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    }
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntrypointNotFetchableError::Protocol)
}
```

The root's identity is not read: the top level has no identity selection. `EntrypointNotFetchableError` gains:

```rust
// from crates/isograph_compiler/src/validation.rs
    #[error("`{0}` has no selectables, so it cannot parent an entrypoint.")]
    NotComposite(EntityName),
```

`entrypoint_declaration_error` locates `NotComposite` at `parent_type`, with `SourceEntity` and `Protocol`. The origin `match` binds the data the `if let` used to discard, so the shape is read from the same value the origin check ruled on. For GraphQL, `entrypoint String.X` is now `NotComposite` where it was `Protocol(NotRootOperationType)`.

### GraphQL printing

`GraphQLNetworkProtocol::query_text` is a pure function of the protocol's options and the operation: every fact it prints is in the completed map and the fetch plan, and the layout is `options.query_text_format`. It reads no database; the core reads the options (Config: query text format, below) and passes them. It prints the map directly; there is no GraphQL document type between the map and the text, and the printer is the only function that writes GraphQL text.

Upstream has no structure between the merged map and the string. `generate_query_text` (`isograph/crates/graphql_network_protocol/src/query_text.rs:13`) pushes the header onto one `String` (`:23`), and `write_selections_for_query_text` (`:70`) recurses over the `MergedSelectionMap` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:33`, a `BTreeMap<NormalizationKey, MergedServerSelection>`) with `push_str` and `format!`, writing aliases at `:89-92` and `:99-102`. Arguments are strings from `get_serialized_arguments_for_query_text` (`:135`), values from `serialize_non_constant_value_for_graphql` (`:158`), and variable definitions from `write_variables_to_string` (`:38`). Like upstream, i2's printer walks the core's map; unlike upstream, the map is protocol-neutral and GraphQL's names arrive in it as values.

```rust
// from crates/graphql_network_protocol/src/query_text.rs
pub const QUERY_TEXT_FILE_NAME: &str = "query_text.graphql";

pub(crate) fn query_text(
    format: QueryTextFormat,
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> ProtocolQueryText {
    ProtocolQueryText {
        text: print_operation(format, operation),
        file_name: QUERY_TEXT_FILE_NAME.intern().to::<ArtifactFileName>().wrap_some(),
    }
}
```

The printer builds the operation as `PrintItem`s, then lays them out in the format. Each `print_*` function returns items; `lay_out` is the only function that reads the format, so the two formats cannot print different tokens.

```rust
// from crates/graphql_network_protocol/src/print_operation.rs
/// One piece of an operation's text before `lay_out` writes it in a `QueryTextFormat`.
enum PrintItem {
    /// Text both formats write as is: one or more GraphQL tokens with no whitespace or comma
    /// between them (`pets`, `$id`, `[ID!]!`, `"a b"`, `{`).
    Text(String),
    /// A space `Expanded` writes and `Collapsed` does not.
    Space,
    /// A comma between two arguments, object entries, list items, or variable definitions.
    /// GraphQL ignores it (spec 2.1.8); `Expanded` writes it and `Collapsed` does not.
    Comma,
    /// The start of a line at this depth. `Expanded` writes a newline and two spaces per depth;
    /// `Collapsed` writes nothing.
    LineBreak(usize),
}

/// The text of `operation` in `format`.
pub(crate) fn print_operation(
    format: QueryTextFormat,
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> QueryText {
    QueryText(lay_out(format, operation_items(operation).as_slice()))
}

/// The header, then `print_selection_block(selections, 0)`.
fn operation_items(operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>) -> Vec<PrintItem>;

/// `items` as text. `Expanded` writes every item as its doc comment says. `Collapsed` writes
/// only the `Text` items, each preceded by `collapsed_separator` of the previous `Text`'s
/// last character and its first.
fn lay_out(format: QueryTextFormat, items: &[PrintItem]) -> String;

/// `" "` when `previous` and `next` are both name characters (`_`, `0`-`9`, `A`-`Z`, `a`-`z`;
/// spec 2.1.9), because the two tokens would otherwise lex as one; otherwise `""`.
fn collapsed_separator(previous: char, next: char) -> &'static str;

/// `PrintItem::Text(text.to_owned())`.
fn text(text: &str) -> PrintItem;

/// Every selection of `selections` at `depth`, without braces.
fn print_selection_set(selections: &CompletedSelectionMap, depth: usize) -> Vec<PrintItem>;

/// ` {`, `selections` at `depth + 1`, and `}` on its own line at `depth`. The operation's
/// selections are the block at depth 0.
fn print_selection_block(selections: &CompletedSelectionMap, depth: usize) -> Vec<PrintItem>;

/// `LineBreak(depth)`, then the selection.
fn print_selection(selection: &CompletedSelection, depth: usize) -> Vec<PrintItem>;

/// `<alias>: <name>(<arguments>)`, or `<name>` when the key has no arguments.
fn print_selection_head(key: &SelectionKey) -> Vec<PrintItem>;

/// `(name: value, ...)`, or no items when there are none.
fn print_arguments(arguments: &MergedArguments) -> Vec<PrintItem>;

/// `name: value`, an argument or an object entry.
fn print_entry(name: String, value: &MergedValue) -> Vec<PrintItem>;

fn print_value(value: &MergedValue) -> Vec<PrintItem>;

/// `items` with `Comma` and `Space` between consecutive groups.
fn comma_separated(groups: impl Iterator<Item = Vec<PrintItem>>) -> Vec<PrintItem>;

fn print_variable_definition(variable: &FieldVariable) -> Vec<PrintItem>;

/// Unchanged: one `Text`, also read by `argument_errors.rs`.
pub(crate) fn print_variable_type(variable_type: &VariableType) -> String;
```

`print_operation.rs` reads `isograph_parser::Boolean` to print a boolean value, so `graphql_network_protocol` gains the `isograph_parser` dependency. `print_selection` prints the discriminator through `protocol_data.rs`'s `TYPENAME`. `QueryTextFormat` is `isograph_config`'s, so `graphql_network_protocol` gains the `isograph_config` dependency.

`print_selection_set` and `print_selection` return a `Vec` because each nested selection set's items are spliced between its opening and closing items, and a recursive `impl Iterator` is not a nameable type.

The header is `<operation_type> <name>(<variable definitions>) {`, with the operation type the plan's `OperationType` and the parentheses omitted when there are none, then `print_selection_set(selections, 1)`, then `}` after `LineBreak(0)`. A variable definition prints `$name: Type`, then ` = <default>` when it has one. `VariableType` prints `Nullable(t)` as `t`'s text without `!`, `NonNull(t)` with `!`, `List` in brackets. Every space in these descriptions is a `Space` item and every `, ` a `Comma` then a `Space`.

`Expanded` writes exactly today's text: one selection per line, two spaces of indentation per depth, one newline between lines and none at the end. `Collapsed` writes no newline, no comma, and no space except one between two `Text` items whose boundary characters are both name characters. The spaces it keeps are, exhaustively: between the operation type and the name (`query HomeRoute`), between two selections that are names (`__typename id`), between `on` and the entity (`...on BlogItem`), between a value that ends in a name character and the next argument's or object entry's name (`id:$id tagline:$tagline`, `limit:5 skip:0`, `b:true i:-5`), and between a name and a following name in a list (`[A B]`, which v1 does not print because enum values are not `MergedValue`s). It writes no space around `:`, `=`, `(`, `)`, `{`, `}`, `[`, `]`, `...`, `!`, `$`, or a string value, and none before `-` (`i:-5`, `[1-5]`: spec 2.9.1's lookahead after an `IntValue` excludes only a digit, `.`, and a name start). A string value is one `Text` and keeps its own characters, so `"a  b"` keeps its two spaces. Both formats lex to the same token sequence.

`print_selection`:

- `Discriminator` prints `__typename`; `Identity(name)` prints `<name>`.
- `Selectable(selection)` prints `print_selection_head(selection.key)`; with a `Composite(target)` target, then ` {`, `target.selections` at `depth + 1`, and `}` on its own line. `target.entity` and `target.identity` print nothing; the identity's selection is the map's `Identity` entry. `print_selection_head` is `<alias>: <name>(<arguments>)` with the alias `selection.key.normalization_alias()` when the key has arguments, and `<name>` otherwise.
- `Conditional(conditional)` with `SelectionEquals(entity)` prints `... on <entity> {`, its selections at `depth + 1`, and `}`.

Values print as GraphQL: `$name`, integers in decimal, `true`, `false`, `null`, `{ key: value, key: value }` (an object with no entries prints `{}`), and strings in double quotes with `"`, `\`, and the control characters U+0000 to U+001F escaped (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, and `\u00XX` for the rest; spec 2.9.4). Upstream writes the string's text between quotes without escaping (`query_text.rs:164`, whose comment notes the bug).

`query_text` returns `file_name: Some(QUERY_TEXT_FILE_NAME)`. Upstream writes no raw query file; its query text lives only in `query_text.ts` (`export default 'query HomeRoute {\...'`), so the name is new. Upstream separates selections with `,` and builds a TypeScript string with `\` line continuations; here the protocol returns plain text, and the host language encodes it as a string literal.

Upstream's origin for the completion rules is `select_typename_and_id_fields_in_merged_selection` (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1414`). Deltas: the names are the protocol's `discriminator()` and `entity_shape`, applied by the core's `complete`, rather than names hardcoded in the core; the discriminator is selected in every selection set under a selection, and none at the operation's top level; upstream's `node` wrapping (`selection_map_wrapped`, `:1477`, and `GraphQLNetworkProtocol::wrap_merged_selection_map`, `isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:305`) has no counterpart, because v1 has no Node-fetched entrypoints; the text comes from one printer over the completed map.

### Config: query text format

`graphql.query_text_format` decides how the GraphQL protocol lays out each entrypoint's query text. It is GraphQL protocol output, so it sits under the `graphql` key. This change creates `GraphQLConfig` with this one field; `identity_field_selection` (next section) joins it when a non-default variant is implemented.

```json
{
  "source_files": ["src/**"],
  "schema": ["schema/**"],
  "graphql": {
    "query_text_format": "expanded"
  }
}
```

Before:

```rust
// from crates/isograph_config/src/compilation_options.rs
/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub schema: Vec<String>,
    /// The directory, relative to the config file's directory, whose `__isograph` folder holds
    /// the generated artifacts. Absent: the compiler writes no artifacts, and the transforms
    /// look in the config file's directory.
    #[serde(default)]
    pub output_directory: Option<PathBuf>,
}
```

After:

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
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct GraphQLConfig {
    /// How each entrypoint's query text is laid out.
    pub query_text_format: QueryTextFormat,
}

/// How an entrypoint's query text is laid out. Both formats lex to the same tokens.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryTextFormat {
    /// One selection per line, two spaces of indentation per depth.
    Expanded,
    /// One line, with a space only between two tokens that would otherwise lex as one.
    #[default]
    Collapsed,
}
```

The JSON values are `"expanded"` and `"collapsed"`; a missing `graphql` key and a `graphql` object without `query_text_format` are both `Collapsed`. `"compact"`, or any other string, is a config load error naming the two variants, and an unknown key inside `graphql` is a config load error naming the key (`deny_unknown_fields`).

The default is `Collapsed` because the text is the `query` of every request, and `Collapsed` is the smaller request. Upstream has no such option. It has `Format { Pretty, Compact }` (`isograph/crates/isograph_schema/src/network_protocol.rs:69`), chosen per call site: `query_text.ts` is printed `Pretty` (`isograph/crates/artifact_content/src/entrypoint_artifact.rs:153`, `imperatively_loaded_fields.rs:100`), with each line ending in a `\` line continuation, so the string it exports has no newline but keeps the indentation and a `,` after every selection; `Compact` (`operation_text.rs:35`) is used only for persisted documents and prints `query HomeRoute { pets { id, age, }, }`. i2's `Collapsed` drops the commas and every space a lexer does not need.

The pet demo sets `expanded`, so its committed artifacts stay one selection per line and their diffs name the selection that changed.

`query_text.graphql` holds the configured format, the same text as `query_text.ts`, because a tool that registers or hashes operations from the `.graphql` file (a persisted-document allowlist) must see the bytes the client sends. A project that wants readable `.graphql` files sets `expanded`.

#### How the options reach the protocol

The project config is read once, when the daemon starts (`discover::load_config`); `watch.rs` ignores edits of the config file (`ChangedFileKind::Config => {}`), so a config edit takes effect at the next daemon start. Inside the process the protocol's options are a pico singleton source, as pico.md specifies for the config, so every memo that reads them depends on them and setting a different value re-runs those memos.

The core does not name `graphql`. `NetworkProtocol` gains the protocol's options as an associated type and picks them out of the project config:

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // `EntityData`, `SelectableData`, `FetchPlan`, `NotFetchable`, `LookupError`,
    // `ArgumentError`, `SchemaFileError` unchanged
    /// The protocol's options, which it reads from the project config.
    type Options: Clone + Debug + Default + PartialEq + Eq + 'static;

    // every method but `query_text` unchanged

    /// The protocol's options in `config`.
    fn options(config: &IsographProjectConfig) -> Self::Options;

    /// The text of `operation`'s query under `options`, and the protocol's name for a file
    /// holding exactly that text.
    fn query_text(
        options: &Self::Options,
        operation: &EntrypointOperation<'_, Self>,
    ) -> ProtocolQueryText;
}
```

Before, `query_text` took only `operation`. `isograph_compiler` gains the `isograph_config` dependency for `IsographProjectConfig`; `isograph_config` depends only on `serde`.

```rust
// from crates/isograph_compiler/src/database.rs
/// The network protocol's options, a singleton source.
#[derive(Debug, PartialEq, Eq)]
pub struct ProtocolOptions<TOptions>(pub TOptions);

impl<TOptions: 'static> Singleton for ProtocolOptions<TOptions> {
    fn get_singleton_key() -> Key {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<Self>().hash(&mut hasher);
        hasher.finish().to()
    }
}

impl<TOptions: 'static> Source for ProtocolOptions<TOptions> {
    fn get_key(&self) -> Key {
        Self::get_singleton_key()
    }
}

pub type ProfileProtocolOptions<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::Options;

/// The protocol's options as set, or its default options when none are set.
pub fn protocol_options<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
) -> ProfileProtocolOptions<TProfile> {
    db.get_singleton::<ProtocolOptions<ProfileProtocolOptions<TProfile>>>()
        .map(|options| options.0.clone())
        .unwrap_or_default()
}

impl<TProfile: CompilationProfile> IsographDb<TProfile> {
    // `set_editor_buffer`, `remove_editor_buffer`, and the rest unchanged
    pub fn set_protocol_options(&mut self, options: ProfileProtocolOptions<TProfile>) {
        self.set(ProtocolOptions(options));
    }
}
```

The two impls are what `#[derive(Singleton)]` (`crates/pico_macros/src/singleton_macro.rs`) writes, with the type parameter the derive does not accept; `CurrentWorkingDirectory` (`crates/common_lang_types/src/string_key_types.rs`) writes the same impls by hand. `TypeId::of::<Self>()` keys each options type separately. Unset options read as the default: the daemon sets them before the first event, a database a test builds with `IsographDb::default()` has none, and the default is what a config without the protocol's section deserializes to. `get_singleton` records a dependency whether or not the singleton is set (`Storage::try_get`), so a first `set` re-runs a memo that read the default.

`query_for_entrypoint` reads the options and passes them:

```rust
// from crates/isograph_compiler/src/query.rs
    let ProtocolQueryText { text, file_name } = TProfile::NetworkProtocol::query_text(
        protocol_options(db).reference(),
        &EntrypointOperation {
            name: selectable,
            fetch_plan: fetch_plan.reference(),
            variables: variables.as_slice(),
            selections: selections.reference(),
        },
    );
```

The daemon sets them after interning the config directory:

```rust
// from crates/isograph_cli/src/daemon.rs
    let mut state = IsographState::<TProfile>::new(globs.clone(), artifact_output.clone());
    intern_config_directory(&mut state.db, config_path.reference());
    state
        .db
        .set_protocol_options(TProfile::NetworkProtocol::options(config.reference()));
```

The GraphQL protocol:

```rust
// from crates/graphql_network_protocol/src/lib.rs
impl NetworkProtocol for GraphQLNetworkProtocol {
    // the other associated types unchanged
    type Options = GraphQLConfig;

    fn options(config: &IsographProjectConfig) -> GraphQLConfig {
        config.graphql
    }

    fn query_text(
        options: &GraphQLConfig,
        operation: &EntrypointOperation<'_, Self>,
    ) -> ProtocolQueryText {
        query_text::query_text(options.query_text_format, operation)
    }
}
```

The test protocols (`crates/isograph_compiler/src/test_profile.rs`, `crates/isograph_cli/src/test_profile.rs`, `crates/isograph_lsp/src/test_profile.rs`, and the one in `crates/isograph_extract_typescript/src/lib.rs`'s tests) have no options:

```rust
// from crates/isograph_compiler/src/test_profile.rs
    type Options = ();

    fn options(_config: &IsographProjectConfig) {}

    fn query_text(_options: &(), _operation: &crate::EntrypointOperation<'_, Self>) -> crate::ProtocolQueryText {
        // body unchanged
    }
```

Each of those crates gains the `isograph_config` dependency where it lacks it.

### Config: which types get an identity field

This section documents the option that decides GraphQL's `IdField`, which `graphql_entity_shape` turns into `IdentitySelection`. v1 implements only the default, `auto` (a type with a field `id: ID!` is `IdField::Present`, entrypoints.md). v1 does not add the field to `GraphQLConfig` (Config: query text format, above); the other two variants are documented and not implemented.

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
/// The `graphql` section of the config.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct GraphQLConfig {
    // `query_text_format` unchanged
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

The option reaches `graphql_entity` through `protocol_options(db)` (How the options reach the protocol, above), which records the memo's dependency on the options.

## Where the query text goes

The protocol produces the text and the file name; the host language writes the files. `project_artifacts` (`crates/isograph_compiler/src/artifact_types.rs`) builds the `ProjectArtifacts` the host language prints, and it pairs each entrypoint with its query there.

Before:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct ProjectArtifacts<TLiteralContext> {
    /// Sorted by field.
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    /// The `entity` literals, sorted.
    pub entities: Vec<EntityName>,
    /// The `entrypoint` literals, sorted.
    pub entrypoints: Vec<EntityNameAndSelectableName>,
    /// One per entity some pointer points to, sorted by entity.
    pub link_types: Vec<LinkType>,
}

pub fn project_artifacts<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    sources: &BTreeSet<ArtifactSource>,
) -> Result<ProjectArtifacts<LiteralContextOf<TProfile>>, GenerationError>
```

After:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct ProjectArtifacts<TLiteralContext> {
    // `fields`, `entities`, `link_types` unchanged
    /// The `entrypoint` literals with their queries, sorted by entrypoint.
    pub entrypoints: Vec<EntrypointArtifacts>,
}

/// An `entrypoint` literal and its query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointArtifacts {
    pub entrypoint: EntityNameAndSelectableName,
    pub query: QueryArtifact,
}

/// Why the artifacts of a project could not be built.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum ProjectArtifactsError<TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    Generation(GenerationError),
    #[error("{0}")]
    #[from(skip)]
    EntrypointQuery(EntrypointQueryFailed<TNotFetchable>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "The query of `entrypoint {}.{}` could not be generated: {error}",
    .entrypoint.parent_entity_name,
    .entrypoint.selectable_name
)]
pub struct EntrypointQueryFailed<TNotFetchable: std::fmt::Display> {
    pub entrypoint: EntityNameAndSelectableName,
    pub error: EntrypointQueryError<TNotFetchable>,
}

pub type ProfileProjectArtifactsError<TProfile> = ProjectArtifactsError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

pub fn project_artifacts<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    sources: &BTreeSet<ArtifactSource>,
) -> Result<ProjectArtifacts<LiteralContextOf<TProfile>>, ProfileProjectArtifactsError<TProfile>>
```

`project_artifacts` builds `fields` and `link_types` as before, then calls `query_for_entrypoint(db, client, entrypoint.parent_entity_name, entrypoint.selectable_name)` for each entrypoint in order. `Err` is the first entrypoint, in source order, whose query failed, as `EntrypointQueryFailed`. The error type follows `ProfileValidationError` and `ProfileCompilerError`: generic over the protocol's `NotFetchable`, named through `TProfile`. `field_types`' `GenerationError` stays non-generic, and `derive_more::From` converts it at every `?`. `compile.rs`'s `generate_artifacts` is unchanged: it logs `ProjectArtifactsError` with `tracing::warn!(%error, "artifacts not written")` and returns no effects, so a failing query writes no artifacts (open decision 4). `HostLanguage::generate_artifacts(project, options)` is unchanged.

`typescript_artifacts::generate_artifacts`, per `EntrypointArtifacts { entrypoint, query }`, writes into `<Parent>/<name>/`:

- `entrypoint.ts`: the entrypoint value gains `queryText: ArtifactValue::Import(ArtifactImport(<Parent>/<name>/query_text.ts))`, after `name`. Before: `{ kind, name, normalizationAst, readerArtifact }`; after: `{ kind, name, queryText, normalizationAst, readerArtifact }`.
- `query_text.ts`: `query_text_artifact(entrypoint, text)`, an `ArtifactModule` whose default export is `ArtifactValue::String(query.text)`.
- When `query.file_name` is `Some(file_name)`: `raw_query_text_file(entrypoint, file_name, text)`, the pair `(<Parent>/<name>/<file_name>, text + "\n")`, with no printer.
- `normalization_ast.ts`: the stub, unchanged until normalization-ast.md change 3.

```rust
// from crates/typescript_artifacts/src/entrypoint.rs
pub(crate) const QUERY_TEXT: &str = "query_text.ts";

/// The module whose default export is the text of `entrypoint`'s query.
pub(crate) fn query_text_artifact(
    entrypoint: EntityNameAndSelectableName,
    text: &QueryText,
) -> ArtifactModule;

/// The protocol's file holding exactly the text of `entrypoint`'s query, and one newline.
pub(crate) fn raw_query_text_file(
    entrypoint: EntityNameAndSelectableName,
    file_name: ArtifactFileName,
    text: &QueryText,
) -> (ArtifactPath, String);
```

`iso.ts` is unchanged: it imports `entrypoint.ts`, whose default export's type now includes `queryText: string`. The TypeScript host never inspects the text or the file name beyond writing them; another protocol's text (SQL, a persisted id) goes through the same path.

## Memos and invalidation

```text
query_for_entrypoint(client, E, f)
  -> entity_named(client, E), selectable_named(client, E, f)
  -> entrypoint_fetch_plan -> NetworkProtocol::fetch_plan -> graphql_fetch_plan(client, E)
       -> graphql_entity(client, E), root_operation_types(client)
  -> field_selection_map(client, E, f)
       -> inlined_fields(client, ..) (via inline_cycle)
       -> parsed_iso_literals_in_file(client, path of E.f)
       -> selectable_named / entity_named of every selection and target
       -> field_selection_map(client, P, g) of every inlined field g
  -> protocol_options (the ProtocolOptions singleton)
  -> complete, normalization_ast, NetworkProtocol::query_text (plain, no db)
```

Each memo is keyed by the narrowest identity that determines it: `field_selection_map` by the field, `query_for_entrypoint` by the entrypoint, `graphql_fetch_plan` by the entity, `root_operation_types` by the client only. The client is a key because every one reads file text through `file_contents`.

- Editing `field Pet.PetStatsCard`'s selection set re-runs its literal's parse, `field_selection_map(Pet, PetStatsCard)`, and through it the maps of fields that inline it and the queries of entrypoints that reach it (`PetDetailRoute`). `HomeRoute`'s `field_selection_map` reads only `HomeRoute` and `PetSummaryCard` and the fields they inline, so it is not re-run.
- An edit that does not change a field's map backdates there: renaming a reader alias, reordering selections, editing the resolver function body, or adding a selection already present.
- A `field_selection_map` does not depend on how its field was selected. Two selections of `PetCheckinsCard` with different arguments share one map and substitute separately.
- Editing an unrelated schema type re-runs the protocol's per-name lookups (integration.md), which backdate; no query re-runs.
- Setting different protocol options (`set_protocol_options` with another `query_text_format`) re-runs every `query_for_entrypoint`, which returns new text; no `field_selection_map` re-runs. Setting equal options does not advance the epoch, and nothing re-runs.
- Every derived node is retained for the process lifetime (pico.md, Garbage collection). There is one `field_selection_map` per field reached and one `query_for_entrypoint` per entrypoint, per client; the compile reads as `compiler_client`, so the LSP clients add nodes only if an LSP feature reads queries.

### Error isolation

A query fails only on errors inside what it reads. A broken field fails every `field_selection_map` that inlines it, transitively, and every query that reaches one of those; no other memo reads it. A broken schema definition fails `root_operation_types`, which every fetch plan reads, so every entrypoint fails: the root types decide every operation's kind. A syntax error in `type Query` fails `Query`'s fetch plan and every `Query.*` field lookup, so entrypoints on `Query` fail and `Mutation.SetTagline` does not.

## Tests

Facts about generated text, not snapshots. A helper in the test module returns the lines of a query text, and assertions name lines and their relative order (`assert!(lines contains "    tagline")`, "the `id` line precedes the `age` line", "no line contains `friendName`").

`crates/graphql_network_protocol/src/print_operation.rs`, building `EntrypointOperation` values by hand from `complete` of hand-built `MergedSelectionMap`s (no database):

- `Root(Query)` with no variables and one `Selectable` entry `pets` targeting an entity with identity `Select(id)` whose map has `tagline` (leaf): text is `query HomeRoute {`, `  pets {`, `    __typename`, `    id`, `    tagline`, `  }`, `}`.
- `Root(Mutation)` header starts `mutation`; `Root(Subscription)` starts `subscription`; no top-level line is `__typename`.
- A variable `$limit: Int = 3` used in the map prints `query X($limit: Int = 3) {`; `[ID!]!` prints `[ID!]!`; a nullable list of nullable prints `[ID]`.
- An entry whose identity is `Omit` prints `__typename` first in its block and no `id`; one with `Select(id)` prints `__typename` then `id`; a `Conditional` entry `SelectionEquals(BlogItem)` prints `... on BlogItem {` after every `Selectable` entry of its block, with `__typename` first inside it.
- A composite entry whose map is empty and whose identity is `Omit` prints one `__typename` line in its block.
- Every selection line with arguments has the form `<alias>: <name>(<arguments>)`, where the alias is the key's `normalization_alias`, and every selection line without arguments is the bare name.
- A string argument `a"b\c` followed by a newline prints `"a\"b\\c\n"`.
- An object argument `{ id: $id, tagline: $tagline }` prints in key order with `{ ` and ` }`.
- No line ends in whitespace; the text does not end in a newline.
- The tests above print `Expanded`. For every operation the printer tests build (the `cases()` fixtures, which cover a union refinement, an abstract target, nested composites, a mutation, and every value kind; the variable definitions fixture with defaults and list types; and a list and an empty object argument), `Collapsed` text contains no `\n` and no two consecutive spaces, and graphql-parser (`graphql-parser = "0.4.1"`, a dev-dependency) parses both texts and prints the two parses identically. graphql-parser's `Document` carries source positions in its `PartialEq`; its printer writes none, so equal printed parses is the assertion that both texts are one operation. The every-value-kind fixture passes `i: -5`, whose alias chunk is `i___l_n5`.
- `Collapsed` of `pets { __typename id tagline }` under `query X` is `query X{pets{__typename id tagline}}`; of `f(b: true, i: -5, n: null, o: { s: "a b", v: $x }, s: "\"q\"", v: $x)` the arguments are `(b:true i:-5 n:null o:{s:"a b"v:$x}s:"\"q\""v:$x)`; the variable definitions fixture's header is `query X($limit:Int=3$ids:[ID!]!$maybe:[ID]$nested:[[Int!]]!$flag:Boolean!=false){a}`; the union fixture prints `__typename cursor...on BlogItem{__typename id title}`.

`crates/isograph_compiler/src/query.rs` and `merged_selection.rs`, the core's operation rules (GraphQL profile in the next group where a case needs a schema):

- `complete` of a map holding a leaf `id` with no arguments under `Select(id)` has one `Identity(id)` entry and no `Selectable` entry `id`; under `Omit`, the `Selectable` entry `id` remains, in name order.
- `complete` of a map holding a composite selection named `id` with no arguments under `Select(id)` is `AddedSelectionCollision`.
- `top_level` of an empty map is `Empty`; of a map with a `Conditional` entry is `Conditional`; of a map with `pets` has no `Discriminator` entry at its own level and one under `pets`.
- `operation_variables` declares only the used variables, in declaration order, each once.
- With `roots` `{ operation: Query, others: {Mutation} }`: a composite entry targeting `Query` whose shape is `Concrete` with `Select(id)` completes to `CompletedCompositeTarget { entity: Known(Query), identity: Fixed(FixedRecordId::root()), .. }` whose map has a `Discriminator` entry and no `Identity` entry; a leaf `id` selected under it stays a `Selectable` entry. One targeting `Mutation` is `Fixed(FixedRecordId::root())` too. One targeting `Pet` (`Concrete`, `Select(id)`) is `Known(Pet)` with `Select(id)` and an `Identity(id)` entry. One targeting a union (`Abstract`, `Omit`) is `Discriminator` with `Omit`, and its conditional `SelectionEquals(BlogItem)` with `Select(id)` has `identity == Select(id)`.
- `RootEntities::identity_of` of `operation`, of each of `others`, and of another entity under `Select(id)` and `Omit`.

`crates/isograph_compiler/src/merged_selection.rs` (test protocol with `()` data where a case needs no protocol fields; otherwise the GraphQL profile in the next group):

- `MergedArguments::new` sorts `b, a` to `a, b`, and returns `Err(a)` for `a, a`.
- `normalization_alias` of `pet(id: $id)` is `pet____id___v_id`; of `checkins(limit: 5, skip: 0)` is `checkins____limit___l_5____skip___l_0`; of `set_pet_tagline(input: { id: $id, tagline: $tagline })` is `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c`; of a key `firstName` with no arguments is `firstName`.
- Merging `f(s: "a")` with a no-argument selection of a field named `f____s___s_a` is `AliasCollision`, at the top level and inside a composite, and so is substituting `$a = "a"` into `f(s: $a)` beside `f____s___s_a`.
- `normalization_alias` of `checkins(i: -5)` is `checkins____i___l_n5`, of `checkins(i: 5)` is `checkins____i___l_5`, and of `i64::MIN` is `checkins____i___l_n9223372036854775808`; of `f(s: "a b-c.Ü_1")` is `f____s___s_a_32_b_45_c_46__220__95_1`.
- Over argument sets built from variables, integers (negative, zero, `i64::MIN`, `i64::MAX`), booleans, null, and strings with spaces, punctuation, `_`, digits, strings shaped like an escape (`a_32_b`), and non-ASCII text, alone, in lists, in objects with keys `c` and `a_b`, and nested, under argument names with `_` and `___`: every alias matches `[_A-Za-z][_0-9A-Za-z]*`, and no two distinct sets have the same alias.
- Merging a `Leaf` and a `Composite` entry at one key is `SelectionMismatch`.

`crates/graphql_network_protocol/tests` or the crate's test module (GraphQL profile, TypeScript host, the pet demo's schema and source files inserted before the first lookup, and `query_text_format: Expanded` set, as the pet demo's config sets it):

- Options: `HomeRoute`'s first line is `query HomeRoute {` under `Expanded`. Setting `Collapsed` on that database makes the next `query_for_entrypoint(Query, HomeRoute)` `query HomeRoute{pets{__typename id age firstName lastName nickname picture species tagline}}`; removing the singleton (`remove_singleton::<ProtocolOptions<GraphQLConfig>>`) leaves that text, the default; setting `Expanded` makes the first line `query HomeRoute {` again.

- `HomeRoute`: the first line is `query HomeRoute {`; the `pets {` block contains `id`, `age`, `firstName`, `lastName`, `nickname`, `picture`, `species`, `tagline`, each once; the block's first line is `__typename`; no line contains `PetSummaryCard` or `fullName`; the top level has no `__typename` line.
- `PetDetailRoute`: the first line is `query PetDetailRoute($id: ID!) {`; a line is `  pet____id___v_id: pet(id: $id) {`; exactly one line is `    best_friend_relationship {`, and its block contains `picture_together` and a `best_friend {` block with `id`, `firstName`, `lastName`; a line is `    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {`; no line contains `friendName`, `owner`, `OwnerBadge`, or `displayName` (`owner` and `bestFriend` are client pointers, omitted).
- `SpeciesRoute`: the header is `query SpeciesRoute($species: PetSpecies!) {`; lines include `  petsBySpecies____species___v_species: petsBySpecies(species: $species) {`, `    favorite_phrase`, `  viewer {`, and `  node____id___s_1: node(id: "1") {`, whose block's first two lines are `__typename` and `id`.
- `SetTagline`: the header is `mutation SetTagline($id: ID!, $tagline: String!) {`; the second line is the `set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {` line.
- `entrypoint Pet.PetSummaryCard`, `entrypoint PetStats.Card`, and `entrypoint Node.Card` are `NotFetchable(Protocol(NotRootOperationType))`.
- `field Query.NestedQuery { greeting query { tags } }` on a schema whose `Query` has `query: Query!`: the `query {` block is `__typename` then `tags`, with no `id` line even when `Query` has `id: ID!`. With `schema { query: RootQuery }` and `type RootQuery { me: RootQuery! name: String }`, `field RootQuery.Me { me { name } }` gives the `me` target `Known(RootQuery)` with `Fixed(FixedRecordId::root())`; `graphql_fetch_plan(RootQuery).roots` is `{ operation: RootQuery, others: {} }`.
- Selecting `PetCheckinsCard(limit: 5)` in `PetDetailRoute` adds the line `    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {`. A second selection `PetCheckinsCard` with no arguments adds `checkins____limit___l_3____skip___l_0` (the default); both blocks are present.
- `field Pet.Opt($x: Int) { checkins(limit: $x) { id } }` selected as `Opt` with no argument is `MissingFieldArgument` naming `Pet.Opt` and `$x`, located at `Opt`, and a validation diagnostic on `Opt`; selected as `Opt(x: null)` it prints `checkins____limit___l_null: checkins(limit: null) {`. A variable with a default that the selection does not pass takes the default, through a chain of inlined fields.
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

`crates/isograph_config/src/compilation_options.rs`:

- A config without `graphql` and one with `"graphql": {}` have `query_text_format == Collapsed`; `"expanded"` is `Expanded` and `"collapsed"` is `Collapsed`; `"compact"` is an error naming `expanded` and `collapsed`; `"graphql": {"query_text_formats": "expanded"}` is an error naming `query_text_formats`.

`crates/isograph_cli/src/compile.rs` (handle seam, artifact directory set, GraphQL profile, a schema `type Query { id: ID! name: String }`):

- The compile fire after `field Query.HomeRoute { id }` and `entrypoint Query.HomeRoute` returns a write containing `Query/HomeRoute/query_text.ts`, `Query/HomeRoute/query_text.graphql`, and `Query/HomeRoute/entrypoint.ts`; the `.graphql` artifact's contents are `query HomeRoute{id}\n` (the state sets no options, so the format is the default `Collapsed`); `query_text.ts` contains `query HomeRoute{id}`; `entrypoint.ts` contains `queryText` and an import from `./query_text`.
- Editing the selection set of a field the entrypoint inlines returns writes of that field's `param_type.ts` and the entrypoint's `query_text.graphql` and `query_text.ts`, and nothing else (`entrypoint.ts` imports the text and does not change).
- A failing query returns no effects; fixing it returns the write. `project_artifacts` of an entrypoint whose field has no selection set is `EntrypointQuery` naming the entrypoint, with `TopLevel(Empty)`.

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

Status: landed (`103a23b9`). `RootOperationTypes`, `root_operation_types`; `GraphQLFetchPlan`, `GraphQLRootFetch`, `GraphQLNodeFetch`, `GraphQLFetchArgument`, `FetchRefinement`, `GraphQLNotFetchable`, `NodeFieldError`, `NodeFieldTypeMismatch`, `graphql_fetch_plan`, `node_fetch`; `NetworkProtocol::fetch_plan`; `VariableType` and `NonNullVariableType`; `MergedTarget`. The Node removal (entrypoints.md, Delta from the landed fetch plan) deletes the `node` fetch; change 7 makes `GraphQLFetchPlan` a newtype of `OperationType` and accepts only object types.

### 5. Entrypoint validation (isograph_compiler)

Status: landed (`7e38d5c4`). `EntrypointNotFetchableError`, `ValidationError<TNotFetchable>`, `entrypoint_fetch_plan`, `entrypoint_declaration_error`'s new check. Change 7 adds `NotComposite`.

### 6. Protocol shapes and merged selection maps (isograph_compiler, graphql_network_protocol)

The stash `stash@{0}` ("WIP change 6") is the starting point; its deltas are stated in The merged selection map and Errors.

- `crates/isograph_compiler/src/protocol_shape.rs`: `EntityShape`, `IdentitySelection`, `SelectableShape`.
- `NetworkProtocol::entity_shape`, `::selectable_shape`, `::discriminator`; the GraphQL implementations `graphql_entity_shape`, `identity_selection`, `graphql_selectable_shape`, `TYPENAME`, `ID`; `GraphQLCompositeData.id_field` and `IdField` (entrypoints.md) with `graphql_entity` computing it; the test protocols' implementations (`crates/isograph_compiler`, `crates/isograph_cli`, `crates/isograph_lsp` test profiles).
- `MergedSelectionMap`, `MergeKey`, `SelectionKey`, `MergedSelection`, `MergedSelectableSelection`, `MergedSelectionTarget`, `MergedCompositeTarget`, `MergedConditional`, `Condition`, `MergedSelection::key`, `MergedSelectionMap::{empty, single, merge, iter, used_variables}`, `MergeError`, `AliasCollision`; `MergedArguments` (with `Default`), `MergedArgument`, `MergedValue`, `MergedObject`, `MergedObjectEntry`, `NormalizationAlias`, `SelectionKey::normalization_alias`.
- `FieldSelectionMap`, `FieldVariable`, `VariableValue`, `substitute`; `SourceFieldDeclaration`, `source_field_declaration`; `inlined_fields` (client fields only), `InlineCycle`, `inline_cycle`; `field_selection_map`, `merge_selection_set`, `selection_contribution` with the shape errors and the client pointer omission; `SelectionMapError`, `SelectionMapErrorKind`, `UnknownFieldArgument`.
- `isograph_parser::Boolean` and `SelectableKey` gain the `Ord` and `Hash` derives.
- The `crates/isograph_compiler/src/merged_selection.rs` tests and the walk tests of the GraphQL group above that do not read query text, asserted on `field_selection_map`'s value, in `crates/graphql_network_protocol/src/tests/merged_selection.rs`.

The landed `MergedTarget` stays until change 7. Depends on 3 and 5.

### 7. Completion, operations, and GraphQL printing (isograph_compiler, graphql_network_protocol)

- Core: `CompletedSelectionMap`, `CompletedKey`, `CompletedSelection`, `CompletedSelectableSelection`, `CompletedSelectionTarget`, `CompletedConditional`, `CompletedSelection::key`, `CompletedSelectionMap::{iter, top_level}`, `complete`, `AddedSelectionCollision`, `TopLevelError`.
- Core: `EntrypointNotFetchableError::NotComposite`, its check in `entrypoint_fetch_plan`, and its location in `entrypoint_declaration_error`.
- Core: `EntrypointOperation`, `ProtocolQueryText`, `QueryArtifact { text, file_name }`, `EntrypointQueryError` without `IntegerOutOfRange`, `operation_variables`, `query_for_entrypoint` without the `normalization_ast` step; `NetworkProtocol::query_text`; the test protocols' `query_text`. `MergedTarget` is deleted.
- GraphQL: `GraphQLFetchPlan(OperationType)`; objects only (`NotObject`, after the root check).
- GraphQL: `QUERY_TEXT_FILE_NAME`, `query_text`, `print_operation`, `print_selection_set`, `print_selection`, `print_selection_head`, `print_arguments`, `print_value`, `print_variable_definition`, `print_variable_type`; `GraphQLNetworkProtocol::query_text`.
- Tests: the printer tests (`print_operation.rs`), the core operation-rule tests (`merged_selection.rs` and `query.rs`, test protocol), the profile tests that read query text (`crates/graphql_network_protocol/src/tests/query.rs`), and the fetch plan and validation test updates.

Depends on 4, 5, and 6.

### 8. Query artifacts (isograph_compiler, typescript_artifacts, isograph_cli)

`EntrypointArtifacts`, `ProjectArtifacts.entrypoints` carrying them, `ProjectArtifactsError`, `EntrypointQueryFailed`, `ProfileProjectArtifactsError`, `project_artifacts` calling `query_for_entrypoint`; `QUERY_TEXT`, `query_text_artifact`, `raw_query_text_file`, `queryText` on the entrypoint value, and `generate_artifacts` writing both files. Tests: `crates/typescript_artifacts` (the query text module, the raw file, no raw file without a file name), `crates/isograph_cli/src/compile.rs` (the handle seam below; its fixtures parent entrypoints on a schema `Query`, because a source `entity` cannot parent an entrypoint). Depends on 7.

### 9. Pet demo output

The committed `demos/pet-demo/src/__isograph/` gains the query files for `HomeRoute`, `PetDetailRoute`, `SpeciesRoute`, and `SetTagline`, generated by the daemon. Depends on 8.

### 10. Record entities and root identity (isograph_compiler, graphql_network_protocol)

The delta from landed change 7 that normalization-ast.md change 2 builds on:

- Core: `CompositeShape`, `Concreteness`, and `EntityShape::Composite(CompositeShape)` in `protocol_shape.rs`; `MergedCompositeTarget { entity, shape, selections }` and the walk writing it; merging comparing `entity` and `shape`; `CompletedCompositeTarget`, `RecordEntity`, `CompletedIdentity`, `FixedRecordId` with `root`, `CompletedSelectionTarget::Composite(CompletedCompositeTarget)`, `CompletedConditional.identity`; `complete` and `top_level` taking `roots`; `RootEntities` with `identity_of` in `entrypoint.rs`; `NetworkProtocol::root_entities`, and the test protocols' (`crates/isograph_compiler`, `crates/isograph_cli`, `crates/isograph_lsp` test profiles); `query_for_entrypoint` reading `root_entities` and passing `roots` to `top_level`. `entrypoint_fetch_plan`'s `EntityShape::Composite(_)` arm is unchanged.
- Core: `ResponseKey` and `SelectionKey::response_key`, which `print_selection_head` reads.
- GraphQL: `graphql_entity_shape` returning `CompositeShape`; `GraphQLFetchPlan { operation_type, roots }`; `RootOperationTypes::root_entities`; `GraphQLNetworkProtocol::root_entities`; the printer reading `plan.operation_type` and `Composite(target).selections`.
- Tests: the completion tests above with `roots`, the `NestedQuery` and `RootQuery` profile tests, and the fetch plan tests asserting `roots` (entrypoints.md).

Depends on 7.

### 11. Query text format (isograph_config, isograph_compiler, graphql_network_protocol, isograph_cli, pet demo)

- `isograph_config`: `GraphQLConfig { query_text_format }`, `QueryTextFormat`, `IsographProjectConfig.graphql`.
- Core: `NetworkProtocol::Options`, `::options`, `query_text` taking `options`; `ProtocolOptions`, `ProfileProtocolOptions`, `protocol_options`, `IsographDb::set_protocol_options`; `query_for_entrypoint` passing `protocol_options(db)`; the test protocols' `Options = ()`.
- GraphQL: `Options = GraphQLConfig`, `options`, `query_text` taking the format; `PrintItem`, `operation_items`, `lay_out`, `collapsed_separator`, `comma_separated`, and the `print_*` functions returning items.
- CLI: the daemon calling `set_protocol_options` at start.
- Pet demo: `isograph.config.json` sets `"graphql": { "query_text_format": "expanded" }`; its regenerated `src/__isograph/` is unchanged byte for byte, because `Expanded` is the text change 9 committed.
- Tests: the printer format tests, the options test in the GraphQL profile group (whose `project_db` sets `Expanded`), the config tests, and the compile tests' collapsed text. `graphql_network_protocol` gains the dev-dependency `graphql-parser`. Relay's `graphql-syntax` (`relay-crates/graphql-syntax`) parses executable documents too, but a path dependency on it puts `relay-crates/common`, `graphql-syntax`, and `fixture-tests` under `cargo fmt -- --check`, which reorders their imports under style edition 2024.

Depends on 10.

## Open decisions

1. Merge key. Options: selectable name and arguments, with every field printed under its normalization alias (upstream keys the same way and aliases only fields with arguments); the response name (reader alias or field name), where one response name with two argument sets is a conflict error, GraphQL's own field-merging rule. Consequences: the first lets two components select `checkins(limit: 3)` and `checkins(limit: 5)` on the same pet, and its only conflicts are alias collisions and duplicate arguments; the second makes composition of independently written fields fail. Held: the first.
2. A variable with no argument and no default. Settled: a compile error at the selection, `MissingFieldArgument` (store.md, open decision 11, option (a)). The earlier options were omitting the argument and printing `null` (upstream).
3. Operation name. Options: the selectable name (upstream: `query HomeRoute`); `Entity__name` (`query Query__HomeRoute`). Consequences: two entrypoints `Query.Foo` and `Pet.Foo` send the same operation name, which servers that key persisted operations or metrics by name conflate. Held: the selectable name, upstream's.
4. Writing when a query fails. Options: no artifacts written (artifact-generation.md's rule for a parse error); everything written except the failing entrypoint's files, which keep their previous contents. Consequences: the first keeps the generated folder consistent and blocks all output for one broken field; the second mixes a stale query with new readers. Held: the first.
5. List arguments. Settled by refactors/pending/validation.md change 13: `MergedValue::List` with alias chunk `a_<items>_c`; the runtime reads the response key from the AST and never builds the alias.
6. `entrypoint Mutation.set_pet_tagline` on a schema field. Options: not allowed (held); allowed with the field's whole response selected; allowed with a selection set written in the entrypoint literal. Consequences: the grammar has no selection set on `entrypoint`, so the third is a grammar change. Held: not allowed; every root field is reachable through a `field Mutation.X { set_pet_tagline(..) { .. } }` declaration.
7. Ownership of change 3. The protocol lookups in `entity_named` and `selectable_named` are the schema integration step. Options: this doc owns it as written; a schema-integration doc owns it and this doc's change 3 is deleted. Held: this doc owns it unless that doc exists when implementation starts.
8. `ValidationError` carrying the protocol's reason. Options: generic over `TNotFetchable` (held); a `String` message from the protocol. Consequences: generic keeps the reason's structure for a later code action at the cost of a type parameter through the diagnostics path; a string discards it.
