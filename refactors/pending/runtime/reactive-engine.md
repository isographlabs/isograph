# Reactive engine: source nodes and derived nodes as a standalone package

Status: under discussion. Not approved for implementation. This doc is the complete specification of the engine that `refactors/pending/runtime/pico-runtime.md` (The graph, Mark, Pull, The engine and the Isograph runtime, The engine package, What resists the split) places in its own package. It restates those sections in engine terms, adds what they leave out (the full API under one guard option, disposal of a pulled node, cycles, throws, the README, the design description, and the test inventory against the Rust pico crate), and lists the open decisions. Where this doc and pico-runtime.md differ, What this doc changes in pico-runtime.md names each difference.

pico paths are under `crates/pico/src` and `crates/pico/tests` of this repo. Reactively is `packages/core/src/core.ts` of `milomg/reactively`; Preact signals is `packages/core/src/index.ts` of `preactjs/signals` (both read at their `main` branches; no line numbers, because neither checkout is local).

The package is `@isograph/reactive-graph` in `libs/reactive-graph` (decision 1, decided; pico-runtime.md open decision 18).

## Goal

The engine is a graph of values. A source node holds a value that code outside the graph sets. A derived node holds the value of a computation over source nodes and other derived nodes. The engine records which nodes each computation read, marks the derived nodes above a written source, and recomputes a derived node only when it is read and an input it read changed value.

The engine imports nothing. It names no store, record, link, key, reader AST, entity, resolver, or other Isograph type. A project with values of its own types uses it with no Isograph code. The Isograph runtime is one consumer: its server values are source nodes and its resolver results are derived nodes (pico-runtime.md, The engine and the Isograph runtime). This doc describes the engine alone.

The engine's public behavior is pico's: which values a read returns, which computations run again after a write, cutoff by equality, dependencies recorded per computation and replaced after every computation, and a write during a computation failing (open decision 2). Its invalidation differs from pico's. pico stamps sources with epochs and verifies a demanded memo by walking its dependencies. The engine keeps reverse edges and marks the nodes above a write eagerly, then recomputes lazily on a read (Comparison with the Rust pico crate).

## The model

### Nodes and marks

A source node holds a value and the set of derived nodes whose last computation read it (`dependents`).

A derived node holds a computation, an equality, a state, and the set of derived nodes whose last computation read it. Its state is one of three marks, the marks of Reactively (`CacheClean`, `CacheCheck`, `CacheDirty`):

- `Clean`: the stored value is what the computation would return now.
- `Check`: a derived node this one read, directly or transitively, is `Dirty`. The stored value may be current.
- `Dirty`: a source node the last computation read was written, or a derived node it read recomputed to a value its equality rejected, or the node was never computed.

`Clean` and `Check` hold a value. `Dirty` holds the previous value if there was one. Every state holds the dependencies the last computation recorded, in read order.

### Eager invalidation on write

A write sets a source node's value, marks each of its dependents `Dirty`, and marks every node above those `Check`. Marking stops at a node that is already marked. A write runs no computation.

```text
a = source(1)
b = source(2)
sum = derived(read a + read b)
parity = derived(read sum % 2 === 0 ? 'even' : 'odd')
label = derived(`the sum is ${read parity}`)

pull label   -> label, parity, sum each computed once; all Clean
write a = 3  -> a.dependents = {sum} -> sum Dirty
             -> sum.dependents = {parity} -> parity Check
             -> parity.dependents = {label} -> label Check
write b = 4  -> b.dependents = {sum} -> sum already Dirty -> stop
```

Marking is transitive because nothing computes during a write: whether `parity` changes is known only by computing `sum` and then `parity`.

Invariant, outside a pull: every dependent of a node that is not `Clean` is not `Clean`. Marking keeps it, because a node that leaves `Clean` marks its dependents. A dependent is added only by a computation, which pulled the node first, and a pulled node ends `Clean`. So marking may stop at a marked node, and a second write to a source costs one step per direct dependent.

### Lazy recomputation on pull

A pull returns a derived node's current value.

- `Clean` returns the stored value.
- `Dirty` runs the computation.
- `Check` pulls the node's derived dependencies in recorded order and stops at the first one whose value changed. That dependency's recomputation marked this node `Dirty`, so this node then runs its computation. When no dependency changed, the node becomes `Clean` without running.

```text
pull label: Check
  -> parity: Check
    -> sum: Dirty -> run sum: reads a, b -> 7, and 7 !== 3
       -> sum Clean 7 -> sum.dependents = {parity} -> parity Dirty
    -> parity Dirty -> run parity: reads sum -> 'odd', and 'odd' === 'odd'
       -> parity Clean, keeps 'odd'; parity's dependents are not marked
  -> label: parity did not change -> label Clean without running
returns 'the sum is odd'
```

The pull runs `sum` and `parity` once each and does not run `label`. That is the short circuit: the cutoff happens on the pull, at the first node whose value its equality accepts.

Source dependencies are never checked during verification. A changed source marked its readers `Dirty` when it was written, so the sources of a `Check` node did not change.

Walking in recorded order is sound when a recomputation would read other nodes. A computation's later reads depend only on what it read earlier, so up to the first dependency whose value changed, the recorded reads are the reads a recomputation would make.

```text
x = source(2)
y = source(10)
gate = derived(read x % 2 === 0 ? 'even' : 'odd')
other = derived(read y * 2)
node = derived(read gate === 'even' ? read other : 0)

pull node       -> reads gate ('even'), then other (20); node.dependencies = [gate, other]
write x = 3     -> gate Dirty -> node Check
write y = 11    -> other Dirty -> node already Check -> stop
pull node: Check
  -> gate: Dirty -> run gate -> 'odd' !== 'even' -> node Dirty -> stop the walk
  -> run node: reads gate ('odd') -> 0; node.dependencies = [gate]
other is never pulled, and its computation does not run
```

### Cutoff by per-node equality

Each derived node carries an equality. After a recomputation, the engine compares the previous value with the new one. When the equality accepts, the node keeps the previous value object and does not mark its dependents. When it rejects, the node stores the new value and marks its dependents `Dirty`. The engine's default identity equality is `sameValue`; which comparison it makes is open decision 6 (pico-runtime.md open decision 20).

A source node has no equality: `writeSource` marks unconditionally, and the caller decides that the value changed (open decision 8).

### Dynamic dependencies

Every computation records its reads afresh, and the engine replaces the node's dependencies with them after every computation. A node that read `a` on one run and `b` on the next is a dependent of `b` only.

```text
useA = source(true)
a = source(1)
b = source(2)
pick = derived(read useA ? read a : read b)

pull pick         -> reads useA, a -> 1; useA.dependents = {pick}, a.dependents = {pick}
write useA = false -> pick Dirty
pull pick         -> reads useA, b -> 2; a.dependents = {}, b.dependents = {pick}
write a = 5       -> a.dependents = {} -> nothing marked
pull pick         -> Clean, returns 2 without running
```

### Pull order

Top-level pull order does not affect values, and it does not affect which computations run or how often. The graph is acyclic and every computation is pure, so the first pull that reaches a marked node brings it up to date and every later pull finds it `Clean`.

```text
from a = 1, b = 2, all Clean, then write a = 3 -> sum Dirty, parity Check, label Check

pull sum, then pull label
  pull sum: Dirty -> run sum -> 5 !== 3 -> sum Clean 5 -> parity Dirty
  pull label: Check -> parity: Dirty -> run parity -> 'odd' === 'odd' -> parity Clean
    -> label Clean without running
pull label, then pull sum
  pull label: Check -> parity: Check -> sum: Dirty -> run sum -> 5 !== 3 -> parity Dirty
    -> run parity -> 'odd' === 'odd' -> label Clean without running
  pull sum: Clean -> 5
either order: sum runs once, parity once, label not at all
```

A node recomputes at most once per write. After its pull it is `Clean`, and during a pull only a recomputation's marking of dependents runs, which reaches only nodes that are not `Clean` (the invariant). No order is stored: a pull brings a node's inputs up to date before the node uses them.

### Reverse edges, no epochs

Every edge is an object reference, held in both directions: a derived node's dependency list points down, and every node's `dependents` set points up. pico keeps forward edges only and finds nodes by id; it decides staleness with epochs (Comparison with the Rust pico crate). The engine needs no epoch: the mark on a node says whether its value is current, and the reverse edges let a write reach exactly the nodes above it. A write costs the nodes it reaches, and a pull of a `Clean` node is one step no matter how many unrelated writes happened. Both designs return the same values and run the same computations, except where open decision 8 differs.

## The public API

The API is written under open decision 2 option (f): an engine `Graph` object holds the write-during-computation guard, and every public function that reads or writes the graph takes it as its first parameter. Decision 2 is not decided. (f) is written because pico-runtime.md establishes that a guard no caller can bypass lives in the engine (its open decision 2, last paragraph), which leaves (f) and (g), and (f) has pico's shape: every pico read and write takes the database (`Database::set`, `crates/pico/src/database.rs:151`; a memo's first argument is `&Db`). Option (g) below lists the deltas for (g). Under (a) every `graph` parameter and `Graph` itself drop out.

```ts
// from libs/reactive-graph/src/index.ts
export {
  type Computation,
  type ComputeContext,
  createDerivedNode,
  createGraph,
  createSourceNode,
  type DerivedNode,
  disposeDerivedNode,
  type Graph,
  markSourceChanged,
  pull,
  sameValue,
  type SourceNode,
  sourceValue,
  writeSource,
} from './graph';
```

`AnyDerivedNode`, `StateSlot`, `DerivedNodeState`, `CleanState`, `CheckState`, `DirtyState`, `PreviousValue`, and `Dependency` are exported from `graph.ts` because the declaration of `DerivedNode` names them, and not from the package index: no consumer names them. Open decision 7 (pico-runtime.md open decision 21) covers that a consumer can still read and write a node's fields.

### Types

```ts
// from libs/reactive-graph/src/graph.ts
/** One graph of source and derived nodes. It holds the guard that rejects a write, a disposal, or
 * a top-level pull while a computation runs (open decision 2). Created by `createGraph`; every
 * public function but the constructors and `sourceValue` takes it first. */
export type Graph = {
  /** The number of computations running. Incremented and decremented only by `compute`. A count,
   * not a two-state enum, because `readDerived` recomputes a node inside the computation that
   * reads it. */
  computingDepth: number;
};

/** A value set from outside the graph, and the derived nodes whose last computation read it. */
export type SourceNode<TValue> = {
  /** Read only by `readSource` and `sourceValue`. Written only by `writeSource`. */
  value: TValue;
  /** Written only by `setDependencies`. */
  readonly dependents: Set<AnyDerivedNode>;
};

/** A computation's value, kept until an input it read changes. */
export type DerivedNode<TValue> = {
  readonly computation: Computation<TValue>;
  /** Whether two values of this node are equal, for cutoff. Called only by `recompute`, with the
   * node's previous value and the value its computation just returned. A method signature, so that
   * a `DerivedNode<T>` is an `AnyDerivedNode` (open decision 5). */
  equal(previous: TValue, next: TValue): boolean;
  /** Value, mark, and dependencies. Read and written only through `stateOf`. */
  readonly slot: StateSlot<TValue>;
  /** Every derived node whose dependencies hold this one. Written only by `setDependencies`. */
  readonly dependents: Set<AnyDerivedNode>;
};

/** A derived node of any value type: the element of every `dependents` set and of a `Derived`
 * dependency. */
export type AnyDerivedNode = DerivedNode<unknown>;

/** The holder of one derived node's state. A separate object so that a later layered graph gives a
 * node one slot per layer by changing `stateOf` alone. */
export type StateSlot<TValue> = { state: DerivedNodeState<TValue> };

/** What the node's last computation established, and whether it is still current. */
export type DerivedNodeState<TValue> =
  | CleanState<TValue>
  | CheckState<TValue>
  | DirtyState<TValue>;

/** `value` is what the computation would return now. */
export type CleanState<TValue> = {
  readonly mark: 'Clean';
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency>;
};

/** A derived node this one reads, directly or transitively, is `Dirty`. Only a `Clean` node becomes
 * `Check`, so a `Check` node always has a value. */
export type CheckState<TValue> = {
  readonly mark: 'Check';
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency>;
};

/** A source node it read was written, or a derived node it read recomputed to a value `equal`
 * rejected, or it was never computed, or it was disposed. */
export type DirtyState<TValue> = {
  readonly mark: 'Dirty';
  readonly previous: PreviousValue<TValue>;
  readonly dependencies: ReadonlyArray<Dependency>;
};

/** `NoValue`: the node was never computed, or it was disposed. */
export type PreviousValue<TValue> =
  | { readonly kind: 'NoValue' }
  | { readonly kind: 'Value'; readonly value: TValue };

/** One read of the node's last computation, in read order. A node read twice appears twice. */
export type Dependency =
  | { readonly kind: 'Source'; readonly source: SourceNode<unknown> }
  | { readonly kind: 'Derived'; readonly node: AnyDerivedNode };

/** A derived node's computation. It reads every node through `context`, and is a pure function of
 * what it reads. */
export type Computation<TValue> = (context: ComputeContext) => TValue;

/** The reads a computation makes. Built per computation by `compute`; there is no ambient current
 * computation. */
export type ComputeContext = {
  /** Records that the computation read `source`, and returns its value. */
  readSource<TValue>(source: SourceNode<TValue>): TValue;
  /** Pulls `node`, records that the computation read it, and returns its value. */
  readDerived<TValue>(node: DerivedNode<TValue>): TValue;
};
```

