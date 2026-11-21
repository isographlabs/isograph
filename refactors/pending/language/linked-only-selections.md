# Linked-only selections: compiler, validation, and artifacts

Status: under discussion. Not approved for implementation. The runtime side (scalar records, entity tables at run time, the walk, derived nodes on scalar records, traces) is `refactors/pending/runtime/pico-runtime.md`, Selections in the Isograph layer. This doc is the schema, validation, query generation, normalization AST, entity tables, type generation, and artifact generation for the same model. The ideas that remain parked are in `refactors/pending/language/scalar-entities.md`.

## The model

There is one kind of entity (`docs-website/docs/design-docs/mental-model.md:5`). Every selection follows a link to an entity's record. Scalars (`String`, `Int`, `Float`, `Boolean`, `ID`, and custom scalars) are entities, and a scalar field's value is its own record: `Pet:1.name` holds a link to the `String` record `Pet:1.name`, whose source field `__value` holds the string.

A reader selection is `{ alias, fieldName, arguments, selections }`, with `selections` absent when the selection has no selection set. It names no kind. What a field is, a source or derived, and which entity its value links to, is a property of (entity, field name), decided at compile time and written into the entity's table:

- A server field is `Source` with its target entity: its source node holds a link, `null`, or a list of these.
- A client field is `Derived` with no target: its derived node's value is returned as is.
- A generated refinement field (`asPet` on `Node`) is `Derived` with its target entity: its value is a link or `null`, read as the target.
- `__self`, on every entity, is the current record, read as the same entity. It is not in the table.
- A selection set is read at each linked record as the target entity. Without one, a scalar target reads the record's `__value` and an object target reads nothing and is one shared empty object.
- `null` stays `null` at every list level, and a source's lists map.

The walk carries the entity it reads a record as. `User:4` read through `actor` is read as `Actor`, and through `actor { asUser { .. } }` as `User`; `Actor.nickname` and `User.nickname` are different fields, each a source or derived on its own.

Consequences for the language:

- `name` returns the string. `name {}` returns `{}`. They differ: an empty selection set reads nothing at the record.
- `bestFriend` and `bestFriend {}` both return `{}` or `null`.
- `name { toUppercase }` selects the client field `toUppercase` declared on `String`, read at the record `String:Pet:1.name`.
- `field String.toUppercase { __self }` is a client field on a scalar entity; its resolver reads the value as `data.__self` (open decision 3).
- A selection set on a client field's result is rejected in v0, as upstream rejects it (open decision 2).

## Upstream

Upstream classifies a selection by the presence of a selection set: with one it is `SelectionType::Object`, without one `SelectionType::Scalar` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:450` to `:476`). Validation rejects:

- a scalar selection of a server field whose target is an object (`../isograph/crates/isograph_schema/src/validate_selection_sets.rs:140`),
- a scalar selection of a client pointer (`:157`),
- an object selection of a server field whose target is a scalar (`:293`),
- an object selection of a client field (`:309`),
- a client field on a scalar parent, "Invalid parent type. `X` is a scalar." (`../isograph/crates/isograph_schema/src/process_client_field_declaration.rs:72`),
- a client pointer to a scalar, "Invalid client pointer target type." (`:135`).

Upstream stores a scalar inline in its parent's record (`StoreRecord`, `../isograph/libs/isograph-react/src/core/IsographEnvironment.ts:150-158`). Its reader AST names a kind per node (`Scalar`, `Linked`, `Resolver`, `Link`, and others, `../isograph/libs/isograph-react/src/core/reader.ts`), and a node carries `fieldName`. It reads a linked field's value from the store, or from a resolver when the node's `condition` is set: an inline fragment's and a client pointer's condition is a reader artifact whose resolver returns the link (`readLinkedFieldData`, `../isograph/libs/isograph-react/src/core/read.ts:651` to `:700`). An inline fragment's condition is compiler-generated: `Actor/asUser/resolver_reader.ts` reads `__typename` and `__link` and returns `data.__typename === "User" ? data.__link : null` (`../isograph/demos/github-demo/src/isograph-components/__isograph/Actor/asUser/resolver_reader.ts`; imported at `../isograph/crates/artifact_content/src/reader_ast.rs:282` to `:285`). `__link` is a client field the compiler adds to every object entity (`../isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:14` to `:38`). Upstream's normalization AST names each linked field's concrete type as a string and imports nothing (`../isograph/crates/artifact_content/src/normalization_ast_text.rs:68`).

Deviations and their reasons:

- `:140`, `:293`, and `:72` go. Reason: a selection's result follows from its selectable's value and its target entity, and scalars gain client fields. `:157`, `:309`, and `:135` stay (open decisions 1 and 2).
- A scalar value is its own record. Reason: one representation; every position is a record link, and a derived node at a scalar has an owner container. Cost: pico-runtime.md, Scalar records.
- The reader AST names no kind and imports no reader artifact; the entity table names each field's origin. Reason: the user's model, in which source or derived is a property of (entity, field name) that every reader of the entity shares.
- The normalization AST imports the tables of the entities it writes, for their storage, where upstream's imports nothing. Reason: identity is a property of the entity (`CompositeShape.identity`, `crates/isograph_compiler/src/protocol_shape.rs:17-20`), which the table holds once.
- A refinement's generated resolver reads nothing from the store; upstream's reads `__typename`. Reason: a link's `entityName` is the `__typename` `normalize` stored the record under (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`).

