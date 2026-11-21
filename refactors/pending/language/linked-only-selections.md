# Linked-only selections: compiler, validation, and artifacts

Status: under discussion. Not approved for implementation. The runtime side (positions, the walk, derived nodes at scalar positions, traces) is `refactors/pending/runtime/pico-runtime.md`, Selections in the Isograph layer. This doc is the parser, schema, validation, query generation, type generation, and artifact generation for the same model. The parked ideas that remain parked are in `refactors/pending/language/scalar-entities.md`.

## The model

Every selection follows a link to an entity. Scalars (`String`, `Int`, `Float`, `Boolean`, `ID`, and custom scalars) are entities. A selection reads one value at the current position through its selectable, then applies its target to each item of that value:

- A selectable is a source (a server field: a source node in the store) or derived (a client field, a client pointer, `__self`, or a refinement: a derived node).
- A selection with a selection set reads those selections at each item: at the linked record when the target is an object entity, at a scalar position when the target is a scalar entity.
- A selection without a selection set calls the target entity's default reader on each item. `String`'s default reader returns the value. An object entity's default reader returns an empty object.
- `null` stays `null` at every list level. Lists map.

Consequences for the language:

- `name` returns the string. `name {}` returns `{}`. They differ: the selection set is not the default reader.
- `bestFriend` and `bestFriend {}` both return `{}` or `null`: an object entity's default reader returns the empty object.
- `name { toUppercase }` selects the client field `toUppercase` declared on `String`.
- `field String.toUppercase { __self }` is a client field on a scalar entity. Its resolver receives `data.__self` and `self`, both the string.
- `field Pet.fullName to String { firstName lastName }` is a client field whose target entity is `String`. `fullName` returns its value through `String`'s default reader; `fullName { toUppercase }` reads `toUppercase` at a scalar position whose value is `fullName`'s result.
- `__self` is a selectable on every entity. At a record it is the record's link, so `__self { name }` reads `name` at the same record. At a scalar it is the value.
- A refinement `asPet` is a derived selectable whose resolver the compiler generates.

## Upstream

Upstream classifies a selection by the presence of a selection set: with one it is `SelectionType::Object`, without one `SelectionType::Scalar` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:450` to `:476`). Validation then rejects:

- a scalar selection of a server field whose target is an object (`../isograph/crates/isograph_schema/src/validate_selection_sets.rs:140`),
- a scalar selection of a client pointer (`:157`),
- an object selection of a server field whose target is a scalar (`:293`),
- an object selection of a client field (`:309`),
- a client field on a scalar parent, "Invalid parent type. `X` is a scalar." (`../isograph/crates/isograph_schema/src/process_client_field_declaration.rs:72`),
- a client pointer to a scalar, "Invalid client pointer target type." (`:135`).

Upstream already reads three kinds of linked value through one node. `ReaderLinkedField` carries `condition`, a reader artifact whose resolver returns the link, for an inline fragment and for a client pointer, and `null` for a server field (`../isograph/libs/isograph-react/src/core/reader.ts:103` to `:121`). `readLinkedFieldData` takes the value from the store, or calls `condition`'s resolver when it is set, and then reads the selections at the resulting links (`../isograph/libs/isograph-react/src/core/read.ts:651` to `:700`). An inline fragment's condition is a compiler-generated reader artifact: `Actor/asUser/resolver_reader.ts` reads `__typename` and `__link` and returns `data.__typename === "User" ? data.__link : null` (`../isograph/demos/github-demo/src/isograph-components/__isograph/Actor/asUser/resolver_reader.ts`; imported at `../isograph/crates/artifact_content/src/reader_ast.rs:282` to `:285`). `__link` is a client field the compiler adds to every object entity (`../isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:14` to `:38`), read by a `Link` reader node that returns the root (`read.ts:189` to `:191`).

Deviations and their reasons:

- The four selection-set rules and the two parent and target rules go, except `:309` for a client field without `to` (Validation). Reason: a selection's result follows from its selectable's value and its target entity alone, and scalars gain client fields.
- Every linked value comes from one selectable kind, source or derived, as upstream's `condition` already does for inline fragments and pointers, extended to every selection (server scalars included).
- `__self` replaces the `Link` node's special case: it is a generated derived field like upstream's `asUser`, and the runtime has no case for it. At a record it returns the link, which upstream's `__link` returns.
- A refinement's generated resolver reads nothing from the store. Upstream's reads `__typename` (`asUser/resolver_reader.ts`). Reason: in i2 a link's entity is part of its identity (`Link.entityName`, `libs/isograph-runtime/src/store.ts:23-27`), and `normalize` stores a record under the entity its response object names (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`).
- A client pointer to an object entity reads at its target record without refetching. Upstream refetches through `readClientPointerData` when the target lacks data (`read.ts:723` onward). In v0 such pointers stay unread (open decision 5).

