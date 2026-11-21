# Linked-only selections: compiler, validation, and artifacts

Status: under discussion. Not approved for implementation. The runtime side (scalar records, the walk, default readers at run time, derived nodes on scalar records, traces) is `refactors/pending/runtime/pico-runtime.md`, Selections in the Isograph layer. This doc is the schema, validation, query generation, normalization AST, type generation, and artifact generation for the same model. The ideas that remain parked are in `refactors/pending/language/scalar-entities.md`.

## The model

Every selection follows a link to an entity's record. Scalars (`String`, `Int`, `Float`, `Boolean`, `ID`, and custom scalars) are entities, and a scalar field's value is its own record: `Pet:1.name` holds a link to the `String` record `Pet:1.name`, whose source field `__value` holds the string. The selection's kind decides what it reads and which targets it takes:

- `Link`: a server field, whose source node holds a link, `null`, or a list of these. Its target is `Selections` (the selection set, read at each linked record) or `Bare` (the target entity's default reader, walked at each linked record, then its `read` called on the data). `String`'s default reader reads `__self` and returns it; an object entity's reads nothing and returns an empty object.
- `Self`: `__self`, on every entity, the record the walk is at. It reads nothing and takes the targets `Link` takes: `__self { name }` at `Pet:1` reads `name` at `Pet:1`, `__self { toUppercase }` at `String:Pet:1.name` reads `toUppercase` there, and a bare `__self` applies the record entity's default reader.
- `Value`: the stored value of a scalar record, its `__value` source, as is. It takes no target, and it is not a user selectable: only a generated default reader of a scalar entity selects it. `String`'s default reader is `[{ kind: 'Value', alias: 'value' }]` with `read: (data) => data.value`, so a bare `__self` at a scalar record ends at a `Value`.
- `Derived`: a client field, whose derived node's value is returned as is. It takes no target in v0 (open decision 5).
- `DerivedLink`: a refinement, whose derived node's value is a link or `null`. Its target is `Selections` or `Bare`, as `Link`'s.
- `null` stays `null` at every list level, and lists map.

Consequences for the language:

- `name` returns the string. `name {}` returns `{}`. They differ: an empty selection set is not `String`'s default reader.
- `bestFriend` and `bestFriend {}` both return `{}` or `null`, because an object entity's default reader reads nothing and returns the empty object.
- `name { toUppercase }` selects the client field `toUppercase` declared on `String`, read at the record `String:Pet:1.name`.
- `field String.toUppercase { __self }` is a client field on a scalar entity; its resolver reads the value as `data.__self`, a bare `__self` through `String`'s default reader (open decision 6).
- A selection set on a client field's result is rejected in v0, as upstream rejects it (open decision 5).

## Upstream

Upstream classifies a selection by the presence of a selection set: with one it is `SelectionType::Object`, without one `SelectionType::Scalar` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:450` to `:476`). Validation rejects:

- a scalar selection of a server field whose target is an object (`../isograph/crates/isograph_schema/src/validate_selection_sets.rs:140`),
- a scalar selection of a client pointer (`:157`),
- an object selection of a server field whose target is a scalar (`:293`),
- an object selection of a client field (`:309`),
- a client field on a scalar parent, "Invalid parent type. `X` is a scalar." (`../isograph/crates/isograph_schema/src/process_client_field_declaration.rs:72`),
- a client pointer to a scalar, "Invalid client pointer target type." (`:135`).

Upstream stores a scalar inline in its parent's record (`StoreRecord`, `../isograph/libs/isograph-react/src/core/IsographEnvironment.ts:150-158`). It reads a linked field's value from the store, or from a resolver when the node's `condition` is set: an inline fragment's and a client pointer's condition is a reader artifact whose resolver returns the link (`ReaderLinkedField`, `../isograph/libs/isograph-react/src/core/reader.ts:103` to `:121`; `readLinkedFieldData`, `../isograph/libs/isograph-react/src/core/read.ts:651` to `:700`). An inline fragment's condition is compiler-generated: `Actor/asUser/resolver_reader.ts` reads `__typename` and `__link` and returns `data.__typename === "User" ? data.__link : null` (`../isograph/demos/github-demo/src/isograph-components/__isograph/Actor/asUser/resolver_reader.ts`; imported at `../isograph/crates/artifact_content/src/reader_ast.rs:282` to `:285`). `__link` is a client field the compiler adds to every object entity (`../isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:14` to `:38`).

Deviations and their reasons:

- `:140`, `:293`, and `:72` go. Reason: a selection's result follows from its selectable's value and its target entity, and scalars gain client fields. `:157`, `:309`, and `:135` stay (open decisions 4 and 5).
- A scalar value is its own record. Reason: one representation; every position is a record link, and a derived node at a scalar has an owner container. Cost: pico-runtime.md, Scalar records.
- Every selection's value comes from a source or derived selectable, as upstream's `condition` already does for inline fragments and pointers.
- A refinement's generated resolver reads nothing from the store; upstream's reads `__typename`. Reason: a link's entity is part of its identity in i2 (`Link.entityName`, `libs/isograph-runtime/src/store.ts:23-27`), and `normalize` stores a record under the entity its response object names (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`).

