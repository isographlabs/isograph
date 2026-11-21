# Runtime: resolver results in one graph

Status: under discussion. Not approved for implementation. This is the planned runtime step after v1 (`refactors/pending/runtime/store.md`, `refactors/pending/runtime/reader-ast.md`, `refactors/pending/runtime/network-and-subscriptions.md`). It describes what changes from the landed code in `libs/isograph-runtime/src`.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9b81e0b6`; `libs/isograph-react/src` last changed at `c09c9dbf`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`. pico paths are under `crates/pico/src` of this repo.

## Goal

Everything is graph. A server value is a source node in the store, and a resolver result is a derived node beside it in the same record container. Every edge is an object reference in both directions. A write sets a source node's value and marks the derived nodes above it, and runs no resolver. A read pulls: it brings a marked derived node up to date, and recomputes it only when an input it read changed value (short circuit). A resolver is called again on a pull only when its node is `Dirty`: a source node its last computation read was written with a value `normalize` found changed, a derived node it read recomputed to a value its `equal` rejected, the record container of a source node it read was deleted, a derived node it read was disposed, or the record that owns its node was deleted (the next read creates a new node).

The engine is independent of Isograph and usable on its own by other projects (the user's goal). The engine is the graph: source nodes, derived nodes, marks, marking, pull with short circuit, dependency recording, per-node equality, and disposal. It ships as its own package with no runtime dependencies (reactive-engine.md, The package), imports nothing from the Isograph runtime, and names no Isograph type in its API, so a project that has no store, no reader AST, and no resolver uses it with values of its own types. Everything Isograph-specific (record containers, absent records and `MISSING`, the reader walk and its hooks, resolver nodes and their keys, `normalize`'s marking, resolver result equality, record deletion, and garbage collection) lives in the Isograph runtime and calls the engine's public API (The engine). `refactors/pending/runtime/reactive-engine.md` is the engine's specification.

The core is a single graph with reads and short circuit. Everything that keeps values up to date without a read (observers, subscriptions as derived nodes, effects), retention, the collection of unreached derived nodes, and optimistic layers are later sections, built on the core and not part of its first implementation.

Upstream stores no resolver result. Its store is plain values per record (`StoreRecord`, `IsographEnvironment.ts:150-158`), a read calls every resolver it reaches (`read.ts:600`), and after a write `callSubscriptions` re-reads every subscription whose records overlap the written ones (`subscribe.ts:63-109`). The deviation is the user's: one structure for values and results, so that one layering mechanism serves Later: a DAG of graphs, and so that every edge is an identity edge.

## The running example

Two records, `root` and `other`; `root.2` holds the link to `other`. Two fields on `root`:

```text
FP reads: root.1, root.2, other.1, other.2, FC
FC reads: root.2, root.3, other.2, other.3
```

`root.2` is the source node of key `2` on record `root`. `FP` and `FC` are the derived nodes of those fields on `root`. `X.dependents` is the set of derived nodes whose last computation read `X`. The traces are in Traces, after the types.

## The engine

The engine is `refactors/pending/runtime/reactive-engine.md`. That doc is the single specification of the engine: its types and public functions (The public API), every function body (The implementation: Construction and plain reads, The guard, Mark, Pull, Dependencies, Disposal, Throws, Cycles), the package README, the tests, and the package files (The package). This doc does not restate any of it. It specifies the Isograph layer and the calls that layer makes into the engine. The package is `@isograph/reactive-graph` in `libs/reactive-graph` (open decision 18).

pico's shape: a source node holds a value that the outside world sets, and a derived node holds the value of a computation over source nodes and other derived nodes (`source.rs:83-86`, `derived_node.rs:55-58`, both kept by one database, `database.rs:78-80`). pico finds nodes by id and keeps forward edges only. The engine holds every edge as an object reference in both directions, and marks on write through the reverse edges, so a write costs the nodes it reaches, not the live graph (reactive-engine.md, Reverse edges, no epochs).

The engine's signatures depend on its open decision 2, which is not decided (reactive-engine.md open decision 2; this doc's open decision 2). Every engine call in this doc follows reactive-engine.md's form for whichever option is decided. This doc writes the calls in the (f) form of reactive-engine.md, The public API: `createSourceNode(value, equal)`, `createDerivedNode(computation, equal)`, `writeSource(store.graph, source, value)`, `markSourceChanged(store.graph, source)`, `pull(store.graph, node)`, `disposeDerivedNode(store.graph, node)`. Under (g), the held option, the constructors take `store.graph` first and the other four calls drop it (reactive-engine.md, Option (g)). The Isograph layer is otherwise the same under both.

### The engine and the Isograph runtime

The Isograph runtime imports the engine's package index and nothing else from it. No Isograph function reads or writes a node's `value`, `dependents`, `slot`, or a graph's `computingDepth`, and no Isograph function calls an engine function the index does not export (`stateOf`, `neverComputed`, `assertNotComputing`, `guarded`, `markDirty`, `markOneDirty`, `markCheck`, `verify`, `recompute`, `compute`, `setDependencies`, `dependentsOf`). The engine's public behavior is pico's: which values a read returns, which computations run again on a write, cutoff by equality, dependencies recorded per computation, and a write during a computation failing (open decision 2). Reverse edges and marks are internal, as pico's epochs are.

The calls the Isograph runtime makes, by export of the engine's index (reactive-engine.md, The public API):

- `createGraph`: `createStore` (The store).
- `createSourceNode`: `sourceNodeAt`, with `MISSING` and `isUnchanged(key)` (The store).
- `sourceValue`: `storeValue` and `storeSnapshot` (The store).
- `writeSource`: `normalize`'s `writeValue` (`normalize` writes source nodes), and `storeFrom` in `test/fixtures.ts`.
- `markSourceChanged`: `deleteRecord`, for each source node of the deleted container (The store).
- `createDerivedNode`: `resolverNodeAt`, with `resolverComputation(..)` and `resolverResultsEqual(artifact, identical)` (Stored resolver results).
- `pull`: `readStoredResult` (Stored resolver results).
- `disposeDerivedNode`: `deleteRecord`, for each derived node the deleted container owns (core change 8).
- `GraphWriteDuringComputationError`: `callResolver` rethrows it and catches every other throw into `ResolverThrew` (Stored resolver results, open decision 30).
- The types `Graph`, `SourceNode`, `DerivedNode`, `Computation`, and `ComputeContext`: `store.ts` and `storedResults.ts`.

Every export has a caller in the Isograph runtime. The computation of a resolver node reads only through its `ComputeContext` (`contextHooks`, Stored resolver results), so the runtime makes no untracked read inside a computation (open decision 25).

### Which layer owns each item

Engine (`libs/reactive-graph/src/graph.ts`), as reactive-engine.md's Types, The guard, and The implementation define them:

- Types: `Graph`, `SourceNode`, `DerivedNode`, `AnyDerivedNode`, `StateSlot`, `DerivedNodeState`, `CleanState`, `CheckState`, `DirtyState`, `PreviousValue`, `Dependency`, `Computation`, `ComputeContext`, `Computed`, `ContextStatus`.
- The marks `Clean`, `Check`, and `Dirty`.
- Construction and plain reads: `createGraph`, `createSourceNode`, `createDerivedNode`, `neverComputed`, `sourceValue`, `stateOf`.
- The guard: `Graph.computingDepth`, `assertNotComputing`, `guarded`, `GraphWriteDuringComputationError`.
- Mark: `writeSource`, `markSourceChanged`, `markDirty`, `markOneDirty`, `markCheck`.
- Pull: `pull`, `verify`, `recompute`, `compute` (the context, its `ContextStatus`, and dependency recording).
- Dependencies: `setDependencies`, `dependentsOf`.
- Disposal: `disposeDerivedNode`.
- Later: `rootCount` and `retainRoot` (Later: retention); the reachability walk `collect` needs, as a function over root nodes that returns the derived nodes they reach through `Derived` dependencies (Later: collecting derived nodes); `listen`, `notify`, and node effects with `reconcileEffects` (Later: keeping values up to date automatically); layers, worlds, `stateOf(node, world)`, and source overrides read through `readSource` and `sourceValue` (Later: a DAG of graphs).

Isograph runtime (`libs/isograph-runtime/src`):

- `store.ts`: `Store`, `createStore`, `RecordContainer`, `RecordPresence`, `StoreSourceNode`, `ResolverNode`, `MISSING`, `Missing`, `recordContainer`, `containerAt`, `presentContainerAt`, `sourceNodeAt`, `storeValue`, `recordPresence`, `storeSnapshot`, `PlainStore`, `deleteRecord`.
- `read.ts`: `ResolverResult`, `ReadResolverOk`, `OkOrigin`, `ReadResolverThrew`, `ReadHooks`, `readData`, `readStoreValue`, `readButDoNotEvaluate`, `evaluatingHooks`, `read`, `readSelection`, `linkTargetValue`, `eachLink`, `nested`.
- `artifacts.ts`: the reader AST (`ReaderSelection`), `EntityTable`, `EntityTables`, `FieldEntry`, `EntityShape`, `RecordStorage`, `NormalizationTarget`, and `NormalizationAst.tables`. `store.ts`: `VALUE_KEY`. `read.ts`: `EMPTY_DATA`, `readAt`, `fieldOf`, `tableOf`, `derivedLink`. `recordId.ts`: `omittedRecordId`, `recordId` over `RecordStorage`. `normalize.ts`: `leafRecords`, `storageOf`, `Normalization.tables`.
- `storedResults.ts`: `readStoredResult`, `resolverNodeAt`, `resolverComputation`, `contextHooks`, `callResolver`, `resolverResultsEqual`; later `collect`, which iterates the containers' `derived` maps, calls the engine's reachability walk, and calls `disposeDerivedNode` and removes each unreached node from its container.
- `storeKey.ts`: `resolverStoreKey`. `artifacts.ts`: `ReaderArtifact.parentEntityName`, `ReaderArtifact.selectableName`, `ReaderArtifact.tables`.
- `normalize.ts`: `writeValue`. `store.ts`: `isUnchanged`, `leafUnchanged`, `compositeUnchanged`.
- `check`, `garbageCollect` (mark and sweep), `test/fixtures.ts`'s `storeFrom`, the subscription re-read (`readSubscription`, `storedResultHooks`, `callSubscriptions`, `subscriptionResultsAreEqual`).
- Later: `retention.ts` (`Retention`, `retainEntrypoint`, `dispose`, `Entrypoint`), the fetch decision, subscriptions as derived nodes, React's `useSyncExternalStore` hook, and `normalize` writing into a layer.

### What resists the split

- `stateOf` and layering. Later: a DAG of graphs gives a node one slot per world by changing `stateOf`, and a source one value per world. That holds across the package boundary only if the Isograph runtime never reads a node's fields, so the runtime reads a source through `readSource` and `sourceValue` and never calls `stateOf`, which the package does not export. Layers are then the engine's (Later: a DAG of graphs).
- The write-during-computation guard. It is engine state, because a standalone consumer has no store (reactive-engine.md, The guard). The Isograph `Store` holds the engine `Graph` beside its records under both (f) and (g) (open decision 2).
- Equality defaults. The engine has none: every node's `equal` is a constructor argument (reactive-engine.md open decision 6). `===` is the Isograph layer's default output equality inside `resolverResultsEqual`, which also reads the reader AST for walk-assembled data, so both stay Isograph's (open decision 20).
- One value type per graph. A standalone engine has nodes of different types, so `TValue` is per node and the links hold an erased node type (reactive-engine.md, Types; open decision 19). In the Isograph runtime, a resolver node is a `DerivedNode<ResolverResult<unknown>>` and a store source node a `SourceNode<StoreValue | Missing>` (The store).
- Throws. A computation must not throw (reactive-engine.md, Throws). The engine stores no error and catches nothing; the Isograph layer keeps the contract by catching the resolver's own throw into `ResolverThrew`, except the engine's `GraphWriteDuringComputationError`, which it rethrows (Stored resolver results, open decision 30).
- Field visibility. A consumer outside this repo can write a node's fields; open decision 21.
- Construction. `sourceNodeAt` calls `createSourceNode` and `resolverNodeAt` calls `createDerivedNode`, so the Isograph runtime builds no engine object by hand.

### The engine package and the runtime

The package's files, its two pre-commit hooks, and its CI step are reactive-engine.md, The package. Core change 4 ships them. Core change 5 adds the runtime's dependency on the engine:

```json
// from libs/isograph-runtime/package.json
  "dependencies": {
    "@isograph/disposable-types": "workspace:*",
    "@isograph/reactive-graph": "workspace:*"
  },
```

The runtime resolves the engine through `node_modules` to its `dist`, as it resolves `@isograph/disposable-types`: `tsconfig.build.json`'s `"@isograph/*": ["./libs/*"]` maps to a directory without the `isograph-` prefix, which does not exist. turbo's `tsc` task depends on `^compile-libs` (`turbo.json`), so the engine is built before the runtime is type-checked. In the same change the runtime's three pre-commit hooks match the engine's directory (reactive-engine.md, Pre-commit hooks).

## The store

The store is the set of source nodes. A record is a container that holds its source nodes by store key and the derived nodes it owns by resolver store key.

```ts
// from libs/isograph-runtime/src/store.ts
import { createGraph, createSourceNode, type DerivedNode, type Graph, type SourceNode, sourceValue } from '@isograph/reactive-graph';

/** The graph and every record container. */
export type Store = {
  /** Passed to every engine write and pull on this store's nodes. */
  readonly graph: Graph;
  /** Every record container: `EntityName`, then record id. */
  readonly records: Map<EntityName, Map<RecordId, RecordContainer>>;
};

/** A store with no record. Caller: `createIsographEnvironment`. */
export function createStore(): Store {
  return { graph: createGraph(), records: new Map() };
}

/** One record's source nodes, and the derived nodes computed on it, which it owns. */
export type RecordContainer = {
  /** Read by `check`, the garbage collector's mark, and `storeSnapshot`; never by a computation. */
  presence: RecordPresence;
  /** Server values by store key. */
  readonly sources: Map<StoreKey, StoreSourceNode>;
  /** Resolver derived nodes by resolver store key. */
  readonly derived: Map<StoreKey, ResolverNode>;
};

/** `Present`: `normalize` wrote the record. `Absent`: a graph read created the container to hold a
 * source node or derived node of a record the store does not hold. */
export type RecordPresence = 'Present' | 'Absent';

export type StoreSourceNode = SourceNode<StoreValue | Missing>;
export type ResolverNode = DerivedNode<ResolverResult<unknown>>;

/** A source node's value when the record holds no value under its key: missing data. */
export const MISSING: unique symbol = Symbol('Missing');
export type Missing = typeof MISSING;
```

Before (`libs/isograph-runtime/src/store.ts:4-8`):

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = Map<EntityName, Map<RecordId, StoreRecord>>;
export type StoreRecord = Map<StoreKey, StoreValue>;
```

`records` is the user's `Map<EntityName, Map<RecordId, Map<StoreKey, SourceNode>>>` with two additions. `derived` puts a resolver derived node in the container of the record it was computed on, so that container is its owner: deleting the record disposes it with no `owned` map and no id table. Server keys and resolver store keys are separate maps, so a server field and a client field of one name do not collide. `presence` keeps v1's distinction between a record `normalize` wrote with no values (EmptyStats's `stats {}` writes an empty `PetStats` record, `test/normalize.test.ts:270`) and no record, for the readers that are not computations. `MISSING` is a symbol, so no JSON value equals it, and `=== MISSING` narrows with no cast.

There is no `SourceId`, no `DerivedNodeId`, no `sourceDependents` map, and no `RecomputationGraph` object. The environment's `store` is the whole graph: `graph` is the guard every engine write on its nodes passes, and `records` holds the nodes. `createIsographEnvironment` calls `createStore` where it built `new Map()`. The store builds every node with the engine's `createSourceNode` and `createDerivedNode` and reads a source's value with `sourceValue` or `ComputeContext.readSource`; no store function reads or writes a node's fields.

### Accessors

Every store consumer reads through these. No function outside `store.ts`, `normalize.ts`, and `storedResults.ts` touches `sources`, `derived`, or `presence`.

```ts
// from libs/isograph-runtime/src/store.ts
/** The container at `link`, or `undefined`. */
function recordContainer(store: Store, link: StoreLink): RecordContainer | undefined {
  return store.records.get(link.entityName)?.get(link.id);
}

/** The container at `link`, inserted `Absent` when the store has none. */
export function containerAt(store: Store, link: StoreLink): RecordContainer {
  return getOrInsert(getOrInsert(store.records, link.entityName, () => new Map()), link.id, () => ({
    presence: 'Absent',
    sources: new Map(),
    derived: new Map(),
  }));
}

/** The container at `link`, made `Present`. `normalize`'s `recordAt`. */
export function presentContainerAt(store: Store, link: StoreLink): RecordContainer {
  const container = containerAt(store, link);
  container.presence = 'Present';
  return container;
}