## i2 today

The parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`:89`, called at `:130`) for any selection. The parser does not change.

Validation rejects both, matching upstream: `target_errors` (`crates/isograph_compiler/src/validation.rs:559`) reports `SelectionSetOnLeaf` (`:583`) and `MissingSelectionSet` (`:587`); `parent_error` (`:314`) reports `InvalidEntityError::LeafParent` (`:322`); `pointer_target_error` reports `InvalidEntityError::LeafTarget` (`:343`). Query generation repeats both selection-set checks (`SelectionMapErrorKind::MissingSelectionSet` at `crates/isograph_compiler/src/merged_selection.rs:1167`, `SelectionSetOnLeaf` at `:1182`), and so does reader generation (`GenerationError::SelectionSetOnScalar` at `crates/isograph_compiler/src/artifact_types.rs:412`, `MissingSelectionSet` at `:419`). A client pointer is not read: `selection_field` returns `SelectionRead::NotRead` for it (`artifact_types.rs:389`), and query generation gives it an empty contribution (`merged_selection.rs:1131`).

## Reader AST (compiler)

```rust
// from crates/isograph_compiler/src/reader_ast.rs
/// What a reader reads for one selection set, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelectionSet(pub Vec<ReaderSelection>);

/// One selection of the field `field_name` of the entity the walk reads the record as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelection {
    /// The key of the value in the resolver's `data`: the reader alias, else the field name.
    pub alias: SelectionName,
    pub field_name: SelectableName,
    /// For a server field, the arguments of its store key; for a client field, the arguments the
    /// selection passes and the default of each of the field's variables it does not pass.
    pub arguments: NormalizationArguments,
    /// `None` when the selection has no selection set.
    pub selections: Option<ReaderSelectionSet>,
}
```

Before (`crates/isograph_compiler/src/reader_ast.rs:11-54`): `ReaderSelection` is an enum of `Scalar(ReaderScalarSelection)`, `Linked(ReaderLinkedSelection)`, `Refinement(ReaderRefinementSelection)`, and `Resolver(ReaderResolverSelection)`. `Scalar` is a selection with `selections: None`. `Linked` keeps `selections`. `Refinement`'s `condition` becomes the generated refinement field the selection names (Generated refinement fields); a refinement takes no arguments (`ArgumentsOnRefinement`). `Resolver`'s `field` goes: the entity table names the field's reader artifact.

## Entity tables (compiler)

```rust
// from crates/isograph_compiler/src/entity_table.rs
/// What the runtime needs about one entity, shared by every reader and every normalization AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityTable {
    pub entity: EntityName,
    /// By field name.
    pub fields: BTreeMap<SelectableName, FieldEntry>,
    pub shape: TableShape,
    pub storage: RecordStorage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldEntry {
    /// A protocol selectable, by its target entity.
    Source(EntityName),
    Derived(DerivedEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedEntry {
    /// The field whose reader artifact the table imports.
    pub field: EntityNameAndSelectableName,
    /// The entity its value links to; `None` for a client field, whose value is opaque.
    pub target: Option<EntityName>,
}

/// `EntityShape` without `Input`, which has no table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableShape {
    Leaf,
    Composite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordStorage {
    /// `IdentitySelection::Select(name)`.
    OwnId(SelectableName),
    /// `IdentitySelection::Omit`, and every leaf entity.
    FromParent,
    /// A root entity (`RootEntities`): `CompletedIdentity::Fixed`.
    Fixed(FixedRecordId),
}

/// The table of `entity` as `client` reads the schema and sources.
#[memo]
pub fn entity_table<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<EntityTable, GenerationError>;
```

`entity_table` builds:

- `fields`: every protocol selectable of `entity` as `Source(target)` (`target` from `server.target.inner_values()`, as `inlined_in_selection` reads it, `crates/isograph_compiler/src/merged_selection.rs:1325-1338`), every `field entity.f` declaration without `to` as `Derived { target: None }`, and every generated refinement field on `entity` that some reader in the project selects as `Derived { target: Some(T) }`. A client pointer is not read in v0 (open decision 1) and is not listed. Which fields are listed is open decision 4.
- `shape`: `EntityShape::Leaf` is `Leaf`, `EntityShape::Composite(_)` is `Composite` (`crates/isograph_compiler/src/protocol_shape.rs:5-13`).
- `storage`: from the entity's `CompositeShape.identity` (`protocol_shape.rs:17-20`), `Fixed` for a root entity as `complete` assigns it (`merged_selection.rs:1450-1458`), and `FromParent` for a leaf.

