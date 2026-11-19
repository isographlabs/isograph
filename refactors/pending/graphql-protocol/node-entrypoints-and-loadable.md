# GraphQL protocol: entrypoints on Node entities, and `@loadable` validation

This doc specifies which entities the network protocol can fetch by themselves, and two uses of that answer. For GraphQL an entity can be fetched by itself when it is a root operation type, or an object type that implements `Node` while the query root has `node(id: ID!)`. The first use is `@loadable`: the iso parser accepts `@loadable` on a selection, and validation checks that the selection's parent can be fetched by itself. That check is the only effect of `@loadable`; the selection otherwise behaves exactly as it does without the directive. The second use is an `entrypoint` on such an entity. In the first version, an entrypoint on a root operation type generates as before, and an entrypoint on a Node entity passes the fetchability rules but is reported as not supported yet, at the entrypoint's parent type, and generates no artifacts. Later, the operation of an entrypoint on a Node entity reaches the entity's record through `node(id:)`, and its artifacts are generated (Later: entrypoints on Node entities).

The core has no concept of a root operation type and no concept of `node`. The protocol tells the core, per fetchable entity, whether the operation reaches the record the entrypoint's selections are on directly or through a wrapper the protocol selects (the entrypoint wrapper). Later, the core completes the selections under that record, builds the normalization AST with the wrapper's path, and hands both to the protocol to print.

The first version amends `entrypoints.md` and `refactors/pending/iso-directives-unexpected.md`. Later amends `refactors/pending/runtime/network-and-subscriptions.md` (not landed) and the landed `libs/isograph-runtime` code from `refactors/pending/runtime/store.md` (Docs this doc amends).

Requires (landed): `query-generation.md` changes 1 to 10, `normalization-ast.md` changes 1 to 4, `entrypoints.md`, `refactors/pending/iso-directives-unexpected.md` changes 1 to 4, `refactors/pending/runtime/store.md` changes 1 to 7.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `072af275`). i2 references are at `a2ad2679`.

## End-user experience

```graphql
# from demos/pet-demo/schema/schema.graphql (excerpt)
type Query {
  pet(id: ID!): Pet
  node(id: ID!): Node
}
interface Node {
  id: ID!
}
type Pet implements Node & Namable {
  id: ID!
  tagline: String!
  stats: PetStats
}
type PetStats {
  weight: Int
}
```

```ts
// from crates/graphql_network_protocol/src/tests/query.rs (the fixture literals)
iso(`entrypoint Query.HomeRoute`); // ok: Query is the query root
iso(`field Pet.PetTagline { tagline }`);
iso(`entrypoint Pet.PetTagline`); // error on `Pet`: not supported yet
iso(`entrypoint PetStats.Card`); // error on `PetStats`
iso(`entrypoint Node.Card`); // error on `Node`
iso(`entrypoint Owner.OwnerBadge`); // error on `Owner` (declared by `entity Owner`)

iso(`
  field Query.PetDetailRoute($id: ID!) {
    pet(id: $id) {
      PetCheckinsCard @loadable
    }
  }
`); // ok: Pet implements Node; PetCheckinsCard is selected exactly as without `@loadable`
iso(`field Query.HomeFeed { HomeRoute @loadable }`); // ok: Query is the query root
iso(`
  field Pet.PetStatsCard {
    stats {
      StatsBadge @loadable
    }
  }
`); // error on `@loadable`: PetStats cannot be fetched by itself
iso(`field Pet.X { tagline @loadable }`); // error on `@loadable`: `tagline` is a schema field
iso(`field Pet.Y { PetCheckinsCard @loadable(lazyLoadArtifact: true) }`); // parse error on `(lazyLoadArtifact: true)`
iso(`field Pet.Z @loadable { name }`); // parse error `unexpected directive`, unchanged
```

The diagnostics, each an LSP diagnostic published like every other validation or parse error:

- `Pet` in `entrypoint Pet.PetTagline`: `` `Pet` is fetched by its id; entrypoints on entities fetched by id are not supported yet. ``
- `PetStats`: `` `PetStats` is not a root operation type and does not implement `Node`, so it cannot be fetched by itself. ``
- `Node`: `` `Node` is not an object type, so it cannot be fetched by itself. ``
- `Owner`: `` `Owner` is declared in source files, so the network protocol cannot fetch it. ``
- `@loadable` under `stats`: `` `@loadable` on `PetStats.StatsBadge` needs its parent to be fetchable by itself. `PetStats` is not a root operation type and does not implement `Node`, so it cannot be fetched by itself. ``
- `@loadable` on `tagline`: `` `Pet.tagline` comes from the schema; `@loadable` selects a `field` declaration. ``
- `(lazyLoadArtifact: true)`: `` `@loadable` takes no arguments. ``

`PetDetailRoute`'s query selects `PetCheckinsCard`'s selections under `pet`, its reader reads `PetCheckinsCard`, and its data type has the `PetCheckinsCard` key, all as without `@loadable`.

### Entrypoints on Node entities: a reported error, not a panic

The user's direction was that the compiler "might panic on entrypoints on items that impl Node". This doc holds a located error instead. The parser and the compiler never panic on user input (AGENTS.md, Panic and UnexpectedError), and the compiler runs inside the daemon, where a panic ends the LSP for every connected client. An entrypoint on a Node entity is user input, so it is a representable state: `EntrypointNotFetchableError::WrappedNotSupported`.

The error is reported through the entrypoint's existing fetchability check, `entrypoint_fetch_plan`, which both validation and generation call. Validation (`entrypoint_declaration_error`) publishes it as a diagnostic at the entrypoint's `parent_type`. Generation (`query_for_entrypoint`) returns it as `EntrypointQueryError::NotFetchable`, so `project_artifacts` fails and no artifacts are written. Generation alone cannot show it: a generation error today is a `tracing::warn` in `generate_artifacts` (`crates/isograph_cli/src/compile.rs`), not an LSP diagnostic and not located; the only located diagnostics are the parse and validation errors that `compiler_errors_in_file` returns. The fetchability rules pass for `Pet` (it implements `Node` and `Query.node(id: ID!)` exists), which is why a `@loadable` selection on a `Pet` field passes.

The message says "fetched by its id" instead of "Node": the core has no concept of `Node`, and for GraphQL the entities fetched by id are exactly the Node entities.

## Upstream

`@loadable`:

- Parsing. `ScalarSelectionDirectiveSet` has `Loadable`, `Updatable`, `None` (`isograph/crates/isograph_lang_types/src/selection_directive_set.rs:9-13`); `ObjectSelectionDirectiveSet` has `Updatable`, `None` (`:17-20`). So upstream parses `@loadable` only on a selection without a selection set: a client field selection, never a linked field or a client pointer selection. The one parameter is `lazyLoadArtifact: bool`, default `false`, unknown parameters denied (`:39-44`).
- Validation. `@loadable` on a server scalar field is an error at the selection's name: "`{}.{}` is a server scalar field, and @loadable is not supported on selections of server scalar fields." (`isograph/crates/isograph_schema/src/validate_selection_sets.rs:177-186`). On an exposed field and on `__link` it is an error (`:196-216`). A user-written client field passes (`:199`).
- The parent. Upstream does not check that the parent can be refetched. `get_refetch_stategy` (`isograph/crates/isograph_schema/src/process_client_field_declaration.rs:215-247`) returns `RefetchFromRoot` for a root operation type (`:219-231`), `UseRefetchField` with the selection set `{ id }` for a type with an `id` field (`:233-245`), and `None` otherwise; `None` reaches `.expect("Expected refetch strategy. This is indicative of a bug in Isograph.")` (`isograph/crates/artifact_content/src/generate_artifacts.rs:281-283`, `reader_ast.rs:622-624`, `refetch_reader_artifact.rs:49-51`), a panic.
- Arguments. A loadable selection may omit a required argument of the field (`isograph/crates/isograph_schema/src/validate_use_of_arguments.rs:161-165`); the omitted ones become runtime variables (`isograph/crates/isograph_schema/src/variable_context.rs:47-53`).
- The parent's query does not inline the loadable field's selections; the parent's reader produces a `LoadablySelectedField` whose refetch reader reads `{ id }` from the parent (`isograph/demos/pet-demo/src/components/__isograph/BlogItem/BlogItemDisplay/resolver_reader.ts:31-52`).

Fetching a non-root entity:

- `graphql_network_protocol_associated_data_for_object` (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:177-213`) gives the three root operation types `LeaveAsIs` and every other object with a field named `id` `Node { query_root }` (`:200-207`, `has_id_field.note_todo("Check node implementation instead")`). It checks neither that the type implements `Node` nor that the query root has a `node` field.
- `wrap_merged_selection_map` (`isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:305-357`) wraps the entity's merged selection map in `InlineFragment(entity)` inside `LinkedField { node, arguments [id: $id], Abstract }` and returns the query root as the operation's root entity (`:334-354`). An entity that is not fetchable is "Type `X` is not fetchable." (`:326`).
- The query text: `query BlogItemMoreDetail($id: ID!) { node____id___v_id: node(id: $id) { ... on BlogItem { __typename id moreContent } } }` (`isograph/demos/pet-demo/src/components/__isograph/BlogItem/BlogItemMoreDetail/query_text.ts:1-3`). `__typename` is selected inside the fragment (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1510`), not at the `node` level.
- The normalization AST is rooted at the query root: a top-level `Linked` `node` with `concreteType: null` holding an `InlineFragment` on the entity (`BlogItemMoreDetail/normalization_ast.ts:6-19`); the entrypoint's `concreteType` is `"Query"` (`BlogItemMoreDetail/entrypoint.ts:25`). The runtime normalizes from `{ __link: ROOT_ID, __typename: concreteType }` (`isograph/libs/isograph-react/src/core/makeNetworkRequest.ts:233`), so the store gets `Query.__ROOT.node(id:)` as a link.
- The `$id` variable is added only on the loadable path (`generate_artifacts.rs:258-293`). A user entrypoint on a non-root type passes only the field's own arguments as variables (`isograph/crates/artifact_content/src/entrypoint_artifact.rs:99`); its `$id` has no definition, which reaches `panic!("Expected to find a variable defined at the root with name ..")` (`:341-356`) unless the field declares `$id`. No upstream demo has a non-root entrypoint.
- Collisions. Upstream does not separate the two sets. `get_used_variable_definitions` takes the first definition named `id` (`entrypoint_artifact.rs:347-349`), the refetch path skips a user `id` with a comment "HACK" (`isograph/crates/artifact_content/src/imperatively_loaded_fields.rs:189`), and at runtime `variables.id = readOutData.id` overwrites the user's `id` (`isograph/libs/isograph-react/src/core/read.ts:317-324`).
- Types. `id` appears in no generated parameters type: `BlogItemMoreDetail/param_type.ts:6` has `parameters: Record<PropertyKey, never>`, and `Pet/PetCheckinsCard/parameters_type.ts:1-4` has only `skip` and `limit`.