/** The source node of `key` on `link`, inserted with `MISSING` (and its container `Absent`) when
 * absent. Callers: `normalize`'s `writeValue` and the graph read's `readValue` hook. */
export function sourceNodeAt(store: Store, link: StoreLink, key: StoreKey): StoreSourceNode {
  return getOrInsert(containerAt(store, link).sources, key, () => createSourceNode(MISSING, isUnchanged(key)));
}

/** The equality of the source node of `key`: `leafUnchanged` for a scalar record's `__value`, which
 * holds the response's value, and `compositeUnchanged` for every other key, which holds links. `MISSING`
 * equals only `MISSING`. */
function isUnchanged(key: StoreKey): (previous: StoreValue | Missing, next: StoreValue | Missing) => boolean {
  const unchanged = key === VALUE_KEY ? leafUnchanged : compositeUnchanged;
  return (previous, next) =>
    previous === MISSING || next === MISSING ? previous === next : unchanged(previous, next);
}

/** `written` a list: `stored` is a list of the same length whose items are `leafUnchanged` in order, at
 * any depth. Otherwise `stored === written`. */
function leafUnchanged(stored: StoreValue, written: StoreValue): boolean {
  if (!isArray(written)) {
    return stored === written;
  }
  return (
    isArray(stored) &&
    stored.length === written.length &&
    written.every((item, index) => leafUnchanged(stored[index], item))
  );
}

/** `null`: `stored` is `null`. A link: a link with the same `entityName` and `id`. A list: a list of the
 * same length whose items are `compositeUnchanged` in order. */
function compositeUnchanged(stored: StoreValue, written: StoreValue): boolean;

/** The value of `key` on `link`, or `MISSING`; inserts nothing and records nothing. Callers: v1's read
 * hooks, `check`, the garbage collector's mark, `storeSnapshot`. */
export function storeValue(store: Store, link: StoreLink, key: StoreKey): StoreValue | Missing {
  const source = recordContainer(store, link)?.sources.get(key);
  return source === undefined ? MISSING : sourceValue(source);
}

/** `Absent` when the store has no container at `link`. Callers: `check`, the garbage collector's mark. */
export function recordPresence(store: Store, link: StoreLink): RecordPresence {
  return recordContainer(store, link)?.presence ?? 'Absent';
}

/** The store's server data as plain values: every `Present` record with every value that is not
 * `MISSING`. Callers: serialization, and tests comparing a store. */
export function storeSnapshot(store: Store): PlainStore;

/** v1's `Store` shape, as data. */
export type PlainStore = Map<EntityName, Map<RecordId, Map<StoreKey, StoreValue>>>;

/** Marks every dependent of each source node of the container at `link`, disposes each derived node
 * it owns, and removes the container. Caller: `garbageCollect`'s sweep. */
export function deleteRecord(store: Store, link: StoreLink): void;
```

`storeValue` and `recordPresence` insert nothing, so `check`, the mark, a v1 read, and a snapshot never grow the store.

What each consumer becomes:

- `read` and `readButDoNotEvaluate` (v1): the walk's `readValue` hook is `storeValue` (The reader walk).
- The graph read: `readValue` is `sourceNodeAt`, reported to the computation (The reader walk).
- `check`: `storeRecord(store, link) === undefined` becomes `recordPresence(store, link) === 'Absent'` (`libs/isograph-runtime/src/check.ts:30`, `:54`), and `record.get(key)` becomes `storeValue(store, link, key)` with `=== MISSING` for `=== undefined` (`:49-50`). Its results are v1's for every input.
- The garbage collector's mark: the same two replacements (`libs/isograph-runtime/src/garbageCollection.ts:51`, `:70-73`), and one change: a link reached to an `Absent` container keeps that container, and the mark does not recurse into it (Garbage collection in the core). The sweep calls `deleteRecord`.
- `readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:196-218`): calls `read`, unchanged (open decision 6).
- Serialization: `storeSnapshot`.
- Test fixtures: `storeOf` (`libs/isograph-runtime/test/fixtures.ts`) returns a `PlainStore`; a test compares `storeSnapshot(store)` with it. A test that needs a store built from data calls a helper in `test/fixtures.ts`, `storeFrom(plain: PlainStore): Store`, which calls `presentContainerAt` and `writeSource(store.graph, sourceNodeAt(..), value)` for each value. A test that corrupted a stored value (`environment.store.get('Pet')?.get('1')?.set('bestFriend', 'x')`) calls `writeSource(store.graph, sourceNodeAt(store, pet('1'), 'bestFriend'), 'x')`; one that deleted a record calls `deleteRecord`; one that deleted a key builds its store with `storeFrom` without that key. No API exists for tests alone.

### Absent records

Record presence is not a source. An absent record reads as a record whose every key is missing: the walk asks for the source node of the first key it tries, `sourceNodeAt` creates it with `MISSING` in an `Absent` container, the computation records it, and the read is `MissingData` naming that key. When `normalize` later writes the record, it finds that container and that source node, and writing the key marks the reader. A reader whose key the write does not set stays `MissingData`, which is correct. Deleting a record marks the dependents of each of its source nodes, `MISSING` ones included, and disposes the derived nodes it owns; ownership is the container's `derived` map.

A resolver selected on an absent record (a parent reads a link to `Pet:9`, and `Pet:9` is absent): the child node is created in `Pet:9`'s `Absent` container, reads its keys, and gets `MissingData`. It is owned by the absent record and is disposed when the garbage collector deletes that container.

`noRecord` and `MissingData` are one result. v1 builds "No record for X" at `readButDoNotEvaluate` and `readStoreValue` (`libs/isograph-runtime/src/read.ts:68-74`, `:184-191`); both checks go, and the walk reports "No value for K on X". Two consequences, for every walk (v1's and the graph's):

- `read.test.ts`'s "a read of an absent root record is missing data on the root" and "an absent root is missing data on the root" assert `reason: 'No value for pet____id___"1" on Query:__ROOT'`.
- A reader AST that reads no key on an absent record is `Ok`. "a field with an empty reader AST is missing data on an absent record" (`test/read.test.ts`) becomes: `Constant` on the absent `Pet:9` is `Ok` with `data: {}` and its resolver is called. v1 already reads an empty linked selection on an absent record as `Ok` (`readLinkedValue` checks no presence, `read.ts:207-229`); now a resolver's root does the same. Upstream checks presence at every record it reads (`read.ts:161-167`); the deviation follows from presence not being a source: a read that reads no key has nothing to depend on, so a `MissingData` it returned could never be marked when the record appears.

`check` and the fetch decision. `check` reads presence through `recordPresence` and is not a computation, so its results do not change. The two places it reads presence (`check.ts:30-34`, `:54-57`) yield the same link a missing first key would, except for an empty selection set. The fetch decision of Later: retention and the fetch decision reads the root derived node instead, so it fetches less than `check` in two cases: a reader that reads no key on an absent record, and a record a partial write left without its `id`. In both, every value the readers read is present.

### Equality

Each derived node has an equality function, used for cutoff: `recompute` keeps the previous value object and marks no dependent when `equal(previous, next)` holds. The engine has no default (open decision 20): each resolver node's equality is `resolverResultsEqual`, whose output equality is `===` unless the field supplies one (open decision 7).

Correctness: an equality function only decides whether the dependents recompute. One that answers unequal for equal values makes them recompute from equal inputs to their old values, which costs work and never yields a wrong value. One that answers equal for unequal values yields a wrong value, so every rule other than `===` must answer equal only when every position a reader can observe is `===`.

The resolver node's equality:

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** Kinds differ: unequal. `MissingData`: equal `reason`, `recordLink` entity and id, and `nestedReason`,
 * recursively. `ResolverThrew`: errors `===`. `Ok`: both `WalkData`, `selectionSetDataAreEqual` over the
 * artifact's reader AST and tables; otherwise `outputEqual`. */
export function resolverResultsEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  outputEqual: (previous: unknown, next: unknown) => boolean,
): (previous: ResolverResult<unknown>, next: ResolverResult<unknown>) => boolean;
```

- A resolver's return value: `outputEqual`, `===` unless the field supplies one (open decision 7). A primitive or a reused object cuts off; a new object marks the dependents.
- Objects the runtime itself creates get a structural rule. `MissingData` is built from strings and links. A fragment's data object is assembled by the walk (`readSelectionSet`, `read.ts:101-121`); when a resolver returns that object (`({ data }) => data`, as every store fixture resolver does), the result is tagged `WalkData` and two such results compare by `selectionSetDataAreEqual(tables, entity, selections, previous, next)` (`libs/isograph-runtime/src/areEqualWithDeepComparison.ts:9-67`), which reads each selection's entry in the entity's table as the walk does: a `Derived` entry with target `null` compares by `===`; a `Source` entry, a `Derived` entry with a target, and `__self` compare `null` to `null`, lists by length and item at any depth, and each linked record's value by its selections recursively (at the entry's target) or, with `selections: null`, by `===` (a scalar's `__value`, or `EMPTY_DATA`). That is upstream's `mergeObjectsUsingReaderAst` rule (`areEqualWithDeepComparison.ts:55-121`) without the recycling. So `{ initials }` assembled by the walk compares equal when `initials` is the same string, a link moved to a record with equal data cuts off, and an opaque derived output is never compared structurally.
- Source writes: each store source node compares a written value with the stored one by `isUnchanged` (The store): `leafUnchanged` for `__value` and `compositeUnchanged` for every other key, as upstream does (`normalizeScalarField`, `cache.ts:232-249`; `normalizeLinkedField` and `dataIdsAreTheSame`, `:255-393`).

```ts
// from libs/isograph-runtime/src/read.ts
/** A stored resolver result: `Ok`, `MissingData`, or the resolver threw. */
export type ResolverResult<T> = ReadResolverOk<T> | ReadMissingData | ReadResolverThrew;

/** The resolver returned `value`. */
export type ReadResolverOk<T> = {
  readonly kind: 'Ok';
  readonly value: T;
  /** `WalkData`: `value` is the data object the walk assembled for this call. `ResolverValue`: any other value. */
  readonly origin: OkOrigin;
};

export type OkOrigin = 'WalkData' | 'ResolverValue';

/** The resolver of `selectableName` on `recordLink` threw `error`. */
export type ReadResolverThrew = {
  readonly kind: 'ResolverThrew';
  readonly selectableName: string;
  readonly recordLink: StoreLink;
  readonly error: unknown;
};
```

`ReadResolverOk<T>` is assignable to v1's `ReadOk<T>`, so a parent's walk reads `.value` from either. v1's `ReadResult` is unchanged. Returned values are shared, not copied, as upstream's are (`read.ts:616-634`): resolvers and callers must not mutate a stored value or a stored result, because a mutation changes it with no write and marks nothing.

A separate future exploration, `refactors/pending/runtime/fragment-getters.md`, considers exposing fragment data through getters; this doc materializes fragment data.

## The reader walk

The walk reads through hooks its caller passes. One walk serves v1's reads and the graph's.

```ts
// from libs/isograph-runtime/src/read.ts
/** How the walk reads a value and a nested resolver. `TFailure` is what a nested resolver's result
 * can be besides `Ok` and `MissingData`. */
export type ReadHooks<TFailure> = {
  /** The value of `key` on `record`, or `MISSING`. */
  readonly readValue: (record: StoreLink, key: StoreKey) => StoreValue | Missing;
  /** The result of `artifact` on `root` with `variables`. */
  readonly readResolver: (
    artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
    root: StoreLink,
    variables: Variables,
  ) => ReadResult<unknown> | TFailure;
};

/** `artifact`'s reader AST read at `root` with `variables` through `hooks`. `artifact`'s own resolver
 * is not called. */
export function readData<TFailure>(
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure;
```

`readData` is v1's `readButDoNotEvaluate` body without the presence check (`read.ts:68-74`). v1's `readSelectionSet`, `readSelection`, `readLinkedValue`, and `allOk` take `hooks` where they took `store` and return `ReadResult<..> | TFailure`; `allOk` stops at the first result whose `kind` is not `Ok`, so a `TFailure` passes up unchanged. `readStoreValue`:

```ts
// from libs/isograph-runtime/src/read.ts
// before:
function readStoreValue(store: Store, root: StoreLink, key: StoreKey): ReadResult<StoreValue> {
  const record = storeRecord(store, root);
  if (record === undefined) {
    return { kind: 'MissingData', reason: `No record for ${recordName(root)}`, recordLink: root };
  }
  const value = record.get(key);
  if (value === undefined) {
    return { kind: 'MissingData', reason: `No value for ${key} on ${recordName(root)}`, recordLink: root };
  }
  return ok(value);
}
// after:
function readStoreValue<TFailure>(hooks: ReadHooks<TFailure>, root: StoreLink, key: StoreKey): ReadResult<StoreValue> {
  const value = hooks.readValue(root, key);
  if (value === MISSING) {
    return { kind: 'MissingData', reason: `No value for ${key} on ${recordName(root)}`, recordLink: root };
  }
  return ok(value);
}
```

The `Resolver` case calls `hooks.readResolver(selection.readerArtifact(), root, childVariables(variables, selection.arguments))` where it called `read(store, ..)` (`read.ts:164-174`). Core change 6 replaces the four cases with the walk of Selections in the Isograph layer, whose `Derived` entries call `hooks.readResolver` with the artifact the entity table names.

v1's exports keep their signatures through v1's hooks:

```ts
// from libs/isograph-runtime/src/read.ts
export function readButDoNotEvaluate<TEntityName extends EntityName>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<SelectionSetData> {
  return readData(evaluatingHooks(store), artifact, root, variables);
}

/** v1's reads: nothing recorded, each nested resolver called by `read`, its throw propagating. */
function evaluatingHooks(store: Store): ReadHooks<never> {
  return {
    readValue: (record, key) => storeValue(store, record, key),
    readResolver: (artifact, root, variables) => read(store, artifact, root, variables),
  };
}
```

`read` keeps its body. `TFailure` is `never`, so `read` and `readButDoNotEvaluate` keep their return types.

## Selections in the Isograph layer

Every selection follows a link to an entity's record, and scalars are entities (`docs-website/docs/design-docs/mental-model.md:5`: there is one kind of entity). A scalar field's value is its own record: `Pet:1.name` holds a link to the `String` record `Pet:1.name`, whose source node `__value` holds the string. So every position the walk reads at is a record link, and a derived node computed on a scalar lives in the scalar record's container. The compiler side (validation, query text, type generation, entity tables, generated refinement fields) is `refactors/pending/language/linked-only-selections.md`. The engine is unchanged.

Two structures carry what the runtime needs, and they stay separate:

- The response normalization AST, one per operation, shaped by the server fields the query fetches, as upstream's is (`NormalizationAst`, Scalar records).
- The entity table, one per entity, shared by every operation and every reader, generated from the schema and the client field declarations (`EntityTable`, Entity tables).

### The reader AST

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderSelectionSet = ReadonlyArray<ReaderSelection>;

/** One selection: the field `fieldName` of the entity the walk reads the record as, under `alias`.
 * `selections` is read at each record the value links to; `null` when the selection has no selection
 * set. */
