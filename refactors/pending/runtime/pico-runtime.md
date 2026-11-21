# Runtime: resolver results in one graph

Status: under discussion. Not approved for implementation. This is the planned runtime step after v1 (`refactors/pending/runtime/store.md`, `refactors/pending/runtime/reader-ast.md`, `refactors/pending/runtime/network-and-subscriptions.md`). It describes what changes from the landed code in `libs/isograph-runtime/src`.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9b81e0b6`; `libs/isograph-react/src` last changed at `c09c9dbf`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`. pico paths are under `crates/pico/src` of this repo.

## Goal

Everything is graph. A server value is a source node in the store, and a resolver result is a derived node beside it in the same record container. Every edge is an object reference in both directions. A write sets a source node's value and marks the derived nodes above it, and runs no resolver. A read pulls: it brings a marked derived node up to date, and recomputes it only when an input it read changed value (short circuit). A resolver is called again on a pull only when its node is `Dirty`: a source node its last computation read was written with a value `normalize` found changed, a derived node it read recomputed to a value its `equal` rejected, the record container of a source node it read was deleted, a derived node it read was disposed, the record that owns its node was deleted (the next read creates a new node), or its last computation threw outside the resolver (open decision 3).

The core is a single graph with reads and short circuit. Everything that keeps values up to date without a read (observers, subscriptions as derived nodes, effects), retention, the collection of unreached derived nodes, and optimistic layers are later sections, built on the core and not part of its first implementation.

Upstream stores no resolver result. Its store is plain values per record (`StoreRecord`, `IsographEnvironment.ts:150-158`), a read calls every resolver it reaches (`read.ts:600`), and after a write `callSubscriptions` re-reads every subscription whose records overlap the written ones (`subscribe.ts:63-109`). The deviation is the user's: one structure for values and results, so that one layering mechanism serves Later: a DAG of graphs, and so that every edge is an identity edge.

## The running example

Two records, `root` and `other`; `root.2` holds the link to `other`. Two fields on `root`:

```text
FP reads: root.1, root.2, other.1, other.2, FC
FC reads: root.2, root.3, other.2, other.3
```

`root.2` is the source node of key `2` on record `root`. `FP` and `FC` are the derived nodes of those fields on `root`. `X.dependents` is the set of derived nodes whose last computation read `X`. The traces are in Traces, after the types.

## The graph

pico's shape: a source node holds a value that the outside world sets, and a derived node holds the value of a computation over source nodes and other derived nodes (`source.rs:83-86`, `derived_node.rs:55-58`, both kept by one database, `database.rs:78-80`). pico finds nodes by id and keeps forward edges only. Here every edge is an object reference, and each node holds both directions: a derived node's `dependencies` point down, and every node's `dependents` point up. Deviation reason: marking walks `dependents` from the written node, so a write costs the nodes it reaches, not the live graph (pico's epoch verification walks every dependency under the demanded memo).

```ts
// from libs/isograph-runtime/src/graph.ts
/** A value set from outside the graph, and the derived nodes whose last computation read it. */
export type SourceNode<TSourceValue, TValue> = {
  /** Written only by `writeSource`. */
  value: TSourceValue;
  /** Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

/** A computation's value, kept until an input it read changes. */
export type DerivedNode<TValue> = {
  readonly computation: Computation<TValue>;
  /** Whether two values of this node are equal, for cutoff. */
  readonly equal: (previous: TValue, next: TValue) => boolean;
  /** Value, mark, and dependencies. Read and written only through `stateOf`. */
  readonly slot: StateSlot<TValue>;
  /** Every derived node whose dependencies hold this one. Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

/** The holder of one derived node's state. */
export type StateSlot<TValue> = { state: DerivedNodeState<TValue> };

/** What the node's last computation established, and whether it is still current. */
export type DerivedNodeState<TValue> = CleanState<TValue> | CheckState<TValue> | DirtyState<TValue>;

/** `value` is what a computation would return now. */
export type CleanState<TValue> = {
  readonly mark: 'Clean';
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency<TValue>>;
};

/** A derived node this one reads, directly or transitively, is `Dirty`. */
export type CheckState<TValue> = {
  readonly mark: 'Check';
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency<TValue>>;
};

/** A source node it read changed, or a derived node it read recomputed to a value `equal` rejects,
 * or it was never computed. */
export type DirtyState<TValue> = {
  readonly mark: 'Dirty';
  readonly previous: PreviousValue<TValue>;
  readonly dependencies: ReadonlyArray<Dependency<TValue>>;
};

/** `NoValue`: the node was created and not yet computed. */
export type PreviousValue<TValue> =
  | { readonly kind: 'NoValue' }
  | { readonly kind: 'Value'; readonly value: TValue };

/** One read of the node's last computation, in read order. */
export type Dependency<TValue> =
  | { readonly kind: 'Source'; readonly source: SourceNode<unknown, TValue> }
  | { readonly kind: 'Derived'; readonly node: DerivedNode<TValue> };

/** A derived node's computation. It reports every node it reads through `context`. */
export type Computation<TValue> = (context: ComputeContext<TValue>) => TValue;

export type ComputeContext<TValue> = {
  /** Records that the computation read `source`. */
  readSource(source: SourceNode<unknown, TValue>): void;
  /** Pulls `node`, records the dependency, and returns its value. */
  readDerived(node: DerivedNode<TValue>): TValue;
};
```

The marks are Reactively's `CacheClean`, `CacheCheck`, and `CacheDirty` (`packages/core/src/core.ts` of `milomg/reactively`); MobX's `UP_TO_DATE_`, `POSSIBLY_STALE_`, and `STALE_` are the same scheme (`packages/mobx/src/core/derivation.ts`). Only a `Clean` node becomes `Check`, so `CheckState` always has a value; a `Dirty` node created and never computed has none. `TValue` is one type per graph: `ResolverResult<unknown>` in the core (Equality).

