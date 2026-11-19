# GraphQL protocol: protocol data and entrypoint rules

This is the first of two files. It specifies the data the GraphQL protocol attaches to entities and selectables, which entities may parent an entrypoint, and the validation error for an entrypoint that cannot be fetched. `query-generation.md` in this directory specifies the merged selection map, the printer, the query artifact, and the memos. The ordered change list for both files is at the end of `query-generation.md`.

Requires: `refactors/past/source-backed-lookups.md` (landed), `refactors/pending/validation-diagnostics.md`, `refactors/past/graphql-schema-parser/integration.md` changes 1 to 3 (`schema_file_paths`, `parsed_schema_file`, the protocol's `entity_named` and `selectable_named`). In this file, `graphql_network_protocol::entity_named` and `graphql_network_protocol::selectable_named` name integration.md's protocol lookups, which return `SchemaEntity` and `InSchemaFile<FieldDefinition>`. `entity_named` and `selectable_named` without a crate name are the core lookups in `crates/isograph_compiler/src/schema.rs`.

## End-user experience

```graphql
# schema/schema.graphql
schema {
  query: Query
  mutation: Mutation
}
type Query {
  node(id: ID!): Node
  petByName(name: String!): Pet
  pets: [Pet!]!
}
interface Node {
  id: ID!
}
type Pet implements Node {
  id: ID!
  name: String!
}
type PetStats {
  weight: Int
}
```

```ts
iso(`entrypoint Query.HomeRoute`); // ok: Query is the query root
iso(`entrypoint Mutation.SetTagline`); // ok: Mutation is the mutation root
iso(`entrypoint Pet.PetSummaryCard`); // ok: Pet implements Node
iso(`entrypoint PetStats.Card`); // error on `PetStats`
iso(`entrypoint Owner.Badge`); // error on `Owner` (declared by `entity Owner`)
iso(`entrypoint Query.pets`); // error on `pets` (a schema field, not a `field` declaration)
```

The three errors are LSP diagnostics, published like every other validation error:

- `PetStats`: `` `PetStats` cannot be fetched: it is not a root operation type and does not implement `Node`. ``
- `Owner`: `` `Owner` is declared in source files; only entities from the schema can parent an entrypoint. ``
- `pets`: `` `Query.pets` comes from the schema; an entrypoint names a `field` declaration. ``

`entrypoint Pet.PetSummaryCard` is valid because `Pet` implements `Node` and `Query` has `node(id: ID!): Node`; its query is `node(id: $id) { ... on Pet { ... } }`. `petByName` is not used to fetch `Pet`: in v0 the only entrypoint parents are the root operation types and types that are or implement `Node`.

A broken `Query.node` is an error on every entrypoint whose parent is fetched through it, and on no root operation type's entrypoint: `node(id: ID!): [Node]` gives `` `Query.node` returns a list; it returns one value. `` on `entrypoint Pet.PetSummaryCard`.

## Protocol data on entities and selectables

The core lookups return one `Entity` and one `Selectable` type. Each carries where it came from: a source file declaration, or the network protocol. Protocol-produced ones carry the protocol's data, which the core stores and passes back to the protocol without matching on it.

```rust
// from crates/isograph_compiler/src/host_language.rs
/// The way the compiler communicates with the upstream.
pub trait NetworkProtocol: Sized + 'static {
    /// What the protocol records about an entity it defines.
    type EntityData: Clone + Debug + PartialEq + Eq + 'static;
    /// What the protocol records about a selectable it defines.
    type SelectableData: Clone + Debug + PartialEq + Eq + 'static;
    /// How the protocol fetches an entity that parents an entrypoint.
    type FetchPlan: Clone + Debug + PartialEq + Eq + 'static;
    /// Why an entity defined by the protocol cannot parent an entrypoint.
    type NotFetchable: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;
    /// Why the protocol could not produce a query from a merged selection map.
    type QueryError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;
    /// Why the protocol's lookup of an entity or selectable failed.
    type LookupError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile;

    /// The entity `name` as the protocol defines it; `Ok(None)` when it defines none.
    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        name: EntityName,
    ) -> Result<Option<ProtocolEntity<Self::EntityData>>, Self::LookupError>;

    /// The selectable `selectable` of `entity` as the protocol defines it; `Ok(None)` when it
    /// defines none.
    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
        selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<Self::SelectableData>>, Self::LookupError>;

    /// How `entity`, which the protocol defines, is fetched when it parents an entrypoint.
    fn fetch_plan<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        entity: EntityName,
    ) -> Result<Self::FetchPlan, Self::NotFetchable>;

    /// The query one entrypoint sends, and the protocol's name for a file holding exactly
    /// that text. (query-generation.md)
    fn query_artifact(
        operation: &EntrypointOperation<'_, Self>,
    ) -> Result<QueryArtifact, Self::QueryError>;
}
```

Before, the trait had only `should_include_schema_file`. The core reads every method above: `entity` and `selectable` from `entity_named` and `selectable_named`, `fetch_plan` from validation and from `query_for_entrypoint`, `query_artifact` from `query_for_entrypoint`. `entity`, `selectable`, and `fetch_plan` return owned values: the GraphQL implementation clones them out of its own memos (below), whose results are small. pico cannot yet memoize a trait method (`refactors/pending/memo-on-trait-methods.md`), so each method is a plain function that calls a `#[memo]` free function in the implementing crate.

```rust
// from crates/isograph_compiler/src/schema.rs
/// An entity the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolEntity<TEntityData> {
    pub definition: DefinitionLocation,
    pub data: TEntityData,
}

/// A selectable the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolSelectable<TSelectableData> {
    pub description: Option<DescriptionValue>,
    pub target: Wrapper<EntityName>,
    pub definition: DefinitionLocation,
    pub data: TSelectableData,
}

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity<TEntityData> {
    pub name: EntityName,
    pub definition: DefinitionLocation,
    pub origin: EntityOrigin<TEntityData>,
}

/// Who defines an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityOrigin<TEntityData> {
    /// `entity Owner` in a source file.
    Source,
    Protocol(TEntityData),
}

/// A named pointer from an entity to a wrapper of an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable<TSelectableData> {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub definition: DefinitionLocation,
    pub kind: SelectableKind<TSelectableData>,
}

/// Who defines a selectable, and what it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientPointerTarget {
    Parsed(Wrapper<EntityName>),
    /// A `to` type with a list whose inner type did not parse (`to [!]`).
    Unparsed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerSelectable<TSelectableData> {
    pub target: Wrapper<EntityName>,
    pub data: TSelectableData,
}

impl<TSelectableData> Selectable<TSelectableData> {
    /// The entity selections under a selection of this selectable select on: the first
    /// inner entity of its target.
    pub fn target_entity(&self) -> Option<EntityName> {
        match self.kind.reference() {
            SelectableKind::ClientField => None,
            SelectableKind::ClientPointer(target) => target.target_entity(),
            SelectableKind::Server(server) => server.target.inner_values().next().copied(),
        }
    }
}

/// The entity and selectable types of a profile.
pub type ProfileEntity<TProfile> =
    Entity<<<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityData>;
pub type ProfileSelectable<TProfile> = Selectable<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableData,
>;
```

Before (source-backed-lookups.md):

```rust
// from crates/isograph_compiler/src/schema.rs
pub struct Entity {
    pub name: EntityName,
    pub definition: DefinitionLocation,
}

pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    /// `None` for a field declared without `to`.
    pub target_entity: Option<EntityName>,
    pub definition: DefinitionLocation,
}
```

The structs are generic over the data type, not over the protocol, so `#[derive(Clone, PartialEq, Eq, Debug)]` applies with the bounds the associated types already have. `Wrapper` is artifact-set-and-types.md's (Schema model, Wrapper). The target moves into the kind: a server selectable always has a target, a client field has none, and a client pointer's `to` type may not have parsed. Readers of `selectable.target_entity` (`selectable_target`, hover, `validate_selection`) call `selectable.target_entity()`. `entity_named` returns `DiagnosticResult<ProfileEntity<TProfile>>`; `selectable_named` returns `DiagnosticResult<ProfileSelectable<TProfile>>`.

`entity_named` matches on the source definitions chained with the protocol's definition (source-backed-lookups.md, Later):

```rust
// from crates/isograph_compiler/src/schema.rs
/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<ProfileEntity<TProfile>> {
    let protocol = TProfile::NetworkProtocol::entity(db, client, entity_name)
        .map_err(|error| Diagnostic::new(error.to_string(), None))?;
    let source = source_entity_definitions(db, client, entity_name);
    match (source.as_slice(), protocol) {
        ([definition], None) => Entity {
            name: entity_name,
            definition: definition.dereference(),
            origin: EntityOrigin::Source,
        }
        .wrap_ok(),
        ([], Some(protocol)) => Entity {
            name: entity_name,
            definition: protocol.definition,
            origin: EntityOrigin::Protocol(protocol.data),
        }
        .wrap_ok(),
        ([], None) => Diagnostic::new(format!("Entity `{entity_name}` is not defined."), None).wrap_err(),
        _ => Diagnostic::new(
            format!("Entity `{entity_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
    }
}
```

`selectable_named` is the same shape over `source_selectable_definitions` and `TProfile::NetworkProtocol::selectable`, building `SelectableKind::ClientField` or `SelectableKind::ClientPointer(target)` from the source definition and `SelectableKind::Server(ServerSelectable { target, data })` from the protocol's. `source_selectable_definitions` keeps returning the source-only list; its element type becomes `SourceSelectableDefinition { name, parent_entity, description, kind: SourceSelectableKind, definition }`, where `SourceSelectableKind` is `ClientField` or `ClientPointer(ClientPointerTarget)`, the kinds a `field` declaration can define.

The test protocol (`crates/isograph_compiler/src/test_profile.rs`) defines nothing: every data type is `()`; `LookupError` is `std::convert::Infallible`, because `entity` and `selectable` return `Ok(None)` and `selectables` returns no entries; `NotFetchable` and `QueryError` are a unit struct `TestProtocolError` with `#[error("test protocol")]`, which `fetch_plan` and `query_artifact` return. `Infallible` is acceptable only in these `#[cfg(test)]` profiles.