export type ReaderSelection = {
  readonly alias: string;
  readonly fieldName: string;
  readonly arguments: Arguments | null;
  readonly selections: ReadonlyArray<ReaderSelection> | null;
};
```

Before (`libs/isograph-runtime/src/artifacts.ts:144-193`): `ReaderSelection` is the union of `ReaderScalarSelection` (`kind: 'Scalar'`), `ReaderLinkedSelection` (`kind: 'Linked'`, `selections`), `ReaderRefinementSelection` (`kind: 'Refinement'`, `condition`, `selections`), and `ReaderResolverSelection` (`kind: 'Resolver'`, `readerArtifact`). The mapping: `Scalar` is a selection with `selections: null`; `Linked` keeps its `selections`; `Refinement` is a selection of the generated refinement field (`asPet`) with its `selections`; `Resolver` is a selection with `selections: null` whose field the entity table lists as `Derived`. `selectableName` is `fieldName`, upstream's name for it (`ReaderScalarField.fieldName` and `ReaderLinkedField.fieldName`, `../isograph/libs/isograph-react/src/core/reader.ts`). `Condition` stays, for the normalization AST.

The AST carries no kind, no artifact import, and no target. Whether a field is a source or derived, and which entity its value links to, are properties of (entity, field name), and the walk reads them from the entity table. The compiler rejects a selection set on a client field's opaque output (`fullName { toUppercase }`, linked-only-selections.md open decision 2 (b)); the AST type does not encode that rule, and the walk relies on the compiler.

`storeKey` takes the name and the arguments: `storeKey(name: string, args: Arguments | null, variables: Variables): StoreKey`, so `normalize`, `check`, and the mark pass `selection.selectableName` and the walk passes `selection.fieldName` (`libs/isograph-runtime/src/storeKey.ts:13-21`).

The index exports `ReaderSelection`, `ReaderSelectionSet`, `EntityTable`, `FieldEntry`, `EntityShape`, `RecordStorage`, `EMPTY_DATA`, `NormalizationTarget`, and `NormalizationLeafTarget`, and stops exporting `ReaderLinkedSelection`, `ReaderRefinementSelection`, `ReaderResolverSelection`, `ReaderScalarSelection`, and `NormalizationIdentity` (`libs/isograph-runtime/src/index.ts`).

### Entity tables

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** What every reader and every normalization of one entity needs: each field's origin, and how a
 * record of the entity gets its id. The default export of an `entity_table.ts` artifact is a function
 * returning it. */
export type EntityTable = {
  /** Every field of the entity, by field name. */
  readonly fields: { readonly [fieldName: string]: FieldEntry };
  /** What a selection of the entity with no selection set reads. */
  readonly shape: EntityShape;
  /** How `normalize` finds the id of a record of this entity. */
  readonly storage: RecordStorage;
};

/** Where a field's value comes from. */
export type FieldEntry =
  /** A server field: the source node under its store key holds a link to a `target` record, `null`, or
   * a list of these. */
  | { readonly kind: 'Source'; readonly target: EntityName }
  /** A client field: the value of its derived node. `target` is the entity its value links to (a
   * refinement, a client pointer), or `null` for a client field whose value is opaque. */
  | {
      readonly kind: 'Derived';
      readonly target: EntityName | null;
      readonly artifact: () => ReaderArtifact<ResolverParam, unknown, EntityName>;
    };

/** `Leaf`: a scalar entity; a bare selection reads the record's `__value`. `Composite`: a bare
 * selection reads nothing and is `EMPTY_DATA`. */
export type EntityShape = 'Leaf' | 'Composite';

export type RecordStorage =
  /** The response object's value under `selectableName` is the record id. */
  | { readonly kind: 'OwnId'; readonly selectableName: string }
  /** The record has no identity of its own: its id is `omittedRecordId` of its position. */
  | { readonly kind: 'FromParent' }
  /** Every record of the entity is the record `id`. */
  | { readonly kind: 'Fixed'; readonly id: string };
```

Derived or source is decided at compile time per (entity, field name), and records of one entity never differ: `Pet.name` is a source on every `Pet` record. Two entities are two sets of fields even when one record is read as both: `Actor.nickname` and `User.nickname` are different fields, each a source or derived on its own, and `User:4` read through `actor` is read as `Actor` while `actor { asUser { .. } }` reads it as `User`. A per-record choice is expressible without a trap door: a derived field that reads a source under another name (`field User.displayName { nickname login }` returning `data.nickname ?? data.login`) varies per record by data, and the table stays static.

`RecordStorage` is what the normalization AST carried per selection as `NormalizationIdentity` (`Select`, `Omit`, `Fixed`, `libs/isograph-runtime/src/artifacts.ts:54-62`); it is a property of the entity, so it moves to the table. `OwnId` keeps `Select`'s `selectableName`: the identity selection is the protocol's (`IdentitySelection::Select(name)`, `crates/graphql_network_protocol/src/print_operation.rs:453`), and dropping the name would hard-code `id` in the runtime. Scalars and objects with no identity are `FromParent` records: `Pet:1.name`, `Pet:1.stats`, and in v0 `date { hour }` is the `FromParent` record `Pet:1.date` with its own container (Follow-ups 11).

`shape` exists because a bare selection must know whether the record holds a value: a bare `name` reads `__value` and a bare `bestFriend` reads nothing. It is a property of the entity (`EntityShape`, `crates/isograph_compiler/src/schema.rs`), and the alternative of reading `__value` and treating `MISSING` as an object is open decision 29.

Generated modules, one per entity, at `<Entity>/entity_table.ts`:

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
    bestFriend: { kind: 'Source', target: 'Pet' },
    checkins: { kind: 'Source', target: 'Checkin' },
    id: { kind: 'Source', target: 'ID' },
    name: { kind: 'Source', target: 'String' },
    nickname: { kind: 'Source', target: 'String' },
    stats: { kind: 'Source', target: 'PetStats' },
    tagline: { kind: 'Source', target: 'String' },
    Constant: { kind: 'Derived', target: null, artifact: Pet__Constant__resolver_reader },
    NullableCheckins: { kind: 'Derived', target: null, artifact: Pet__NullableCheckins__resolver_reader },
    PetLink: { kind: 'Derived', target: null, artifact: Pet__PetLink__resolver_reader },
    PetName: { kind: 'Derived', target: null, artifact: Pet__PetName__resolver_reader },
    RecentCheckins: { kind: 'Derived', target: null, artifact: Pet__RecentCheckins__resolver_reader },
    ThrowingName: { kind: 'Derived', target: null, artifact: Pet__ThrowingName__resolver_reader },
  },
  shape: 'Composite',
  storage: { kind: 'OwnId', selectableName: 'id' },
});
export default entityTable;
```

The `Node` table lists `id`, `NodeLink`, and each generated refinement field a reader selects on `Node` (`asPet: { kind: 'Derived', target: 'Pet', artifact: Node__asPet__resolver_reader }`). The `Query` table's storage is `{ kind: 'Fixed', id: '__ROOT' }`.

Who imports a table:

- A reader artifact imports the table of every entity its own walk reads at: its parent entity, and the target of every selection whose value the walk reads at (a selection set, or a bare selection, whose `shape` the walk reads). `Query/PetDetailPage/resolver_reader.ts` imports `Query`, `Pet`, and `String`. A nested derived field's artifact brings its own tables: the walk calls `entry.artifact()` and reads that field's reader AST with that artifact's `tables`, so `String/toUppercase/resolver_reader.ts` imports `String`'s table itself, and the outer artifact imports a table only for the records its own reader AST visits.
- A normalization AST imports the table of every entity whose records `normalize` writes with it, for `storage`: the target entity of each `Known` composite target and each leaf, each entity a `Discriminator` target's response can name, and each conditional's entity.
- A table imports the reader artifact of each of its derived fields.

Every table and artifact module default-exports a function, and every reference to an imported binding is inside a function body (the table's body, the reader artifact's body, and `() => Pet__entity_table()` in the normalization AST, printed as `ImportCallThunk`, `crates/typescript_artifacts/src/module.rs:44-57`). Tables import reader artifacts, reader artifacts import tables and user source files, and user source files import entrypoints, which import normalization ASTs, which import tables; reading a binding at module evaluation would throw in that cycle. Cost: the walk calls a table's function once per selection set it reads, which allocates the table object each time, as `selection.readerArtifact()` allocates the reader artifact today.

### Refinements

A refinement `asPet` on `Node` is a derived field the compiler generates on `Node` with target `Pet` (linked-only-selections.md, Generated refinement fields). Its reader AST is empty and its resolver is `({ link }) => (link.entityName === 'Pet' ? { entityName: 'Pet', id: link.id } : null)`: a link's `entityName` is the `__typename` `normalize` stored the record under (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`), so the resolver reads nothing. When the value is a link, the walk continues at it as `Pet`. Its node on `Pet:1` is keyed `Node.asPet/{}` in `Pet:1`'s `derived`, has no dependencies, is computed once and never marked, and is disposed with its record. Cost: one node per record per selected refinement, and one edge per reader.

`__self` is not in a table. It is a field of every entity whose value is the current record's link, read as the same entity, so `__self { name }` at `Pet:1` reads `name` at `Pet:1` as `Pet`, and a bare `__self` at a scalar record reads `__value`. The walk recognizes the name (open decision 27).

### The walk

```ts
// from libs/isograph-runtime/src/read.ts
/** The tables of the entities one reader artifact's walk reads at, by entity name. */
type EntityTables = { readonly [entityName: string]: () => EntityTable };

/** What a bare selection of a composite entity reads: one frozen empty object, so that two reads
 * compare `===`. */
export const EMPTY_DATA: Readonly<Record<PropertyKey, never>> = Object.freeze({});

export function readData<TFailure>(
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure {
  return readSelectionSet(hooks, artifact.tables, artifact.readerAst, root, artifact.parentEntityName, variables);
}

/** `selections` read at `record` as `entity`: each selection's value under its alias; the first result
 * that is not `Ok` ends the read. */
function readSelectionSet<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  selections: ReaderSelectionSet,
  record: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure;

function readSelection<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  table: EntityTable,
  selection: ReaderSelection,
  record: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<unknown> | TFailure {
  const field = fieldOf(table, entity, selection.fieldName);
  switch (field.kind) {
    case 'Self':
      return nested(selection.alias, record, readAt(hooks, tables, selection.selections, record, entity, variables));
    case 'Source': {
      const key = storeKey(selection.fieldName, selection.arguments, variables);
      const value = readStoreValue(hooks, record, key);
      if (value.kind === 'MissingData') {
        return value;
      }
      return nested(
        selection.alias,
        record,
        eachLink(value.value, record, key, (link) => readAt(hooks, tables, selection.selections, link, field.target, variables)),
      );
    }
    case 'Derived': {
      const derived = hooks.readResolver(field.artifact(), record, childVariables(variables, selection.arguments));
      if (derived.kind !== 'Ok' || field.target === null) {
        return nested(selection.alias, record, derived);
      }
      const target = field.target;
      return nested(
        selection.alias,
        record,
        derived.value === null
          ? ok(null)
          : readAt(hooks, tables, selection.selections, derivedLink(derived.value, record, selection.fieldName), target, variables),
      );
    }
  }
}

/** `link`'s record read as `entity`: `selections` there; with none, a `Leaf` entity's `__value` and a
 * `Composite` entity's `EMPTY_DATA`. */
function readAt<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  selections: ReaderSelectionSet | null,
  link: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<unknown> | TFailure {
  if (selections !== null) {
    return readSelectionSet(hooks, tables, selections, link, entity, variables);
  }
  switch (tableOf(tables, entity).shape) {
    case 'Leaf':
      return readStoreValue(hooks, link, VALUE_KEY);
    case 'Composite':
      return ok(EMPTY_DATA);
  }
}

/** The origin of `fieldName` on `entity`: `__self`, or its table entry. Throws when the table has no
 * entry: the compiler lists every field of the entity. */
function fieldOf(table: EntityTable, entity: EntityName, fieldName: string): FieldEntry | { readonly kind: 'Self' };

/** `tables[entity]()`. Throws when absent: the compiler imports every entity the walk reads at. */
function tableOf(tables: EntityTables, entity: EntityName): EntityTable;

/** A linked derived field's non-`null` value as a link. Throws when it is not one: the field's resolver
 * is generated and returns a link or `null`. */
function derivedLink(value: unknown, owner: StoreLink, fieldName: string): StoreLink;

/** `missing` wrapped by `missingDataFor(alias, root, ..)`; any other result unchanged. */
function nested<T, TFailure>(alias: string, root: StoreLink, result: ReadResult<T> | TFailure): ReadResult<T> | TFailure;

/** v1's `readLinkedValue` (`read.ts:207-229`) with `f` where it called `readSelectionSet`: `null` is
 * `null`, a list maps item by item at any depth, and a link is `f(link)`; `linkAt` throws on a non-link. */
function eachLink<TFailure>(
  value: StoreValue,
  owner: StoreLink,
  key: StoreKey,
  f: (link: StoreLink) => ReadResult<unknown> | TFailure,
): ReadResult<unknown> | TFailure;
```

At the record `R` read as entity `E`, for the selection `{ alias, fieldName, arguments, selections }`:

- The entry of `fieldName` in `E`'s table decides the value `v`: a `Source` reads the source node of the store key on `R`; a `Derived` pulls its derived node on `R`; `__self` is `R`, read as `E`.
- `MISSING` is `MissingData`. A derived node's `MissingData` or `ResolverThrew` passes up unchanged. `null` is `null`.
- A source's list maps item by item at any depth. A derived value is never mapped.
- A `Derived` entry with target `null` returns `v` as is: the walk does not inspect an opaque output.
- A link with `selections` is read there as the entry's target; a link with `selections: null` reads its record's `__value` when the target is `Leaf` and is `EMPTY_DATA` when it is `Composite`.
- Every read is through `hooks`, so a bare scalar's `__value` read is recorded on the computation that walks it, and creates no node.
- `ReadHooks`, `resolverNodeAt`, `resolverComputation`, and `contextHooks` keep their signatures: a position is a record link. A derived node on a scalar is `resolverNodeAt(store, artifact, String:Pet:1.name, variables)`, keyed `String.toUppercase/{}` in that record's `derived`.

An absent record keeps the rule of Absent records: a source field on it reads a `MISSING` source node and is `MissingData`; a derived field on a link to it still creates its node in the `Absent` container, because the table is static; a bare composite selection on it reads nothing and is `EMPTY_DATA`; creating the record writes the source nodes those readers read and marks them.

`EMPTY_DATA` is one shared frozen object. A walk-assembled empty object (`bestFriend {}`, an empty selection set) compares equal to another by the structural rule (Equality), because the rule compares a selection set's data field by field and an empty set has none. A bare selection has no selection set to compare by, so its value compares by `===`, and a new `{}` per read would be unequal on every recomputation and mark every reader.

### Scalar records

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type NormalizationAst<TEntityName extends EntityName = EntityName> = {
  readonly discriminatorResponseKey: string;
  readonly root: NormalizationRootRecord<TEntityName>;
  readonly selections: NormalizationSelectionSet;
  /** The table of every entity whose records `normalize` writes with this AST, for its `storage`. */
  readonly tables: { readonly [entityName: string]: () => EntityTable };
};

export type NormalizationSelection = {
  readonly kind: 'Selection';
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  readonly target: NormalizationTarget;
};

export type NormalizationTarget = NormalizationLeafTarget | NormalizationCompositeTarget;

/** A scalar entity: each value is written to the `__value` of its own record. */
export type NormalizationLeafTarget = {
  readonly kind: 'Leaf';
  readonly entityName: EntityName;
  /** The number of list levels of the selectable's type. A custom scalar's value can be a JSON array
   * that is one value, so `normalize` does not descend into arrays below this depth. */
  readonly listDepth: number;
};

export type NormalizationCompositeTarget = {
  readonly kind: 'Composite';
  readonly entity: NormalizationEntity;
  readonly selections: NormalizationSelectionSet;
};

export type NormalizationConditional = {
  readonly kind: 'Conditional';
  readonly condition: Condition;
  readonly selections: NormalizationSelectionSet;
};
```

Before (`artifacts.ts:5-71`): `NormalizationAst` has no `tables`; `target: NormalizationCompositeTarget | null`, `null` for a leaf; `NormalizationCompositeTarget` has no `kind` and has `identity: NormalizationIdentity`; `NormalizationConditional` has `identity`. What moves to the entity table: the identity, as `storage`, and `recordIdentity`'s search of a target's conditionals for the concrete entity's identity (`recordId.ts:73-84`), which the concrete entity's own table answers. What stays in the response AST, as upstream's: `discriminatorResponseKey`, `root`, each selection's response key, name, arguments, and target, `Known` or `Discriminator` per composite target (a `Discriminator` still reads `__typename` from the response object to pick the entity whose table gives the storage), and each conditional's condition and selections.

```ts
// from libs/isograph-runtime/src/store.ts
/** The store key of a scalar record's value. GraphQL reserves names that start with `__`, and the
 * name differs from the `__self` field. */
export const VALUE_KEY: StoreKey = '__value';

// from libs/isograph-runtime/src/recordId.ts
/** The id of a record with `FromParent` storage: the parent's link, the selectable name, one
 * `.<index>` per list level, then the argument chunks. */
export function omittedRecordId(
  parent: StoreLink,
  selection: NormalizationSelection,
  listIndexes: ReadonlyArray<number>,
  variables: Variables,
): RecordId {
  return (
    `${parent.entityName}:${parent.id}.${selection.selectableName}` +
    listIndexes.map((index) => `.${index}`).join('') +
    argumentChunks(selection.arguments, variables)
  );
}

/** The id of the record of `value`, at `position`, of an entity with `storage`. Throws on an `OwnId`
 * value that is not an object holding a string under `storage.selectableName`. */
export function recordId(
  value: JsonValue,
  storage: RecordStorage,
  position: RecordPosition,
  variables: Variables,
): RecordId;

/** `tables[entityName]().storage`. Throws when the AST has no table for it: the response names an
 * entity the compiled schema does not have. */
export function storageOf(normalization: Normalization, entityName: EntityName, path: ResponsePath): RecordStorage;
```

`omittedRecordId` is `recordId`'s `Omit` branch (`libs/isograph-runtime/src/recordId.ts:109-114`), extracted verbatim; `recordId`'s `FromParent` arm calls it, its `OwnId` arm is the `Select` branch (`:95-108`), and its `Fixed` arm the `Fixed` branch (`:93-94`). A scalar record has the id an id-less object at the same place has: `Pet:1.name`, `Pet:1.tags.0`, `Pet:1.checkins____limit___2.0.location`. Its collisions are those of `Omit` ids (store.md).

`normalize` writes a leaf as records:

```ts
// from libs/isograph-runtime/src/normalize.ts
// writeSelection, before (normalize.ts:96-109):
  record.set(
    storeKey(selection, normalization.variables),
    selection.target == null
      ? value
      : compositeValue(normalization, selection, selection.target, value, link, [], [...path, responseKey]),
  );