## i2 today

The parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`:89`, called at `:130`) for any selection. The parser does not change.

Validation rejects both, matching upstream: `target_errors` (`crates/isograph_compiler/src/validation.rs:559`) reports `SelectionSetOnLeaf` (`:583`) and `MissingSelectionSet` (`:587`); `parent_error` (`:314`) reports `InvalidEntityError::LeafParent` (`:322`); `pointer_target_error` reports `InvalidEntityError::LeafTarget` (`:343`). Query generation repeats both checks (`SelectionMapErrorKind::MissingSelectionSet` at `crates/isograph_compiler/src/merged_selection.rs:1167`, `SelectionSetOnLeaf` at `:1182`), and so does reader generation (`GenerationError::SelectionSetOnScalar` at `crates/isograph_compiler/src/artifact_types.rs:412`, `MissingSelectionSet` at `:419`). A client pointer is not read: `selection_field` returns `SelectionRead::NotRead` for it (`artifact_types.rs:389`), and query generation gives it an empty contribution (`merged_selection.rs:1131`).

## Reader AST (compiler)

```rust
// from crates/isograph_compiler/src/reader_ast.rs
/// What a reader reads for one selection set, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelectionSet(pub Vec<ReaderSelection>);

/// One selection: its selectable's value at the position, then its target applied to each
/// item of that value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelection {
    /// The key of the value in the resolver's `data`: the reader alias, else the selectable name.
    pub alias: SelectionName,
    pub selectable: ReaderSelectable,
    /// The number of list wrappers of the selectable's type.
    pub list_depth: ListDepth,
    pub target: ReaderTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReaderSelectable {
    /// A protocol selectable: the source node under its store key on the position's record.
    Source(ReaderSourceSelectable),
    /// A client field, a client pointer to a scalar entity, `__self`, or a refinement: the
    /// derived node of the field's reader artifact at the position.
    Derived(ReaderDerivedSelectable),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSourceSelectable {
    pub selectable: SelectableName,
    pub arguments: NormalizationArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderDerivedSelectable {
    /// The field whose reader artifact the selection imports.
    pub field: EntityNameAndSelectableName,
    /// The arguments the selection passes, and the default of each of the field's variables it
    /// does not pass.
    pub arguments: NormalizationArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReaderTarget {
    /// The selection's selection set.
    Selections(ReaderSelections),
    /// No selection set: the default reader of the target.
    Bare(DefaultReader),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelections {
    pub position: PositionKind,
    pub selections: ReaderSelectionSet,
}

/// Where a selection set under a selection is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionKind {
    /// At each linked record: the target is an object entity.
    Record,
    /// At each value: the target is a scalar entity.
    Scalar,
}

/// The default reader a bare selection imports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefaultReader {
    /// The target entity's: `E/default_reader.ts`.
    Entity(EntityName),
    /// The identity on a client field's output, for a client field without `to`:
    /// `P/f/default_reader.ts` (open decision 2).
    FieldOutput(EntityNameAndSelectableName),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListDepth(pub u32);
```

Before (`crates/isograph_compiler/src/reader_ast.rs:11-54`): `ReaderSelection` is an enum of `Scalar(ReaderScalarSelection)`, `Linked(ReaderLinkedSelection)`, `Refinement(ReaderRefinementSelection)`, and `Resolver(ReaderResolverSelection)`. All four structs go. `ReaderScalarSelection`'s and `ReaderLinkedSelection`'s `selectable` and `arguments` become `ReaderSourceSelectable`; `ReaderResolverSelection`'s `field` and `arguments` become `ReaderDerivedSelectable`; `ReaderRefinementSelection`'s `condition` becomes the generated refinement field (Generated derived fields).

`ListDepth` is the number of `List` layers of the selectable's `Wrapper<EntityName>` (`crates/isograph_compiler/src/wrapper.rs:8-22`): a server field's `server.target`, a pointer's `ClientPointerTarget::Parsed`. `ListDepth(0)` for a client field without `to`, `__self`, and a refinement.

```rust
// from crates/isograph_compiler/src/wrapper.rs
impl<T> Wrapper<T> {
    /// The number of `List` layers, when every variant of a union has the same number;
    /// `None` otherwise.
    pub fn list_depth(&self) -> Option<ListDepth>;
}
```