`TValue` is per node: a `DerivedNode<string>` reads a `DerivedNode<number>` and a `SourceNode<number>`. The links hold `AnyDerivedNode` and `SourceNode<unknown>`, and the engine's functions are generic in the node they receive, so a node reached through `dependents` is handled at `TValue = unknown`. `equal` is a method signature: under `strict` (`tsconfig.build.json`), TypeScript checks the parameters of a function-typed property contravariantly and those of a method bivariantly, so with `readonly equal: (previous: TValue, next: TValue) => boolean` a `DerivedNode<number>` is not assignable to `DerivedNode<unknown>`. The engine calls `equal` only in `recompute`, with two values the node's own computation produced, so the bivariance admits no call the engine makes with a value of another type (open decision 5).

`equal` returns `boolean`. It is a comparator, a pure yes or no with no domain names (AGENTS.md, Booleans).

### Functions

```ts
// from libs/reactive-graph/src/graph.ts
/** A graph with no computation running. */
export function createGraph(): Graph;

/** A source node holding `value`, read by no derived node. */
export function createSourceNode<TValue>(value: TValue): SourceNode<TValue>;

/** A derived node that was never computed: `Dirty`, no value, no dependencies. Its computation
 * first runs on the first pull that reaches it. */
export function createDerivedNode<TValue>(
  computation: Computation<TValue>,
  equal: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue>;

/** The value of `source`, read without recording a dependency. For code outside a computation;
 * inside one it records nothing (open decision 10). */
export function sourceValue<TValue>(source: SourceNode<TValue>): TValue;

/** Sets `source`'s value and marks its dependents. Runs no computation. The caller decided that
 * the value changed (open decision 8). Throws while a computation runs. */
export function writeSource<TValue>(
  graph: Graph,
  source: SourceNode<TValue>,
  value: TValue,
): void;

/** Marks every dependent of `source` `Dirty` and every node above those `Check`, without changing
 * its value: for a consumer whose source went away or whose value object it changed in place.
 * Throws while a computation runs. */
export function markSourceChanged(
  graph: Graph,
  source: SourceNode<unknown>,
): void;

/** `node`'s current value: `Clean` returns it, `Check` verifies, `Dirty` recomputes. Throws while a
 * computation runs: a computation reads other nodes through its context. A computation or an
 * equality must not throw (Throws). */
export function pull<TValue>(graph: Graph, node: DerivedNode<TValue>): TValue;

/** Removes `node` from the dependents of everything it read, resets it to never computed, and marks
 * its dependents `Dirty`. A later pull computes it again (open decision 9). Throws while a
 * computation runs. */
export function disposeDerivedNode(graph: Graph, node: AnyDerivedNode): void;

/** The identity equality of a node whose value is compared by identity (open decision 6). */
export function sameValue(previous: unknown, next: unknown): boolean;
```

Callers. In the Isograph runtime (pico-runtime.md, The engine's public API): `createSourceNode` and `sourceValue` in `store.ts`, `writeSource` in `normalize.ts` and `test/fixtures.ts`, `markSourceChanged` and `disposeDerivedNode` in `deleteRecord`, `createDerivedNode`, `pull`, and `sameValue` in `storedResults.ts`. Under (f) `createGraph` is called by `createIsographEnvironment` or by whoever builds the `Store`, which becomes `{ readonly graph: Graph; readonly records: ... }` (pico-runtime.md open decision 2 (f)). Outside Isograph: the consumer's own code, as in The package README.

### Option (g): each node holds its graph

Under open decision 2 (g), a node records its graph at construction and the public functions read it from the node.

```ts
// from libs/reactive-graph/src/graph.ts
// under open decision 2 (g); deltas from the API above
export type SourceNode<TValue> = {
  readonly graph: Graph;
  value: TValue;
  readonly dependents: Set<AnyDerivedNode>;
};
export type DerivedNode<TValue> = {
  readonly graph: Graph;
  readonly computation: Computation<TValue>;
  equal(previous: TValue, next: TValue): boolean;
  readonly slot: StateSlot<TValue>;
  readonly dependents: Set<AnyDerivedNode>;
};
export function createSourceNode<TValue>(
  graph: Graph,
  value: TValue,
): SourceNode<TValue>;
export function createDerivedNode<TValue>(
  graph: Graph,
  computation: Computation<TValue>,
  equal: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue>;
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void;
export function markSourceChanged(source: SourceNode<unknown>): void;
export function pull<TValue>(node: DerivedNode<TValue>): TValue;
export function disposeDerivedNode(node: AnyDerivedNode): void;
```

`compute` reads `node.graph` instead of a parameter. Under (g), a computation of a node in graph `G` that reads a node of graph `H` through its context is representable; `readSource` and `readDerived` throw on it, because a dependents edge across graphs would let a write in `H` mark a node whose guard is `G`'s. Under (f), nodes do not know their graph, so passing a node of one graph to another graph's `pull` is not detected; the guard of the graph passed is the one checked.

## The implementation

`graph.ts` is the whole engine. Every function below is in it. No code outside `graph.ts` reads or writes a node's `value`, `dependents`, or `slot`, or calls an unexported function.

### Construction and plain reads

```ts
// from libs/reactive-graph/src/graph.ts
export function createGraph(): Graph {
  return { computingDepth: 0 };
}

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return { value, dependents: new Set() };
}

export function createDerivedNode<TValue>(
  computation: Computation<TValue>,
  equal: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue> {
  return {
    computation,
    equal,
    slot: { state: neverComputed() },
    dependents: new Set(),
  };
}

/** The state of a node that was never computed or was disposed. */
function neverComputed<TValue>(): DirtyState<TValue> {
  return { mark: 'Dirty', previous: { kind: 'NoValue' }, dependencies: [] };
}

export function sourceValue<TValue>(source: SourceNode<TValue>): TValue {
  return source.value;
}

/** The slot of `node`'s state. Every read and write of a node's value, mark, and dependencies goes
 * through it, so that a later layered graph changes this function alone. Not exported. */
function stateOf<TValue>(node: DerivedNode<TValue>): StateSlot<TValue> {
  return node.slot;
}

export function sameValue(previous: unknown, next: unknown): boolean {
  return previous === next;
}
```

`sameValue` is written for open decision 6 (a), `===`; under the held (b) its body is `Object.is(previous, next)`.

### The guard

```ts
// from libs/reactive-graph/src/graph.ts
/** Throws when a computation of `graph` is running. Called first by every public function that
 * writes the graph and by the public `pull`. */
function assertNotComputing(graph: Graph): void {
  if (graph.computingDepth > 0) {
    throw new Error(
      'Cannot write or pull the graph while a computation is running.',
    );
  }
}
```

The message follows pico's "Cannot modify database while a memoized function is being invoked." (`crates/pico/src/database.rs:193-198`). The engine throws a plain `Error`: no consumer distinguishes the guard's error by type, so no error class exists (AGENTS.md, nothing we do not use).

### Mark

```ts
// from libs/reactive-graph/src/graph.ts
export function writeSource<TValue>(
  graph: Graph,
  source: SourceNode<TValue>,
  value: TValue,
): void {
  assertNotComputing(graph);
  source.value = value;
  source.dependents.forEach(markDirty);
}

export function markSourceChanged(
  graph: Graph,
  source: SourceNode<unknown>,
): void {
  assertNotComputing(graph);
  source.dependents.forEach(markDirty);
}

/** `Clean`: becomes `Dirty` and marks its dependents `Check`. `Check`: becomes `Dirty`; its
 * dependents are already marked. `Dirty`: unchanged. Called by `writeSource`, `markSourceChanged`,
 * `recompute` (on the dependents of a node whose value changed), and `disposeDerivedNode`. */
function markDirty<TValue>(node: DerivedNode<TValue>): void {
  const slot = stateOf(node);
  const state = slot.state;
  switch (state.mark) {
    case 'Clean':
      slot.state = {
        mark: 'Dirty',
        previous: { kind: 'Value', value: state.value },
        dependencies: state.dependencies,
      };
      node.dependents.forEach(markCheck);
      return;
    case 'Check':
      slot.state = {
        mark: 'Dirty',
        previous: { kind: 'Value', value: state.value },
        dependencies: state.dependencies,
      };
      return;
    case 'Dirty':
      return;
  }
}

/** A `Clean` node becomes `Check` and marks its dependents `Check`. A marked node is left as is.
 * Called only by `markDirty` and by itself. */
function markCheck<TValue>(node: DerivedNode<TValue>): void {
  const slot = stateOf(node);
  const state = slot.state;
  if (state.mark !== 'Clean') {
    return;
  }
  slot.state = {
    mark: 'Check',
    value: state.value,
    dependencies: state.dependencies,
  };
  node.dependents.forEach(markCheck);
}
```

`markDirty` and `markCheck` never add or remove an element of a `dependents` set, so iterating a set while marking is safe.

### Pull

```ts
// from libs/reactive-graph/src/graph.ts
export function pull<TValue>(graph: Graph, node: DerivedNode<TValue>): TValue {
  assertNotComputing(graph);
  return pullNode(graph, node);
}

/** The unguarded pull. Called by `pull` after the guard, by `readDerived`, and by `verify`. */
function pullNode<TValue>(graph: Graph, node: DerivedNode<TValue>): TValue {
  const state = stateOf(node).state;
  switch (state.mark) {
    case 'Clean':
      return state.value;
    case 'Check':
      return verify(graph, node, state);
    case 'Dirty':
      return recompute(graph, node, state);
  }
}

/** Pulls each `Derived` dependency in recorded order until one recomputes to a value its `equal`
 * rejects, which marks `node` `Dirty`; then recomputes `node`. Otherwise `node` ends `Clean`
 * without running. */
function verify<TValue>(
  graph: Graph,
  node: DerivedNode<TValue>,
  state: CheckState<TValue>,
): TValue {
  state.dependencies.some((dependency) => {
    if (dependency.kind === 'Derived') {
      pullNode(graph, dependency.node);
    }
    return stateOf(node).state.mark === 'Dirty';
  });
  const after = stateOf(node).state;
  switch (after.mark) {
    case 'Dirty':
      return recompute(graph, node, after);
    case 'Check':
      stateOf(node).state = {
        mark: 'Clean',
        value: after.value,
        dependencies: after.dependencies,
      };
      return after.value;
    case 'Clean':
      return after.value;
  }
}

/** Runs the computation, compares its value with the previous one, replaces the dependencies, and
 * stores the new value unless `equal` accepted it, in which case the previous value object stays.
 * A new value marks the dependents `Dirty`. A throw out of the computation or out of `equal` is a
 * programmer error; it leaves the state and every `dependents` set untouched. */
function recompute<TValue>(
  graph: Graph,
  node: DerivedNode<TValue>,
  state: DirtyState<TValue>,
): TValue {
  const computed = compute(graph, node.computation);
  const previous = state.previous;
  if (previous.kind === 'Value' && node.equal(previous.value, computed.value)) {
    setDependencies(node, state.dependencies, computed.dependencies);
    stateOf(node).state = {
      mark: 'Clean',
      value: previous.value,
      dependencies: computed.dependencies,
    };
    return previous.value;
  }
  setDependencies(node, state.dependencies, computed.dependencies);
  stateOf(node).state = {
    mark: 'Clean',
    value: computed.value,
    dependencies: computed.dependencies,
  };
  node.dependents.forEach(markDirty);
  return computed.value;
}

/** A computation's value and the reads it made, in read order. */
type Computed<TValue> = {
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<Dependency>;
};

/** Runs `computation` with a context that records what it reads, in read order, while
 * `graph.computingDepth` counts it. */
function compute<TValue>(
  graph: Graph,
  computation: Computation<TValue>,
): Computed<TValue> {
  const dependencies: Dependency[] = [];
  graph.computingDepth += 1;
  try {
    const value = computation({
      readSource: (source) => {
        dependencies.push({ kind: 'Source', source });
        return source.value;
      },
      readDerived: (node) => {
        const read = pullNode(graph, node);
        dependencies.push({ kind: 'Derived', node });
        return read;
      },
    });
    return { value, dependencies };
  } finally {
    graph.computingDepth -= 1;
  }
}
```

- `verify` reads the node's state again after each dependency's pull, because a dependency's `recompute` marks this node `Dirty` through `dependents`.
- `readDerived` records the dependency after the pull returns. A pull that throws records nothing, and the throw leaves the reading computation, so no dependency list holds a node whose pull threw.
- `recompute` calls `equal` before `setDependencies`. A throw out of `equal` then leaves the node's dependency list and every `dependents` set consistent. Called after, a throw would leave the node in the `dependents` sets of the new dependencies while its state lists the old ones, and a later `setDependencies` from the old list would never remove it from the sets only the failed run added. Both branches store `computed.dependencies`, so a node whose value came back equal still depends on what it read this time.
- The `finally` in `compute` restores `computingDepth` when the computation throws, so the graph accepts writes after a failed pull.
- The context is a parameter built per computation. There is no ambient current computation (Reactively's `CurrentReaction`, Preact's `evalContext`), so a computation's reads cannot be recorded on the wrong node, and two graphs never share tracking state.

### Dependencies

```ts
// from libs/reactive-graph/src/graph.ts
/** Removes `node` from the dependents of each of `previous` and adds it to those of each of `next`.
 * The only writer of any `dependents` set. Called by `recompute` and `disposeDerivedNode`. */
function setDependencies(
  node: AnyDerivedNode,
  previous: ReadonlyArray<Dependency>,
  next: ReadonlyArray<Dependency>,
): void {
  previous.forEach((dependency) => dependentsOf(dependency).delete(node));
  next.forEach((dependency) => dependentsOf(dependency).add(node));
}

/** The `dependents` set of the node `dependency` names. */
function dependentsOf(dependency: Dependency): Set<AnyDerivedNode> {
  switch (dependency.kind) {
    case 'Source':
      return dependency.source.dependents;
    case 'Derived':
      return dependency.node.dependents;
  }
}
```

Removing every old edge and then adding every new one is `O(old + new)`. Removal runs first, so a dependency in both lists ends present, and a dependency read twice is one member of its set. A violation of "the `dependents` sets are the inverse of the dependency lists" is a silently stale result, which one writer guards.

The dependency list keeps a node read twice as two entries. pico drops a read equal to the last one recorded (`TrackedDependencies::push`, `crates/pico/src/dependency.rs:33-41`). The engine does not: `Set.add` makes the edge idempotent, and the second verify step on that node finds it `Clean` and returns at once, so dropping repeats would cost a comparison per read to save a step that costs one.

Reactively reuses the unchanged prefix of the previous source list (`CurrentGets`, `CurrentGetsIndex`) to avoid detaching and reattaching it. The engine replaces the whole list. The prefix reuse saves set operations on a recomputation that reads what it read before; it is an optimization behind the same `setDependencies` contract and can be added later without an API change.

### Disposal

```ts
// from libs/reactive-graph/src/graph.ts
export function disposeDerivedNode(graph: Graph, node: AnyDerivedNode): void {
  assertNotComputing(graph);
  const slot = stateOf(node);
  setDependencies(node, slot.state.dependencies, []);
  slot.state = neverComputed();
  node.dependents.forEach(markDirty);
}
```

- The node leaves the `dependents` set of everything it read, so no later write reaches it, and its state is reset to never computed, so its state lists no dependency that does not list it back.
- Its value is dropped (`NoValue`), so the engine holds no reference to it.
- Its dependents are marked `Dirty`: their last computation read a value that no longer exists. Their dependency lists still name the disposed node, and `node.dependents` still names them, so the inverse holds. Their next recomputation either reads the node again, which computes it from scratch (open decision 9), or does not, and `setDependencies` removes them from `node.dependents`.
- Disposing a node that was never computed or was already disposed changes nothing but marking its dependents, of which a never-computed node has none.

pico-runtime.md's body of `disposeDerivedNode` (The graph) calls `setDependencies` and marks dependents but leaves the state as it was. A `Clean` node disposed that way keeps its value and its dependency list while no source lists it, so a later pull returns its old value forever. The reset closes that.

### Throws

A computation and an equality must not throw. A throw out of either is a programmer error, and the engine defines no behavior for the node that threw: no retry, no stored error, no value. The engine guarantees only that its own state is not corrupted. A throw out of a computation propagates out of `compute`, and a throw out of `equal` propagates out of `recompute`, before `setDependencies` and before any state is written, so every mark, value, dependency list, and `dependents` set is as it was before the throwing run. The `finally` in `compute` restores `computingDepth`, so the graph accepts writes after the throw. Nodes pulled inside the failed computation before the throw are `Clean` and correct, because each pull completed.

pico-runtime.md open decision 3 (a) has the Isograph runtime catch the walk's throw inside its computation and return it as a value, so an Isograph computation does not throw.

### Cycles

The body has no cycle check. A computation that reads its own node, directly or through other nodes, makes `pullNode` re-enter a node that is recomputing or verifying. With no check, the re-entry recurses until the JavaScript stack overflows and a `RangeError` propagates as any throw does. Marking terminates on a cycle, because it stops at a marked node. Open decision 4 lists the options.

## The package README

`libs/reactive-graph/README.md`, as `libs/isograph-disposable-types/README.md` is for its package. Its text:

````markdown
# @isograph/reactive-graph

A graph of values that recomputes on read. You hold source nodes, whose values you set, and derived nodes, whose values are computations over other nodes. The engine records what each computation read. When you write a source, the engine marks the derived nodes that read it, directly or transitively, and runs nothing. When you read a derived node, the engine recomputes it only if something it read changed value, and stops at the first node whose new value equals its old one.

Use it when values are derived from inputs that change one at a time, computing a value is costly, and you read values more often than they change: a compiler's analyses over files, view data over a normalized store, a form's validation over its fields.

## Example

```ts
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  pull,
  sameValue,
  writeSource,
} from '@isograph/reactive-graph';

const graph = createGraph();
const price = createSourceNode(100);
const quantity = createSourceNode(3);

const total = createDerivedNode(
  (context) => context.readSource(price) * context.readSource(quantity),
  sameValue,
);
const isLargeOrder = createDerivedNode(
  (context) => context.readDerived(total) >= 1000,
  sameValue,
);
const banner = createDerivedNode(
  (context) => (context.readDerived(isLargeOrder) ? 'Free shipping' : ''),
  sameValue,
);

pull(graph, banner); // '' (runs total, isLargeOrder, banner)
writeSource(graph, quantity, 4); // runs nothing
pull(graph, banner); // '' (runs total and isLargeOrder; isLargeOrder is still false, so banner does not run)
writeSource(graph, quantity, 10);
pull(graph, banner); // 'Free shipping' (runs all three)
```

## Rules

- A computation reads nodes only through its `context`: `context.readSource(source)` and `context.readDerived(node)`. A read through a closure or `sourceValue` is not recorded, and the node will not recompute when that value changes.
- A computation is a pure function of what it reads. It does not write a source, dispose a node, or call `pull`; each of those throws while a computation runs.
- Each derived node has an equality. When a recomputed value is equal to the previous one, the node keeps the previous value object and the nodes above it do not recompute. Pass `sameValue` for identity, or your own comparator for values you build fresh on each run.
- `writeSource` always marks. Skip the call when the value did not change.
- A computation and an equality must not throw. A throw is a programmer error: it propagates out of `pull`, and the engine guarantees only that its own state is as it was before that pull.
- `disposeDerivedNode` detaches a node you no longer need. Nodes that read it recompute on their next pull.
- A computation must not read its own node, directly or through other nodes.
````

The README's rules restate the engine's behavior under the options the body is written for: open decision 2 (f), 8 (a), 9 (a), and 10 (a). Each decision that changes names the README line it changes.

## The design

This section is the engine's counterpart of `docs-website/docs/design-docs/pico.md`. It describes what each part is and why, for an agent working inside the engine or on a consumer of it.

### Sources

A source node is an input fact: a value the graph does not compute. The consumer creates it with `createSourceNode(value)` and holds the returned object. The object is the source's identity. There is no key, no lookup by key, and no absent source: a source exists from creation and always holds a value. A consumer that needs "not set yet" stores a value of its own that says so (`undefined`, a sentinel symbol, or a tagged union), and readers depend on that value like any other. The Isograph runtime's `MISSING` is such a value.

A source is written with `writeSource(graph, source, value)`. The write sets the value and marks. It does not compare: the caller already knows whether the value changed, and a caller that does not know compares before calling (open decision 8). `markSourceChanged(graph, source)` marks without setting, for a consumer that changed the value object in place or whose source went away.

Code outside a computation reads a source with `sourceValue(source)`, which records nothing. A computation reads it with `context.readSource(source)`.

A source is not collected by the engine. It lives as long as the consumer and the dependency lists of its readers hold it.

### Derived nodes

A derived node is a computation, an equality, a state, and a set of dependents. The consumer creates it with `createDerivedNode(computation, equal)` and holds the returned object. The object is the node's identity. A consumer that wants one node per key (pico's memo arguments) keeps its own map from key to node; the engine has no memo table. The Isograph runtime's record containers are such a map.