// after:
  const key = storeKey(selection.selectableName, selection.arguments, normalization.variables);
  const target = selection.target;
  const written = target.kind === 'Leaf'
    ? leafRecords(normalization, selection, target, value, link, [], target.listDepth, [...path, responseKey])
    : compositeValue(normalization, selection, target, value, link, [], [...path, responseKey]);
  writeValue(normalization, link, key, written);

/** `value` with each scalar written to the `__value` of its record and replaced by the link to it;
 * `null` at any level is `null`. Throws on a non-list where `depth` says list. */
function leafRecords(
  normalization: Normalization,
  selection: NormalizationSelection,
  target: NormalizationLeafTarget,
  value: JsonValue,
  parent: StoreLink,
  listIndexes: ReadonlyArray<number>,
  depth: number,
  path: ResponsePath,
): CompositeValue {
  if (value === null) {
    return null;
  }
  if (depth > 0) {
    if (!isArray(value)) {
      throw new Error(`Malformed response at ${JSON.stringify(path)}: expected a list or null, got ${JSON.stringify(value)}.`);
    }
    return value.map((item, index) =>
      leafRecords(normalization, selection, target, item, parent, [...listIndexes, index], depth - 1, [...path, index]),
    );
  }
  const position = { parent, selection, listIndexes, path };
  const link = {
    entityName: target.entityName,
    id: recordId(value, storageOf(normalization, target.entityName, path), position, normalization.variables),
  };
  presentContainerAt(normalization.store, link);
  writeValue(normalization, link, VALUE_KEY, value);
  return link;
}
```

`compositeValue` takes the link's entity from `Known` or the discriminator as today (`recordEntityName`), then the id from `recordId(value, storageOf(normalization, entityName, path), position, variables)`. The compiler gives every scalar entity `FromParent` storage, so a leaf's id is `omittedRecordId`. `presentContainerAt` is `recordAt` (`normalize.ts:163-169`) until core change 5. A rewrite with a new value changes only `__value`: the parent's link is unchanged, so its source's `compositeUnchanged` accepts it. The parent's key changes when the value becomes or stops being `null`, or a list's length or item nullness changes.

`check` and the garbage collector's mark follow a `Leaf` target as they follow a `Composite` one, one level: for each link under the key (`linksIn`), `check` yields the linked record when it is absent or its `__value` is missing (`libs/isograph-runtime/src/check.ts:47-68`), and the mark yields the linked record and does not recurse (`libs/isograph-runtime/src/garbageCollection.ts:68-86`). A scalar record no retained query reaches, such as `Pet:1.tags.2` after the list shrank to two, is swept; `deleteRecord` disposes the derived nodes it owns.

`storeSnapshot` returns scalar records as stored (open decision 22).

Costs: one record container and one more source node per scalar value; two engine reads per bare scalar selection (the parent's key, then `__value`), of which the first edge fires only on the transitions above. The walk, `check`, and the mark read the store only through the accessors (`sourceNodeAt`, `storeValue`, `recordPresence`), so storing a value inline in its parent later changes the accessors and `leafRecords`, not the readers (Follow-ups 11).

### Example

```text
field Query.PetDetailPage($id: ID!) { pet(id: $id) { n1: name n2: name { toUppercase } bestFriend { name } } }
field String.toUppercase { __self }
```

The generated artifacts are in linked-only-selections.md, Generated files for the example. The store holds `Query:__ROOT.pet____id___"1"` linking to `Pet:1`; `Pet:1.name` linking to `String:Pet:1.name`, whose `__value` is `"Ricky"`; `Pet:1.bestFriend` linking to `Pet:2`; `Pet:2.name` linking to `String:Pet:2.name`, whose `__value` is `"Mango"`. `PetDetailPage` is read with `{ id: "1" }`; its node is `Query.PetDetailPage/{"id":"1"}` on `Query:__ROOT`. `X.__value` is the `__value` source node of the scalar record `X`; `T.f` names the entry of `f` in `T`'s table.

First read:

```text
read PetDetailPage: absent -> create Dirty, no value -> recompute at Query:__ROOT as Query
  pet: Query.pet Source, target Pet -> Query:__ROOT.pet____id___"1" -> dependents {PetDetailPage} -> Pet:1 -> selections at Pet:1 as Pet
    n1: Pet.name Source, target String -> Pet:1.name -> {PetDetailPage} -> String:Pet:1.name, no selections
      String shape Leaf -> String:Pet:1.name.__value -> {PetDetailPage} -> "Ricky"
    n2: Pet.name Source -> Pet:1.name (second entry) -> String:Pet:1.name -> selections at String:Pet:1.name as String
      toUppercase: String.toUppercase Derived, target null -> String:Pet:1.name.derived "String.toUppercase/{}" absent -> create Dirty
        -> recompute at String:Pet:1.name as String, with toUppercase's tables
        __self -> String:Pet:1.name as String, no selections -> String shape Leaf
          -> String:Pet:1.name.__value -> {PetDetailPage, toUppercase@String:Pet:1.name} -> "Ricky"
        resolver({ data: { __self: "Ricky" }, parameters: {}, link: String:Pet:1.name }) -> toUppercase Clean "RICKY"
      toUppercase@String:Pet:1.name.dependents = {PetDetailPage} -> target null -> "RICKY" as is
    bestFriend: Pet.bestFriend Source, target Pet -> Pet:1.bestFriend -> {PetDetailPage} -> Pet:2 -> selections at Pet:2 as Pet
      name: Pet:2.name -> String:Pet:2.name -> Leaf -> String:Pet:2.name.__value -> "Mango"
  resolver -> PetDetailPage Clean p1 = { pet: { n1: "Ricky", n2: { toUppercase: "RICKY" }, bestFriend: { name: "Mango" } } }
```

```text
PetDetailPage reads:                 Query:__ROOT.pet____id___"1", Pet:1.name, String:Pet:1.name.__value, Pet:1.name,
                                     toUppercase@String:Pet:1.name, Pet:1.bestFriend, Pet:2.name, String:Pet:2.name.__value
toUppercase@String:Pet:1.name reads: String:Pet:1.name.__value
```

Write `Pet:1.name`:

```text
normalize Pet:1.name = "Rex": Pet:1.name's link String:Pet:1.name -> compositeUnchanged accepts -> nothing marked
  String:Pet:1.name.__value = "Rex" -> leafUnchanged rejects -> {PetDetailPage, toUppercase@String:Pet:1.name} -> both Dirty
read PetDetailPage: Dirty -> recompute
  n1 "Rex"; n2: toUppercase Dirty -> recompute "REX" !== "RICKY" -> Clean "REX"; bestFriend -> PetDetailPage Clean p2
```

Write `Pet:2.name`:

```text
normalize Pet:2.name = "Moe": String:Pet:2.name.__value -> {PetDetailPage} -> Dirty
read PetDetailPage: Dirty -> recompute: n2 -> toUppercase@String:Pet:1.name Clean -> "REX", not run -> PetDetailPage Clean p3
```

Only `PetDetailPage`'s resolver runs: the scalar's client field is not called again when its parent recomputes for another input.

Write `Pet:1.name` to a value with the same uppercase:

```text
normalize Pet:1.name = "rex": String:Pet:1.name.__value -> {PetDetailPage, toUppercase} -> both Dirty
read PetDetailPage: Dirty -> recompute
  n1 "rex"; n2: toUppercase Dirty -> recompute "REX" === "REX" -> Clean, keeps "REX", marks nothing
  -> PetDetailPage Clean p4 (n1 changed)
```

`toUppercase` cuts off, but `PetDetailPage` reruns anyway, because `n1` reads `String:Pet:1.name.__value` directly. Without `n1`, `PetDetailPage` reads `Pet:1.name` (the link, unchanged) and the `toUppercase` node, so the same write marks `toUppercase` `Dirty` and `PetDetailPage` `Check`, the pull recomputes `toUppercase` to `"REX"`, and `PetDetailPage` ends `Clean` without running.

`Pet:1.name` becomes `null`. The fixture declares `Pet.name: String!`; `normalize` does not check nullability, so this is the runtime's behavior on such a response:

```text
normalize Pet:1.name = null: leafRecords -> null, writes no scalar record
  Pet:1.name: link -> null -> compositeUnchanged rejects -> {PetDetailPage} -> Dirty
  toUppercase@String:Pet:1.name is not marked: it reads only String:Pet:1.name.__value
read PetDetailPage: Dirty -> recompute
  n1: Pet:1.name -> null -> null; n2: Pet:1.name -> null -> null, toUppercase not read; bestFriend -> "Mango"
  -> PetDetailPage Clean p5 = { pet: { n1: null, n2: null, bestFriend: { name: "Mango" } } }
  toUppercase@String:Pet:1.name.dependents -= PetDetailPage -> {}; String:Pet:1.name.__value.dependents = {toUppercase}
collection: no retained query reaches String:Pet:1.name through Pet:1.name -> deleteRecord(String:Pet:1.name)
  -> dispose toUppercase@String:Pet:1.name (no dependents) -> container removed
```

`bestFriend`'s record is absent and is later created. The store holds `Pet:1.bestFriend` linking to `Pet:2` and no `Pet:2` record, which a collection deleted while a retained query kept `Pet:1` (Garbage collection in the core):

```text
read PetDetailPage: recompute
  ... bestFriend: Pet:1.bestFriend -> Pet:2 -> selections at Pet:2 as Pet
    name: Pet.name Source -> sourceNodeAt(Pet:2, name): create Pet:2 Absent, name MISSING -> {PetDetailPage}
    -> MissingData "No value for name on Pet:2" -> "Missing data for bestFriend on Pet:1" -> "Missing data for pet on Query:__ROOT"
  -> PetDetailPage Clean MissingData, resolver not called
normalize a response with Pet:2: compositeValue -> Pet's table storage OwnId "id" -> Pet:2 -> presentContainerAt -> Present
  Pet:2.name = String:Pet:2.name -> MISSING rejected -> {PetDetailPage} -> Dirty
  String:Pet:2.name created, __value "Mango", read by nothing
  Pet:1.bestFriend = Pet:2 -> compositeUnchanged accepts -> nothing marked
read PetDetailPage: Dirty -> recompute -> bestFriend: name -> "Mango" -> PetDetailPage Clean p6
```

A refinement, `field Query.F { node(id: "1") { asPet { name } } }`:

```text
read F: recompute at Query:__ROOT as Query
  node: Query.node Source, target Node -> Query:__ROOT.node____id___"1" -> {F} -> Pet:1 -> selections at Pet:1 as Node
    asPet: Node.asPet Derived, target Pet -> Pet:1.derived "Node.asPet/{}" absent -> create Dirty
      -> recompute at Pet:1 as Node: empty reader AST, reads nothing
      resolver({ data: {}, parameters: {}, link: Pet:1 }) -> link.entityName is "Pet" -> Pet:1 -> asPet Clean Pet:1, dependencies []
    asPet@Pet:1.dependents = {F} -> Pet:1 is a link -> selections at Pet:1 as Pet
      name: Pet.name -> Pet:1.name -> String:Pet:1.name -> Leaf -> String:Pet:1.name.__value -> {F}
normalize Pet:1.name = "Rex": String:Pet:1.name.__value -> {F} -> F Dirty; asPet@Pet:1 is not marked
read F: Dirty -> recompute -> asPet@Pet:1 Clean, not run -> F Clean
delete Pet:1 -> Pet:1.derived = {Node.asPet/{}} -> dispose asPet -> F Dirty
```

On a `Checkin` record the same selection reads `Node.asPet/{}` on `Checkin:7`, whose resolver returns `null`, and `asPet` is `null`.

Duplicate dependency entries. A computation that reads one node twice records it twice (`PetDetailPage` reads `Pet:1.name` for `n1` and for `n2`). The `dependents` set holds the reader once (reactive-engine.md, Dependencies). `verify` skips `Source` entries and finds a second `Derived` entry of the same node `Clean`, one step. The list is kept as read: deduplicating costs a set per computation on every read to shorten a list by entries that cost one step each.

## Stored resolver results

```ts
// from libs/isograph-runtime/src/storedResults.ts
import {
  type Computation,
  type ComputeContext,
  createDerivedNode,
  GraphWriteDuringComputationError,
  pull,
} from '@isograph/reactive-graph';

/** `===`: the output equality of a resolver whose field supplies none. */
function identical(previous: unknown, next: unknown): boolean {
  return previous === next;
}

/** The stored result of `artifact` on `root` with `variables`, pulled. */
export function readStoredResult<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ResolverResult<TOutput> {
  return pull(store.graph, resolverNodeAt(store, artifact, root, variables)) as ResolverResult<TOutput>;
}

/** The derived node of `artifact` on `root` with `variables`, created `Dirty` in `root`'s container
 * when absent. */
