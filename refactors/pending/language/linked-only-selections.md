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
- The selectable for the record itself is `__self`, where upstream generates `__link` on every object entity (`../isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:14-38`). Reason, as pico-runtime.md open decision 27 gives it: upstream's `__link` is a `ClientScalarSelectable` with `ClientFieldVariant::Link` (`add_link_fields.rs:22-34`), whose value is the record's link and which takes no selection set; `__self` yields what the record is, the data read at the current record under a selection set on an object entity (`__self { name }` is `{ name }`) and the value on a scalar entity (`data.__self` is the string), where upstream has no `__link`. Naming a selection that yields data, and a value on a scalar entity, `__link` would give upstream's name a second meaning. The name is the user's pending decision (pico-runtime.md open decision 27). If `__self` stays, change 3 renames the resolver's link type, which i2 prints under upstream's field name today, from `E/__link/output_type.ts` and `E____link__output_type` to `E/__self/output_type.ts` and `E____self__output_type` (Type generation). If the user picks `__link`, `SELF_RECORD_NAME` is `"__link"`, the walk's reserved name changes with it (pico-runtime.md open decision 27), and the link type keeps its path.
- A refinement's generated resolver reads nothing from the store; upstream's reads `__typename`. Reason: a link's `entityName` is the `__typename` `normalize` stored the record under (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`).

## i2 today

The parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`:89`, called at `:130`) for any selection. The parser does not change. It accepts an empty selection set, which `name {}` and `bestFriend {}` need: `parse_each_chunk` over a level with no chunks returns no slots and no errors (`crates/isograph_parser/src/chunk.rs:167-190`, test `parse_each_chunk_on_an_empty_level_is_no_slots_and_no_errors` at `:1306`), and `empty_selection_sets_hold_zero_selections` (`crates/isograph_parser/src/parse_iso_literal.rs:1663`) parses `{}`, `{ }`, and `{\n}` as a selection set with zero selections and no errors. That test parses a declaration's selection set; a selection's selection set goes through the same `consume_selection_set`.

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