The computation receives a `ComputeContext` and returns the value. It must be a pure function of what it reads through the context: no clock, no network, no read through a closure. A computation runs only during a pull: first on the first pull that reaches the node, and again on a pull that finds it `Dirty`. It never runs during a write.

Two pulls with no write between them run nothing the second time. A pull of a `Clean` node is one step.

### How dependencies are recorded

`compute` builds a fresh context for each run. `readSource` appends a `Source` dependency and returns the value. `readDerived` pulls the node, appends a `Derived` dependency, and returns its value. The list is in read order and keeps repeats.

When the computation returns, `recompute` replaces the node's dependency list with the new one and moves the reverse edges with `setDependencies`. The new list replaces the old one even when the value came back equal. A read that a branch skips is not a dependency: a node that stopped reading `a` is not marked by a write of `a`.

Only `readSource` and `readDerived` record. A computation that reads a source through `sourceValue`, or a derived node through `pull`, records nothing; `pull` throws during a computation (the guard), `sourceValue` does not (open decision 10). pico has the same hole and the same rule: `MemoRef::lookup` reads without recording and `lookup_tracked` records (`crates/pico/src/memo_ref.rs:57`, `:80`; pico.md, Memo).

### Verification

A write marks the nodes above it. A pull of a `Check` node verifies: it pulls the node's derived dependencies in recorded order, each of which is `Clean` (one step), `Check` (verified the same way), or `Dirty` (recomputed). When one of them recomputes to a new value, its `recompute` marks this node `Dirty`, the walk stops, and this node recomputes. When the walk ends with the node still `Check`, every derived input came back equal and no source input was written, so the node becomes `Clean` without running.

The walk stops at the first changed dependency because the recomputation that follows decides which nodes are read next. A dependency after the first changed one may be one the recomputation never reads, and pulling it would run a computation whose value nobody needs.

### Cutoff

`equal(previous, next)` decides whether the nodes above a recomputed node recompute. When it accepts, the node keeps the previous value object, so a reader comparing by identity sees no change, and its dependents stay as marked: a `Check` dependent then verifies to `Clean` without running. When it rejects, the node stores the new value and marks its dependents `Dirty`.

An equality may answer equal only when no reader can tell the two values apart. An equality that answers equal for distinguishable values makes readers keep a stale value. An equality that answers unequal for indistinguishable values costs recomputation, never a wrong value. `sameValue` is the engine's identity comparison (open decision 6). A consumer whose computation builds a new object on each run supplies a structural comparator, or the nodes above it recompute on every change below.

pico's equivalent is backdating: an equal recomputation keeps the old `time_updated` (pico.md, Equality and backdating).

### Disposal

`disposeDerivedNode(graph, node)` is the only way a derived node leaves the graph. It detaches the node from what it read, resets it to never computed, and marks its dependents `Dirty` (Disposal). The engine does not decide when to dispose: there is no reachability walk, no LRU, and no reference count in the engine's first implementation. The consumer disposes a node when it drops the key that held it. pico collects instead (`run_garbage_collection`, `crates/pico/src/database.rs:169`), keeping the last 10,000 top-level calls and what they reach (pico.md, Garbage collection); a collection policy for the engine is pico-runtime.md's Later: collecting derived nodes.

A derived node with no dependencies is computed once and stays `Clean` under every write, because no source lists it. A `Check` dependent's verify pulls it in one step. Only disposal changes it, which marks its dependents `Dirty`; its next pull computes it again. In the Isograph runtime, a refinement such as `asPet` is a derived node of this kind.

### Writes during a computation

A computation that reaches a source or a node through a closure could write it while it computes. The write would mark nodes whose recorded dependencies are being replaced, and the computing node would end `Clean` over an input that changed during its run:

```text
p Dirty -> recompute p: reads c (Clean) -> p's computation writes s -> s.dependents = {c} -> c Dirty
  -> c.dependents = {} (p is added only after compute returns)
p finishes -> setDependencies: c.dependents = {p} -> p Clean, over a Dirty c
write s again -> c already Dirty -> stop -> p is never marked
```

The guard rejects `writeSource`, `markSourceChanged`, `disposeDerivedNode`, and the public `pull` while `graph.computingDepth > 0`. A read through the context is allowed, because it records the dependency. A public `pull` inside a computation is rejected because it records nothing on the outer node. Open decision 2.

## Comparison with the Rust pico crate

The data model is pico's:

- Sources are set from outside; derived nodes are values of computations (`source.rs:83-86`, `derived_node.rs:55-58`).
- A computation's reads are recorded as it runs, and the list replaces the previous one after each run (`invoke_with_dependency_tracking`, `execute_memoized_function.rs:249`).
- An equal recomputation cuts off: pico keeps `time_updated` when `prev_value == value` (`update_derived_node`, `execute_memoized_function.rs:163`); the engine keeps the previous value object and does not mark dependents.
- Verification walks dependencies in recorded order and stops at the first changed one (`any_dependency_changed`, `execute_memoized_function.rs:183-206`, an `Iterator::any` over the list; `derived_node_changed_since`, `:208-247`, re-executes a derived dependency to find out).
- A write during a computation fails (`assert_empty_dependency_stack`, `database.rs:193-198`; open decision 2).
- A cycle fails, in pico by a panic (`assert_no_cycles`, `dependency.rs:95-99`; open decision 4).