### GraphQL entity data

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// What the GraphQL protocol records about a type: the facts the printer and the fetch plan
/// read, each from the type's own definition and extensions.
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

// id_field lands with query-generation.md change 7, the change whose code reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLCompositeData {
    /// The interfaces this type declares it implements, in definition then extension order.
    pub interfaces: Vec<EntityName>,
    pub id_field: IdField,
}

/// Whether a type has a field `id: ID!`, which the printer selects for normalization.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IdField {
    Present,
    Absent,
}
```

Scalars and enums are one variant because nothing reads the difference: both print as a bare field name. Object versus interface versus union is read: the printer adds `__typename` under an interface or union, the fetch plan accepts objects and interfaces only. `IdField::Present` is upstream's `server_id_selectable` rule (`isograph/crates/isograph_schema/src/memoized/server_selectable_access.rs`): the type has a field named `id`. Delta: the type must be exactly `ID!`; upstream checks the inner type and has a config option for other types. A field `id: String` is `Absent`, so the printer does not add it.

Root operation kind is not entity data, and neither is fetchability. Both depend on definitions other than the entity's own (the schema definition, the query root's fields). Putting them in `GraphQLEntityData` would make `entity_named(Pet)` fail when the schema definition or `type Query` has a syntax error, which breaks integration.md's isolation rule. They are the memos `root_operation_types` and `graphql_fetch_plan` below.

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// The GraphQL type `name` as `client` reads the schema, with the data the protocol records.
#[memo]
pub fn graphql_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: EntityName,
) -> Result<Option<ProtocolEntity<GraphQLEntityData>>, EntityLookupError>
```

