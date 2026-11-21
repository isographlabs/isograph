# Reactive engine: source nodes and derived nodes as a standalone package

Status: under discussion. Not approved for implementation. This doc is the complete specification of the engine that `refactors/pending/runtime/pico-runtime.md` (The graph, Mark, Pull, The engine and the Isograph runtime, The engine package, What resists the split) places in its own package. It restates those sections in engine terms, adds what they leave out (the full API under one guard option, disposal of a pulled node, cycles, throws, the README, the design description, and the test inventory against the Rust pico crate), and lists the open decisions. Where this doc and pico-runtime.md differ, What this doc changes in pico-runtime.md names each difference.

pico paths are under `crates/pico/src` and `crates/pico/tests` of this repo. Reactively is `packages/core/src/core.ts` of `milomg/reactively`; Preact signals is `packages/core/src/index.ts` of `preactjs/signals` (both read at their `main` branches; no line numbers, because neither checkout is local).

The package's name and directory are open decision 1 (pico-runtime.md open decision 18). Snippets write them as `<engine package>` and `libs/<engine directory>`.

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

The API is written under open decision 2 option (f): an engine `Graph` object holds the write-during-computation guard, and every public function that reads or writes the graph takes it as its first parameter. Decision 2 is not decided. (f) is written because pico-runtime.md establishes that a guard no caller can bypass lives in the engine (its open decision 2, last paragraph), which leaves (f) and (g), and (f) has pico's shape: every pico read and write takes the database (`Database::set`, `crates/pico/src/database.rs:151`; a memo's first argument is `&Db`). Under (g) Option (g) lists the deltas; under (a) every `graph` parameter and `Graph` itself drop out.

```ts
// from libs/<engine directory>/src/index.ts
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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
 * computation runs: a computation reads other nodes through its context. Propagates a throw out of a
 * computation or an equality it runs (open decision 3). */
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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
 * A new value marks the dependents `Dirty`. A throw out of the computation or out of `equal` leaves
 * the state and every `dependents` set untouched. */
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
// from libs/<engine directory>/src/graph.ts
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
// from libs/<engine directory>/src/graph.ts
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

A throw out of a computation propagates out of `compute`, and a throw out of `equal` propagates out of `recompute`, before `setDependencies` and before any state is written. The node keeps its mark, its previous value, its dependencies, and its place in the `dependents` sets of those dependencies. The next pull recomputes it again. A verifying ancestor keeps its `Check` mark, because the throw leaves `verify` before it writes, and its next pull verifies again. Nodes pulled inside the failed computation before the throw are `Clean` and correct, because each pull completed.

This is open decision 3 option (a) in engine terms, and is what both pico-runtime.md open decision 3 (a) (the Isograph runtime catches the walk's throw inside its computation and returns it as a value) and (c) (propagate) need from the engine. Open decision 3 lists the engine options that change it.

### Cycles

The body has no cycle check. A computation that reads its own node, directly or through other nodes, makes `pullNode` re-enter a node that is recomputing or verifying. With no check, the re-entry recurses until the JavaScript stack overflows and a `RangeError` propagates as any throw does. Marking terminates on a cycle, because it stops at a marked node. Open decision 4 lists the options.

## The package README

`libs/<engine directory>/README.md`, as `libs/isograph-disposable-types/README.md` is for its package. Its text:

````markdown
# <engine package>

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
} from '<engine package>';

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
- A throw out of a computation propagates out of `pull`. The node keeps its previous value and is computed again on the next pull.
- `disposeDerivedNode` detaches a node you no longer need. Nodes that read it recompute on their next pull.
- A computation must not read its own node, directly or through other nodes.
````

The README's rules restate the engine's behavior under the options the body is written for: open decision 2 (f), 3 (a), 8 (a), 9 (a), and 10 (a). Each decision that changes names the README line it changes.

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

Preact signals uses versions instead of marks. Every signal has a `_version`, a global `globalVersion` counts writes, and each dependency link (a doubly linked list `Node` with `_source`, `_target`, and `_version`) records the version the target last saw. A computed decides with `needsToRecompute`, which walks its sources in order and returns at the first whose version changed, refreshing derived sources as it goes: pico's epochs per link, with pico's in-order walk. Writes notify effects eagerly through flags (`OUTDATED`, `NOTIFIED`). Its comparison is `!==`, a setter with an equal value notifies nothing, and a computed that throws stores the error (`HAS_ERROR`) and rethrows it on read, which is open decision 3 (c). It throws "Cycle detected" when a computed is read while `RUNNING`, which is open decision 4 (b). It records reads through an ambient `evalContext`, and a signal written inside a computed is allowed.

The engine takes Reactively's marking and pull, pico's equality, guard, and per-computation dependency recording, and neither library's ambient tracking context.