Invalidation differs:

- pico has a database-wide epoch. A write that changes a source increments it and stamps the source (`set_source`, `database.rs:310-329`). A demanded memo verified in the current epoch returns at once (`node_verified_in_current_epoch`, `execute_memoized_function.rs:80`); otherwise pico walks its dependencies and compares each one's `time_updated` with the memo's `time_verified`. After any write anywhere, the first demand of a memo walks every dependency beneath it, though it runs nothing that did not change. The engine marks on write through reverse edges, so a write costs the nodes above it and a pull after an unrelated write is one step.
- pico keeps forward edges only, as ids into storage; the engine keeps object references in both directions.
- pico checks source dependencies during verification; the engine does not, because a changed source already marked its readers `Dirty`.
- pico's `set` compares the new source value with the old one and does not advance the epoch when they are equal (`database.rs:321`). The engine's `writeSource` does not compare (open decision 8).
- pico finds a memo by its function and arguments, interned as `DerivedNodeId`; the engine has no keys, and the consumer holds node objects.
- pico has one equality per type (`PartialEq` through `DynEq`); the engine has one per node.
- pico removes a source by key, and a later `get` of its id panics (`removing_params.rs`); the engine has no removal, and a source object cannot dangle.
- pico collects derived nodes by LRU and reachability; the engine disposes on request.

Observable behavior is the same: for any sequence of writes and reads, both return the same values and run the same computations, except that pico skips a source write of an equal value and the engine marks it (open decision 8).

## Comparison with Reactively and Preact signals

Reactively is the same scheme. Its states are `CacheClean`, `CacheCheck`, and `CacheDirty`. `set` marks direct observers `CacheDirty` through `stale`, which marks their observers `CacheCheck` and stops at a node already at or above the requested state. `updateIfNecessary` walks `sources` in order when `CacheCheck`, breaks when the node becomes `CacheDirty`, and then calls `update`. Its default equality is `===` (`defaultEquality`), per node through `equals`. A throw out of the function propagates and leaves the state and the links unchanged, as the engine's does. It has no cycle check. Its differences from the engine: an ambient `CurrentReaction` global records reads where the engine passes a context; `set` compares and skips an equal value, where `writeSource` does not (open decision 8); it reuses the unchanged prefix of the source list; and it schedules effects (`stabilize`), which the engine's core has none of.

Preact signals uses versions instead of marks. Every signal has a `_version`, a global `globalVersion` counts writes, and each dependency link (a doubly linked list `Node` with `_source`, `_target`, and `_version`) records the version the target last saw. A computed decides with `needsToRecompute`, which walks its sources in order and returns at the first whose version changed, refreshing derived sources as it goes: pico's epochs per link, with pico's in-order walk. Writes notify effects eagerly through flags (`OUTDATED`, `NOTIFIED`). Its comparison is `!==`, a setter with an equal value notifies nothing, and a computed that throws stores the error (`HAS_ERROR`) and rethrows it on read; the engine forbids the throw (Throws). It throws "Cycle detected" when a computed is read while `RUNNING`, which is open decision 4 (b). It records reads through an ambient `evalContext`, and a signal written inside a computed is allowed.

The engine takes Reactively's marking and pull, pico's equality, guard, and per-computation dependency recording, and neither library's ambient tracking context.

## Tests

The tests assert values with `toBe` and `toEqual`, and assert which computations ran by counting calls in a `calls` object each test owns. Counting re-invocations is allowed: which writes make which computations run again is the engine's own dependency tracking (AGENTS.md, Tests). No test reads a node's `slot`, `dependents`, or mark, and no test calls an unexported function: marks and edges are implementation, and each test observes them only through values and call counts. There are no snapshot tests and no fixture files; every input is built in the test.

The tests are written under the options the body is written for: open decision 2 (f), 4 with no check, 6 (b) where a test depends on it, 8 (a), 9 (a), and 10 (a). Each test whose assertions change under another option says how.

### The pico crate's tests

`crates/pico/tests/tests.rs` declares the modules under `absent_source`, `garbage_collection`, `params`, and `tracking_field`; each other file is its own test crate. pico has no `#[test]` in `crates/pico/src` or `crates/pico_macros/src`. There are 58 tests. 21 port, 37 do not.

Each port keeps pico's scenario and counters and states its delta. pico's memo keyed by a `SourceId` becomes a derived node whose computation closes over the source node, pico's `db.set` of an existing key becomes `writeSource`, and pico's `*memo(&db, ..)` at top level becomes `pull(graph, node)`.

Top-level files:

- `basic.rs:14` `basic`: ports as `recompute.test.ts` "a derived node recomputes after its source is written".
- `basic_multi_function_chain.rs:21` `multi_function_chain`: ports as "a chain recomputes every node whose input changed".
- `basic_multi_function_chain.rs:48` `multi_function_chain_with_irrelevant_change`: ports as "an equal value cuts off the node above it".
- `basic_multi_function_chain.rs:75` `sequential_functions_with_memoref_param`: ports as "a top-level pull of an inner node, then of the node above it". Delta: pico passes the inner memo's `MemoRef` as the outer memo's argument and reads it with `lookup_tracked`; the engine's outer computation closes over the inner node and reads it with `readDerived`. The `MemoRef` parameter is pico representation; the behavior kept is that pulling the inner node first does not change what the outer pull runs.
- `cyclic_dependency_panic.rs:11` `test_calls_itself`, `:20` `test_calls_calls_itself`, `:29` `test_a`, `:38` `test_calls_a`: port as `cycles.test.ts`'s four tests. Delta: pico panics; the engine's pull throws (a `RangeError` with no check, open decision 4 (a); "Cyclic dependency detected." under (b)).
- `equal_reset_after_unrelated_epoch.rs:14` `equal_reset_after_unrelated_epoch_does_not_rerun`: does not port. It pins epoch bookkeeping: an unrelated set advances the epoch, and a later equal set of the read source must not move that source's `time_updated`. The engine has no epoch, and an unrelated write reaches no reader of another source; the behavior left is "an equal write", which the port of `overriding_same_source` covers.
- `intern.rs:12` `intern`, `intern_ref.rs:12` `intern_ref`, `intern_then_intern_ref.rs:10` `intern_then_intern_ref`: do not port. They pin `intern_value` and `intern_ref` identity; the engine interns nothing.
- `intern_ref_chain.rs:14` `intern_ref_chain`: ports as "a node's own equality cuts off a selection out of a changed collection". Delta: pico cuts off because `intern_ref` of an equal value keeps its `time_updated`; the engine cuts off because the selecting node's `equal` compares the selected element's fields. The behavior kept is that a reader of one element does not run when other elements change.
- `overriding_same_source.rs:14` `overriding_same_source`: ports as "writing an equal value". Delta under open decision 8 (a): the reader of the source runs again and the node above it does not; pico's assertion that the reader does not run holds under (b).
- `partial_reuse_multi_function.rs:16` `multi_chain_reuse`: ports as "two readers share one computation of their dependency".
- `partial_reuse_single_chain.rs:15` `single_chain_reuse`: ports as "a node pulled before its reader is not computed again for the reader".
- `removing_params.rs:11` `removing_direct_param_causes_panic`, `:26` `removing_indirect_param_causes_panic`: do not port. They pin that a `SourceId` after `remove` is a false proof and `get` panics. The engine has no removal by key, and a source node is an object that cannot dangle. The engine's removal is disposal of a derived node, which `disposal.test.ts` covers.
- `return_value_not_cloned.rs:14` `arg_reference`: ports as "a pull returns the stored value object". Delta: pico counts `Clone` calls; the engine never copies, so the port asserts identity (`toBe`) of the returned object across pulls and across a recomputation its equality accepts.
- `same_source_key.rs:10` `same_source_key`: does not port. It pins that pico's source `Key` hashes the type with the key field, so two types with one key do not collide. The engine has no keys.
- `side_chain.rs:16` `side_chain`: ports as "a dependency with no inputs is not computed again when its reader recomputes".
- `singleton.rs:14` `singleton`: does not port. It pins keyless singleton sources and `get_singleton` returning `None` before the first set; the engine has no keys and no absent source.
- `store_memo_ref.rs:15` `store_memo_ref`: does not port. It pins that a memo can return a `MemoRef` to a raw memo and that `lookup` of it reads the latest value; that is `MemoRef` representation. Its cutoff (the outer memo does not run when the inner value is equal) is the port of `multi_function_chain_with_irrelevant_change`.
- `unrelated_source_change.rs:14` `unrelated_source_changes`: ports as "a write to another source runs nothing".

`absent_source`:

- `missing_key.rs:57` `missing_then_set_reruns`: ports as `dependencies.test.ts` "a source that holds the consumer's absence value". Delta: pico reads an absent key; the engine's source holds a sentinel the test defines, because an engine source always holds a value.
- `missing_key.rs:78` `missing_and_still_missing_across_epochs_does_not_rerun`: does not port. It pins that an absent key's dependency stays at the initial epoch across unrelated epochs; there is no epoch and no absent key.
- `missing_key.rs:100` `unrelated_set_does_not_rerun_reader_of_missing_key`: does not port. It pins keyed absence (`OtherConfig` versus `Config`); what is left is "a write to another source runs nothing", the port of `unrelated_source_changes`.
- `missing_key.rs:133` `set_reruns_only_readers_of_that_key`: does not port, for the same reason; the remainder is the ports of `basic` and `unrelated_source_changes`.
- `missing_key.rs:164` `same_missing_key_twice_in_one_memo`: ports as "a computation that reads one source twice". Delta: the source holds a value; the port adds a second write to assert that the edge survives the duplicate read.
- `missing_key.rs:192` `parent_of_missing_reader_reruns`: does not port. It pins that an absent key's dependency propagates to a parent memo; with absence as a value, the remainder is the port of `multi_function_chain`.
- `missing_key.rs:212` `read_outside_a_memo_then_set`: ports as "sourceValue outside a computation, then a reader created after the write".
- `missing_key.rs:233` `read_outside_a_memo_after_absent_node_exists`: ports as "sourceValue outside a computation leaves the readers' dependency".
- `remove.rs:52` `present_then_removed_reruns_and_returns_none`, `:77` `removed_then_set_reruns`, `:102` `removed_and_still_removed_does_not_rerun`, `:128` `remove_of_missing_key_does_not_rerun`, `:145` `remove_of_never_read_key_then_read`: do not port. They pin `remove_singleton`'s epoch rules (a remove of a present key advances the epoch, a remove of an absent key does not). The engine has no removal; a consumer writes its absence value, which is a write like any other (the ports of `basic` and `overriding_same_source`).
- `try_get.rs:43` `set_of_never_set_key_reruns_reader`, `:64` `unrelated_set_does_not_rerun_reader_of_never_set_key`, `:83` `id_for_key_matches_the_id_set_returns`: do not port. They pin `try_get` of a never-set key and `id_for_key`; the engine has no keys.
- `garbage_collection.rs:46` `collected_reader_of_missing_key_sees_set`: ports as `disposal.test.ts` "a disposed node recomputes on its next pull and sees writes made after disposal". Delta: pico's LRU collects the reader; the test disposes it.
- `garbage_collection.rs:72` `retained_reader_of_missing_key_sees_set`, `:97` `remove_after_collection`: do not port. They pin that an LRU-retained reader keeps its dependency through `run_garbage_collection`; the engine has no collection, and a node not disposed keeps its edges (every other test).

`garbage_collection`:

- `basic_gc.rs:25` `basic_gc`, `inner_retained.rs:27` `inner_retained`, `multiple_calls.rs:27` `multiple_calls`, `outer_retained.rs:27` `outer_retained`, `retained.rs:26` `basic_retained`, `retained_and_in_lru.rs:26` `basic_retained`, `retained_clear.rs:26` `clear_retained`: do not port. They pin pico's collection policy: the LRU of top-level calls, its capacity, reachability from retained calls, and `retain` / `clear_retain`. The engine disposes on request and has no policy (The design, Disposal).

`params`:

- `memo_ref_never_cloned.rs:14` `memo_ref_never_cloned`, `other_param_cloned_on_execute.rs:19` `owned_param`, `:51` `borrowed_param`, `source_id_never_cloned.rs:15` `source_id_never_cloned`, `with_serialize.rs:16` `with_serialize`: do not port. They pin param interning and when pico clones a param; the engine has no params.

`tracking_field`:

- `correctness.rs:17` `tracking_field_correctness`: ports as `dependencies.test.ts` "sourceValue inside a computation records nothing". Delta: pico's untracked iteration of a `#[tracked]` map becomes `sourceValue` inside a computation, which is the engine's untracked read (open decision 10), and pico's tracked iteration becomes `readSource`.
- `efficiency.rs:23` `untracked_efficiency`, `:57` `tracked_inefficiency`: do not port. They pin the `#[tracked]` counter's granularity (a tracked read depends on the whole map); the engine has no tracked map, and a consumer that wants per-key granularity holds one source per key, which the port of `unrelated_source_changes` covers.
- `never_written.rs:54` `iteration_before_first_write_then_insert_reruns`, `:79` `untracked_miss_then_tracked_then_insert_reruns`, `:91` `first_tracked_write_writes_counter_zero`: do not port. They pin the tracked counter's never-written singleton; the engine has neither.