The body reads `graphql_network_protocol::entity_named(db, client, name)` and returns `Ok(None)` for its `Ok(None)`. `definition` is the definition's path and name span. The variant follows `TypeDefinition`'s variant (`Scalar` and `Enum` are `Leaf`). `interfaces` is the definition's `interfaces` then each object or interface extension's `interfaces`. `id_field` is `Present` when `graphql_network_protocol::selectable_named(db, client, name, "id")` is `Ok(Some(field))` and `field.item.type_` is `NonNull(Named(ID))`. A `selectable_named` error on `id` (a duplicate `id`) is `IdField::Absent`: the error belongs to lookups of `id`, not to the type.

### GraphQL selectable data

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// What the GraphQL protocol records about a selectable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphQLSelectableData {
    /// A field of the schema, printed as `name(arguments)`.
    Field,
    /// `asUser` on a union or interface: printed as `... on User`, where `User` is the
    /// selectable's target.
    Refinement,
}
```

Field arguments are not here. The printer prints the arguments the selection passed; checking them against the definition is a later validation pass, and the fetch plan reads the root field's definition through its own lookup. A refinement's concrete type is the selectable's `target_entity()`.

```rust
// from crates/graphql_network_protocol/src/protocol_data.rs
/// The selectable `selectable` of the GraphQL type `entity`: a field of the type, or a
/// refinement `as<Type>` of a union or interface.
#[memo]
pub fn graphql_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<Option<ProtocolSelectable<GraphQLSelectableData>>, GraphQLSelectableLookupError>
```

A field is `graphql_network_protocol::selectable_named`'s result: `target` is the type annotation as a wrapper by mental-model.md's table (`Foo!` is `Inner(Foo)`, `Foo` is `Union([Inner(Foo), Null])`), `definition` is the field's path and name span, `description` the field's. A refinement is the decision of the integration step: `asX` on union `U` when `U`'s member types include `X`, on interface `I` when `X` is an object whose `interfaces` include `I`; no interface-on-interface refinements. Its `target` is `Union([Inner(X), Null])`, nullable as upstream's refinements are, and its `definition` is the target type's definition: `X`'s definition name, so go-to-definition on `asUser` jumps to `type User`. `GraphQLSelectableLookupError` is `SelectableLookupError` plus `RefinementTarget` (the target's own lookup failed) and `RefinementCollision` (a field named `asX` collides with a refinement), as integration.md specifies.

`GraphQLNetworkProtocol::entity` and `::selectable` clone these memos' results; `LookupError` is an enum of `EntityLookupError` and `GraphQLSelectableLookupError` with `derive_more::From`.

## Root operation types

```rust
// from crates/graphql_network_protocol/src/root_operation_types.rs
/// The root operation types of the schema as `client` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootOperationTypes {
    pub query: EntityName,
    pub mutation: Option<EntityName>,
    pub subscription: Option<EntityName>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RootOperationTypesError {
    #[error("The schema definition has a syntax error.")]
    Syntax(InSchemaFile<DefinitionError>),
    #[error("The schema is defined more than once.")]
    DuplicateSchemaDefinition(Box<DuplicateSchemaDefinition>),
    #[error("The schema is extended but never defined.")]
    ExtensionWithoutDefinition(Box<InSchemaFile<SchemaExtension>>),
    #[error("The root operation type `{}` is defined more than once.", .0.operation)]
    DuplicateOperation(DuplicateOperation),
    #[error("The schema definition has no `query` root operation type.")]
    MissingQuery(Box<InSchemaFile<SchemaDefinition>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateSchemaDefinition {
    pub first: InSchemaFile<SchemaDefinition>,
    pub second: InSchemaFile<SchemaDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateOperation {
    pub operation: OperationType,
    pub second: InSchemaFile<RootOperationTypeDefinition>,
}

impl RootOperationTypes {
    /// The operation whose root type is `entity`.
    pub fn operation_type_of(&self, entity: EntityName) -> Option<OperationType>;
}

#[memo]
pub fn root_operation_types<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Result<RootOperationTypes, RootOperationTypesError>
```

The body walks `schema_file_paths(db, client)` and `parsed_schema_file(db, client, path)`, as `entity_named` does, and keeps only schema entries: `Definition(Schema(_))`, `Extension(Schema(_))`, and `Err` whose `DefinitionError.name` is `Some(DefinitionName::Schema)` (prefactor below). Then, in order: a syntax error is `Syntax` of the first; two definitions are `DuplicateSchemaDefinition`; extensions with no definition are `ExtensionWithoutDefinition`; no definition and no extension is the spec 3.3.1 default `{ query: Query, mutation: Some(Mutation), subscription: Some(Subscription) }`; otherwise the definition's then each extension's `root_operation_types` fill the three slots, a slot filled twice is `DuplicateOperation`, and an empty `query` slot is `MissingQuery`. Upstream's origin is `graphql_root_types_from_documents` (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs`). Delta: upstream defaults each missing slot of a present schema definition to the conventional name, and reports a duplicate definition as non-fatal; i2 follows the spec, where a schema definition without `mutation` has no mutation root, and a duplicate is an error of every lookup that reads root types.

The default names do not check that `type Mutation` exists. A root that does not exist fails at `entity_named`, before anything asks whether it is a root.

Every schema file edit re-runs `root_operation_types`, a scan of already-parsed entries that backdates when the result is `==`.

### Prefactor: `DefinitionName::Schema`

`DefinitionError.name` is `None` for a failed schema definition today, the same as for a definition that failed before its name. `root_operation_types` cannot then tell a broken `schema {` from a broken `type {`.

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
/// The name a top-level definition or extension declares.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DefinitionName {
    Entity(EntityName),
    Directive(DirectiveName),
    /// `schema` or `extend schema`, which declare the one schema.
    Schema,
}

pub struct DefinitionError {
    /// The name the failed definition declared; `None` when the parse failed before the
    /// name.
    pub name: Option<DefinitionName>,
    pub error: WithSpan<ParseError>,
}
```

A schema definition or extension whose keyword parsed and whose body then failed is `Some(DefinitionName::Schema)`. Tests (`crates/graphql_schema_parser`): `schema { query: }` and `extend schema { mutation: }` each have `name: Some(Schema)`; `type { a: A }` keeps `None`.

## Which entities may parent an entrypoint

An entrypoint `entrypoint E.f` is allowed when:

1. `E` is defined by the protocol (`EntityOrigin::Protocol`). An `entity` declaration in a source file has nothing to fetch from.
2. `E.f` is a `field` declaration (`SelectableKind::ClientField` or `ClientPointer`). A schema field has no selection set to read.
3. The protocol's `fetch_plan(E)` is `Ok`.

Rules 1 and 2 are the core's; rule 3 is the protocol's. For GraphQL, `fetch_plan(E)` is `Ok` when `E` is an object or interface and one of these holds, checked in this order, the first that applies deciding:

1. `E` is a root operation type: the operation is `query`, `mutation`, or `subscription` with the entrypoint's selections at the top level.
2. `E` is `Node` or implements `Node`: the operation is a `query` whose top-level selection is the query root's field `node`.

No other entity can parent an entrypoint (Later: fetch fields other than `node`).

Every field of the mutation and subscription root can thus parent an entrypoint, as every field of the query root can.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How the GraphQL protocol fetches an entity that parents an entrypoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphQLFetchPlan {
    /// The entity is a root operation type: its selections are the operation's.
    Root(GraphQLRootFetch),
    /// A `query` that selects the query root's `node` field, which returns the entity or an
    /// interface containing it.
    Node(GraphQLNodeFetch),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLRootFetch {
    pub operation_type: OperationType,
    /// The root type, whose data decides the top-level `id` and `__typename`.
    pub entity: MergedTarget<GraphQLEntityData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLNodeFetch {
    /// `node`'s arguments, each passed as the operation variable of the same name.
    pub arguments: Vec<GraphQLFetchArgument>,
    /// The type `node` returns, which the operation builder completes like any selection
    /// target.
    pub field_target: MergedTarget<GraphQLEntityData>,
    pub refinement: FetchRefinement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQLFetchArgument {
    pub name: InputValueName,
    /// The argument's type, made nullable when the argument has a default, so the caller
    /// may omit it (spec 5.8.5, All Variable Usages Are Allowed).
    pub variable_type: VariableType,
}

/// How the entity's selections sit under `node`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchRefinement {
    /// `node` returns the entity itself.
    Exact,
    /// `node` returns an interface or union that contains the entity:
    /// `... on Entity { }`. The entity's data decides the fragment's `id`.
    InlineFragment(MergedTarget<GraphQLEntityData>),
}
```

`OperationType` is the parser's (`crates/graphql_schema_parser/src/ast.rs`); it gains `strum::Display` with `serialize_all = "lowercase"` so the printer writes `query`, `mutation`, `subscription`. `MergedTarget` and `VariableType` are core types (query-generation.md).

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// How `entity` is fetched when it parents an entrypoint, as `client` reads the schema.
#[memo]
pub fn graphql_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<GraphQLFetchPlan, GraphQLNotFetchable>
```

The body:

1. `graphql_entity(db, client, entity)`: `Err` is `GraphQLNotFetchable::Entity`, `Ok(None)` is `UnknownEntity`, a variant other than `Object` or `Interface` is `NotObjectOrInterface`.
2. `root_operation_types(db, client)`, `Err` is `RootOperationTypes`. `entity` equal to `query`, `mutation`, or `subscription` is `Root` of that operation type, with `entity` and step 1's data as `entity`.
3. `entity` named `Node`, or `interfaces` containing `Node`: `node_fetch(db, client, roots.query, entity)`, `Err` is `NodeField`.
4. Otherwise `NoFetchRoot`.

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// The fetch of `entity` through the `node` field of the query root `query_root`.
fn node_fetch<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    query_root: EntityName,
    entity: MergedTarget<GraphQLEntityData>,
) -> Result<GraphQLNodeFetch, NodeFieldError>;
```

```rust
// from crates/graphql_network_protocol/src/fetch_plan.rs
/// Whether the interface or union `returns`, whose data is `returns_data`, contains `entity`.
fn contains<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    returns: EntityName,
    returns_data: &GraphQLEntityData,
    entity: &MergedTarget<GraphQLEntityData>,
) -> bool;
```

`graphql_fetch_plan`'s step 2 is `roots.operation_type_of(entity)`. `node_fetch` reads `graphql_network_protocol::selectable_named(db, client, query_root, "node")`. `Err` is `Selectable`, `Ok(None)` is `NotFound`. The field's type with `NonNull` removed must be `Named(T)`; a list is `ReturnsList`. `T == entity` is `Exact`. `T` an interface in `entity`'s `interfaces`, or a union whose member types contain `entity`, is `InlineFragment` of `entity` and its data; any other `T` is `TypeMismatch`; `contains` decides the interface and union cases, reading the union's members through `graphql_network_protocol::entity_named(T)`. `node_fetch` takes the entity's data from `graphql_fetch_plan`'s step 1 through its caller, as `entity: MergedTarget<GraphQLEntityData>` in place of `entity: EntityName`. `field_target` is `T` and `graphql_entity(T)`'s data (`Err` is `Target`, `Ok(None)` is `UnknownTarget`). Each argument definition becomes a `GraphQLFetchArgument` in definition order: its GraphQL type converted to `VariableType` (`TypeAnnotation::NonNull` to `VariableType::NonNull`, `Named` and `List` under it to `NonNullVariableType::Named` and `::List`), with the outer `NonNull` dropped when the argument has a default value.

`node`'s arguments are read from its definition, not assumed to be `id: ID!`. With pet demo's `node(id: ID!): Node` and `type Pet implements Node`, `graphql_fetch_plan(Pet)` is `Node(GraphQLNodeFetch { arguments: [id: ID!], field_target: Node (interface, id present), refinement: InlineFragment(Pet (object, id present)) })`.

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
    #[error("`{0}` cannot be fetched: only object and interface types can parent an entrypoint.")]
    NotObjectOrInterface(EntityName),
    #[error("`{0}` cannot be fetched: it is not a root operation type and does not implement `Node`.")]
    NoFetchRoot(EntityName),
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeFieldTypeMismatch {
    pub query_root: EntityName,
    pub returns: EntityName,
    pub entity: EntityName,
}
```

`GraphQLNetworkProtocol::fetch_plan` is `graphql_fetch_plan(db, client, entity).clone()`; `type FetchPlan = GraphQLFetchPlan`, `type NotFetchable = GraphQLNotFetchable`.

The memo reads `Pet`'s data, the root types, and for a `node` fetch the `node` field and its target type. Editing an unrelated type does not change any of those reads, so the memo backdates. Editing `type Query` re-runs every `node` fetch plan, because each reads `Query.node`.

## Validation

validation-diagnostics.md's `entrypoint_declaration_error` checks that `E` and `E.f` are defined. After this change it then checks rules 1 to 3.

```rust
// from crates/isograph_compiler/src/validation.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError<TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    UnknownEntity(UnknownEntityError),
    #[error("{0}")]
    UnknownSelectable(UnknownSelectableError),
    #[error("{0}")]
    DuplicateDefinition(DuplicateDefinitionError),
    #[error("{0}")]
    SelectionSetOnFieldWithoutTarget(SelectionSetOnFieldWithoutTargetError),
    #[error("{0}")]
    EntrypointNotFetchable(EntrypointNotFetchableError<TNotFetchable>),
}

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
    #[error("{0}")]
    Protocol(TNotFetchable),
}