Which tables are generated: every entity a reader artifact's `tables` or a normalization AST's `tables` names. `ProjectArtifacts` gains them:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct ProjectArtifacts<TLiteralContext> {
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    pub entities: Vec<EntityName>,
    pub entrypoints: Vec<EntrypointArtifacts>,
    pub link_types: Vec<LinkType>,
    /// The table of every entity some artifact imports, sorted by entity.
    pub entity_tables: Vec<EntityTable>,
}
```

Derived or source is decided per (entity, field name) and never per record: records of one entity always agree. A per-record choice is a derived field reading a source under another name (`field User.displayName { nickname login }` returning `data.nickname ?? data.login`), so the static table is not a trap door.

## Normalization AST (compiler)

A leaf selection names its scalar entity and its list depth, because `normalize` writes a record per scalar value and a custom scalar's value can be a JSON array that is one value. Identity leaves the AST: it is the table's `storage`.

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
pub struct NormalizationAst {
    // .. the existing fields, and:
    /// Every entity whose records `normalize` writes with this AST: each `Known` composite target,
    /// each entity a `Discriminator` target can name (`NetworkProtocol::concrete_types`), each leaf
    /// target, and each conditional's entity. Sorted.
    pub tables: BTreeSet<EntityName>,
}

pub enum NormalizationTarget {
    /// A scalar entity: each value is written to its own record.
    Leaf(NormalizationLeafTarget),
    Composite(NormalizationCompositeTarget),
}

pub struct NormalizationLeafTarget {
    pub entity: EntityName,
    /// The number of list levels of the selectable's type.
    pub list_depth: ListDepth,
}

// from crates/isograph_compiler/src/wrapper.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListDepth(pub u32);

impl<T> Wrapper<T> {
    /// The number of `List` layers along the non-`Null` variants, when every non-`Null` variant
    /// of each union has the same number; `None` otherwise.
    pub fn list_depth(&self) -> Option<ListDepth> {
        match self {
            Wrapper::Inner(_) => ListDepth(0).wrap_some(),
            Wrapper::List(list) => list.list_depth().map(|depth| ListDepth(depth.0 + 1)),
            Wrapper::Union(union) => {
                let mut depths = union.0.iter().filter_map(|variant| match variant {
                    WrapperVariant::Inner(_) => ListDepth(0).wrap_some().wrap_some(),
                    WrapperVariant::List(list) => {
                        list.list_depth().map(|depth| ListDepth(depth.0 + 1)).wrap_some()
                    }
                    WrapperVariant::Null => None,
                });
                let first = depths.next().unwrap_or(ListDepth(0).wrap_some())?;
                depths.all(|depth| depth == first.wrap_some()).then_some(first)
            }
        }
    }
}
```

`Null` adds no layer and is not compared: a nullable `W` is `Union([W, Null])` (`crates/isograph_compiler/src/wrapper.rs:5-6`), so `W` and `W | null` have the same depth. A union whose only variant is `Null` has depth 0, since its only value is `null`. By type, on the store fixture's schema (`libs/isograph-runtime/fixtures/store/schema/schema.graphql`):

- `name: String!`, `Inner(String)`: 0.
- `nickname: String`, `Union([Inner(String), Null])`: 0.
- `tags: [String]`, `Union([List(Union([Inner(String), Null])), Null])`: 1.
- `pets: [Pet!]!`, `List(Inner(Pet))`: 1.
- `petGrid: [[Pet]]`: 2.

Before: `NormalizationTarget::Leaf` is a unit variant (`crates/isograph_compiler/src/normalization_ast.rs:51-56`, built at `:146`), and the composite target and each conditional carry a `CompletedIdentity`. The identity of a composite target and of a conditional goes: the runtime reads the concrete entity's table (`recordIdentity`'s search of the conditionals goes with it, `libs/isograph-runtime/src/recordId.ts:73-84`). `MergedSelectionTarget::Leaf` and `CompletedSelectionTarget::Leaf` (`merged_selection.rs`) carry the same `NormalizationLeafTarget`, built in `server_contribution` from `target_name` and `server.target.list_depth()`. A protocol type whose non-`Null` union variants differ in list depth is `SelectionMapErrorKind::MixedListDepth(SelectableKey)`, "`P.f` mixes list and non-list types." No GraphQL type has such a union: a GraphQL union type is a union of entities, and a wrapper union arises only from nullability, whose second variant is `Null`. Two merged leaf selections with one key have one target, since the target follows from the selectable. Query generation is unchanged by the identity's move: `complete` still adds the discriminator and the identity selection to the query (`merged_selection.rs:1547-1566`).

`normalization_ast.rs` in `typescript_artifacts` prints `target: { kind: "Leaf", entityName: "String", listDepth: 0 }` where it printed `target: null` (`crates/typescript_artifacts/src/normalization_ast.rs:91-95`), `target: { kind: "Composite", entity, selections }` for a composite, no `identity`, and `tables: { Pet: () => Pet__entity_table(), .. }`, each entry an `ArtifactValue::ImportCallThunk` (`crates/typescript_artifacts/src/module.rs:44-57`). The runtime types are pico-runtime.md's.

## Schema

### `__self`