### Test files

```text
libs/reactive-graph/test/
  recompute.test.ts      ports of chains, reuse, cutoff; marking and short circuit; pull order; diamond
  dependencies.test.ts   recorded order, branch changes, repeated reads, absence as a value, untracked reads
  errors.test.ts         a throwing computation leaves the graph's state unchanged
  disposal.test.ts       disposal
  guard.test.ts          the write-during-computation guard (open decision 2 (f))
  cycles.test.ts         cycles (open decision 4)
  degenerate.test.ts     degenerate nodes, sources, and values
  graph.test-d.ts        types
```

Every test file imports from `../src`, which is `src/index.ts`. `graph.test-d.ts` also imports `AnyDerivedNode` from `../src/graph`, the one type the index does not export that a type assertion names.

### `recompute.test.ts`

```ts
// from libs/reactive-graph/test/recompute.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  pull,
  sameValue,
  writeSource,
} from '../src';

describe('ported from crates/pico/tests', () => {
  // crates/pico/tests/basic.rs:14
  test('a derived node recomputes after its source is written', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0 };
    const input = createSourceNode('asdf');
    const firstLetter = createDerivedNode((context) => {
      calls.firstLetter += 1;
      return context.readSource(input).charAt(0);
    }, sameValue);

    expect(pull(graph, firstLetter)).toBe('a');
    expect(calls.firstLetter).toBe(1);

    writeSource(graph, input, 'qwer');
    expect(pull(graph, firstLetter)).toBe('q');
    expect(calls.firstLetter).toBe(2);
  });

  // crates/pico/tests/basic_multi_function_chain.rs:21
  test('a chain recomputes every node whose input changed', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0, capitalized: 0 };
    const input = createSourceNode('asdf');
    const firstLetter = createDerivedNode((context) => {
      calls.firstLetter += 1;
      return context.readSource(input).charAt(0);
    }, sameValue);
    const capitalized = createDerivedNode((context) => {
      calls.capitalized += 1;
      return context.readDerived(firstLetter).toUpperCase();
    }, sameValue);

    expect(pull(graph, capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1 });

    writeSource(graph, input, 'qwer');
    expect(pull(graph, capitalized)).toBe('Q');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2 });
  });

  // crates/pico/tests/basic_multi_function_chain.rs:48
  test('an equal value cuts off the node above it', () => {
    // setup as in 'a chain recomputes every node whose input changed'
    expect(pull(graph, capitalized)).toBe('A');
    writeSource(graph, input, 'alto');
    expect(pull(graph, capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 1 });
  });

  // crates/pico/tests/basic_multi_function_chain.rs:75
  test('a top-level pull of an inner node, then of the node above it', () => {
    // setup as in 'a chain recomputes every node whose input changed'
    expect(pull(graph, capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1 });

    writeSource(graph, input, 'bsdf');
    expect(pull(graph, firstLetter)).toBe('b');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 1 });
    expect(pull(graph, capitalized)).toBe('B');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2 });

    writeSource(graph, input, 'balt');
    expect(pull(graph, capitalized)).toBe('B');
    expect(calls).toEqual({ firstLetter: 3, capitalized: 2 });
  });

  // crates/pico/tests/partial_reuse_single_chain.rs:15
  test('a node pulled before its reader is not computed again for the reader', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0, exclaimed: 0 };
    const input = createSourceNode('asdf');
    const firstLetter = createDerivedNode((context) => {
      calls.firstLetter += 1;
      return context.readSource(input).charAt(0);
    }, sameValue);
    const exclaimed = createDerivedNode((context) => {
      calls.exclaimed += 1;
      return `${context.readDerived(firstLetter)}!`;
    }, sameValue);

    expect(pull(graph, firstLetter)).toBe('a');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 0 });
    expect(pull(graph, exclaimed)).toBe('a!');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 1 });
  });

  // crates/pico/tests/partial_reuse_multi_function.rs:16
  test('two readers share one computation of their dependency', () => {
    // firstLetter and exclaimed as above, plus:
    const questioned = createDerivedNode((context) => {
      calls.questioned += 1;
      return `${context.readDerived(firstLetter)}?`;
    }, sameValue);

    expect(pull(graph, exclaimed)).toBe('a!');
    expect(pull(graph, questioned)).toBe('a?');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 1, questioned: 1 });
  });

  // crates/pico/tests/side_chain.rs:16
  test('a dependency with no inputs is not computed again when its reader recomputes', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0, capitalized: 0, unchanged: 0 };
    const input = createSourceNode('asdf');
    const firstLetter = createDerivedNode((context) => {
      calls.firstLetter += 1;
      return context.readSource(input).charAt(0);
    }, sameValue);
    const unchanged = createDerivedNode(() => {
      calls.unchanged += 1;
      return 'this node is computed once';
    }, sameValue);
    const capitalized = createDerivedNode((context) => {
      calls.capitalized += 1;
      context.readDerived(unchanged);
      return context.readDerived(firstLetter).toUpperCase();
    }, sameValue);

    expect(pull(graph, capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1, unchanged: 1 });
    expect(pull(graph, capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1, unchanged: 1 });

    writeSource(graph, input, 'qwer');
    expect(pull(graph, capitalized)).toBe('Q');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2, unchanged: 1 });
  });

  // crates/pico/tests/unrelated_source_change.rs:14
  test('a write to another source runs nothing', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0 };
    const input = createSourceNode('asdf');
    const other = createSourceNode('isograph');
    const firstLetter = createDerivedNode((context) => {
      calls.firstLetter += 1;
      return context.readSource(input).charAt(0);
    }, sameValue);

    expect(pull(graph, firstLetter)).toBe('a');
    writeSource(graph, other, 'pico');
    expect(pull(graph, firstLetter)).toBe('a');
    expect(calls.firstLetter).toBe(1);
  });

  // crates/pico/tests/return_value_not_cloned.rs:14
  test('a pull returns the stored value object', () => {
    const graph = createGraph();
    const calls = { letter: 0 };
    const input = createSourceNode('asdf');
    const letter = createDerivedNode(
      (context) => {
        calls.letter += 1;
        return { letter: context.readSource(input).charAt(0) };
      },
      (previous, next) => previous.letter === next.letter,
    );

    const first = pull(graph, letter);
    expect(pull(graph, letter)).toBe(first);

    writeSource(graph, input, 'alto');
    expect(pull(graph, letter)).toBe(first);
    expect(calls.letter).toBe(2);

    writeSource(graph, input, 'qwer');
    const second = pull(graph, letter);
    expect(second).not.toBe(first);
    expect(second).toEqual({ letter: 'q' });
  });

  // crates/pico/tests/intern_ref_chain.rs:14
  test('a node’s own equality cuts off a selection out of a changed collection', () => {
    type Economist = { readonly name: string; readonly bestBook: string };
    const economistsEqual = (
      previous: Economist | undefined,
      next: Economist | undefined,
    ): boolean =>
      previous === undefined || next === undefined
        ? previous === next
        : previous.name === next.name && previous.bestBook === next.bestBook;
    const graph = createGraph();
    const calls = { bestBook: 0 };
    const table = createSourceNode<ReadonlyArray<Economist>>([
      { name: 'John Maynard Keynes', bestBook: 'General theory' },
      {
        name: 'Milton Friedman',
        bestBook: 'A monetary history of the United States',
      },
    ]);
    const keynes = createDerivedNode(
      (context) =>
        context
          .readSource(table)
          .find((economist) => economist.name === 'John Maynard Keynes'),
      economistsEqual,
    );
    const bestBook = createDerivedNode((context) => {
      calls.bestBook += 1;
      return context.readDerived(keynes)?.bestBook;
    }, sameValue);

    expect(pull(graph, bestBook)).toBe('General theory');

    writeSource(graph, table, [
      { name: 'John Maynard Keynes', bestBook: 'General theory' },
      { name: 'Adam Smith', bestBook: 'The wealth of nations' },
    ]);
    expect(pull(graph, bestBook)).toBe('General theory');
    expect(calls.bestBook).toBe(1);
  });

  // crates/pico/tests/overriding_same_source.rs:14
  test('writing an equal value', () => {
    const graph = createGraph();
    const calls = { firstLetter: 0, exclaimed: 0 };
    // firstLetter over input = 'asdf' and exclaimed over firstLetter, as above
    expect(pull(graph, exclaimed)).toBe('a!');

    writeSource(graph, input, 'asdf');
    expect(pull(graph, exclaimed)).toBe('a!');
    // open decision 8 (a). Under (b): { firstLetter: 1, exclaimed: 1 }.
    expect(calls).toEqual({ firstLetter: 2, exclaimed: 1 });
  });
});

describe('marking and the short circuit', () => {
  test('a write runs no computation', () => {
    const graph = createGraph();
    const calls = { doubled: 0, label: 0 };
    const count = createSourceNode(1);
    const doubled = createDerivedNode((context) => {
      calls.doubled += 1;
      return context.readSource(count) * 2;
    }, sameValue);
    const label = createDerivedNode((context) => {
      calls.label += 1;
      return `${context.readDerived(doubled)}`;
    }, sameValue);
    pull(graph, label);

    writeSource(graph, count, 2);
    writeSource(graph, count, 3);
    expect(calls).toEqual({ doubled: 1, label: 1 });

    expect(pull(graph, label)).toBe('6');
    expect(calls).toEqual({ doubled: 2, label: 2 });
  });

  test('a direct reader of a written source recomputes; a reader above it recomputes only when that value changed', () => {
    const graph = createGraph();
    const calls = { parity: 0, label: 0 };
    const count = createSourceNode(1);
    const parity = createDerivedNode((context) => {
      calls.parity += 1;
      return context.readSource(count) % 2 === 0 ? 'even' : 'odd';
    }, sameValue);
    const label = createDerivedNode((context) => {
      calls.label += 1;
      return `the count is ${context.readDerived(parity)}`;
    }, sameValue);
    const first = pull(graph, label);

    writeSource(graph, count, 3);
    expect(pull(graph, label)).toBe(first);
    expect(calls).toEqual({ parity: 2, label: 1 });

    writeSource(graph, count, 4);
    expect(pull(graph, label)).toBe('the count is even');
    expect(calls).toEqual({ parity: 3, label: 2 });
  });

  test('the short circuit holds through every level above the node that came back equal', () => {
    const graph = createGraph();
    const calls = { clamped: 0, doubled: 0, label: 0 };
    const count = createSourceNode(20);
    const clamped = createDerivedNode((context) => {
      calls.clamped += 1;
      return Math.min(context.readSource(count), 10);
    }, sameValue);
    const doubled = createDerivedNode((context) => {
      calls.doubled += 1;
      return context.readDerived(clamped) * 2;
    }, sameValue);
    const label = createDerivedNode((context) => {
      calls.label += 1;
      return `${context.readDerived(doubled)}`;
    }, sameValue);
    expect(pull(graph, label)).toBe('20');

    writeSource(graph, count, 30);
    expect(pull(graph, label)).toBe('20');
    expect(calls).toEqual({ clamped: 2, doubled: 1, label: 1 });
  });

  test('a diamond computes each node once per write', () => {
    const graph = createGraph();
    const calls = { left: 0, right: 0, bottom: 0 };
    const count = createSourceNode(1);
    const left = createDerivedNode((context) => {
      calls.left += 1;
      return context.readSource(count) + 1;
    }, sameValue);
    const right = createDerivedNode((context) => {
      calls.right += 1;
      return context.readSource(count) * 2;
    }, sameValue);
    const bottom = createDerivedNode((context) => {
      calls.bottom += 1;
      return context.readDerived(left) + context.readDerived(right);
    }, sameValue);
    expect(pull(graph, bottom)).toBe(4);

    writeSource(graph, count, 2);
    expect(pull(graph, bottom)).toBe(7);
    expect(pull(graph, left)).toBe(3);
    expect(pull(graph, right)).toBe(4);
    expect(calls).toEqual({ left: 2, right: 2, bottom: 2 });
  });

  test('top-level pull order changes neither values nor what runs', () => {
    const build = () => {
      const graph = createGraph();
      const calls = { sum: 0, parity: 0, label: 0 };
      const a = createSourceNode(1);
      const b = createSourceNode(2);
      const sum = createDerivedNode((context) => {
        calls.sum += 1;
        return context.readSource(a) + context.readSource(b);
      }, sameValue);
      const parity = createDerivedNode((context) => {
        calls.parity += 1;
        return context.readDerived(sum) % 2 === 0 ? 'even' : 'odd';
      }, sameValue);
      const label = createDerivedNode((context) => {
        calls.label += 1;
        return `the sum is ${context.readDerived(parity)}`;
      }, sameValue);
      pull(graph, label);
      writeSource(graph, a, 3);
      return { graph, calls, sum, label };
    };
    const sumFirst = build();
    const labelFirst = build();

    const sumThenLabel = [
      pull(sumFirst.graph, sumFirst.sum),
      pull(sumFirst.graph, sumFirst.label),
    ];
    const labelThenSum = [
      pull(labelFirst.graph, labelFirst.label),
      pull(labelFirst.graph, labelFirst.sum),
    ];

    expect(sumThenLabel).toEqual([5, 'the sum is odd']);
    expect(labelThenSum).toEqual(['the sum is odd', 5]);
    expect(sumFirst.calls).toEqual({ sum: 2, parity: 2, label: 1 });
    expect(labelFirst.calls).toEqual(sumFirst.calls);
  });
});
```