## i2 today

The parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`:89`, called at `:130`) for any selection. The parser does not change.

Validation rejects both, matching upstream: `target_errors` (`crates/isograph_compiler/src/validation.rs:559`) reports `SelectionSetOnLeaf` (`:583`) and `MissingSelectionSet` (`:587`); `parent_error` (`:314`) reports `InvalidEntityError::LeafParent` (`:322`); `pointer_target_error` reports `InvalidEntityError::LeafTarget` (`:343`). Query generation repeats both selection-set checks (`SelectionMapErrorKind::MissingSelectionSet` at `crates/isograph_compiler/src/merged_selection.rs:1167`, `SelectionSetOnLeaf` at `:1182`), and so does reader generation (`GenerationError::SelectionSetOnScalar` at `crates/isograph_compiler/src/artifact_types.rs:412`, `MissingSelectionSet` at `:419`). A client pointer is not read: `selection_field` returns `SelectionRead::NotRead` for it (`artifact_types.rs:389`), and query generation gives it an empty contribution (`merged_selection.rs:1131`).

## Reader AST (compiler)

```rust
// from crates/isograph_compiler/src/reader_ast.rs
/// What a reader reads for one selection set, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelectionSet(pub Vec<ReaderSelection>);

/// One selection. The kind decides where its value comes from and which targets it takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReaderSelection {
    /// A protocol selectable: its source node on the record holds a link, `null`, or a list.
    Link(ReaderLinkSelection),
    /// `__self`: the record the walk is at, then its target there. Reads nothing.
    SelfRecord(ReaderSelfSelection),
    /// The `__value` source node of a scalar record. Only in generated default readers.
    Value(ReaderValueSelection),
    /// A client field: its derived node's value, as is.
    Derived(ReaderDerivedSelection),
    /// A refinement: its derived node's value, a link or `null`.
    DerivedLink(ReaderDerivedLinkSelection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderLinkSelection {
    /// The key of the value in the resolver's `data`: the reader alias, else the selectable name.
    pub alias: SelectionName,
    pub selectable: SelectableName,
    pub arguments: NormalizationArguments,
    pub target: ReaderLinkTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelfSelection {
    pub alias: SelectionName,
    pub target: ReaderLinkTarget,
}

/// The alias of a `Value` selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderValueSelection(pub SelectionName);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderDerivedSelection {
    pub alias: SelectionName,
    /// The field whose reader artifact the selection imports.
    pub field: EntityNameAndSelectableName,
    /// The arguments the selection passes, and the default of each of the field's variables it
    /// does not pass.
    pub arguments: NormalizationArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderDerivedLinkSelection {
    pub alias: SelectionName,
    /// The generated refinement field whose reader artifact the selection imports.
    pub field: EntityNameAndSelectableName,
    pub target: ReaderLinkTarget,
}

/// What a selection that holds links reads at each linked record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReaderLinkTarget {
    /// The selection's selection set.
    Selections(ReaderSelectionSet),
    /// No selection set: this entity's default reader, `E/default_reader.ts`.
    Bare(EntityName),
}
```