Every entity has the selectable `__self`. `selectable_definitions` (`crates/isograph_compiler/src/schema.rs:347`) returns it for the name `__self` on every entity when no declaration defines that name, and `Definitions::Duplicated` when a `field E.__self` declaration also does, which `DuplicateDefinition` reports. GraphQL reserves names that start with `__` for introspection, so no schema field collides with it. `__value` is a store key (`VALUE_KEY`, pico-runtime.md), not a selectable; a user selection named `__value` is `UnknownSelectable`.

```rust
// from crates/isograph_compiler/src/schema.rs
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
    /// `__self`, on every entity: the record the selection is read at, as the same entity.
    SelfRecord,
}
```

`target_entity` returns `self.parent_entity` for `SelfRecord`, so validation checks a selection set on `__self` against the same entity, and `target_errors` applies as for a server field. `inlined_in_selection` (`merged_selection.rs:1314-1348`) gains a `SelfRecord` arm that descends into its selection set on `parent`, so `__self { F }` inside `field E.F` is `InlineCycle` (pico-runtime.md open decision 8).

### Client fields on scalars

A `field` declaration may name a scalar parent (`field String.toUppercase { __self }`). Its resolver receives `link: Link<'String'>`, the scalar record's link, and `data.__self`, the string.

## Validation

```rust
// from crates/isograph_compiler/src/validation.rs
/// What a selection of a selectable whose target entity has this shape needs.
enum TargetShape {
    /// Read bare (its record's `__value`) or through a selection set at each value's record.
    Leaf,
    /// Never selected.
    Input,
    /// Read bare (nothing) or through a selection set at each record.
    Composite,
}

fn target_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    variables: Option<&'a WithSpan<VariableDeclarationList>>,
    key: SelectableKey,
    selection: &'a Selection,
    selection_set: Option<&'a WithSpan<SelectionSet>>,
    target: EntityName,
) -> ValidationErrors<'a, TProfile> {
    // the `let Ok(entity) = ..` arm is unchanged
    match (target_shape::<TProfile>(entity), selection_set) {
        (TargetShape::Input, _) => {
            std::iter::once(ValidationError::InputTarget(target).with_span(selection.name.location))
                .boxed()
        }
        (TargetShape::Leaf | TargetShape::Composite, None) => std::iter::empty().boxed(),
        (TargetShape::Leaf | TargetShape::Composite, Some(nested)) => {
            validate_selection_set(db, client, variables, target, nested.item.reference()).boxed()
        }
    }
}

fn parent_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
) -> Option<InvalidEntityError> {
    let entity = entity_named(db, client, parent).as_ref().ok()?;
    match target_shape::<TProfile>(entity) {
        TargetShape::Composite | TargetShape::Leaf => None,
        TargetShape::Input => InvalidEntityError::InputParent(parent).wrap_some(),
    }
}
```

Before: `target_errors` (`validation.rs:559-591`) returned `SelectionSetOnLeaf` for `(Leaf, Some)` and `MissingSelectionSet` for `(Composite, None)`; `parent_error` (`:314-325`) returned `LeafParent` for `Leaf`.

A client pointer's selection still needs a selection set (upstream's `:157`, open decision 1): `validate_selection` reports `ValidationError::PointerMissingSelectionSet(SelectableKey)`, "`P.f` is a client pointer and needs a selection set.", for a `ClientPointer` selection with none, before `target_errors`.

Removed: `ValidationError::SelectionSetOnLeaf` and `ValidationError::MissingSelectionSet` (`:40-43`), and `InvalidEntityError::LeafParent` (`:74-78`).

Kept, with their messages: `InvalidEntityError::LeafTarget` (a pointer to a scalar, upstream's `:135`); `SelectionSetOnFieldWithoutTarget` (a selection set on a client field, upstream's `:309`); `InputTarget`; `InvalidEntityError::InputParent` and `InvalidEntityError::InputTarget`; `UnknownEntity`, `UnknownSelectable`, `DuplicateDefinition`, `DuplicateResponseName`, `MissingFieldArgument`, `UnknownFieldArgument`, `Argument`, `EntrypointNotFetchable`, `LoadableParentNotFetchable`. `__self` with arguments is `UnknownFieldArgument`.

## Query generation

```rust
// from crates/isograph_compiler/src/merged_selection.rs
// server_contribution, the leaf arm, before (merged_selection.rs:1181-1190):
        (SelectableShape::Direct, EntityShape::Leaf) => match selection.selection_set {
            Some(_) => at_name(SelectionMapErrorKind::SelectionSetOnLeaf(key)).wrap_err(),
            None => {
                MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                    key: selection_key,
                    target: MergedSelectionTarget::Leaf,
                }))
                .wrap_ok()
            }
        },
// after:
        (SelectableShape::Direct, EntityShape::Leaf) => {
            let list_depth = server
                .target
                .list_depth()
                .ok_or_else(|| at_name(SelectionMapErrorKind::MixedListDepth(key)))?;
            MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                key: selection_key,
                target: MergedSelectionTarget::Leaf(NormalizationLeafTarget { entity: target_name, list_depth }),
            }))
            .wrap_ok()
        }
// nested_map, before (merged_selection.rs:1163-1169):
    let nested_map = || {
        let nested = selection
            .selection_set
            .as_ref()
            .ok_or_else(|| at_name(SelectionMapErrorKind::MissingSelectionSet(key)))?;
        merge_selection_set(db, client, context, target_name, nested.item.reference())
    };
// after:
    let nested_map = || match selection.selection_set.as_ref() {
        None => MergedSelectionMap::empty().wrap_ok(),
        Some(nested) => merge_selection_set(db, client, context, target_name, nested.item.reference()),
    };
```