A test whose comment says "setup as in" repeats that test's setup lines in the file; the doc elides them.

### `dependencies.test.ts`

```ts
// from libs/reactive-graph/test/dependencies.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  pull,
  sameValue,
  sourceValue,
  writeSource,
} from '../src';

const MISSING: unique symbol = Symbol('missing');

describe('recorded dependencies', () => {
  test('verification stops at the first dependency that changed, in recorded order', () => {
    const graph = createGraph();
    const calls = { gate: 0, other: 0, node: 0 };
    const x = createSourceNode(2);
    const y = createSourceNode(10);
    const gate = createDerivedNode((context) => {
      calls.gate += 1;
      return context.readSource(x) % 2 === 0 ? 'even' : 'odd';
    }, sameValue);
    const other = createDerivedNode((context) => {
      calls.other += 1;
      return context.readSource(y) * 2;
    }, sameValue);
    const node = createDerivedNode((context) => {
      calls.node += 1;
      return context.readDerived(gate) === 'even' ? context.readDerived(other) : 0;
    }, sameValue);
    expect(pull(graph, node)).toBe(20);

    writeSource(graph, x, 3);
    writeSource(graph, y, 11);
    expect(pull(graph, node)).toBe(0);
    expect(calls).toEqual({ gate: 2, other: 1, node: 2 });

    expect(pull(graph, other)).toBe(22);
    expect(calls.other).toBe(2);
  });

  test('a branch change replaces the dependencies', () => {
    const graph = createGraph();
    const calls = { pick: 0 };
    const useA = createSourceNode(true);
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const pick = createDerivedNode((context) => {
      calls.pick += 1;
      return context.readSource(useA) ? context.readSource(a) : context.readSource(b);
    }, sameValue);
    expect(pull(graph, pick)).toBe(1);

    writeSource(graph, useA, false);
    expect(pull(graph, pick)).toBe(2);
    expect(calls.pick).toBe(2);

    writeSource(graph, a, 5);
    expect(pull(graph, pick)).toBe(2);
    expect(calls.pick).toBe(2);

    writeSource(graph, b, 7);
    expect(pull(graph, pick)).toBe(7);
    expect(calls.pick).toBe(3);
  });

  test('a derived node read twice is computed once per write', () => {
    const graph = createGraph();
    const calls = { inner: 0, outer: 0 };
    const count = createSourceNode(1);
    const inner = createDerivedNode((context) => {
      calls.inner += 1;
      return context.readSource(count) * 2;
    }, sameValue);
    const outer = createDerivedNode((context) => {
      calls.outer += 1;
      return context.readDerived(inner) + context.readDerived(inner);
    }, sameValue);
    expect(pull(graph, outer)).toBe(4);

    writeSource(graph, count, 2);
    expect(pull(graph, outer)).toBe(8);
    expect(calls).toEqual({ inner: 2, outer: 2 });
  });

  // crates/pico/tests/absent_source/missing_key.rs:164
  test('a computation that reads one source twice', () => {
    const graph = createGraph();
    const calls = { twice: 0 };
    const config = createSourceNode('a');
    const twice = createDerivedNode((context) => {
      calls.twice += 1;
      return `${context.readSource(config)}${context.readSource(config)}`;
    }, sameValue);
    expect(pull(graph, twice)).toBe('aa');

    writeSource(graph, config, 'b');
    expect(pull(graph, twice)).toBe('bb');
    writeSource(graph, config, 'c');
    expect(pull(graph, twice)).toBe('cc');
    expect(calls.twice).toBe(3);
  });

  // crates/pico/tests/absent_source/missing_key.rs:57
  test('a source that holds the consumer’s absence value', () => {
    const graph = createGraph();
    const calls = { name: 0 };
    const config = createSourceNode<string | typeof MISSING>(MISSING);
    const name = createDerivedNode((context) => {
      calls.name += 1;
      const value = context.readSource(config);
      return value === MISSING ? null : value;
    }, sameValue);
    expect(pull(graph, name)).toBe(null);

    writeSource(graph, config, 'a');
    expect(pull(graph, name)).toBe('a');
    expect(calls.name).toBe(2);
  });

  // crates/pico/tests/absent_source/missing_key.rs:212
  test('sourceValue outside a computation, then a reader created after the write', () => {
    const graph = createGraph();
    const calls = { name: 0 };
    const config = createSourceNode<string | typeof MISSING>(MISSING);
    expect(sourceValue(config)).toBe(MISSING);

    writeSource(graph, config, 'a');
    const name = createDerivedNode((context) => {
      calls.name += 1;
      const value = context.readSource(config);
      return value === MISSING ? null : value;
    }, sameValue);
    expect(pull(graph, name)).toBe('a');
    expect(calls.name).toBe(1);
    expect(sourceValue(config)).toBe('a');
  });

  // crates/pico/tests/absent_source/missing_key.rs:233
  test('sourceValue outside a computation leaves the readers’ dependency', () => {
    // config and name as in the previous test
    expect(pull(graph, name)).toBe(null);
    expect(sourceValue(config)).toBe(MISSING);

    writeSource(graph, config, 'a');
    expect(pull(graph, name)).toBe('a');
    expect(calls.name).toBe(2);
  });

  // crates/pico/tests/tracking_field/correctness.rs:17
  test('sourceValue inside a computation records nothing', () => {
    const graph = createGraph();
    const items = createSourceNode<ReadonlyArray<string>>(['asdf']);
    const untrackedCount = createDerivedNode(() => sourceValue(items).length, sameValue);
    const trackedCount = createDerivedNode(
      (context) => context.readSource(items).length,
      sameValue,
    );
    expect(pull(graph, untrackedCount)).toBe(1);

    writeSource(graph, items, ['asdf', 'isograph']);
    // open decision 10 (a). Under (b) the first pull of untrackedCount throws.
    expect(pull(graph, untrackedCount)).toBe(1);
    expect(pull(graph, trackedCount)).toBe(2);

    writeSource(graph, items, ['asdf', 'isograph', 'database']);
    expect(pull(graph, trackedCount)).toBe(3);
  });
});
```

### `errors.test.ts`

A computation must not throw (Throws). The one test asserts the non-corruption guarantee, observed through values and call counts after the throw; it asserts nothing about the node that threw.

```ts
// from libs/reactive-graph/test/errors.test.ts
import { expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  pull,
  sameValue,
  writeSource,
} from '../src';

test('a throwing computation leaves the graph state unchanged', () => {
  const graph = createGraph();
  const calls = { scaled: 0, parent: 0 };
  const count = createSourceNode(1);
  const scaled = createDerivedNode((context) => {
    calls.scaled += 1;
    const value = context.readSource(count);
    if (value < 0) {
      throw new Error('negative');
    }
    return value * 10;
  }, sameValue);
  const parent = createDerivedNode((context) => {
    calls.parent += 1;
    return `${context.readDerived(scaled)}`;
  }, sameValue);
  expect(pull(graph, parent)).toBe('10');

  writeSource(graph, count, -1);
  expect(() => pull(graph, parent)).toThrow('negative');

  writeSource(graph, count, 2);
  expect(pull(graph, parent)).toBe('20');
  expect(calls).toEqual({ scaled: 3, parent: 2 });

  writeSource(graph, count, 3);
  expect(pull(graph, parent)).toBe('30');
  expect(calls).toEqual({ scaled: 4, parent: 3 });
});
```

The write after the throw succeeds, which asserts that `computingDepth` was restored. The `3` write recomputes both nodes, which asserts that the throwing run left `scaled` in the `dependents` set of `count` and `parent` in that of `scaled`.

### `disposal.test.ts`

```ts
// from libs/reactive-graph/test/disposal.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  disposeDerivedNode,
  pull,
  sameValue,
  writeSource,
} from '../src';

describe('disposeDerivedNode (open decision 9 (a))', () => {
  test('marks the dependents Dirty, and their next pull recomputes the disposed node', () => {
    const graph = createGraph();
    const calls = { scaled: 0, parent: 0 };
    const count = createSourceNode(1);
    const scaled = createDerivedNode((context) => {
      calls.scaled += 1;
      return context.readSource(count) * 10;
    }, sameValue);
    const parent = createDerivedNode((context) => {
      calls.parent += 1;
      return `${context.readDerived(scaled)}`;
    }, sameValue);
    expect(pull(graph, parent)).toBe('10');

    disposeDerivedNode(graph, scaled);
    expect(calls).toEqual({ scaled: 1, parent: 1 });
    expect(pull(graph, parent)).toBe('10');
    expect(calls).toEqual({ scaled: 2, parent: 2 });
  });

  // crates/pico/tests/absent_source/garbage_collection.rs:46
  test('a disposed node recomputes on its next pull and sees writes made after disposal', () => {
    // count and scaled as above
    expect(pull(graph, scaled)).toBe(10);

    disposeDerivedNode(graph, scaled);
    writeSource(graph, count, 2);
    expect(pull(graph, scaled)).toBe(20);
    expect(calls.scaled).toBe(2);
  });

  test('a dependent that stops reading a disposed node is detached from it', () => {
    const graph = createGraph();
    const calls = { scaled: 0, parent: 0 };
    const count = createSourceNode(1);
    const useScaled = createSourceNode(true);
    // scaled as above
    const parent = createDerivedNode((context) => {
      calls.parent += 1;
      return context.readSource(useScaled) ? context.readDerived(scaled) : 0;
    }, sameValue);
    expect(pull(graph, parent)).toBe(10);

    disposeDerivedNode(graph, scaled);
    writeSource(graph, useScaled, false);
    expect(pull(graph, parent)).toBe(0);
    expect(calls).toEqual({ scaled: 1, parent: 2 });

    expect(pull(graph, scaled)).toBe(10);
    writeSource(graph, count, 2);
    expect(pull(graph, scaled)).toBe(20);
    expect(pull(graph, parent)).toBe(0);
    expect(calls).toEqual({ scaled: 3, parent: 2 });
  });

  test('disposing a never-computed node, or a node twice, changes nothing else', () => {
    const graph = createGraph();
    const calls = { node: 0 };
    const count = createSourceNode(1);
    const node = createDerivedNode((context) => {
      calls.node += 1;
      return context.readSource(count);
    }, sameValue);

    disposeDerivedNode(graph, node);
    disposeDerivedNode(graph, node);
    expect(calls.node).toBe(0);
    expect(pull(graph, node)).toBe(1);
    expect(calls.node).toBe(1);
  });
});
```

### `guard.test.ts`

Written for open decision 2 (f). Under (g) the calls drop the `graph` argument and the constructors take it. Under (a) the file does not exist.

```ts
// from libs/reactive-graph/test/guard.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  disposeDerivedNode,
  type Graph,
  markSourceChanged,
  pull,
  sameValue,
  sourceValue,
  writeSource,
} from '../src';

const guardMessage = 'Cannot write or pull the graph while a computation is running.';

describe('the write-during-computation guard', () => {
  test.each([
    ['writeSource', (graph: Graph) => writeSource(graph, createSourceNode(0), 1)],
    ['markSourceChanged', (graph: Graph) => markSourceChanged(graph, createSourceNode(0))],
    [
      'disposeDerivedNode',
      (graph: Graph) => disposeDerivedNode(graph, createDerivedNode(() => 0, sameValue)),
    ],
    ['pull', (graph: Graph) => pull(graph, createDerivedNode(() => 0, sameValue))],
  ])('%s inside a computation throws', (_name, action) => {
    const graph = createGraph();
    const node = createDerivedNode(() => {
      action(graph);
      return 0;
    }, sameValue);
    expect(() => pull(graph, node)).toThrow(guardMessage);
  });

  test('a rejected write changes nothing, and the graph accepts writes after the throw', () => {
    const graph = createGraph();
    const other = createSourceNode(0);
    const writer = createDerivedNode(() => {
      writeSource(graph, other, 1);
      return 0;
    }, sameValue);
    expect(() => pull(graph, writer)).toThrow(guardMessage);
    expect(sourceValue(other)).toBe(0);

    writeSource(graph, other, 2);
    expect(sourceValue(other)).toBe(2);
  });
});
```

### `cycles.test.ts`

Written for open decision 4 (a), no check: each pull throws a `RangeError` when the stack overflows. Under (b) each `toThrow()` becomes `toThrow('Cyclic dependency detected.')`. Every test ends by writing a source, which asserts that the guard was released.