A union whose variants have different list depths (`to Pet | [Pet]`) is `GenerationError::MixedListDepth(SelectableKey)`, "`P.f` mixes list and non-list types, so it cannot be read." No GraphQL type has such a union; a `to` clause can.

## Schema

### `__self`

Every entity has the selectable `__self`. `selectable_definitions` (`crates/isograph_compiler/src/schema.rs:347`) returns it for the name `__self` on every entity when no declaration defines that name, and `Definitions::Duplicated` when a `field E.__self` declaration also does, which `DuplicateDefinition` reports.

```rust
// from crates/isograph_compiler/src/schema.rs
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a value of `T`: a link for an object entity, the
    /// value for a scalar entity.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
    /// `__self`, on every entity: the position's own link or value.
    SelfValue,
}

impl<TSelectableData> Selectable<TSelectableData> {
    pub fn target_entity(&self) -> Option<EntityName> {
        match self.kind.reference() {
            SelectableKind::ClientField => None,
            SelectableKind::ClientPointer(target) => target.target_entity(),
            SelectableKind::Server(server) => server.target.inner_values().next().copied(),
            SelectableKind::SelfValue => self.parent_entity.wrap_some(),
        }
    }
}
```

Before: `SelectableKind` has no `SelfValue` (`schema.rs:83-90`); the doc comment of `ClientPointer` reads "its resolver returns a link to a `T`" (`:86`).

### Client fields on scalars and `to <scalar>`

A `field` declaration may name a scalar parent (`field String.toUppercase { __self }`) and a scalar `to` type (`field Pet.fullName to String { firstName lastName }`). Both were rejected (`LeafParent`, `LeafTarget`). A `to` type of a scalar entity is how a client field states that its output is that entity's value (open decision 6).

## Validation

```rust
// from crates/isograph_compiler/src/validation.rs
/// What a selection of a selectable whose target entity has this shape needs.
enum TargetShape {
    /// Selected through its default reader or a selection set at each value.
    Leaf,
    /// Never selected.
    Input,
    /// Selected through its default reader or a selection set at each record.
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

fn pointer_target_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: WithSpan<EntityNameWrapper>,
) -> Option<WithSpan<ProfileValidationError<TProfile>>> {
    // the `Undefined` check is unchanged
    let entity = entity_named(db, client, target).as_ref().ok()?;
    match target_shape::<TProfile>(entity) {
        TargetShape::Composite | TargetShape::Leaf => None,
        TargetShape::Input => ValidationError::InvalidEntity(InvalidEntityError::InputTarget(target))
            .with_span(name.location)
            .wrap_some(),
    }
}
```

Before: `target_errors` (`validation.rs:559-591`) returned `SelectionSetOnLeaf` for `(Leaf, Some)` and `MissingSelectionSet` for `(Composite, None)`; `parent_error` (`:314-325`) returned `LeafParent` for `Leaf`; `pointer_target_error` (`:330-350`) returned `LeafTarget` for `Leaf`.

Removed: `ValidationError::SelectionSetOnLeaf` and `ValidationError::MissingSelectionSet` (`:40-43`), `InvalidEntityError::LeafParent` (`:74-78`), and `InvalidEntityError::LeafTarget` (`:82-86`).