Before (`crates/isograph_compiler/src/reader_ast.rs:11-54`): `ReaderSelection` is an enum of `Scalar(ReaderScalarSelection)`, `Linked(ReaderLinkedSelection)`, `Refinement(ReaderRefinementSelection)`, and `Resolver(ReaderResolverSelection)`. `ReaderScalarSelection` (a stored scalar read whole) goes: a scalar is a `Link` to its record, read through `String`'s default reader, whose own reader AST is one `Value`. `ReaderLinkedSelection` becomes `ReaderLinkSelection` with `selections` moved into `ReaderLinkTarget::Selections`. `ReaderResolverSelection` becomes `ReaderDerivedSelection` unchanged. `ReaderRefinementSelection`'s `condition` becomes a generated refinement field read by `DerivedLink` (Generated refinement fields); a refinement takes no arguments (`ArgumentsOnRefinement`), so `ReaderDerivedLinkSelection` has none.

`DerivedLink` is a fifth kind beside `Link`, `Self`, `Value`, and `Derived`: a refinement is a derived node (pico-runtime.md, Refinements) whose value is a link, so it needs a link target, which `Derived` does not take in v0. Under open decision 5 (a), `Derived` gains the same `ReaderLinkTarget`, and `DerivedLink` folds into it.

## Normalization AST (compiler)

A leaf selection names its scalar entity and its list depth, because `normalize` writes a record per scalar value and a custom scalar's value can be a JSON array that is one value.

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
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
    /// The number of `List` layers, when every variant of a union has the same number;
    /// `None` otherwise.
    pub fn list_depth(&self) -> Option<ListDepth>;
}
```

Before: `NormalizationTarget::Leaf` is a unit variant (`crates/isograph_compiler/src/normalization_ast.rs:51-56`, built at `:146`). `MergedSelectionTarget::Leaf` and `CompletedSelectionTarget::Leaf` (`merged_selection.rs`) carry the same `NormalizationLeafTarget`, built in `server_contribution` from `target_name` and `server.target.list_depth()`. A protocol type whose union variants differ in list depth is `SelectionMapErrorKind::MixedListDepth(SelectableKey)`, "`P.f` mixes list and non-list types." No GraphQL type has such a union. Two merged leaf selections with one key have one target, since the target follows from the selectable.

`normalization_ast.rs` in `typescript_artifacts` prints `target: { kind: "Leaf", entityName: "String", listDepth: 0 }` where it printed `target: null` (`crates/typescript_artifacts/src/normalization_ast.rs:91-95`), and `target: { kind: "Composite", entity, identity, selections }` for a composite. The runtime types are pico-runtime.md's.

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
    /// `__self`, on every entity: the record the selection is read at.
    SelfRecord,
}
```

`target_entity` returns `self.parent_entity` for `SelfRecord`, so validation checks a selection set on `__self` against the same entity, and `target_errors` applies as for a server field.

### Client fields on scalars

A `field` declaration may name a scalar parent (`field String.toUppercase { __self }`). Its resolver receives `link: Link<'String'>`, the scalar record's link, and `data.__self`, the string.

## Validation

```rust
// from crates/isograph_compiler/src/validation.rs
/// What a selection of a selectable whose target entity has this shape needs.
enum TargetShape {
    /// Read through its default reader or a selection set at each value's record.
    Leaf,
    /// Never selected.
    Input,
    /// Read through its default reader or a selection set at each record.
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

A client pointer's selection still needs a selection set (upstream's `:157`, open decision 4): `validate_selection` reports `ValidationError::PointerMissingSelectionSet(SelectableKey)`, "`P.f` is a client pointer and needs a selection set.", for a `ClientPointer` selection with none, before `target_errors`.

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

- Server field, scalar target, no set: `Link` with `Bare(scalar)`, type `ArtifactInnerType::Scalar(kind)` (unchanged).
- Server field, scalar target, set: `Link` with `Selections` of the set on the scalar entity, type `ArtifactInnerType::Object(nested)`.
- Server field, object target, no set: `Link` with `Bare(target)`, type `ArtifactInnerType::EmptyObject`.
- Server field, object target, set: `Link` with `Selections`, type `Object(nested)` (unchanged).
- `__self`: `SelfRecord` with `Selections` or `Bare(parent entity)`; type `Object(nested)`, or bare the parent's own bare type (`EmptyObject` for an object entity, the scalar's type for a scalar entity), not nullable.
- Refinement (`SelectableShape::Refinement`): `DerivedLink { field: (abstract entity, asT) }` with `Selections` or `Bare(T)`; type `Object(nested)` or `EmptyObject`, nullable. `ArgumentsOnRefinement` stays.
- Client field without a set: `Derived`, type `ClientFieldOutput(field)` (unchanged). With a set: `GenerationError::SelectionSetOnClientField` (unchanged).
- Client pointer: `SelectionRead::NotRead` (unchanged; open decision 4).

Each `Bare` target adds its entity to `FieldTypes::default_readers`, and each refinement selection adds its generated field to `FieldTypes::selected`, so its reader artifact is generated.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    pub reader_ast: ReaderSelectionSet,
    pub selected: Vec<EntityNameAndSelectableName>,
    /// Every entity a `Bare` target in `reader_ast` names, at any depth.
    pub default_readers: BTreeSet<EntityName>,
}

pub enum ArtifactInnerType {
    Scalar(ScalarKind),
    Object(ArtifactObjectType),
    /// What a client field's resolver returns: its `output_type` artifact.
    ClientFieldOutput(EntityNameAndSelectableName),
    /// What an object entity's default reader returns.
    EmptyObject,
}
```