export function resolverNodeAt(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ResolverNode {
  return getOrInsert(
    containerAt(store, root).derived,
    resolverStoreKey(artifact, variables),
    () => createDerivedNode(resolverComputation(store, artifact, root, variables), resolverResultsEqual(artifact, identical)),
  );
}

/** Reads `artifact`'s reader AST at `root` through `context`, then calls the resolver. */
function resolverComputation(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): Computation<ResolverResult<unknown>> {
  return (context) => {
    const data = readData(contextHooks(store, context), artifact, root, variables);
    return data.kind === 'Ok' ? callResolver(artifact, root, data.value, variables) : data;
  };
}

/** Hooks that report each source node read to `context` and read each nested resolver as a derived node. */
function contextHooks(store: Store, context: ComputeContext): ReadHooks<ReadResolverThrew> {
  return {
    readValue: (record, key) => context.readSource(sourceNodeAt(store, record, key)),
    readResolver: (artifact, root, variables) => context.readDerived(resolverNodeAt(store, artifact, root, variables)),
  };
}

/** `artifact.resolver` called with `data`; `WalkData` when it returned `data` itself. A throw becomes
 * `ResolverThrew`, except the engine's `GraphWriteDuringComputationError`, which propagates (open
 * decision 30). */
function callResolver(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ResolverResult<unknown> {
  try {
    const value = artifact.resolver({ data, parameters: variables, link: root });
    return { kind: 'Ok', value, origin: value === data ? 'WalkData' : 'ResolverValue' };
  } catch (error: unknown) {
    if (error instanceof GraphWriteDuringComputationError) {
      throw error;
    }
    return { kind: 'ResolverThrew', selectableName: artifact.selectableName, recordLink: root, error };
  }
}
```

The computation closes over `store`, the environment's `Map`, which is written in place and never replaced (network-and-subscriptions.md, The environment). The cast in `readStoredResult` is the one place a stored `ResolverResult<unknown>` is returned at the caller's type. Its type parameters and `root` parameter are `read`'s. `readStoredResult` takes the store, as `read` does; every function that reads or writes the graph takes the store, because the store is the graph.

A resolver computation keeps the engine's contract that a computation must not throw (reactive-engine.md, Throws): `callResolver` catches the resolver's own throw into `ResolverThrew`, a value. The one throw it does not catch is the engine's `GraphWriteDuringComputationError`, from a resolver that wrote the store through a closure: `callResolver` rethrows it, so it propagates out of `readStoredResult` (open decision 30). Any other throw out of `resolverComputation` or out of `resolverResultsEqual` is a broken programmer invariant: a store that disagrees with the reader AST (`linkAt` on a non-link, `libs/isograph-runtime/src/store.ts:74-86`), a missing variable (`variableValue`, `storeKey.ts:56-63`), or a field-supplied `outputEqual` that throws (open decision 7). It propagates out of `readStoredResult`; `readSubscription`'s `try` (`libs/isograph-runtime/src/subscribe.ts:126-130`) catches what escapes a subscription re-read as `ReadErr`.

Every read inside a computation goes through the walk, so none goes unrecorded. User code never receives a `ComputeContext` or the store. A resolver that reaches the store through a closure is open decision 2.

The resolver store key:

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** `<parentEntityName>.<selectableName>/<variables>`, with `<variables>` the field's variables as
 * `JSON.stringify(stableCopy(variables))` writes them (open decision 1). */
export function resolverStoreKey(
  field: Pick<ReaderArtifact<ResolverParam, unknown, EntityName>, 'parentEntityName' | 'selectableName'>,
  variables: Variables,
): StoreKey {
  return `${field.parentEntityName}.${field.selectableName}/${JSON.stringify(stableCopy(variables))}`;
}
```

The variables are the field's own: for a `Derived` entry, `childVariables(variables, selection.arguments)` (`read.ts:232-242`); for a top-level read, the read's variables. `PetName` on `Pet:1` is `Pet.PetName/{}`; `PetSummary` read with `{ id: "1" }` is `Query.PetSummary/{"id":"1"}` on `Query:__ROOT`; `NodeLink` read under `node(id: $id)` on `Pet:1` is `Node.NodeLink/{}`, a different field from any `Pet.NodeLink/{}`. The entity is part of the key because one record is read as several entities (Entity tables). The variables are upstream's encoding (`stableIdForFragmentReference`, `FragmentReference.ts:75-79`; open decision 1). A GraphQL name cannot hold `.` or `/`, so the key is injective: the first `.` ends the entity name, the first `/` ends the selectable name, and the rest is the JSON. Two selections of one field with equal argument values on one record are one node, whatever their aliases, read by both parents.

The key needs the client field's entity and selectable name, and the walk needs the tables of the entities it reads at, which v1's reader artifact does not carry:

```ts
// from libs/isograph-runtime/src/artifacts.ts
// ReaderArtifact, before:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
// after:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  /** The entity the client field is declared on: the entity the walk reads its root as. */
  readonly parentEntityName: EntityName;
  /** The client field's selectable name. */
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  /** The table of every entity the walk of `readerAst` reads at, by entity name (Entity tables). */
  readonly tables: { readonly [entityName: string]: () => EntityTable };
  resolver(param: TParam): TOutput;
};
```

`reader_artifact_file` (`crates/typescript_artifacts/src/reader_artifact.rs`) writes `parentEntityName` and `selectableName` as the object's first entries and `tables` after `readerAst`; `Pet/PetName/resolver_reader.ts` gains `parentEntityName: 'Pet', selectableName: 'PetName',` and `tables: { Pet: Pet__entity_table, String: String__entity_table }`. `TEntityName` stays the entity name of the records the field is read on, which is what `link.entityName` holds (`artifacts.ts:110-114`): the parent entity, or for an entity whose records are stored under other entities (`Node`, read on `Pet:1`), the union of those entities' names. So `parentEntityName` is an `EntityName`, not `TEntityName`: `Node` is not in `'Pet' | 'Checkin' | 'BlogItem'`.

## `normalize` writes source nodes

`normalize` keeps its parameters and its `void` return (`libs/isograph-runtime/src/normalize.ts:31-50`). It writes each value into its source node with `writeSource`, which marks as it writes. A value the source's equality accepts (`isUnchanged`, The store) writes nothing and marks nothing, and the stored value object keeps its identity.

```ts
// from libs/isograph-runtime/src/normalize.ts
import { writeSource } from '@isograph/reactive-graph';

/** Writes `value` into the source node of `key` on `link`. */
function writeValue(normalization: Normalization, link: StoreLink, key: StoreKey, value: StoreValue): void {
  writeSource(normalization.store.graph, sourceNodeAt(normalization.store, link, key), value);
}
```

Deltas from v1 (`normalize.ts`): `writeSelection` calls `writeValue` where it called `record.set` (`:96-109`), and `leafRecords` calls it for a scalar record's `__value` (Scalar records); `writeSelections` and `compositeValue` pass the record's link where they passed its `StoreRecord`; `recordAt` is `presentContainerAt` (`:163-169`). The comparison core change 1 adds to `normalize` on v1's plain store moves into the source nodes' equality in core change 5, where source nodes first exist: `leafUnchanged` and `compositeUnchanged` move from `normalize.ts` to `store.ts`, and the key decides which applies, because after core change 3 only `__value` holds a leaf.

Deltas from upstream: upstream compares a scalar with `!==` only (`cache.ts:248-249`), so a list of scalars counts as changed on every response; here a leaf list compares by length and item at any depth. Upstream compares a link by `__link` and `__typename` and a list of links one level deep (`cache.ts:337-338`, `:372-393`); here a link compares by `entityName` and `id`, and lists nest at any depth. A leaf that is a JSON object compares by `===`, so it counts as changed on every rewrite: one mark too many, never a missed one.

Writing marks as it goes, so a response that throws part-way has marked every write it made. `normalize` throws v1's error unchanged (open decision 4). Absent to present is `presentContainerAt` and marks nothing: presence is not a source, and every reader of the absent record read a key, which the write marks if it sets it.

## Traces

First read:

```text
read FP: absent -> create FP Dirty, no value -> recompute FP
  root.1 -> root.1.dependents = {FP}
  root.2 -> root.2.dependents = {FP}            (link to other)
  other.1 -> other.1.dependents = {FP}
  other.2 -> other.2.dependents = {FP}
  FC: absent -> create FC Dirty, no value -> recompute FC
    root.2 -> root.2.dependents = {FP, FC}
    root.3 -> root.3.dependents = {FC}
    other.2 -> other.2.dependents = {FP, FC}
    other.3 -> other.3.dependents = {FC}
    FC Clean v1
  FC.dependents = {FP}
  FP Clean p1
read FP: Clean -> p1
```

Write and mark:

```text
write root.3 -> root.3.dependents = {FC} -> FC Dirty -> FC.dependents = {FP} -> FP Check
write other.3 -> other.3.dependents = {FC} -> FC already Dirty -> stop
write root.1 -> root.1.dependents = {FP} -> FP Check -> FP Dirty (dependents already marked)
```

Pull with short circuit, after only `write root.3`:

```text
read FP: Check -> FC Dirty -> recompute -> v2 === v1 -> FC Clean, keeps v1 -> FP Clean without running
```

The same pull when `FC` changed:

```text
read FP: Check -> FC Dirty -> recompute -> v2 !== v1 -> FC Clean v2 -> FC.dependents = {FP} -> FP Dirty
  -> recompute FP: root.1, root.2, other.1, other.2, FC Clean -> v2 -> FP Clean p2
```

A source only `FP` read:

```text
write root.1 -> root.1.dependents = {FP} -> FP Dirty -> FP.dependents = {}
read FP: Dirty -> recompute FP: root.1, root.2, other.1, other.2, FC Clean -> v1 (FC not run) -> FP Clean
```

A link retarget: `root.2` moves from `other` to `next`.

```text
write root.2 = next -> root.2.dependents = {FP, FC} -> FP Dirty, FC Dirty -> FC.dependents = {FP} (already Dirty)
read FP: Dirty -> recompute FP
  root.1, root.2 -> next, next.1, next.2
  FC: Dirty -> recompute FC: root.2, root.3, next.2, next.3 -> FC Clean
    other.2.dependents -= FC, other.3.dependents -= FC, next.2.dependents += FC, next.3.dependents += FC
  FP Clean
    other.1.dependents -= FP, other.2.dependents -= FP, next.1.dependents += FP, next.2.dependents += FP
write other.2 -> other.2.dependents = {} -> nothing marked
```

Record deletion of `other`:

```text
delete other -> other.1.dependents = {FP} -> FP Dirty
             -> other.2.dependents = {FP, FC} -> FC Dirty
             -> other.3.dependents = {FC}
             -> other.derived = {} -> nothing to dispose -> container removed
read FP: Dirty -> recompute FP: root.1, root.2 -> other, other.1: no container -> other Absent, other.1 MISSING
  -> MissingData naming other.1 -> walk stops -> FP Clean MissingData
  FP now reads root.1, root.2, other.1; FC.dependents -= FP; FC stays Dirty with v1, read by nothing
```

Record deletion of `root`, which owns `FP` and `FC`:

```text
delete root -> root.1, root.2, root.3 dependents -> FP, FC Dirty
            -> root.derived = {FP, FC} -> dispose FP: remove from root.1, root.2, other.1, other.2, FC dependents
                                       -> dispose FC: remove from root.2, root.3, other.2, other.3 dependents
            -> container removed
read FP: root absent -> create root Absent, FP Dirty, no value -> recompute FP: root.1 MISSING -> FP Clean MissingData
```

## End-user experience

The pet demo's fetch-and-read script (`demos/pet-demo/src/fetchAndRead.ts`, `demos/pet-demo/README.md`, Fetching and reading) reads with `readStoredResult` where it read with `read`, and prints the same output. Its graph:

```text
HomeRoute reads: Query.__ROOT.pets, Pet:0.id, PetSummaryCard@Pet:0, Pet:1.id, PetSummaryCard@Pet:1
PetSummaryCard@Pet:n reads: Pet:n.id, fullName@Pet:n, Pet:n.nickname, Pet:n.age, Pet:n.picture, Pet:n.tagline, Pet:n.species
fullName@Pet:n reads: Pet:n.firstName, Pet:n.lastName
```

```text
read HomeRoute: creates and runs HomeRoute, PetSummaryCard@Pet:0, @Pet:1, fullName@Pet:0, @Pet:1, once each
PetDetailRoute response: Pet:0.firstName "Makayla" -> leafUnchanged -> no write; likewise every HomeRoute input -> nothing marked
SetTagline response: Pet:0.id "0" -> unchanged
                     Pet:0.tagline "I AM FULL" -> {PetSummaryCard@Pet:0} -> Dirty -> {HomeRoute} -> Check
read HomeRoute: Check -> PetSummaryCard@Pet:0 Dirty -> recompute (fullName@Pet:0 Clean, not run)
  -> "Makayla Balicki: I AM FULL" !== old -> HomeRoute Dirty -> recompute HomeRoute
  -> PetSummaryCard@Pet:1 Clean, not run -> HomeRoute Clean
```

What a user observes: after the tagline write, the `HomeRoute` read calls `PetSummaryCard` on `Pet:0` and `HomeRoute` once each; `PetSummaryCard` on `Pet:1` and `fullName` on either pet are not called; a second read with no write between calls nothing and returns the same object. The script's `valueOrThrow` gains a `ResolverThrew` arm that throws `result.error`.

```ts
// from demos/pet-demo/src/fetchAndRead.ts
// readHomeRoute, before:
  const readHomeRoute = () =>
    valueOrThrow(homeRoute.name, read(environment.store, homeRoute.readerArtifact(), homeRoute.normalizationAst.root, {}));
// after:
  const readHomeRoute = () =>
    valueOrThrow(homeRoute.name, readStoredResult(environment.store, homeRoute.readerArtifact(), homeRoute.normalizationAst.root, {}));
```

## Garbage collection in the core

`garbageCollect(store, retained)` keeps its parameters (`libs/isograph-runtime/src/garbageCollection.ts:24-43`) and its rule: a record survives while a retained query's normalization AST reaches it. Under everything-is-graph:

- The mark reads through `storeValue` and `recordPresence`. A link that reaches an `Absent` container keeps it (a reader that followed that link found it absent) and the mark does not recurse into it. The root of a retained query keeps its container whatever its presence.
- The sweep calls `deleteRecord` for each container the mark did not reach, `Present` or `Absent`: the dependents of every source node are marked `Dirty`, every derived node in `derived` is disposed (its dependents marked `Dirty`), and the container is removed. Open decision 9 covers an unreached `Absent` container whose source nodes still have readers.
- A derived node lives as long as its record container. A node a read created on a record no retained query reaches is disposed with that record. A node on a retained record lives until that record is deleted, read or not; disposing unreached derived nodes earlier is Later: collecting derived nodes.
- `Query.__ROOT`'s `derived` map grows. A retained query's root keeps the `Query.__ROOT` container, and a top-level read's node is keyed by its variables, so the map gains one root node per distinct variables value read on it, and the core removes none while any retained query keeps `Query.__ROOT`. Later: collecting derived nodes' `collect` is the first code that removes them. The values stay correct.
- Reads create `MISSING` source nodes. `sourceNodeAt` inserts one for each distinct key a computation reads that its container does not hold. On a container the mark keeps, nothing removes such a node, including after its `dependents` set is empty: `deleteRecord` runs only on an unreached container, and `collect` disposes derived nodes only. `storeSnapshot`, `storeValue`, and `check` skip `MISSING`, so the growth is not visible in the store's server data.

Upstream's collection deletes records only and runs only when the base layer is the only layer (`garbageCollection.ts:65-86`).

## Core changes

Each is one commit, in order; each is independently shippable.

1. Prefactor: `normalize` skips an unchanged value (`writeValue`, `isUnchanged`, `leafUnchanged`, `compositeUnchanged`) on v1's plain store, comparing before `record.set`. It stays a prefactor rather than folding into the record containers change: it is observable and shippable on the plain store with no engine, and core change 5 moves the same comparison into the source nodes' `equal`, unchanged in behavior. Observable: normalizing the same response twice keeps every stored list and link object (`toBe`). In the pet script, the `PetDetailRoute` response leaves `Pet:0.firstName`'s stored value untouched.
2. Prefactor: the walk takes `ReadHooks`; an absent record reads as all keys missing; `noRecord`'s two checks go (Absent records). `read` and `readButDoNotEvaluate` use `evaluatingHooks`. Observable: an absent root's `MissingData` reads "No value for pet____id___"1" on Query:__ROOT"; `Constant` on an absent record is `Ok`. The pet script's output is unchanged.
3. Prefactor: scalar records on v1's reader AST (linked-only-selections.md, change 1; Scalar records). The normalization AST's `Leaf` target with its entity and list depth, `VALUE_KEY`, `omittedRecordId`, `leafRecords` (ids by `omittedRecordId` directly, before entity tables exist), and `check`'s and the mark's `Leaf` case. v1's `Scalar` case reads the link under its key, then `__value` at each linked record, lists mapped. Observable: `storeSnapshot` after normalizing `PetDetail`'s response holds `Pet:1.name` as a link to `String:Pet:1.name` with `__value` `"Ricky"`; every read returns the values it returned before; normalizing a response that changes only `Pet:1.name` writes only `String:Pet:1.name.__value`; the pet script's output is unchanged.
4. Prefactor: the engine package, shipped alone before the Isograph runtime uses it (open decision 26). This change implements `refactors/pending/runtime/reactive-engine.md`: `libs/reactive-graph` with `src/graph.ts` and `src/index.ts` as its The public API and The implementation write them, its test files (Tests) and `README.md` (The package README), and its `package.json`, tsconfig, tsdown, and vitest files, its two pre-commit hooks, and its CI step (The package), in the form of whichever option of its open decision 2 is decided. Nothing in the Isograph runtime imports it yet. Observable: `pnpm --filter @isograph/reactive-graph test` passes the sum, parity, and label example (`sum` and `parity` run twice, `label` once); `pnpm compile-libs` writes its `dist/index.cjs` and `dist/index.mjs`; its `package.json` lists no dependencies. Its reader is the stated goal, a project outside Isograph; the runtime is its first workspace caller in change 5.
5. Prefactor: the store becomes record containers (`Store` with `graph` and `records`, `createStore`, `RecordContainer`, `presence`, `sources`, the accessors, `storeSnapshot`, `deleteRecord` without derived nodes). The runtime gains its dependency on the engine (The engine package and the runtime) and its three pre-commit hooks match the engine's directory. A container's `sources` hold the engine's source nodes: `sourceNodeAt` calls `createSourceNode` with `isUnchanged(key)`, so `normalize`'s comparison moves out of `normalize` into each source's `equal`, `storeValue` and `storeSnapshot` call `sourceValue`, `writeValue` calls `writeSource`, and `deleteRecord` calls `markSourceChanged` for each source node. No derived node exists, so every `dependents` set stays empty and `writeSource` and `markSourceChanged` mark nothing. `check`, `garbageCollect`, `read`, and `normalize` read and write through the accessors; tests compare `storeSnapshot`. Observable: none in behavior; `environment.store` is no longer v1's nested map, and `storeSnapshot(environment.store)` is.
6. Prefactor: entity tables and the minimal reader AST (linked-only-selections.md, change 2). The compiler emits `ReaderSelection` as `{ alias, fieldName, arguments, selections }`, an `entity_table.ts` per entity a reader artifact or normalization AST imports, `ReaderArtifact.parentEntityName`, `selectableName`, and `tables`, `NormalizationAst.tables`, and a generated refinement field (`Node/asPet/`) per selected refinement; the normalization AST loses `identity`. The runtime gains `EntityTable`, the walk of Selections in the Isograph layer (`fieldOf`, `tableOf`, `readAt`, `EMPTY_DATA`), `recordId` over `RecordStorage` and `storageOf`, `storeKey(name, args, variables)`, and `selectionSetDataAreEqual` over tables; the four v1 selection kinds and `NormalizationIdentity` go. Observable: every generated artifact is regenerated in the new shape; every read returns the values it returned before; a `Node`-typed `node(id:)` selection of `asPet { name }` on a `Checkin` record is `null`; the pet script's output is unchanged.
7. Prefactor: linked-only selections (linked-only-selections.md, change 3). Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, client fields on scalar entities, and `__self`. Observable: `read` of the Example's `PetDetailPage` on the store fixture returns `{ pet: { n1: "Ricky", n2: { toUppercase: "RICKY" }, bestFriend: { name: "Mango" } } }`, calling `toUppercase` on every read; a bare `bestFriend` is `EMPTY_DATA` or `null`.
8. The graph in the Isograph runtime: `RecordContainer.derived`, `readStoredResult`, `resolverNodeAt` (`createDerivedNode` with `resolverResultsEqual(artifact, identical)`), `resolverComputation`, `contextHooks` (`context.readSource`, `context.readDerived`), `callResolver`, `ResolverResult` with `WalkData`, `resolverResultsEqual`, `resolverStoreKey` with the entity, `deleteRecord` disposing owned nodes with `disposeDerivedNode`. The engine is unchanged. The package index adds `readStoredResult` and the types `ResolverResult`, `ReadResolverOk`, `OkOrigin`, `ReadResolverThrew`; `StoreRecord` becomes `RecordContainer`. The pet script reads with `readStoredResult`. Observable: End-user experience above, and the Example's traces: after a write to `Pet:2.name`, a read of `PetDetailPage` does not call `toUppercase`; after a write that leaves `toUppercase`'s result equal, a reader without `n1` does not rerun.
9. The subscription re-read reads nested resolvers through the graph. `readSubscription` reads with `readData(storedResultHooks(store), ..)`, whose `readValue` is `storeValue` (nothing recorded) and whose `readResolver` is `readStoredResult`; `SubscriptionResult` gains `ReadResolverThrew`, `ReadErr` is only a broken store invariant, and `subscriptionResultsAreEqual` compares two `ResolverThrew` by `===` of their errors. Without it, the landed re-read (`libs/isograph-runtime/src/subscribe.ts:120-131`) calls every nested resolver on every write through v1's `read`. `makeNetworkRequest` calls `callSubscriptions` in a `finally` after `normalize`, so a response that throws part-way re-reads the subscriptions over the writes it made (open decision 4), and `garbageCollectEnvironment` calls `callSubscriptions` after the sweep, so a subscription that read a deleted record is re-read. Neither call depends on subscriptions being derived nodes. `callSubscriptions` never throws (`libs/isograph-runtime/src/subscribe.ts:92-95`; `readSubscription` catches a throwing pull as `ReadErr`, `:126-130`), so `normalize`'s throw still reaches the `.catch`. Observable: in the pet script, the subscription's re-read after the tagline write calls `PetSummaryCard` on `Pet:0` only, and a re-read after an unrelated write calls no resolver and does not call the callback; a response that changes `Pet:0.tagline` and then throws delivers the new tagline to the subscription; a collection that deletes a record a subscription read delivers that subscription's `MissingData`.

   ```ts
   // from libs/isograph-runtime/src/network.ts
   // makeNetworkRequest's .then body, before (libs/isograph-runtime/src/network.ts:143-149):
         normalize(
           environment.store,
           entrypoint.normalizationAst,
           response,
           variables,
         );
         callSubscriptions(environment);
   // after:
         try {
           normalize(
             environment.store,
             entrypoint.normalizationAst,
             response,
             variables,
           );
         } finally {
           callSubscriptions(environment);
         }
   ```

   ```ts
   // from libs/isograph-runtime/src/environment.ts
   // before (libs/isograph-runtime/src/environment.ts:62-66):
   export function garbageCollectEnvironment(
     environment: IsographEnvironment,
   ): void {
     garbageCollect(environment.store, Array.from(environment.retainedQueries));
   }
   // after:
   /** `garbageCollect` of the store, keeping what the retained queries reach, then the subscription re-read. */
   export function garbageCollectEnvironment(
     environment: IsographEnvironment,
   ): void {
     garbageCollect(environment.store, Array.from(environment.retainedQueries));
     callSubscriptions(environment);
   }
   ```

   `environment.ts` imports `callSubscriptions` from `./subscribe`, which imports only types from `./environment`.

## Later: retention and the fetch decision

Not part of the core. Retaining a query computes nothing: nothing runs at normalization time, and resolvers run when read.

```ts
// from libs/reactive-graph/src/graph.ts
export type DerivedNode<TValue> = {
  // .. the core's fields, and:
  /** Root holds: retentions and observers. Read by `collect`. */
  rootCount: number;
};

/** Adds one root hold to `node`. Computes nothing. The cleanup removes the hold. */
export function retainRoot<TValue>(node: DerivedNode<TValue>): CleanupFn;
```

```ts
// from libs/isograph-runtime/src/retention.ts
/** One `retainEntrypoint` call's hold on its query's records and its root resolver node. */
export type Retention = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly root: ResolverNode;
  readonly releaseRoot: CleanupFn;
};

/** Gets or creates the root node of `entrypoint` on `variables` (`Dirty`, no value, no dependencies
 * when created), adds a root hold, and adds the retention to `environment.retainedQueries`. Calls no
 * resolver. */
export function retainEntrypoint<TOutput>(
  environment: IsographEnvironment,
  entrypoint: Entrypoint<TOutput>,
  variables: Variables,
): Retention;

/** Ends `retention`: removes it from `environment.retainedQueries` and releases its root hold. */
export function dispose(environment: IsographEnvironment, retention: Retention): void;

/** What `retainEntrypoint` needs from an `entrypoint.ts` artifact. */
export type Entrypoint<TOutput> = {
  readonly normalizationAst: NormalizationAst;
  readonly readerArtifact: () => ReaderArtifact<ResolverParam, TOutput, EntityName>;
};
```

A `Retention` has `ast` and `variables`, so it is a `RetainedQuery` (`garbageCollection.ts:18-21`), and the store's mark keeps its records. `makeNetworkRequest` and `retainQueryWithoutMakingNetworkRequest` take a `Retention` where they take a `RetainedQuery` (`libs/isograph-runtime/src/network.ts:107-125`, `:163-170`), and their cleanups call `dispose`. This is upstream's `retainQuery` and `unretainQuery` (`garbageCollection.ts:40-63`) with the root hold added.

```text
retainEntrypoint HomeRoute -> HomeRoute absent -> create Dirty, no value -> rootCount 1
HomeRoute response -> writes, marks nothing (no node has read anything)
read HomeRoute -> Dirty -> recompute -> creates and computes PetSummaryCard@Pet:0, @Pet:1, fullName@...
```

The fetch decision. With `shouldFetch` `IfNecessary`, `fetchEntrypoint` pulls the root node at fetch time, before the network call, and fetches when it is `MissingData`; it serves from the store otherwise. `check` stays a plain exported function (`libs/isograph-runtime/src/index.ts:43`).

```ts
// from libs/isograph-runtime/src/network.ts
// fetchEntrypoint, before (libs/isograph-runtime/src/network.ts:95-103):
    case 'IfNecessary':
      return check(environment.store, entrypoint.normalizationAst, variables).kind === 'EnoughData'
        ? retainQueryWithoutMakingNetworkRequest(environment, entrypoint.normalizationAst, variables)
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
// after:
    case 'IfNecessary':
      return readStoredResult(environment.store, entrypoint.readerArtifact(), entrypoint.normalizationAst.root, variables)
        .kind === 'MissingData'
        ? makeNetworkRequest(environment, entrypoint, variables, fetchOptions)
        : retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables);
```

- A repeated call with no write between is a lookup: the root is `Clean`. `check` walks the whole normalization AST on every call.
- A complete read implies the normalization AST's data is present. The normalization AST is the merge of the reader ASTs the entrypoint reaches (query-generation.md, Merging) plus the discriminator, which has no normalization node, and the identity, which `normalize` writes in the pass that creates a record (`crates/isograph_compiler/src/merged_selection.rs:1555-1560`, `crates/isograph_compiler/src/normalization_ast.rs:135-150`). It fetches less than `check` in the cases Absent records names, and in each, every value a reader reads is present.
- `ResolverThrew` at the root is open decision 15.
- A component field's data is read at render (components-in-userland.md), so the root does not cover it; an entrypoint that reaches one needs `check` until that doc settles its open question 3.

## Later: collecting derived nodes

In the core a derived node lives as long as its record container. Later, `collect` disposes every derived node no root reaches through `Derived` dependencies, so a node a plain read created on a retained record, a disposed retention's nodes, and a node a moved link stopped reaching are freed before their records.

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** Disposes every derived node in `store` that no node with `rootCount > 0` and no node in `extraRoots`
 * reaches through `Derived` dependencies, and removes it from its container's `derived`. */
export function collect(store: Store, extraRoots: Iterable<ResolverNode>): void;
```

`garbageCollectEnvironment` calls it after `garbageCollect`. The roots are root holds (retentions and observers) and the subscriptions' nodes (open decision 10). When a node is disposed is open decision 11; whether records are kept by live dependencies instead of retained ASTs is open decision 12; who calls `garbageCollectEnvironment` is open decision 17.

## Later: keeping values up to date automatically

Not part of the core. Observers, subscriptions, and effects are derived nodes that are read again as needed. A write marks; whoever needs a value pulls. No resolver runs during a write, and a retained query nobody observes stays marked until something reads it (the user's choice).

- Subscriptions. A subscription is a derived node whose computation is `readData(contextHooks(store, context), artifact, root, variables)` without the subscription's own resolver, and whose equality is `subscriptionResultsAreEqual` (`libs/isograph-runtime/src/subscribe.ts:135-155`): the data is an object the walk assembled, so it compares by `selectionSetDataAreEqual`. The `Subscription` holds its node, which lives in no container; unsubscribing disposes it. `callSubscriptions` pulls each subscription's node after every write and calls the callback when the pulled value is not `===` `delivered`; a `Clean` node is one step, and a marked one verifies, so the reader AST is walked only when something it read changed. `callSubscriptions` keeps core change 9's call sites. This replaces core change 9's `readSubscription`.
- React. `useSyncExternalStore(subscribe, getSnapshot)`: `listen(node, onChange)` adds an observer and a root hold; after a write, `notify` calls each observer whose node is not `Clean`; React then calls `getSnapshot`, which is `pull(store.graph, node)`. A node whose inputs came back equal is verified without recomputing and returns the same object, so React does not render again. `getSnapshot` pulls during render; an observer whose node stays marked is called on every write until something pulls it.
- Loaders. loaders.md's effect-set nodes are derived nodes that the runtime pulls after every write (`reconcileEffectRoots`), and its loader items are source nodes (loaders.md, An item that changes over time; After a write).
- Callback errors. Every callback, listener, and observer runs through `reportCallbackError` (`libs/isograph-runtime/src/subscribe.ts:159-167`): a throw is rethrown in its own microtask and the loop continues (network-and-subscriptions.md, Decisions, 3).
- Scheduling. Marking is separate from pulling, so a scheduler can decide when each observer's pull runs (Follow-ups 9).

### Values with lifecycles

A computation is pure and runs on every pull, including a pull a discarded render or an uncommitted world (Later: a DAG of graphs) makes. A resource a node's value needs is the node's effect, run apart from its computation: set up when the node is mounted (`listen` at React's commit), torn down when it is unmounted, disposed, or given a new value. After a write, `reconcileEffects` pulls every mounted node with an effect, because an effect whose inputs changed must be replaced even when no reader reads the node. A new effect is set up before the old one is torn down. A replaced or disposed effect is torn down after every observer of that write was notified. A `===` value leaves the effect as is. Effects run only in the committed world. Open decision 13.

## Later: a DAG of graphs

Not part of the core: optimistic layers and forks. The base graph is the core's store. Layer graphs sit above it, each with one parent, and form a tree. A world is a path from the base to a layer.

- Layers are the engine's. A layer, its source overrides, its shadow states, and world-aware reads live in the engine package; the Isograph runtime never reads `value` or `slot` directly (The engine and the Isograph runtime), so giving `readSource`, `sourceValue`, and `stateOf` a world changes the engine alone. `normalize` writing into a layer is the Isograph runtime's, through the engine's layer API.
- Each layer holds source overrides (its value for a source node of the base) and derived shadow states (its `StateSlot` for a derived node).
- Reading in world `W` takes, for both node kinds, the deepest entry on `W`'s path: a source's value from the deepest layer that overrides it, else the base; a derived node's slot from the deepest layer that holds one, else the base. `stateOf(node, world)` is that lookup; no other code changes (reactive-engine.md, Construction and plain reads).
- Placement. A node computed in `W` is stored in the deepest layer on `W`'s path that supplied any of its inputs, a source value or a derived input's slot. That is usually lower than `W`'s top: a node that read nothing the top layer overrides is stored in the base, shared with every world. Valid because the value is a function of its inputs: every world whose path holds that layer reads the same value for each input, unless a deeper layer on its path overrides one, and that override's write marked the node in the deeper layer (next bullet), whose shadow state that world reads first.
- Writes mark in the layer where they happen. A write in layer `L` gives each node above the written source a marked shadow state in `L` (`Dirty` for direct dependents, `Check` above), copied from the state its worlds through `L` read. A base write marks the base states and also the shadow states that descendant layers hold for those nodes.
- Reverse edges are the union across layers: a node is in a source's or node's `dependents` while any layer's slot lists that dependency. Each layer records its contributions, so discarding it removes them.
- Revert discards the layer graph: its overrides and shadow states. The base is untouched, because by the placement rule no base slot read a layer's override.
- Commit turns the layer's overrides into base writes (`writeSource`, which marks the base), and discards its shadow states. Upstream never commits an optimistic layer: `revertOptimisticStoreLayerAndMaybeReplace` empties it and normalizes the real response into a network response layer in its place, re-executes the layers above, and merges down (`optimisticProxy.ts:393-493`; called on response, error, and dispose, `makeNetworkRequest.ts:305-334`).
- Garbage collection roots are (node, layer) pairs; a root hold and an observer belong to a world. Upstream collects only when the base is the only layer (`garbageCollection.ts:65-68`).
- Effects run only in the committed world.

Upstream has stacks, not trees: each layer has one `childStoreLayer` (`optimisticProxy.ts:100-159`); a read walks from the child-most layer to the base and takes the first value, and a `null` record is a deletion (`getStoreRecordProxy`, `optimisticProxy.ts:24-44`; `getMutableStoreRecordProxy`, `:46-98`).

## Later: other derived node kinds

A source other than a server field is a source node its owner holds, written with `writeSource` or marked with `markSourceChanged` by the code that owns the value. User code reaches it only through a selection, so the walk records the read.

- Request state. The state of a request is a source node on its request entry (network-and-subscriptions.md, Request entries in the store). Which request a selection refers to is open decision 14.
- Browser-owned state (a `localStorage` key, a cookie, the URL): the environment mirrors each value as events arrive and writes the mirror's source node. Such a value is current only when every write to it reports a change: the `storage` event does not fire in the window that made the change (MDN, Window: storage event), and a `document.cookie` assignment fires none (the Cookie Store API's `change` event does, MDN, CookieStore: change event). Polling is not an option (AGENTS.md, Wake on events). Not planned.
- Client roots through `with` blocks, a possible later direction whose syntax is not decided:

  ```text
  field User.fullName
  with User { friendName }
  with Root1 { url }
  ```

  A block is allowed when its entity's record is determined by the node's key: the parent entity, or a singleton whose identity is `Fixed` (`CompletedIdentity::Fixed(FixedRecordId)`, `crates/isograph_compiler/src/merged_selection.rs:1450-1458`). A client root's type is static and its values come from a provider `createIsographEnvironment` requires per declared root; each provider selectable is a source node and its subscription writes it. Relay's live resolvers are the precedent: a `LiveState<T>` with `read()` and `subscribe(cb)` (Relay docs, Live Fields). Upstream has no client root and no client-only source.
- Runtime-defined computations (Follow-ups 10) and the suspension decision of react.md are derived nodes whose computations call `ComputeContext` directly; their values are not `ResolverResult`s, and each is a `DerivedNode` of its own value type (reactive-engine.md, Types; open decision 19).

A derived node whose computation reads nothing (a refinement, Selections in the Isograph layer) could later be evaluated in the walk without a node.

## Follow-ups

The user's design notes, recorded for later. None is part of the core, and none is approved.

1. Constructors and destructors at normalization time: a constructor runs when a retained query first reaches a resolver and a destructor when the last reference is released; lifetime follows retain and release, not render. Upstream ties such lifetimes to render (`useDisposableState`, `isograph/libs/isograph-react-disposable-state/src/useDisposableState.ts:16`). It conflicts with retention computing nothing and with Values with lifecycles (open decision 13).
2. A client pointer is a function plus a follow-up query: its resolver returns an id; when the target lacks the data the selection through the pointer needs, a refetch through `node(id:)` starts, as a retained child of the parent retention.
3. A pointer whose target is one of the records it read: query generation spreads the target fragment's selections where the pointer read ids, so no follow-up query is needed, at the cost of overfetching and an enforced guarantee. Upstream merges the pointer's reader selections and refetches the target (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:852-900`, `:1135-1233`).
4. Handles: a reference-counted handle like `PromiseWrapper` plus a cleanup (`ItemCleanupPair`, `isograph/libs/isograph-disposable-types/src/index.ts:2`), readable during render as a lookup.
5. Deduplication: two retained queries that reach the same pointer on the same record share one handle; the key is the record container and the resolver store key.
6. Cancellation: upstream's network function takes no abort signal (`IsographEnvironment.ts:103-106`); one that accepts an `AbortSignal` lets a destructor cancel a follow-up's request.
7. Deferral: construction defers subscription setup to a caller-supplied scheduler; destruction buffers releases and collects in a batch, as upstream's `gcBuffer` does (`garbageCollection.ts:40-53`, default 10 at `IsographEnvironment.ts:177`).
8. Pre-memoized selection sets: a selection set read on a record as its own derived node, keyed by a compiler-emitted key, compared by `selectionSetDataAreEqual`. To explore; the user expects it probably does not matter.
9. Scheduled recalculation: an unobserved node is marked and pulled when read (the core already does this), and a scheduler decides when observed nodes are pulled and at which priority. `useSyncExternalStore` performs a store update as blocking even inside a transition, so deferred rendering needs a deferred snapshot in the React package.
10. Runtime-defined computations: a memoized computation the runtime or a user defines at run time over other derived nodes, which needs a caller-supplied computation and a wider value type (Later: other derived node kinds).

11. An `Embedded` storage kind: a record of an entity with `Embedded` storage is stored as one value in its parent's source node, with no container of its own (`date { hour }` held whole in `Pet:1.date`). Cost: the walk reads fields out of a value as well as out of a record, a second representation the store does not have in v0, and every reader of `date` is marked together when any field of it changes. Benefit: one container and its source nodes fewer per such object. In v0 such an object is a `FromParent` record (Entity tables).

12. Entity tables loaded as needed. In v0 a reader artifact imports the table of every entity its walk reads at, and a table imports the reader artifact of every derived field it lists, so the bundle that holds one reader holds every client field of those entities, and their tables in turn (linked-only-selections.md open decision 4). A table, or each entry's artifact, loaded on demand (`import()`) when the walk first reads at that entity cuts that cost, and needs the walk to wait for a module, which v0's synchronous walk cannot.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: `Store` becomes `Map<EntityName, Map<RecordId, RecordContainer>>`; its functions read and write through the accessors; `normalize` skips unchanged values and marks as it writes; `garbageCollect`'s sweep calls `deleteRecord`; its tests compare `storeSnapshot(store)` with `storeOf(..)`, which returns a `PlainStore`.
- `refactors/pending/runtime/reader-ast.md`: the reader AST is `{ alias, fieldName, arguments, selections }` and the walk reads each field's origin from entity tables (Selections in the Isograph layer); `ReaderArtifact.parentEntityName`, `selectableName`, and `tables`; the walk takes `ReadHooks`; an absent record reads as all keys missing, which changes its two absent-root `reason` assertions and makes `Constant` on an absent record `Ok`; `readStoredResult` is the stored read beside `read`.
- `refactors/pending/runtime/network-and-subscriptions.md`: its "`Store` stays store.md's nested map" no longer holds; the request entry's state is a source node on the entry, not `requestSource(key): SourceId`; `readSubscription` reads nested resolvers through the graph and `SubscriptionResult` gains `ReadResolverThrew` (core change 9); `makeNetworkRequest` re-reads subscriptions in a `finally` after `normalize`, and `garbageCollectEnvironment` re-reads them after the sweep (core change 9).
- `refactors/pending/runtime/react.md`: `unwrapOrSuspend` throws `result.error` on `ResolverThrew`; `getSnapshot` pulls (Later: keeping values up to date automatically), against its rule that a render reads and computes nothing.
- `refactors/pending/runtime/components-in-userland.md`: `useClosureData`'s switch gains `case 'ResolverThrew': throw result.error;`; a closure field's wrapper is a derived node in its record container's `derived`, owned and freed with the record.
- `refactors/pending/language/linked-only-selections.md`: the compiler side of Selections in the Isograph layer (core changes 3, 6, and 7).
- `refactors/pending/language/scalar-entities.md`: its selection shape rule, client fields on scalar entities, query text, and upstream sections moved to linked-only-selections.md and Selections in the Isograph layer; the normalization AST's per-entity imports are planned as entity tables; the rest stays parked.
- `refactors/pending/runtime/store.md`: a scalar value is its own record (Scalar records); `NormalizationSelection.target` is `NormalizationTarget`; a record's identity is its entity table's `storage`, not the normalization AST's `identity`.
- `refactors/pending/runtime/loaders.md`: edited with this rewrite where its references to this doc's sections and `SourceId`s broke.

## Open decisions

### 1. The resolver store key format

A resolver node's key in its container is `resolverStoreKey(artifact, variables)`: `<parentEntityName>.<selectableName>/` and the variables (Stored resolver results).

Decided by the model: the key names the field's entity. There are no abstract entities, and one record is read as several entities: `User:4` read through `actor` is read as `Actor`, and `Actor.foo` and `User.foo` on `User:4` are different fields (Entity tables), so a key without the entity would give both one node. This was option (a); `ReaderArtifact.parentEntityName` carries the entity.

The remaining question is how the key encodes the variables. Upstream's key for a field's cached result on a record is `stableIdForFragmentReference`: `` `${root.__typename}/${root.__link}/${fieldName}/${JSON.stringify(stableCopy(variables))}` `` (`FragmentReference.ts:75-79`). It names the record, the field, and the variables as JSON of their stable copy; it does not use the server store key's argument chunks. The chunk format (`____<name>___<value>`) is upstream's server store key only (`getParentRecordKey`, `cache.ts:454-479`; `:594-595`), and it collides: a GraphQL name may contain `_`, so a field named `F____n___2` read with no variables and `F` read with `{ n: 2 }` both have the chunk key `F____n___2` (`argumentChunks`, `libs/isograph-runtime/src/storeKey.ts:22-34`).

- (a) Upstream's encoding: `` `${parentEntityName}.${selectableName}/${JSON.stringify(stableCopy(variables))}` ``. The container supplies the record, which upstream's key spells as typename and link. Held, not decided. Reason: upstream's behavior, and it is injective: a GraphQL name holds no `.` and no `/`, so `F____n___2` with no variables is `E.F____n___2/{}` and `F` with `{ n: 2 }` is `E.F/{"n":2}`. Cost: the resolver key does not share the server key's format. The server `storeKey` keeps upstream's chunk format; its collision is store.md's.
- (b) The chunk format, `argumentChunks` of the variables after the selectable name. Cost: the `F____n___2` collision; two fields share one stored result, with no error.

The body is written with (a).

### 2. A write during a computation

A resolver receives no store, but one that reaches the store through a closure can call `normalize` while its node computes:

```text
P Dirty -> recompute P: reads C Clean -> P's resolver calls normalize -> write c.1 -> c.1.dependents = {C} -> C Dirty
  -> C.dependents = {} (P is added only after compute returns)
P finishes -> setDependencies: C.dependents = {P} -> P Clean, reading a Dirty C
write c.1 again -> C already Dirty -> stop -> P is never marked
```

A resolver that calls `readStoredResult` through a closure reads with no dependency recorded on the outer node.

- (a) No guard. A closure that reaches the store is undefined behavior.
- (b) A computing depth on the environment, which `readStoredResult` and every write through the environment check and throw on. Cost: a field on the environment, and `normalize`, a store function, is guarded only through its environment callers.
- (c) (b) for writes only. pico guards writes this way: `Database::set`, `remove`, and garbage collection assert an empty dependency stack, panicking with "Cannot modify database while a memoized function is being invoked." (`crates/pico/src/database.rs:151-170`, `:193-198`). Cost: (b)'s field; an inner `readStoredResult` still records nothing on the outer node.
- (d) A write counter read at the start and end of `compute`; when it moved, `recompute` leaves the node `Dirty`. Cost: a counter visible to every write; it repairs the stale `Clean` only.
- (e) After `setDependencies`, `recompute` leaves the node `Dirty` when a recorded derived dependency is not `Clean`. Cost: one check per dependency per recompute; a source the outer computation read before the inner write changed it is not covered.
- (f) A guard in the engine, on an engine object: `Graph` and `createGraph` (reactive-engine.md, The guard). `computingDepth` is a count, not a two-state enum, because `readDerived` recomputes a node inside the computation that reads it; `compute` increments it before the computation runs and decrements it after. The guard throws on writes during a computation only: `writeSource`, `markSourceChanged`, and `disposeDerivedNode` take the graph first and throw while `computingDepth > 0`, as pico asserts an empty dependency stack before every write (`assert_empty_dependency_stack`, `crates/pico/src/database.rs:193-198`, called from `set`, `remove`, `remove_singleton`, and `run_garbage_collection`, `:151-170`); `deleteRecord` reaches the guard through `markSourceChanged` and `disposeDerivedNode`. Untracked reads are not guarded: the engine allows `sourceValue` and a public `pull` inside a computation by decision (open decision 25), and `pull` takes the graph only because `compute` counts on it. The Isograph layer makes no untracked read inside a computation: every read in `resolverComputation` goes through `contextHooks`. In the Isograph runtime, `Store` is `{ readonly graph: Graph; readonly records: Map<EntityName, Map<RecordId, RecordContainer>> }` (The store). The guard is not a module static, so two environments do not share it. Cost: every engine write and `pull` takes a `Graph`; `Store` is an object instead of the bare `Map`; `deleteRecord` of a container with no source and no derived node reaches no engine call and is not guarded, which loses nothing, since no computation read such a container.
- (g) As (f), but each node holds its graph: `SourceNode.graph` and `DerivedNode.graph`, set by the constructors, which take the graph first. `pull`, `writeSource`, `markSourceChanged`, and `disposeDerivedNode` drop their `graph` parameter and read `node.graph` (reactive-engine.md, Option (g)). Held, not decided. Reason: under (f) the guard checks the `Graph` the caller passes, and a source has no graph of its own, so a computation that writes through a closure with any other `Graph` object (`writeSource(createGraph(), s, 2)`) passes the guard and leaves its node `Clean` over a changed input with no error (reactive-engine.md open decision 2, the trace under (f)). Under (g) the write reads the guard from the source it writes, so that write throws, and (g) is the only option that meets the criterion of the last paragraph below, a guard no caller can bypass. `readSource` and `readDerived` also throw on a node of another graph, so no `dependents` edge crosses graphs. Cost: one field per node, and only the constructors take a graph; `sourceNodeAt` and `resolverNodeAt` pass `store.graph`, so `Store` is still (f)'s object; one comparison per context read.

Every graph mutation passes through the engine's `writeSource`, `markSourceChanged`, or `disposeDerivedNode` (`normalize` calls `writeSource`; `garbageCollect`'s sweep calls `deleteRecord`, which calls `markSourceChanged` and `disposeDerivedNode`; a later owner of a source node calls `writeSource` or `markSourceChanged`, Later: other derived node kinds), and every read through `readStoredResult` passes through the engine's `pull`. Each of those takes a node, not the environment. `normalize` and `garbageCollect` are exported from the package index (`libs/isograph-runtime/src/index.ts:42`, `:44`), and `environment.store` is a public field (`libs/isograph-runtime/src/environment.ts:10`). So a closure that holds the store or the environment reaches those functions without passing an environment-level check, and the guards of (b) and (c) are bypassable. A guard that no caller can bypass is checked in the engine's write functions, which is where (f) and (g) put it. Because the engine is usable without Isograph, the guard belongs to the engine: a guard in the Isograph runtime would leave a standalone consumer's write during a computation unguarded.

