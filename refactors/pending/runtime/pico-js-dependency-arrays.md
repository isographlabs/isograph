# pico-js: dependency arrays and joins

This is a design sketch for a pico-js engine in which every derived node receives its dependencies as an array at creation, and the only dependency that changes after creation is the target of a join. A computation receives the values of its dependencies. It reads nothing inline. Eager nodes are scheduled in height order, as Jane Street's Incremental schedules its necessary nodes. Nothing here is implemented in `libs/pico-js`.

Sources this doc cites:

- `libs/pico-js/...` is the v0 engine at commit `95b11b6f3` in the `../i2-worktrees/pico-js-v0` worktree.
- "Research" is `../pico-js-artifacts/research/dynamic-deps.md`. Its parts (a) to (f) are cited by letter. Incremental paths (`src/...`) are at Incremental commit `371fc83a75dc691e135d36c6d3cf25aadcbf3acc`, as research part (a) gives them.
- `proofs/A6.md` and `proofs/A9.md` are in `../pico-js-artifacts/proofs/`. They define the fallback this doc reuses.
- "The inline-read model (N1)" is the model of research part (e).

Upstream isograph has no dependency graph to compare against. `callSubscriptions` re-reads each fragment subscription whose encountered records overlap the records a write touched (`../isograph/libs/isograph-react/src/core/subscribe.ts:63-110`).

## The model

- A source node holds a value and a list of dependents. It is a distinct type and a distinct object shape (research part (f)).
- A static derived node is `createDerivedNode(dependencies, compute)`. The array is fixed at creation. pico-js links one edge per element into that element's list of dependents when the node is created, and the set of edges never changes. `compute(values, previous)` receives the values of the dependencies in array order. It receives no `callingNode`.
- Every dependency exists before its reader, so creation order is a topological order. A node's height at creation is one more than the greatest height among its dependencies, with a source at 0. A computation runs after its dependencies have values, so a read never points up, nothing suspends, and nothing walks.
- A join follows data. Its outer node's value is a node, the target. The join's value is the target's value. This is Incremental's `join : 'a t t -> 'a t`, which "returns an incremental that behaves like the incremental that [t] currently holds" (`src/incremental_intf.ml:1177-1179`). The edge from a join to its target is the graph's only dynamic edge.
- `bind` is `join(map(...))`: a static selector node whose value is a node, and a join of it.
- Eager nodes are the necessary nodes and every cached join. A node is necessary if it is observed or a necessary node depends on it (`src/incremental_intf.ml:96-103`). Every other derived node is lazy and keeps v0's marking and pull.
- The flush takes eager nodes from a bucketed intrusive heap in height order. A node that recomputes to a value `===` to its previous value queues nothing. Callbacks run after the heap is empty.
- Heights only rise. Only a join that switches to a target at or above its own height raises heights.
- Owners store nodes. pico-js frees cached values and edges, and owners remove node objects.

## Public API

```ts
// from libs/pico-js/src/index.ts
declare const sourceBrand: unique symbol;
declare const derivedBrand: unique symbol;
declare const uncachedJoinBrand: unique symbol;
declare const retainBrand: unique symbol;

export type SourceNode<TValue> = { readonly [sourceBrand]: TValue };

/** A static derived node or a join. */
export type DerivedNode<TValue> = { readonly [derivedBrand]: TValue };

/** A join that writeSource keeps current. It stores no value of its own. */
export type UncachedJoinNode<TValue> = DerivedNode<TValue> & {
  readonly [uncachedJoinBrand]: TValue;
};

export type GraphNode<TValue> = SourceNode<TValue> | DerivedNode<TValue>;

export type Retain<TValue> = { readonly [retainBrand]: TValue };

/** The dependencies an uncached join's selector may receive. */
export type UncachedSelectorInput<TValue> =
  | SourceNode<TValue>
  | UncachedJoinNode<TValue>;

export type ValuesOf<
  TDependencies extends ReadonlyArray<GraphNode<unknown>>,
> = {
  readonly [K in keyof TDependencies]: TDependencies[K] extends GraphNode<
    infer TValue
  >
    ? TValue
    : never;
};

/** Selects the cached form where the uncached form is the default. */
export type CachedJoinOptions = { readonly cache: 'cached' };

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue>;

export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void;

export function createDerivedNode<
  const TDependencies extends ReadonlyArray<GraphNode<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  compute: (
    values: ValuesOf<TDependencies>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue>;

// Joins of a node-valued node. The name gives the outer node's kind, then the
// kind of the node it holds.
export function joinSourceHoldingSource<TValue>(
  outer: SourceNode<SourceNode<TValue>>,
): UncachedJoinNode<TValue>;
export function joinSourceHoldingSource<TValue>(
  outer: SourceNode<SourceNode<TValue>>,
  options: CachedJoinOptions,
): DerivedNode<TValue>;

export function joinSourceHoldingDerived<TValue>(
  outer: SourceNode<DerivedNode<TValue>>,
): DerivedNode<TValue>;

export function joinDerivedHoldingSource<TValue>(
  outer: UncachedJoinNode<SourceNode<TValue>>,
): UncachedJoinNode<TValue>;
export function joinDerivedHoldingSource<TValue>(
  outer: DerivedNode<SourceNode<TValue>>,
  options?: CachedJoinOptions,
): DerivedNode<TValue>;

export function joinDerivedHoldingDerived<TValue>(
  outer: DerivedNode<DerivedNode<TValue>>,
): DerivedNode<TValue>;

// List joins. The value is the array of the targets' values. Every list join
// is cached.
export function joinListSourceHoldingSources<TValue>(
  outer: SourceNode<ReadonlyArray<SourceNode<TValue>>>,
): DerivedNode<ReadonlyArray<TValue>>;

export function joinListSourceHoldingDerived<TValue>(
  outer: SourceNode<ReadonlyArray<DerivedNode<TValue>>>,
): DerivedNode<ReadonlyArray<TValue>>;

export function joinListDerivedHoldingSources<TValue>(
  outer: DerivedNode<ReadonlyArray<SourceNode<TValue>>>,
): DerivedNode<ReadonlyArray<TValue>>;

export function joinListDerivedHoldingDerived<TValue>(
  outer: DerivedNode<ReadonlyArray<DerivedNode<TValue>>>,
): DerivedNode<ReadonlyArray<TValue>>;

// bind: join(createDerivedNode(dependencies, select)).
export function bindSource<
  const TDependencies extends ReadonlyArray<UncachedSelectorInput<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  select: (values: ValuesOf<TDependencies>) => SourceNode<TValue>,
): UncachedJoinNode<TValue>;
export function bindSource<
  const TDependencies extends ReadonlyArray<GraphNode<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  select: (values: ValuesOf<TDependencies>) => SourceNode<TValue>,
  options?: CachedJoinOptions,
): DerivedNode<TValue>;

export function bindDerived<
  const TDependencies extends ReadonlyArray<GraphNode<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  select: (values: ValuesOf<TDependencies>) => DerivedNode<TValue>,
): DerivedNode<TValue>;

export function bindListSources<
  const TDependencies extends ReadonlyArray<GraphNode<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  select: (
    values: ValuesOf<TDependencies>,
  ) => ReadonlyArray<SourceNode<TValue>>,
): DerivedNode<ReadonlyArray<TValue>>;

export function bindListDerived<
  const TDependencies extends ReadonlyArray<GraphNode<unknown>>,
  TValue,
>(
  dependencies: TDependencies,
  select: (
    values: ValuesOf<TDependencies>,
  ) => ReadonlyArray<DerivedNode<TValue>>,
): DerivedNode<ReadonlyArray<TValue>>;

export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue;

export function retainDerivedNode<TValue>(
  node: DerivedNode<TValue>,
  onChange?: (value: TValue) => void,
): Retain<TValue>;

export function releaseRetain<TValue>(retain: Retain<TValue>): void;

export function startWriteBatch(): void;

export function endWriteBatch(): void;

export const scheduler: { schedule: () => void };

export function disposeNext(): boolean;

export function disposeAll(): void;
```