Kept, with their messages: `SelectionSetOnFieldWithoutTarget` (a selection set on a client field without `to`, upstream's `:309`); `InputTarget`; `InvalidEntityError::InputParent` and `InvalidEntityError::InputTarget`; `UnknownEntity`, `UnknownSelectable`, `DuplicateDefinition`, `DuplicateResponseName`, `MissingFieldArgument`, `UnknownFieldArgument`, `Argument`, `EntrypointNotFetchable`, `LoadableParentNotFetchable`. `__self` on an entity validates its selection set on that same entity, through `target_entity`. `__self` with arguments is `UnknownFieldArgument` (it declares no variable).

## Query generation

```rust
// from crates/isograph_compiler/src/merged_selection.rs
// server_contribution, the two arms that change:
        (SelectableShape::Direct, EntityShape::Leaf) => {
            MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                key: selection_key,
                target: MergedSelectionTarget::Leaf,
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

- A leaf with a selection set adds the leaf alone. A selection set on a scalar entity adds nothing to the query: a scalar entity has no protocol selectable, and its derived selectables (`__self`, client fields on it) read only the scalar. The nested set is not merged; validation checks it.
- A composite without a selection set adds an empty map, and `complete` adds the discriminator and the identity (`merged_selection.rs:1547-1566`), so the query requests `bestFriend { __typename id }`, which distinguishes `null` and gives the normalizer the link. The normalization AST is unchanged.
- A refinement without a selection set adds a conditional with an empty map; `complete` adds its discriminator and identity.
- `SelectionMapErrorKind::SelectionSetOnLeaf` and `MissingSelectionSet` (`:230-232`) go.

`selection_contribution` gains the `SelfValue` arm, and a client pointer to a scalar entity contributes as a client field does, because v0 reads it (open decision 5):

```rust
// from crates/isograph_compiler/src/merged_selection.rs
    match selectable.kind.reference() {
        SelectableKind::Server(server) => server_contribution(db, client, context, key, selection, arguments.merged, server),
        SelectableKind::ClientField => client_field_contribution(db, client, context, key, selection, arguments),
        SelectableKind::ClientPointer(target) => match pointer_target_kind(db, client, target)? {
            PointerTargetKind::Scalar => client_field_contribution(db, client, context, key, selection, arguments),
            PointerTargetKind::Object => MergedSelectionMap::empty().wrap_ok(),
        },
        SelectableKind::SelfValue => match (entity_named(db, client, parent).kind::<TProfile::NetworkProtocol>(), selection.selection_set.as_ref()) {
            (EntityKind::Object, Some(nested)) => merge_selection_set(db, client, context, parent, nested.item.reference()),
            (EntityKind::Object, None) | (EntityKind::Scalar(_), _) => MergedSelectionMap::empty().wrap_ok(),
        },
    }

/// Whether a client pointer's `to` type names scalar entities or object entities.
enum PointerTargetKind {
    Scalar,
    Object,
}
```

`pointer_target_kind` reads the first inner entity of `ClientPointerTarget::Parsed` (`target_entity`) and its `EntityKind`. A pointer whose `to` union mixes scalar and object entities is `SelectionMapErrorKind::MixedPointerTarget(SelectableKey)`, "`P.f` points to both scalar and object entities." The `entity_named` lookup error maps to `SelectionMapErrorKind::Lookup` as in `server_contribution`.

## Reader generation and types

`selection_field` (`crates/isograph_compiler/src/artifact_types.rs:315-470`) builds one `ReaderSelection` per selection and the data field's type, by case:

- Server field, scalar target, no set: `Source`, `Bare(Entity(scalar))`, type `ArtifactInnerType::Scalar(kind)` (unchanged).
- Server field, scalar target, set: `Source`, `Selections { position: Scalar, .. }` from the set on the scalar entity, type `ArtifactInnerType::Object(nested)`.
- Server field, object target, no set: `Source`, `Bare(Entity(target))`, type `ArtifactInnerType::EmptyObject`.
- Server field, object target, set: `Source`, `Selections { position: Record, .. }`, type `Object(nested)` (unchanged).
- Refinement (`SelectableShape::Refinement`), set or none: `Derived { field: (abstract entity, asT) }`, target `Selections { position: Record, .. }` or `Bare(Entity(T))`; type `Object(nested)` or `EmptyObject`, nullable. `ArgumentsOnRefinement` stays.
- Client field without `to`, no set: `Derived`, `Bare(FieldOutput(field))`, type `ClientFieldOutput(field)` (unchanged). With a set: `GenerationError::SelectionSetOnClientField` (unchanged).
- Client pointer to a scalar entity: `Derived`, target as for a server field of that scalar, with the pointer's `Wrapper` for list depth and nullability. Type: the scalar's type, or `Object(nested)`.
- Client pointer to an object entity: `SelectionRead::NotRead` (unchanged; open decision 5). `GenerationError::MissingSelectionSet` stays for it alone, renamed `PointerMissingSelectionSet`, "`P.f` points to an entity with fields, so it needs a selection set."
- `__self` on an object entity: `Derived { field: (E, __self) }`, target `Selections { position: Record, .. }` or `Bare(Entity(E))`; type `Object(nested)` or `EmptyObject`, not nullable.
- `__self` on a scalar entity: `Derived { field: (S, __self) }`, `Bare(Entity(S))` or `Selections { position: Scalar, .. }`; type the scalar's type or `Object(nested)`.

Each `Derived` selection adds its field to `FieldTypes::selected`, so its reader artifact is generated (`__self` and refinements included, Generated derived fields).

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub enum ArtifactInnerType {
    Scalar(ScalarKind),
    Object(ArtifactObjectType),
    /// What a client field's resolver returns: its `output_type` artifact.
    ClientFieldOutput(EntityNameAndSelectableName),
    /// What an object entity's default reader returns.
    EmptyObject,
}
```

`EmptyObject` prints `Readonly<Record<PropertyKey, never>>`, the type `Pet/default_reader.ts` returns. `Record<PropertyKey, never>` is what an empty `parameters` prints today (`demos/pet-demo/src/__isograph/Owner/OwnerBadge/param_type.ts`).

`GenerationError` loses `SelectionSetOnScalar` (`artifact_types.rs:150`); `MissingSelectionSet` (`:154`) becomes `PointerMissingSelectionSet`; it gains `MixedListDepth(SelectableKey)` and `MixedPointerTarget(SelectableKey)`. `ReaderTarget` (the private enum at `:196-204`) goes: the target is the `ReaderTarget` of `reader_ast.rs`.

### The field's parent kind

A field's resolver parameter and reader artifact type depend on whether its parent is an object entity or a scalar entity.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldArtifacts<TLiteralContext> {
    pub field: EntityNameAndSelectableName,
    pub kind: FieldArtifactKind,
    pub resolver: FieldResolver<TLiteralContext>,
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    pub output_type: Generation,
    pub resolver_reader: Option<ReaderSelectionSet>,
    pub parent: FieldParent,
}