i2's removed Node fetch (`dddbc8ef3`, "Remove Node-fetched entrypoints") was stricter than upstream: the entity had to be `Node` or implement it, and the query root had to have a `node` field returning one value of the entity or of an interface or union containing it (`graphql_fetch_plan` and `node_fetch`, `crates/graphql_network_protocol/src/fetch_plan.rs:113-219` at `dddbc8ef3^`). This doc restores that check, without `GraphQLFetchArgument` and `FetchRefinement` (The GraphQL fetch plan).

## `@loadable` in the parser

`iso-directives-unexpected.md` said: "The first directive that is valid in only some positions splits the set into one enum per position." `@loadable` is that directive.

Before:

```rust
// from crates/isograph_parser/src/directives.rs
/// A directive name the iso parser accepts. v1 accepts none.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
enum SupportedDirectiveName {}
```

After:

```rust
// from crates/isograph_parser/src/directives.rs
/// A directive name the iso parser accepts on a selection.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
enum SelectionDirectiveName {
    #[strum(serialize = "loadable")]
    Loadable,
}

/// A directive name the iso parser accepts on an `entrypoint` or `field` declaration. v1
/// accepts none.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
enum DeclarationDirectiveName {}

/// Where a directive list sits, which decides the names it accepts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DirectivePosition {
    Declaration,
    Selection,
}
```

`consume_directives(cursor)` becomes `consume_directives(cursor, position)`, and passes `position` to `parse_directive_after_at`. Callers: `consume_selection` in `selections.rs:95` passes `Selection`; the entrypoint and field declaration parses in `parse_iso_literal.rs:263` and `:279` pass `Declaration`.

```rust
// from crates/isograph_parser/src/directives.rs
fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
    position: DirectivePosition,
) -> Result<WithSpan<IsographFieldDirective>, WithSpan<AstError>> {
    let name = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::DirectiveName,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let arguments = consume_argument_list(cursor);
    let end = arguments
        .as_ref()
        .map(|list| list.location.end)
        .unwrap_or(name.location.end);
    let location = Span::new(at.start, end);
    match position {
        DirectivePosition::Declaration => match name.text().parse::<DeclarationDirectiveName>() {
            Ok(supported) => match supported {},
            Err(strum::ParseError::VariantNotFound) => {
                cursor.report_error(AstError::UnexpectedDirective.with_span(location));
            }
        },
        DirectivePosition::Selection => match name.text().parse::<SelectionDirectiveName>() {
            Ok(SelectionDirectiveName::Loadable) => {
                if let Some(arguments) = arguments.as_ref() {
                    cursor.report_error(AstError::LoadableArguments.with_span(arguments.location));
                }
            }
            Err(strum::ParseError::VariantNotFound) => {
                cursor.report_error(AstError::UnexpectedDirective.with_span(location));
            }
        },
    }
    IsographFieldDirective {
        name: name.interned().map(IsographDirectiveNameWrapper),
        arguments,
    }
    .with_span(location)
    .wrap_ok()
}
```

Delta from the landed function: the `position` parameter and the two-armed `match`. `@loadable()` (an empty argument list) is `LoadableArguments` too: the argument list is present.

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum AstError {
    // .. unchanged variants
    #[error("unexpected directive")]
    UnexpectedDirective,
    #[error("`@loadable` takes no arguments.")]
    LoadableArguments,
}
```

The reading side. Validation reads the directive through one function, so no pass compares a directive name to a string:

```rust
// from crates/isograph_parser/src/selections.rs
/// Whether a selection carries `@loadable`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Loadability {
    /// Without `@loadable`.
    Eager,
    /// `@loadable`, at this literal-relative span.
    Loadable(Span),
}

impl Selection {
    /// `Loadable` at the first `@loadable` in this selection's directive list.
    pub fn loadability(&self) -> Loadability {
        self.directive_set
            .iter()
            .flat_map(|list| list.item.0.iter())
            .find(|directive| {
                matches!(
                    directive.item.name.item.0.lookup().parse::<SelectionDirectiveName>(),
                    Ok(SelectionDirectiveName::Loadable)
                )
            })
            .map_or(Loadability::Eager, |directive| {
                Loadability::Loadable(directive.location)
            })
    }
}
```

`SelectionDirectiveName` becomes `pub(crate)` so `selections.rs` reads it. `Loadability` is re-exported from `crates/isograph_parser/src/lib.rs`. `@loadable @loadable` is one `Loadable`; nothing reads the second.

Unchanged: the AST types, semantic tokens, resolve paths, the formatter. A literal with `@loadable` and no other error now has no parse error.

## `@loadable` validation

A `@loadable` selection is valid when:

1. Its selectable is a `field` declaration: `SelectableKind::ClientField` or `ClientPointer`. Upstream allows only client fields (it cannot parse `@loadable` on a selection with a selection set); i2 also allows client pointers, by user decision.
2. Its parent entity can be fetched by itself: `entity_fetch_plan(parent)` is `Ok`. This is the same question an entrypoint on the parent asks (Which entities can be fetched by themselves). For GraphQL that is a root operation type, or an object type implementing `Node` while the query root has `node(id: ID!)`.

Rule 2 allows a `@loadable` selection on a field of a root operation type, by user decision, as upstream refetches such a field from the root (`RefetchFromRoot`, `isograph/crates/isograph_schema/src/process_client_field_declaration.rs:219-231`). `@loadable` on a `Pet` field passes because `Pet` implements `Node`; `entrypoint_fetch_plan`'s `WrappedNotSupported` is an entrypoint error and is not part of rule 2.

```rust
// from crates/isograph_compiler/src/validation.rs
pub enum ValidationError<TNotFetchable: std::fmt::Display> {
    // .. unchanged variants
    #[error("{0}")]
    EntrypointNotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    MissingFieldArgument(MissingFieldArgument),
    #[error("{0}")]
    Loadable(LoadableError<TNotFetchable>),
}

/// Why a `@loadable` selection is invalid.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoadableError<TNotFetchable: std::fmt::Display> {
    #[error("`{0}` comes from the schema; `@loadable` selects a `field` declaration.")]
    ProtocolSelectable(SelectableKey),
    #[error(
        "`@loadable` on `{}` needs its parent to be fetchable by itself. {}",
        .0.selection, .0.reason
    )]
    ParentNotFetchable(LoadableParentNotFetchable<TNotFetchable>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadableParentNotFetchable<TNotFetchable: std::fmt::Display> {
    pub selection: SelectableKey,
    pub reason: EntityNotFetchableError<TNotFetchable>,
}
```

Both variants are located at the `@loadable` directive's span (`Loadability::Loadable(span)`). Upstream locates its server-field error at the selection's name; the directive is the part to remove, so i2 marks it.

`validate_selection` (`crates/isograph_compiler/src/validation.rs`), `Definitions::Unique(selectable)` arm, before and after:

```rust
// from crates/isograph_compiler/src/validation.rs
        Definitions::Unique(selectable) => {
            let missing = match selectable.kind {
                SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {
                    missing_field_arguments(db, client, key, selection)
                }
                SelectableKind::Server(_) => Vec::new(),
            };
```

```rust
// from crates/isograph_compiler/src/validation.rs
        Definitions::Unique(selectable) => {
            let loadable = match selection.loadability() {
                Loadability::Eager => None,
                Loadability::Loadable(span) => {
                    loadable_error(db, client, key, selectable.kind.reference())
                        .map(|error| ValidationError::Loadable(error).with_span(span))
                }
            };
            let missing = match selectable.kind {
                SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {
                    missing_field_arguments(db, client, key, selection)
                }
                SelectableKind::Server(_) => Vec::new(),
            };
```

The arm then chains `loadable` before `missing` and `nested`. A `@loadable` selection is checked for missing arguments like any selection, because its selections are part of the parent's query. The nested selection set of a `@loadable` client pointer is validated as before.

```rust
// from crates/isograph_compiler/src/validation.rs
/// Why the `@loadable` selection of `key`, whose selectable has `kind`, is invalid; `None` when
/// it is valid, or when the parent's lookup fails, which the declaration's own checks report.
fn loadable_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    key: SelectableKey,
    kind: &SelectableKind<<TProfile::NetworkProtocol as NetworkProtocol>::SelectableData>,
) -> Option<ProfileLoadableError<TProfile>> {
    match kind {
        SelectableKind::Server(_) => LoadableError::ProtocolSelectable(key).wrap_some(),
        SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {
            let parent = entity_named(db, client, key.entity).as_ref().ok()?;
            entity_fetch_plan(db, client, parent).err().map(|reason| {
                LoadableError::ParentNotFetchable(LoadableParentNotFetchable {
                    selection: key,
                    reason,
                })
            })
        }
    }
}

pub type ProfileLoadableError<TProfile> = LoadableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;
```

### `@loadable` has no other effect

By user decision, a `@loadable` selection behaves exactly as the same selection without the directive: query generation merges its field's selections into the parent's query, cycle detection (`inlined_in_selection`) handles it as any other selection, the parent's reader reads it, and the parent's data type has its key with the field's output type. `selection_contribution` (`crates/isograph_compiler/src/merged_selection.rs`) and `selection_field` (`crates/isograph_compiler/src/artifact_types.rs`) do not call `Selection::loadability`; validation is its only reader.

This restores i2's behavior before the directives change (`46577a1ea`, "Remove the @loadable path"), where the merge did not read directives, so a `@loadable` client field's selections were inlined into the parent's query and the parent's data type had the field's output type. Upstream instead inlines nothing of a loadable field and reads a `LoadablySelectedField`; that is the shape loadable artifacts take later (`@loadable` later), and landing it changes the type of every existing `@loadable` selection.

## Which entities can be fetched by themselves

The core asks the protocol one question for both entrypoints and `@loadable`: can this entity be fetched by itself. Prefactor: the entity half of `entrypoint_fetch_plan` becomes its own function.

Before:

```rust
// from crates/isograph_compiler/src/validation.rs
/// Why `entrypoint E.f` cannot be fetched.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointNotFetchableError<TNotFetchable: std::fmt::Display> {
    #[error(
        "`{0}` is declared in source files; only entities from the schema can parent an \
        entrypoint."
    )]
    SourceEntity(EntityName),
    #[error("`{}.{}` comes from the schema; an entrypoint names a `field` declaration.", .0.entity, .0.selectable)]
    ProtocolSelectable(SelectableKey),
    #[error("`{0}` has no selectables, so it cannot parent an entrypoint.")]
    NotComposite(EntityName),
    #[error("{0}")]
    Protocol(TNotFetchable),
}
```

After:

```rust
// from crates/isograph_compiler/src/validation.rs
/// Why `entrypoint E.f` cannot be fetched.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointNotFetchableError<TNotFetchable: std::fmt::Display> {
    #[error("`{}.{}` comes from the schema; an entrypoint names a `field` declaration.", .0.entity, .0.selectable)]
    ProtocolSelectable(SelectableKey),
    #[error("{0}")]
    Entity(EntityNotFetchableError<TNotFetchable>),
    #[error(
        "`{0}` is fetched by its id; entrypoints on entities fetched by id are not supported \
        yet."
    )]
    WrappedNotSupported(EntityName),
}