`readSource`, `readDerived`, and `CallingNode` do not exist. `readDerivedNode`, `retainDerivedNode`, `releaseRetain`, the batch functions, `scheduler`, `disposeNext`, and `disposeAll` keep v0's meaning, and they accept every join because `UncachedJoinNode<TValue>` is a `DerivedNode<TValue>`.

The README's first graph, before and after:

```ts
// from libs/pico-js/README.md
const sum = createDerivedNode(
  (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
);
const doubled = createDerivedNode(
  (callingNode) => readDerived(callingNode, sum) * 2,
);
const chosen = createDerivedNode((callingNode) =>
  readSource(callingNode, useA)
    ? readSource(callingNode, a)
    : readSource(callingNode, b),
);
```

```ts
// from libs/pico-js/README.md
const sum = createDerivedNode([a, b], ([a, b]) => a + b);
const doubled = createDerivedNode([sum], ([sum]) => sum * 2);
const chosen = bindSource([useA], ([useA]) => (useA ? a : b));
```

`chosen` is an uncached join. A write to `b` while `useA` is true reaches nothing, as it does in v0, because the join's target edge is on `a`.

## Joins

### Constructors

A join constructor creates a node. It is not a marker on one reader's read. Any number of nodes may list the join as a dependency, and it can be retained and observed like any derived node.

Each constructor's outer and target kinds are fixed by its type, so each join's refresh code reads one object shape for the outer node and one for the target. A link's type says statically whether it holds a source or a derived node, so the four single constructors and the four list constructors cover every case. A join whose target is a source never compares heights, because a source has no height.

A join's outer node has a value that is a node. The outer node is a source holding a node, a derived node computing one, or an uncached join whose target source holds one.

### Uncached joins

An uncached join is the default when its selector reads only sources or other uncached joins and its target is a source. Those are `joinSourceHoldingSource`, `joinDerivedHoldingSource` over an `UncachedJoinNode`, and `bindSource` over `UncachedSelectorInput` dependencies.

- writeSource computes it. A write that reaches an uncached join through its selector's edges runs the selector on the new values and moves the target edge if the selected source differs. A write that reaches it through its target edge changes its value. writeSource knows the written source's old and new values, so it decides whether the join's value changed without storing that value on the join.
- It stores no value, has no status, and never enters the flush heap. A read of it returns its target source's value.
- Its height is one more than the greatest height among its selector's dependencies, with a source at 0. Its target is a source, so a switch never raises a height.
- `bindSource` in the uncached form allocates one node. The selector's edges and the target edge belong to the join, and there is no separate map node, because writeSource evaluates the selector and nothing caches its result.
- `CachedJoinOptions` selects the cached form instead.

### Cached joins

Every other join is cached. It stores its value and is eager. Marking stops at it and puts it on the flush heap, and the flush refreshes it in height order, whether or not it is necessary. No lazy selector is offered.

A cached join refreshes in this order:

1. It reads its outer node's value, which is the new target.
2. If the target is a derived node at or above the join, the join raises its own height and its dependents' heights above the target. A join taken from the heap then puts itself back on the heap at its new height and returns. It has read nothing from the target and stored nothing.
3. It moves its target edge to the new target, if the target changed.
4. It reads the target's value, and stores it if it is not `===` to its previous value.

Incremental builds a join from two nodes. `Join_lhs_change` installs the new target, and `Join_main` reads it one height later (`src/state.ml:1574-1583`, `755-766`). A pico-js join is one node. The requeue in step 2 does the work of the second node, so a switch that raises costs one more heap pass and every join saves a node.

### List joins

A list join's outer value is an array of nodes, and its value is the array of their values. It reuses target edges by position, as v0 reuses edges (`libs/pico-js/src/pull.ts:78-127`). A refresh that finds every element `===` to the previous array's element at the same index, with the same length, keeps the previous array, so the `===` cutoff holds for lists. Every list join is cached, because an uncached list join would build a new array on each read, and no two of those arrays are `===`.

### bind

`bindDerived(dependencies, select)` is `joinDerivedHoldingDerived(createDerivedNode(dependencies, (values) => select(values)))`. `bindSource` with derived dependencies, `bindListSources`, and `bindListDerived` are the same over their join constructors. A cached bind is two nodes: the selector node and the join. The selector node is internal, and only the join's outer edge points at it.

There are no scopes. Incremental's scopes invalidate the nodes a bind's right-hand side created when the left-hand side changes (`src/incremental_intf.ml:315-325`). pico-js has a contract instead:

- A selector or a computation may create nodes. A node it creates may capture the values the selector or computation received.
- Such a node belongs to that selection. It may be stored only under a key made of the values it captured, and it must not be returned for another key.
- When the join switches away from it, the old target loses its dependent, becomes unneeded, and is freed by cleanup.

Nodes stored by owners under their own key, as in the examples below, meet this contract. A node that a selector creates fresh on each run and stores nowhere also meets it.

## Examples

The examples share this record type and these owners:

```ts
type Person = {
  readonly name: SourceNode<string>;
  readonly bestFriend: SourceNode<Person | null>;
  readonly parent: SourceNode<Person | null>;
};

function createPerson(name: string, parent: Person | null): Person {
  return {
    name: createSourceNode(name),
    bestFriend: createSourceNode<Person | null>(null),
    parent: createSourceNode(parent),
  };
}

function render(value: string): void {
  console.log(value);
}
```

### A best friend's name

```ts
const noName = createSourceNode('');

const bestFriendNameNodes = new Map<Person, UncachedJoinNode<string>>();

function bestFriendName(person: Person): UncachedJoinNode<string> {
  let node = bestFriendNameNodes.get(person);
  if (node === undefined) {
    node = bindSource([person.bestFriend], ([friend]) =>
      friend === null ? noName : friend.name,
    );
    bestFriendNameNodes.set(person, node);
  }
  return node;
}

const alice = createPerson('Alice', null);
const bob = createPerson('Bob', null);
const carol = createPerson('Carol', null);
writeSource(alice.bestFriend, bob);

const greeting = createDerivedNode(
  [bestFriendName(alice)],
  ([name]) => `Hello, ${name}`,
);
retainDerivedNode(greeting, render);
```

`bestFriendName(alice)` is an uncached join at height 1, with a selector edge on `alice.bestFriend` and a target edge on `bob.name`. `greeting` is a static node at height 2. The retain with an `onChange` makes `greeting` necessary.