/// What the field's resolver receives as `self`.
pub enum FieldParent {
    /// The parent is an object entity: `self` is a link to one of these concrete entities.
    Record(Vec<EntityName>),
    /// The parent is a scalar entity: `self` is its value.
    Scalar(ScalarKind),
}

/// Where the field's resolver is defined.
pub enum FieldResolver<TLiteralContext> {
    /// A `field` declaration's export.
    Declared(FieldDeclarationSource<TLiteralContext>),
    /// A resolver the compiler writes (Generated derived fields).
    Generated(GeneratedResolver),
}

pub enum GeneratedResolver {
    /// `__self`: returns `self`.
    SelfValue,
    /// A refinement to the concrete entity: `self` when its entity is that one, else `null`.
    Refinement(EntityName),
}
```

Before: `FieldArtifacts` has `resolver: FieldDeclarationSource<TLiteralContext>` and `concrete_entities: Vec<EntityName>` (`artifact_types.rs:700-712`); `concrete_entities` moves into `FieldParent::Record`.

### Generated derived fields

`__self` and each refinement are fields whose reader artifacts the compiler writes, beside the field's other artifacts: `E/__self/` and `Node/asPet/`. Each has an empty `resolver_reader`, a param type, an output type (`ReturnType<typeof resolver>` of `./resolver`), and `resolver.ts`, which `resolver_reader.ts` imports as it imports a declared resolver from its source file. The runtime has no case for either.

## Artifact generation

### Reader AST printing

```rust
// from crates/typescript_artifacts/src/reader_artifact.rs
fn selection_value(selection: &ReaderSelection) -> ArtifactValue {
    ArtifactValue::Object(vec![
        entry("alias", string(selection.alias)),
        entry("selectable", selectable_value(selection.selectable.reference())),
        entry("listDepth", ArtifactValue::Number(f64::from(selection.list_depth.0))),
        entry("target", target_value(selection.target.reference())),
    ])
}

fn selectable_value(selectable: &ReaderSelectable) -> ArtifactValue {
    match selectable {
        ReaderSelectable::Source(source) => ArtifactValue::Object(vec![
            kind("Source"),
            entry("selectableName", string(source.selectable)),
            entry("arguments", arguments_value(source.arguments.reference())),
        ]),
        ReaderSelectable::Derived(derived) => ArtifactValue::Object(vec![
            kind("Derived"),
            entry(
                "readerArtifact",
                ArtifactValue::Import(ArtifactImport(field_artifact_path(derived.field, RESOLVER_READER))),
            ),
            entry("arguments", arguments_value(derived.arguments.reference())),
        ]),
    }
}

fn target_value(target: &ReaderTarget) -> ArtifactValue {
    match target {
        ReaderTarget::Selections(selections) => ArtifactValue::Object(vec![
            kind("Selections"),
            entry("position", string(position_name(selections.position))),
            entry("selections", selection_set_value(selections.selections.reference())),
        ]),
        ReaderTarget::Bare(reader) => ArtifactValue::Object(vec![
            kind("Bare"),
            entry("read", ArtifactValue::Import(ArtifactImport(default_reader_path(reader.dereference())))),
        ]),
    }
}

/// `"Record"` or `"Scalar"`.
fn position_name(position: PositionKind) -> &'static str;