`EmptyObject` prints `Readonly<Record<PropertyKey, never>>`, the type `Pet/default_reader.ts` returns; `Record<PropertyKey, never>` is what an empty `parameters` prints today (`demos/pet-demo/src/__isograph/Owner/OwnerBadge/param_type.ts`).

`GenerationError` loses `SelectionSetOnScalar` (`artifact_types.rs:150`) and `MissingSelectionSet` (`:154`, a pointer without a set is validation's `PointerMissingSelectionSet`). The private `ReaderTarget` (`:196-204`) goes: the target is `reader_ast.rs`'s `ReaderTarget`.

### Generated refinement fields

A refinement `asT` on an abstract entity `A` is a field whose artifacts the compiler writes at `A/asT/`: `param_type.ts` (`data: Record<PropertyKey, never>`, `parameters`, `link: A____link__output_type`), `output_type.ts` (`ReturnType<typeof resolver>` of `./resolver`), `resolver.ts`, and `resolver_reader.ts` with an empty reader AST, which imports `./resolver` as a declared field's imports its source file. The runtime has no case for it.

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
    /// A refinement to this concrete entity: the compiler writes `resolver.ts`.
    Refinement(EntityName),
}
```

Before: `resolver: FieldDeclarationSource<TLiteralContext>` (`artifact_types.rs:700-712`).

## Artifact generation

### Reader AST printing

```rust
// from crates/typescript_artifacts/src/reader_artifact.rs
fn selection_value(selection: &ReaderSelection) -> ArtifactValue {
    match selection {
        ReaderSelection::Link(link) => ArtifactValue::Object(vec![
            kind("Link"),
            entry("alias", string(link.alias)),
            entry("selectableName", string(link.selectable)),
            entry("arguments", arguments_value(link.arguments.reference())),
            entry("target", link_target_value(link.target.reference())),
        ]),
        ReaderSelection::SelfRecord(self_record) => ArtifactValue::Object(vec![
            kind("Self"),
            entry("alias", string(self_record.alias)),
            entry("target", link_target_value(self_record.target.reference())),
        ]),
        ReaderSelection::Value(value) => {
            ArtifactValue::Object(vec![kind("Value"), entry("alias", string(value.0))])
        }
        ReaderSelection::Derived(derived) => ArtifactValue::Object(vec![
            kind("Derived"),
            entry("alias", string(derived.alias)),
            entry("arguments", arguments_value(derived.arguments.reference())),
            entry("readerArtifact", reader_artifact_import(derived.field)),
        ]),
        ReaderSelection::DerivedLink(derived) => ArtifactValue::Object(vec![
            kind("DerivedLink"),
            entry("alias", string(derived.alias)),
            entry("readerArtifact", reader_artifact_import(derived.field)),
            entry("target", link_target_value(derived.target.reference())),
        ]),
    }
}