- A change of the target's value: `writeSource(bob.name, 'Robert')`. The only dependent of `bob.name` is the join, through its target edge. writeSource puts the join on the write heap, re-runs the selector, which selects `bob.name` again, and compares `'Bob'` with `'Robert'`. The value changed, so `greeting` goes on the flush heap. The flush recomputes `greeting` to `'Hello, Robert'`, and the callback runs. The write ran one selector and one computation, and touched nothing else.
- A link change: `writeSource(alice.bestFriend, carol)`. The join's selector edge reaches it. The selector returns `carol.name`, so the target edge moves from `bob.name`'s dependents to `carol.name`'s. `'Robert'` is not `'Carol'`, so `greeting` recomputes as above. No height changes. If Carol were also named `'Robert'`, the compare would stop there and `greeting` would not be queued.
- An unrelated field change: `writeSource(bob.parent, carol)` or `writeSource(alice.name, 'Alicia')`. Neither source has a dependent in this graph, so the write only stores the value. After the link change, `writeSource(bob.name, ...)` reaches nothing either.

### Parent's name or own name

```ts
const parentOrOwnNameNodes = new Map<Person, UncachedJoinNode<string>>();

function parentOrOwnName(person: Person): UncachedJoinNode<string> {
  let node = parentOrOwnNameNodes.get(person);
  if (node === undefined) {
    node = bindSource([person.parent], ([parent]) =>
      parent === null ? person.name : parent.name,
    );
    parentOrOwnNameNodes.set(person, node);
  }
  return node;
}

const dana = createPerson('Dana', null);
const erin = createPerson('Erin', dana);

const label = createDerivedNode([parentOrOwnName(erin)], ([name]) =>
  name.toUpperCase(),
);
retainDerivedNode(label, render);
```

The selector captures `person`, which is the key its owner stores the node under.

- A change of the target's value: `writeSource(dana.name, 'Danielle')`. The join's target edge reaches it. The selector selects `dana.name` again, `'Dana'` is not `'Danielle'`, and `label` recomputes to `'DANIELLE'`. With `erin.parent` null, the target is `erin.name`, and `writeSource(erin.name, ...)` takes the same path.
- A link change: `writeSource(erin.parent, null)`. The selector edge reaches the join. The selector returns `erin.name`, the target edge moves from `dana.name` to `erin.name`, and `'Dana'` is not `'Erin'`, so `label` recomputes.
- An unrelated field change: `writeSource(erin.bestFriend, dana)` or `writeSource(dana.parent, erin)`. Neither source has a dependent here. `parentOrOwnName(dana)` was never created, so nothing reads `dana.parent`.

### The root ancestor's name

```ts
const ownNameNodes = new Map<Person, DerivedNode<string>>();

function ownName(person: Person): DerivedNode<string> {
  let node = ownNameNodes.get(person);
  if (node === undefined) {
    node = createDerivedNode([person.name], ([name]) => name);
    ownNameNodes.set(person, node);
  }
  return node;
}

const rootNameNodes = new Map<Person, DerivedNode<string>>();

function rootName(person: Person): DerivedNode<string> {
  let node = rootNameNodes.get(person);
  if (node === undefined) {
    node = bindDerived([person.parent], ([parent]) =>
      parent === null ? ownName(person) : rootName(parent),
    );
    rootNameNodes.set(person, node);
  }
  return node;
}

const rhea = createPerson('Rhea', null);
const ari = createPerson('Ari', rhea);
const bo = createPerson('Bo', ari);
const cy = createPerson('Cy', bo);

const banner = createDerivedNode([rootName(cy)], ([name]) => `House ${name}`);
retainDerivedNode(banner, render);
```

`rootName(person)` is a cached join per record. Its target is a derived node, so the uncached form does not apply. `bindDerived` creates a selector node at height 1 over `person.parent` and a join at height 2 over the selector node. The selector creates `rootName(parent)` and `ownName(person)` through their owners, so each is stored under its own key.

The retain pulls `banner`, which pulls `rootName(cy)`. No flush is running, so every raise only adjusts heights:

- `rootName(cy)` selects `rootName(bo)`, which was just created at height 2. 2 is not below 2, so `rootName(cy)` rises to 3 and `banner` to 4. `rootName(cy)` links its target edge, then pulls `rootName(bo)`.
- `rootName(bo)` selects `rootName(ari)` at 2, rises to 3, and the raise reaches `rootName(cy)` through the linked edge, so `rootName(cy)` rises to 4 and `banner` to 5. The same happens one level down for `rootName(ari)` over `rootName(rhea)`.
- `rootName(rhea)` selects `ownName(rhea)` at height 1, below its own 2, and reads `'Rhea'`.

The final heights are `ownName(rhea)` 1, each selector node 1, `rootName(rhea)` 2, `rootName(ari)` 3, `rootName(bo)` 4, `rootName(cy)` 5, and `banner` 6. Every one of these nodes is necessary.

- A change of the target's value: `writeSource(rhea.name, 'Rhiannon')`. `ownName(rhea)` goes on the flush heap. The flush refreshes it at height 1, then `rootName(rhea)`, `rootName(ari)`, `rootName(bo)`, and `rootName(cy)` one height each, and `banner` at 6. Each join's selector node is unchanged, so no target edge moves. Six refreshes run, and nothing else is visited.
- A link change to a record with the same root: `writeSource(bo.parent, rhea)`. `rootName(bo)`'s selector node refreshes at height 1 and returns `rootName(rhea)`, a different node, so `rootName(bo)` goes on the heap. The flush takes it at 4. `rootName(rhea)` is at 2, so there is no raise. The target edge moves from `rootName(ari)` to `rootName(rhea)`, the value is `'Rhea'` both before and after, and nothing past `rootName(bo)` is queued. `rootName(ari)` and its selector node lose their only necessary dependent and become lazy. `rootName(ari)` has no dependent and no retain, so it goes on the cleanup queue, and cleanup later frees its value and edges and then its selector node's. `rootNameNodes` still holds the `rootName(ari)` object.
- A link change to a new root: `writeSource(bo.parent, zed)`, where `zed` is a new root named `'Zed'`. `rootName(bo)`'s selector node calls `rootName(zed)`, which creates a join at height 2 and its selector node at 1. The flush takes `rootName(bo)` at 4. The target is at 2, so the edge moves and the join pulls `rootName(zed)` as a lazy node. Nothing below height 4 is on the heap, so the lazy pull reads current values. `rootName(zed)` creates and reads `ownName(zed)`, then `rootName(zed)`, its selector node, and `ownName(zed)` become necessary. `'Zed'` is not `'Rhea'`, so `rootName(cy)` and `banner` refresh.
- A link change to a deeper chain: `writeSource(ari.parent, yara)`, where `yara` is the bottom of seven records under another root and `rootName(yara)` is computed at height 9. The flush takes `rootName(ari)` at 3. Its target is at 9, so `rootName(ari)` rises to 10, `rootName(bo)` to 11, `rootName(cy)` to 12, and `banner` to 13, and any of them on the heap moves to its new bucket. `rootName(ari)` goes back on the heap at 10 and returns. The flush takes it again at 10, moves the edge, reads the target, and the change refreshes `rootName(bo)`, `rootName(cy)`, and `banner`. No fallback runs. A later switch back to `rhea` lowers no height.
- A link change that pulls a join on demand: `writeSource(ari.parent, wren)`, where `wren.parent` is `yara` and `rootName(wren)` was never created. The flush takes `rootName(ari)` at 3. Its selector node created `rootName(wren)` at height 2, so there is no raise, and `rootName(ari)` pulls `rootName(wren)`. `rootName(wren)` is a waiting reader's dependency and cannot requeue. It selects `rootName(yara)` at 9 and rises to 10, and through its linked edge `rootName(ari)` rises to 11 and the nodes above follow. Its pull of `rootName(yara)` is a pull of a node at or above the flush position 3, so the fallback runs first. `rootName(ari)` then stores its new value, and the flush finishes with marking that goes past eager nodes.
- An unrelated field change: `writeSource(bo.name, 'Bea')` or `writeSource(cy.bestFriend, ari)`. `ownName(bo)` does not exist, because `bo` is not a root, and nothing reads `bo.name` or `cy.bestFriend`.