/// `E/default_reader.ts` or `P/f/default_reader.ts`.
pub(crate) fn default_reader_path(reader: DefaultReader) -> ArtifactPath;
```

Before: `selection_value` (`reader_artifact.rs:102-148`) matched four kinds. Every import is an `ArtifactValue::Import`, so `imports_of` (`crates/typescript_artifacts/src/imports.rs:9`) collects the default readers with the reader artifacts and the module hoists each once, named by `artifact_import_identifier` (`imports.rs:44`): `String/default_reader.ts` is `String__default_reader`. `reader_artifact_file` writes `selectableName: "<name>",` first (pico-runtime.md, Stored resolver results).

The reader artifact's type argument is `entity_name_type` of `FieldParent::Record`'s entities, or the scalar entity's name as a string literal for `FieldParent::Scalar`.

### Per-type and generated modules

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct ProjectArtifacts<TLiteralContext> {
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    pub entities: Vec<EntityName>,
    pub entrypoints: Vec<EntrypointArtifacts>,
    pub link_types: Vec<LinkType>,
    /// Every default reader some reader AST's `Bare` target names, sorted.
    pub default_readers: Vec<DefaultReaderArtifact>,
}

pub struct DefaultReaderArtifact {
    pub reader: DefaultReader,
    pub kind: DefaultReaderKind,
}

pub enum DefaultReaderKind {
    /// A scalar entity: the value, at the scalar's type.
    Identity(ScalarKind),
    /// An object entity: the one empty object.
    EmptyObject,
    /// A client field without `to`: the value, at the field's output type.
    FieldOutputIdentity(EntityNameAndSelectableName),
}
```

`DefaultReaderKind` is decided from the entity's `EntityKind` (`crates/isograph_compiler/src/schema.rs:36-41`). Built-in scalars are `ScalarKind::Known`, which the protocol assigns (`crates/graphql_network_protocol/src/lib.rs:58-61`, through `NetworkProtocol::entity_kind`); the core crate reads `EntityKind` and does not depend on `graphql_network_protocol`. A custom scalar is `ScalarKind::Custom`; its default reader is open decision 1.

`typescript_artifacts` prints these modules. `generate_artifacts` (`crates/typescript_artifacts/src/lib.rs:63`) gains one module per `default_readers` entry after `link_types`, and per generated field `resolver.ts`. The fixed function bodies are one `TypeScriptItem` variant:

```rust
// from crates/typescript_artifacts/src/module.rs
pub(crate) enum TypeScriptItem {
    // .. the existing variants, and:
    /// `const <binding> = <function>;`
    DeclaredGeneratedFunction(GeneratedFunction),
}

/// A function whose body the compiler writes.
pub(crate) enum GeneratedFunction {
    /// `const defaultReader = (value: T): T => value;`
    Identity(TypeScriptType),
    /// `const empty: Readonly<Record<PropertyKey, never>> = Object.freeze({});` then
    /// `const defaultReader = (): Readonly<Record<PropertyKey, never>> => empty;`
    EmptyObject,
    /// `export const resolver = ({ self }: P) => self;`
    SelfResolver(String),
    /// `export const resolver = ({ self }: P) => (self.entityName === "E" ? self : null);`
    RefinementResolver(RefinementResolver),
}

pub(crate) struct RefinementResolver {
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
import String__toUppercase__default_reader from './../../String/toUppercase/default_reader';
import String__toUppercase__resolver_reader from './../../String/toUppercase/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Query__PetDetailPage__param,
  Query__PetDetailPage__output_type,
  'Query'
> => ({
  selectableName: 'PetDetailPage',
  readerAst: [
    {
      alias: 'pet',
      selectable: {
        kind: 'Source',
        selectableName: 'pet',
        arguments: [['id', { kind: 'Variable', name: 'id' }]],
      },
      listDepth: 0,
      target: {
        kind: 'Selections',
        position: 'Record',
        selections: [
          {
            alias: 'n1',
            selectable: { kind: 'Source', selectableName: 'name', arguments: null },
            listDepth: 0,
            target: { kind: 'Bare', read: String__default_reader },
          },
          {
            alias: 'n2',
            selectable: { kind: 'Source', selectableName: 'name', arguments: null },
            listDepth: 0,
            target: {
              kind: 'Selections',
              position: 'Scalar',
              selections: [
                {
                  alias: 'toUppercase',
                  selectable: {
                    kind: 'Derived',
                    readerArtifact: String__toUppercase__resolver_reader,
                    arguments: null,
                  },
                  listDepth: 0,
                  target: { kind: 'Bare', read: String__toUppercase__default_reader },
                },
              ],
            },
          },
          {
            alias: 'bestFriend',
            selectable: { kind: 'Source', selectableName: 'bestFriend', arguments: null },
            listDepth: 0,
            target: {
              kind: 'Selections',
              position: 'Record',
              selections: [
                {
                  alias: 'name',
                  selectable: { kind: 'Source', selectableName: 'name', arguments: null },
                  listDepth: 0,
                  target: { kind: 'Bare', read: String__default_reader },
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
  readonly self: Query____link__output_type;
};
```

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/toUppercase/resolver_reader.ts
import type { ReaderArtifact } from '@isograph/runtime';
import type { String__toUppercase__output_type } from './output_type';
import type { String__toUppercase__param } from './param_type';
import { toUppercase as resolver } from './../../../ToUppercase';
import String____self__resolver_reader from './../__self/resolver_reader';
import String__default_reader from './../default_reader';