fn link_target_value(target: &ReaderLinkTarget) -> ArtifactValue {
    match target {
        ReaderLinkTarget::Selections(selections) => ArtifactValue::Object(vec![
            kind("Selections"),
            entry("selections", selection_set_value(selections)),
        ]),
        ReaderLinkTarget::Bare(entity) => ArtifactValue::Object(vec![
            kind("Bare"),
            entry("defaultReader", ArtifactValue::Import(ArtifactImport(default_reader_path(entity.dereference())))),
        ]),
    }
}

/// The field's `resolver_reader.ts`, as `ReaderResolverSelection`'s `readerArtifact` was.
fn reader_artifact_import(field: EntityNameAndSelectableName) -> ArtifactValue {
    ArtifactValue::Import(ArtifactImport(field_artifact_path(field, RESOLVER_READER)))
}

/// `E/default_reader.ts`.
pub(crate) fn default_reader_path(entity: EntityName) -> ArtifactPath;
```

Before: `selection_value` (`reader_artifact.rs:102-148`) matched `Scalar`, `Linked`, `Refinement`, and `Resolver`. A default reader module default-exports a thunk, as a reader artifact module does, so `defaultReader` is the import itself. Every import is an `ArtifactValue::Import`, so `imports_of` (`crates/typescript_artifacts/src/imports.rs:9`) collects the default readers with the reader artifacts, and the module hoists each once under `artifact_import_identifier` (`imports.rs:44`): `String/default_reader.ts` is `String__default_reader`. `reader_artifact_file` writes `selectableName: "<name>",` first (pico-runtime.md, Stored resolver results).

### Default reader modules

`ProjectArtifacts` gains the default readers its fields name:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct ProjectArtifacts<TLiteralContext> {
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    pub entities: Vec<EntityName>,
    pub entrypoints: Vec<EntrypointArtifacts>,
    pub link_types: Vec<LinkType>,
    /// The union of every field's `default_readers`, sorted.
    pub default_readers: Vec<DefaultReaderArtifact>,
}

pub struct DefaultReaderArtifact {
    pub entity: EntityName,
    pub kind: DefaultReaderKind,
}

pub enum DefaultReaderKind {
    /// A scalar entity: reads `__value` and returns it, at the scalar's type.
    Value(ScalarKind),
    /// An object entity: reads nothing and returns the one empty object.
    EmptyObject,
}
```

`DefaultReaderKind` follows from the entity's `EntityKind` (`crates/isograph_compiler/src/schema.rs:36-41`). Which scalars are built in is the protocol's: `ScalarKind::Known` is what `graphql_network_protocol` assigns through `NetworkProtocol::entity_kind` (`crates/graphql_network_protocol/src/lib.rs:58-61`); the core crate reads `EntityKind` and does not depend on `graphql_network_protocol`. A custom scalar is `ScalarKind::Custom`; its default reader is open decision 1.

`typescript_artifacts` prints the modules. `generate_artifacts` (`crates/typescript_artifacts/src/lib.rs:63`) prints one module per `default_readers` entry after `link_types`, and each refinement field's `resolver.ts`. The function bodies the compiler writes are one `ArtifactValue` variant and one `TypeScriptItem` variant:

```rust
// from crates/typescript_artifacts/src/module.rs
pub enum ArtifactValue {
    // .. the existing variants, and:
    /// A function the compiler writes.
    GeneratedFunction(GeneratedFunction),
}

pub(crate) enum TypeScriptItem {
    // .. the existing variants, and:
    /// `const empty: Readonly<Record<PropertyKey, never>> = Object.freeze({});`
    DeclaredEmptyObject,
}

/// A function whose body the compiler writes.
#[derive(Clone, Debug, PartialEq)]
pub enum GeneratedFunction {
    /// `(data) => data.value`
    ReadValue,
    /// `() => empty`
    ReadEmptyObject,
    /// `({ link }: P) => (link.entityName === "E" ? link : null)`
    Refinement(RefinementFunction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinementFunction {
    pub param: String,
    pub entity: EntityName,
}
```

`swc.rs` prints each, as it prints `ExportedThrowingFunction`.

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
import String__default_reader from './../../String/default_reader';
import String__toUppercase__resolver_reader from './../../String/toUppercase/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Query__PetDetailPage__param,
  Query__PetDetailPage__output_type,
  'Query'