- A leaf with a selection set adds the leaf alone. A selection set on a scalar entity adds nothing to the query: a scalar entity has no protocol selectable, `__self` there reads the scalar record the parent selection already requests, and a client field on a scalar entity selects only `__self` and other client fields on that entity. The nested set is not merged; validation checks it.
- A composite without a selection set adds an empty map, and `complete` adds the discriminator and the identity (`merged_selection.rs:1547-1566`), so the query requests `bestFriend { __typename id }`, which distinguishes `null` and gives the normalizer the record's id.
- A refinement without a selection set adds a conditional with an empty map; `complete` adds its discriminator and identity.
- `selection_contribution` gains `SelectableKind::SelfRecord`: on an object entity with a selection set, `merge_selection_set` of that set on the same entity; otherwise an empty map.
- `SelectionMapErrorKind::SelectionSetOnLeaf` and `MissingSelectionSet` (`:230-232`) go.

## Reader generation and types

`selection_field` (`crates/isograph_compiler/src/artifact_types.rs:315-470`) builds one `ReaderSelection` per selection and the data field's type, by case:

- Server field, scalar target, no set: `selections: None`, type `ArtifactInnerType::Scalar(kind)` (unchanged).
- Server field, scalar target, set: `selections: Some(..)` of the set on the scalar entity, type `ArtifactInnerType::Object(nested)`.
- Server field, object target, no set: `selections: None`, type `ArtifactInnerType::EmptyObject`.
- Server field, object target, set: `selections: Some(..)`, type `Object(nested)` (unchanged).
- `__self`: `field_name: __self` with or without a set; type `Object(nested)`, or bare the parent's own bare type (`EmptyObject` for an object entity, the scalar's type for a scalar entity), not nullable.
- Refinement (`SelectableShape::Refinement`): `field_name: asT`, the generated refinement field on the parent, with or without a set; type `Object(nested)` or `EmptyObject`, nullable. `ArgumentsOnRefinement` stays.
- Client field without a set: `selections: None`, type `ClientFieldOutput(field)` (unchanged). With a set: `GenerationError::SelectionSetOnClientField` (unchanged).
- Client pointer: `SelectionRead::NotRead` (unchanged; open decision 1).

Each selection whose value the walk reads at a record (a server field, a refinement, `__self`) adds its target entity to `FieldTypes::tables`, and each refinement selection adds its generated field to `FieldTypes::selected`, so its reader artifact is generated.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    pub reader_ast: ReaderSelectionSet,
    pub selected: Vec<EntityNameAndSelectableName>,
    /// The parent entity and every entity the walk of `reader_ast` reads a record as, at any
    /// depth. A nested client field's artifact lists its own.
    pub tables: BTreeSet<EntityName>,
}

pub enum ArtifactInnerType {
    Scalar(ScalarKind),
    Object(ArtifactObjectType),
    /// What a client field's resolver returns: its `output_type` artifact.
    ClientFieldOutput(EntityNameAndSelectableName),
    /// What a bare selection of an object entity reads: `EMPTY_DATA`.
    EmptyObject,
}
```

`EmptyObject` prints `Readonly<Record<PropertyKey, never>>`, the type of the runtime's `EMPTY_DATA`; `Record<PropertyKey, never>` is what an empty `parameters` prints today (`demos/pet-demo/src/__isograph/Owner/OwnerBadge/param_type.ts`).

`GenerationError` loses `SelectionSetOnScalar` (`artifact_types.rs:150`) and `MissingSelectionSet` (`:154`, a pointer without a set is validation's `PointerMissingSelectionSet`). The private `ReaderTarget` (`:196-204`) goes.

### Generated refinement fields

A refinement `asT` on an entity `A` is a field the compiler generates on `A`, with target `T`, written at `A/asT/resolver_reader.ts` when some reader selects it. Its reader AST is empty, its tables are empty, and its resolver is printed in the artifact: `({ link }) => (link.entityName === 'T' ? { entityName: 'T', id: link.id } : null)`. A `field A.asT` declaration is `DuplicateDefinition` with it.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldArtifacts<TLiteralContext> {
    // .. the other fields unchanged, and:
    pub resolver: FieldResolver<TLiteralContext>,
}

/// Where the field's resolver is defined.
pub enum FieldResolver<TLiteralContext> {
    /// A `field` declaration's export.
    Declared(FieldDeclarationSource<TLiteralContext>),
    /// A refinement to this entity: the compiler writes the resolver in the reader artifact.
    Refinement(EntityName),
}
```