/// The table of `entity` as `client` reads the schema and sources. `reachable` is the set of
/// fields whose reader artifact `project_artifacts` generates; `roots` is every root entity of
/// the project's operations.
pub fn entity_table<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    reachable: &BTreeSet<EntityNameAndSelectableName>,
    roots: &BTreeSet<EntityName>,
) -> Result<EntityTable, GenerationError> {
    let found = entity_named(db, client, entity)
        .clone()
        .map_err(GenerationError::Lookup)?;
    let fields = selectables_for_entity(db, client, entity)
        .clone()
        .map_err(GenerationError::Lookup)?
        .into_iter()
        .filter_map(|selectable| {
            let field = EntityNameAndSelectableName {
                parent_entity_name: entity,
                selectable_name: selectable.name,
            };
            let entry = match selectable.kind {
                SelectableKind::Server(server) => {
                    let target = server.target.inner_values().next().copied()?;
                    match TProfile::NetworkProtocol::selectable_shape(server.data.reference()) {
                        SelectableShape::Direct => FieldEntry::Source(target),
                        SelectableShape::Refinement => FieldEntry::Derived(DerivedEntry {
                            field,
                            target: target.wrap_some(),
                        }),
                    }
                }
                SelectableKind::ClientField => {
                    if !reachable.contains(field.reference()) {
                        return None;
                    }
                    FieldEntry::Derived(DerivedEntry { field, target: None })
                }
                SelectableKind::ClientPointer(_) | SelectableKind::SelfRecord => return None,
            };
            (selectable.name, entry).wrap_some()
        })
        .collect();
    let (shape, storage) = match found.origin.reference() {
        EntityOrigin::Source => (TableShape::Composite, RecordStorage::FromParent),
        EntityOrigin::Protocol(data) => match TProfile::NetworkProtocol::entity_shape(data) {
            EntityShape::Leaf => (TableShape::Leaf, RecordStorage::FromParent),
            EntityShape::Composite(composite) => {
                let storage = if roots.contains(entity.reference()) {
                    RecordStorage::Fixed(FixedRecordId::root())
                } else {
                    match composite.identity {
                        IdentitySelection::Select(name) => RecordStorage::OwnId(name),
                        IdentitySelection::Omit => RecordStorage::FromParent,
                    }
                };
                (TableShape::Composite, storage)
            }
            EntityShape::Input => {
                return GenerationError::Lookup(Diagnostic::new(
                    format!("`{entity}` is an input type and has no entity table."),
                    None,
                ))
                .wrap_err();
            }
        },
    };
    EntityTable {
        entity,
        fields,
        shape,
        storage,
    }
    .wrap_ok()
}
```

`project_artifacts` calls it once per generated table, after it computes `reachable` (`crates/isograph_compiler/src/artifact_types.rs:782-788`). It is a plain function and not a memo: `reachable` is built inside `project_artifacts`, which is not a memo, from every entrypoint of the project. `roots` is the union, over every entrypoint, of its `EntrypointFetch::roots`' `operation` and `others` (`crates/isograph_compiler/src/entrypoint.rs:13-20`, `:85-108`); `complete` gives a root entity's records the same fixed id (`RootEntities::identity_of`, `entrypoint.rs:22-38`). For GraphQL, every operation's `RootEntities` names every root operation type (`root_entities`, `crates/graphql_network_protocol/src/root_operation_types.rs:35-46`), so the union is the schema's `query`, `mutation`, and `subscription` types.

The rules, by `SelectableKind`:

- `Server` with `SelectableShape::Direct`: `Source(target)`, `target` the first inner entity of `server.target`, as `inlined_in_selection` reads it (`crates/isograph_compiler/src/merged_selection.rs:1325-1338`). `__typename` is one: `graphql_selectables` lists it on every object, interface, and union (`crates/graphql_network_protocol/src/protocol_data.rs:535-538`). A target with no inner entity, which no GraphQL type has, is not listed.
- `Server` with `SelectableShape::Refinement`: `Derived { field: E.asT, target: Some(T) }`. Every refinement the schema gives the entity is listed, selected or not. i2's GraphQL protocol defines an entity's refinements in `graphql_selectables` (`crates/graphql_network_protocol/src/protocol_data.rs:506-545`): for a union, one `asT` per member, and for an interface, one `asT` per object type that implements it (`object_types_implementing`, `:584`), each only when `T` is an object type (`refinement_to`, `:404-435`). That is the set upstream generates `asT` fields for: one per concrete child of each abstract entity (`../isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:362-375`). The set is a function of the schema and the entity, so no project-wide query collects it.
- `ClientField`: `Derived { field, target: None }` when `field` is in `reachable` (open decision 4).
- `ClientPointer`: not listed; a client pointer is not read in v0 (open decision 1).
- `SelfRecord`: not listed; the walk recognizes `__self` by name (pico-runtime.md open decision 27).

A source entity's table is `Composite` with `FromParent` storage. No artifact names one in v0: a source entity is reachable only through a client pointer, which is not read, or through an entrypoint, which `EntityNotFetchableError::SourceEntity` rejects (`crates/isograph_compiler/src/entrypoint.rs:46-49`). No artifact names an input entity either, since no selection reads one.

Which tables are generated: every entity the `tables` of a generated reader artifact or of an entrypoint's normalization AST names. The refinement fields' reader artifacts are generated with the tables: for each generated table, `project_artifacts` adds a `FieldArtifacts` with `resolver: FieldResolver::Refinement(T)` for each `Derived` entry whose `target` is `Some(T)` (Generated refinement fields). A refinement artifact's `tables` names only its parent entity, whose table is already generated, so this adds no table. `ProjectArtifacts` gains the tables:

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

`normalization_ast.rs` in `typescript_artifacts` prints the target in two steps.

Change 1 prints `target: { kind: 'Leaf', entityName: 'String', listDepth: 0 }` where it printed `target: null` (`crates/typescript_artifacts/src/normalization_ast.rs:91-95`), and adds `kind: 'Composite'` as the first entry of a composite target, which keeps its `identity`; a conditional is unchanged. The runtime tells the two targets apart by `kind` from this change on. After change 1, `Query/PetDetail/normalization_ast.ts`'s `pet` selection prints:

```ts
// generated by crates/typescript_artifacts/src/normalization_ast.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Query/PetDetail/normalization_ast.ts
    {
      kind: 'Selection',
      responseKey: 'pet____id___v_id',
      selectableName: 'pet',
      arguments: [['id', { kind: 'Variable', name: 'id' }]],
      target: {
        kind: 'Composite',
        entity: { kind: 'Known', entityName: 'Pet' },
        identity: { kind: 'Select', selectableName: 'id' },
        selections: [
          { kind: 'Selection', responseKey: null, selectableName: 'id', arguments: null, target: { kind: 'Leaf', entityName: 'ID', listDepth: 0 } },
          // ..
        ],
      },
    },
```

```rust
// from crates/typescript_artifacts/src/normalization_ast.rs
// change 1:
fn composite_target_value(target: &NormalizationCompositeTarget) -> ArtifactValue {
    ArtifactValue::Object(vec![
        kind("Composite"),
        entry("entity", entity_value(&target.entity)),
        entry("identity", identity_value(&target.identity)),
        entry("selections", selection_set_value(&target.selections)),
    ])
}