```ts
// from libs/reactive-graph/test/cycles.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  type DerivedNode,
  pull,
  sameValue,
  writeSource,
} from '../src';

describe('cycles', () => {
  // crates/pico/tests/cyclic_dependency_panic.rs:11
  test('a node that reads itself throws', () => {
    const graph = createGraph();
    const callsItself: DerivedNode<number> = createDerivedNode(
      (context) => context.readDerived(callsItself),
      sameValue,
    );
    expect(() => pull(graph, callsItself)).toThrow();
    writeSource(graph, createSourceNode(0), 1);
  });

  // crates/pico/tests/cyclic_dependency_panic.rs:20
  test('a node that reads a node that reads itself throws', () => {
    // callsItself as above
    const callsCallsItself = createDerivedNode(
      (context) => context.readDerived(callsItself),
      sameValue,
    );
    expect(() => pull(graph, callsCallsItself)).toThrow();
    writeSource(graph, createSourceNode(0), 1);
  });

  // crates/pico/tests/cyclic_dependency_panic.rs:29
  test('two nodes that read each other throw', () => {
    const graph = createGraph();
    const a: DerivedNode<number> = createDerivedNode(
      (context) => context.readDerived(b),
      sameValue,
    );
    const b: DerivedNode<number> = createDerivedNode(
      (context) => context.readDerived(a),
      sameValue,
    );
    expect(() => pull(graph, a)).toThrow();
    writeSource(graph, createSourceNode(0), 1);
  });

  // crates/pico/tests/cyclic_dependency_panic.rs:38
  test('a node that reads into a cycle throws', () => {
    // a and b as above
    const callsA = createDerivedNode((context) => context.readDerived(a), sameValue);
    expect(() => pull(graph, callsA)).toThrow();
    writeSource(graph, createSourceNode(0), 1);
  });

  test('a cycle formed by a dependency change throws on verification', () => {
    const graph = createGraph();
    const closeLoop = createSourceNode(false);
    const b: DerivedNode<number> = createDerivedNode(
      (context) => (context.readSource(closeLoop) ? context.readDerived(a) : 1),
      sameValue,
    );
    const a: DerivedNode<number> = createDerivedNode(
      (context) => context.readDerived(b) + 1,
      sameValue,
    );
    expect(pull(graph, a)).toBe(2);

    writeSource(graph, closeLoop, true);
    expect(() => pull(graph, a)).toThrow();
    writeSource(graph, closeLoop, false);
    expect(pull(graph, a)).toBe(2);
  });
});
```

The last test reaches the cycle through `verify`: `a` is `Check`, its verification recomputes `b`, and `b` reads `a`. A check under open decision 4 (b) must cover a node that is verifying, not only one that is recomputing. Its last assertion holds under both options: after the throw, `a` and `b` are still marked, and the write that opens the loop lets them compute.

### `degenerate.test.ts`

```ts
// from libs/reactive-graph/test/degenerate.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  disposeDerivedNode,
  markSourceChanged,
  pull,
  sameValue,
  sourceValue,
  writeSource,
} from '../src';

describe('degenerate cases', () => {
  test('a derived node with no dependencies is computed once, never marked, and changed only by disposal', () => {
    const graph = createGraph();
    const calls = { constant: 0, parity: 0, reader: 0 };
    const count = createSourceNode(1);
    const constant = createDerivedNode(() => {
      calls.constant += 1;
      return 'fixed';
    }, sameValue);
    const parity = createDerivedNode((context) => {
      calls.parity += 1;
      return context.readSource(count) % 2 === 0 ? 'even' : 'odd';
    }, sameValue);
    const reader = createDerivedNode((context) => {
      calls.reader += 1;
      return `${context.readDerived(constant)} ${context.readDerived(parity)}`;
    }, sameValue);
    expect(pull(graph, reader)).toBe('fixed odd');

    writeSource(graph, count, 3);
    expect(pull(graph, reader)).toBe('fixed odd');
    expect(calls).toEqual({ constant: 1, parity: 2, reader: 1 });

    writeSource(graph, count, 4);
    expect(pull(graph, reader)).toBe('fixed even');
    expect(calls).toEqual({ constant: 1, parity: 3, reader: 2 });

    disposeDerivedNode(graph, constant);
    expect(pull(graph, reader)).toBe('fixed even');
    expect(calls).toEqual({ constant: 2, parity: 3, reader: 3 });
  });

  test('a source with no dependents', () => {
    const graph = createGraph();
    const count = createSourceNode(1);
    writeSource(graph, count, 2);
    markSourceChanged(graph, count);
    expect(sourceValue(count)).toBe(2);
  });

  test('a pull of a never-computed node computes it once', () => {
    const graph = createGraph();
    const calls = { node: 0 };
    const count = createSourceNode(1);
    const node = createDerivedNode((context) => {
      calls.node += 1;
      return context.readSource(count);
    }, sameValue);
    expect(calls.node).toBe(0);
    expect(pull(graph, node)).toBe(1);
    expect(pull(graph, node)).toBe(1);
    expect(calls.node).toBe(1);
  });

  test('writing the value a source already holds (open decision 8 (a))', () => {
    const graph = createGraph();
    const calls = { reader: 0, parent: 0 };
    const items = createSourceNode<ReadonlyArray<number>>([1, 2]);
    const reader = createDerivedNode((context) => {
      calls.reader += 1;
      return context.readSource(items).length;
    }, sameValue);
    const parent = createDerivedNode((context) => {
      calls.parent += 1;
      return context.readDerived(reader) * 2;
    }, sameValue);
    expect(pull(graph, parent)).toBe(4);

    writeSource(graph, items, sourceValue(items));
    expect(pull(graph, parent)).toBe(4);
    // Under (b): { reader: 1, parent: 1 }.
    expect(calls).toEqual({ reader: 2, parent: 1 });
  });

  test('markSourceChanged after the value object changed in place', () => {
    const graph = createGraph();
    const scores = createSourceNode(new Map([['a', 1]]));
    const score = createDerivedNode(
      (context) => context.readSource(scores).get('a'),
      sameValue,
    );
    expect(pull(graph, score)).toBe(1);

    sourceValue(scores).set('a', 2);
    expect(pull(graph, score)).toBe(1);
    markSourceChanged(graph, scores);
    expect(pull(graph, score)).toBe(2);
  });

  test('undefined is a value, distinct from never computed', () => {
    const graph = createGraph();
    const calls = { nothing: 0, parent: 0 };
    const count = createSourceNode(1);
    const nothing = createDerivedNode((context) => {
      calls.nothing += 1;
      context.readSource(count);
      return undefined;
    }, sameValue);
    const parent = createDerivedNode((context) => {
      calls.parent += 1;
      return context.readDerived(nothing) === undefined;
    }, sameValue);
    expect(pull(graph, parent)).toBe(true);
    expect(pull(graph, nothing)).toBe(undefined);

    writeSource(graph, count, 2);
    expect(pull(graph, parent)).toBe(true);
    expect(calls).toEqual({ nothing: 2, parent: 1 });
  });

  test('NaN under sameValue', () => {
    const graph = createGraph();
    const calls = { notANumber: 0, parent: 0 };
    const count = createSourceNode(1);
    const notANumber = createDerivedNode((context) => {
      calls.notANumber += 1;
      return context.readSource(count) * Number.NaN;
    }, sameValue);
    const parent = createDerivedNode((context) => {
      calls.parent += 1;
      return Number.isNaN(context.readDerived(notANumber));
    }, sameValue);
    expect(pull(graph, parent)).toBe(true);

    writeSource(graph, count, 2);
    expect(pull(graph, parent)).toBe(true);
    // open decision 6 (b), Object.is. Under (a), ===: { notANumber: 2, parent: 2 }.
    expect(calls).toEqual({ notANumber: 2, parent: 1 });
  });

  test('0 and -0 under sameValue', () => {
    const graph = createGraph();
    const sign = createSourceNode(1);
    const zero = createDerivedNode((context) => context.readSource(sign) * 0, sameValue);
    const parent = createDerivedNode(
      (context) => (Object.is(context.readDerived(zero), -0) ? 'negative zero' : 'zero'),
      sameValue,
    );
    expect(pull(graph, parent)).toBe('zero');

    writeSource(graph, sign, -1);
    // open decision 6 (b), Object.is. Under (a), === keeps 0 and parent stays 'zero'.
    expect(pull(graph, parent)).toBe('negative zero');
  });
});
```

### `graph.test-d.ts`

```ts
// from libs/reactive-graph/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import {
  createDerivedNode,
  createGraph,
  createSourceNode,
  type DerivedNode,
  pull,
  sameValue,
  writeSource,
} from '../src';
import { type AnyDerivedNode } from '../src/graph';

test('readDerived and pull return the node’s value type', () => {
  const graph = createGraph();
  const count = createSourceNode(1);
  const doubled = createDerivedNode((context) => context.readSource(count) * 2, sameValue);
  const label = createDerivedNode((context) => {
    expectTypeOf(context.readDerived(doubled)).toEqualTypeOf<number>();
    return `${context.readDerived(doubled)}`;
  }, sameValue);
  expectTypeOf(pull(graph, label)).toEqualTypeOf<string>();
});

test('a derived node of any value type is an AnyDerivedNode', () => {
  expectTypeOf<DerivedNode<number>>().toMatchTypeOf<AnyDerivedNode>();
  expectTypeOf<DerivedNode<ReadonlyArray<string>>>().toMatchTypeOf<AnyDerivedNode>();
});

test('writes and equalities are typed by the node', () => {
  const graph = createGraph();
  const count = createSourceNode(1);
  // @ts-expect-error a string written to a number source
  writeSource(graph, count, 'one');
  createDerivedNode(
    () => 1,
    // @ts-expect-error an equality over strings for a number node
    (previous: string, next: string) => previous === next,
  );
});
```

## The package

### Location and files

The engine is a workspace library at `libs/reactive-graph`, covered by `pnpm-workspace.yaml`'s `libs/*`. Its build and test setup copies `libs/isograph-runtime`'s without the Babel plugin, which only the runtime's fixtures need.

```text
libs/reactive-graph/
  package.json
  README.md                 The package README
  tsconfig.json             references tsconfig.package.json and tsconfig.test.json
  tsconfig.package.json     extends ../../tsconfig.build.json, includes src/**/*.ts
  tsconfig.test.json        extends ../../tsconfig.build.json, includes test/**/*.ts
  tsdown.config.ts          merges ../../tsdown.config.ts, dts.build true
  vitest.config.ts          typecheck enabled with tsconfig.test.json
  src/graph.ts
  src/index.ts
  test/*.test.ts, test/graph.test-d.ts   Tests
```

Adding the package changes `pnpm-lock.yaml` (a new importer, written by `pnpm install`), which the same commit includes.

### `package.json`

```json
// from libs/reactive-graph/package.json
{
  "name": "@isograph/reactive-graph",
  "description": "A graph of source nodes and derived nodes, recomputed on read when an input changed.",
  "private": true,
  "main": "./dist/index.cjs",
  "types": "./dist/index.d.cts",
  "license": "MIT",
  "scripts": {
    "compile-libs": "tsdown",
    "watch-libs": "tsdown --watch",
    "test": "vitest run",
    "test-watch": "vitest watch",
    "coverage": "vitest run --coverage",
    "prepack": "pnpm run test && pnpm run compile-libs",
    "tsc": "tsc -b",
    "tsc-force": "tsc --build --clean && tsc --build --force"
  },
  "dependencies": {},
  "devDependencies": {
    "typescript": "5.6.3",
    "vitest": "^2.1.2"
  },
  "repository": {
    "type": "git",
    "url": "git+https://github.com/isographlabs/isograph.git",
    "directory": "libs/reactive-graph"
  },
  "sideEffects": false,
  "module": "./dist/index.mjs",
  "exports": {
    ".": {
      "require": "./dist/index.cjs",
      "import": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  }
}
```

Deltas from `libs/isograph-runtime/package.json`: `name`, `description`, and `repository.directory`; `dependencies` is empty; `devDependencies` drops `@babel/core` and `@isograph/babel-plugin`. `private: true` stays, as the runtime's and `@isograph/disposable-types`'s do: publishing to npm is distribution, which `IN_PROGRESS.md` lists as not planned. The one export is `.`, built from `src/index.ts` by the root `tsdown.config.ts` (`entry: ['src/index.ts']`, `format: ['commonjs', 'esm']`).

### TypeScript, tsdown, vitest

`tsconfig.json`, `tsconfig.package.json`, and `tsdown.config.ts` are the runtime's, byte for byte (`libs/isograph-runtime/tsconfig.json`, `tsconfig.package.json`, `tsdown.config.ts`). The engine compiles under the root `tsconfig.build.json`: `target: ES2022`, `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `noUnusedLocals`, `noUnusedParameters`.

```json
// from libs/reactive-graph/tsconfig.test.json
{
  "extends": "../../tsconfig.build.json",
  "compilerOptions": {
    "noUnusedLocals": false
  },
  "include": ["test/**/*.ts"],
  "exclude": ["**/node_modules", "**/dist"]
}
```

```ts
// from libs/reactive-graph/vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { typecheck: { enabled: true, tsconfig: './tsconfig.test.json' } },
});
```

Deltas of `tsconfig.test.json` from the runtime's: no `paths` entry and no `fixtures/**/*.ts`, because the tests import `../src` and there are no fixtures. Deltas of `vitest.config.ts`: no Babel plugin. With `typecheck.enabled`, vitest type-checks `graph.test-d.ts` as the runtime's `read.test-d.ts` is.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`; every type field is `readonly` except `SourceNode.value`, `StateSlot.state`, and `Graph.computingDepth`, which the engine writes; arrays are `ReadonlyArray`. The engine has no class, so the ES private field rule applies only under open decision 7 (b). oxlint's `switch-exhaustiveness-check` and `strict-boolean-expressions` (`.oxlintrc.json`) hold for every `switch` and condition in the body.

### Pre-commit hooks

`.pre-commit-config.yaml` gains two hooks, after `disposable-types-typecheck`:

```yaml
# from .pre-commit-config.yaml
      - id: engine-test
        name: engine test
        entry: pnpm --filter @isograph/reactive-graph test
        language: system
        files: '^libs/reactive-graph/'
        pass_filenames: false
      - id: engine-typecheck
        name: engine typecheck
        entry: pnpm --filter @isograph/reactive-graph tsc
        language: system
        files: '^(libs/reactive-graph/|tsconfig\.build\.json$)'
        pass_filenames: false