/// Why an entity cannot be fetched by itself.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntityNotFetchableError<TNotFetchable: std::fmt::Display> {
    #[error("`{0}` is declared in source files, so the network protocol cannot fetch it.")]
    SourceEntity(EntityName),
    #[error("`{0}` has no selectables, so it cannot be fetched.")]
    NotComposite(EntityName),
    #[error("{0}")]
    Protocol(TNotFetchable),
}
```

The messages no longer say "parent an entrypoint", because `@loadable` reports the same reasons. `WrappedNotSupported` is added by change 2 and removed Later.

```rust
// from crates/isograph_compiler/src/entrypoint.rs
pub type ProfileEntityNotFetchableError<TProfile> = EntityNotFetchableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

/// The protocol's plan for fetching `entity` by itself.
pub fn entity_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntityNotFetchableError<TProfile>> {
    let data = match entity.origin.reference() {
        EntityOrigin::Source => {
            return EntityNotFetchableError::SourceEntity(entity.name).wrap_err();
        }
        EntityOrigin::Protocol(data) => data,
    };
    match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(_) => {}
        EntityShape::Leaf | EntityShape::Input => {
            return EntityNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    }
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntityNotFetchableError::Protocol)
}

/// The protocol's plan for an entrypoint's operation, and the operation's root entities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointFetch<TFetchPlan> {
    pub plan: TFetchPlan,
    pub roots: RootEntities,
}

pub type ProfileEntrypointFetch<TProfile> = EntrypointFetch<ProfileFetchPlan<TProfile>>;

/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileEntrypointFetch<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    let plan = entity_fetch_plan(db, client, entity).map_err(EntrypointNotFetchableError::Entity)?;
    match TProfile::NetworkProtocol::entrypoint_wrapper(plan.reference()) {
        EntrypointWrapper::Unwrapped(roots) => EntrypointFetch { plan, roots }.wrap_ok(),
        EntrypointWrapper::Wrapped => {
            EntrypointNotFetchableError::WrappedNotSupported(entity.name).wrap_err()
        }
    }
}
```

Delta from the landed `entrypoint_fetch_plan`: the source check, the shape check, and the protocol call move into `entity_fetch_plan`; the selectable check now runs before the source check (a source entity has only source selectables, so no input reaches both); the wrapper is read here, so `query_for_entrypoint` destructures `EntrypointFetch { plan, roots }` where it called `root_entities(plan)`. Change 1 lands `entity_fetch_plan` and returns the plan from `entrypoint_fetch_plan` unchanged; change 2 adds `EntrypointFetch` and the wrapper match.

`entrypoint_declaration_error` locates `ProtocolSelectable` at `name`, and `Entity(_)` and `WrappedNotSupported(_)` at `parent_type`:

```rust
// from crates/isograph_compiler/src/validation.rs
    let location = |error: &EntrypointNotFetchableError<_>| match error {
        EntrypointNotFetchableError::ProtocolSelectable(_) => declaration.name.location,
        EntrypointNotFetchableError::Entity(_)
        | EntrypointNotFetchableError::WrappedNotSupported(_) => declaration.parent_type.location,
    };
```

### The entrypoint wrapper

The protocol's fetch plan says whether the operation's response holds the entity's record at its top level. The core reads that through one method, which replaces `root_entities`.

Before:

```rust
// from crates/isograph_compiler/src/host_language.rs
    /// The root entities of the operation `plan` describes.
    fn root_entities(plan: &Self::FetchPlan) -> RootEntities;
```

After:

```rust
// from crates/isograph_compiler/src/host_language.rs
    /// How the operation `plan` describes reaches the record the fetched entity's selections
    /// are on.
    fn entrypoint_wrapper(plan: &Self::FetchPlan) -> EntrypointWrapper;
```

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// How an operation reaches the record an entrypoint's selections are on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntrypointWrapper {
    /// The entity is `RootEntities::operation`: the response's top-level object is its one
    /// record, and the entrypoint's selections are the operation's top level.
    Unwrapped(RootEntities),
    /// The entrypoint's selections are under a path the protocol selects, on the record the
    /// caller names by its id.
    Wrapped,
}
```

`Wrapped` has no payload in the first version, because nothing reads the path: `entrypoint_fetch_plan` reports it as `WrappedNotSupported`. Later it becomes `Wrapped(WrappedEntrypoint)` (Later: entrypoints on Node entities).

The test protocols (`crates/isograph_compiler/src/test_profile.rs`, `crates/isograph_cli/src/test_profile.rs`, `crates/isograph_lsp/src/test_profile.rs`, `crates/isograph_extract_typescript/src/lib.rs`) rename `root_entities` to `entrypoint_wrapper` and wrap their `RootEntities` body in `EntrypointWrapper::Unwrapped(..)`.

## The GraphQL fetch plan

By user decision, a GraphQL entity that is not a root operation type is fetched by id when it is an object type implementing `Node` and the query root has `node(id: ID!)` returning one value of the entity or of an interface or union containing it. Upstream's rule, any object with an `id` field, is not used: it sends a query the server rejects when `node` is missing or broken, where this rule reports that at the entrypoint or `@loadable` that needs it.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How the GraphQL protocol fetches an entity by itself: the operation type, and how the
/// operation's response reaches the entity's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchPlan {
    pub operation_type: OperationType,
    pub wrapper: EntrypointWrapper,
}
```

Before: `GraphQLFetchPlan { operation_type, roots: RootEntities }`. `GraphQLNetworkProtocol::entrypoint_wrapper(plan)` is `plan.wrapper.clone()`. The printer reads `plan.operation_type` as before.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLNotFetchable {
    #[error("{0}")]
    RootOperationTypes(RootOperationTypesError),
    #[error("{0}")]
    Entity(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownEntity(EntityName),
    #[error("`{0}` is not an object type, so it cannot be fetched by itself.")]
    NotObject(EntityName),
    #[error(
        "`{0}` is not a root operation type and does not implement `Node`, so it cannot be \
        fetched by itself."
    )]
    NeitherRootNorNode(EntityName),
    #[error("{0}")]
    NodeField(NodeFieldError),
}

/// Why the query root's `node` field cannot fetch an entity. Each `EntityName` payload is the
/// query root.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NodeFieldError {
    #[error("{0}")]
    Selectable(SelectableLookupError),
    #[error("`{0}.node` is not defined.")]
    NotFound(EntityName),
    #[error("`{0}.node` returns a list; it returns one value.")]
    ReturnsList(EntityName),
    #[error(
        "`{}.node` returns `{}`, which is not `{}` or an interface or union containing it.",
        .0.query_root, .0.returns, .0.entity
    )]
    TypeMismatch(NodeFieldTypeMismatch),
    #[error("{0}")]
    Target(EntityLookupError),
    #[error("`{0}` is not defined in the schema.")]
    UnknownTarget(EntityName),
    #[error("`{0}.node` has no argument `id: ID!`.")]
    IdArgument(EntityName),
    #[error(
        "`{}.node` requires `{}`, which a fetch by id does not pass.",
        .0.query_root, .0.argument
    )]
    RequiredArgument(NodeFieldArgument),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldTypeMismatch {
    pub query_root: EntityName,
    pub returns: EntityName,
    pub entity: EntityName,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldArgument {
    pub query_root: EntityName,
    pub argument: InputValueName,
}
```

`NotRootOperationType` is renamed `NeitherRootNorNode`: an entity now fails only when it is neither. `NodeFieldError` and `NodeFieldTypeMismatch` are restored verbatim from `dddbc8ef3^` (`fetch_plan.rs:82-107`); `IdArgument`, `RequiredArgument`, and `NodeFieldArgument` are new. `GraphQLNotFetchable::NodeField` is restored; `NoFetchRoot` is not (it is `NeitherRootNorNode`), and `NotObjectOrInterface` is not (interfaces are `NotObject`).

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
const NODE_INTERFACE: &str = "Node";
const NODE_FIELD: &str = "node";
const ID_ARGUMENT: &str = "id";

/// How `entity` is fetched by itself, as `client` reads the schema.
#[memo]
pub fn graphql_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<GraphQLFetchPlan, GraphQLNotFetchable> {
    let Some(ProtocolEntity { data, .. }) = graphql_entity(db, client, entity)
        .clone()
        .map_err(GraphQLNotFetchable::Entity)?
    else {
        return GraphQLNotFetchable::UnknownEntity(entity).wrap_err();
    };
    let composite = match data {
        GraphQLEntityData::Object(composite) => composite,
        GraphQLEntityData::Scalar(_)
        | GraphQLEntityData::Enum
        | GraphQLEntityData::InputObject
        | GraphQLEntityData::Interface(_)
        | GraphQLEntityData::Union => return GraphQLNotFetchable::NotObject(entity).wrap_err(),
    };
    let roots = root_operation_types(db, client)
        .clone()
        .map_err(GraphQLNotFetchable::RootOperationTypes)?;
    if let Some(operation_type) = roots.operation_type_of(entity) {
        return GraphQLFetchPlan {
            operation_type,
            wrapper: EntrypointWrapper::Unwrapped(roots.root_entities(entity)),
        }
        .wrap_ok();
    }
    let node: EntityName = NODE_INTERFACE.intern().to();
    if !composite.interfaces.contains(node.reference()) {
        return GraphQLNotFetchable::NeitherRootNorNode(entity).wrap_err();
    }
    node_field(db, client, roots.query, entity, composite.interfaces.as_slice())
        .map_err(GraphQLNotFetchable::NodeField)?;
    GraphQLFetchPlan {
        operation_type: OperationType::Query,
        wrapper: EntrypointWrapper::Wrapped,
    }
    .wrap_ok()
}
```

Delta from the landed body: the object check moves before the root check, because every fetched entity must be an object (a record fetched by id is named by `(T, id)`, which needs `T` concrete); a non-root entity implementing `Node` gets `Wrapped` when `node_field` passes. Delta from `dddbc8ef3^`: interfaces are no longer fetchable (`entity == node` and `NotObjectOrInterface` are gone), and the plan is the core's `EntrypointWrapper` instead of `GraphQLFetchPlan::Node(GraphQLNodeFetch)`.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// Whether the `node` field of the query root `query_root` fetches the object type `entity`,
/// which implements `interfaces`, by id; the reason when it does not.
fn node_field<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    query_root: EntityName,
    entity: EntityName,
    interfaces: &[EntityName],
) -> Result<(), NodeFieldError> {
    let field = selectable_named(db, client, query_root, NODE_FIELD.intern().to())
        .clone()
        .map_err(NodeFieldError::Selectable)?
        .ok_or(NodeFieldError::NotFound(query_root))?;
    let returns = match field.item.item.type_.item.reference() {
        TypeAnnotation::Named(named) => named.0.item.0,
        TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
            NullableTypeAnnotation::Named(named) => named.0.item.0,
            NullableTypeAnnotation::List(_) => {
                return NodeFieldError::ReturnsList(query_root).wrap_err();
            }
        },
        TypeAnnotation::List(_) => return NodeFieldError::ReturnsList(query_root).wrap_err(),
    };
    let returns_data = graphql_entity(db, client, returns)
        .clone()
        .map_err(NodeFieldError::Target)?
        .ok_or(NodeFieldError::UnknownTarget(returns))?
        .data;
    if returns != entity && !contains(db, client, returns, returns_data.reference(), entity, interfaces) {
        return NodeFieldError::TypeMismatch(NodeFieldTypeMismatch {
            query_root,
            returns,
            entity,
        })
        .wrap_err();
    }
    let arguments = field.item.item.arguments.iter().map(|argument| argument.item.reference());
    let id: InputValueName = ID_ARGUMENT.intern().to();
    if !arguments.clone().any(|argument| {
        argument.name.item.0 == id && is_non_null_id(argument.type_.item.reference())
    }) {
        return NodeFieldError::IdArgument(query_root).wrap_err();
    }
    if let Some(required) = arguments.clone().find(|argument| {
        argument.name.item.0 != id
            && argument.default_value.is_none()
            && matches!(argument.type_.item.reference(), TypeAnnotation::NonNull(_))
    }) {
        return NodeFieldError::RequiredArgument(NodeFieldArgument {
            query_root,
            argument: required.name.item.0,
        })
        .wrap_err();
    }
    ().wrap_ok()
}

/// Whether `type_annotation` is `ID!`.
fn is_non_null_id(type_annotation: &TypeAnnotation) -> bool
```