## Scheduling

### Heights

- A static node's height is set at creation, one more than the greatest height among its dependencies, with a source at 0 and an uncached join at its own height.
- A cached join's height at creation is one more than its outer node's height. Its target is unknown until its first refresh.
- A raise is Incremental's `adjust_heights`: it lifts the node and every dependent whose height is not above the new height of the node below it, in increasing order of pre-adjusted height, and moves each one that is on the flush heap (`src/adjust_heights_heap.ml:136-203`). It throws if it reaches the new target, which is its cycle check (`src/adjust_heights_heap.mli:39-71`).
- Only a join starts a raise, when it switches to a derived target whose height is at or above its own.
- A freed node is out of its dependencies' lists, so a raise does not reach it. When it links its edges again, its height becomes the greater of its old height and one more than its dependencies' heights, and the raise continues to its dependents. This catches up with raises that joins made while it was unlinked.
- Heights never go down. This is Incremental's pseudo-height, which "never goes down" so a bind that flips between targets does not renumber the graph each time (talk ¶211-213).
- The bucket array grows with the greatest height. Incremental bounds height at 128 by default (`src/incremental_intf.ml:134-139`). pico-js does not, as research part (e) states under "Bounded heights".

### The flush heap

The flush heap is Incremental's recompute heap. It is an array of buckets indexed by height, each a doubly linked list through the nodes' `heapPrevious` and `heapNext` fields, with a lower-bound cursor (`src/recompute_heap.ml:20-197`). Add, remove, and move are O(1) and allocate nothing. Taking the minimum scans forward from the cursor. Nothing is added below the cursor during a flush, because a node that becomes necessary during a flush is pulled first and is `Clean` (see "Necessity and lazy nodes").

The write heap is a second heap of the same type. It holds uncached joins during one writeSource and is empty between calls.

### writeSource

```ts
// from libs/pico-js/src/write.ts
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  const mutableSource = source as MutableSourceNode<TValue>;
  const oldValue = mutableSource.value;
  mutableSource.value = value;
  markDependents(mutableSource.firstDependent);
  drainWriteHeap(mutableSource, oldValue);
  if (batchDepth === 0) {
    flush();
  }
  scheduleIfOutermost();
}
```

`markDependents` visits each dependent:

- An uncached join goes on the write heap.
- An eager node that is needed goes on the flush heap as `Dirty`, unless it is already queued. An eager node is a necessary node or a cached join. A cached join that is unneeded is set `Dirty` and goes on the marked-unneeded list instead, and the update frees it, as A9's `stopAtEagerNode` does (`proofs/A9.md`, "The build", item 9).
- A lazy node is marked as v0 marks it, `Clean` to `Dirty` and then `MaybeDirty` onward through lazy nodes (`libs/pico-js/src/marking.ts:61-121`). The walk stops at an eager node and queues it. A necessary node is never above a lazy one, so the only eager nodes the walk meets are cached joins.

```ts
// from libs/pico-js/src/write.ts
function drainWriteHeap(
  written: MutableSourceNode<unknown>,
  writtenOldValue: unknown,
): void {
  let join = heapRemoveMin(writeHeap);
  while (join !== undefined) {
    const targetEdge = join.firstDependency!;
    const oldTarget = targetEdge.dependency as MutableSourceNode<unknown>;
    const oldValue = oldTarget === written ? writtenOldValue : oldTarget.value;
    const newTarget = selectUncached(join);
    if (newTarget !== oldTarget) {
      moveEdge(targetEdge, newTarget);
    }
    if (newTarget.value !== oldValue) {
      if ((join.flags & OBSERVED) !== 0) {
        recordUncachedChange(join, oldValue);
      }
      markDependents(join.firstDependent);
    }
    join = heapRemoveMin(writeHeap);
  }
}
```

The write heap takes uncached joins in height order, so a selector that reads another uncached join runs after that join has its final target for this write. One write writes one source, so `oldTarget === written` decides each join's old value in O(1).

### The flush

```ts
// from libs/pico-js/src/flush.ts
function flush(): void {
  flushState = FLUSH_ORDERED;
  try {
    let node = heapRemoveMin(flushHeap);
    while (node !== undefined) {
      flushHeight = node.height;
      refreshTaken(node);
      node = heapRemoveMin(flushHeap);
    }
    if (markedUnneededCount > 0) {
      disposeMarked();
    }
    callOnChanges();
  } finally {
    flushState = FLUSH_IDLE;
    clearMarkedUnneeded();
    clearChangedNodes();
  }
}
```

`refreshTaken` dispatches on `kind`:

- A static node that is `Dirty` recomputes. One that is `MaybeDirty` is verified first. One that is `Clean` was refreshed on demand after a fallback, and is skipped.
- A cached join refreshes as "Cached joins" describes, with `TAKEN_FROM_HEAP`.

A recompute pulls each derived dependency, then calls `compute(values, previous)`. In `FLUSH_ORDERED`, every dependency is below the node and final, so each pull is a status check. If the result is `===` to the previous value, nothing else happens, and nothing past the node is visited. Otherwise `propagateChange` visits each dependent: a needed eager one goes on the flush heap as `Dirty`, and a lazy one is marked `Dirty` with the in-line walk beyond it. An observed node goes on the list of changed nodes. Callbacks run in `callOnChanges` after the heap is empty, at most once per node per flush, as Incremental's on-update handlers do (`src/state.ml:1321-1363`).

An observed uncached join that changed during the batch is on the list of changed nodes with its value before the batch. `callOnChanges` calls its `onChange` only if its current value is not `===` to that value.

### Refreshing a cached join

```ts
// from libs/pico-js/src/join.ts
function refreshJoinDerivedHoldingDerived(
  join: MutableDerivedNode<unknown>,
  taken: Taken,
): void {
  const outer = outerOf(join) as MutableDerivedNode<
    MutableDerivedNode<unknown>
  >;
  const target = pull(outer);
  if (target.height >= join.height) {
    adjustHeights(join, target.height + 1, target);
    if (taken === TAKEN_FROM_HEAP) {
      heapAdd(flushHeap, join);
      return;
    }
  }
  const previousTarget = moveTargetEdge(join, target);
  const value = pullTarget(target);
  transferNecessity(join, previousTarget, target);
  storeJoinValue(join, value, taken);
}

function pullTarget<TValue>(target: MutableDerivedNode<TValue>): TValue {
  if (flushState === FLUSH_ORDERED && target.height >= flushHeight) {
    fallBack();
  }
  return pull(target);
}
```

The other cached kinds differ only in the shapes they read. A source outer is read as `outer.value`. A source target skips the height compare and `transferNecessity`. A list join runs the same steps over its target edges by position and raises once, above its tallest target.

A join taken from the heap is at `flushHeight`. After the raise check its target is below it, so `pullTarget` never falls back for it, and a lazy target's pull reads only nodes below the flush position.