`graph.ts` imports nothing from the store, the reader, or the environment, and names no record, key, or artifact.

```ts
// from libs/isograph-runtime/src/graph.ts
/** The slot of `node`'s state. Every read and write of a node's value, mark, and dependencies goes
 * through it. */
export function stateOf<TValue>(node: DerivedNode<TValue>): StateSlot<TValue> {
  return node.slot;
}

/** A derived node that was never computed: `Dirty`, no value, no dependencies. */
export function createDerivedNode<TValue>(
  computation: Computation<TValue>,
  equal: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue> {
  return {
    computation,
    equal,
    slot: { state: { mark: 'Dirty', previous: { kind: 'NoValue' }, dependencies: [] } },
    dependents: new Set(),
  };
}

/** Sets `source.value` and marks its dependents. The caller decided the value changed. */
export function writeSource<TSourceValue, TValue>(
  source: SourceNode<TSourceValue, TValue>,
  value: TSourceValue,
): void {
  source.value = value;
  markSourceChanged(source);
}

/** Marks every dependent of `source` `Dirty` and every node above those `Check`. Runs no computation. */
export function markSourceChanged<TValue>(source: SourceNode<unknown, TValue>): void {
  source.dependents.forEach(markDirty);
}

/** `node`'s current value: `Clean` returns it, `Check` verifies, `Dirty` recomputes. */
export function pull<TValue>(node: DerivedNode<TValue>): TValue;

/** Removes `node` from the dependents of everything it read, and marks its dependents `Dirty`. */
export function disposeDerivedNode<TValue>(node: DerivedNode<TValue>): void {
  setDependencies(node, stateOf(node).state.dependencies, []);
  node.dependents.forEach(markDirty);
}
```

`stateOf` is the one accessor of per-node state so that Later: a DAG of graphs can give a node one slot per layer by changing `stateOf` alone. `computation`, `equal`, and `dependents` are not per world and stay on the node.

### Mark

```ts
// from libs/isograph-runtime/src/graph.ts
/** `Clean`: becomes `Dirty` and marks its dependents `Check`. `Check`: becomes `Dirty`; its dependents
 * are already marked. `Dirty`: unchanged. */
function markDirty<TValue>(node: DerivedNode<TValue>): void {
  const slot = stateOf(node);
  const state = slot.state;
  switch (state.mark) {
    case 'Clean':
      slot.state = { mark: 'Dirty', previous: { kind: 'Value', value: state.value }, dependencies: state.dependencies };
      node.dependents.forEach(markCheck);
      return;
    case 'Check':
      slot.state = { mark: 'Dirty', previous: { kind: 'Value', value: state.value }, dependencies: state.dependencies };
      return;
    case 'Dirty':
      return;
  }
}

/** A `Clean` node becomes `Check` and marks its dependents `Check`. A marked node is left as is. */
function markCheck<TValue>(node: DerivedNode<TValue>): void {
  const slot = stateOf(node);
  const state = slot.state;
  if (state.mark !== 'Clean') {
    return;
  }
  slot.state = { mark: 'Check', value: state.value, dependencies: state.dependencies };
  node.dependents.forEach(markCheck);
}
```

A write marks the direct dependents of each source node it changed `Dirty` and their transitive dependents `Check`, stopping at a node already marked, and runs no resolver. Marking is transitive because nothing computes during a write: whether a derived node's value changes is known only by computing it.

Invariant, outside a pull: every dependent of a node that is not `Clean` is not `Clean`. Marking keeps it, because a node that leaves `Clean` marks its dependents. A dependent is added only by a computation, which pulled the node first, and a pulled node ends `Clean`. So `markCheck` may stop at a marked node, and a second write to a source costs one step per direct dependent.

### Pull

```ts
// from libs/isograph-runtime/src/graph.ts
export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  const state = stateOf(node).state;
  switch (state.mark) {
    case 'Clean':
      return state.value;
    case 'Check':
      return verify(node, state);
    case 'Dirty':
      return recompute(node, state);
  }
}

/** Pulls each `Derived` dependency in recorded order until one recomputes to a value its `equal`
 * rejects, which marks `node` `Dirty`; then recomputes. Otherwise `node` ends `Clean` without running. */
function verify<TValue>(node: DerivedNode<TValue>, state: CheckState<TValue>): TValue {
  state.dependencies.some((dependency) => {
    if (dependency.kind === 'Derived') {
      pull(dependency.node);
    }
    return stateOf(node).state.mark === 'Dirty';
  });
  const after = stateOf(node).state;
  switch (after.mark) {
    case 'Dirty':
      return recompute(node, after);
    case 'Check':
      stateOf(node).state = { mark: 'Clean', value: after.value, dependencies: after.dependencies };
      return after.value;
    case 'Clean':
      return after.value;
  }
}

/** Runs the computation, compares its value with the previous one, replaces the dependencies, and
 * stores the new value unless `equal` accepted it, in which case the previous value object stays. A
 * new value marks the dependents `Dirty`. A throw out of the computation or out of `equal` leaves the
 * state and every `dependents` set untouched. */
function recompute<TValue>(node: DerivedNode<TValue>, state: DirtyState<TValue>): TValue {
  const computed = compute(node.computation);
  const previous = state.previous;
  if (previous.kind === 'Value' && node.equal(previous.value, computed.value)) {
    setDependencies(node, state.dependencies, computed.dependencies);
    stateOf(node).state = { mark: 'Clean', value: previous.value, dependencies: computed.dependencies };
    return previous.value;
  }
  setDependencies(node, state.dependencies, computed.dependencies);
  stateOf(node).state = { mark: 'Clean', value: computed.value, dependencies: computed.dependencies };
  node.dependents.forEach(markDirty);
  return computed.value;
}

type Computed<TValue> = {
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency<TValue>>;
};

/** Runs `computation` with a context that collects what it reads, in read order. */
function compute<TValue>(computation: Computation<TValue>): Computed<TValue> {
  const dependencies: Dependency<TValue>[] = [];
  const value = computation({
    readSource: (source) => {
      dependencies.push({ kind: 'Source', source });
    },
    readDerived: (node) => {
      const read = pull(node);
      dependencies.push({ kind: 'Derived', node });
      return read;
    },
  });
  return { value, dependencies };
}

/** Removes `node` from the dependents of each of `previous` and adds it to those of each of `next`. */
function setDependencies<TValue>(
  node: DerivedNode<TValue>,
  previous: ReadonlyArray<Dependency<TValue>>,
  next: ReadonlyArray<Dependency<TValue>>,
): void;
```