`node_field` returns `Result<(), NodeFieldError>` because the first version needs only whether `node` fetches the entity; Later it becomes `node_wrapper`, which runs the same checks and returns the `WrappedEntrypoint` (The GraphQL node wrapper).

`is_non_null_id` returns a `bool` because it is a yes/no predicate with no domain names, the AGENTS.md fallback. It is `pub(crate)` in `crates/graphql_network_protocol/src/protocol_data.rs`, and `id_field` there calls it too (`Present` when the `id` field's type is `ID!`), so the two `ID!` checks are one function. Its body is `matches!(type_annotation, TypeAnnotation::NonNull(non_null) if matches!(non_null.0.item.reference(), NullableTypeAnnotation::Named(named) if named.0.item.0.lookup() == "ID"))`.

`contains` is restored from `dddbc8ef3^` (`fetch_plan.rs:222-248`) with one delta: it takes the entity's name and its interfaces instead of `&MergedTarget<GraphQLEntityData>`, because the plan no longer carries entity data. For an interface `returns`, it is `interfaces.contains(&returns)`; for a union, `union_members(entity_named(returns)).contains(&entity)`; otherwise `false`. `entity` implements `Node` here, so `node(id: ID!): Node` passes.

Delta from `dddbc8ef3^`: the removed plan passed every `node` argument as an operation variable (`GraphQLFetchArgument`, `fetch_plan.rs:199-211`); i2 passes only `id`, because a fetch by id is named by one variable, and a required other argument is `RequiredArgument`.

Memo invalidation: `graphql_fetch_plan(Pet)` now also reads `Query.node`'s definition and its return type's data, so editing `type Query` re-runs it; it backdates when the plan is `==`. A syntax error in `type Query` fails `Pet`'s plan (`NodeField(Selectable(..))`) and leaves `Mutation`'s plan unchanged, as at `dddbc8ef3^`.

## Errors and where they are reported

- `EntrypointNotFetchableError::ProtocolSelectable`: at the entrypoint's `name`. Unchanged text.
- `EntrypointNotFetchableError::Entity(SourceEntity | NotComposite | Protocol(_))`: at the entrypoint's `parent_type`. New texts: `` `Owner` is declared in source files, so the network protocol cannot fetch it. ``, `` `String` has no selectables, so it cannot be fetched. ``, and the protocol's.
- `EntrypointNotFetchableError::WrappedNotSupported`: at the entrypoint's `parent_type`, `` `Pet` is fetched by its id; entrypoints on entities fetched by id are not supported yet. `` Generation returns the same error, so no artifacts are written.
- `GraphQLNotFetchable::NotObject`: `` `Node` is not an object type, so it cannot be fetched by itself. ``
- `GraphQLNotFetchable::NeitherRootNorNode`: `` `PetStats` is not a root operation type and does not implement `Node`, so it cannot be fetched by itself. ``
- `GraphQLNotFetchable::NodeField(_)`: the `NodeFieldError` text, for example `` `Query.node` is not defined. ``, `` `Query.node` returns a list; it returns one value. ``, `` `Query.node` returns `Viewer`, which is not `Pet` or an interface or union containing it. ``, `` `Query.node` has no argument `id: ID!`. ``, `` `Query.node` requires `tenant`, which a fetch by id does not pass. ``
- `LoadableError::ProtocolSelectable`, `LoadableError::ParentNotFetchable`: at the `@loadable` directive.
- `AstError::LoadableArguments`: at the argument list, a parse error. The literal produces no artifacts, as with any parse error.
- `AstError::UnexpectedDirective`: unchanged, now also for `@loadable` on a declaration.

## Tests

`crates/isograph_parser` (`parse_iso_literal.rs` tests, full semantic tokens through `parsed`):

- `a_loadable_selection_parses_without_errors`: `field Pet.X { PetCheckinsCard @loadable }` has no errors; `selection.loadability()` is `Loadable(span_of(text, "@loadable"))`.
- `a_loadable_selection_with_arguments_is_an_error_on_the_arguments`: `PetCheckinsCard @loadable(lazyLoadArtifact: true)` has one error, `LoadableArguments` at `(lazyLoadArtifact: true)`; `@loadable()` has one at `()`.
- `loadable_on_a_declaration_is_unexpected`: `field Pet.X @loadable { a }` and `entrypoint Pet.X @loadable` each have `unexpected_directives(text, &["@loadable"])`.
- `a_selection_without_loadable_is_eager`: `a`, `a @updatable` (with its `unexpected directive`) are `Eager`.
- The landed tests that fixture `@loadable` on a selection (`two_directives_on_one_selection_stay_in_one_list`, `a_directive_with_empty_arguments_parses`, `a_selection_with_alias_arguments_directives_and_a_nested_set_parses`) drop `@loadable` from their `unexpected_directives` lists; `a_directive_with_empty_arguments_parses` expects `LoadableArguments` at `()`.

`crates/isograph_compiler`:

- `entity_fetch_plan` of a source entity is `SourceEntity` (the landed `a_source_entity_cannot_parent_an_entrypoint`, asserting `EntrypointNotFetchableError::Entity(SourceEntity(owner))`).
- Merge: the map of `field Pet.X { PetCheckinsCard @loadable }` equals the map of `field Pet.X { PetCheckinsCard }`.
- `field_types`: the data type and reader AST of `field Pet.X { PetCheckinsCard @loadable }` equal those of `field Pet.X { PetCheckinsCard }`.

`crates/graphql_network_protocol` (pet demo schema):

- `graphql_fetch_plan`: `Query` is `{ Query, Unwrapped({ Query, {Mutation} }) }`; `Pet`, `Checkin`, `Viewer`, `BlogItem`, `AdItem`, `Image` are `{ Query, Wrapped }`; `PetStats` and `BestFriendRelationship` are `NeitherRootNorNode`; `Node`, `ICheckin`, `NewsfeedItem`, `String`, `PetSpecies`, `SetPetTaglineParams` are `NotObject`; `Nobody` is `UnknownEntity`. Replaces `only_the_root_operation_types_of_the_pet_demo_are_fetchable`.
- Restored from `dddbc8ef3^` with the new variants: `a_broken_node_field_fails_node_fetches_only` (`node(id: ID!): [Node]` is `ReturnsList`, `node(id: ID!): Viewer` is `TypeMismatch`, no `node` is `NotFound`, `node(key: ID!): Node` is `IdArgument`, `node(id: ID): Node` is `IdArgument`, `node(id: ID!, tenant: String!): Node` is `RequiredArgument(tenant)`, `node(id: ID!): [Node]!` is `ReturnsList`, `node(id: ID!): Missing` is `UnknownTarget`, `node(id: String!): Node` and `node: Node` are `IdArgument`, and in each case `Query` and `Mutation` keep their plans and `Viewer` without `implements Node` is `NeitherRootNorNode`); `node` returning `Node`, `Node!`, a union containing the entity, or the entity itself gives a plan; `a_syntax_error_in_the_query_root_fails_node_fetches_and_not_other_roots`; `node(id: ID!, hint: Int = 0, ids: [ID!], tenant: String! = "a"): Node` gives `Pet` a plan.
- `query_for_entrypoint(Pet, PetTagline)` is `NotFetchable(WrappedNotSupported(Pet))`, and `project_artifacts` of the pet demo plus `entrypoint Pet.PetTagline` is `EntrypointQuery` naming `Pet.PetTagline`.
- Validation (`iso_literal_validation_errors_in_file`): `entrypoint Pet.PetTagline` has one at `Pet` (`WrappedNotSupported`); `entrypoint PetStats.Card`, `entrypoint Node.Card`, `entrypoint Owner.OwnerBadge` each have one at the parent with the texts above; with `node(id: ID!): [Node]`, `entrypoint Pet.PetTagline` has one at `Pet` (`ReturnsList`) and `entrypoint Query.HomeRoute` none. `PetCheckinsCard @loadable` on `pet` has none; `StatsBadge @loadable` under `stats` has one at `@loadable`; `tagline @loadable` has one at `@loadable`; a `@loadable` selection omitting a required argument of its field has `MissingFieldArgument`, as without the directive; a `@loadable` selection on a `Query` field has none.

## Changes

Each change ships alone, prefactors first. Changes 1 to 3 are the first version. Later is not part of it.

### 1. `entity_fetch_plan` (isograph_compiler)

`EntityNotFetchableError`, `entity_fetch_plan` returning the plan; `EntrypointNotFetchableError` keeps `ProtocolSelectable` and gains `Entity`; `entrypoint_fetch_plan` calls `entity_fetch_plan` and returns the plan as before; the messages above. Tests: the compiler test and the `graphql_network_protocol` validation tests assert the new texts.

### 2. Node entities are fetchable; entrypoints on them are not supported yet (isograph_compiler, graphql_network_protocol)

`NetworkProtocol::entrypoint_wrapper` replaces `root_entities`; `EntrypointWrapper` with `Unwrapped(RootEntities)` and the unit `Wrapped`; the four test protocols returning `Unwrapped`; `EntrypointFetch`, `entrypoint_fetch_plan` matching the wrapper, `EntrypointNotFetchableError::WrappedNotSupported` and its location; `query_for_entrypoint` destructuring `EntrypointFetch { plan, roots }`; `GraphQLFetchPlan { operation_type, wrapper }`; `NeitherRootNorNode`, `NodeField`, `NodeFieldError`, `node_field`, `contains`, `is_non_null_id`. Tests above. Depends on 1.

This plan's earlier change 2 (root access in the normalization AST, the TS root, `wrapper_variables_type.ts`) is not needed for validation and moves to Later.

### 3. `@loadable` (isograph_parser, isograph_compiler)

`SelectionDirectiveName`, `DeclarationDirectiveName`, `DirectivePosition`, `AstError::LoadableArguments`, `Loadability`, `Selection::loadability`; `LoadableError`, `LoadableParentNotFetchable`, `loadable_error`, and the `validate_selection` arm. Tests above. Depends on 1; after 2, `@loadable` under a `Pet` passes.

### Later. Entrypoints on Node entities (isograph_compiler, graphql_network_protocol, typescript_artifacts, libs/isograph-runtime, demos)

Specified in Later: entrypoints on Node entities. Two changes, in order:

1. Root access in the normalization AST: `NormalizationRootRecord { entity_name, access }` with `NormalizationRootAccess::Fixed` only; the TS `NormalizationRootRecord`, `entrypointRoot`, and `root` parameters of `normalize`, `check`, and `RetainedQuery`; `IsographEntrypoint`'s fourth type argument and `wrapperVariableNames: {}`; `wrapper_variables_type.ts` with `Record<PropertyKey, never>`; regenerated pet demo artifacts.
2. Node-wrapped generation: `EntrypointWrapper::Wrapped(WrappedEntrypoint)`, `WrapperStep`, `WrapperVariable`, `MergedArguments::single`, `with_variable_renamed`; `WrappedNotSupported` removed, `entrypoint_fetch_plan` returning `EntityFetch { plan, shape }`; `OperationWrapper`, `WrapperVariableBinding`, `QueryArtifact.wrapper_variable`, `wrapped_operation`, `operation_variable_name`; `NormalizationRootAccess::Wrapped` and the path types in Rust and TS; `node_field` becoming `node_wrapper`; the printer; `EntrypointArtifacts.wrapper_parameters`, `wrapper_of_variable_type`; `rootResponseObject` in `normalize`. Tests in Later's Tests.

## Later: entrypoints on Node entities

Not part of the first version. This section is the design for generating the operation and artifacts of an entrypoint on a Node entity, and for `@loadable` artifacts after it. Its snippets are written against the first version.

### End-user experience

`entrypoint Pet.PetTagline` generates this query text:

```graphql
# from libs/isograph-runtime/fixtures/store/src/__isograph/Pet/PetTagline/query_text.graphql
query PetTagline($id: ID!) {
  node____id___v_id: node(id: $id) {
    __typename
    ... on Pet {
      __typename
      id
      tagline
    }
  }
}
```

The caller passes the wrapper's variables (`{ id }`) apart from the field's own variables:

```ts
// from libs/isograph-runtime/test/network.test.ts
import petTagline from "../fixtures/store/src/__isograph/Pet/PetTagline/entrypoint";

const [request, release] = fetchEntrypoint(
  environment,
  petTagline,
  { id: "1" }, // the wrapper's variables: which Pet
  {}, // PetTagline declares no variables
  null,
);
```

With a schema field `friend(id: ID!): Pet` on `Pet`, a field `field Pet.BestFriendCard($id: ID!) { friend(id: $id) { tagline } }` that declares its own `$id` does not collide: the caller passes `{ id: "1" }` as the wrapper's variables and `{ id: "2" }` as the field's, and the printed operation declares the wrapper's variable as `$id_1` (The operation's variables).