pub type ProfileValidationError<TProfile> = ValidationError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;
```

`iso_literal_validation_errors_in_file` returns `Option<Vec<WithSpan<ProfileValidationError<TProfile>>>>`; the diagnostics code reads `error.item.to_string()` and is otherwise unchanged. `CompilerError`, which wraps a validation error, takes the same parameter:

```rust
// from crates/isograph_compiler/src/compiler_errors.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompilerError<TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    Parse(ParseError),
    #[error("{0}")]
    Validation(ValidationError<TNotFetchable>),
}

pub type ProfileCompilerError<TProfile> = CompilerError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;
```

`compiler_errors_in_file` returns `Option<Vec<WithSpan<ProfileCompilerError<TProfile>>>>`.

```rust
// from crates/isograph_compiler/src/validation.rs
/// `entrypoint E.f`: `E` and `E.f` are defined, `E` comes from the protocol, `E.f` is a
/// `field` declaration, and the protocol can fetch `E`.
fn entrypoint_declaration_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    declaration: &EntrypointDeclaration,
) -> Option<WithSpan<ProfileValidationError<TProfile>>> {
    // unchanged: UnknownEntity on `parent_type`, then UnknownSelectable on `name`
    let entity = entity_named(db, client, declaration.parent_type.item.0).as_ref().ok()?;
    let selectable =
        selectable_named(db, client, entity.name, declaration.name.item.0).as_ref().ok()?;
    let location = |error: &EntrypointNotFetchableError<_>| match error {
        EntrypointNotFetchableError::ProtocolSelectable(_) => declaration.name.location,
        EntrypointNotFetchableError::SourceEntity(_) | EntrypointNotFetchableError::Protocol(_) => {
            declaration.parent_type.location
        }
    };
    entrypoint_fetch_plan(db, client, entity, selectable).err().map(|error| {
        let span = location(error.reference());
        ValidationError::EntrypointNotFetchable(error).with_span(span)
    })
}
```

`entrypoint_fetch_plan` takes the entity and selectable its callers have already looked up. Taking the key instead would make the lookup failures a reason here, which the validation caller has already reported on its own spans.

```rust
// from crates/isograph_compiler/src/entrypoint.rs
/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
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