- `Clean` returns. `Dirty` recomputes. `Check` brings its derived dependencies up to date in recorded order, stops at the first whose value changed (that dependency's `recompute` marked this node `Dirty`), and recomputes; when none changed, it ends `Clean` without running. That is the short circuit, the cutoff on the pull, which is the user's specification.
- Source dependencies are never checked during the walk. A changed source already marked its readers `Dirty`, so a `Check` node's sources did not change.
- The walk in recorded order is sound when a recomputation would read other nodes: a computation's later reads depend only on what it read earlier, so up to the first dependency whose value changed, the recorded reads are the reads a recomputation would make.
- A node recomputes at most once per write. After its pull it is `Clean`, and during pulls only `recompute`'s marking of dependents runs, which reaches only nodes that are not `Clean` (the invariant). The parent whose read reached the node is verifying or computing: marking it `Dirty` makes a verifying parent recompute, and a computing parent reads the new value.
- Top-level pull order does not affect values. The graph is a DAG and every computation is pure, so the first pull that reaches a marked node brings it up to date and every later pull finds it `Clean`. Which nodes run and how often is the same in any order.
- No order is stored: a pull brings a node's inputs up to date before the node uses them, through `readDerived` and `verify`.
- `setDependencies` is the only writer of `dependents`. Removing every old edge and adding every new one is `O(old + new)`; a dependency read twice is one member of each set. A violation of "the dependents sets are the inverse of the dependency lists" is a silently stale result, which one writer guards.
- The context is a parameter built per computation; there is no ambient current computation (MobX's `globalState.trackingDerivation`, Reactively's `CurrentReaction`).

A throw out of a computation propagates out of `compute`, and a throw out of `equal` propagates out of `recompute`, before `setDependencies` and before any state is written, so the node keeps its mark, its previous value, and its previous dependencies and dependents, and the next pull recomputes it again. A verifying ancestor keeps its mark too. `recompute` calls `equal` before `setDependencies` for that reason: called after, a throw from `equal` would leave the node in the `dependents` sets of the new dependencies while its state lists the old ones, and a later `setDependencies` from the old list would never remove it from the sets only the failed run added. Calling `equal` first changes nothing else: `equal` compares two values and reads no node, and both branches store `computed.dependencies`, so a link that moved to a record with equal data still cuts off and the node depends on the new record. `resolverResultsEqual` does not throw; a field-supplied `outputEqual` (open decision 7) can. A resolver's own throw does not propagate: it is a stored value (Equality, `ResolverThrew`). What propagates is a throw of the walk outside the resolver, such as `linkAt` on a non-link (`libs/isograph-runtime/src/store.ts:74-86`). Open decision 3 covers that throw.

## The store

The store is the set of source nodes. A record is a container that holds its source nodes by store key and the derived nodes it owns by resolver store key.

```ts
// from libs/isograph-runtime/src/store.ts
/** Every record container: `EntityName`, then record id. */
export type Store = Map<EntityName, Map<RecordId, RecordContainer>>;

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

export type StoreSourceNode = SourceNode<StoreValue | Missing, ResolverResult<unknown>>;
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

This is the user's `Map<EntityName, Map<RecordId, Map<StoreKey, SourceNode>>>` with two additions. `derived` puts a resolver derived node in the container of the record it was computed on, so that container is its owner: deleting the record disposes it with no `owned` map and no id table. Server keys and resolver store keys are separate maps, so a server field and a client field of one name do not collide. `presence` keeps v1's distinction between a record `normalize` wrote with no values (EmptyStats's `stats {}` writes an empty `PetStats` record, `test/normalize.test.ts:270`) and no record, for the readers that are not computations. `MISSING` is a symbol, so no JSON value equals it, and `=== MISSING` narrows with no cast.

There is no `SourceId`, no `DerivedNodeId`, no `sourceDependents` map, and no `RecomputationGraph` object. The environment's `store` is the whole graph.

### Accessors

Every store consumer reads through these. No function outside `store.ts`, `normalize.ts`, and `storedResults.ts` touches `sources`, `derived`, or `presence`.

```ts
// from libs/isograph-runtime/src/store.ts
/** The container at `link`, or `undefined`. */
function recordContainer(store: Store, link: StoreLink): RecordContainer | undefined {
  return store.get(link.entityName)?.get(link.id);
}

/** The container at `link`, inserted `Absent` when the store has none. */
export function containerAt(store: Store, link: StoreLink): RecordContainer {
  return getOrInsert(getOrInsert(store, link.entityName, () => new Map()), link.id, () => ({
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
  return getOrInsert(containerAt(store, link).sources, key, () => ({ value: MISSING, dependents: new Set() }));
}

/** The value of `key` on `link`, or `MISSING`; inserts nothing and records nothing. Callers: v1's read
 * hooks, `check`, the garbage collector's mark, `storeSnapshot`. */
export function storeValue(store: Store, link: StoreLink, key: StoreKey): StoreValue | Missing {
  return recordContainer(store, link)?.sources.get(key)?.value ?? MISSING;
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

`?? MISSING` is safe because a stored value is never `undefined` (`JsonValue` and `CompositeValue` exclude it). `storeValue` and `recordPresence` insert nothing, so `check`, the mark, a v1 read, and a snapshot never grow the store.

What each consumer becomes:

- `read` and `readButDoNotEvaluate` (v1): the walk's `readValue` hook is `storeValue` (The reader walk).
- The graph read: `readValue` is `sourceNodeAt`, reported to the computation (The reader walk).
- `check`: `storeRecord(store, link) === undefined` becomes `recordPresence(store, link) === 'Absent'` (`libs/isograph-runtime/src/check.ts:30`, `:54`), and `record.get(key)` becomes `storeValue(store, link, key)` with `=== MISSING` for `=== undefined` (`:49-50`). Its results are v1's for every input.
- The garbage collector's mark: the same two replacements (`libs/isograph-runtime/src/garbageCollection.ts:51`, `:70-73`), and one change: a link reached to an `Absent` container keeps that container, and the mark does not recurse into it (Garbage collection in the core). The sweep calls `deleteRecord`.
- `readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:196-218`): calls `read`, unchanged (open decision 6).
- Serialization: `storeSnapshot`.
- Test fixtures: `storeOf` (`libs/isograph-runtime/test/fixtures.ts`) returns a `PlainStore`; a test compares `storeSnapshot(store)` with it. A test that needs a store built from data calls a helper in `test/fixtures.ts`, `storeFrom(plain: PlainStore): Store`, which calls `presentContainerAt` and `writeSource(sourceNodeAt(..), value)` for each value. A test that corrupted a stored value (`environment.store.get('Pet')?.get('1')?.set('bestFriend', 'x')`) calls `writeSource(sourceNodeAt(store, pet('1'), 'bestFriend'), 'x')`; one that deleted a record calls `deleteRecord`; one that deleted a key builds its store with `storeFrom` without that key. No API exists for tests alone.

### Absent records

Record presence is not a source. An absent record reads as a record whose every key is missing: the walk asks for the source node of the first key it tries, `sourceNodeAt` creates it with `MISSING` in an `Absent` container, the computation records it, and the read is `MissingData` naming that key. When `normalize` later writes the record, it finds that container and that source node, and writing the key marks the reader. A reader whose key the write does not set stays `MissingData`, which is correct. Deleting a record marks the dependents of each of its source nodes, `MISSING` ones included, and disposes the derived nodes it owns; ownership is the container's `derived` map.

A resolver selected on an absent record (a parent reads a link to `Pet:9`, and `Pet:9` is absent): the child node is created in `Pet:9`'s `Absent` container, reads its keys, and gets `MissingData`. It is owned by the absent record and is disposed when the garbage collector deletes that container.

`noRecord` and `MissingData` are one result. v1 builds "No record for X" at `readButDoNotEvaluate` and `readStoreValue` (`libs/isograph-runtime/src/read.ts:68-74`, `:184-191`); both checks go, and the walk reports "No value for K on X". Two consequences, for every walk (v1's and the graph's):

- `read.test.ts`'s "a read of an absent root record is missing data on the root" and "an absent root is missing data on the root" assert `reason: 'No value for pet____id___"1" on Query:__ROOT'`.
- A reader AST that reads no key on an absent record is `Ok`. "a field with an empty reader AST is missing data on an absent record" (`test/read.test.ts`) becomes: `Constant` on the absent `Pet:9` is `Ok` with `data: {}` and its resolver is called. v1 already reads an empty linked selection on an absent record as `Ok` (`readLinkedValue` checks no presence, `read.ts:207-229`); now a resolver's root does the same. Upstream checks presence at every record it reads (`read.ts:161-167`); the deviation follows from presence not being a source: a read that reads no key has nothing to depend on, so a `MissingData` it returned could never be marked when the record appears.

`check` and the fetch decision. `check` reads presence through `recordPresence` and is not a computation, so its results do not change. The two places it reads presence (`check.ts:30-34`, `:54-57`) yield the same link a missing first key would, except for an empty selection set. The fetch decision of Later: retention and the fetch decision reads the root derived node instead, so it fetches less than `check` in two cases: a reader that reads no key on an absent record, and a record a partial write left without its `id`. In both, every value the readers read is present.

### Equality

Each derived node has an equality function, used for cutoff: `recompute` keeps the previous value object and marks no dependent when `equal(previous, next)` holds. The default is `===`.

Correctness: an equality function only decides whether the dependents recompute. One that answers unequal for equal values makes them recompute from equal inputs to their old values, which costs work and never yields a wrong value. One that answers equal for unequal values yields a wrong value, so every rule other than `===` must answer equal only when every position a reader can observe is `===`.

The resolver node's equality:

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** Kinds differ: unequal. `MissingData`: equal `reason`, `recordLink` entity and id, and `nestedReason`,
 * recursively. `ResolverThrew`: errors `===`. `Ok`: both `WalkData`, `selectionSetDataAreEqual` over the
 * artifact's reader AST; otherwise `outputEqual`. */
export function resolverResultsEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  outputEqual: (previous: unknown, next: unknown) => boolean,
): (previous: ResolverResult<unknown>, next: ResolverResult<unknown>) => boolean;
```

- A resolver's return value: `outputEqual`, `===` unless the field supplies one (open decision 7). A primitive or a reused object cuts off; a new object marks the dependents.
- Objects the runtime itself creates get a structural rule. `MissingData` is built from strings and links. A fragment's data object is assembled by the walk (`readSelectionSet`, `read.ts:101-121`); when a resolver returns that object (`({ data }) => data`, as every store fixture resolver does), the result is tagged `WalkData` and two such results compare by `selectionSetDataAreEqual` (`libs/isograph-runtime/src/areEqualWithDeepComparison.ts:9-67`): scalars by `===`, lists by length and item, linked and refined selections recursively, nested `Resolver` values by `===`. That is upstream's `mergeObjectsUsingReaderAst` rule (`areEqualWithDeepComparison.ts:55-121`) without the recycling. So `{ initials }` assembled by the walk compares equal when `initials` is the same string, and a link moved to a record with equal data cuts off.
- Source writes: `normalize` compares the value it writes with the stored one by the shape the AST gives it, `leafUnchanged` and `compositeUnchanged` (normalize writes source nodes), as upstream does (`normalizeScalarField`, `cache.ts:232-249`; `normalizeLinkedField` and `dataIdsAreTheSame`, `:255-393`).

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

The `Resolver` case calls `hooks.readResolver(selection.readerArtifact(), root, childVariables(variables, selection.arguments))` where it called `read(store, ..)` (`read.ts:164-174`).

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

## Stored resolver results

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** The stored result of `artifact` on `root` with `variables`, pulled. */
export function readStoredResult<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ResolverResult<TOutput> {
  // The node under `artifact.selectableName`'s resolver store key on `root` was created from
  // `artifact`: one selectable of one entity has one reader artifact (open decision 1).
  return pull(resolverNodeAt(store, artifact, root, variables)) as ResolverResult<TOutput>;
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
    resolverStoreKey(artifact.selectableName, variables),
    () => createDerivedNode(resolverComputation(store, artifact, root, variables), resolverResultsEqual(artifact, sameValue)),
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
function contextHooks(
  store: Store,
  context: ComputeContext<ResolverResult<unknown>>,
): ReadHooks<ReadResolverThrew> {
  return {
    readValue: (record, key) => {
      const source = sourceNodeAt(store, record, key);
      context.readSource(source);
      return source.value;
    },
    readResolver: (artifact, root, variables) => context.readDerived(resolverNodeAt(store, artifact, root, variables)),
  };
}

/** `artifact.resolver` called with `data`; `WalkData` when it returned `data` itself; a throw becomes
 * `ResolverThrew`. */
function callResolver(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ResolverResult<unknown>;

/** `===`, the default output equality. */
function sameValue(previous: unknown, next: unknown): boolean {
  return previous === next;
}
```

The computation closes over `store`, the environment's `Map`, which is written in place and never replaced (network-and-subscriptions.md, The environment). The cast in `readStoredResult` is the one place a stored `ResolverResult<unknown>` is returned at the caller's type. Its type parameters and `root` parameter are `read`'s. `readStoredResult` takes the store, as `read` does; every function that reads or writes the graph takes the store, because the store is the graph.

Every read inside a computation goes through the walk, so none goes unrecorded. User code never receives a `ComputeContext` or the store. A resolver that reaches the store through a closure is open decision 2.

The resolver store key:

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** `selectableName`, then one `____<name>___<stableStringify(value)>` chunk per entry of `variables`,
 * names in code unit order: `argumentChunks`'s format. */
export function resolverStoreKey(selectableName: string, variables: Variables): StoreKey;
```

The variables are the field's own: for a `Resolver` node, `childVariables(variables, selection.arguments)` (`read.ts:232-242`); for a top-level read, the read's variables. `PetName` on `Pet:1` is `PetName`; `PetSummary` read with `{ id: "1" }` is `PetSummary____id___"1"` on `Query.__ROOT`. Two selections of one field with equal argument values on one record are one node, whatever their aliases, read by both parents. The format can collide; open decision 1.

The key needs the client field's selectable name, which v1's reader artifact does not carry:

```ts
// from libs/isograph-runtime/src/artifacts.ts
// ReaderArtifact, before:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
// after:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  /** The client field's selectable name: the prefix of its resolver store key. */
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
```

`reader_artifact_file` (`crates/typescript_artifacts/src/reader_artifact.rs`) writes `selectableName: "<name>",` as the object's first entry; `Pet/PetName/resolver_reader.ts` gains `selectableName: "PetName",`.

## `normalize` writes source nodes

`normalize` keeps its parameters and its `void` return (`libs/isograph-runtime/src/normalize.ts:31-50`). It writes each value into its source node and marks as it writes. A write `isUnchanged` accepts writes nothing and marks nothing.

```ts
// from libs/isograph-runtime/src/normalize.ts
/** A value `normalize` writes, with the shape its selection gives it. */
type WrittenValue =
  | { readonly kind: 'Leaf'; readonly value: LeafValue }
  | { readonly kind: 'Composite'; readonly value: CompositeValue };

/** Writes `written.value` into the source node of `key` on `link`, which marks its readers, unless the
 * stored value is unchanged by `isUnchanged`. */
function writeValue(normalization: Normalization, link: StoreLink, key: StoreKey, written: WrittenValue): void {
  const source = sourceNodeAt(normalization.store, link, key);
  if (source.value === MISSING || !isUnchanged(source.value, written)) {
    writeSource(source, written.value);
  }
}

/** `Leaf`: `leafUnchanged`. `Composite`: `compositeUnchanged`. */
function isUnchanged(stored: StoreValue, written: WrittenValue): boolean;

/** `written` a list: `stored` is a list of the same length whose items are `leafUnchanged` in order, at
 * any depth. Otherwise `stored === written`. */
function leafUnchanged(stored: unknown, written: LeafValue): boolean {
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
function compositeUnchanged(stored: StoreValue, written: CompositeValue): boolean;
```

Deltas from v1 (`normalize.ts`): `writeSelection` builds a `WrittenValue` (`Leaf` when `selection.target == null`) and calls `writeValue` where it called `record.set` (`:96-109`); `writeSelections` and `compositeValue` pass the record's link where they passed its `StoreRecord`; `recordAt` is `presentContainerAt` (`:163-169`). An unchanged value keeps its identity.

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

A throw outside a resolver: `root.2` holds the string `"x"` under a composite key.

```text
write root.2 = "x" -> root.2.dependents = {FP, FC} -> FP Dirty, FC Dirty
read FP: Dirty -> recompute FP: root.1, root.2 -> linkAt throws
  FP stays Dirty with p1 and its edges (root.1, root.2, other.1, other.2, FC); FC stays Dirty; the throw propagates
read FP: Dirty -> recompute FP -> linkAt throws again
write root.2 = other -> FP, FC already Dirty
read FP: Dirty -> recompute FP: root.1, root.2, other.1, other.2, FC Dirty -> recompute FC -> v1 === v1 -> FC keeps v1 -> FP Clean
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

Upstream's collection deletes records only and runs only when the base layer is the only layer (`garbageCollection.ts:65-86`).

## Core changes

Each is one commit, in order; each is independently shippable.

1. Prefactor: `normalize` skips an unchanged value (`writeValue`, `isUnchanged`, `leafUnchanged`, `compositeUnchanged`) on v1's plain store. Observable: normalizing the same response twice keeps every stored list and link object (`toBe`). In the pet script, the `PetDetailRoute` response leaves `Pet:0.firstName`'s stored value untouched.
2. Prefactor: the walk takes `ReadHooks`; an absent record reads as all keys missing; `noRecord`'s two checks go (Absent records). `read` and `readButDoNotEvaluate` use `evaluatingHooks`. Observable: an absent root's `MissingData` reads "No value for pet____id___"1" on Query:__ROOT"; `Constant` on an absent record is `Ok`. The pet script's output is unchanged.
3. Prefactor: the store becomes record containers (`RecordContainer`, `presence`, `sources` of `{ value }`, the accessors, `storeSnapshot`, `deleteRecord` without derived nodes). `check`, `garbageCollect`, `read`, and `normalize` read and write through the accessors; tests compare `storeSnapshot`. `SourceNode` has `value` only in this change. Observable: none in behavior; `environment.store` is no longer v1's nested map, and `storeSnapshot(environment.store)` is.
4. The graph: `graph.ts` (`SourceNode.dependents`, `DerivedNode`, `stateOf`, mark, pull with short circuit, `setDependencies`, `disposeDerivedNode`), `RecordContainer.derived`, `writeSource` in `writeValue`, `readStoredResult`, `resolverNodeAt`, `contextHooks`, `ResolverResult` with `WalkData`, `resolverResultsEqual`, `resolverStoreKey`, `ReaderArtifact.selectableName` and the compiler's printing of it, `deleteRecord` disposing owned nodes. The package index adds `readStoredResult` and the types `ResolverResult`, `ReadResolverOk`, `OkOrigin`, `ReadResolverThrew`; `StoreRecord` becomes `RecordContainer`. The pet script reads with `readStoredResult`. Observable: End-user experience above.
5. The subscription re-read reads nested resolvers through the graph. `readSubscription` reads with `readData(storedResultHooks(store), ..)`, whose `readValue` is `storeValue` (nothing recorded) and whose `readResolver` is `readStoredResult`; `SubscriptionResult` gains `ReadResolverThrew`, `ReadErr` is only a broken store invariant, and `subscriptionResultsAreEqual` compares two `ResolverThrew` by `===` of their errors. Without it, the landed re-read (`libs/isograph-runtime/src/subscribe.ts:120-131`) calls every nested resolver on every write through v1's `read`. `makeNetworkRequest` calls `callSubscriptions` in a `finally` after `normalize`, so a response that throws part-way re-reads the subscriptions over the writes it made (open decision 4), and `garbageCollectEnvironment` calls `callSubscriptions` after the sweep, so a subscription that read a deleted record is re-read. Neither call depends on subscriptions being derived nodes. `callSubscriptions` never throws (`libs/isograph-runtime/src/subscribe.ts:92-95`; `readSubscription` catches a throwing pull as `ReadErr`, `:126-130`), so `normalize`'s throw still reaches the `.catch`. Observable: in the pet script, the subscription's re-read after the tagline write calls `PetSummaryCard` on `Pet:0` only, and a re-read after an unrelated write calls no resolver and does not call the callback; a response that changes `Pet:0.tagline` and then throws delivers the new tagline to the subscription; a collection that deletes a record a subscription read delivers that subscription's `MissingData`.

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
// from libs/isograph-runtime/src/graph.ts
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

- Subscriptions. A subscription is a derived node whose computation is `readData(contextHooks(store, context), artifact, root, variables)` without the subscription's own resolver, and whose equality is `subscriptionResultsAreEqual` (`libs/isograph-runtime/src/subscribe.ts:135-155`): the data is an object the walk assembled, so it compares by `selectionSetDataAreEqual`. The `Subscription` holds its node, which lives in no container; unsubscribing disposes it. `callSubscriptions` pulls each subscription's node after every write and calls the callback when the pulled value is not `===` `delivered`; a `Clean` node is one step, and a marked one verifies, so the reader AST is walked only when something it read changed. `callSubscriptions` keeps core change 5's call sites. This replaces core change 5's `readSubscription`.
- React. `useSyncExternalStore(subscribe, getSnapshot)`: `listen(node, onChange)` adds an observer and a root hold; after a write, `notify` calls each observer whose node is not `Clean`; React then calls `getSnapshot`, which is `pull(node)`. A node whose inputs came back equal is verified without recomputing and returns the same object, so React does not render again. `getSnapshot` pulls during render; an observer whose node stays marked is called on every write until something pulls it.
- Loaders. loaders.md's effect-set nodes are derived nodes that the runtime pulls after every write (`reconcileEffectRoots`), and its loader items are source nodes (loaders.md, An item that changes over time; After a write).
- Callback errors. Every callback, listener, and observer runs through `reportCallbackError` (`libs/isograph-runtime/src/subscribe.ts:159-167`): a throw is rethrown in its own microtask and the loop continues (network-and-subscriptions.md, Decisions, 3).
- Scheduling. Marking is separate from pulling, so a scheduler can decide when each observer's pull runs (Follow-ups 9).

### Values with lifecycles

A computation is pure and runs on every pull, including a pull a discarded render or an uncommitted world (Later: a DAG of graphs) makes. A resource a node's value needs is the node's effect, run apart from its computation: set up when the node is mounted (`listen` at React's commit), torn down when it is unmounted, disposed, or given a new value. After a write, `reconcileEffects` pulls every mounted node with an effect, because an effect whose inputs changed must be replaced even when no reader reads the node. A new effect is set up before the old one is torn down. A replaced or disposed effect is torn down after every observer of that write was notified. A `===` value leaves the effect as is. Effects run only in the committed world. Open decision 13.

## Later: a DAG of graphs

Not part of the core: optimistic layers and forks. The base graph is the core's store. Layer graphs sit above it, each with one parent, and form a tree. A world is a path from the base to a layer.

- Each layer holds source overrides (its value for a source node of the base) and derived shadow states (its `StateSlot` for a derived node).
- Reading in world `W` takes, for both node kinds, the deepest entry on `W`'s path: a source's value from the deepest layer that overrides it, else the base; a derived node's slot from the deepest layer that holds one, else the base. `stateOf(node, world)` is that lookup; no other code changes (The graph).
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
- Runtime-defined computations (Follow-ups 10) and the suspension decision of react.md are derived nodes whose computations call `ComputeContext` directly; their values are not `ResolverResult`s, so the graph's value type widens to a union.

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

## Docs this doc amends

- `refactors/pending/runtime/store.md`: `Store` becomes `Map<EntityName, Map<RecordId, RecordContainer>>`; its functions read and write through the accessors; `normalize` skips unchanged values and marks as it writes; `garbageCollect`'s sweep calls `deleteRecord`; its tests compare `storeSnapshot(store)` with `storeOf(..)`, which returns a `PlainStore`.
- `refactors/pending/runtime/reader-ast.md`: `ReaderArtifact.selectableName`; the walk takes `ReadHooks`; an absent record reads as all keys missing, which changes its two absent-root `reason` assertions and makes `Constant` on an absent record `Ok`; `readStoredResult` is the stored read beside `read`.
- `refactors/pending/runtime/network-and-subscriptions.md`: its "`Store` stays store.md's nested map" no longer holds; the request entry's state is a source node on the entry, not `requestSource(key): SourceId`; `readSubscription` reads nested resolvers through the graph and `SubscriptionResult` gains `ReadResolverThrew` (core change 5); `makeNetworkRequest` re-reads subscriptions in a `finally` after `normalize`, and `garbageCollectEnvironment` re-reads them after the sweep (core change 5).
- `refactors/pending/runtime/react.md`: `unwrapOrSuspend` throws `result.error` on `ResolverThrew`; `getSnapshot` pulls (Later: keeping values up to date automatically), against its rule that a render reads and computes nothing.
- `refactors/pending/runtime/components-in-userland.md`: `useClosureData`'s switch gains `case 'ResolverThrew': throw result.error;`; a closure field's wrapper is a derived node in its record container's `derived`, owned and freed with the record.
- `refactors/pending/runtime/loaders.md`: edited with this rewrite where its references to this doc's sections and `SourceId`s broke.

## Open decisions

### 1. The resolver store key format

A resolver node's key in its container is `resolverStoreKey(artifact.selectableName, variables)`, and the container is the concrete record's: `normalize` stores a record under the entity its response object names (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`). Two collisions follow.

- Abstract and concrete. A client field can be declared on an abstract entity and read on a concrete record: `field Node.NodeLink` (`libs/isograph-runtime/fixtures/store/src/NodeLink.ts`) is read on `Pet:1` under `node(id: $id)` (`fixtures/store/src/Links.ts`). The compiler accepts `field Node.Label` beside `field Pet.Label`: its duplicate check is per (entity, selectable name) (`crates/isograph_compiler/src/schema.rs:278-294`, `:322-344`), and a selection resolves on its parent's static entity (`crates/isograph_compiler/src/merged_selection.rs:1105`, `:1321`). Both reads on `Pet:1` use the key `Label` in `Pet:1`'s `derived`, so the second returns the first field's result.
- Argument chunks. A GraphQL name may contain `_`, so a field named `F____n___2` read with no variables and `F` read with `{ n: 2 }` both have the key `F____n___2` (`argumentChunks`, `libs/isograph-runtime/src/storeKey.ts:22-34`).

Upstream has both: its server key is the field name and `____<name>___<value>` chunks (`getParentRecordKey`, `cache.ts:454-479`; `:594-595`), and its per-(record, field) key `stableIdForFragmentReference` names the concrete typename and the field name (`FragmentReference.ts:75-79`). The body is written with `selectableName` alone until this is decided.

- (a) The key names the declaring entity: `<declaring entity>.<selectable name>` and the chunks; `ReaderArtifact` gains `readonly parentEntityName: EntityName`, and `resolverStoreKey` takes `Pick<ReaderArtifact<..>, 'parentEntityName' | 'selectableName'>`. A GraphQL name cannot hold `.`. Held, not decided. Cost: one more artifact field and a longer key. It fixes the first collision only.
- (b) The compiler rejects a client field on a concrete entity when an abstract entity it implements declares one of the same name, and the reverse. Cost: a validation over the protocol's implements relation (`NetworkProtocol::concrete_types`, `crates/isograph_compiler/src/artifact_types.rs:901-923`) and a rule upstream does not state.
- (c) Accept both collisions. Cost: two resolvers share one stored result, with no error.
- (d) A key that cannot collide, alone or with (a): a separator outside the name alphabet, `` `${parentEntityName}.${selectableName}(${name}:${json},..)` ``, or `JSON.stringify([parentEntityName, selectableName, entries])` with `entries` the `[name, stableCopy(value)]` pairs in code unit order. Both are injective. Cost: the resolver key no longer shares the server key's format. The server `storeKey` keeps upstream's format; its collision is store.md's.

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

Not held. The body behaves as (a) by omission.

### 3. A throw inside a computation outside its resolver

`callResolver` catches only the resolver's throw. The walk throws on a broken store invariant: `linkAt` on a non-link under a composite key (`libs/isograph-runtime/src/store.ts:74-86`, reached from `readLinkedValue`, `read.ts:226`), and `variableValue` on a missing variable (`storeKey.ts:56-63`, reached from `read.ts:131`, `:133`). Step A's subscription read catches both as `ReadErr` (`subscribe.ts:120-131`).

- (a) `compute` catches any throw of the computation and stores it as the node's value, `{ kind: 'Err', error }`; `ResolverResult` gains `ReadErr`, and the node ends `Clean` with the dependencies it recorded before the throw, so a write that repairs one of them recomputes it. Held, not decided. Cost: an engine bug is stored as a value instead of surfacing; the node is retried only when an input it recorded changes.
- (b) `recompute` catches the throw, restores the node `Clean` with its previous value and dependencies, and rethrows. Cost: the node holds a value no current input explains.
- (c) Propagate, and leave the node marked with its previous value and dependencies; the next pull retries it. The body and the trace in Traces are written for (c), as the user specified the trace. Cost: every pull of the node and of every node above it throws until a write repairs the store.

A second question arises once something pulls inside a write (`reconcileEffects`, Values with lifecycles; loaders.md's `reconcileEffectRoots`) and does not arise under (a): when `normalize` threw part-way and that pull then throws, which error the caller of the write receives.

- (i) The pull's error. Cost: the response's error is lost.
- (ii) The response's error; the pull's error is reported through `reportCallbackError`. Cost: the broken store invariant surfaces in a microtask with no stack frame of the write.
- (iii) An `AggregateError` of both. Cost: a caller testing for the response's error no longer matches.

### 4. A response that throws part-way

`normalize` writes in place and throws at the first position that does not match the AST (store.md, Errors).

- (a) `normalize` marks each write as it makes it (`writeValue`), and its throw propagates as v1's error; the writes before it and their marks stand. `makeNetworkRequest` re-reads subscriptions in a `finally` after `normalize` (core change 5). Held: upstream keeps a partial write too (`normalizeData` writes in place, `cache.ts:77-85`; a throw reaches `makeNetworkRequest`'s `.catch`, `makeNetworkRequest.ts:299-319`).
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

`resolverNodeAt` passes `sameValue` as `outputEqual`. A user who returns a new object per call (`{ length: data.name.length }`) and wants cutoff supplies a comparator, such as a deep equality. A comparator must answer equal only for values no reader can tell apart (Equality).

- (a) A second argument to the function a field literal returns: `iso(`field Pet.Shape { name }`)(resolver, { equal: deepEqual })`. The Babel plugin replaces a field literal with `x => x` (`libs/isograph-babel-plugin/compileTag.js:53`); the replacement becomes `(resolver, options) => Object.assign(resolver, options)`, so the export is still the resolver function, now carrying `equal`. The generated `IdentityWithParam` (`fixtures/store/src/__isograph/iso.ts:44-48`) gains the optional parameter and returns the resolver's type with `readonly equal?: (previous: TOutput, next: TOutput) => boolean`, and `resolverNodeAt` passes `artifact.resolver.equal ?? sameValue`. Held, not decided. Reason: it is the simplest: no directive, no new iso syntax, no compiler change beyond the generated `iso` overload, and the comparator sits next to the resolver it belongs to. Cost: a property written onto the user's function object, and an optional property on the resolver's type.
- (b) A declaration directive naming a built-in comparator (`field Pet.Shape @equality(kind: Deep) { name }`), from which the compiler writes `outputEqual: deepEqual` imported from the runtime. Cost: i2 accepts no declaration directive yet (`DeclarationDirectiveName` is empty, `crates/isograph_parser/src/directives.rs:38-41`); only built-in comparators.
- (c) A map from selectable to comparator passed to `createIsographEnvironment`. Cost: the comparator is far from its field and keyed by a string the compiler does not check.
- (d) No API: a resolver that wants cutoff returns a stable value itself. Cost: no cutoff for object outputs without user memoization.

Upstream compares resolver values by `===` only (`areEqualWithDeepComparison.ts:84-95`).

### 8. A cycle of derived nodes

The core cannot form one: a cycle of client field selections is an `InlineCycle` query error (query-generation.md, Cycles), and every runtime dependency goes to a node of a field the reader AST selects, on whichever record a link leads to. An eager client pointer can: `field Pet.Card { bestFriend { Card } }` with `Pet:1` and `Pet:2` each other's best friend makes `Card@Pet:1` read `Card@Pet:2`, which reads `Card@Pet:1`. `pull` then recomputes a node inside its own computation, and `verify` recurses the same way; neither checks. Marking terminates on a cycle, because it stops at a marked node. Incremental fails on a cycle with "adding edge made graph cyclic" (`ensure_height_requirement`, `src/adjust_heights_heap.ml` of `janestreet/incremental`).

Not decided; it arises with client pointers.

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