fn leaf_target_value(target: &NormalizationLeafTarget) -> ArtifactValue {
    ArtifactValue::Object(vec![
        kind("Leaf"),
        entry("entityName", string(target.entity)),
        entry("listDepth", ArtifactValue::Number(target.list_depth.0.to())),
    ])
}
```

Before (`crates/typescript_artifacts/src/normalization_ast.rs:107-113`): `composite_target_value` has no `kind` entry, and `NormalizationTarget::Leaf` prints `ArtifactValue::Null` (`:91-95`). `listDepth` prints through the existing `ArtifactValue::Number(f64)` (`crates/typescript_artifacts/src/module.rs:47`); `f64: From<u32>` gives `.to()`.

Change 2 removes the `identity` entry from `composite_target_value` and from the conditional's object (`:98-103`), and adds `tables: { Pet: () => Pet__entity_table(), .. }`, each entry an `ArtifactValue::ImportCallThunk` (`crates/typescript_artifacts/src/module.rs:44-57`). The composite target is then `{ kind: 'Composite', entity, selections }`. The runtime types are pico-runtime.md's.

## Schema

### `__self`

Every entity whose shape is `Leaf` or `Composite` has the selectable `__self`; an input entity does not, since `input_objects` reads an input entity's selectables as its input fields (`crates/isograph_compiler/src/artifact_types.rs:610-640`). GraphQL reserves names that start with `__` for introspection, so no schema field collides with it. `__value` is a store key (`VALUE_KEY`, pico-runtime.md), not a selectable; a user selection named `__value` is `UnknownSelectable`.

```rust
// from crates/isograph_compiler/src/schema.rs
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
    /// `__self`, on every entity that is not an input entity: the record the selection is read
    /// at, as the same entity.
    SelfRecord,
}

/// The name of the `SelfRecord` selectable.
pub const SELF_RECORD_NAME: &str = "__self";

/// `__self` on `entity_name`, when `selectable_name` is `__self` and the entity exists and is
/// not an input entity.
fn self_record_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Option<ProfileSelectable<TProfile>> {
    if selectable_name.lookup() != SELF_RECORD_NAME {
        return None;
    }
    let entity = entity_named(db, client, entity_name).as_ref().ok()?;
    match entity.origin.reference() {
        EntityOrigin::Source => {}
        EntityOrigin::Protocol(data) => match TProfile::NetworkProtocol::entity_shape(data) {
            EntityShape::Input => return None,
            EntityShape::Leaf | EntityShape::Composite(_) => {}
        },
    }
    Selectable {
        name: selectable_name,
        parent_entity: entity_name,
        description: format!("The record itself, as the {entity_name} type.")
            .intern()
            .to::<DescriptionValue>()
            .wrap_some(),
        definition: None,
        kind: SelectableKind::SelfRecord,
    }
    .wrap_some()
}
```

A source entity is composite, as `target_shape` reads it (`crates/isograph_compiler/src/validation.rs:545-555`). The description follows upstream's generated `__link`, "A store Link for the {} type." (`../isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:24-28`).

Where `__self` is defined:

- `selectable_definitions` (`crates/isograph_compiler/src/schema.rs:278-294`) chains `self_record_selectable(db, client, entity_name, selectable_name)` onto the source definitions it counts. So `selectable_named` returns it for `__self` when no declaration defines that name, and `Definitions::Duplicated` when a `field E.__self` declaration also does, which `DuplicateDefinition` reports.
- `selectables_for_entity` (the `#[memo]` at `crates/isograph_compiler/src/schema.rs:348`, body `:349-389`) chains `self_record_selectable(db, client, entity_name, SELF_RECORD_NAME.intern().to())` onto `source`, so `__self` is one of the entity's selectables and the LSP's completion (`crates/isograph_lsp/src/completion.rs:135`) offers it. A `field E.__self` declaration makes two definitions under one name, and `selectables_for_entity` drops the name, as it drops every duplicated name.

Every `match` on `SelectableKind` and the `SelfRecord` arm each gains. The search was `SelectableKind::` over `crates/`; the two hits in `crates/isograph_lsp` (`entity_or_selectable.rs:151`, `hover.rs:484`) construct a `ClientPointer` in `#[cfg(test)]` code and match nothing.