Before: `resolver: FieldDeclarationSource<TLiteralContext>` (`artifact_types.rs:700-712`). A refinement field has no `param_type.ts` or `output_type.ts`: a reader of `asT` types its data from its own selection set.

## Artifact generation

### Reader AST printing

```rust
// from crates/typescript_artifacts/src/reader_artifact.rs
fn selection_value(selection: &ReaderSelection) -> ArtifactValue {
    ArtifactValue::Object(vec![
        entry("alias", string(selection.alias)),
        entry("fieldName", string(selection.field_name)),
        entry("arguments", arguments_value(selection.arguments.reference())),
        entry(
            "selections",
            selection.selections.as_ref().map_or(ArtifactValue::Null, selection_set_value),
        ),
    ])
}

/// `{ E: E__entity_table, .. }`, one entry per entity, sorted.
fn tables_value(tables: &BTreeSet<EntityName>) -> ArtifactValue {
    ArtifactValue::Object(
        tables
            .iter()
            .map(|entity| entry(entity.as_str(), ArtifactValue::Import(ArtifactImport(entity_table_path(entity.dereference())))))
            .collect(),
    )
}

/// `E/entity_table.ts`.
pub(crate) fn entity_table_path(entity: EntityName) -> ArtifactPath;
```

Before: `selection_value` (`reader_artifact.rs:102-148`) matched `Scalar`, `Linked`, `Refinement`, and `Resolver`, and `Resolver` imported its field's reader artifact. A reader artifact now imports only the user's resolver and the tables. `reader_artifact_file` writes `parentEntityName` and `selectableName` first and `tables` after `readerAst` (pico-runtime.md, Stored resolver results). Every table is an `ArtifactValue::Import`, so `imports_of` (`crates/typescript_artifacts/src/imports.rs:9`) collects it and the module hoists it once under `artifact_import_identifier` (`imports.rs:44`): `Pet/entity_table.ts` is `Pet__entity_table`. The binding is read only when the reader artifact's function is called.

### Entity table modules

`generate_artifacts` (`crates/typescript_artifacts/src/lib.rs:63`) prints one `E/entity_table.ts` per `ProjectArtifacts::entity_tables` entry, after `link_types`, from `crates/typescript_artifacts/src/entity_table.rs`. A table module default-exports a function returning the table, as a reader artifact module does, and imports the reader artifact of each `Derived` entry. Field entries are printed in name order. `GeneratedFunction` prints the refinement resolvers:

```rust
// from crates/typescript_artifacts/src/module.rs
pub enum ArtifactValue {
    // .. the existing variants, and:
    /// A function the compiler writes.
    GeneratedFunction(GeneratedFunction),
}

/// A function whose body the compiler writes.
#[derive(Clone, Debug, PartialEq)]
pub enum GeneratedFunction {
    /// `({ link }) => (link.entityName === "E" ? { entityName: "E", id: link.id } : null)`
    Refinement(EntityName),
}
```

`swc.rs` prints it, as it prints `ExportedThrowingFunction`.

### Generated files for the example

```text
field Query.PetDetailPage($id: ID!) { pet(id: $id) { n1: name n2: name { toUppercase } bestFriend { name } } }
field String.toUppercase { __self }
```

declared in `libs/isograph-runtime/fixtures/store/src/PetDetailPage.ts` and `ToUppercase.ts` over that fixture's schema (`Pet.name: String!`, `Pet.bestFriend: Pet`, `Query.pet(id: ID!): Pet`).

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Query/PetDetailPage/resolver_reader.ts
import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__PetDetailPage__output_type } from './output_type';
import type { Query__PetDetailPage__param } from './param_type';
import { PetDetailPage as resolver } from './../../../PetDetailPage';
import Pet__entity_table from './../../Pet/entity_table';
import Query__entity_table from './../entity_table';
import String__entity_table from './../../String/entity_table';

const readerArtifact = (): ReaderArtifact<
  Query__PetDetailPage__param,
  Query__PetDetailPage__output_type,
  'Query'