A join pulled by a waiting reader cannot requeue, because its reader is waiting for its value. It raises, links the edge, and pulls the target. The edge is linked before the pull, so a raise of the target during its own pull reaches the join. If the target is at or above the flush position, `pullTarget` runs the fallback first.

The same compare runs where a freed node links its edges again during a flush and pulls its dependencies, because the catch-up can put a dependency at or above the flush position.

### The fallback

The fallback is A6's fallback marking (`proofs/A6.md`, "Stopped nodes"), with the queued eager nodes in the place of A6's stopped source-only nodes, as A9's `fallBack` and `markPastEagerNodes` apply it (`proofs/A9.md`, "The fallback"):

- `flushState` becomes `FLUSH_FALLEN_BACK`.
- One beyond walk runs from every queued node and from the node the flush took last. It sets every `Clean` node above them `MaybeDirty` with `MARKED_BEYOND` and walks past it, eager or lazy, without queueing it (`libs/pico-js/src/marking.ts:188-213`).
- Until the flush ends, a pull of a queued node removes it from the heap and refreshes it at once, `verify` pulls a queued node instead of descending into it, and `propagateChange` marks in line past eager nodes, as v0's `markFromChange` marks past lazy ones.
- The flush keeps taking nodes in height order. A taken node that a pull already refreshed is `Clean` and is skipped.

After a fallback the flush no longer stops at an unchanged eager node, so goal 2 of research part (c) does not hold for the rest of that flush. Solid 2.0's `markHeap` has the same cost for the rest of its pass (research part (a), "Signals libraries").

### Necessity and lazy nodes

Necessity follows research part (e), "Primitives" and "Invariants" 1:

- `necessaryDependentCount` counts a node's necessary dependents. A node is necessary if the count is positive or the node has the `OBSERVED` bit, as Incremental's necessity is `num_parents > 0 || observers <> Null` (`src/types.ml:484-491`).
- A retain with an `onChange` pulls the node, sets `OBSERVED`, and runs `becomeNecessary`. Releasing the last one runs `becomeUnnecessary`. A join that moves its target edge, while it is necessary, runs `becomeNecessary` on the new target after pulling it and `becomeUnnecessary` on the old target after that, as Incremental's `change_child` adds the new child before it removes the old one (`src/state.ml:636-663`).
- `becomeNecessary` walks down through each node whose count goes from 0 to 1. Every node it reaches was just pulled and is `Clean`, so none goes on the heap. Research part (e) pulls an unnecessary node and then makes it necessary for the same reason ("Read inside a computation of a necessary node", third bullet).
- `becomeUnnecessary` walks down through each node whose count goes to 0 and that is not observed, as `became_unnecessary` does (`src/state.ml:361-387`). A node it reaches that is on the flush heap leaves the heap, stays `Dirty`, and marks its dependents in line, as v0's `unobserveIfLast` does (`libs/pico-js/src/retain.ts:136-146`), so no lazy node keeps a `Clean` dependent. A cached join is eager when unnecessary, so it stays on the heap.

A lazy node is a static node that is not necessary. It keeps v0's statuses, its marking, its pull, and its `verify` (`libs/pico-js/src/pull.ts:135-237`), without the beyond walk and `BEYOND_DONE`. A write marks lazy nodes in line and stops at eager nodes. A changed eager node marks its lazy dependents in line. A lazy node is computed only when something pulls it: a top-level read, a retain, a lazy dependent's pull, or a cached join that reads it as its outer node or its target.

Lazy nodes keep their edges, stay cached, and can be read at any time. Incremental keeps no edges to unnecessary nodes and computes nothing unnecessary (research part (e), "Where pico-js's goals differ from Incremental").

### Cleanup and ownership

- A node is needed while it has a retain or a dependent, as in v0 (`libs/pico-js/docs/freeing-nodes.md`). A dependent counts from the moment its edge is linked, which for a static node is its creation.
- A new node has no retain and no dependent, so `createDerivedNode` and the join constructors put it on the cleanup queue. Without that, a node created and never read would never be freed, and its edges would keep it and its dependencies reachable after its owner dropped it. `scheduler.schedule` runs at the end of the next outermost operation, as v0 states for the cleanup queue.
- Freeing a node unlinks each of its edges from its dependency's list of dependents, drops its value, and sets it `Uncomputed`. The node keeps its edge objects. Its next pull links the same edges again, catches up its height, and computes. A dependency that a free leaves unneeded goes on the cleanup queue.
- An update frees the unneeded nodes it took out of `Clean`, after the flush and before the callbacks, as v0's `disposeMarked` does (`libs/pico-js/src/cleanup.ts:163-202`).
- A join's old target loses the join as a dependent, and goes on the cleanup queue if that leaves it unneeded.
- A necessary node is needed, so cleanup never frees it.
- pico-js never removes a node from an owner. Owners are module variables or maps keyed by a parameter, and an owner removes a node object when it chooses. A freed node that an owner keeps costs its object and its unlinked edge objects, and computes again when read.

## Internal types