(g) is held, not decided, as in `refactors/pending/runtime/reactive-engine.md` open decision 2. The body writes the engine calls in the (f) form, as reactive-engine.md's body does, and follows reactive-engine.md's form for whichever option is decided (The engine).

### 3. A throw inside a computation outside its resolver (decided)

Decided by the user: a computation must not throw (reactive-engine.md, Throws; Stored resolver results).

### 4. A response that throws part-way

`normalize` writes in place and throws at the first position that does not match the AST (store.md, Errors).

- (a) `normalize` marks each write as it makes it (`writeValue`), and its throw propagates as v1's error; the writes before it and their marks stand. `makeNetworkRequest` re-reads subscriptions in a `finally` after `normalize` (core change 9). Held: upstream keeps a partial write too (`normalizeData` writes in place, `cache.ts:77-85`; a throw reaches `makeNetworkRequest`'s `.catch`, `makeNetworkRequest.ts:299-319`).
- (b) `normalize` validates the whole response before it writes. Cost: a second walk of every response.
- (c) `normalize` writes into a layer (Later: a DAG of graphs) that is committed when the walk ends and discarded on a throw. Cost: needs layers; a malformed response writes nothing.

### 5. A resolver that throws

- (a) Stored as `ResolverThrew`, a value that a parent's walk passes up as it passes `MissingData` (held). Reason: the throw is memoized like any value: the next read returns it without calling the resolver, and it changes only when an input changes. Deviation from upstream, which propagates the throw out of the read (`read.ts:600`).
- (b) Propagate, v1's and upstream's behavior. Cost: nothing is stored, so the node stays `Dirty` and every read calls the resolver again and throws.