### Fetch plans with the entity's shape

Delta from the first version: `entity_fetch_plan` returns the entity's composite shape with the plan, because `query_for_entrypoint` completes the selections under the entity's identity when the entity is wrapped (The operation); `entrypoint_fetch_plan` no longer matches the wrapper and returns `EntityFetch`; `EntrypointFetch` and `WrappedNotSupported` are removed; `query_for_entrypoint` reads the wrapper through `wrapped_operation`.

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// How the protocol fetches an entity by itself, and the entity's composite shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityFetch<TFetchPlan> {
    pub plan: TFetchPlan,
    pub shape: CompositeShape,
}

pub type ProfileEntityFetch<TProfile> = EntityFetch<ProfileFetchPlan<TProfile>>;
pub type ProfileEntityNotFetchableError<TProfile> = EntityNotFetchableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

/// The protocol's plan for fetching `entity` by itself.
pub fn entity_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
) -> Result<ProfileEntityFetch<TProfile>, ProfileEntityNotFetchableError<TProfile>> {
    let data = match entity.origin.reference() {
        EntityOrigin::Source => {
            return EntityNotFetchableError::SourceEntity(entity.name).wrap_err();
        }
        EntityOrigin::Protocol(data) => data,
    };
    let shape = match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(shape) => shape,
        EntityShape::Leaf | EntityShape::Input => {
            return EntityNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    };
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map(|plan| EntityFetch { plan, shape })
        .map_err(EntityNotFetchableError::Protocol)
}

/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileEntityFetch<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    entity_fetch_plan(db, client, entity).map_err(EntrypointNotFetchableError::Entity)
}
```

`entrypoint_declaration_error` locates `ProtocolSelectable` at `name` and `Entity(_)` at `parent_type`.

### The entrypoint wrapper's path

`EntrypointWrapper::Wrapped` gains its payload:

The core types, in `crates/isograph_compiler/src/entrypoint.rs`:

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// How an operation reaches the record an entrypoint's selections are on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntrypointWrapper {
    /// The entity is `RootEntities::operation`: the response's top-level object is its one
    /// record, and the entrypoint's selections are the operation's top level.
    Unwrapped(RootEntities),
    /// The entrypoint's selections are the selection set of the last step of a path the
    /// protocol selects, on the record the caller names by a variable.
    Wrapped(WrappedEntrypoint),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedEntrypoint {
    /// The operation's root entities; the response's top-level object is
    /// `roots.operation`'s one record.
    pub roots: RootEntities,
    /// From the response's top-level object to the entrypoint's response object, outermost
    /// first. Argument values name `id_variable`.
    pub path: NonEmpty<WrapperStep>,
    /// The variable the caller passes, apart from the entrypoint field's variables, whose
    /// value is the id of the entrypoint's record.
    pub id_variable: WrapperVariable,
}

/// One step of a wrapper's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WrapperStep {
    /// A selection of a protocol selectable whose target is composite; the next step, or the
    /// entrypoint's selections, select on its response object.
    Selectable(SelectionKey),
    /// The next step, or the entrypoint's selections, apply when the response object's
    /// discriminator names the condition's entity; otherwise the response holds no record
    /// for the entrypoint.
    Conditional(Condition),
}

/// A variable a wrapper declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapperVariable {
    pub name: VariableName,
    pub variable_type: VariableType,
}
```

`RootEntities`, `SelectionKey`, `Condition`, and `VariableType` are the landed core types. `RootEntities` stays: it is what the protocol says about which entities have one fixed record, which completion reads; the core does not decide it. `NonEmpty` is the `nonempty` crate the core already depends on. `WrappedEntrypoint` has exactly one variable because a fetch by identity is named by one id; GraphQL's `node` must take no other required argument (The GraphQL fetch plan). A protocol that needs more variables is a later change to this type.

The `... on Pet` is `WrapperStep::Conditional(Condition::SelectionEquals(Pet))`, the condition type of every conditional, and it becomes a `Conditional` step of the normalization AST's root path (The normalization AST's root). That keeps normalization-ast.md's decision: the id comes from the caller, and for another entity's id the server returns `{ __typename: "Other" }`, which the conditional skips instead of throwing.

`MergedArguments` gains an infallible constructor for one argument, which the GraphQL wrapper uses:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
impl MergedArguments {
    /// The arguments holding only `argument`.
    pub fn single(argument: MergedArgument) -> Self {
        MergedArguments(argument.wrap_vec())
    }
}
```

### The GraphQL node wrapper

`node_field` becomes `node_wrapper`: the same checks, then the wrapper. `graphql_fetch_plan` ends with `node_wrapper(db, client, roots.reference(), entity, composite.interfaces.as_slice()).map(|wrapped| GraphQLFetchPlan { operation_type: OperationType::Query, wrapper: EntrypointWrapper::Wrapped(wrapped) }).map_err(GraphQLNotFetchable::NodeField)`.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// The wrapper that fetches the object type `entity`, which implements `interfaces`, through
/// the `node` field of the schema's query root.
fn node_wrapper<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    roots: &RootOperationTypes,
    entity: EntityName,
    interfaces: &[EntityName],
) -> Result<WrappedEntrypoint, NodeFieldError> {
    let query_root = roots.query;
    let field = selectable_named(db, client, query_root, NODE_FIELD.intern().to())
        .clone()
        .map_err(NodeFieldError::Selectable)?
        .ok_or(NodeFieldError::NotFound(query_root))?;
    let returns = match field.item.item.type_.item.reference() {
        TypeAnnotation::Named(named) => named.0.item.0,
        TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
            NullableTypeAnnotation::Named(named) => named.0.item.0,
            NullableTypeAnnotation::List(_) => {
                return NodeFieldError::ReturnsList(query_root).wrap_err();
            }
        },
        TypeAnnotation::List(_) => return NodeFieldError::ReturnsList(query_root).wrap_err(),
    };
    let returns_data = graphql_entity(db, client, returns)
        .clone()
        .map_err(NodeFieldError::Target)?
        .ok_or(NodeFieldError::UnknownTarget(returns))?
        .data;
    if returns != entity && !contains(db, client, returns, returns_data.reference(), entity, interfaces) {
        return NodeFieldError::TypeMismatch(NodeFieldTypeMismatch {
            query_root,
            returns,
            entity,
        })
        .wrap_err();
    }
    let arguments = field.item.item.arguments.iter().map(|argument| argument.item.reference());
    let id: InputValueName = ID_ARGUMENT.intern().to();
    if !arguments.clone().any(|argument| {
        argument.name.item.0 == id && is_non_null_id(argument.type_.item.reference())
    }) {
        return NodeFieldError::IdArgument(query_root).wrap_err();
    }
    if let Some(required) = arguments.clone().find(|argument| {
        argument.name.item.0 != id
            && argument.default_value.is_none()
            && matches!(argument.type_.item.reference(), TypeAnnotation::NonNull(_))
    }) {
        return NodeFieldError::RequiredArgument(NodeFieldArgument {
            query_root,
            argument: required.name.item.0,
        })
        .wrap_err();
    }
    let id_variable: VariableName = ID_ARGUMENT.intern().to();
    WrappedEntrypoint {
        roots: roots.root_entities(query_root),
        path: NonEmpty::from((
            WrapperStep::Selectable(SelectionKey {
                name: NODE_FIELD.intern().to(),
                arguments: MergedArguments::single(MergedArgument {
                    name: ID_ARGUMENT.intern().to(),
                    value: MergedValue::Variable(id_variable),
                }),
            }),
            WrapperStep::Conditional(Condition::SelectionEquals(entity)).wrap_vec(),
        )),
        id_variable: WrapperVariable {
            name: id_variable,
            variable_type: VariableType::NonNull(NonNullVariableType::Named(
                "ID".intern().to(),
            )),
        },
    }
    .wrap_ok()
}

### The operation

`query_for_entrypoint` (`crates/isograph_compiler/src/query.rs`) completes the entrypoint field's selections under the record the wrapper names, and hands the protocol the wrapper and the selections separately.

```rust
// from crates/isograph_compiler/src/query.rs
/// What the core hands the protocol to print one entrypoint's query.
pub struct EntrypointOperation<'a, TProtocol: NetworkProtocol> {
    /// The operation name: the entrypoint's selectable name.
    pub name: SelectableName,
    pub fetch_plan: &'a TProtocol::FetchPlan,
    /// The operation's variables: the wrapper's variable under its operation name, then the
    /// entrypoint field's variables that the selections use, in declaration order.
    pub variables: &'a [FieldVariable],
    pub wrapper: &'a OperationWrapper,
    /// The entrypoint field's selections, completed: the operation's top level when `wrapper`
    /// is `Unwrapped`, otherwise the selection set of the wrapper's last step.
    pub selections: &'a CompletedSelectionMap,
}