```ts
// from libs/pico-js/src/nodes.ts
const SOURCE = 0;
const STATIC = 1;
const UNCACHED_JOIN_SOURCE_HOLDING_SOURCE = 2;
const UNCACHED_JOIN_DERIVED_HOLDING_SOURCE = 3;
const UNCACHED_BIND = 4;
const JOIN_SOURCE_HOLDING_SOURCE = 5;
const JOIN_SOURCE_HOLDING_DERIVED = 6;
const JOIN_DERIVED_HOLDING_SOURCE = 7;
const JOIN_DERIVED_HOLDING_DERIVED = 8;
const LIST_JOIN_SOURCE_HOLDING_SOURCES = 9;
const LIST_JOIN_SOURCE_HOLDING_DERIVED = 10;
const LIST_JOIN_DERIVED_HOLDING_SOURCES = 11;
const LIST_JOIN_DERIVED_HOLDING_DERIVED = 12;

type DerivedKind =
  | typeof STATIC
  | typeof UNCACHED_JOIN_SOURCE_HOLDING_SOURCE
  | typeof UNCACHED_JOIN_DERIVED_HOLDING_SOURCE
  | typeof UNCACHED_BIND
  | typeof JOIN_SOURCE_HOLDING_SOURCE
  | typeof JOIN_SOURCE_HOLDING_DERIVED
  | typeof JOIN_DERIVED_HOLDING_SOURCE
  | typeof JOIN_DERIVED_HOLDING_DERIVED
  | typeof LIST_JOIN_SOURCE_HOLDING_SOURCES
  | typeof LIST_JOIN_SOURCE_HOLDING_DERIVED
  | typeof LIST_JOIN_DERIVED_HOLDING_SOURCES
  | typeof LIST_JOIN_DERIVED_HOLDING_DERIVED;

const UNCOMPUTED = 0;
const DIRTY = 1;
const MAYBE_DIRTY = 2;
const CLEAN = 3;

type Status =
  | typeof UNCOMPUTED
  | typeof DIRTY
  | typeof MAYBE_DIRTY
  | typeof CLEAN;

/** Bits of `flags`. */
const QUEUED = 1; // on the flush heap or the write heap
const OBSERVED = 2; // has a retain with an onChange
const ON_CHANGE_QUEUED = 4; // on the list of changed nodes of the running flush
const MARKED_BEYOND = 8; // MaybeDirty from a fallback's beyond walk
const LINKED = 16; // every edge is in its dependency's list of dependents
const TARGET_SELECTED = 32; // a single join's first edge is its target edge

type MutableSourceNode<TValue> = SourceNode<TValue> & {
  readonly kind: typeof SOURCE;
  value: TValue;
  firstDependent: Edge | undefined;
  lastDependent: Edge | undefined;
};

/**
 * Every derived kind is this one object literal, so marking and the heaps
 * read one shape. A static node's edges are its dependencies in array order.
 * A single join's edges are its target edge, once it has selected a target,
 * then its outer edge or its selector's edges. A list join's edges are its
 * outer edge, then its target edges in order. An uncached join leaves
 * `value`, `status`, and `necessaryDependentCount` unused.
 */
type MutableDerivedNode<TValue> = DerivedNode<TValue> & {
  readonly kind: DerivedKind;
  value: TValue;
  status: Status;
  flags: number;
  height: number;
  necessaryDependentCount: number;
  heapPrevious: MutableDerivedNode<unknown> | undefined;
  heapNext: MutableDerivedNode<unknown> | undefined;
  firstRetain: MutableRetain<TValue> | undefined;
  firstDependency: Edge | undefined;
  firstDependent: Edge | undefined;
  lastDependent: Edge | undefined;
  readonly compute: Compute<TValue> | Select | undefined;
};

type AnyNode = MutableSourceNode<unknown> | MutableDerivedNode<unknown>;

/** v0's edge. Only a target edge changes its `dependency`. */
type Edge = {
  dependency: AnyNode;
  readonly dependent: MutableDerivedNode<unknown>;
  nextDependency: Edge | undefined;
  previousDependent: Edge | undefined;
  nextDependent: Edge | undefined;
};

type Compute<TValue> = {
  method(values: ReadonlyArray<unknown>, previous: TValue | undefined): TValue;
}['method'];

type Select = (values: ReadonlyArray<unknown>) => AnyNode;

/** v0's retain, unchanged. */
type MutableRetain<TValue> = Retain<TValue> & {
  readonly node: MutableDerivedNode<TValue>;
  readonly onChange: OnChange<TValue> | undefined;
  previous: MutableRetain<TValue> | undefined;
  next: MutableRetain<TValue> | undefined;
};

type OnChange<TValue> = { method(value: TValue): void }['method'];

type Heap = {
  readonly buckets: Array<MutableDerivedNode<unknown> | undefined>;
  lowestHeight: number;
  count: number;
};

const FLUSH_IDLE = 0;
const FLUSH_ORDERED = 1;
const FLUSH_FALLEN_BACK = 2;

type FlushState =
  | typeof FLUSH_IDLE
  | typeof FLUSH_ORDERED
  | typeof FLUSH_FALLEN_BACK;

const TAKEN_FROM_HEAP = 0;
const PULLED = 1;

type Taken = typeof TAKEN_FROM_HEAP | typeof PULLED;
```

v0's derived node for comparison. The new literal drops `evaluation` (now the `OBSERVED` bit), the four dependency-list fields (now `firstDependency`), and `BEYOND_DONE`, and adds `kind`, `height`, `necessaryDependentCount`, `heapPrevious`, and `heapNext`: 13 fields against 12.

```ts
// from libs/pico-js/src/nodes.ts
export type MutableDerivedNode<TValue> = DerivedNode<TValue> &
  CallingNode & {
    value: TValue;
    firstRetain: MutableRetain<TValue> | undefined;
    status: Status;
    evaluation: EvaluationState;
    // The bits `ON_CHANGE_QUEUED`, `MARKED_BEYOND`, and `BEYOND_DONE`.
    flags: number;
    firstSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
    lastSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
    firstDerivedDependency:
      | MutableEdge<MutableDerivedNode<unknown>>
      | undefined;
    lastDerivedDependency: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    firstDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    lastDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    computation(callingNode: CallingNode, previous: TValue | undefined): TValue;
  };
```

Module state:

```ts
// from libs/pico-js/src/flush.ts
let flushState: FlushState = FLUSH_IDLE;
/** The height of the node the running flush took last. */
let flushHeight = 0;
const flushHeap: Heap = { buckets: [], lowestHeight: 0, count: 0 };
const writeHeap: Heap = { buckets: [], lowestHeight: 0, count: 0 };
```

## Internal functions

```ts
// from libs/pico-js/src/edges.ts
function linkEdge(edge: Edge): void;
function unlinkEdge(edge: Edge): void;
/** Unlinks `edge` from its dependency's dependents and links it into `to`'s. */
function moveEdge(edge: Edge, to: AnyNode): void;

// from libs/pico-js/src/heap.ts
function heapAdd(heap: Heap, node: MutableDerivedNode<unknown>): void;
function heapRemove(heap: Heap, node: MutableDerivedNode<unknown>): void;
function heapRemoveMin(heap: Heap): MutableDerivedNode<unknown> | undefined;
function heapMove(
  heap: Heap,
  node: MutableDerivedNode<unknown>,
  previousHeight: number,
): void;

// from libs/pico-js/src/heights.ts
/** Raises `node` to at least `height` and its dependents above it. Throws if it reaches `child`. */
function adjustHeights(
  node: MutableDerivedNode<unknown>,
  height: number,
  child: MutableDerivedNode<unknown>,
): void;

// from libs/pico-js/src/write.ts
function markDependents(first: Edge | undefined): void;
function drainWriteHeap(
  written: MutableSourceNode<unknown>,
  writtenOldValue: unknown,
): void;
/** Runs an uncached join's selector on the current values of its inputs. */
function selectUncached(
  join: MutableDerivedNode<unknown>,
): MutableSourceNode<unknown>;
function recordUncachedChange(
  join: MutableDerivedNode<unknown>,
  valueBeforeBatch: unknown,
): void;

// from libs/pico-js/src/flush.ts
function flush(): void;
function refreshTaken(node: MutableDerivedNode<unknown>): void;
function propagateChange(node: MutableDerivedNode<unknown>): void;
function fallBack(): void;
function callOnChanges(): void;

// from libs/pico-js/src/pull.ts
function pull<TValue>(node: MutableDerivedNode<TValue>): TValue;
function verify(root: MutableDerivedNode<unknown>): typeof DIRTY | typeof CLEAN;
function recompute<TValue>(node: MutableDerivedNode<TValue>, taken: Taken): TValue;
/** Links a freed node's edges again and catches up its height. */
function relink(node: MutableDerivedNode<unknown>): void;
function gatherValues(node: MutableDerivedNode<unknown>): ReadonlyArray<unknown>;

// from libs/pico-js/src/join.ts
function outerOf(join: MutableDerivedNode<unknown>): AnyNode;
/** Moves the target edge if `target` differs and returns the previous target. */
function moveTargetEdge(
  join: MutableDerivedNode<unknown>,
  target: AnyNode,
): AnyNode | undefined;
function pullTarget<TValue>(target: MutableDerivedNode<TValue>): TValue;
function transferNecessity(
  join: MutableDerivedNode<unknown>,
  previousTarget: AnyNode | undefined,
  target: MutableDerivedNode<unknown>,
): void;
function storeJoinValue(
  join: MutableDerivedNode<unknown>,
  value: unknown,
  taken: Taken,
): void;
function refreshJoinSourceHoldingSource(
  join: MutableDerivedNode<unknown>,
  taken: Taken,
): void;
function refreshJoinSourceHoldingDerived(
  join: MutableDerivedNode<unknown>,
  taken: Taken,
): void;
function refreshJoinDerivedHoldingSource(
  join: MutableDerivedNode<unknown>,
  taken: Taken,
): void;
function refreshJoinDerivedHoldingDerived(
  join: MutableDerivedNode<unknown>,
  taken: Taken,
): void;
function refreshListJoin(join: MutableDerivedNode<unknown>, taken: Taken): void;

// from libs/pico-js/src/necessity.ts
function becomeNecessary(node: MutableDerivedNode<unknown>): void;
function becomeUnnecessary(node: MutableDerivedNode<unknown>): void;

// from libs/pico-js/src/cleanup.ts
function addToCleanupQueue(node: MutableDerivedNode<unknown>): void;
function freeNode(node: MutableDerivedNode<unknown>): void;
function disposeMarked(): void;
```