- `Selectable::target_entity` (`crates/isograph_compiler/src/schema.rs:108-114`): `SelfRecord => self.parent_entity.wrap_some()`. So `validate_selection` checks a selection set on `__self` against the same entity through `target_errors`, as for a server field (`crates/isograph_compiler/src/validation.rs:494-508`), and hover prints the target.
- `validate_selection`'s arguments (`crates/isograph_compiler/src/validation.rs:479-492`): `SelfRecord => self_record_argument_errors(key, selection)`, every argument `__self` is passed as `UnknownFieldArgument`. `field_argument_errors` cannot serve: it returns no errors when the field has no declaration (`:607-616`).

  ```rust
  // from crates/isograph_compiler/src/validation.rs
  /// `__self` takes no arguments: each one `selection` passes is unknown.
  fn self_record_argument_errors<TProfile: CompilationProfile>(
      field: SelectableKey,
      selection: &Selection,
  ) -> Vec<WithSpan<ProfileValidationError<TProfile>>> {
      selection
          .arguments
          .iter()
          .flat_map(|arguments| arguments.item.0.iter())
          .filter_map(|slot| slot.item.item.as_ref())
          .map(|argument| {
              ValidationError::UnknownFieldArgument(UnknownFieldArgument {
                  field,
                  argument: argument.item.name.item.0,
              })
              .with_span(argument.item.name.location)
          })
          .collect()
  }
  ```

- `entrypoint_fetch_plan` (`crates/isograph_compiler/src/entrypoint.rs:89-95`): the `if let SelectableKind::Server(_)` becomes a `match` whose `SelfRecord` arm returns the new `EntrypointNotFetchableError::SelfRecord(EntityName)`, "`{0}.__self` is the record itself; an entrypoint names a `field` declaration.", located at the declaration's name like `ProtocolSelectable` (`crates/isograph_compiler/src/validation.rs:405-409`). `ClientField` and `ClientPointer(_)` continue to the fetch plan.
- `selection_contribution` (`crates/isograph_compiler/src/merged_selection.rs:1118-1132`): `SelfRecord`, as Query generation states.
- `inlined_in_selection` (`crates/isograph_compiler/src/merged_selection.rs:1314-1348`, its match at `:1324`): `SelfRecord` descends into its selection set on `parent`, so `__self { F }` inside `field E.F` is `InlineCycle` (pico-runtime.md open decision 8); a bare `__self` inlines nothing.
- `source_field_declaration` (`crates/isograph_compiler/src/merged_selection.rs:713-718`): `SelfRecord` joins the `Server(_)` arm and returns `FieldSelectionMapError::NotASourceField(key)`, whose message (`merged_selection.rs:181`) becomes "`{0}` has no `field` declaration and no selection set to inline.", which holds for both arms.
- `selection_field` (`crates/isograph_compiler/src/artifact_types.rs:315-470`, its match at `:346`): `SelfRecord`, as Reader generation and types states.
- `input_objects` (`crates/isograph_compiler/src/artifact_types.rs:624-640`): `SelfRecord` joins the `ClientField | ClientPointer(_)` error arm. `self_record_selectable` defines no `__self` on an input entity, so `selectables_for_entity` of an input object never returns it.
- `project_artifacts` (`crates/isograph_compiler/src/artifact_types.rs:818-830`): `SelfRecord` returns `GenerationError::Lookup` with "`{E}.__self` is declared in a source file and is the record itself.", as the `Server(_)` arm does for a protocol selectable. A `field E.__self` declaration is `Duplicated`, so `selectable_named` fails before this match; the arm exists because the match is exhaustive.

The LSP:

- Hover on `__self` in `Pet`'s selection set prints `field **Pet.__self**: **Pet**` and, after a blank line, `The record itself, as the Pet type.`, from `selectable_hover_markdown` (`crates/isograph_lsp/src/hover.rs:65-78`) with no change.
- Go-to-definition on `__self` has no target: `definition` is `None`, and `definition_at_lsp_position` returns `None` (`crates/isograph_lsp/src/definition.rs:9-18`), as for `__typename`, which the protocol defines with no text.
- Find references on `__self` lists the `__self` selections on that entity: its `reference_target` is `Selectable(SelectableKey { entity, selectable: __self })` (`crates/isograph_lsp/src/entity_or_selectable.rs:33-43`).

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

`EntrypointNotFetchableError::SelfRecord(EntityName)` is new, for `entrypoint E.__self` (`__self`).

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