> => ({
  selectableName: 'PetDetailPage',
  readerAst: [
    {
      kind: 'Link',
      alias: 'pet',
      selectableName: 'pet',
      arguments: [['id', { kind: 'Variable', name: 'id' }]],
      target: {
        kind: 'Selections',
        selections: [
          {
            kind: 'Link',
            alias: 'n1',
            selectableName: 'name',
            arguments: null,
            target: { kind: 'Bare', defaultReader: String__default_reader },
          },
          {
            kind: 'Link',
            alias: 'n2',
            selectableName: 'name',
            arguments: null,
            target: {
              kind: 'Selections',
              selections: [
                {
                  kind: 'Derived',
                  alias: 'toUppercase',
                  arguments: null,
                  readerArtifact: String__toUppercase__resolver_reader,
                },
              ],
            },
          },
          {
            kind: 'Link',
            alias: 'bestFriend',
            selectableName: 'bestFriend',
            arguments: null,
            target: {
              kind: 'Selections',
              selections: [
                {
                  kind: 'Link',
                  alias: 'name',
                  selectableName: 'name',
                  arguments: null,
                  target: { kind: 'Bare', defaultReader: String__default_reader },
                },
              ],
            },
          },
        ],
      },
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
```

`String__default_reader` is imported once and referenced twice.

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
import String__default_reader from './../default_reader';

const readerArtifact = (): ReaderArtifact<
  String__toUppercase__param,
  String__toUppercase__output_type,
  'String'
> => ({
  selectableName: 'toUppercase',
  readerAst: [
    { kind: 'Self', alias: '__self', target: { kind: 'Bare', defaultReader: String__default_reader } },
  ],
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

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/default_reader.ts
import type { DefaultReader } from '@isograph/runtime';

const defaultReader = (): DefaultReader<{ readonly value: string }, string> => ({
  readerAst: [{ kind: 'Value', alias: 'value' }],
  read: (data) => data.value,
});
export default defaultReader;
```

An object entity's, emitted when a reader selects `Pet` bare (`bestFriend` with no selection set):

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Pet/default_reader.ts
import type { DefaultReader } from '@isograph/runtime';

const empty: Readonly<Record<PropertyKey, never>> = Object.freeze({});
const defaultReader = (): DefaultReader<Record<PropertyKey, never>, Readonly<Record<PropertyKey, never>>> => ({
  readerAst: [],
  read: () => empty,
});
export default defaultReader;
```

A refinement's resolver, emitted when a reader selects `asPet` on `Node`:

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Node/asPet/resolver.ts
import type { Node__asPet__param } from './param_type';

export const resolver = ({ link }: Node__asPet__param) => (link.entityName === 'Pet' ? link : null);
```

The normalization AST's `name` selection, before and after:

```ts
// generated by crates/typescript_artifacts/src/normalization_ast.rs
// before:
{ kind: 'Selection', responseKey: null, selectableName: 'name', arguments: null, target: null }
// after:
{ kind: 'Selection', responseKey: null, selectableName: 'name', arguments: null, target: { kind: 'Leaf', entityName: 'String', listDepth: 0 } }
```

### Type generation

- `inner_type` prints `EmptyObject` as `Readonly<Record<PropertyKey, never>>`.
- A selection set on a scalar prints the nested object type, as a selection set on an object does.
- A bare `__self` on a scalar entity is the scalar's type, printed by `scalar_type` (`crates/typescript_artifacts/src/generate_types.rs:440-449`).
- `param_type_file` is unchanged: a field on a scalar entity has `link: S____link__output_type`.

## Changes

Each is one commit, in order; each is independently shippable. They are pico-runtime.md's core changes 3, 4, and 5.

1. The selection kinds on v1's language, before scalar records. The compiler emits `Link` with `Selections` for a v1 linked selection, `Derived` for a client field, and `DerivedLink` of a generated refinement field for a refinement; a v1 scalar selection stays v1's `Scalar` kind (the value read whole) until change 2. The runtime walk reads the new AST. Validation is unchanged. Observable: every artifact under `demos/pet-demo/src/__isograph` and the runtime fixtures is regenerated in the new shape, with `Node/asPet/` style refinement fields; the pet script prints the same output.
2. Scalar records. The normalization AST's `Leaf` target; `normalize`, `check`, and the garbage collector's mark handle scalar records (pico-runtime.md, Scalar records); `__self` (`SelfRecord`) and `Value`; a scalar selection becomes `Link` with `Bare(scalar)` and the `Scalar` kind goes; `default_reader.ts` per scalar entity a reader selects. Observable: a normalized store holds a `String` record per scalar value with its `__self`; reads return the same values; the pet script prints the same output.
3. Linked-only selections. Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, and client fields on scalar entities; `EmptyObject`; the object default reader. Observable: the example compiles to the artifacts above; `read` of `PetDetailPage` on the store fixture returns `{ pet: { n1: 'Ricky', n2: { toUppercase: 'RICKY' }, bestFriend: { name: 'Mango' } } }`; a bare `bestFriend` returns `{}` or `null`, and its query text is `bestFriend { __typename id }`.

## Open decisions

### 1. A custom scalar's default reader

- (a) The config supplies it: each `custom_scalar_types` entry gains an optional default reader import beside its type import (`custom_scalar_imports`, `crates/typescript_artifacts/src/generate_types.rs:473-490`). Cost: a config field and its validation.
- (b) The compiler generates the built-in scalars' default reader, `__value` at the scalar's type (`scalar_type`, `unknown` without a config entry). Held. Reason: upstream's behavior; upstream has no default reader and returns every scalar whole (`case 'Scalar'`, `../isograph/libs/isograph-react/src/core/read.ts`), as i2 does today. (a) can be added later at the same path.

### 2. Which entities get default reader modules

- (a) Only the entities some `Bare` target names. Held. Reason: AGENTS.md, nothing we do not use.
- (b) Every entity, as `link_types` covers every entity a field is declared on (`crates/isograph_compiler/src/artifact_types.rs:663-664`). Cost: unread modules.

### 3. The object default reader's value

`selectionSetDataAreEqual` compares `Bare` outputs by `===` (pico-runtime.md, Equality).

- (a) One frozen empty object per module, returned on every call. Held. Reason: a recomputation that reaches the same bare object selection compares equal, with no structural rule for `Bare`.
- (b) A new `{}` per call, and `Bare` outputs compared structurally. Cost: a deep comparison of what a default reader returns, which for a later user-defined entity's reader can be any object.
- (c) A new `{}` per call, compared by `===`. Cost: every recomputation of a reader with a bare object selection marks its dependents.

### 4. Client pointers

- (a) Not read in v0, as today (`SelectionRead::NotRead`, `artifact_types.rs:389`; empty query contribution, `merged_selection.rs:1131`), and a pointer selection needs a selection set (upstream's `:157`). Held. Reason: the query does not fetch the target's selections, so a read pointer returns `MissingData` unless another selection fetched them; fetching through a pointer is pico-runtime.md, Follow-ups 2 and 3.
- (b) Read in v0: `DerivedLink` with `Selections` or `Bare(target)`. Cost: `MissingData` for data no other selection fetches.

### 5. A selection set on a client field's scalar result

`fullName { toUppercase }` with `fullName` a client field returning a string: a resolver's output is not a record, so `toUppercase` has no record to live on.

- (a) The runtime creates a record for the output, keyed from the producing derived node's record and resolver store key (`Pet:1.fullName`), whose `__value` is the derived node's value. `Derived` then takes `ReaderLinkTarget` as `Link` does, client pointers and outputs are both links, and `DerivedLink` folds into `Derived`; `Self` stays the only read of a value as is. Cost: a record whose `__value` is a derived node, which the store does not have; `fullName` needs `to String` (today `LeafTarget`) so the compiler knows the output entity.
- (b) Rejected in v0: `SelectionSetOnClientField` and `SelectionSetOnFieldWithoutTarget` stay. Held. Reason: upstream rejects an object selection of a client field (`validate_selection_sets.rs:309`).

### 6. How a client field on a scalar entity reads the scalar

- (a) A `__self` selection: `field String.toUppercase { __self }`, read as `data.__self` through a bare `Self`, which walks `String`'s default reader. Held. Reason: every value a resolver reads arrives through a selection, recorded by the walk; the resolver's `link` is the scalar record's, as for every field.
- (b) The value as a resolver parameter (`{ data, parameters, link, value }`). Cost: a read outside the walk, which the runtime makes for fields on scalar entities only, a case the runtime has nowhere else.