pub type ProfileFetchPlan<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::FetchPlan;
pub type ProfileEntrypointNotFetchableError<TProfile> = EntrypointNotFetchableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;
```

`entrypoint_declaration_error` calls `entity_named` and `selectable_named` (it already does, through `source_entity_definitions` and `source_selectable_definitions`; after change 3 it reads the core lookups so protocol entities count as defined), and on `Ok` of both calls `entrypoint_fetch_plan(db, client, &entity, &selectable)`. A duplicate `E` or `E.f` is reported at the declarations and the fetch check is skipped. `query_for_entrypoint` calls the same function (query-generation.md).

`UnknownEntity` and `UnknownSelectable` in validation-diagnostics.md read `source_entity_definitions` and `source_selectable_definitions` today. Change 3 switches the "is it defined" checks in `entrypoint_declaration_error`, `selectable_declaration_errors`, and `validate_selection` to "at least one source definition or a protocol definition", so `field Pet.x` with `type Pet` in the schema has no `UnknownEntity`. `DuplicateDefinition` counts source and protocol definitions together: `entity Pet` beside `type Pet` is a duplicate at the `entity Pet` name.

## Tests

`crates/graphql_network_protocol` (GraphQL profile with `TypeScriptHostLanguage` as a dev-dependency, schema and source files inserted before the first lookup per integration.md's Known gap):

- `root_operation_types`: pet demo's `schema { query: Query mutation: Mutation }` is `{ Query, Some(Mutation), None }`; no schema definition is `{ Query, Some(Mutation), Some(Subscription) }`; `schema { query: RootQuery }` plus `extend schema { subscription: S }` is `{ RootQuery, None, Some(S) }`; two `schema` definitions in two files is `DuplicateSchemaDefinition` with first from the earlier path; `extend schema { query: A }` with a definition naming `query` is `DuplicateOperation(Query)`; `schema { mutation: M }` is `MissingQuery`; `schema { query: }` is `Syntax`; a broken `type Broken { a: ! }` in the same file does not change the result.
- `graphql_entity`: `Pet` is `Object` with `interfaces == [Node, Namable]` and `id_field == Present`; `PetStats` is `Object` with `Absent`; a type with `id: String!` is `Absent`; `NewsfeedItem` is `Union`; `ICheckin` is `Interface` with `interfaces == [Node]`; `PetSpecies` and `ID` are `Leaf`; `SetPetTaglineParams` is `InputObject`.
- `graphql_fetch_plan` over the pet demo schema: `Query` is `Root` with operation type `Query` and `entity.name == Query`; `Mutation` is `Root` with `Mutation`; `Pet` is `Node` with one argument `id` of type `ID!`, refinement `InlineFragment(Pet)`, and `field_target.name == Node`; `ICheckin` (an interface implementing `Node`) is the same with `InlineFragment(ICheckin)`; `Node` itself is `Node` with `Exact`; `PetStats` is `NoFetchRoot`; `NewsfeedItem` is `NotObjectOrInterface`; `String` is `NotObjectOrInterface`; `Nobody` is `UnknownEntity`.
- `petByName(name: String!): Pet` on `Query` changes no plan: `Pet` stays `Node`, `PetStats` stays `NoFetchRoot`.
- With `node(id: ID!): [Node]`: `Pet` is `NodeField(ReturnsList(Query))`. With `node(id: ID!): Viewer`: `NodeField(TypeMismatch(..))` naming `Viewer` and `Pet`. With no `node` field: `NodeField(NotFound(Query))`. In each failing case, `graphql_fetch_plan(Query)` and `graphql_fetch_plan(Mutation)` are unchanged.
- `node(id: ID!, hint: Int = 0): Node` gives arguments `id` of type `ID!` and `hint` of type `Int` (nullable, because it has a default).
- With no `schema` definition and `type Subscription { petAdded: Pet }`, `Subscription` is `Root` with `Subscription`; with `schema { query: Query }`, a `type Mutation` is `NoFetchRoot`.
- A syntax error in `type Query` makes `Pet`'s plan `NodeField(Selectable(..))` and leaves `Mutation`'s plan `Root` with `Mutation`.

`crates/isograph_compiler` (test protocol):

- `entrypoint_fetch_plan` of a source entity is `SourceEntity`; with a source field on a source entity, `SourceEntity` comes first.

`crates/graphql_network_protocol` validation tests, which read `iso_literal_validation_errors_in_file` (GraphQL profile, pet demo schema and source files; `isograph_lsp` does not depend on `graphql_network_protocol`):

- The pet demo's `entrypoint Query.HomeRoute`, `entrypoint Query.PetDetailRoute`, `entrypoint Query.SpeciesRoute`, and `entrypoint Mutation.SetTagline` have no diagnostics.
- `entrypoint PetStats.Card` with `field PetStats.Card { weight }` has one diagnostic, on `PetStats`, whose message is the `NoFetchRoot` text.
- `entrypoint Owner.OwnerBadge` has one diagnostic, on `Owner`, the `SourceEntity` text.
- `entrypoint Query.pets` has one diagnostic, on `pets`, the `ProtocolSelectable` text.
- `entrypoint Pet.PetSummaryCard` has no diagnostic; changing `Query.node` to return `[Node]` gives one on `Pet` with the `ReturnsList` text, and `entrypoint Query.HomeRoute` still has none.

## Later

- Fetch fields other than `node`. A type that does not implement `Node` would be fetched through a query root field the schema names, for example a directive `extend type User @fetchable(field: "userByName")` on the fetched type, with the root field's arguments passed through as operation variables of the same names. Mapping an already-loaded instance's fields to that field's arguments belongs to the refetch design. Neither is in v0.