> => ({
  parentEntityName: 'Query',
  selectableName: 'PetDetailPage',
  readerAst: [
    {
      alias: 'pet',
      fieldName: 'pet',
      arguments: [['id', { kind: 'Variable', name: 'id' }]],
      selections: [
        { alias: 'n1', fieldName: 'name', arguments: null, selections: null },
        {
          alias: 'n2',
          fieldName: 'name',
          arguments: null,
          selections: [{ alias: 'toUppercase', fieldName: 'toUppercase', arguments: null, selections: null }],
        },
        {
          alias: 'bestFriend',
          fieldName: 'bestFriend',
          arguments: null,
          selections: [{ alias: 'name', fieldName: 'name', arguments: null, selections: null }],
        },
      ],
    },
  ],
  tables: { Pet: Pet__entity_table, Query: Query__entity_table, String: String__entity_table },
  resolver: resolver,
});
export default readerArtifact;
```

The walk reads `Query:__ROOT` as `Query`, `Pet:1` and `Pet:2` as `Pet`, and the scalar records as `String` (for `n2`'s selection set and for the bare `n1` and `name`, whose `shape` it reads). `toUppercase`'s reads are its own artifact's.

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Query/PetDetailPage/param_type.ts
import type { Query____link__output_type } from './../__link/output_type';
import type { Query__PetDetailPage__parameters } from './parameters_type';
import type { String__toUppercase__output_type } from './../../String/toUppercase/output_type';

export type Query__PetDetailPage__param = {
  readonly data: {
    readonly pet: {
      readonly n1: string;
      readonly n2: {
        readonly toUppercase: String__toUppercase__output_type;
      };
      readonly bestFriend: {
        readonly name: string;
      } | null;
    } | null;
  };
  readonly parameters: Query__PetDetailPage__parameters;
  readonly link: Query____link__output_type;
};
```

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/toUppercase/resolver_reader.ts
import type { ReaderArtifact } from '@isograph/runtime';
import type { String__toUppercase__output_type } from './output_type';
import type { String__toUppercase__param } from './param_type';
import { toUppercase as resolver } from './../../../ToUppercase';
import String__entity_table from './../entity_table';

const readerArtifact = (): ReaderArtifact<
  String__toUppercase__param,
  String__toUppercase__output_type,
  'String'
> => ({
  parentEntityName: 'String',
  selectableName: 'toUppercase',
  readerAst: [{ alias: '__self', fieldName: '__self', arguments: null, selections: null }],
  tables: { String: String__entity_table },
  resolver: resolver,
});
export default readerArtifact;
```

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/toUppercase/param_type.ts
import type { String____link__output_type } from './../__link/output_type';

export type String__toUppercase__param = {
  readonly data: {
    readonly __self: string;
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly link: String____link__output_type;
};
```

`String/__link/output_type.ts` is `Link<'String'>`: `link_types` already covers every entity a field is declared on (`artifact_types.rs:663-664`).

The tables: `String/entity_table.ts` and `Pet/entity_table.ts` are pico-runtime.md's (Entity tables). `Query/entity_table.ts` lists `Query`'s protocol selectables as `Source` entries, its client fields (`PetDetailPage` among them) as `Derived` entries with target `null`, `shape: 'Composite'`, and `storage: { kind: 'Fixed', id: '__ROOT' }`.

The normalization AST:

```ts
// generated by crates/typescript_artifacts/src/normalization_ast.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Query/PetDetailPage/normalization_ast.ts
import type { NormalizationAst } from '@isograph/runtime';
import ID__entity_table from './../../ID/entity_table';
import Pet__entity_table from './../../Pet/entity_table';
import String__entity_table from './../../String/entity_table';

const normalizationAst: NormalizationAst<'Query'> = {
  discriminatorResponseKey: '__typename',
  root: { entityName: 'Query', id: '__ROOT' },
  selections: [
    {
      kind: 'Selection',
      responseKey: null,
      selectableName: 'pet',
      arguments: [['id', { kind: 'Variable', name: 'id' }]],
      target: {
        kind: 'Composite',
        entity: { kind: 'Known', entityName: 'Pet' },
        selections: [
          { kind: 'Selection', responseKey: null, selectableName: 'id', arguments: null, target: { kind: 'Leaf', entityName: 'ID', listDepth: 0 } },
          {
            kind: 'Selection',
            responseKey: null,
            selectableName: 'bestFriend',
            arguments: null,
            target: {
              kind: 'Composite',
              entity: { kind: 'Known', entityName: 'Pet' },
              selections: [
                { kind: 'Selection', responseKey: null, selectableName: 'id', arguments: null, target: { kind: 'Leaf', entityName: 'ID', listDepth: 0 } },
                { kind: 'Selection', responseKey: null, selectableName: 'name', arguments: null, target: { kind: 'Leaf', entityName: 'String', listDepth: 0 } },
              ],
            },
          },
          { kind: 'Selection', responseKey: null, selectableName: 'name', arguments: null, target: { kind: 'Leaf', entityName: 'String', listDepth: 0 } },
        ],
      },
    },
  ],
  tables: {
    ID: () => ID__entity_table(),
    Pet: () => Pet__entity_table(),
    String: () => String__entity_table(),
  },
};
export default normalizationAst;
```

Before, each composite target held `identity: { kind: 'Select', selectableName: 'id' }`; it is `Pet`'s table's `storage` now. The root's record stays in `root`, as upstream's `ROOT_ID` does; `Query`'s table is imported only by an AST that writes `Query` records below the root.

A refinement's reader artifact, emitted when a reader selects `asPet` on `Node`:

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Node/asPet/resolver_reader.ts
import type { Link, ReaderArtifact, ResolverParam } from '@isograph/runtime';
import type { Node____link__output_type } from './../__link/output_type';

const readerArtifact = (): ReaderArtifact<
  ResolverParam<Node____link__output_type['entityName']>,
  Link<'Pet'> | null,
  Node____link__output_type['entityName']