/// A wrapper as one operation prints it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationWrapper {
    Unwrapped,
    /// Outermost first; argument values name the wrapper's variable by its operation name.
    Wrapped(NonEmpty<WrapperStep>),
}
```

Before, `EntrypointOperation` had `name`, `fetch_plan`, `variables` (the field's used variables), and `selections`. The wrapper's variable rides in `variables` as a `FieldVariable { name, variable_type, default_value: None }`: `FieldVariable` is the core's "a variable an operation declares", which the printer already prints.

```rust
// from crates/isograph_compiler/src/query.rs
/// One entrypoint's query: its text, the protocol's file name for the text, the
/// normalization AST of its response, and the wrapper's variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryArtifact {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
    pub normalization_ast: NormalizationAst,
    /// `None` when the wrapper is `Unwrapped`.
    pub wrapper_variable: Option<WrapperVariableBinding>,
}

/// A wrapper variable: the name the caller passes it under, the name the operation declares
/// it under, and its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapperVariableBinding {
    pub name: VariableName,
    pub operation_name: VariableName,
    pub variable_type: VariableType,
}
```

```rust
// from crates/isograph_compiler/src/query.rs
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
    let EntityFetch { plan, shape } = entrypoint_fetch_plan(
        db,
        client,
        entity_value.reference(),
        selectable_value.reference(),
    )
    .map_err(EntrypointQueryError::NotFetchable)?;
    let field = field_selection_map(db, client, entity, selectable)
        .as_ref()
        .map_err(|error| EntrypointQueryError::SelectionMap(error.clone()))?;
    let wrapped = wrapped_operation::<TProfile::NetworkProtocol>(
        entity,
        shape,
        TProfile::NetworkProtocol::entrypoint_wrapper(plan.reference()).reference(),
        field,
    )
    .map_err(EntrypointQueryError::TopLevel)?;
    let normalization_ast = normalization_ast(
        wrapped.selections.reference(),
        TProfile::NetworkProtocol::discriminator(),
        wrapped.root.clone(),
    )
    .map_err(EntrypointQueryError::IntegerOutOfRange)?;
    let variables = wrapped
        .wrapper_variable
        .iter()
        .map(|binding| FieldVariable {
            name: binding.operation_name,
            variable_type: binding.variable_type.clone(),
            default_value: None,
        })
        .chain(operation_variables(field))
        .collect::<Vec<_>>();
    let ProtocolQueryText { text, file_name } =
        TProfile::NetworkProtocol::query_text(&EntrypointOperation {
            name: selectable,
            fetch_plan: plan.reference(),
            variables: variables.as_slice(),
            wrapper: wrapped.wrapper.reference(),
            selections: wrapped.selections.reference(),
        });
    QueryArtifact {
        text,
        file_name,
        normalization_ast,
        wrapper_variable: wrapped.wrapper_variable,
    }
    .wrap_ok()
}
```

```rust
// from crates/isograph_compiler/src/query.rs
/// The parts of one entrypoint's operation that its wrapper decides.
struct WrappedOperation {
    wrapper: OperationWrapper,
    wrapper_variable: Option<WrapperVariableBinding>,
    selections: CompletedSelectionMap,
    root: NormalizationRootRecord,
}

/// The operation parts of an entrypoint on `entity`, whose shape is `shape`, under `wrapper`,
/// with `field`'s selections.
fn wrapped_operation<TProtocol: NetworkProtocol>(
    entity: EntityName,
    shape: CompositeShape,
    wrapper: &EntrypointWrapper,
    field: &FieldSelectionMap,
) -> Result<WrappedOperation, TopLevelError> {
    match wrapper {
        EntrypointWrapper::Unwrapped(roots) => WrappedOperation {
            wrapper: OperationWrapper::Unwrapped,
            wrapper_variable: None,
            selections: CompletedSelectionMap::top_level::<TProtocol>(
                field.selections.reference(),
                roots,
            )?,
            root: NormalizationRootRecord {
                entity_name: roots.operation,
                access: NormalizationRootAccess::Fixed(FixedRecordId::root()),
            },
        }
        .wrap_ok(),
        EntrypointWrapper::Wrapped(wrapped) => {
            let operation_name = operation_variable_name(
                wrapped.id_variable.name,
                field.variables.as_slice(),
                0,
            );
            let path = wrapped
                .path
                .clone()
                .map(|step| step.with_variable_renamed(wrapped.id_variable.name, operation_name));
            let identity = wrapped.roots.identity_of(entity, shape.identity);
            WrappedOperation {
                selections: complete::<TProtocol>(
                    field.selections.reference(),
                    identity.reference(),
                    wrapped.roots.reference(),
                )
                .map_err(TopLevelError::AddedSelectionCollision)?,
                root: NormalizationRootRecord {
                    entity_name: entity,
                    access: NormalizationRootAccess::Wrapped(NormalizationWrappedRoot {
                        path: path.clone().map(|step| step.normalization_path_step()),
                        id_variable: wrapped.id_variable.name,
                    }),
                },
                wrapper_variable: WrapperVariableBinding {
                    name: wrapped.id_variable.name,
                    operation_name,
                    variable_type: wrapped.id_variable.variable_type.clone(),
                }
                .wrap_some(),
                wrapper: OperationWrapper::Wrapped(path),
            }
            .wrap_ok()
        }
    }
}
```

Under a wrapper, the entrypoint's selections are completed like any nested selection set on `entity`: the discriminator and the entity's identity (`id` for `Pet`) are added. `TopLevelError::Empty` and `TopLevelError::Conditional` apply only to `Unwrapped`: a wrapped selection set always holds the added selections, and a wrapped entity is a GraphQL object, which has no refinements.

#### The operation's variables

The caller passes the wrapper's variables and the field's variables as two objects, so a field `$id` and the wrapper's `id` never meet at the call site. The printed operation has one variable namespace. The core gives the wrapper's variable an operation name no field variable has:

```rust
// from crates/isograph_compiler/src/query.rs
/// `name` when `attempt` is 0, else `name_<attempt>`, or the next attempt when a variable of
/// `field_variables` has that name.
fn operation_variable_name(
    name: VariableName,
    field_variables: &[FieldVariable],
    attempt: usize,
) -> VariableName {
    let candidate: VariableName = match attempt {
        0 => name,
        attempt => format!("{name}_{attempt}").intern().to(),
    };
    if field_variables.iter().any(|variable| variable.name == candidate) {
        operation_variable_name(name, field_variables, attempt + 1)
    } else {
        candidate
    }
}
```

The check runs against every variable the field declares, used or not, so adding a use of a declared variable does not rename the wrapper's. A field declaring `$id` and `$id_1` gets `$id_2`. The recursion depth is at most the number of field variables. It is recursive rather than `(0..).find(..)` because `find` returns an `Option` the body would always unwrap.

```rust
// from crates/isograph_compiler/src/entrypoint.rs
impl WrapperStep {
    /// This step with every `Variable(from)` in its arguments, at any depth, as `Variable(to)`.
    pub fn with_variable_renamed(self, from: VariableName, to: VariableName) -> Self;