`storeJoinValue` with `TAKEN_FROM_HEAP`, and with `PULLED` for a queued join refreshed on demand after a fallback, calls `propagateChange` on a change. With `PULLED` for a lazy pull, it marks `MaybeDirty` dependents `Dirty`, as v0's `recompute` does (`libs/pico-js/src/pull.ts:437-450`). `recompute` takes the same `Taken` for the same choice.

## Contracts

pico-js checks none of these at run time, except the cycle check that `adjustHeights` performs as part of a raise.

- Computations and selectors are synchronous and pure. A computation depends only on its `values`, its `previous`, and values its owner key fixes. A selector depends only on its `values` and its owner key. Looking a node up in its owner, and storing a node it creates there, is allowed.
- Computations and selectors may create nodes. They may not write, retain, release, batch, or throw. As in v0, they call nothing else in pico-js.
- A node a selector or computation creates belongs to that selection. It is stored only under a key made of the values it captured, and never reused under another key (see "bind").
- pico-js compares values with `===` everywhere. That covers the cutoff of a static node, a cached join, and a list join's elements, the uncached join's compare in writeSource, and the `onChange` rule. `===`-equal values count as identical.
- The graph has no cycles. This includes a cycle through joins that data creates when it points back to itself, such as two records that are each other's parent under `rootName`. `adjustHeights` throws when a switch would close such a cycle. pico-js does not recover from that throw, as it does not recover from any throw.
- A write batch body writes sources, releases retains, and opens nested batches. It does not read derived nodes and does not retain them.
- v0's remaining contracts are unchanged. Each retain is released once, each `startWriteBatch` is matched with one `endWriteBatch`, no batch spans an `await`, a custom scheduler drains the queue, values are not mutated after they pass through pico-js, an `onChange` calls only `readDerivedNode`, and, for performance, a source is written only when its value changed and a computation returns `previous` for an equal result.

## Proof outline

The claim is research part (b)'s goal 1: no computation, selector, top-level read, or callback receives a value that differs from the value computed from the current sources. Goals 2 and 3 follow below it.

Dynamic behavior is confined to joins:

- S, static edges. A static node's edges, a cached join's outer edge, and an uncached join's selector edges are created with the node and never point at another node. Only `freeNode` unlinks them, and only `relink` links them again, to the same dependencies.
- D, dynamic edges. Only `moveTargetEdge`, `refreshListJoin`, and `drainWriteHeap` move an edge, and each moves only a join's target edge.
- R, raises. Only a cached join's refresh or pull starts `adjustHeights`, when its new target is at or above it. `relink` raises a freed node to catch up with raises that reached its dependencies while it was unlinked, and those raises were started by joins. An uncached join's target is a source, so its switches raise nothing.

The invariants:

- H, heights. For every linked edge `X -> Y` with Y derived, `Y.height < X.height`. A static edge satisfies it at creation and at `relink`. A raise keeps it for every edge into a raised node. A target edge is linked only after the raise check. A pulled join links its target edge before the target's pull, so a raise during that pull reaches it.
- N, necessity. Every derived dependency of a necessary node is necessary, through every kind of edge. `becomeNecessary` and `becomeUnnecessary` keep it, and a join's switch adds the new target before it removes the old one. So no lazy node has a necessary dependent, and the in-line marking of lazy nodes reaches no necessary node.
- Q, the flush heap. Between batches and flushes the heap is empty. In a batch body and in a flush, every queued node is eager, `Dirty` or `MaybeDirty`, and in `FLUSH_ORDERED` its height is at least `flushHeight`. No write runs in a flush, because computations, selectors, and callbacks do not write. Nothing is added below the cursor, because the only node that becomes necessary in a flush was just pulled and is `Clean`.
- C, A9's C9 for this engine (`proofs/A9.md`, "Invariant C9"). For each linked edge `X -> Y` with X `Clean`, Y is `Clean`, or Y is queued and the flush has not fallen back, or Y is the node being refreshed. Marking stops only at eager nodes, and it queues each eager node it stops at, so C admits the edges into it. A changed eager node marks or queues its dependents. The fallback's beyond walk removes the queued clause.
- CUR, current reads in `FLUSH_ORDERED`. When the flush takes a node at `flushHeight`, every pull its refresh starts is of a node below `flushHeight`, unless `pullTarget` or `relink` saw a node at or above it and fell back first. By Q no queued node is below `flushHeight`, so by C every `Clean` node below `flushHeight` has only `Clean` nodes below it, and a node that is not `Clean` recomputes from values pulled first. A9's CUR induction on height applies to that region (`proofs/A9.md`, "Lemma CUR").
- REQ, the requeue. A taken join that raises has read no target value and stored nothing. It returns to the heap above its target, so when the flush takes it again, every eager node below the target that was queued has been refreshed.
- FB, after a fallback. No `Clean` node is above a queued node or the taken node. A pull of a queued node refreshes it at once, and verify pulls it instead of descending. From here v0's per-path argument applies to every pull (`libs/pico-js/docs/correctness.md`, "Proof of I for each path"), and the order of the remaining takes does not matter for correctness.
- U, uncached joins. When writeSource returns, every linked uncached join's target is its selector's result on the current values, because the write heap takes uncached joins in height order and each selector's inputs are sources or lower uncached joins. A static node or cached join that reads an uncached join whose value changed was queued or marked by `markDependents` before writeSource returned.
- L, lazy nodes. Between a batch and the next batch, a lazy node that is not `Clean` has no `Clean` lazy dependent, as in v0 (`libs/pico-js/docs/correctness.md`, "L"). `becomeUnnecessary` keeps it by marking in line from a queued node that leaves the heap.

From these:

- Goal 1. A static computation receives values of nodes below it that are final (CUR) or were refreshed on demand after a fallback (FB). A cached join reads its target only below itself (REQ, H) or after a fallback. A selector of an uncached join reads current sources and lower uncached joins (U). A top-level read and a retain run with an empty heap (Q), so v0's lazy argument applies. Callbacks run after the heap is empty, and an `onChange` read is a top-level read.
- Goal 2. In `FLUSH_ORDERED`, a node whose value is `===` to its previous value queues and marks nothing, and marking from a write stops at the first eager node on each path. Nothing past an unchanged eager node is visited until a fallback in the same flush.
- Goal 3. A join's target changes whenever its outer value names another node, its height adapts upward by R, and necessity follows the moved edge.
- Callbacks. Each observed node is refreshed with a change at most once per flush, because after its refresh every node below it is final (H, CUR), and no write runs in the flush. An observed uncached join's callback compares against its value before the batch.