- A leaf with a selection set adds the leaf alone. A variable used only inside that set is therefore not seen by `merged_arguments`, whose `UndefinedVariable` (`merged_selection.rs:691`) query generation reports today. Two passes still report it, so change 3 adds no check. Validation descends into the set (`target_errors`' `(Leaf, Some)` arm) and checks each selection's arguments: a client field's through `field_argument_errors` (`crates/isograph_compiler/src/validation.rs:600-653`), which reports an argument the field does not declare as `UnknownFieldArgument` and hands each declared argument's value to the protocol, whose `value_errors` reports an undeclared variable as `GraphQLArgumentError::UndefinedVariable` (`crates/graphql_network_protocol/src/argument_errors.rs:285-300`); and `__self`'s through `self_record_argument_errors`, which reports every argument. Reader generation reads the set too, and `resolver_arguments` returns `UndefinedVariable` through `merged_arguments` for a client field's argument (`crates/isograph_compiler/src/artifact_types.rs:499-507`). A scalar entity has no protocol selectable, so no server field argument can sit in that set. A selection set on a scalar entity adds nothing to the query: a scalar entity has no protocol selectable, `__self` there reads the scalar record the parent selection already requests, and a client field on a scalar entity selects only `__self` and other client fields on that entity. The nested set is not merged; validation checks it.
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

Each selection whose value the walk reads at a record (a server field, a refinement, `__self`) adds its target entity to `FieldTypes::tables`. A refinement selection does not add its generated field to `FieldTypes::selected`: the refinement's reader artifact is generated with its parent entity's table (Entity tables (compiler)), selected or not.

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

A refinement `asT` on an entity `A` is a field the compiler generates on `A`, with target `T`, written at `A/asT/resolver_reader.ts` whenever `A`'s table is generated, whether or not a reader selects it, since `A`'s table lists it and imports that file. Its reader AST is empty, its `tables` lists its parent entity `A` alone (`FieldTypes::tables` holds the parent entity of every field, and the walk looks up the parent's table before it reads the empty reader AST), and its resolver is printed in the artifact: `({ link }) => (link.entityName === 'T' ? { entityName: 'T', id: link.id } : null)`. A `field A.asT` declaration is `DuplicateDefinition` with it.

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

`generate_artifacts` (`crates/typescript_artifacts/src/lib.rs:63`) prints one `E/entity_table.ts` per `ProjectArtifacts::entity_tables` entry, after `link_types`, from `crates/typescript_artifacts/src/entity_table.rs`. A table module default-exports a function returning the table, as a reader artifact module does, and imports the reader artifact of each `Derived` entry. Field entries are printed in name order: the iteration order of `EntityTable::fields`, a `BTreeMap<SelectableName, FieldEntry>`. `SelectableName`'s `Ord` is derived over `StringKey` (`crates/string_key_newtype/src/lib.rs:7`), whose `Ord` is derived over `StringId` (`relay-crates/intern/src/string_key.rs:29-31`), which compares the interned strings with `str::cmp` (`relay-crates/intern/src/string.rs:136-140`). That is byte order: uppercase letters, then `_`, then lowercase letters, so a table lists `Constant` and `PetLink` before `__typename`, and `__typename` before `asPet`, `bestFriend`, and `id`. `GeneratedFunction` prints the refinement resolvers:

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

The fixtures are source files of the store fixture project, `libs/isograph-runtime/fixtures/store` (`isograph.config.json`: `src/**/*.ts` over `schema/**/*.graphql`), whose schema has `Pet.name: String!`, `Pet.bestFriend: Pet`, and `Query.pet(id: ID!): Pet` (`libs/isograph-runtime/fixtures/store/schema/schema.graphql:14`, `:37`, `:41`). Each follows the fixture's existing form (`libs/isograph-runtime/fixtures/store/src/PetDetail.ts`, `PetName.ts`).

```ts
// from libs/isograph-runtime/fixtures/store/src/PetDetailPage.ts
import { iso } from './__isograph/iso';

export const PetDetailPage = iso(`
  field Query.PetDetailPage($id: ID!) {
    pet(id: $id) {
      n1: name
      n2: name {
        toUppercase
      }
      bestFriend {
        name
      }
    }
  }
`)(function PetDetailPage({ data }) {
  return data;
});

iso(`entrypoint Query.PetDetailPage`);
```

```ts
// from libs/isograph-runtime/fixtures/store/src/ToUppercase.ts
import { iso } from './__isograph/iso';

export const toUppercase = iso(`
  field String.toUppercase {
    __self
  }
`)(function toUppercase({ data }) {
  return data.__self.toUpperCase();
});
```

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
import type { Query____self__output_type } from './../__self/output_type';
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
  readonly link: Query____self__output_type;
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
import type { String____self__output_type } from './../__self/output_type';

export type String__toUppercase__param = {
  readonly data: {
    readonly __self: string;
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly link: String____self__output_type;
};
```

`String/__self/output_type.ts` is `Link<'String'>`: `link_types` already covers every entity a field is declared on (`artifact_types.rs:663-664`).

The tables, printed with entries in name order (Entity table modules):

```ts
// generated by crates/typescript_artifacts/src/entity_table.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/entity_table.ts
import type { EntityTable } from '@isograph/runtime';
import String__toUppercase__resolver_reader from './toUppercase/resolver_reader';

const entityTable = (): EntityTable => ({
  fields: {
    toUppercase: { kind: 'Derived', target: null, artifact: String__toUppercase__resolver_reader },
  },
  shape: 'Leaf',
  storage: { kind: 'FromParent' },
});
export default entityTable;
```

```ts
// generated by crates/typescript_artifacts/src/entity_table.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/ID/entity_table.ts
import type { EntityTable } from '@isograph/runtime';

const entityTable = (): EntityTable => ({
  fields: {},
  shape: 'Leaf',
  storage: { kind: 'FromParent' },
});
export default entityTable;
```

```ts
// generated by crates/typescript_artifacts/src/entity_table.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Pet/entity_table.ts
import type { EntityTable } from '@isograph/runtime';
import Pet__Constant__resolver_reader from './Constant/resolver_reader';
import Pet__NullableCheckins__resolver_reader from './NullableCheckins/resolver_reader';
import Pet__PetLink__resolver_reader from './PetLink/resolver_reader';
import Pet__PetName__resolver_reader from './PetName/resolver_reader';
import Pet__RecentCheckins__resolver_reader from './RecentCheckins/resolver_reader';
import Pet__ThrowingName__resolver_reader from './ThrowingName/resolver_reader';

const entityTable = (): EntityTable => ({
  fields: {
    Constant: { kind: 'Derived', target: null, artifact: Pet__Constant__resolver_reader },
    NullableCheckins: { kind: 'Derived', target: null, artifact: Pet__NullableCheckins__resolver_reader },
    PetLink: { kind: 'Derived', target: null, artifact: Pet__PetLink__resolver_reader },
    PetName: { kind: 'Derived', target: null, artifact: Pet__PetName__resolver_reader },
    RecentCheckins: { kind: 'Derived', target: null, artifact: Pet__RecentCheckins__resolver_reader },
    ThrowingName: { kind: 'Derived', target: null, artifact: Pet__ThrowingName__resolver_reader },
    __typename: { kind: 'Source', target: 'String' },
    bestFriend: { kind: 'Source', target: 'Pet' },
    checkins: { kind: 'Source', target: 'Checkin' },
    id: { kind: 'Source', target: 'ID' },
    name: { kind: 'Source', target: 'String' },
    nickname: { kind: 'Source', target: 'String' },
    stats: { kind: 'Source', target: 'PetStats' },
    tagline: { kind: 'Source', target: 'String' },
  },
  shape: 'Composite',
  storage: { kind: 'OwnId', selectableName: 'id' },
});
export default entityTable;
```

`ID` is a scalar with no client field, so its table has no entries; it is generated because the normalization AST writes `ID` records (`Pet:1.id`). `Pet`'s six client fields are each reachable from an entrypoint of the fixture today (each has a `resolver_reader.ts` under `libs/isograph-runtime/fixtures/store/src/__isograph/Pet/`). `__typename` is a protocol selectable of every object (`graphql_selectables`, `crates/graphql_network_protocol/src/protocol_data.rs:535-538`). `Query/entity_table.ts` lists `Query`'s protocol selectables as `Source` entries, its reachable client fields (`PetDetailPage` among them) as `Derived` entries with target `null`, `shape: 'Composite'`, and `storage: { kind: 'Fixed', id: '__ROOT' }`.

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
      responseKey: 'pet____id___v_id',
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

The query text and the response the tests of change 3 normalize, with `{ id: '1' }`:

```graphql
# generated by crates/graphql_network_protocol/src/query_text.rs
# libs/isograph-runtime/fixtures/store/src/__isograph/Query/PetDetailPage/query_text.graphql
query PetDetailPage($id: ID!) {
  pet____id___v_id: pet(id: $id) {
    __typename
    id
    bestFriend {
      __typename
      id
      name
    }
    name
  }
}
```

```ts
// from libs/isograph-runtime/test/read.test.ts
const petDetailPageResponse = {
  pet____id___v_id: {
    __typename: 'Pet',
    id: '1',
    bestFriend: { __typename: 'Pet', id: '2', name: 'Mango' },
    name: 'Ricky',
  },
};
```

After `normalize`, the store holds what pico-runtime.md's Example states: `Query:__ROOT`'s `pet____id___"1"` links to `Pet:1`; `Pet:1.name` links to `String:Pet:1.name`, whose `__value` is `'Ricky'`; `Pet:1.bestFriend` links to `Pet:2`; `Pet:2.name` links to `String:Pet:2.name`, whose `__value` is `'Mango'`; and `Pet:1.id` and `Pet:2.id` link to `ID:Pet:1.id` and `ID:Pet:2.id`, whose `__value`s are `'1'` and `'2'`. `read` of `PetDetailPage` with `{ id: '1' }` returns `{ pet: { n1: 'Ricky', n2: { toUppercase: 'RICKY' }, bestFriend: { name: 'Mango' } } }`.

A refinement's reader artifact, emitted for each refinement of `Node` whenever `Node`'s table is generated, as change 2 prints it (change 3 renames `Node____link__output_type` to `Node____self__output_type`, Type generation):

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Node/asPet/resolver_reader.ts
import type { Link, ReaderArtifact, ResolverParam } from '@isograph/runtime';
import type { Node____link__output_type } from './../__link/output_type';
import Node__entity_table from './../entity_table';

const readerArtifact = (): ReaderArtifact<
  ResolverParam<Node____link__output_type['entityName']>,
  Link<'Pet'> | null,
  Node____link__output_type['entityName']
> => ({
  parentEntityName: 'Node',
  selectableName: 'asPet',
  readerAst: [],
  tables: { Node: Node__entity_table },
  resolver: ({ link }) => (link.entityName === 'Pet' ? { entityName: 'Pet', id: link.id } : null),
});
export default readerArtifact;
```

The fixture for change 2's refinement observable, in the same project:

```ts
// from libs/isograph-runtime/fixtures/store/src/NodeAsPet.ts
import { iso } from './__isograph/iso';

export const NodeAsPet = iso(`
  field Query.NodeAsPet($id: ID!) {
    node(id: $id) {
      asPet {
        name
      }
    }
  }
`)(function NodeAsPet({ data }) {
  return data;
});

iso(`entrypoint Query.NodeAsPet`);
```

```graphql
# generated by crates/graphql_network_protocol/src/query_text.rs
# libs/isograph-runtime/fixtures/store/src/__isograph/Query/NodeAsPet/query_text.graphql
query NodeAsPet($id: ID!) {
  node____id___v_id: node(id: $id) {
    __typename
    id
    ... on Pet {
      __typename
      id
      name
    }
  }
}
```

The walk reads `node`'s record as `Node`, so `Node`'s table is generated, and with it a reader artifact for each of `Node`'s refinements: `asBlogItem`, `asCheckin`, and `asPet` (`BlogItem`, `Checkin`, and `Pet` implement `Node`, `libs/isograph-runtime/fixtures/store/schema/schema.graphql:57`, `:50`, `:35`).

```ts
// generated by crates/typescript_artifacts/src/entity_table.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Node/entity_table.ts
import type { EntityTable } from '@isograph/runtime';
import Node__NodeLink__resolver_reader from './NodeLink/resolver_reader';
import Node__asBlogItem__resolver_reader from './asBlogItem/resolver_reader';
import Node__asCheckin__resolver_reader from './asCheckin/resolver_reader';
import Node__asPet__resolver_reader from './asPet/resolver_reader';

const entityTable = (): EntityTable => ({
  fields: {
    NodeLink: { kind: 'Derived', target: null, artifact: Node__NodeLink__resolver_reader },
    __typename: { kind: 'Source', target: 'String' },
    asBlogItem: { kind: 'Derived', target: 'BlogItem', artifact: Node__asBlogItem__resolver_reader },
    asCheckin: { kind: 'Derived', target: 'Checkin', artifact: Node__asCheckin__resolver_reader },
    asPet: { kind: 'Derived', target: 'Pet', artifact: Node__asPet__resolver_reader },
    id: { kind: 'Source', target: 'ID' },
  },
  shape: 'Composite',
  storage: { kind: 'OwnId', selectableName: 'id' },
});
export default entityTable;
```

The responses the tests normalize, each with `{ id }` as its variables:

```ts
// from libs/isograph-runtime/test/read.test.ts
const checkinNodeResponse = {
  node____id___v_id: { __typename: 'Checkin', id: '7' },
};
const petNodeResponse = {
  node____id___v_id: { __typename: 'Pet', id: '1', name: 'Ricky' },
};
```

- "a Node-typed node(id:) selection of asPet on a Checkin record is null": after normalizing `checkinNodeResponse`, `read` of `NodeAsPet` with `{ id: '7' }` returns `{ node: { asPet: null } }`. `Query:__ROOT`'s `node____id___"7"` links to `Checkin:7`, and `Node.asPet`'s resolver returns `null` for a link whose `entityName` is `Checkin`.
- "asPet on a Pet record reads the selections at the record as Pet": after normalizing `petNodeResponse`, `read` of `NodeAsPet` with `{ id: '1' }` returns `{ node: { asPet: { name: 'Ricky' } } }`.

### Type generation

- `inner_type` prints `EmptyObject` as `Readonly<Record<PropertyKey, never>>`.
- A selection set on a scalar prints the nested object type, as a selection set on an object does.
- A bare `__self` on a scalar entity is the scalar's type, printed by `scalar_type` (`crates/typescript_artifacts/src/generate_types.rs:440-449`).
- `param_type_file` is unchanged: a field on a scalar entity has `link: S____self__output_type`.
- If the name `__self` stays (Deviations), the link type is renamed after it:

```rust
// from crates/typescript_artifacts/src/generate_types.rs
/// `E____self__output_type`.
pub(crate) fn link_type_name(entity: EntityName) -> String {
    format!("{entity}____self__output_type")
}

pub(crate) fn link_type_path(entity: EntityName) -> ArtifactPath {
    ArtifactPath(format!("{entity}/__self/output_type.ts"))
}
```

Before (`crates/typescript_artifacts/src/generate_types.rs:43-50`): `E____link__output_type` at `E/__link/output_type.ts`. Every artifact that imports the link type is regenerated with the new name.

## Changes

Each is one commit, in order; each is independently shippable. They are pico-runtime.md's core changes 3, 6, and 7.

1. Scalar records on v1's reader AST. The normalization AST's `Leaf` target with its entity and list depth (`NormalizationLeafTarget`, `ListDepth`, `MixedListDepth`), and `kind: 'Composite'` on the composite target, which keeps its `identity` (Normalization AST (compiler), change 1's printed form); `normalize`, `check`, and the garbage collector's mark handle scalar records (pico-runtime.md, Scalar records); v1's `Scalar` reader kind reads the link, then `__value`. Observable: a normalized store holds a `String` record per scalar value with its `__value`; reads return the same values; the pet script prints the same output.
2. Entity tables and the minimal reader AST. `ReaderSelection` as a struct; `EntityTable`, `entity_table`, `ProjectArtifacts::entity_tables`, and `E/entity_table.ts`; `FieldTypes::tables`; `parentEntityName`, `selectableName`, and `tables` in every reader artifact; `NormalizationAst::tables` and the identity's removal from composite targets and conditionals (`CompletedIdentity` leaves `NormalizationCompositeTarget` and `NormalizationConditional`); generated refinement fields (`FieldResolver::Refinement`, `GeneratedFunction::Refinement`), one per refinement of each entity whose table is generated. Observable: every artifact under `demos/pet-demo/src/__isograph` and the runtime fixtures is regenerated in the new shape, with `E/entity_table.ts` modules and `Node/asPet/` style refinement fields; the store fixture gains `NodeAsPet.ts`, and `read` of `NodeAsPet` returns `{ node: { asPet: null } }` on `Checkin:7` and `{ node: { asPet: { name: 'Ricky' } } }` on `Pet:1` (Generated files for the example); the pet script prints the same output.
3. Linked-only selections. Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, client fields on scalar entities, and `__self` (`SelfRecord`, with its `inlined_in_selection` arm); `EmptyObject`; the link type's rename to `E/__self/output_type.ts` while the name `__self` stands (Deviations). Observable: the store fixture gains `PetDetailPage.ts` and `ToUppercase.ts`, which compile to the artifacts above; `read` of `PetDetailPage` on the store fixture returns `{ pet: { n1: 'Ricky', n2: { toUppercase: 'RICKY' }, bestFriend: { name: 'Mango' } } }`; a bare `bestFriend` returns `{}` or `null`, and its query text is `bestFriend { __typename id }`.

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

- (a) Every protocol selectable of the entity, every client field declared on it, and every generated refinement field on it. Cost: the reader artifact of every declared client field must be generated, reachable or not, since the table imports it; today `project_artifacts` generates a reader artifact only for a field reachable from an entrypoint (`resolver_reader: reachable.contains(..).then_some(..)`, `crates/isograph_compiler/src/artifact_types.rs:782-788`, `:847`), so (a) changes that rule and generates a reader for every declaration. A bundle that reads one field of `Pet` holds every `Pet` client field (pico-runtime.md, Follow-ups 12).
- (b) Every protocol selectable of the entity, every generated refinement field on it, and only the client fields on it whose reader artifact is generated: the fields reachable from an entrypoint, as `project_artifacts` computes them (`crates/isograph_compiler/src/artifact_types.rs:728-731`, `:782-805`). Held. Reason: a client field no entrypoint reaches is never read at run time, and a table entry for it would import a `resolver_reader.ts` the compiler does not write, which breaks the build; the bundle holds only reachable fields. Cost: a field declared and not yet reachable has no entry, and the change that makes it reachable regenerates its entity's table.