```

`prettier` and `oxlint` already cover every file in the workspace. When the Isograph runtime gains its dependency on the engine (pico-runtime.md core change 4), the runtime's three hooks also match the engine's directory:

```yaml
# from .pre-commit-config.yaml
      # runtime-dependencies-build and runtime-typecheck, files:
        files: '^(libs/isograph-runtime/|libs/isograph-disposable-types/|libs/reactive-graph/|tsconfig\.build\.json$)'
      # runtime-test, files:
        files: '^libs/(isograph-runtime|isograph-disposable-types|reactive-graph)/'
```

### CI

`.github/workflows/js.yml`'s `babel-plugin` job runs `pnpm compile-libs`, which turbo runs for every workspace package with a `compile-libs` script; turbo's `compile-libs` task depends on `tsc` (`turbo.json`), so the engine is type-checked and built there with no change. The job gains one step after "Test @isograph/disposable-types":

```yaml
# from .github/workflows/js.yml
      - name: Test @isograph/reactive-graph
        run: pnpm --filter @isograph/reactive-graph test
```

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal). pico-runtime.md core change 3 ships the package before its first workspace caller on that ground; core change 4 is the first caller of the source functions and core change 5 of the derived-node functions. Open decision 11 restates whether the package ships alone or merges into its first use.

## What this doc changes in pico-runtime.md

pico-runtime.md is not edited by this doc. These are the differences to apply there once the decisions settle:

- The engine's public functions take a `Graph` first under open decision 2 (f), and the index exports `createGraph` and `Graph`. pico-runtime.md's The engine's public API, its use without Isograph, and its Which layer owns each item write the signatures without a graph. Which layer owns each item gains `Graph`, `createGraph`, `assertNotComputing`, `pullNode`, `neverComputed`, and `dependentsOf`.
- `disposeDerivedNode` resets the node to never computed (Disposal, open decision 9). pico-runtime.md's body leaves the state, which lets a later pull return a value no source explains.
- `compute` counts running computations in a `finally`, under open decision 2 (f).
- The tests are the files of Tests instead of one `test/graph.test.ts`, and the package has a `README.md`.
- The CI step of CI.
- A computation and an equality must not throw, and the engine guarantees only that its own state is not corrupted (decision 3). pico-runtime.md open decision 3's engine-side options go.
- New open decisions 8, 9, 10, and 11 below.

## Open decisions

### 1. The package's name (decided)

The package is `@isograph/reactive-graph` and its directory is `libs/reactive-graph` (pico-runtime.md open decision 18). The directory does not follow the `libs/isograph-<name>` pattern of the other library directories. Whether the name is free on the npm registry was not verified, because the registry was not reachable offline.

### 2. A write during a computation

pico-runtime.md open decision 2, restated in engine terms. A computation that reaches a source or a node through a closure can write, dispose, or pull at the top level while it computes (The design, Writes during a computation). pico rejects every write while a memo runs (`assert_empty_dependency_stack`, `crates/pico/src/database.rs:193-198`, called from `set`, `remove`, `remove_singleton`, and `run_garbage_collection`, `:151-170`). Reactively and Preact allow it.

- (a) No guard. A write inside a computation is undefined behavior. Cost: the stale `Clean` of the trace, with no error.
- (d) A write counter on an engine object, read by `compute` before and after the computation; when it moved, `recompute` leaves the node `Dirty`. Cost: the engine object and a counter every write increments; it repairs the computing node only, and the write still ran.
- (e) After `setDependencies`, `recompute` leaves the node `Dirty` when a recorded derived dependency is not `Clean`. Cost: one check per dependency per recompute; a source the computation read before the write changed it is not covered.
- (f) `Graph` holds `computingDepth`; `writeSource`, `markSourceChanged`, `disposeDerivedNode`, and `pull` take it first and throw while it is positive; `readDerived` and `verify` call the unguarded `pullNode`. The body of this doc. Cost: every public call but the constructors and `sourceValue` takes a `Graph`; a node passed to another graph's functions is not detected.
- (g) As (f), but each node holds its graph, set by the constructors (Option (g)). Cost: one field per node; only the constructors change signature; a cross-graph read is detectable and throws.

pico-runtime.md's (b) and (c) are guards on the Isograph environment. A closure that holds the store reaches the engine's functions without passing them, so they are not engine options. Not held. The API is written under (f) (The public API).

### 3. A throw out of a computation or an equality (decided)

A computation and an equality must not throw; a throw is a programmer error (Throws). The engine guarantees only that its own state is not corrupted, which holds because the throw leaves before any state is written (pico-runtime.md open decision 3).

### 4. A cycle of derived nodes

pico-runtime.md open decision 8, restated. A cycle check belongs to the engine, because `pullNode` is where a node is re-entered, through `recompute` or `verify` (`cycles.test.ts`, the last test). pico panics with "Cyclic dependency detected. This is not supported in pico." when a memo on the dependency stack is entered again (`assert_no_cycles`, `crates/pico/src/dependency.rs:95-99`). Preact throws "Cycle detected". Reactively does not check.

- (a) No check. The recursion overflows the stack and a `RangeError` propagates (Throws). Cost: the error names no node, and a computation that catches errors from `readDerived` catches it at some depth and ends `Clean` there.
- (b) The graph holds the nodes being verified or recomputed, and `pullNode` throws on re-entry:

  ```ts
  // from libs/reactive-graph/src/graph.ts
  // under open decision 4 (b); deltas from the body
  export type Graph = {
    computingDepth: number;
    /** The derived nodes being verified or recomputed. Written only by `pullNode`. */
    readonly entered: Set<AnyDerivedNode>;
  };

  function pullNode<TValue>(graph: Graph, node: DerivedNode<TValue>): TValue {
    const state = stateOf(node).state;
    if (state.mark === 'Clean') {
      return state.value;
    }
    if (graph.entered.has(node)) {
      throw new Error('Cyclic dependency detected.');
    }
    graph.entered.add(node);
    try {
      switch (state.mark) {
        case 'Check':
          return verify(graph, node, state);
        case 'Dirty':
          return recompute(graph, node, state);
      }
    } finally {
      graph.entered.delete(node);
    }
  }
  ```

  pico's behavior. Cost: a set insertion and removal per verification and recomputation; the set needs the `Graph` of open decision 2 (f) or (g).
- (c) A fourth state, `Computing`, written during `verify` and `recompute` and restored afterwards. Cost: every `switch` on the mark gains a case, the state before entry must be restored on a throw, and a layered `stateOf` must hold it per layer.

Not held in pico-runtime.md, because the Isograph core cannot form a cycle until eager client pointers. By AGENTS.md's rule the candidate is (b), pico's behavior.

### 5. How the engine types the links between nodes of different value types

pico-runtime.md open decision 19, restated.

- (a) `AnyDerivedNode = DerivedNode<unknown>`, with `equal` a method signature so that `DerivedNode<T>` is assignable to it. Held, not decided. Reason: no cast and no wrapper; the engine calls `equal` only with two values the node's own computation produced. Cost: method bivariance also accepts a call such as `(node as AnyDerivedNode).equal('a', 'b')` on a `DerivedNode<number>`; only code outside the engine could make it.
- (b) One value type per graph. Cost: a consumer whose nodes have different value types declares a union and narrows every `readDerived` result.
- (c) An existential encoding: a link holds `<R>(visit: <T>(node: DerivedNode<T>) => R) => R`. Cost: one closure per link and a call per traversal step; it is sound.

### 6. The engine's identity equality

pico-runtime.md open decision 20, restated. An equality may answer equal only when no reader can tell the values apart (The design, Cutoff). `===` answers equal for `0` and `-0`, which a reader can tell apart, and unequal for `NaN` and `NaN`, which costs recomputation only (`degenerate.test.ts`, the last two tests).

- (a) `===`: upstream Isograph's comparison of resolver values (`areEqualWithDeepComparison.ts:84-95` in `../isograph`), Reactively's `defaultEquality`, and Preact's `!==`. The body is written for (a). Cost: a node whose value moves from `0` to `-0` cuts off and its readers keep `0`.
- (b) `Object.is`. Held, not decided. Reason: it is the comparison the correctness rule admits. Cost: a deviation from upstream Isograph, Reactively, and Preact; a node that returns `-0` and then `0` marks its readers.
- (c) No default: every `createDerivedNode` caller writes its own. Cost: every caller restates `===` or `Object.is`.

### 7. Whether a consumer can write a node's fields

pico-runtime.md open decision 21, restated; it covers `Graph` as well. The types are plain object types whose fields every consumer sees, and a consumer that writes `dependents`, `slot`, or `computingDepth` breaks an invariant silently.

- (a) Plain object types; each field's comment names its only reader and writer. Held, not decided. Reason: (b) and (c) each cost a cast or a slower link type. Cost: the comments are the only guard, against AGENTS.md's "Never rely on discipline what we can enforce with newtypes".
- (b) Classes with ES private fields, with module-local accessors assigned in each class's `static {}` block. Cost: a private `#equal` makes `DerivedNode<number>` not assignable to `DerivedNode<unknown>`, so it needs open decision 5 (c) or a cast per link; a node cannot be wrapped in a `Proxy`.
- (c) Opaque branded public types, cast to field types inside `graph.ts`. Cost: a cast in every engine function that receives a node or a graph.

### 8. Whether `writeSource` compares the new value with the old

New. pico's `set` does not advance the epoch when the new value `==` the stored one (`set_source`, `crates/pico/src/database.rs:310-329`); Reactively's `set` and Preact's setter skip an equal value too.

- (a) No comparison: `writeSource` always marks, and the caller decides that the value changed. The body of this doc and of pico-runtime.md. Held. Reason: the Isograph consumer compares before it writes, with knowledge the engine does not have (`normalize`'s `isUnchanged`, pico-runtime.md core change 1), so (b) would make it compare twice or pass a comparator that always answers unequal; a standalone consumer compares at the call. Cost: a deviation from pico, Reactively, and Preact; a consumer that writes an equal value runs the readers of the source again, which is work and never a wrong value, because each reader's own equality still cuts off above it (`overriding_same_source`'s port, `degenerate.test.ts`).
- (b) A source carries an equality: `createSourceNode(value, equal)`, and `writeSource` sets the value and marks only when `equal(previous, value)` rejects. Cost: one more argument on every source, and the Isograph consumer's double comparison above.

### 9. A pull of a disposed node

New. pico-runtime.md's body leaves a disposed node's state as it was, so a `Clean` node pulled after disposal returns its old value although no source lists it; that is a defect, not an option. pico removes a collected memo's revision, and the next call of the memo computes it again (`crates/pico/tests/absent_source/garbage_collection.rs:46`).

- (a) Disposal resets the node to never computed; a later pull computes it from scratch and reattaches its edges. The body of this doc. Held. Reason: pico's behavior. Cost: a consumer that pulls a node it disposed gets a working node again instead of an error.
- (b) A fourth state, `Disposed`; `pullNode` throws on it, and so does a dependent's recomputation that reads it. Cost: every `switch` on the mark gains a case, and a dependent marked `Dirty` by the disposal throws on its next pull when its computation still reads the node.

### 10. An untracked read inside a computation

New. `sourceValue` records nothing. Called inside a computation, it reads a value the node will not be marked for (`dependencies.test.ts`, "sourceValue inside a computation records nothing"). pico allows the same hole: `MemoRef::lookup` reads without recording and `lookup_tracked` records (`crates/pico/src/memo_ref.rs:57`, `:80`), and `#[tracked]` maps have `untracked()`.

- (a) Allowed. Held. Reason: pico's behavior; a deliberate untracked read inside a computation has uses (reading a value that only affects logging). Cost: a mistaken untracked read is a silently stale value.
- (b) `sourceValue` takes the graph and throws while `computingDepth > 0`. Cost: under open decision 2 (f), every read outside a computation passes a graph; a deliberate untracked read is impossible.

Recording the read is not an option: `sourceValue` has no context, and finding the running computation from it needs the ambient tracking the engine does not have.

### 11. Whether the package ships before its first caller

New in this doc; pico-runtime.md core change 3 holds (a).

- (a) Its own commit (pico-runtime.md core change 3), with its tests and no Isograph caller. Held. Reason: the user's goal that the engine is usable outside Isograph is its reader (Readers before the Isograph runtime depends on it). Cost: the package exists for two commits with only tests reading it, against AGENTS.md's "nothing we do not use".
- (b) Merged into core change 4, where `store.ts` first calls `createSourceNode`, `sourceValue`, `writeSource`, and `markSourceChanged`. Cost: a larger commit, and `createDerivedNode`, `pull`, `disposeDerivedNode`, and `sameValue` still have no Isograph caller until core change 5.
- (c) Merged into core change 5, the first commit in which every export has an Isograph caller. Cost: change 4 then needs the engine's source functions before the package exists, so changes 4 and 5 merge too.