## Comparison

### v0

v0 reads inline through `readSource` and `readDerived` with a `callingNode`, reuses edges by position on every read, and keeps no heights (`libs/pico-js/src/pull.ts:31-127`). A write marks every node above the written source. Marking walks past an observed node with a beyond walk, and `BEYOND_DONE` skips repeated walks over a node whose dependents are all marked (`libs/pico-js/src/marking.ts:131-215`).

```ts
// from libs/pico-js/src/pull.ts
export function readDerived<TValue>(
  callingNode: CallingNode,
  node: DerivedNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  // The calling node becomes `Clean` when its computation returns, and may be
  // a new dependent of `node`.
  const flags = mutableNode.flags;
  if ((flags & BEYOND_DONE) !== 0) {
    mutableNode.flags = flags & ~BEYOND_DONE;
  }
  const last = mutableCallingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstDerivedDependency
      : last.nextDependency;
  // ... reuse `next` by position, or link a new edge
}
```

- v0 costs, per derived read, a pull, a flag check, and an edge-reuse check. Per write, it marks every node above the written source, including the readers of an observed node that will keep its value. Per read of a marked node, it verifies.
- v0 cannot express goal 2. An eager node that keeps its value still has its dependents marked, and they are verified on their next read.
- v0 expresses any dynamic read inline, with no API change at the read.
- This engine costs, against v0: one more field per derived node, a height and heap links to maintain, a values array per computation, a join node (two for a cached bind) at every place a computation follows data, edges and a cleanup-queue entry for every created node, and a necessity walk on every retain or release with an `onChange` that changes necessity.
- This engine saves, against v0: the edge-reuse check on every read, the beyond walk on every write that reaches an observed node, and the verification of everything past an unchanged eager node.

### The inline-read model (N1)

N1 keeps v0's inline reads and adds Incremental's heap for necessary nodes, with a refresh walk for a read of a node at or above the flush position (research part (e), "Operations").

- N1 costs a height, two heap links, and a necessary-dependent count per derived node, as this engine does. On top of that it costs a height compare on every derived read, a raise whenever a necessary reader reads a node at least as tall as itself, through any edge, a refresh walk on each upward read while a heap bucket in range is occupied, and the edge-reuse check of v0 on every read (research part (e), "Costs").
- N1 expresses every dynamic read with no API change. Its refresh walk is a suspending read, the one piece of its scheduler without a counterpart in Incremental (research part (e), "Where pico-js's goals differ from Incremental").
- This engine cannot express a computation whose set of reads depends on data without a join. A computation that reads a node only on one branch either lists the node as a dependency, and recomputes when it changes on the other branch, or moves the branch into a join, as `chosen` does above. Following data to a depth that data decides needs one join per step, and a data-sized fan-out needs a list join.
- This engine needs no suspension and no refresh walk. A static node's dependencies are below it by construction. A join chooses its target before it reads it, and requeues on a raise. The only upward read is a join pulled by a waiting reader, and it falls back. Research part (c) describes this tradeoff: "Static nodes plus a join that flattens a node-valued node is Incremental's map plus join. It needs no suspension ... It costs two nodes per join and an API change at every dynamic read."
- Jane Street states that bind is often expensive, because each change of its left-hand side reruns its right-hand side and allocates nodes (`doc/part2-dynamic.mdx:153-188`), and that it pays when the right-hand side is small or the left-hand side changes rarely (`doc/part4-pitfalls.mdx:113-120`). A pico-js join reruns no right-hand side. A selector returns an existing node from its owner, or creates one when the owner has none.

## Benchmarks

Each benchmark runs against v0 at `95b11b6f3`, with the counters this engine adds. They count flush-heap takes, write-heap takes, raises and the nodes each raise visits, requeues, fallbacks, and marks of lazy nodes.

- The app scenarios in `../pico-js-artifacts/benchS16h/app/app.ts`: S1, S2, S3a, S3b, S4a, S4b, S5a, S5b, S6a, S6b, S7, S8, S9a, S9b, S10, S13, S14, S15a, S15b, and S12 at depths 50, 200, and 1000, at the reference size. The bench's client fields read inline, so the port writes each field as a static node over its dependencies, each followed link to a scalar as `bindSource`, each followed link to another field as `bindDerived`, and each plural link as a list join. S12 pulls a recursive field for the first time at each depth. A chain that a pull discovers from the top costs one raise per level, and each raise walks the levels above it, so the first pull at depth d visits on the order of d²/2 nodes, as the `rootName` trace shows for three levels. S12 measures that cost.
- The S16 family, each at 1, 10, 100, and 1000 subscribers: S16 (one source holding an object of 50 fields, a projection of one field, N observed readers of the projection, and 5% of events changing the projected field), S16c (every event changes it), S16r (each reader also reads its own scalar, and every other event re-renders one reader), S16v (the projected field never changes), and S16d (S16 with five pass-through layers under the projection). The projection is necessary, so it is an eager stop point, and an event that leaves its value unchanged should visit no reader.
- A link change at depths 1, 3, and 10. The graph is the `rootName` chain under two roots with different names, with one observed reader of the bottom record's `rootName`. Each event writes the parent link of the record directly under the root, alternating between the two roots, so the change passes through d joins to the reader. Three variants: both roots at the same height (no raise), one root under a deeper chain (one raise on the first switch, none after), and a switch to a record whose `rootName` was never created over a deeper chain (the fallback path).
- A list join of 10, 100, and 1000 targets with one observed reader of the list join. Four kinds of event run as separate scenarios. In the first, one target's value changes. In the second, the outer array replaces one element. In the third, the outer array appends one element. In the fourth, a write goes to a source that nothing in the graph reads.

## Open questions

1. How an owner learns that pico-js freed a node, so that it can delete the map entry. The options are a callback per node, a callback per owner, or no notification, with owners deciding by their own rules.
2. Whether a static node's `values` array is a new array per computation or one array per node that pico-js refills, which needs a contract that a computation does not keep it.
3. How the engine tells node kinds apart at creation and when it gathers values. The options are the `kind` field this doc holds, or a property-presence test on sources, which would let sources drop `kind`.
4. Whether sources keep v0's record that every dependent is already `Dirty` (`libs/pico-js/src/marking.ts:41-51`). Without `readSource`, a computation would clear it while gathering values, one store per source per computation, and a source with an uncached join among its dependents must reach that join on every write. This doc drops the record.
5. Whether an uncached `bindSource` allocates one node, as this doc holds, or a selector node and a join, as `join(map(...))` reads literally.
6. Whether an uncached join keeps a copy of its target's value in `value`, so that a static node reads `value` from every kind of dependency, or reads through to its target, as this doc holds.
7. Whether a join that selects a freed target links that target again and catches up its height before the raise check, so that a taken join requeues instead of falling back when the catch-up puts the target at or above it.
8. Whether a list join whose outer value is unchanged updates only the entries of targets whose values changed, instead of rebuilding the array and comparing every element.
9. Whether a derived node keeps one dependency list in array order, as this doc holds, or v0's two lists, which let `verify` skip source edges without a kind check (research part (f)).
10. Whether `CachedJoinOptions` exists at all. This doc offers it on the constructors whose default is uncached, because the default implies a choice. No case in this doc needs it.
11. What `adjustHeights` throws on a cycle. The options are an `Error` naming the two nodes, or a `RangeError` as a stack overflow in v0 would give.