    /// The normalization AST's step for this step.
    pub fn normalization_path_step(&self) -> NormalizationPathStep;
}
```

`with_variable_renamed` maps `Selectable(key)` to `Selectable(SelectionKey { name, arguments: key.arguments.with_variable_renamed(from, to) })` and returns a `Conditional` unchanged. `MergedArguments::with_variable_renamed` and `MergedValue::with_variable_renamed` are new infallible methods next to `substitute_arguments`, which fails on an undeclared variable and so does not fit. `normalization_path_step` maps `Selectable(key)` to `NormalizationPathStep::Selection(NormalizationPathSelection { response_key: key.response_key(), selectable_name: key.name })` and `Conditional(condition)` to `NormalizationPathStep::Conditional(condition.clone())`. The response key is computed after renaming, so the alias printed (`node____id___v_id_1`) and the AST's agree.

The runtime builds the request's variables from both objects and the artifact's mapping (Runtime).

#### GraphQL printing

```rust
// from crates/graphql_network_protocol/src/print_operation.rs
pub(crate) fn print_operation(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> QueryText {
    // header unchanged: `{operation_type} {name}({variables}) {`
    let body = match operation.wrapper {
        OperationWrapper::Unwrapped => print_selection_set(operation.selections, 1),
        OperationWrapper::Wrapped(path) => {
            let (first, rest) = path.split_first();
            print_wrapper_step(first, rest, operation.selections, 1)
        }
    };
    QueryText(
        std::iter::once(header)
            .chain(body)
            .chain(std::iter::once("}".to_owned()))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The lines of `step` at `depth`: its head, its selection set holding `rest` (or
/// `selections` when `rest` is empty), and its closing brace.
fn print_wrapper_step(
    step: &WrapperStep,
    rest: &[WrapperStep],
    selections: &CompletedSelectionMap,
    depth: usize,
) -> Vec<String> {
    let prefix = indent(depth);
    let head = match step {
        WrapperStep::Selectable(key) => print_selection_head(key),
        WrapperStep::Conditional(Condition::SelectionEquals(entity)) => format!("... on {entity}"),
    };
    let discriminator = match (step, rest.first()) {
        (WrapperStep::Selectable(_), Some(WrapperStep::Conditional(_))) => {
            format!("{}{TYPENAME}", indent(depth + 1)).wrap_some()
        }
        (WrapperStep::Selectable(_), Some(WrapperStep::Selectable(_)) | None)
        | (WrapperStep::Conditional(_), _) => None,
    };
    let inner = match rest.split_first() {
        Some((next, rest)) => print_wrapper_step(next, rest, selections, depth + 1),
        None => print_selection_set(selections, depth + 1),
    };
    std::iter::once(format!("{prefix}{head} {{"))
        .chain(discriminator)
        .chain(inner)
        .chain(std::iter::once(format!("{prefix}}}")))
        .collect()
}
```

The `__typename` under `node` is printed because the next step is a conditional, which reads the discriminator; without it a `node` of another type returns `{}` and the runtime cannot tell it from a malformed response. Upstream prints no `__typename` at the `node` level (`BlogItemMoreDetail/query_text.ts:1-3`). The entrypoint's selections under `... on Pet` begin with the `__typename` and `id` completion added. `entrypoint Pet.PetTagline` prints the text in End-user experience.

`print_selection_head` is the landed function: `node(id: $id)` has an argument, so it prints under its normalization alias `node____id___v_id`, as upstream does.

### The normalization AST's root

Before:

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
/// The operation's root entity's one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationRootRecord {
    pub entity_name: EntityName,
    pub id: FixedRecordId,
}
```

After:

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
/// The normalization AST of one query's response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationAst {
    pub discriminator_response_key: SelectableName,
    /// The record the entrypoint's selections are written onto, where read and check start.
    pub root: NormalizationRootRecord,
    pub selections: NormalizationSelectionSet,
}

/// The record the entrypoint's selections are on, and how the response reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationRootRecord {
    pub entity_name: EntityName,
    pub access: NormalizationRootAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationRootAccess {
    /// The response's top-level object is the record with this id.
    Fixed(FixedRecordId),
    /// The response reaches the record's response object through a path, and the record's
    /// id is a wrapper variable's value.
    Wrapped(NormalizationWrappedRoot),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationWrappedRoot {
    /// From the response's top-level object, outermost first.
    pub path: NonEmpty<NormalizationPathStep>,
    /// The wrapper variable, by the name the caller passes it under.
    pub id_variable: VariableName,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationPathStep {
    /// The response object's value under the response key: an object, or `null` when the
    /// response holds no record for the entrypoint.
    Selection(NormalizationPathSelection),
    /// Continue when the response object's discriminator names the condition's entity.
    Conditional(Condition),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationPathSelection {
    pub response_key: ResponseKey,
    pub selectable_name: SelectableName,
}
```

`normalization_ast(selections, discriminator_response_key, root: NormalizationRootRecord)` takes the root where it took `root_entity: EntityName`; `wrapped_operation` builds it. The record id is a variable's value: `(Pet, $id)` is the "variable root identity". Upstream roots the AST at the query root and selects `node` as a linked field (`BlogItemMoreDetail/normalization_ast.ts:6-19`); i2 roots it at the entrypoint's record and keeps the wrapper as a path the normalizer walks without writing. Open decision 3.

The path is not written to the store: no `Query.__ROOT` record gets a `node(id:)` key. Read, check, and garbage collection start at `(Pet, id)` and never read that key, so writing it would add a record that only the garbage collector reads.

TypeScript, in `libs/isograph-runtime/src/artifacts.ts`, before:

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The operation's root entity's one record. */
export type NormalizationRootRecord<
  TEntityName extends EntityName = EntityName,
> = {
  readonly entityName: TEntityName;
  readonly id: string;
};
```

After:

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The record the entrypoint's selections are on, and how the response reaches it. */
export type NormalizationRootRecord<
  TEntityName extends EntityName = EntityName,
> = {
  readonly entityName: TEntityName;
  readonly access: NormalizationRootAccess;
};

export type NormalizationRootAccess =
  /** The response's top-level object is the record `id`. */
  | { readonly kind: 'Fixed'; readonly id: string }
  /** The response reaches the record's response object through `path`; the record's id is
   * the wrapper variable `idVariable`'s value. */
  | {
      readonly kind: 'Wrapped';
      readonly path: readonly [NormalizationPathStep, ...NormalizationPathStep[]];
      readonly idVariable: string;
    };

export type NormalizationPathStep =
  /** The response object's value under `responseKey ?? selectableName`. */
  | {
      readonly kind: 'Selection';
      readonly responseKey: string | null;
      readonly selectableName: string;
    }
  /** Continue when the response object's discriminator is `condition.entityName`. */
  | { readonly kind: 'Conditional'; readonly condition: Condition };
```

`typescript_artifacts/src/normalization_ast.rs` prints the root as `{ entityName, access: { kind: "Fixed", id: "__ROOT" } }` or `{ entityName, access: { kind: "Wrapped", path: [..], idVariable: "id" } }`, each path step as the TS type above, with `responseKey` `null` for `ResponseKey::SelectableName`, as selections print it.

The store fixture's `Pet/PetTagline/normalization_ast.ts`:

```ts
// from libs/isograph-runtime/fixtures/store/src/__isograph/Pet/PetTagline/normalization_ast.ts
import type { NormalizationAst } from "@isograph/runtime";
const normalizationAst: NormalizationAst<"Pet"> = {
  discriminatorResponseKey: "__typename",
  root: {
    entityName: "Pet",
    access: {
      kind: "Wrapped",
      path: [
        { kind: "Selection", responseKey: "node____id___v_id", selectableName: "node" },
        { kind: "Conditional", condition: { kind: "SelectionEquals", entityName: "Pet" } },
      ],
      idVariable: "id",
    },
  },
  selections: [
    { kind: "Selection", responseKey: null, selectableName: "id", arguments: null, target: null },
    { kind: "Selection", responseKey: null, selectableName: "tagline", arguments: null, target: null },
  ],
};
export default normalizationAst;
```

### Generated artifacts

`EntrypointArtifacts` (`crates/isograph_compiler/src/artifact_types.rs`) gains the wrapper's parameters:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// An `entrypoint` literal, its query, and the parameters type of its wrapper's variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointArtifacts {
    pub entrypoint: EntityNameAndSelectableName,
    pub query: QueryArtifact,
    /// `None` when the wrapper declares no variable.
    pub wrapper_parameters: Option<ParametersType>,
}
```

`project_artifacts` builds `wrapper_parameters` from `query.wrapper_variable`: one `ArtifactParameter { name: binding.name, type_: wrapper_of_variable_type(binding.variable_type.reference()).try_map(&mut |entity| input_type(db, client, entity.dereference()))? }`, and `input_objects` of the types it names, as `parameters_type` does for a field. `wrapper_of_variable_type(&VariableType) -> Wrapper<EntityName>` is new in `crates/isograph_compiler/src/wrapper.rs`: `NonNull(Named(n))` is `Inner(n)`, `NonNull(List(t))` is `List(of(t))`, `Nullable(x)` is `Union([that of NonNull(x) as a variant, Null])`, matching mental-model.md's table.

`typescript_artifacts` writes, per entrypoint:

- `<Entity>/<selectable>/wrapper_variables_type.ts`, exporting `<Entity>__<selectable>__wrapper_variables`: the `parameters_type_file` printer over `wrapper_parameters` (`{ readonly id: string }` for `ID!`), or `Record<PropertyKey, never>` when `None`. `parameters_type_file` takes the type name and path as parameters where it computed them from the field.
- `entrypoint.ts`, with the fourth type argument imported from `./wrapper_variables_type`, and an entry `wrapperVariableNames` mapping each wrapper variable's caller name to its operation name (`{}` when `None`).

```ts
// from libs/isograph-runtime/fixtures/store/src/__isograph/Pet/PetTagline/entrypoint.ts
import type { IsographEntrypoint } from "@isograph/runtime";
import type { Pet__PetTagline__param } from "./param_type";
import type { Pet__PetTagline__output_type } from "./output_type";
import type { Pet__PetTagline__wrapper_variables } from "./wrapper_variables_type";
import Pet__PetTagline__normalization_ast from "./normalization_ast";
import Pet__PetTagline__query_text from "./query_text";
import Pet__PetTagline__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<
  Pet__PetTagline__param,
  Pet__PetTagline__output_type,
  "Pet",
  Pet__PetTagline__wrapper_variables
> = {
  kind: "Entrypoint",
  name: "Pet.PetTagline",
  queryText: Pet__PetTagline__query_text,
  normalizationAst: Pet__PetTagline__normalization_ast,
  wrapperVariableNames: { id: "id" },
  readerArtifact: Pet__PetTagline__resolver_reader,
};
export default artifact;
```

`parameters_type.ts` is unchanged: it is the field's parameters type, which any selection of the field also uses, and the wrapper's variable is not a parameter of the field. `entrypoint_artifact(entrypoint, root_entity)` reads `query.normalization_ast.root.entity_name` for the third type argument, which is now the entrypoint's entity (`"Pet"`) for a wrapped entrypoint.

### Runtime

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The default export of an `entrypoint.ts` artifact: an entrypoint whose field's param type
 * is `TParam` and output type is `TOutput`, on the entity `TEntityName`, whose wrapper takes
 * `TWrapperVariables`. */
export type IsographEntrypoint<
  TParam,
  TOutput,
  TEntityName extends EntityName,
  TWrapperVariables extends Variables,
> = {
  readonly kind: 'Entrypoint';
  readonly name: string;
  readonly queryText: string;
  readonly normalizationAst: NormalizationAst<TEntityName>;
  /** The operation variable name of each wrapper variable. */
  readonly wrapperVariableNames: { readonly [K in keyof TWrapperVariables]: string };
  readonly readerArtifact: unknown;
  readonly '~param'?: TParam;
  readonly '~output'?: TOutput;
};
```

```ts
// from libs/isograph-runtime/src/root.ts
/** The record `ast`'s selections are on, given the entrypoint's wrapper variables. Throws when
 * the id variable's value is not a string. */
export function entrypointRoot<TEntityName extends EntityName>(
  ast: NormalizationAst<TEntityName>,
  wrapperVariables: Variables,
): Link<TEntityName> {
  const access = ast.root.access;
  switch (access.kind) {
    case 'Fixed':
      return { entityName: ast.root.entityName, id: access.id };
    case 'Wrapped': {
      const id = ownValue(wrapperVariables, access.idVariable);
      if (typeof id !== 'string') {
        throw new Error(
          `The wrapper variable "${access.idVariable}" is ${JSON.stringify(id)}; a record id is a string.`,
        );
      }
      return { entityName: ast.root.entityName, id };
    }
  }
}

/** The response object the entrypoint's selections are read from, or `null` when the path
 * ends at `null` or at a conditional whose entity the response object is not. Throws on a
 * response that does not match the path. */
export function rootResponseObject(
  ast: NormalizationAst,
  response: JsonObject,
): JsonObject | null;
```

`rootResponseObject` returns `response` for `Fixed`. For `Wrapped` it walks the path: a `Selection` step reads `ownValue(object, responseKey ?? selectableName)`, throws `Malformed response at [..]: expected the response key ".."` when absent, returns `null` for `null`, and throws on a non-object or a list; a `Conditional` step returns `null` unless `ownValue(object, ast.discriminatorResponseKey) === condition.entityName`.

The store functions take the root link where they read `ast.root`:

- `normalize(store, ast, response, root: StoreLink, variables)`: `const object = rootResponseObject(ast, response); if (object === null) return;` then writes `ast.selections` from `object` onto `root`, as it wrote from `response` onto `ast.root`. A `node` that is `null`, or of another entity, writes nothing.
- `check(store, ast, root: StoreLink, variables)`: unchanged except that it starts at `root`. A Node entrypoint is `EnoughData` when `(Pet, "1")` holds every selected value, however the record got there.
- `garbageCollect`: `RetainedQuery` gains `root: StoreLink`; the walk starts at `retained.root`.
- `read(store, readerArtifact, root, variables)` already takes the root.

`variables` in all four is the entrypoint field's variables, which the selections' arguments name. The wrapper's variables are read only by `entrypointRoot`.

`fetchEntrypoint` (network-and-subscriptions.md, `fetchEntrypoint`) takes the wrapper's variables as their own parameter:

```ts
// from libs/isograph-runtime/src/network.ts
export function fetchEntrypoint<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
  TWrapperVariables extends Variables,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName, TWrapperVariables>,
  wrapperVariables: TWrapperVariables,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>>;

/** The request's variables: the field's, and each wrapper variable under its operation name. */
function operationVariables<TWrapperVariables extends Variables>(
  wrapperVariableNames: { readonly [K in keyof TWrapperVariables]: string },
  wrapperVariables: TWrapperVariables,
  variables: Variables,
): Variables {
  return {
    ...variables,
    ...Object.fromEntries(
      Object.entries(wrapperVariableNames).map(([name, operationName]) => [
        operationName,
        ownValue(wrapperVariables, name),
      ]),
    ),
  };
}
```

`fetchEntrypoint` computes `const root = entrypointRoot(entrypoint.normalizationAst, wrapperVariables)` once and passes `root` to `check`, `normalize`, `read` (in `readDataForOnComplete`, where it passed `entrypoint.normalizationAst.root`), and the `RetainedQuery`. The network function receives `operationVariables(..)`. The request cache (network-and-subscriptions.md, The request cache) keys on the query text and the operation variables, so two calls differing only in `id` are two requests. A root entrypoint's call passes `{}` as `wrapperVariables`; its type is `Record<PropertyKey, never>`, so passing `{ id }` there is a type error.

### `@loadable` later

When loadable artifacts land, the loadable query of `P.f @loadable` is the entrypoint query of `entrypoint P.f`, with `P`'s wrapper. Its wrapper variable is the parent record's id, which the reader holds as the parent's link (`Link<P>.id`): no selection of the parent is read for it, and no protocol method maps parent selections to variables. The field's own variables come from the selection's arguments and the caller, in the field's variables object, so a field `$id` does not collide with the wrapper's `id`. For a root `P` the wrapper is `Unwrapped` and takes no variables. Open decision 5.

### Tests

`crates/isograph_compiler`:

- `operation_variable_name`: no field variables gives `id`; `[id]` gives `id_1`; `[id, id_1]` gives `id_2`; `[id_1]` gives `id`.
- `with_variable_renamed` renames a variable inside an object argument and leaves a string `"id"` unchanged.

`crates/graphql_network_protocol` (pet demo schema):

- `graphql_fetch_plan`: `Pet`, `Viewer`, `BlogItem`, `AdItem`, `Image` are `{ Query, Wrapped }` with `path == [Selectable(node(id: $id)), Conditional(SelectionEquals(T))]`, `id_variable == { id, ID! }`, `roots == { Query, {Mutation} }`.
- Query text of `entrypoint Pet.PetTagline` is the text in End-user experience, and `assert_lines_match` holds; `text_lines` and `ast_lines` walk the wrapper path (`ast_lines` emits the path's lines before the selections).
- A wrapper variable whose name a field variable has: the pet demo schema plus `extend type Pet { friend(id: ID!): Pet }`, `field Pet.BestFriendCard($id: ID!) { friend(id: $id) { tagline } }`, and `entrypoint Pet.BestFriendCard`. The operation declares `($id_1: ID!, $id: ID!)`, prints `node____id___v_id_1: node(id: $id_1)` with `friend____id___v_id: friend(id: $id)` under `... on Pet`, has `wrapper_variable == { name: id, operation_name: id_1, variable_type: ID! }`, and its AST path's first response key is `node____id___v_id_1`.
- The AST of `entrypoint Pet.PetTagline`: `root == { Pet, Wrapped { path, id_variable: id } }`, selections `id` and `tagline`.
- Validation: `entrypoint Pet.PetTagline` has none.

`crates/typescript_artifacts`: `entrypoint.ts` of a wrapped entrypoint has the fourth type argument and `wrapperVariableNames: { id: "id" }`; of a root entrypoint, `wrapperVariableNames: {}` and `wrapper_variables_type.ts` exporting `Record<PropertyKey, never>`; the printed normalization root for both accesses.

`libs/isograph-runtime` (fixture `libs/isograph-runtime/fixtures/store` gains `entrypoint Pet.PetTagline` and `field Pet.PetTagline { tagline }`; its schema has `node(id: ID!): Node` and `Pet implements Node`):

- `entrypointRoot`: `Fixed` gives `{ Query, "__ROOT" }`; `Wrapped` with `{ id: "1" }` gives `{ Pet, "1" }`; `{ id: 1 }` throws.
- `normalize` of `{ node____id___v_id: { __typename: "Pet", id: "1", tagline: "Rex" } }` writes `Pet:1` with `id` and `tagline` and no `Query` record; `{ node____id___v_id: null }` and `{ node____id___v_id: { __typename: "Owner" } }` write nothing; `{}` throws the malformed-response error.
- `check` at `{ Pet, "1" }` is `EnoughData` after a `Query.PetDetail` normalize that wrote `Pet:1`'s `tagline`, with no `node` key anywhere.
- `garbageCollect` keeps `Pet:1` for a retained `{ ast, root: { Pet, "1" }, variables: {} }`.
- The landed tests pass `entrypointRoot(ast, {})` where they passed nothing.

## Docs this doc amends

First version:

- `entrypoints.md`: "Which entities may parent an entrypoint" rule 4 and its GraphQL paragraph (root operation types, or objects implementing `Node` while `Query.node(id: ID!)` exists; an entrypoint on a Node entity is `WrappedNotSupported` until Later), the `GraphQLFetchPlan` and `GraphQLNotFetchable` snippets, and the Later note on Node fetches.
- `refactors/pending/iso-directives-unexpected.md`: the supported set is split by position, and `@loadable` is supported on selections.

Later:

- `query-generation.md`, The operation: "There is no fetch selection between the operation and the entrypoint's selections" holds for `Unwrapped` only.
- `normalization-ast.md`, The record: the top level's `root` is the entrypoint's record, `Fixed` for a root entity.
- `refactors/pending/runtime/network-and-subscriptions.md`: `fetchEntrypoint`, `retainQueryWithoutMakingNetworkRequest`, `makeNetworkRequest`, `readDataForOnComplete`, and `getOrCreateCacheForArtifact` take `wrapperVariables` and use `entrypointRoot`; the network function receives `operationVariables(..)`; the request cache keys on them.
- `refactors/pending/runtime/store.md`: `normalize`, `check`, and `RetainedQuery` take the root link.

## Open decisions

Decisions 1 and 6 apply to the first version; the others apply to Later.

1. `@loadable` arguments. Options: none, `LoadableArguments` on any (held: nothing reads `lazyLoadArtifact`); accept upstream's `lazyLoadArtifact: Boolean` and ignore it. Consequences: upstream literals with `@loadable(lazyLoadArtifact: true)` fail to parse under the held option until loadable artifacts read it.
2. The wrapper variable's operation name. Options: the core picks `id`, `id_1`, … avoiding the field's declared variables (held); the protocol renames in the printed text only and returns the mapping from `query_text`; always a reserved name (`$__wrapper_id`) with field variables starting `__` rejected; a field declaring `$id` is a validation error on Node entrypoints. Consequences: the held option keeps the alias, the AST path, and the artifact's `wrapperVariableNames` computed from one value in the core, with one namespace that the runtime's single variables object already assumes; the protocol-only option must hand the name back for the AST's response key and the artifact; the reserved name adds a rule on user names; the error forbids a natural field variable.
3. Where the wrapper sits in the AST. Options: a path on the root that normalize walks without writing (held); the full tree rooted at the query root as upstream, with a separate read root, where `normalize` writes `Query.__ROOT.node(id:)`. Consequences: the full tree makes `check` from `(Pet, id)` need the subtree under the conditional, or makes `check` start at `Query.__ROOT` and report missing data for a `Pet` another query wrote; the held option writes no key that nothing reads.
4. `__typename` at the `node` level. Options: printed (held); not printed (upstream). Consequences: without it a `node` of another type returns `{}`, which the path cannot tell from a response missing `__typename`.
5. `@loadable`'s parent reads (later). Options: the parent record's link id is the wrapper's variable, with no protocol method (held); a protocol method returning the selections to read from the parent (`id`) and their map to the loadable query's variables. Consequences: the method duplicates `WrappedEntrypoint.id_variable`, and a protocol could make the two disagree; the held option ties the loadable query's id to the record the reader is on.
6. Interfaces implementing `Node` as entrypoint parents. Options: not allowed (held); allowed, with the record's entity read from `__typename`. Consequences: allowing it makes the root record's entity a response value, so `check` and `read` cannot name the record before the fetch.
7. `fetchEntrypoint`'s wrapper variables. Options: a positional parameter before the field's variables (held); one object `{ wrapper, field }`; an overload without the parameter for `Unwrapped` entrypoints. Consequences: every root call passes `{}` under the held option; the object form changes every call site's shape; overloads need a type-level test of the wrapper variables type.
8. The record id when the server returns another `id`. Options: the selections are written onto `(T, $id)` from the caller's variable (held); onto the record the response's `id` names. Consequences: under the held option a server that answers `node(id: "1")` with `id: "2"` writes `Pet:1` with `id: "2"`; under the other, `Pet:1` stays absent and every read of the entrypoint misses.