const readerArtifact = (): ReaderArtifact<
  String__toUppercase__param,
  String__toUppercase__output_type,
  'String'
> => ({
  selectableName: 'toUppercase',
  readerAst: [
    {
      alias: '__self',
      selectable: { kind: 'Derived', readerArtifact: String____self__resolver_reader, arguments: null },
      listDepth: 0,
      target: { kind: 'Bare', read: String__default_reader },
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
```

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/toUppercase/param_type.ts
export type String__toUppercase__param = {
  readonly data: {
    readonly __self: string;
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly self: string;
};
```

```ts
// generated by crates/typescript_artifacts/src/reader_artifact.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/__self/resolver_reader.ts
import type { ReaderArtifact } from '@isograph/runtime';
import type { String____self__output_type } from './output_type';
import type { String____self__param } from './param_type';
import { resolver } from './resolver';

const readerArtifact = (): ReaderArtifact<String____self__param, String____self__output_type, 'String'> => ({
  selectableName: '__self',
  readerAst: [],
  resolver: resolver,
});
export default readerArtifact;
```

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/__self/resolver.ts
import type { String____self__param } from './param_type';

export const resolver = ({ self }: String____self__param) => self;
```

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/default_reader.ts
const defaultReader = (value: string): string => value;
export default defaultReader;
```

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/toUppercase/default_reader.ts
import type { String__toUppercase__output_type } from './output_type';

const defaultReader = (value: String__toUppercase__output_type): String__toUppercase__output_type => value;
export default defaultReader;
```

An object entity's default reader, emitted when a reader selects `Pet` bare (`bestFriend` with no selection set):

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Pet/default_reader.ts
const empty: Readonly<Record<PropertyKey, never>> = Object.freeze({});
const defaultReader = (): Readonly<Record<PropertyKey, never>> => empty;
export default defaultReader;
```

A refinement, emitted when a reader selects `asPet` on `Node`:

```ts
// generated by crates/typescript_artifacts/src/lib.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/Node/asPet/resolver.ts
import type { Node__asPet__param } from './param_type';

export const resolver = ({ self }: Node__asPet__param) => (self.entityName === 'Pet' ? self : null);
```

`Node__asPet__param` is `{ readonly data: Record<PropertyKey, never>; readonly parameters: Record<PropertyKey, never>; readonly self: Node____link__output_type }`.

### Type generation

- `param_type_file` (`crates/typescript_artifacts/src/generate_types.rs`) writes `readonly self: E____link__output_type;` for `FieldParent::Record` where it wrote `readonly link: ..` (pico-runtime.md, open decision 24), and `readonly self: <scalar type>;` for `FieldParent::Scalar`, printed by `scalar_type` (`generate_types.rs:440-449`). The test `the_param_type_ends_with_the_parent_entitys_link` (`reader_artifact.rs:289`) asserts `self`.
- `inner_type` prints `EmptyObject` as `Readonly<Record<PropertyKey, never>>`.
- `iso_overloads.rs` types a pointer to a scalar entity's resolver as returning the scalar's type, where a pointer to an object entity returns its link type (`crates/typescript_artifacts/src/lib.rs:780-786` asserts the object case).

## Changes

Each is one commit, in order; each is independently shippable. They are pico-runtime.md's core changes 3 and 4.

1. The single selection shape, on today's language. The compiler emits `ReaderSelection` with `Source` selectables and `Selections`/`Bare` targets for every selection v1 accepts; refinements become generated derived fields (`Node/asPet/`); `default_reader.ts` per scalar entity a reader selects; `listDepth`; `self` in the param type. The runtime walk (pico-runtime.md, The walk) reads the new AST through v1's hooks. Validation is unchanged. Observable: every artifact under `demos/pet-demo/src/__isograph` and the runtime fixtures is regenerated in the new shape; `fetchAndRead.ts` prints the same output; the runtime's read tests pass with `link` renamed `self` in resolvers that read it.
2. Linked-only selections. Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, client fields on scalar parents, `to <scalar>`, and `__self`; `SelfValue`; `EmptyObject`; `FieldParent::Scalar`; the object default reader; the field-output default reader; the walk's scalar positions. Observable: the example compiles to the artifacts above; `read` of `PetDetailPage` on the fixture store returns `{ pet: { n1: 'Ricky', n2: { toUppercase: 'RICKY' }, bestFriend: { name: 'Mango' } } }`; `bestFriend` bare returns `{}` and `null`; the query text of a bare `bestFriend` is `bestFriend { __typename id }`; a declaration `field Pet.x to [Pet] | Pet` reports `MixedListDepth`.

## Open decisions

### 1. A custom scalar's default reader

- (a) The config supplies it: each `custom_scalar_types` entry gains an optional default reader import beside its type import (`custom_scalar_imports`, `crates/typescript_artifacts/src/generate_types.rs:473-490`). Cost: a config field and its validation.
- (b) The compiler generates the identity at the scalar's type (`scalar_type`, `unknown` without a config entry). Held. Reason: it is upstream's behavior; upstream has no default reader and reads every scalar whole (`case 'Scalar'`, `../isograph/libs/isograph-react/src/core/read.ts`), and so does i2 today. (a) can be added later without changing (b)'s files' paths.

### 2. The default reader of a client field without `to`

- (a) `P/f/default_reader.ts`, the identity at `P__f__output_type`. Held. Reason: the field's output is the only type the compiler knows for it, and a per-field module is the same shape as a per-entity one. Cost: one module per such field selected bare.
- (b) One shared identity module. Cost: a module that belongs to no type, against the per-type direction.

### 3. Which entities and fields get generated modules

- (a) Only the default readers some `Bare` target names, and only the `__self` and refinement fields some reader selects. Held. Reason: AGENTS.md, nothing we do not use.
- (b) Every entity, as `link_types` covers every entity a field is declared on (`crates/isograph_compiler/src/artifact_types.rs:663-664`). Cost: unread modules.

### 4. The object default reader's value

Bare outputs are compared by `===` in `selectionSetDataAreEqual` (pico-runtime.md, Equality).

- (a) One frozen empty object per module, returned on every call. Held. Reason: a recomputation that reaches the same bare object selection compares equal by `===`, so the parent's cutoff works with no structural rule for `Bare`.
- (b) A new `{}` per call, and `selectionSetDataAreEqual` compares `Bare` outputs structurally. Cost: a deep comparison of values a default reader returns, which for a later user-defined entity's reader can be any object.
- (c) A new `{}` per call, compared by `===`. Cost: every recomputation of a reader with a bare object selection marks its dependents.

### 5. Client pointers to object entities

- (a) Not read in v0, as today (`SelectionRead::NotRead`, `artifact_types.rs:389`; empty query contribution, `merged_selection.rs:1131`). Held. Reason: the query does not fetch the target's selections, so a read pointer would return `MissingData` unless another selection fetched them; fetching through the pointer is pico-runtime.md, Follow-ups 2 and 3.
- (b) Read in v0: `Derived` with a `Selections { position: Record }` target. Cost: `MissingData` for data no other selection fetches.

A pointer to a scalar entity is read in v0 under either option: its value is the resolver's output, with nothing to fetch.

### 6. How a client field states a scalar output entity

- (a) `to <scalar>`: `field Pet.fullName to String { .. }`. Held. Reason: `to` already names a derived selectable's target entity, and the target entity decides the default reader and whether a selection set is read at a scalar position.
- (b) Inferred from the resolver's return type. Not available: the compiler does not type-check TypeScript.

### 7. How a client field on a scalar entity reads the scalar

- (a) Both: `__self` is a selectable (`field String.toUppercase { __self }` reads `data.__self`), and the param carries `self`. Held. Reason: `__self` is a derived field whose generated resolver returns `self`, so `self` is in every param; the selectable form lets `__self { toUppercase }` and `__self` with an alias work like any selection.
- (b) `self` only; `__self` is not a selectable. Cost: `__self { .. }` at a record and at a scalar is not expressible.
- (c) `__self` only; type generation omits `self` from declared fields' params while the runtime passes it. Cost: a param type that hides a field the runtime passes.