> => ({
  parentEntityName: 'Node',
  selectableName: 'asPet',
  readerAst: [],
  tables: {},
  resolver: ({ link }) => (link.entityName === 'Pet' ? { entityName: 'Pet', id: link.id } : null),
});
export default readerArtifact;
```

### Type generation

- `inner_type` prints `EmptyObject` as `Readonly<Record<PropertyKey, never>>`.
- A selection set on a scalar prints the nested object type, as a selection set on an object does.
- A bare `__self` on a scalar entity is the scalar's type, printed by `scalar_type` (`crates/typescript_artifacts/src/generate_types.rs:440-449`).
- `param_type_file` is unchanged: a field on a scalar entity has `link: S____link__output_type`.

## Changes

Each is one commit, in order; each is independently shippable. They are pico-runtime.md's core changes 3, 6, and 7.

1. Scalar records on v1's reader AST. The normalization AST's `Leaf` target with its entity and list depth (`NormalizationLeafTarget`, `ListDepth`, `MixedListDepth`); `normalize`, `check`, and the garbage collector's mark handle scalar records (pico-runtime.md, Scalar records); v1's `Scalar` reader kind reads the link, then `__value`. Observable: a normalized store holds a `String` record per scalar value with its `__value`; reads return the same values; the pet script prints the same output.
2. Entity tables and the minimal reader AST. `ReaderSelection` as a struct; `EntityTable`, `entity_table`, `ProjectArtifacts::entity_tables`, and `E/entity_table.ts`; `FieldTypes::tables`; `parentEntityName`, `selectableName`, and `tables` in every reader artifact; `NormalizationAst::tables` and the identity's removal from composite targets and conditionals; generated refinement fields (`FieldResolver::Refinement`, `GeneratedFunction::Refinement`). Observable: every artifact under `demos/pet-demo/src/__isograph` and the runtime fixtures is regenerated in the new shape, with `E/entity_table.ts` modules and `Node/asPet/` style refinement fields; the pet script prints the same output.
3. Linked-only selections. Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, client fields on scalar entities, and `__self` (`SelfRecord`, with its `inlined_in_selection` arm); `EmptyObject`. Observable: the example compiles to the artifacts above; `read` of `PetDetailPage` on the store fixture returns `{ pet: { n1: 'Ricky', n2: { toUppercase: 'RICKY' }, bestFriend: { name: 'Mango' } } }`; a bare `bestFriend` returns `{}` or `null`, and its query text is `bestFriend { __typename id }`.

## Open decisions

### 1. Client pointers

- (a) Not read in v0, as today (`SelectionRead::NotRead`, `artifact_types.rs:389`; empty query contribution, `merged_selection.rs:1131`), not listed in entity tables, and a pointer selection needs a selection set (upstream's `:157`). Held. Reason: the query does not fetch the target's selections, so a read pointer returns `MissingData` unless another selection fetched them; fetching through a pointer is pico-runtime.md, Follow-ups 2 and 3.
- (b) Read in v0: a `Derived` entry with the pointer's target, read with or without a set. Cost: `MissingData` for data no other selection fetches, and `inlined_fields` must list the pointer and the fields under it (pico-runtime.md open decision 8).

### 2. A selection set on a client field's scalar result

`fullName { toUppercase }` with `fullName` a client field returning a string: a resolver's output is not a record, so `toUppercase` has no record to live on.

- (a) The runtime creates a record for the output, keyed from the producing derived node's record and resolver store key (`Pet:1.fullName`), whose `__value` is the derived node's value, and the table entry gains the output's entity as its target. Cost: a record whose `__value` is a derived node, which the store does not have; `fullName` needs `to String` (today `LeafTarget`) so the compiler knows the output entity.
- (b) Rejected in v0: `SelectionSetOnClientField` and `SelectionSetOnFieldWithoutTarget` stay. Held. Reason: upstream rejects an object selection of a client field (`validate_selection_sets.rs:309`).

### 3. How a client field on a scalar entity reads the scalar

- (a) A `__self` selection: `field String.toUppercase { __self }`, read as `data.__self` through a bare `__self`, which reads the scalar record's `__value`. Held. Reason: every value a resolver reads arrives through a selection, recorded by the walk; the resolver's `link` is the scalar record's, as for every field.
- (b) The value as a resolver parameter (`{ data, parameters, link, value }`). Cost: a read outside the walk, which the runtime makes for fields on scalar entities only, a case the runtime has nowhere else.

### 4. Which fields an entity table lists

A table is per entity and shared by every reader, so whichever set it lists is the same for every reader of the project.

- (a) Every protocol selectable of the entity, every client field declared on it, and every generated refinement field on it that some reader in the project selects. Held. Reason: the table is a function of the schema and the declarations, as the user specified; one rule. Cost: a table imports the reader artifact of every client field of the entity, selected or not, so a bundle that reads one field of `Pet` holds every `Pet` client field (pico-runtime.md, Follow-ups 12).
- (b) Only the fields some reader in the project selects. Cost: a field declared and not yet selected has no entry, and adding its first selection regenerates the table; the import cost of (a) remains for every selected field.