### 6. Whether `readDataForOnComplete` reads through the graph

`readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:196-218`) calls `read`, so it calls the entrypoint's resolver and every nested resolver once more per response; a nested resolver's throw makes the request `Err`; `MissingData` throws "This is indicative of a bug in Isograph.". Upstream does the same (`makeNetworkRequest.ts:362-420`).

- (a) Unchanged: `read`. Held: upstream's behavior. Cost: the resolvers run again, and the values `onComplete` receives are not the stored ones.
- (b) `readData(storedResultHooks(store), ..)`, then the entrypoint's resolver. Cost: a nested `ResolverThrew` is a value, so `readDataForOnComplete` rethrows its `error` to keep the request `Err`, or `onComplete`'s type admits it.
- (c) `readStoredResult` of the entrypoint at its root. Cost: (b)'s rethrow for `ResolverThrew`, and the root node it creates lives until `Query.__ROOT` is deleted.

### 7. How a field supplies its output equality

`resolverNodeAt` passes `identical` (`===`) as `outputEqual`. A user who returns a new object per call (`{ length: data.name.length }`) and wants cutoff supplies a comparator, such as a deep equality. A comparator must answer equal only for values no reader can tell apart (Equality).

- (a) A second argument to the function a field literal returns: `iso(`field Pet.Shape { name }`)(resolver, { equal: deepEqual })`. The Babel plugin replaces a field literal with `x => x` (`libs/isograph-babel-plugin/compileTag.js:53`); the replacement becomes `(resolver, options) => Object.assign(resolver, options)`, so the export is still the resolver function, now carrying `equal`. The generated `IdentityWithParam` (`fixtures/store/src/__isograph/iso.ts:44-48`) gains the optional parameter and returns the resolver's type with `readonly equal?: (previous: TOutput, next: TOutput) => boolean`, and `resolverNodeAt` passes `artifact.resolver.equal ?? identical`. Held, not decided. Reason: it is the simplest: no directive, no new iso syntax, no compiler change beyond the generated `iso` overload, and the comparator sits next to the resolver it belongs to. Cost: a property written onto the user's function object, and an optional property on the resolver's type.
- (b) A declaration directive naming a built-in comparator (`field Pet.Shape @equality(kind: Deep) { name }`), from which the compiler writes `outputEqual: deepEqual` imported from the runtime. Cost: i2 accepts no declaration directive yet (`DeclarationDirectiveName` is empty, `crates/isograph_parser/src/directives.rs:38-41`); only built-in comparators.
- (c) A map from selectable to comparator passed to `createIsographEnvironment`. Cost: the comparator is far from its field and keyed by a string the compiler does not check.
- (d) No API: a resolver that wants cutoff returns a stable value itself. Cost: no cutoff for object outputs without user memoization.

Upstream compares resolver values by `===` only (`areEqualWithDeepComparison.ts:84-95`).

### 8. A cycle of derived nodes (decided)

Decided by the user: cycles are avoided at compile time, and the engine has no cycle check. Every runtime edge from one derived node to another, `A@X -> B@Y`, exists only because `A`'s selection set selects `B`: the walk creates or reads a derived node only for a selection in the reader AST it walks, at whichever record a link leads to. So a cycle of nodes at run time, including one that depends on data (`field Pet.Card { bestFriend { Card } }` with `Pet:1` and `Pet:2` each other's best friend), implies a cycle of field selections in the source, which the compiler rejects as `InlineCycle` ("These fields inline each other", `crates/isograph_compiler/src/merged_selection.rs:240`, reported for the field at `:803-809`). `inline_cycle` (`:1352-1383`) searches `inlined_fields` (`:1281-1298`), which lists every client field a field's selection set selects at any depth under protocol selections (`inlined_in_selection`, `:1314-1348`); `Card` selects `Card` under `bestFriend`, so `Pet.Card -> Pet.Card` is reported.

The other derived fields do not break the argument:

- A generated refinement field (`Node.asPet`, Selections in the Isograph layer) has an empty reader AST, so its node reads no derived node and closes no cycle. The selections under `asPet { .. }` belong to the parent's selection set; a refinement is a protocol selection, so `inlined_in_selection` descends into it (`:1325-1338`).
- `__self` is not a derived node: the walk reads it as the current record. `__self { F }` inside `field E.F` selects `F` on `E`; the change that adds `__self` (core change 7) adds a `SelfRecord` arm to `inlined_in_selection` that descends into its selection set on the same entity, so `E.F -> E.F` is `InlineCycle`.
- A client pointer is not read in v0 (`SelectionRead::NotRead`), and `inlined_in_selection` lists nothing under one (`:1346`). The change that reads pointers adds the pointer field and the fields under its selection set to `inlined_fields`, because a pointer node and the fields read at its target are then edges of the reading node.

### 9. An unreached `Absent` container whose source nodes have readers

A reader that found a record absent holds a source node in that record's `Absent` container. When no retained query's links reach the container, the sweep deletes it.

- (a) Delete it and mark its readers, as for any record (`deleteRecord`). The next pull of a reader re-creates the container and recomputes to an equal `MissingData`, which cuts off. Held: the user's rule that deleting a record marks the dependents of each of its source nodes; and upstream deletes every unreached record (`garbageCollection.ts:88-119`). Cost: each collection recomputes the readers of unreached absent records.
- (b) Keep an unreached `Absent` container while any of its source nodes has a dependent or it owns a derived node; delete it once none does. Cost: a container lives as long as its readers, which `collect` (later) or the readers' recomputation must end.

Deleting it without marking is not an option: a reader left `Clean` holding `MissingData` would never see the record appear, because `normalize` would write a new source node.

### 10. How `collect` keeps the derived nodes subscriptions read

A subscription's node holds no root hold, so without a rule `collect` disposes it and every node below it, and the next re-read recomputes them. Upstream stores no resolver results and has no such rule; its subscriptions record what their last read visited to skip re-reads (`encounteredDataAndRecords`, `IsographEnvironment.ts:49`, `subscribe.ts:160`).

- (a) `garbageCollectEnvironment` passes every current subscription's node to `collect` as `extraRoots`, read from `environment.subscriptions` when it runs. Held, not decided. Reason: unsubscribing needs no release, and nothing is left to balance when a read throws. Cost: a second parameter on `collect`.
- (b) Each subscription holds a root hold on its node, released on unsubscribe. Cost: a hold per subscription; benefit: `rootCount` is the one root mechanism, which open decision 11 (b) needs.

### 11. When a derived node no root reaches is disposed

- (a) By `collect`, when `garbageCollectEnvironment` runs (held). A marked node that nothing pulls costs nothing until then.
- (b) By reference count: `rootCount` plus `dependents.size`; at zero the node is disposed, and disposal decrements what it read. Cost: a count kept on every hold, release, and `setDependencies`, and disposal in the middle of a pull. It is what Follow-ups 1's destructors need.

### 12. Records kept by live dependencies

The user's idea, explicitly unsettled: a record is live while a live derived node reads one of its source nodes, so collection needs no AST walk.

- (a) Walk retained normalization ASTs, as v1 and upstream do (`garbageCollection.ts:88-119`). Held.
- (b) A container is live while a source node in it has a live dependent; the edges already exist. Cost: a record a retained query fetched but no live node read is deleted (fields after a `MissingData`, records a reader stops before), so `check` of that query reports `MissingData` and a refetch follows; retention no longer means what store.md and upstream mean.

### 13. Values with lifecycles

- (a) Computation and effect are split (Values with lifecycles; the user's direction, held). Cost: a resource exists only while its node is mounted, so a retained query nobody renders acquires nothing, which conflicts with Follow-ups 1.
- (b) The computation constructs: it returns an `ItemCleanupPair` the engine holds and releases when the value is replaced or the node disposed. Not held: a computation runs in every pull, including a discarded render's and an uncommitted world's, so it acquires resources whose release nothing triggers.
- (c) A registry outside the engine that the resolver's code calls. Cost: nothing releases a handle when the node's value is replaced, so the registry needs its own reachability.

loaders.md places lifecycles per loader instance instead (loaders.md, What this changes in other docs). Not decided: the user's choice.

### 14. Which request a request-state selection refers to

A derived node is keyed by (record, field, variables), not by request, so "the request that fetched me" has no single value for a node shared by two entrypoints. A key the reader's location determines works: a loadable field's request is keyed by its query text and operation variables (network-and-subscriptions.md, The request cache), a function of (record, field, arguments). The state of an entrypoint's own request stays with the caller of `fetchEntrypoint`, which holds the `PromiseWrapper`. Not decided.

### 15. `ResolverThrew` at the root and the fetch decision

The walk stops at a nested `ResolverThrew` (`allOk`), so data missing after the throwing resolver does not show in the root's result.

- (a) `ResolverThrew` counts as present: no fetch. The user's statement. Cost: a fetch `check` would make is skipped, and that fetch could change the throwing resolver's inputs.
- (b) `ResolverThrew` counts as a reason to fetch. Cost: a request on every call while the resolver throws.

Held (a), not settled.

### 16. Observers at render against react.md's render-time subscription

react.md and the landed runtime create a component's subscription at render under a temporary retain and retain it at commit (`refactors/pending/runtime/react.md:359`, `:864-877`; network-and-subscriptions.md, The two-phase lifetime). `listen` runs in React's `subscribe`, at commit, so between render and commit the node has no root hold and `collect` can dispose it. Candidates: no render-time step; a render-time root hold under react.md's temporary retain, with `listen` adding only the observer at commit. Not held: no doc settles it.

### 17. Who calls `garbageCollectEnvironment`

It runs only when a caller calls it (`libs/isograph-runtime/src/environment.ts:62-66`); releasing a request does not collect (network-and-subscriptions.md). It is the only function that deletes records and, with `collect`, disposes derived nodes. When the React package calls it is react.md's open decision 1 (`refactors/pending/runtime/react.md:1297-1305`): after every disposal, never automatically, or upstream's buffer of 10. This doc adds no option of its own.

### 18. The engine package's name (decided)

Decided by the user: `@isograph/reactive-graph`, directory `libs/reactive-graph`. No workspace or upstream package uses the name (`libs/*/package.json`, `../isograph/libs/*/package.json`). Whether the name is free on the npm registry was not verifiable offline; the package is `private: true` (reactive-engine.md, The package), so nothing is published under it.

### 19. How the engine types the links between nodes of different value types

A standalone engine holds nodes of different value types in one graph (a `DerivedNode<string>` that reads a `DerivedNode<number>`), so every `dependents` set and every `Derived` dependency holds a node whose value type is erased.

- (a) `AnyDerivedNode = DerivedNode<unknown>`, with `equal` a method signature so that `DerivedNode<T>` is assignable to it (reactive-engine.md, Types). Held, not decided. Reason: no cast and no wrapper; the engine calls `equal` only with two values the node's own computation produced. Cost: TypeScript's method bivariance also accepts a call such as `(node as AnyDerivedNode).equal('a', 'b')` on a `DerivedNode<number>`; only code outside the engine could make it, and the API gives such code no reason to call `equal`.
- (b) One value type per graph, the previous revision's `SourceNode<TSourceValue, TValue>` and `DerivedNode<TValue>` with `TValue` shared by every node. Cost: a project whose nodes have different value types declares a union and narrows every `readDerived` result; Isograph's own later runtime-defined computations widen `ResolverResult` the same way (Later: other derived node kinds).
- (c) An existential encoding: a link holds `<R>(visit: <T>(node: DerivedNode<T>) => R) => R`. Cost: one closure per link and a call per traversal step; it is sound.

### 20. The engine's identity equality (decided)

Decided by the user: the engine has no default equality and exports no comparison; `equal` is a required argument of `createDerivedNode` and of `createSourceNode` (`refactors/pending/runtime/reactive-engine.md` open decision 6). The Isograph layer does not depend on an engine default: every resolver node passes `resolverResultsEqual(artifact, outputEqual)`, whose `outputEqual` is `===` (`identical`, upstream's comparison of resolver values, `areEqualWithDeepComparison.ts:84-95`) unless the field supplies one, beside the structural rule for walk-assembled data (Equality); every store source node passes `isUnchanged(key)` (The store).

### 21. Whether a consumer can write a node's fields

`SourceNode` and `DerivedNode` are plain object types, and their fields are visible to every consumer of the package. A consumer that writes `dependents` breaks the rule that the `dependents` sets are the inverse of the dependency lists (reactive-engine.md, Dependencies), and the result is a silently stale value.

- (a) Plain object types; each field's comment names its only reader and writer (reactive-engine.md, Types). Held, not decided. Reason: (b) and (c) each cost a cast or a sound-but-slower link type. Cost: the field comments are the only guard, against AGENTS.md's "Never rely on discipline what we can enforce with newtypes", once the engine has consumers outside this repo.
- (b) Classes with ES private fields (AGENTS.md, TypeScript): `class SourceNode<TValue> { #value; #dependents }` and `class DerivedNode<TValue> { #computation; #equal; #slot; #dependents }`. `setDependencies` and `markSourceChanged` read another class's private fields through module-local functions assigned in each class's `static {}` block (ES2022, `tsconfig.build.json`'s `target`). Cost: a private field `#equal: (previous: TValue, next: TValue) => boolean` makes `DerivedNode<number>` not assignable to `DerivedNode<unknown>` (checked with this repo's `tsc` 5.6.3), so the erased link of open decision 19 (a) no longer type-checks; (b) needs open decision 19 (c) or a cast at each link. A node cannot be wrapped in a `Proxy` (AGENTS.md, TypeScript).
- (c) Opaque public types: the index exports `SourceNode<TValue>` and `DerivedNode<TValue>` as branded types with no visible fields, and `graph.ts` casts each to its field type. Cost: a cast in every engine function that receives a node.

### 22. `storeSnapshot` and scalar records

- (a) `storeSnapshot` returns the store as stored: each scalar value is a record of its scalar entity with `__value`, and the parent's key holds the link. Held. Reason: the snapshot is the store's server data, and the runtime cannot tell a scalar record from another record without a field that records it. Cost: every store test fixture (`storeOf`, `test/fixtures.ts`) lists scalar records.
- (b) `RecordContainer` gains `origin: 'Response' | 'ScalarValue'`, written by `leafRecords`, and `storeSnapshot` inlines each `ScalarValue` record's `__value` into the key that links to it. Cost: a field on every container read only by the snapshot; `storeFrom(plain)` still cannot tell which plain values are scalars, so a store built from a `PlainStore` needs the normalization AST.

### 23. Whether `writeSource` compares the new value with the old (decided)

`refactors/pending/runtime/reactive-engine.md` open decision 8. Decided by the user: writing an equal value does not propagate. `createSourceNode(value, equal)` takes a required equality and `writeSource` marks nothing when it accepts (reactive-engine.md, Mark). The store passes `isUnchanged(key)` for every source node (The store), so `normalize` compares nothing itself.

### 24. A pull of a disposed node

`refactors/pending/runtime/reactive-engine.md` open decision 9.

- (a) Disposal resets the node to never computed; a later pull computes it from scratch and reattaches its edges (reactive-engine.md, Disposal). Held. Reason: pico's behavior; the next call of a collected memo computes it again (`crates/pico/tests/absent_source/garbage_collection.rs:46`). Cost: a consumer that pulls a node it disposed gets a working node again instead of an error. In the Isograph runtime no code pulls a node after `deleteRecord` disposed it: the node is removed from its container's `derived`, and the next read creates a new node.
- (b) A fourth state, `Disposed`; `pull` throws on it, and so does a dependent's recomputation that reads it. Cost: every `switch` on the mark gains a case, and a dependent marked `Dirty` by the disposal throws on its next pull when its computation still reads the node.

### 25. An untracked read inside a computation (decided)

`refactors/pending/runtime/reactive-engine.md` open decision 10. Decided by the user: allowed. `sourceValue` and a `pull` called inside a computation outside its context read without recording a dependency, and the guard does not reject them (open decision 2 (f)). The Isograph layer makes no untracked read inside a computation: `resolverComputation` reads only through `contextHooks`.

### 26. Whether the engine package ships before its first caller (decided)

`refactors/pending/runtime/reactive-engine.md` open decision 11. Decided by the user: `@isograph/reactive-graph` ships alone, as its own core change (Core changes), with its tests and before the Isograph runtime uses it. Its reader is the stated goal that the engine is usable by projects outside Isograph (Goal).

### 27. How the walk gives `__self` its value

`__self` is a field of every entity whose value is the current record's link, read as the same entity (Refinements). Upstream's `__link` is a client field the compiler adds to every server object entity (`get_link_fields`, `add_link_fields.rs:14-38` in `../isograph/crates/isograph_schema/src/validated_isograph_schema`), read by a reader AST node of its own kind that assigns the root link (`case 'Link'`, `../isograph/libs/isograph-react/src/core/read.ts:189-192`).

Why the name differs from upstream's. `__link` is a `ClientScalarSelectable` with `ClientFieldVariant::Link` (`add_link_fields.rs:22-34`): its value is the record's link, `{ __link, __typename }`, and it takes no selection set. `__self` yields what the record is, not a pointer to it: on an object entity it takes a selection set and yields the data read at the current record (`__self { name }` is `{ name }`), and on a scalar entity a bare `__self` yields the value (`data.__self` is the string in `field String.toUppercase { __self }`, linked-only-selections.md, `__self`). A selection named `__link` that yields data, and a value on a scalar entity, where upstream has no `__link`, would give upstream's name a second meaning. The doc keeps `__self`. The user may choose to reuse `__link` for this field instead; that changes the reserved name in `fieldOf`, `selectionSetDataAreEqual`, and linked-only-selections.md's `SelfRecord`, and collides with no generated path except `E/__link/output_type.ts`, which i2 already writes for the resolver's `link` type (`crates/typescript_artifacts/src/generate_types.rs:43-49`).

- (a) The walk recognizes the field name `__self` (`fieldOf` returns `{ kind: 'Self' }`), the minimal AST's form of upstream's reader kind. Held. Reason: upstream's behavior; no node, no source, and no table entry. Cost: one reserved field name the walk and `selectionSetDataAreEqual` test for.
- (b) A generated derived field `__self` in every table, whose resolver returns `link`. Cost: one derived node per record per reader of `__self`, and an entry in every table.
- (c) A source `normalize` writes on every record, holding its own link. Cost: one source node per record, written once.

### 28. A `Derived` entry for a client field whose value is opaque

A client field without `to` has no target entity (`SelectableKind::ClientField`, `crates/isograph_compiler/src/schema.rs:84-85`), and a refinement or client pointer has one.

- (a) `target: EntityName | null`: `null` for an opaque client field, and the walk returns its value as is, never inspecting it (The walk). Held. Reason: the table records what the compiler knows, and the walk needs no test of the value's shape. Cost: a nullable field on every `Derived` entry.
- (b) `target: EntityName` on every `Derived` entry, and the walk treats any value that is shaped like a link (an object whose own keys are exactly `entityName` and `id`, both strings, `linkOf`, `libs/isograph-runtime/src/store.ts:88-97`) as a link and every other value as opaque. Cost: an opaque output shaped like a link, such as a resolver that returns its `link`, is read as a record and reaches the reader as `EMPTY_DATA`; and an opaque client field needs a target entity the compiler does not have.

### 29. What a bare selection reads

A bare selection of a scalar reads its record's `__value`; a bare selection of an object reads nothing.

- (a) The entity table's `shape` says which (`Leaf` or `Composite`, Entity tables). Held. Reason: it is a property of the entity, which the compiler knows (`EntityShape`), and the walk then reads `__value` only on scalar records. Cost: one field per table.
- (b) The walk reads `__value` at every bare-selected record and treats `MISSING` as an object, `EMPTY_DATA`. Cost: an absent scalar record reads as `EMPTY_DATA` where it is missing data, and every bare-selected object record gains a `MISSING` `__value` source node.

### 30. The guard's error inside a resolver

A resolver that writes the store through a closure (`normalize(store, ..)` while its node computes) makes the engine throw `GraphWriteDuringComputationError` out of `writeSource` (reactive-engine.md, The guard). The error leaves the resolver, so it reaches `callResolver`'s `catch`. The write was rejected, and no engine state changed.

- (a) `callResolver` rethrows `GraphWriteDuringComputationError` and catches every other throw into `ResolverThrew` (Stored resolver results). The throw leaves the computation; the engine leaves the node's state untouched (reactive-engine.md, Throws), so the node stays `Dirty`, and the throw propagates out of `readStoredResult`; a subscription re-read catches it as `ReadErr` (`readSubscription`, `libs/isograph-runtime/src/subscribe.ts:126-130`). Held, not decided. Reason: a write during a computation is a programmer error (reactive-engine.md, The package README, Rules), and a programmer error must not be memoized as data. Stored as `ResolverThrew`, it would be the node's `Clean` value, returned on every read without calling the resolver and passed up by every parent's walk as a resolver's ordinary failure, until an input of the node changed. Cost: the engine exports one error class, and `callResolver` tests the class of every caught throw.
- (b) `callResolver` catches every throw into `ResolverThrew`, the guard's error included. Cost: the guard's error is memoized as data, as above, and no caller of `readStoredResult` sees a throw for a write the engine rejected.
