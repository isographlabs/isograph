# pico-js: source nodes and derived nodes as a standalone package

Status: under discussion. Not approved for implementation. This doc is the single specification of the engine that `refactors/pending/runtime/pico-runtime.md` places in its own package: the API, every function body, disposal, cycles, throws, the README, the design description, the test inventory against the Rust pico crate, the package files, and the open decisions. pico-runtime.md holds no copy of it and references it by section (What this doc changes in pico-runtime.md).

pico paths are under `crates/pico/src` and `crates/pico/tests` of this repo. Reactively is `packages/core/src/core.ts` of `milomg/reactively`, cited by line at commit `9081fa4bf302`. Preact signals is `packages/core/src/index.ts` of `preactjs/signals`, read at its `main` branch, with no line numbers, because no checkout is local. alien-signals is `src/system.ts` of `stackblitz/alien-signals`, cited by line at tag `v3.2.1`.

The package is `@isograph/pico-js` in `libs/pico-js` (decision 1, decided; pico-runtime.md open decision 18).

## Goal

The engine is a graph of values. A source node holds a value that code outside the graph sets. A derived node holds the value of a computation over source nodes and other derived nodes. The engine records which nodes each computation read, marks the derived nodes above a written source, and recomputes a derived node only when it is read and one of these holds: it was never computed, it was disposed, a source it read was written to a value that source's equality rejected, or a derived node it read recomputed to a value that node's equality rejected. A source written to a new value and then written back to the old one before a pull still makes its direct readers recompute once; each of them computes the old value, its equality accepts it, and nothing above it recomputes.

The engine imports nothing. It names no store, record, link, key, reader AST, entity, resolver, or other Isograph type. A project with values of its own types uses it with no Isograph code. The Isograph runtime is one caller: its server values are source nodes and its resolver results are derived nodes (pico-runtime.md, The engine and the Isograph runtime). This doc describes the engine alone.

The engine's public behavior is pico's: which values a read returns, which computations run again after a write, cutoff by equality, and dependencies recorded per computation and replaced after every computation. Its invalidation differs from pico's. pico stamps sources with epochs and verifies a demanded memo by walking its dependencies. The engine keeps reverse edges and marks the nodes above a write eagerly, then recomputes lazily on a read (Comparison with the Rust pico crate).

The engine gives up no performance to prevent misuse. Misuse is a documented contract: the engine runs no check to catch a caller that breaks one, and a broken contract leaves values silently stale or the engine's state unguaranteed. pico rejects a write during a computation and takes the database as a parameter of every call; the engine has neither the check nor a database object (decision 2).

## The model

### Nodes and marks

A source node holds a value and the set of derived nodes whose last computation read it (`dependents`).

A derived node holds a computation, an equality, its memo, and the set of derived nodes whose last computation read it. A node that was never computed, or was disposed, has no memo. Otherwise the memo holds a value, the nodes its computation read, and one of three marks, the marks of Reactively (`CacheClean`, `CacheCheck`, `CacheDirty`):

- `Clean`: the stored value is what the computation would return now.
- `Check`: a derived node this one read, directly or transitively, is `Dirty`. The stored value may be current.
- `Dirty`: a source node the last computation read was written, or a derived node it read recomputed to a value its equality rejected.

Every mark holds the last computed value and the dependencies the last computation recorded: its source dependencies and its derived dependencies, two lists, each in read order. The first pull of a node with no memo computes it and calls no equality.

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
- `Check` pulls the node's derived dependencies in recorded order and stops at the first one whose value after the pull is not `Object.is` its value before, and this node then runs its computation. When every dependency returns its previous value, the node becomes `Clean` without running.

```text
pull label: Check
  -> parity: Check
    -> sum: Dirty -> run sum: reads a, b -> 7, and 7 !== 3
       -> sum Clean 7 -> sum.dependents = {parity} -> parity Dirty
    -> parity: sum was 3, is 7 -> run parity: reads sum -> 'odd', and 'odd' === 'odd'
       -> parity Clean, keeps 'odd'; parity's dependents are not marked
  -> label: parity was 'odd', is 'odd' -> label Clean without running
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

pull node       -> reads gate ('even'), then other (20); node.derivedDependencies = [gate, other]
write x = 3     -> gate Dirty -> node Check
write y = 11    -> other Dirty -> node already Check -> stop
pull node: Check
  -> gate: Dirty -> run gate -> 'odd' !== 'even' -> node Dirty; gate was 'even', is 'odd' -> stop the walk
  -> run node: reads gate ('odd') -> 0; node.derivedDependencies = [gate]
other is never pulled, and its computation does not run
```

### Cutoff by per-node equality

Each derived node carries an equality. After a recomputation, the engine compares the previous value with the new one. When the equality accepts, the node keeps the previous value object and does not mark its dependents. When it rejects, the node stores the new value and marks its dependents `Dirty`. The engine defines no equality of its own: every `createDerivedNode` caller supplies one (decision 6). The Isograph layer passes `===`.

Each source node carries an equality too. `writeSource` compares the stored value with the new one, and when the equality accepts, it stores nothing and marks nothing (decision 8).

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
  pull label: Check -> parity: Dirty -> run parity -> 'odd' === 'odd' -> parity Clean, keeps 'odd'
    -> parity was 'odd', is 'odd' -> label Clean without running
pull label, then pull sum
  pull label: Check -> parity: Check -> sum: Dirty -> run sum -> 5 !== 3 -> parity Dirty
    -> sum was 3, is 5 -> run parity -> 'odd' === 'odd' -> parity keeps 'odd'
    -> parity was 'odd', is 'odd' -> label Clean without running
  pull sum: Clean -> 5
either order: sum runs once, parity once, label not at all
```

A node recomputes at most once per write. After its pull it is `Clean`, and during a pull only a recomputation's marking of dependents runs, which reaches only nodes that are not `Clean` (the invariant). No order is stored: a pull brings a node's inputs up to date before the node uses them.

### Reverse edges, no epochs

Every edge is an object reference, held in both directions: a derived node's two dependency lists point down, and every node's `dependents` set points up. pico keeps forward edges only and finds nodes by id; it decides staleness with epochs (Comparison with the Rust pico crate). The engine needs no epoch: the mark on a node says whether its value is current, and the reverse edges let a write reach exactly the nodes above it. A write costs the nodes it reaches, and a pull of a `Clean` node is one step no matter how many unrelated writes happened. Both designs return the same values and run the same computations.

## The public API

Every public function takes the node it acts on, and `readSource` and `readDerived` also take the derived node whose computation is running. There is no graph object: a graph is the set of nodes that its edges connect (decision 2).

```ts
// from libs/pico-js/src/index.ts
export {
  type Computation,
  createDerivedNode,
  createSourceNode,
  type DerivedNode,
  disposeDerivedNode,
  markSourceDirty,
  pull,
  readDerived,
  readSource,
  type SourceNode,
  writeSource,
} from './graph';
export { objectIs, tripleEquals } from './equality';
```

The engine has no default equality: `createSourceNode` and `createDerivedNode` take a required `equals` (decisions 6 and 8). The package exports two comparisons that a caller passes explicitly (decision 6).

```ts
// from libs/pico-js/src/equality.ts
export function tripleEquals<T>(a: T, b: T): boolean {
  return a === b;
}

export function objectIs<T>(a: T, b: T): boolean {
  return Object.is(a, b);
}
```

`Memo` and `Status` are exported from `graph.ts` because the declaration of `DerivedNode` names them, and not from the package index: no caller names them. `MISSING`, `Missing`, and `Option` are not exported from `graph.ts`; tsc 5.6 emits them as local declarations in `graph.d.ts` beside the exported `DerivedNode` that names them. Nodes are plain objects whose fields a caller can read and write (decision 7, decided).

### Types

```ts
// from libs/pico-js/src/graph.ts
export type SourceNode<TValue> = {
  value: TValue;
  equals(previous: TValue, next: TValue): boolean;
  readonly dependents: Set<Memo<unknown>>;
};

export type DerivedNode<TValue> = {
  readonly computation: Computation<TValue>;
  equals(previous: TValue, next: TValue): boolean;
  memo: Option<Memo<TValue>>;
  sourceReads: SourceNode<unknown>[];
  derivedReads: DerivedNode<unknown>[];
  readonly dependents: Set<Memo<unknown>>;
};

export type Memo<TValue> = {
  value: TValue;
  status: Status;
  sourceDependencies: SourceNode<unknown>[];
  derivedDependencies: DerivedNode<unknown>[];
  readonly node: DerivedNode<TValue>;
};

export type Status = 'Clean' | 'Check' | 'Dirty';

const MISSING: unique symbol = Symbol('Missing');
type Missing = typeof MISSING;

type Option<TValue> = TValue | Missing;

/** A computation is a pure, synchronous function of the nodes it reads with `readSource` and `readDerived`. */
export type Computation<TValue> = (consumer: DerivedNode<unknown>) => TValue;
```

A `dependents` set holds the memos of the derived nodes that read the node, and a memo exists only after its node computed, so a `dependents` set cannot hold an uncomputed node. A dependency list holds nodes, because `verify` pulls through the node. The traces write a memo in a `dependents` set by its node's name.

`Memo` takes its name from Salsa, whose `Memo<V>` holds a query's cached value, its verification state, and its recorded dependencies; pico uses the word for the same concept (pico.md, Memo).

`TValue` is per node: a `DerivedNode<string>` reads a `DerivedNode<number>` and a `SourceNode<number>`. The links hold `DerivedNode<unknown>` and `SourceNode<unknown>`, written out at each use with no alias, and the engine's functions are generic in the node they receive, so a memo reached through `dependents` is handled at `TValue = unknown`. `equals` is a method signature: under `strict` (`tsconfig.build.json`), TypeScript checks the parameters of a function-typed property contravariantly and those of a method bivariantly, so with `readonly equals: (previous: TValue, next: TValue) => boolean` a `DerivedNode<number>` is not assignable to `DerivedNode<unknown>`. The engine calls `equals` only in `recompute`, with two values the node's own computation produced, so the bivariance admits no call the engine makes with a value of another type (open decision 5).

`equals` returns `boolean`. It is a comparator, a pure yes or no with no domain names (AGENTS.md, Booleans). The name `equals` matches the equality option of TC39 Signals' `Signal.State` and `Signal.Computed`.

### Functions

```ts
// from libs/pico-js/src/graph.ts
export function createSourceNode<TValue>(
  value: TValue,
  equals: (previous: TValue, next: TValue) => boolean,
): SourceNode<TValue>;

/** The computation of the returned node first runs on the first pull that reaches it. */
export function createDerivedNode<TValue>(
  computation: Computation<TValue>,
  equals: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue>;

export function readSource<TValue>(consumer: DerivedNode<unknown>, source: SourceNode<TValue>): TValue;

export function readDerived<TValue>(consumer: DerivedNode<unknown>, node: DerivedNode<TValue>): TValue;

/** `writeSource` stores and marks nothing when `source.equals` accepts `value`, and it must not run inside a computation or an equality (Writes during a computation). */
export function writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void;

/** `markSourceDirty` marks the dependents of `source` without comparing values, and it must not run inside a computation or an equality (Writes during a computation). */
export function markSourceDirty(source: SourceNode<unknown>): void;

/** Inside a computation, `pull` records no dependency (decision 10). */
export function pull<TValue>(node: DerivedNode<TValue>): TValue;

/** A later pull computes the disposed node again, and `disposeDerivedNode` must not run inside a computation or an equality (Writes during a computation). */
export function disposeDerivedNode(node: DerivedNode<unknown>): void;
```

In the Isograph runtime (pico-runtime.md, The engine and the Isograph runtime), the callers are `createSourceNode` in `store.ts`, which reads `source.value` for an untracked read; `writeSource` in `normalize.ts` and `test/fixtures.ts`; `markSourceDirty` and `disposeDerivedNode` in `deleteRecord`; `createDerivedNode`, `pull`, `readSource`, and `readDerived` in `storedResults.ts`, which passes `resolverResultsEqual(artifact, tripleEquals)` as each node's equality, with `===` as its default output equality. Outside Isograph: the caller's own code, as in The package README.

## The implementation

`graph.ts` is the whole engine. Every function below is in it.

### Construction

```ts
// from libs/pico-js/src/graph.ts
export function createSourceNode<TValue>(
  value: TValue,
  equals: (previous: TValue, next: TValue) => boolean,
): SourceNode<TValue> {
  return { value, equals, dependents: new Set() };
}

export function createDerivedNode<TValue>(
  computation: Computation<TValue>,
  equals: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue> {
  return {
    computation,
    equals,
    memo: MISSING,
    sourceReads: [],
    derivedReads: [],
    dependents: new Set(),
  };
}
```

### Mark

```ts
// from libs/pico-js/src/graph.ts
export function writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
  if (source.equals(source.value, value)) {
    return;
  }
  source.value = value;
  markDependentsDirty(source.dependents);
}

export function markSourceDirty(source: SourceNode<unknown>): void {
  markDependentsDirty(source.dependents);
}

function markDependentsDirty(dependents: Set<Memo<unknown>>): void {
  const stack: Memo<unknown>[] = [];
  for (const dependent of dependents) {
    if (dependent.status === 'Clean') {
      stack.push(dependent);
    }
    dependent.status = 'Dirty';
  }
  let memo: Memo<unknown> | undefined;
  while ((memo = stack.pop()) !== undefined) {
    for (const dependent of memo.node.dependents) {
      if (dependent.status !== 'Clean') {
        continue;
      }
      dependent.status = 'Check';
      stack.push(dependent);
    }
  }
}
```

The first loop of `markDependentsDirty` marks each direct dependent `Dirty` whatever its mark, because its own input changed, and pushes each one that was `Clean`. Its second loop pops a memo and walks the `dependents` of its node: each `Clean` dependent becomes `Check` and is pushed, and a dependent that is not `Clean` is skipped, because by the invariant its own dependents are already marked. The marks are the visited set: a node is pushed only when it leaves `Clean`, so each node is pushed at most once and each `dependents` set is walked at most once per write. `dependents` holds memos, so both loops write `status` with no `MISSING` test.

`markDependentsDirty` never adds or removes an element of a `dependents` set, so it iterates the sets directly while it marks. Both loops are `for...of` and `while` with no callback, so a write allocates one stack array and no closure. Each `for...of` over a `Set` creates a `Set` iterator object unless the compiler elides it; nothing else is allocated per marked node, and no `dependents` set is copied.

Marking does not recurse. The second loop of `markDependentsDirty` is a loop over a work list, the worklist case of AGENTS.md (Iterators), so its stack depth is the same for a chain of ten nodes and a chain of 100,000, and marking runs to completion on a graph of any height. A recursive marking would overflow the call stack on a tall enough chain; the `RangeError` would leave the nodes below the overflow `Check` and the nodes above it `Clean`, a `Clean` dependent of a node that is not `Clean`, and a later write could not repair it, because marking stops at the direct reader, which is already `Dirty`. `stack` holds at most one entry per node that leaves `Clean`. Each dependent is pushed singly, so no call receives a set's size in arguments.

### Pull

```ts
// from libs/pico-js/src/graph.ts
export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  const memo = node.memo;
  if (memo === MISSING) {
    return computeFirst(node);
  }
  switch (memo.status) {
    case 'Clean':
      return memo.value;
    case 'Check':
      return verify(node, memo);
    case 'Dirty':
      return recompute(node, memo);
    default:
      return assertNever(memo.status);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled variant: ${String(value)}`);
}

function verify<TValue>(
  node: DerivedNode<TValue>,
  memo: Memo<TValue>,
): TValue {
  for (const dependency of memo.derivedDependencies) {
    const before = dependency.memo;
    if (before === MISSING) {
      throw new Error('A Check node depends on a derived node that was never computed, but disposing a node marks every dependent Dirty.');
    }
    const previousValue = before.value;
    if (!Object.is(pull(dependency), previousValue)) {
      return recompute(node, memo);
    }
  }
  memo.status = 'Clean';
  return memo.value;
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const value = compute(node);
  const sourceDependencies = node.sourceReads;
  const derivedDependencies = node.derivedReads;
  const memo: Memo<TValue> = { value, status: 'Clean', sourceDependencies, derivedDependencies, node };
  node.sourceReads = [];
  node.derivedReads = [];
  node.memo = memo;
  setDependencies(memo, NO_DEPENDENCIES, sourceDependencies);
  setDependencies(memo, NO_DEPENDENCIES, derivedDependencies);
  return value;
}

function recompute<TValue>(node: DerivedNode<TValue>, memo: Memo<TValue>): TValue {
  const value = compute(node);
  commitReads(node, memo);
  if (node.equals(memo.value, value)) {
    memo.status = 'Clean';
    return memo.value;
  }
  memo.value = value;
  memo.status = 'Clean';
  markDependentsDirty(node.dependents);
  return value;
}

function commitReads<TValue>(node: DerivedNode<TValue>, memo: Memo<TValue>): void {
  const previousSources = memo.sourceDependencies;
  setDependencies(memo, previousSources, node.sourceReads);
  memo.sourceDependencies = node.sourceReads;
  node.sourceReads = previousSources;
  const previousDerived = memo.derivedDependencies;
  setDependencies(memo, previousDerived, node.derivedReads);
  memo.derivedDependencies = node.derivedReads;
  node.derivedReads = previousDerived;
}

const NO_DEPENDENCIES: ReadonlyArray<SourceNode<unknown> | DerivedNode<unknown>> = [];

function compute<TValue>(node: DerivedNode<TValue>): TValue {
  node.sourceReads.length = 0;
  node.derivedReads.length = 0;
  return node.computation(node);
}

export function readSource<TValue>(consumer: DerivedNode<unknown>, source: SourceNode<TValue>): TValue {
  consumer.sourceReads.push(source);
  return source.value;
}

export function readDerived<TValue>(consumer: DerivedNode<unknown>, node: DerivedNode<TValue>): TValue {
  const value = pull(node);
  consumer.derivedReads.push(node);
  return value;
}
```

### The consumer

A computation receives the derived node being computed, its consumer, and `compute` passes it with no cast. Consumer is the established term for the node whose computation records a read: the TC39 signal-polyfill (`src/graph.ts`, from Angular signals) tracks it as `activeConsumer` and records a read in `producerAccessed`. The first argument of `readSource` and `readDerived` is the node whose computation is running, and a computation passes only the node it received, only while it runs. The engine does not check this contract (decision 2). A read through the node after the computation returned pushes into a buffer the next `compute` clears, so the read is lost. A read that passes another node records the dependency on that other node.

There are two read functions in place of one `read(consumer, node)`: every caller knows statically whether it reads a source or a derived node, so `readSource` pushes onto the consumer's `sourceReads` and `readDerived` onto its `derivedReads`, and no node carries a `kind` field for the engine to test. `verify` walks only `derivedDependencies`, in the order the computation read derived nodes; the order between a source read and a derived read is not recorded, because verification never checks a source.

- `pull` is the only function that brings a derived node up to date: `computeFirst` for a node with no memo, and otherwise a switch on the mark. `computeFirst`, `verify`, and `recompute` return the node's value, so a pull allocates no result object.
- `verify` reads a dependency's value before it pulls the dependency and compares it with the value the pull returns, by `Object.is`. It reads `before.value` into `previousValue` before the pull because `recompute` writes the new value into the same `Memo`. It returns `recompute` at the first derived dependency whose value is not `Object.is` its previous value. This relies on `recompute` keeping the previous value object whenever `equals` accepts, so a dependency whose `equals` accepted returns the identical value. When `equals` rejects a value that is `Object.is` the previous one (`tripleEquals` on `NaN`), `verify` also treats the dependency as unchanged; the two values are indistinguishable, so the verified node's value is the value its computation would return. The comparison is `Object.is` in place of `!==` because `0 === -0` while a reader can tell them apart: under `objectIs`, a dependency that recomputes from `0` to `-0` is rejected by `equals`, and `!==` would leave its reader `Clean` over `0`. A derived dependency of a `Check` node is never `MISSING`: a node in a `derivedDependencies` list holds a `Memo` from the computation that pulled it, and disposing it marks every dependent `Dirty`, so no `Check` node is left over it. The type does not exclude `MISSING`, so `verify` throws on it, and TypeScript narrows `before` after the throw. Source dependencies are not walked. `recompute` still marks the dependents of a changed node `Dirty`, because other `Check` nodes that read it are not on this walk and find the mark on their own pull. When the loop ends, every derived dependency returned its previous value and no source dependency was written, so the node becomes `Clean`. It is a `for` loop with `return` in place of `some`, which allocated a closure per verify; a verify is one step of every pull of a `Check` node.
- `readDerived` records the dependency after the pull returns. A pull that throws records nothing, and the throw leaves the reading computation, so no `derivedDependencies` list holds a node whose pull threw.
- `recompute` on a computed node calls `commitReads` once, before `equals`. Both outcomes store the new reads as the dependency lists, so a node whose value came back equal still depends on what it read this time. `equals` compares two values and reads no node, so it does not depend on which dependency list the state holds.
- `computeFirst` marks no dependent. A memo enters a `dependents` set only through the `setDependencies` of a reader whose computation pulled the node, so a never-computed node has no dependents. `disposeDerivedNode` marks every dependent of the node it disposes `Dirty`. A dependent stays in that set until its own `recompute` (or its disposal) runs `setDependencies`, and a `Dirty` node leaves `Dirty` only through `recompute`, which either drops the disposed node from its dependencies or pulled it during its computation, while the dependent was still `Dirty`. Every dependent of a node `computeFirst` runs on is therefore `Dirty`, and `markDependentsDirty` on `Dirty` nodes changes nothing.
- A node that was never computed or was disposed holds `MISSING` in place of a `Memo`, so its value cannot be read without first ruling `MISSING` out. Its first pull is `computeFirst`, which calls no `equals`. Marking never reaches such a node, because a `dependents` set holds memos. The `Memo` is allocated once per first computation; every later mark change and recomputation writes `status` and `value` in place, so marking and recomputing allocate no state object.
- A node's source dependency list (`Memo.sourceDependencies`) and `node.sourceReads` are two arrays the node owns, swapped by every `commitReads`: the reads become the dependency list, and the previous dependency list becomes the buffer the next computation clears and fills. `derivedDependencies` and `derivedReads` are a second pair, swapped the same way. A computation allocates no array, no result object, and no context object: `compute` returns the value alone and passes the node itself (The consumer). A throw out of the computation leaves the reads buffers partly filled and the state untouched; the next `compute` clears them. `createDerivedNode` allocates the first pair of reads buffers and `computeFirst` the next pair, two arrays per node creation and two per first computation; no array is shared between two nodes, and `NO_DEPENDENCIES` is only read. The dependency lists are `SourceNode<unknown>[]` and `DerivedNode<unknown>[]` in place of `ReadonlyArray`, because the engine reuses them as buffers.
- A computation is synchronous; a read through its node after it returned breaks the contract (The consumer).
- The consumer is a parameter: the running node itself. There is no ambient current computation (Reactively's `CurrentReaction`, Preact's `evalContext`), so a computation that keeps its contract cannot have its reads recorded on another node, and two graphs never share tracking state. No state names the running computation, so `compute` saves and restores nothing and a throw out of a computation leaves nothing to reset.

`pull`, `verify`, `recompute`, `compute`, and `readDerived` recurse: a pull goes down one chain of calls per level of derived nodes below the pulled node, about five frames per level on the `recompute` path (`readDerived`, `pull`, `recompute`, `compute`, the computation) and two on the `verify` path (`pull`, `verify`). The depth limit is the host's call stack. Pull depth is a contract on the caller (open decision 12): the derived nodes below any node a caller pulls must be well below the limit. In Node 24 with its default stack, a one-frame recursive function reaches about 10,000 frames before `RangeError`, so the limit is an estimated 2,000 levels of derived nodes at five frames per level; larger frames in a computation lower it. The estimate is not measured against the engine. The engine does not raise the limit, does not catch the `RangeError`, and has no iterative pull. A pull that overflows throws `RangeError`, and in that case alone the engine does not guarantee its own state. The overflow can fire in any frame at the limit, including a `setDependencies` call after it removed `memo` from some `dependents` sets of `previous` and before it added `memo` to those of `next`, so a node's edges may be partly moved, and one of its dependency lists then names a dependency whose `dependents` set does not hold it. Marking is iterative (Mark) and never overflows. In the Isograph runtime, pull depth is bounded by client field nesting: each resolver node reads a client field it selects through `readDerived`, so the levels below a node are the nesting depth of client fields selected by client fields.

### Dependencies

```ts
// from libs/pico-js/src/graph.ts
function setDependencies(
  memo: Memo<unknown>,
  previous: ReadonlyArray<SourceNode<unknown> | DerivedNode<unknown>>,
  next: ReadonlyArray<SourceNode<unknown> | DerivedNode<unknown>>,
): void {
  if (previous.length === next.length) {
    let index = 0;
    while (index < previous.length && previous[index] === next[index]) {
      index += 1;
    }
    if (index === previous.length) {
      return;
    }
  }
  for (const dependency of previous) {
    dependency.dependents.delete(memo);
  }
  for (const dependency of next) {
    dependency.dependents.add(memo);
  }
}
```

A dependency list holds the nodes themselves, so `readSource` and `readDerived` allocate nothing per read: each pushes the node it read onto its own list. A computation is a pure function of what it reads, so a recomputation over unchanged inputs reads the same sources in the same order and the same derived nodes in the same order. `setDependencies` diffs one list pair at a time. Its parameters are `ReadonlyArray<SourceNode<unknown> | DerivedNode<unknown>>`, because it touches only `dependents`, which both node types have. It checks the unchanged case first: `n` pointer comparisons, no `Set` operation, and no allocation, and it is correct with duplicate entries, because equal lists name the same edges. A patch that keeps a common prefix and rewrites the tail would be wrong with duplicates unless each edge carried a count, and a hash-based diff of the two lists is not faster than removing and re-adding: it still costs at least two hash operations per kept dependency and allocates a set. Otherwise, removing every old edge and then adding every new one is `O(old + new)`. The loops are `for` loops in place of `forEach`, which allocated a closure per call over `node`. Removal runs first, so a dependency in both lists ends present, and a dependency read twice is one member of its set. A violation of "the `dependents` sets are the inverse of the dependency lists" is a silently stale result, which one writer guards.

The dependency list keeps a node read twice as two entries. pico drops a read equal to the last one recorded (`TrackedDependencies::push`, `crates/pico/src/dependency.rs:33-41`). The engine does not: `Set.add` makes the edge idempotent, and the second verify step on that node finds it `Clean` and returns at once, so dropping repeats would cost a comparison per read to save a step that costs one.

Reactively reuses the unchanged prefix of the previous source list (`CurrentGets`, `CurrentGetsIndex`) to avoid detaching and reattaching it. The engine skips the `Set` operations only when the whole list is unchanged, for the duplicate-entry reason above.

### Disposal

```ts
// from libs/pico-js/src/graph.ts
export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const memo = node.memo;
  if (memo !== MISSING) {
    setDependencies(memo, memo.sourceDependencies, NO_DEPENDENCIES);
    setDependencies(memo, memo.derivedDependencies, NO_DEPENDENCIES);
    node.memo = MISSING;
  }
  markDependentsDirty(node.dependents);
}
```

- Its memo leaves the `dependents` set of everything the node read, so no later write reaches it, and its `Memo`, which held its dependency lists, is dropped, so it lists no dependency that does not list it back.
- Its memo becomes `MISSING`, so the engine holds no reference to the value, and its next pull calls `computeFirst`.
- Its dependents are marked `Dirty`: their last computation read a value that no longer exists. Their `derivedDependencies` lists still name the disposed node, and `node.dependents` still holds their memos, so the inverse holds. Their next recomputation either reads the node again, which computes it from scratch (open decision 9), or does not, and `setDependencies` removes their memos from `node.dependents`.
- Disposing a node that was never computed or was already disposed changes nothing but marking its dependents, of which a never-computed node has none.

Without `MISSING`, a `Clean` node disposed by `setDependencies` and marking alone would keep its value while no source lists it, so a later pull would return its old value forever.

### Throws

A computation must not throw, and an equality, of a source or of a derived node, must not throw either. A throw out of either is a programmer error. For a throw out of a computation, the engine defines no behavior for the node that threw: no retry, no stored error, no value. The engine guarantees only that its own state is not corrupted. A throw out of a computation propagates out of `compute` before `setDependencies` and before any state is written, so every mark, value, dependency list, and `dependents` set is as it was before the throwing run. Nodes pulled inside the failed computation before the throw are `Clean` and correct, because each pull completed.

A `RangeError` from a pull that exceeds the call stack is the one throw this guarantee does not cover (Pull, open decision 12). It can leave from inside `setDependencies`, after some edges moved, so after it the engine's state is not guaranteed. Pull depth is a contract: a graph whose pull depth stays well below the stack limit never overflows.

### Cycles

The derived nodes and the reads between them must form a directed acyclic graph. A computation that reads its own node, directly or through other nodes, is a programmer error, as a throw is (Throws), and the engine has no cycle check. In the Isograph layer the compiler guarantees acyclicity: `SelectionMapErrorKind::InlineCycle` (`crates/isograph_compiler/src/merged_selection.rs:240`, found by `inline_cycle`, `:1352`) rejects a static cycle of field selections, so no runtime node reads itself.

## The package README

`libs/pico-js/README.md`, as `libs/isograph-disposable-types/README.md` is for its package. Its text:

````markdown
# @isograph/pico-js

A graph of values that recomputes on read. You hold source nodes, whose values you set, and derived nodes, whose values are computations over other nodes. The engine records what each computation read. When you write a source, the engine marks the derived nodes that read it, directly or transitively, and runs nothing. When you read a derived node, the engine recomputes it only if something it read changed value, and stops at the first node whose new value equals its old one.

Use it when values are derived from inputs that change one at a time, computing a value is costly, and you read values more often than they change: a compiler's analyses over files, view data over a normalized store, a form's validation over its fields.

## Example

```ts
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '@isograph/pico-js';

const price = createSourceNode(100, tripleEquals);
const quantity = createSourceNode(3, tripleEquals);

const total = createDerivedNode(
  (consumer) => readSource(consumer, price) * readSource(consumer, quantity),
  tripleEquals,
);
const isLargeOrder = createDerivedNode(
  (consumer) => readDerived(consumer, total) >= 1000,
  tripleEquals,
);
const banner = createDerivedNode(
  (consumer) => (readDerived(consumer, isLargeOrder) ? 'Free shipping' : ''),
  tripleEquals,
);

pull(banner); // '' (runs total, isLargeOrder, banner)
writeSource(quantity, 4); // runs nothing
pull(banner); // '' (runs total and isLargeOrder; isLargeOrder is still false, so banner does not run)
writeSource(quantity, 10);
pull(banner); // 'Free shipping' (runs all three)
```

## Rules

- A computation reads nodes only through the `consumer` it receives: `readSource(consumer, source)` and `readDerived(consumer, node)`. A read through a closure, `source.value`, or `pull` is not recorded, and the node will not recompute when that value changes.
- The `consumer` is the node being computed. A computation reads no field of it and passes it only to `readSource` and `readDerived`, and only while it runs. The engine does not check this: a read through a consumer after its computation returned, such as a read after an `await`, is lost, and the node does not recompute when that value changes.
- A computation is a pure function of what it reads. It does not write a source or dispose a node. The engine does not check this: a write inside a computation can leave the computing node holding a value computed from an input that changed during its run, and every later pull returns that stale value with no error. A read of `source.value` or a `pull` inside a computation is an untracked read: it returns the value and records no dependency.
- An equality, of a source or of a derived node, does not write a source or dispose a node either. The engine does not check this, and a violation leaves values stale in the same way.
- Each derived node has an equality. When a recomputed value is equal to the previous one, the node keeps the previous value object and the nodes above it do not recompute. The engine has no default: pass `tripleEquals` for `===`, `objectIs` for `Object.is`, or your own comparator for values you build fresh on each run.
- A node above a recomputed node also does not recompute when the recomputed value is `Object.is` the previous one, whatever the equality answered. A computation therefore does not change in place and return the object it returned before; it returns a new object when its value changes.
- Each source node has an equality too. A `writeSource` of a value equal to the stored one stores nothing and marks nothing. `markSourceDirty` marks a source whose value object you changed in place.
- A computation and an equality must not throw. A throw is a programmer error. A computation's throw propagates out of `pull`, and the engine guarantees only that its own state is consistent. The node that threw keeps its mark, value, and dependencies; nodes the failed computation pulled before the throw are `Clean` and correct.
- `disposeDerivedNode` detaches a node you no longer need. Nodes that read it recompute on their next pull.
- A computation must not read its own node, directly or through other nodes.
- Keep the chain of derived nodes below any node you pull well below the call stack limit, an estimated 2,000 levels in Node with its default stack. A pull past the limit throws `RangeError`, and after it the engine does not guarantee its own state. Writes never overflow, at any height.
````

The README's rules restate the engine's behavior under the option the body is written for: open decision 9 (a). Each decision that changes names the README line it changes.

## The design

This section is the engine's counterpart of `docs-website/docs/design-docs/pico.md`. It describes what each part is and why, for an agent working inside the engine or on code that calls it.

### Sources

A source node is an input fact: a value the graph does not compute. The caller creates it with `createSourceNode(value, equals)` and holds the returned object. The object is the source's identity. There is no key, no lookup by key, and no absent source: a source exists from creation and always holds a value. A caller that needs "not set yet" stores a value of its own that says so (`undefined`, a sentinel symbol, or a tagged union), and readers depend on that value like any other. The Isograph runtime's `MISSING` is such a value.

A source is written with `writeSource(source, value)`. The write compares the stored value with the new one by the source's `equals`. When `equals` accepts, the write stores nothing and marks nothing; when it rejects, the write sets the value and marks (decision 8). The engine has no default equality, for sources as for derived nodes (decision 6). `markSourceDirty(source)` marks without setting, for a caller that changed the value object in place or whose source went away.

Code outside a computation reads a source as `source.value`, which records nothing. A computation reads it with `readSource(consumer, source)`, passing the node it received.

A source is not collected by the engine. It lives as long as the caller and the dependency lists of its readers hold it.

### Derived nodes

A derived node is a computation, an equality, a state, and a set of dependents. The caller creates it with `createDerivedNode(computation, equals)` and holds the returned object. The object is the node's identity. A caller that wants one node per key (pico's memo arguments) keeps its own map from key to node; the engine has no memo table. The Isograph runtime's record containers are such a map.

The computation receives its own node (The consumer) and returns the value. It must be a pure function of what it reads with `readSource` and `readDerived`: no clock, no network, no read through a closure. It must be synchronous, because the dependency list is final when the computation returns; the engine does not check it (decision 2). A computation runs only during a pull: first on the first pull that reaches the node, and again on a pull that finds it `Dirty`. It never runs during a write.

Two pulls with no write between them run nothing the second time. A pull of a `Clean` node is one step.

### How dependencies are recorded

`compute` clears the node's `sourceReads` and `derivedReads` buffers and passes the node itself to the computation. `readSource(consumer, source)` appends the source node to the reading node's `sourceReads` and returns its value. `readDerived(consumer, node)` pulls `node`, appends it to `derivedReads`, and returns its value. Each list is in read order and keeps repeats.

When the computation returns, `recompute` replaces the node's two dependency lists with the new ones and moves the reverse edges with `setDependencies`. The new lists replace the old ones even when the value came back equal. A read that a branch skips is not a dependency: a node that stopped reading `a` is not marked by a write of `a`.

Only `readSource` and `readDerived` record. A computation that reads a source as `source.value`, or a derived node through `pull`, records nothing: both are untracked reads, allowed inside a computation (decision 10). pico has the same hole and the same rule: `MemoRef::lookup` reads without recording and `lookup_tracked` records (`crates/pico/src/memo_ref.rs:57`, `:80`; pico.md, Memo).

### Verification

A write marks the nodes above it. A pull of a `Check` node verifies: it pulls the node's derived dependencies in recorded order, each of which is `Clean` (one step), `Check` (verified the same way), or `Dirty` (recomputed), and compares the value each pull returns with the value the dependency held before it, by `Object.is`. A dependency whose `equals` accepted its recomputed value kept the previous value object, so the comparison sees no change. When one dependency returns a different value, the walk stops and this node recomputes. When every one returns its previous value, every derived input came back equal and no source input was written, so the node becomes `Clean` without running. A recomputation that changes a value still marks its dependents `Dirty`, for the `Check` nodes that are not on the current walk.

The walk stops at the first changed dependency because the recomputation that follows decides which nodes are read next. A dependency after the first changed one may be one the recomputation never reads, and pulling it would run a computation whose value nobody needs.

### Cutoff

`equals(previous, next)` decides whether the nodes above a recomputed node recompute. When it accepts, the node keeps the previous value object, so a reader comparing by identity sees no change, and its dependents stay as marked: a `Check` dependent then verifies to `Clean` without running. When it rejects, the node stores the new value and marks its dependents `Dirty`.

An equality may answer equal only when no reader can tell the two values apart. An equality that answers equal for distinguishable values makes readers keep a stale value. An equality that answers unequal for indistinguishable values costs recomputation, never a wrong value. The engine has no comparison of its own; the caller supplies every equality (decision 6). A caller whose computation builds a new object on each run supplies a structural comparator, or the nodes above it recompute on every change below.

pico's equivalent is backdating: an equal recomputation keeps the old `time_updated` (pico.md, Equality and backdating).

### Disposal

`disposeDerivedNode(node)` is the only way a derived node leaves the graph. It detaches the node from what it read, resets it to never computed, and marks its dependents `Dirty` (Disposal). The engine does not decide when to dispose: there is no reachability walk, no LRU, and no reference count in the engine's first implementation. The caller disposes a node when it drops the key that held it. The Isograph runtime needs this: without disposal, after `deleteRecord` the old derived node recomputes, attaches to the source nodes of the new `Absent` container, and stays referenced and marked, and when it recomputes to an equal value a reader keeps it while `resolverNodeAt` creates a second node for the same key. Disposal removes its edges, resets it to `MISSING`, and marks its readers `Dirty`, so their recomputation reads the new node. pico collects instead (`run_garbage_collection`, `crates/pico/src/database.rs:169`), keeping the last 10,000 top-level calls and what they reach (pico.md, Garbage collection); a collection policy for the engine is pico-runtime.md's Later: collecting derived nodes.

A derived node with no dependencies is computed once and stays `Clean` under every write, because no source lists it. A `Check` dependent's verify pulls it in one step. Only disposal changes it, which marks its dependents `Dirty`; its next pull computes it again. In the Isograph runtime, a refinement such as `asPet` is a derived node of this kind.

### Writes during a computation

A computation and an equality must not write a source or dispose a node. The engine does not check it (decision 2). A computation that reaches a source through a closure and writes it marks nodes whose recorded dependencies are being replaced, and the computing node ends `Clean` over an input that changed during its run:

```text
p Dirty -> recompute p: reads c (Clean) -> p's computation writes s -> s.dependents = {c} -> c Dirty
  -> c.dependents = {} (p's memo is added only after compute returns)
p finishes -> setDependencies: c.dependents = {p} -> p Clean, over a Dirty c
write s again -> c already Dirty -> stop -> p is never marked
```

An equality runs while the engine is between two writes of its own state: `writeSource` calls `source.equals` before it sets the value, and `recompute` calls `node.equals` after the computation returned and its reads were committed, before it writes the value and the mark. A write from inside either leaves a node `Clean` over a changed input in the same way. Reads are not restricted: a read of `source.value` or a `pull` inside a computation is an untracked read, which records no dependency of the running computation and writes no source (decision 10).

## Comparison with the Rust pico crate

The data model is pico's:

- Sources are set from outside; derived nodes are values of computations (`source.rs:83-86`, `derived_node.rs:55-58`).
- A computation's reads are recorded as it runs, and the list replaces the previous one after each run (`invoke_with_dependency_tracking`, `execute_memoized_function.rs:249`).
- An equal recomputation cuts off: pico keeps `time_updated` when `prev_value == value` (`update_derived_node`, `execute_memoized_function.rs:163`); the engine keeps the previous value object and does not mark dependents.
- Verification walks dependencies in recorded order and stops at the first changed one (`any_dependency_changed`, `execute_memoized_function.rs:183-206`, an `Iterator::any` over the list; `derived_node_changed_since`, `:208-247`, re-executes a derived dependency to find out).
- A cycle is not supported. pico panics (`assert_no_cycles`, `dependency.rs:95-99`); the engine has no check, and a cycle is a programmer error (Cycles).

Invalidation differs:

- pico has a database-wide epoch. A write that changes a source increments it and stamps the source (`set_source`, `database.rs:310-329`). A demanded memo verified in the current epoch returns at once (`node_verified_in_current_epoch`, `execute_memoized_function.rs:80`); otherwise pico walks its dependencies and compares each one's `time_updated` with the memo's `time_verified`. After any write anywhere, the first demand of a memo walks every dependency beneath it, though it runs nothing that did not change. The engine marks on write through reverse edges, so a write costs the nodes above it and a pull after an unrelated write is one step.
- pico keeps forward edges only, as ids into storage; the engine keeps object references in both directions.
- pico checks source dependencies during verification; the engine does not, because a changed source already marked its readers `Dirty`.
- pico's `set` compares the new source value with the old one and does not advance the epoch when they are equal (`database.rs:321`). The engine's `writeSource` compares by the source's `equals` and marks nothing when they are equal (decision 8).
- pico finds a memo by its function and arguments, interned as `DerivedNodeId`; the engine has no keys, and the caller holds node objects.
- pico has one equality per type (`PartialEq` through `DynEq`); the engine has one per node.
- pico removes a source by key, and a later `get` of its id panics (`removing_params.rs`); the engine has no removal, and a source object cannot dangle.
- pico collects derived nodes by LRU and reachability; the engine disposes on request.
- pico rejects a write during a computation (`assert_empty_dependency_stack`, `database.rs:193-198`, called from `set`, `remove`, `remove_singleton`, and `run_garbage_collection`, `:151-170`); the engine has no check, and such a write breaks its contract (decision 2).
- pico passes the database to every call and tracks the running memo on it (`Database::set`, `database.rs:151`; a memo's first argument is `&Db`; `dependency_stack`, `database.rs:51`). The engine passes the consumer explicitly to `readSource` and `readDerived`, so a read outside any computation is `pull(node)` (decision 2).

Observable behavior is the same: for any sequence of writes and reads that keeps the engine's contracts, both return the same values and run the same computations, given the same equality per source and per derived node.

## Comparison with Reactively and Preact signals

Reactively is the same scheme. Its states are `CacheClean`, `CacheCheck`, and `CacheDirty`, and it creates a node `CacheDirty` with no value; the engine keeps a node that was never computed apart from the three marks, as `MISSING` in place of a memo, so its first pull calls no equality. `set` marks direct observers `CacheDirty` through `stale`, which marks their observers `CacheCheck` and stops at a node already at or above the requested state. `updateIfNecessary` walks `sources` in order when `CacheCheck`, breaks when the node becomes `CacheDirty`, and then calls `update`. Its default equality is `===` (`defaultEquality`), per node through `equals`. `update` assigns the new value to `_value` (`:222`) before it compares it with the old one (`:259`), so an equal recomputation marks no observer and keeps the new value object. The engine keeps the previous value object on an equal recomputation (Cutoff), so a reader that holds the old object holds the node's current value. A throw out of the function propagates from `:222`, before the links are updated (`:224-251`) and before the state is set `CacheClean` (`:269`), so the state, the links, and `_value` are unchanged, as the engine's are; but `update` has already called and cleared the node's `onCleanup` callbacks (`:218-221`), so a later successful run does not call them. The engine has no cleanup callbacks. It has no cycle check. Its differences from the engine: an ambient `CurrentReaction` global records reads where the engine passes the reading node; `set` compares by `equals` and skips an equal value, as `writeSource` does by the source's `equals`; it reuses the unchanged prefix of the source list; and it schedules effects (`stabilize`), which the engine's core has none of.

Preact signals uses versions instead of marks. Every signal has a `_version`, a global `globalVersion` counts writes, and each dependency link (a doubly linked list `Node` with `_source`, `_target`, and `_version`) records the version the target last saw. A computed decides with `needsToRecompute`, which walks its sources in order and returns at the first whose version changed, refreshing derived sources as it goes: pico's epochs per link, with pico's in-order walk. Writes notify effects eagerly through flags (`OUTDATED`, `NOTIFIED`). Its comparison is `!==`, a setter with an equal value notifies nothing, and a computed that throws stores the error (`HAS_ERROR`) and rethrows it on read; the engine forbids the throw (Throws). It throws "Cycle detected" when a computed is read while `RUNNING`; the engine has no cycle check (Cycles). It records reads through an ambient `evalContext`, and a signal written inside a computed is allowed.

The engine takes Reactively's marking and pull, pico's equality and per-computation dependency recording, and neither library's ambient tracking context.

### Existing libraries

No published library has the engine's contract. Reactively, alien-signals, signal-polyfill, Preact signals, Vue's reactivity, MobX, Solid, Jotai, `@tldraw/state`, and `@liby-tools/salsa` each record reads through ambient tracking state, make equality optional or fix it, and have no explicit disposal of a derived node. alien-signals' `createReactiveSystem` (`src/system.ts:34`) is the only workable base: it holds no module state, and the caller supplies `update`, `notify`, and `unwatched`. The engine does not use it, for five reasons:

- The engine imports nothing (Goal), and a dependency on alien-signals breaks that.
- Its marks are bit flags on one `flags` field of the node (`ReactiveFlags`, `:24-32`), so the engine's `Memo`, which a later layered graph keeps once per layer, has no place.
- `link` returns early when the dependency read is the one read just before (`:52-55`), so consecutive repeated reads are merged into one link, where the engine records each read in its dependency list, and a node read twice is listed twice (pico-runtime.md, Example, Duplicate dependency entries).
- A node whose computation threw is left marked clean, where the engine leaves every mark as it was before the throwing run (Throws).
- It has had three major versions since its first release in 2024-10 (1.0.0 2025-01, 2.0.0 2025-04, 3.0.0 2025-09).

## Tests

The tests assert values with `toBe` and `toEqual`, and assert which computations ran by counting calls in a `calls` object each test owns. Counting re-invocations is allowed: which writes make which computations run again is the engine's own dependency tracking (AGENTS.md, Tests). No test reads a node's `memo`, `reads`, or `dependents`, and no test calls an unexported function: marks and edges are implementation, and each test observes them only through values and call counts. There are no snapshot tests and no fixture files; every input is built in the test.

The tests are written under the option the body is written for: open decision 9 (a). Each test whose assertions change under another option says how.

### The pico crate's tests

`crates/pico/tests/tests.rs` declares the modules under `absent_source`, `garbage_collection`, `params`, and `tracking_field`; each other file is its own test crate. pico has no `#[test]` in `crates/pico/src` or `crates/pico_macros/src`. There are 58 tests. 21 port, 37 do not.

Each port keeps pico's scenario and counters and states its delta. pico's memo keyed by a `SourceId` becomes a derived node whose computation closes over the source node, pico's `db.set` of an existing key becomes `writeSource`, and pico's `*memo(&db, ..)` at top level becomes `pull(node)`.

Top-level files:

- `basic.rs:14` `basic`: ports as `recompute.test.ts` "a derived node recomputes after its source is written".
- `basic_multi_function_chain.rs:21` `multi_function_chain`: ports as "a chain recomputes every node whose input changed".
- `basic_multi_function_chain.rs:48` `multi_function_chain_with_irrelevant_change`: ports as "an equal value cuts off the node above it".
- `basic_multi_function_chain.rs:75` `sequential_functions_with_memoref_param`: ports as "a top-level pull of an inner node, then of the node above it". Delta: pico passes the inner memo's `MemoRef` as the outer memo's argument and reads it with `lookup_tracked`; the engine's outer computation closes over the inner node and reads it with `readDerived`. The `MemoRef` parameter is pico representation; the behavior kept is that pulling the inner node first does not change what the outer pull runs.
- `cyclic_dependency_panic.rs:11` `test_calls_itself`, `:20` `test_calls_calls_itself`, `:29` `test_a`, `:38` `test_calls_a`: do not port. Each asserts only that pico detects the cycle and panics, and the engine has no cycle check (Cycles).
- `equal_reset_after_unrelated_epoch.rs:14` `equal_reset_after_unrelated_epoch_does_not_rerun`: does not port. It pins epoch bookkeeping: an unrelated set advances the epoch, and a later equal set of the read source must not move that source's `time_updated`. The engine has no epoch, and an unrelated write reaches no reader of another source; the behavior left is "an equal write", which the port of `overriding_same_source` covers.
- `intern.rs:12` `intern`, `intern_ref.rs:12` `intern_ref`, `intern_then_intern_ref.rs:10` `intern_then_intern_ref`: do not port. They pin `intern_value` and `intern_ref` identity; the engine interns nothing.
- `intern_ref_chain.rs:14` `intern_ref_chain`: ports as "a node's own equality cuts off a selection out of a changed collection". Delta: pico cuts off because `intern_ref` of an equal value keeps its `time_updated`; the engine cuts off because the selecting node's `equals` compares the selected element's fields. The behavior kept is that a reader of one element does not run when other elements change.
- `overriding_same_source.rs:14` `overriding_same_source`: ports as "writing an equal value". No delta: the write marks nothing, and neither the reader of the source nor the node above it runs.
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

- `missing_key.rs:57` `missing_then_set_reruns`: ports as `dependencies.test.ts` "a source that holds the caller's absence value". Delta: pico reads an absent key; the engine's source holds a sentinel the test defines, because an engine source always holds a value.
- `missing_key.rs:78` `missing_and_still_missing_across_epochs_does_not_rerun`: does not port. It pins that an absent key's dependency stays at the initial epoch across unrelated epochs; there is no epoch and no absent key.
- `missing_key.rs:100` `unrelated_set_does_not_rerun_reader_of_missing_key`: does not port. It pins keyed absence (`OtherConfig` versus `Config`); what is left is "a write to another source runs nothing", the port of `unrelated_source_changes`.
- `missing_key.rs:133` `set_reruns_only_readers_of_that_key`: does not port, for the same reason; the remainder is the ports of `basic` and `unrelated_source_changes`.
- `missing_key.rs:164` `same_missing_key_twice_in_one_memo`: ports as "a computation that reads one source twice". Delta: the source holds a value; the port adds a second write to assert that the edge survives the duplicate read.
- `missing_key.rs:192` `parent_of_missing_reader_reruns`: does not port. It pins that an absent key's dependency propagates to a parent memo; with absence as a value, the remainder is the port of `multi_function_chain`.
- `missing_key.rs:212` `read_outside_a_memo_then_set`: ports as "source.value outside a computation, then a reader created after the write".
- `missing_key.rs:233` `read_outside_a_memo_after_absent_node_exists`: ports as "source.value outside a computation leaves the readers' dependency".
- `remove.rs:52` `present_then_removed_reruns_and_returns_none`, `:77` `removed_then_set_reruns`, `:102` `removed_and_still_removed_does_not_rerun`, `:128` `remove_of_missing_key_does_not_rerun`, `:145` `remove_of_never_read_key_then_read`: do not port. They pin `remove_singleton`'s epoch rules (a remove of a present key advances the epoch, a remove of an absent key does not). The engine has no removal; a caller writes its absence value, which is a write like any other (the ports of `basic` and `overriding_same_source`).
- `try_get.rs:43` `set_of_never_set_key_reruns_reader`, `:64` `unrelated_set_does_not_rerun_reader_of_never_set_key`, `:83` `id_for_key_matches_the_id_set_returns`: do not port. They pin `try_get` of a never-set key and `id_for_key`; the engine has no keys.
- `garbage_collection.rs:46` `collected_reader_of_missing_key_sees_set`: ports as `disposal.test.ts` "a disposed node recomputes on its next pull and sees writes made after disposal". Delta: pico's LRU collects the reader; the test disposes it.
- `garbage_collection.rs:72` `retained_reader_of_missing_key_sees_set`, `:97` `remove_after_collection`: do not port. They pin that an LRU-retained reader keeps its dependency through `run_garbage_collection`; the engine has no collection, and a node not disposed keeps its edges (every other test).

`garbage_collection`:

- `basic_gc.rs:25` `basic_gc`, `inner_retained.rs:27` `inner_retained`, `multiple_calls.rs:27` `multiple_calls`, `outer_retained.rs:27` `outer_retained`, `retained.rs:26` `basic_retained`, `retained_and_in_lru.rs:26` `basic_retained`, `retained_clear.rs:26` `clear_retained`: do not port. They pin pico's collection policy: the LRU of top-level calls, its capacity, reachability from retained calls, and `retain` / `clear_retain`. The engine disposes on request and has no policy (The design, Disposal).

`params`:

- `memo_ref_never_cloned.rs:14` `memo_ref_never_cloned`, `other_param_cloned_on_execute.rs:19` `owned_param`, `:51` `borrowed_param`, `source_id_never_cloned.rs:15` `source_id_never_cloned`, `with_serialize.rs:16` `with_serialize`: do not port. They pin param interning and when pico clones a param; the engine has no params.

`tracking_field`:

- `correctness.rs:17` `tracking_field_correctness`: ports as `dependencies.test.ts` "source.value inside a computation records nothing". Delta: pico's untracked iteration of a `#[tracked]` map becomes a read of `source.value` inside a computation, which is the engine's untracked read (decision 10), and pico's tracked iteration becomes `readSource`.
- `efficiency.rs:23` `untracked_efficiency`, `:57` `tracked_inefficiency`: do not port. They pin the `#[tracked]` counter's granularity (a tracked read depends on the whole map); the engine has no tracked map, and a caller that wants per-key granularity holds one source per key, which the port of `unrelated_source_changes` covers.
- `never_written.rs:54` `iteration_before_first_write_then_insert_reruns`, `:79` `untracked_miss_then_tracked_then_insert_reruns`, `:91` `first_tracked_write_writes_counter_zero`: do not port. They pin the tracked counter's never-written singleton; the engine has neither.

### Test files

```text
libs/pico-js/test/
  recompute.test.ts      ports of chains, reuse, cutoff; marking and short circuit; pull order; diamond
  dependencies.test.ts   recorded order, branch changes, repeated reads, absence as a value, untracked reads
  errors.test.ts         a throwing computation leaves the graph's state unchanged
  disposal.test.ts       disposal
  degenerate.test.ts     degenerate nodes, sources, and values
  graph.test-d.ts        types
```

Every test file imports from `../src`, which is `src/index.ts`.

### `recompute.test.ts`

```ts
// from libs/pico-js/test/recompute.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  type DerivedNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';
import type { Status } from '../src/graph';

function statusOf(node: DerivedNode<unknown>): Status | 'Missing' {
  const memo = node.memo;
  return typeof memo === 'symbol' ? 'Missing' : memo.status;
}

describe('ported from crates/pico/tests', () => {
  // crates/pico/tests/basic.rs:14
  test('a derived node recomputes after its source is written', () => {
    const calls = { firstLetter: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const firstLetter = createDerivedNode((consumer) => {
      calls.firstLetter += 1;
      return readSource(consumer, input).charAt(0);
    }, tripleEquals);

    expect(pull(firstLetter)).toBe('a');
    expect(calls.firstLetter).toBe(1);

    writeSource(input, 'qwer');
    expect(pull(firstLetter)).toBe('q');
    expect(calls.firstLetter).toBe(2);
  });

  // crates/pico/tests/basic_multi_function_chain.rs:21
  test('a chain recomputes every node whose input changed', () => {
    const calls = { firstLetter: 0, capitalized: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const firstLetter = createDerivedNode((consumer) => {
      calls.firstLetter += 1;
      return readSource(consumer, input).charAt(0);
    }, tripleEquals);
    const capitalized = createDerivedNode((consumer) => {
      calls.capitalized += 1;
      return readDerived(consumer, firstLetter).toUpperCase();
    }, tripleEquals);

    expect(pull(capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1 });

    writeSource(input, 'qwer');
    expect(pull(capitalized)).toBe('Q');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2 });
  });

  // crates/pico/tests/basic_multi_function_chain.rs:48
  test('an equal value cuts off the node above it', () => {
    // The setup is the one in 'a chain recomputes every node whose input changed'.
    expect(pull(capitalized)).toBe('A');
    writeSource(input, 'alto');
    expect(pull(capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 1 });
  });

  // crates/pico/tests/basic_multi_function_chain.rs:75
  test('a top-level pull of an inner node, then of the node above it', () => {
    // The setup is the one in 'a chain recomputes every node whose input changed'.
    expect(pull(capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1 });

    writeSource(input, 'bsdf');
    expect(pull(firstLetter)).toBe('b');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 1 });
    expect(pull(capitalized)).toBe('B');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2 });

    writeSource(input, 'balt');
    expect(pull(capitalized)).toBe('B');
    expect(calls).toEqual({ firstLetter: 3, capitalized: 2 });
  });

  // crates/pico/tests/partial_reuse_single_chain.rs:15
  test('a node pulled before its reader is not computed again for the reader', () => {
    const calls = { firstLetter: 0, exclaimed: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const firstLetter = createDerivedNode((consumer) => {
      calls.firstLetter += 1;
      return readSource(consumer, input).charAt(0);
    }, tripleEquals);
    const exclaimed = createDerivedNode((consumer) => {
      calls.exclaimed += 1;
      return `${readDerived(consumer, firstLetter)}!`;
    }, tripleEquals);

    expect(pull(firstLetter)).toBe('a');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 0 });
    expect(pull(exclaimed)).toBe('a!');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 1 });
  });

  // crates/pico/tests/partial_reuse_multi_function.rs:16
  test('two readers share one computation of their dependency', () => {
    // firstLetter and exclaimed are the nodes above.
    const questioned = createDerivedNode((consumer) => {
      calls.questioned += 1;
      return `${readDerived(consumer, firstLetter)}?`;
    }, tripleEquals);

    expect(pull(exclaimed)).toBe('a!');
    expect(pull(questioned)).toBe('a?');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 1, questioned: 1 });
  });

  // crates/pico/tests/side_chain.rs:16
  test('a dependency with no inputs is not computed again when its reader recomputes', () => {
    const calls = { firstLetter: 0, capitalized: 0, unchanged: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const firstLetter = createDerivedNode((consumer) => {
      calls.firstLetter += 1;
      return readSource(consumer, input).charAt(0);
    }, tripleEquals);
    const unchanged = createDerivedNode(() => {
      calls.unchanged += 1;
      return 'this node is computed once';
    }, tripleEquals);
    const capitalized = createDerivedNode((consumer) => {
      calls.capitalized += 1;
      readDerived(consumer, unchanged);
      return readDerived(consumer, firstLetter).toUpperCase();
    }, tripleEquals);

    expect(pull(capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1, unchanged: 1 });
    expect(pull(capitalized)).toBe('A');
    expect(calls).toEqual({ firstLetter: 1, capitalized: 1, unchanged: 1 });

    writeSource(input, 'qwer');
    expect(pull(capitalized)).toBe('Q');
    expect(calls).toEqual({ firstLetter: 2, capitalized: 2, unchanged: 1 });
  });

  // crates/pico/tests/unrelated_source_change.rs:14
  test('a write to another source runs nothing', () => {
    const calls = { firstLetter: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const other = createSourceNode('isograph', tripleEquals);
    const firstLetter = createDerivedNode((consumer) => {
      calls.firstLetter += 1;
      return readSource(consumer, input).charAt(0);
    }, tripleEquals);

    expect(pull(firstLetter)).toBe('a');
    writeSource(other, 'pico');
    expect(pull(firstLetter)).toBe('a');
    expect(calls.firstLetter).toBe(1);
  });

  // crates/pico/tests/return_value_not_cloned.rs:14
  test('a pull returns the stored value object', () => {
    const calls = { letter: 0 };
    const input = createSourceNode('asdf', tripleEquals);
    const letter = createDerivedNode(
      (consumer) => {
        calls.letter += 1;
        return { letter: readSource(consumer, input).charAt(0) };
      },
      (previous, next) => previous.letter === next.letter,
    );

    const first = pull(letter);
    expect(pull(letter)).toBe(first);

    writeSource(input, 'alto');
    expect(pull(letter)).toBe(first);
    expect(calls.letter).toBe(2);

    writeSource(input, 'qwer');
    const second = pull(letter);
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
    const calls = { bestBook: 0 };
    const table = createSourceNode<ReadonlyArray<Economist>>(
      [
        { name: 'John Maynard Keynes', bestBook: 'General theory' },
        {
          name: 'Milton Friedman',
          bestBook: 'A monetary history of the United States',
        },
      ],
      tripleEquals,
    );
    const keynes = createDerivedNode(
      (consumer) =>
        readSource(consumer, table).find(
          (economist) => economist.name === 'John Maynard Keynes',
        ),
      economistsEqual,
    );
    const bestBook = createDerivedNode((consumer) => {
      calls.bestBook += 1;
      return readDerived(consumer, keynes)?.bestBook;
    }, tripleEquals);

    expect(pull(bestBook)).toBe('General theory');

    writeSource(table, [
      { name: 'John Maynard Keynes', bestBook: 'General theory' },
      { name: 'Adam Smith', bestBook: 'The wealth of nations' },
    ]);
    expect(pull(bestBook)).toBe('General theory');
    expect(calls.bestBook).toBe(1);
  });

  // crates/pico/tests/overriding_same_source.rs:14
  test('writing an equal value', () => {
    const calls = { firstLetter: 0, exclaimed: 0 };
    // firstLetter reads input = 'asdf' and exclaimed reads firstLetter, as above.
    expect(pull(exclaimed)).toBe('a!');

    writeSource(input, 'asdf');
    expect(pull(exclaimed)).toBe('a!');
    expect(calls).toEqual({ firstLetter: 1, exclaimed: 1 });
  });
});

describe('marking and the short circuit', () => {
  test('a write runs no computation', () => {
    const calls = { doubled: 0, label: 0 };
    const count = createSourceNode(1, tripleEquals);
    const doubled = createDerivedNode((consumer) => {
      calls.doubled += 1;
      return readSource(consumer, count) * 2;
    }, tripleEquals);
    const label = createDerivedNode((consumer) => {
      calls.label += 1;
      return `${readDerived(consumer, doubled)}`;
    }, tripleEquals);
    pull(label);

    writeSource(count, 2);
    writeSource(count, 3);
    expect(calls).toEqual({ doubled: 1, label: 1 });

    expect(pull(label)).toBe('6');
    expect(calls).toEqual({ doubled: 2, label: 2 });
  });

  test('a direct reader of a written source recomputes; a reader above it recomputes only when that value changed', () => {
    const calls = { parity: 0, label: 0 };
    const count = createSourceNode(1, tripleEquals);
    const parity = createDerivedNode((consumer) => {
      calls.parity += 1;
      return readSource(consumer, count) % 2 === 0 ? 'even' : 'odd';
    }, tripleEquals);
    const label = createDerivedNode((consumer) => {
      calls.label += 1;
      return `the count is ${readDerived(consumer, parity)}`;
    }, tripleEquals);
    const first = pull(label);

    writeSource(count, 3);
    expect(pull(label)).toBe(first);
    expect(calls).toEqual({ parity: 2, label: 1 });

    writeSource(count, 4);
    expect(pull(label)).toBe('the count is even');
    expect(calls).toEqual({ parity: 3, label: 2 });
  });

  test('the short circuit holds through every level above the node that came back equal', () => {
    const calls = { clamped: 0, doubled: 0, label: 0 };
    const count = createSourceNode(20, tripleEquals);
    const clamped = createDerivedNode((consumer) => {
      calls.clamped += 1;
      return Math.min(readSource(consumer, count), 10);
    }, tripleEquals);
    const doubled = createDerivedNode((consumer) => {
      calls.doubled += 1;
      return readDerived(consumer, clamped) * 2;
    }, tripleEquals);
    const label = createDerivedNode((consumer) => {
      calls.label += 1;
      return `${readDerived(consumer, doubled)}`;
    }, tripleEquals);
    expect(pull(label)).toBe('20');

    writeSource(count, 30);
    expect(pull(label)).toBe('20');
    expect(calls).toEqual({ clamped: 2, doubled: 1, label: 1 });
  });

  test('a diamond marks both readers Dirty and the bottom Check, and computes each node once per write', () => {
    const calls = { left: 0, right: 0, bottom: 0 };
    const count = createSourceNode(1, tripleEquals);
    const left = createDerivedNode((consumer) => {
      calls.left += 1;
      return readSource(consumer, count) + 1;
    }, tripleEquals);
    const right = createDerivedNode((consumer) => {
      calls.right += 1;
      return readSource(consumer, count) * 2;
    }, tripleEquals);
    const bottom = createDerivedNode((consumer) => {
      calls.bottom += 1;
      return readDerived(consumer, left) + readDerived(consumer, right);
    }, tripleEquals);
    expect(pull(bottom)).toBe(4);

    writeSource(count, 2);
    expect([statusOf(left), statusOf(right), statusOf(bottom)]).toEqual(['Dirty', 'Dirty', 'Check']);
    expect(pull(bottom)).toBe(7);
    expect(pull(left)).toBe(3);
    expect(pull(right)).toBe(4);
    expect(calls).toEqual({ left: 2, right: 2, bottom: 2 });
  });

  test('marking a chain of 100,000 nodes marks the reader Dirty and every node above it Check', () => {
    const count = createSourceNode(0, tripleEquals);
    const chain: ReadonlyArray<DerivedNode<number>> = Array.from({ length: 100_000 }, (_, index) =>
      createDerivedNode((consumer) => {
        const below = chain[index - 1];
        return below === undefined ? readSource(consumer, count) : readDerived(consumer, below) + 1;
      }, tripleEquals),
    );
    expect(chain.map((node) => pull(node)).at(-1)).toBe(99_999);

    expect(() => writeSource(count, 1)).not.toThrow();
    expect(chain.map(statusOf)).toEqual(chain.map((_, index) => (index === 0 ? 'Dirty' : 'Check')));
    expect(chain.map((node) => pull(node)).at(-1)).toBe(100_000);
  });

  test('top-level pull order changes neither values nor what runs', () => {
    const build = () => {
      const calls = { sum: 0, parity: 0, label: 0 };
      const a = createSourceNode(1, tripleEquals);
      const b = createSourceNode(2, tripleEquals);
      const sum = createDerivedNode((consumer) => {
        calls.sum += 1;
        return readSource(consumer, a) + readSource(consumer, b);
      }, tripleEquals);
      const parity = createDerivedNode((consumer) => {
        calls.parity += 1;
        return readDerived(consumer, sum) % 2 === 0 ? 'even' : 'odd';
      }, tripleEquals);
      const label = createDerivedNode((consumer) => {
        calls.label += 1;
        return `the sum is ${readDerived(consumer, parity)}`;
      }, tripleEquals);
      pull(label);
      writeSource(a, 3);
      return { calls, sum, label };
    };
    const sumFirst = build();
    const labelFirst = build();

    const sumThenLabel = [pull(sumFirst.sum), pull(sumFirst.label)];
    const labelThenSum = [pull(labelFirst.label), pull(labelFirst.sum)];

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
// from libs/pico-js/test/dependencies.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';

const MISSING: unique symbol = Symbol('missing');

describe('recorded dependencies', () => {
  test('verification stops at the first dependency that changed, in recorded order', () => {
    const calls = { gate: 0, other: 0, node: 0 };
    const x = createSourceNode(2, tripleEquals);
    const y = createSourceNode(10, tripleEquals);
    const gate = createDerivedNode((consumer) => {
      calls.gate += 1;
      return readSource(consumer, x) % 2 === 0 ? 'even' : 'odd';
    }, tripleEquals);
    const other = createDerivedNode((consumer) => {
      calls.other += 1;
      return readSource(consumer, y) * 2;
    }, tripleEquals);
    const node = createDerivedNode((consumer) => {
      calls.node += 1;
      return readDerived(consumer, gate) === 'even' ? readDerived(consumer, other) : 0;
    }, tripleEquals);
    expect(pull(node)).toBe(20);

    writeSource(x, 3);
    writeSource(y, 11);
    expect(pull(node)).toBe(0);
    expect(calls).toEqual({ gate: 2, other: 1, node: 2 });

    expect(pull(other)).toBe(22);
    expect(calls.other).toBe(2);
  });

  test('a branch change replaces the dependencies', () => {
    const calls = { pick: 0 };
    const useA = createSourceNode(true, tripleEquals);
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const pick = createDerivedNode((consumer) => {
      calls.pick += 1;
      return readSource(consumer, useA) ? readSource(consumer, a) : readSource(consumer, b);
    }, tripleEquals);
    expect(pull(pick)).toBe(1);

    writeSource(useA, false);
    expect(pull(pick)).toBe(2);
    expect(calls.pick).toBe(2);

    writeSource(a, 5);
    expect(pull(pick)).toBe(2);
    expect(calls.pick).toBe(2);

    writeSource(b, 7);
    expect(pull(pick)).toBe(7);
    expect(calls.pick).toBe(3);
  });

  test('a derived node read twice is computed once per write', () => {
    const calls = { inner: 0, outer: 0 };
    const count = createSourceNode(1, tripleEquals);
    const inner = createDerivedNode((consumer) => {
      calls.inner += 1;
      return readSource(consumer, count) * 2;
    }, tripleEquals);
    const outer = createDerivedNode((consumer) => {
      calls.outer += 1;
      return readDerived(consumer, inner) + readDerived(consumer, inner);
    }, tripleEquals);
    expect(pull(outer)).toBe(4);

    writeSource(count, 2);
    expect(pull(outer)).toBe(8);
    expect(calls).toEqual({ inner: 2, outer: 2 });
  });

  // crates/pico/tests/absent_source/missing_key.rs:164
  test('a computation that reads one source twice', () => {
    const calls = { twice: 0 };
    const config = createSourceNode('a', tripleEquals);
    const twice = createDerivedNode((consumer) => {
      calls.twice += 1;
      return `${readSource(consumer, config)}${readSource(consumer, config)}`;
    }, tripleEquals);
    expect(pull(twice)).toBe('aa');

    writeSource(config, 'b');
    expect(pull(twice)).toBe('bb');
    writeSource(config, 'c');
    expect(pull(twice)).toBe('cc');
    expect(calls.twice).toBe(3);
  });

  // crates/pico/tests/absent_source/missing_key.rs:57
  test('a source that holds the caller’s absence value', () => {
    const calls = { name: 0 };
    const config = createSourceNode<string | typeof MISSING>(MISSING, tripleEquals);
    const name = createDerivedNode((consumer) => {
      calls.name += 1;
      const value = readSource(consumer, config);
      return value === MISSING ? null : value;
    }, tripleEquals);
    expect(pull(name)).toBe(null);

    writeSource(config, 'a');
    expect(pull(name)).toBe('a');
    expect(calls.name).toBe(2);
  });

  // crates/pico/tests/absent_source/missing_key.rs:212
  test('source.value outside a computation, then a reader created after the write', () => {
    const calls = { name: 0 };
    const config = createSourceNode<string | typeof MISSING>(MISSING, tripleEquals);
    expect(config.value).toBe(MISSING);

    writeSource(config, 'a');
    const name = createDerivedNode((consumer) => {
      calls.name += 1;
      const value = readSource(consumer, config);
      return value === MISSING ? null : value;
    }, tripleEquals);
    expect(pull(name)).toBe('a');
    expect(calls.name).toBe(1);
    expect(config.value).toBe('a');
  });

  // crates/pico/tests/absent_source/missing_key.rs:233
  test('source.value outside a computation leaves the readers’ dependency', () => {
    // config and name are the nodes of the previous test.
    expect(pull(name)).toBe(null);
    expect(config.value).toBe(MISSING);

    writeSource(config, 'a');
    expect(pull(name)).toBe('a');
    expect(calls.name).toBe(2);
  });

  // crates/pico/tests/tracking_field/correctness.rs:17
  test('source.value inside a computation records nothing', () => {
    const items = createSourceNode<ReadonlyArray<string>>(['asdf'], tripleEquals);
    const untrackedCount = createDerivedNode(() => items.value.length, tripleEquals);
    const trackedCount = createDerivedNode(
      (consumer) => readSource(consumer, items).length,
      tripleEquals,
    );
    expect(pull(untrackedCount)).toBe(1);

    writeSource(items, ['asdf', 'isograph']);
    expect(pull(untrackedCount)).toBe(1);
    expect(pull(trackedCount)).toBe(2);

    writeSource(items, ['asdf', 'isograph', 'database']);
    expect(pull(trackedCount)).toBe(3);
  });

  test('a recompute that reads the same nodes keeps its edges', () => {
    const calls = { node: 0 };
    const trigger = createSourceNode(0, tripleEquals);
    const useA = createSourceNode(true, tripleEquals);
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(10, tripleEquals);
    const node = createDerivedNode((consumer) => {
      calls.node += 1;
      readSource(consumer, trigger);
      return readSource(consumer, useA) ? readSource(consumer, a) : readSource(consumer, b);
    }, tripleEquals);
    expect(pull(node)).toBe(1);

    writeSource(trigger, 1);
    expect(pull(node)).toBe(1);
    expect(calls.node).toBe(2);

    writeSource(a, 2);
    expect(pull(node)).toBe(2);
    expect(calls.node).toBe(3);

    writeSource(useA, false);
    expect(pull(node)).toBe(10);
    writeSource(a, 3);
    expect(pull(node)).toBe(10);
    expect(calls.node).toBe(4);
  });

  test('pull inside a computation records nothing', () => {
    const calls = { outer: 0 };
    const count = createSourceNode(1, tripleEquals);
    const doubled = createDerivedNode((consumer) => readSource(consumer, count) * 2, tripleEquals);
    const outer = createDerivedNode(() => {
      calls.outer += 1;
      return pull(doubled);
    }, tripleEquals);
    expect(pull(outer)).toBe(2);

    writeSource(count, 2);
    expect(pull(outer)).toBe(2);
    expect(pull(doubled)).toBe(4);
    expect(calls.outer).toBe(1);
  });
});
```

### `errors.test.ts`

A computation must not throw (Throws). The one test asserts the non-corruption guarantee, observed through values and call counts after the throw; it asserts nothing about the node that threw.

```ts
// from libs/pico-js/test/errors.test.ts
import { expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';

test('a throwing computation leaves the graph state unchanged', () => {
  const calls = { scaled: 0, parent: 0 };
  const count = createSourceNode(1, tripleEquals);
  const scaled = createDerivedNode((consumer) => {
    calls.scaled += 1;
    const value = readSource(consumer, count);
    if (value < 0) {
      throw new Error('negative');
    }
    return value * 10;
  }, tripleEquals);
  const parent = createDerivedNode((consumer) => {
    calls.parent += 1;
    return `${readDerived(consumer, scaled)}`;
  }, tripleEquals);
  expect(pull(parent)).toBe('10');

  writeSource(count, -1);
  expect(() => pull(parent)).toThrow('negative');

  writeSource(count, 2);
  expect(pull(parent)).toBe('20');
  expect(calls).toEqual({ scaled: 3, parent: 2 });

  writeSource(count, 3);
  expect(pull(parent)).toBe('30');
  expect(calls).toEqual({ scaled: 4, parent: 3 });
});
```

The `3` write recomputes both nodes, which asserts that the throwing run left `scaled`'s memo in the `dependents` set of `count` and `parent`'s memo in that of `scaled`.

### `disposal.test.ts`

```ts
// from libs/pico-js/test/disposal.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';

describe('disposeDerivedNode (open decision 9 (a))', () => {
  test('the first pull, and the first pull after disposal, call no equals', () => {
    const calls = { equals: 0 };
    const count = createSourceNode(1, tripleEquals);
    const node = createDerivedNode(
      (consumer) => readSource(consumer, count),
      (previous, next) => {
        calls.equals += 1;
        return previous === next;
      },
    );
    expect(pull(node)).toBe(1);
    expect(calls.equals).toBe(0);

    writeSource(count, 2);
    expect(pull(node)).toBe(2);
    expect(calls.equals).toBe(1);

    disposeDerivedNode(node);
    expect(pull(node)).toBe(2);
    expect(calls.equals).toBe(1);
  });

  test('marks the dependents Dirty, and their next pull recomputes the disposed node', () => {
    const calls = { scaled: 0, parent: 0 };
    const count = createSourceNode(1, tripleEquals);
    const scaled = createDerivedNode((consumer) => {
      calls.scaled += 1;
      return readSource(consumer, count) * 10;
    }, tripleEquals);
    const parent = createDerivedNode((consumer) => {
      calls.parent += 1;
      return `${readDerived(consumer, scaled)}`;
    }, tripleEquals);
    expect(pull(parent)).toBe('10');

    disposeDerivedNode(scaled);
    expect(calls).toEqual({ scaled: 1, parent: 1 });
    expect(pull(parent)).toBe('10');
    expect(calls).toEqual({ scaled: 2, parent: 2 });
  });

  // crates/pico/tests/absent_source/garbage_collection.rs:46
  test('a disposed node recomputes on its next pull and sees writes made after disposal', () => {
    // count and scaled are the nodes above.
    expect(pull(scaled)).toBe(10);

    disposeDerivedNode(scaled);
    writeSource(count, 2);
    expect(pull(scaled)).toBe(20);
    expect(calls.scaled).toBe(2);
  });

  test('a dependent that stops reading a disposed node is detached from it', () => {
    const calls = { scaled: 0, parent: 0 };
    const count = createSourceNode(1, tripleEquals);
    const useScaled = createSourceNode(true, tripleEquals);
    // scaled is the node above.
    const parent = createDerivedNode((consumer) => {
      calls.parent += 1;
      return readSource(consumer, useScaled) ? readDerived(consumer, scaled) : 0;
    }, tripleEquals);
    expect(pull(parent)).toBe(10);

    disposeDerivedNode(scaled);
    writeSource(useScaled, false);
    expect(pull(parent)).toBe(0);
    expect(calls).toEqual({ scaled: 1, parent: 2 });

    expect(pull(scaled)).toBe(10);
    writeSource(count, 2);
    expect(pull(scaled)).toBe(20);
    expect(pull(parent)).toBe(0);
    expect(calls).toEqual({ scaled: 3, parent: 2 });
  });

  test('disposing a never-computed node, or a node twice, changes nothing else', () => {
    const calls = { node: 0 };
    const count = createSourceNode(1, tripleEquals);
    const node = createDerivedNode((consumer) => {
      calls.node += 1;
      return readSource(consumer, count);
    }, tripleEquals);

    disposeDerivedNode(node);
    disposeDerivedNode(node);
    expect(calls.node).toBe(0);
    expect(pull(node)).toBe(1);
    expect(calls.node).toBe(1);
  });
});
```

### `degenerate.test.ts`

```ts
// from libs/pico-js/test/degenerate.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  markSourceDirty,
  objectIs,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';

describe('degenerate cases', () => {
  test('a derived node with no dependencies is computed once, never marked, and changed only by disposal', () => {
    const calls = { constant: 0, parity: 0, reader: 0 };
    const count = createSourceNode(1, tripleEquals);
    const constant = createDerivedNode(() => {
      calls.constant += 1;
      return 'fixed';
    }, tripleEquals);
    const parity = createDerivedNode((consumer) => {
      calls.parity += 1;
      return readSource(consumer, count) % 2 === 0 ? 'even' : 'odd';
    }, tripleEquals);
    const reader = createDerivedNode((consumer) => {
      calls.reader += 1;
      return `${readDerived(consumer, constant)} ${readDerived(consumer, parity)}`;
    }, tripleEquals);
    expect(pull(consumer)).toBe('fixed odd');

    writeSource(count, 3);
    expect(pull(consumer)).toBe('fixed odd');
    expect(calls).toEqual({ constant: 1, parity: 2, reader: 1 });

    writeSource(count, 4);
    expect(pull(consumer)).toBe('fixed even');
    expect(calls).toEqual({ constant: 1, parity: 3, reader: 2 });

    disposeDerivedNode(constant);
    expect(pull(consumer)).toBe('fixed even');
    expect(calls).toEqual({ constant: 2, parity: 3, reader: 3 });
  });

  test('a source with no dependents', () => {
    const count = createSourceNode(1, tripleEquals);
    writeSource(count, 2);
    markSourceDirty(count);
    expect(count.value).toBe(2);
  });

  test('a pull of a never-computed node computes it once', () => {
    const calls = { node: 0 };
    const count = createSourceNode(1, tripleEquals);
    const node = createDerivedNode((consumer) => {
      calls.node += 1;
      return readSource(consumer, count);
    }, tripleEquals);
    expect(calls.node).toBe(0);
    expect(pull(node)).toBe(1);
    expect(pull(node)).toBe(1);
    expect(calls.node).toBe(1);
  });

  test("the source's equals decides whether a write marks", () => {
    const calls = { strictReader: 0, structuralReader: 0 };
    const sameElements = (previous: ReadonlyArray<number>, next: ReadonlyArray<number>) =>
      previous.length === next.length && previous.every((element, index) => element === next[index]);
    const strictItems = createSourceNode<ReadonlyArray<number>>([1, 2], tripleEquals);
    const structuralItems = createSourceNode<ReadonlyArray<number>>([1, 2], sameElements);
    const strictReader = createDerivedNode((consumer) => {
      calls.strictReader += 1;
      return readSource(consumer, strictItems).length;
    }, tripleEquals);
    const structuralReader = createDerivedNode((consumer) => {
      calls.structuralReader += 1;
      return readSource(consumer, structuralItems).length;
    }, tripleEquals);
    expect(pull(strictReader)).toBe(2);
    expect(pull(structuralReader)).toBe(2);

    writeSource(strictItems, strictItems.value);
    writeSource(structuralItems, structuralItems.value);
    expect(pull(strictReader)).toBe(2);
    expect(pull(structuralReader)).toBe(2);
    expect(calls).toEqual({ strictReader: 1, structuralReader: 1 });

    writeSource(strictItems, [1, 2]);
    writeSource(structuralItems, [1, 2]);
    expect(pull(strictReader)).toBe(2);
    expect(pull(structuralReader)).toBe(2);
    expect(calls).toEqual({ strictReader: 2, structuralReader: 1 });

    writeSource(structuralItems, [1, 2, 3]);
    expect(pull(structuralReader)).toBe(3);
    expect(calls).toEqual({ strictReader: 2, structuralReader: 2 });
  });

  test('markSourceDirty after the value object changed in place', () => {
    const scores = createSourceNode(new Map([['a', 1]]), tripleEquals);
    const score = createDerivedNode(
      (consumer) => readSource(consumer, scores).get('a'),
      tripleEquals,
    );
    expect(pull(score)).toBe(1);

    scores.value.set('a', 2);
    expect(pull(score)).toBe(1);
    markSourceDirty(scores);
    expect(pull(score)).toBe(2);
  });

  test('undefined is a value, distinct from never computed', () => {
    const calls = { nothing: 0, parent: 0 };
    const count = createSourceNode(1, tripleEquals);
    const nothing = createDerivedNode((consumer) => {
      calls.nothing += 1;
      readSource(consumer, count);
      return undefined;
    }, tripleEquals);
    const parent = createDerivedNode((consumer) => {
      calls.parent += 1;
      return readDerived(consumer, nothing) === undefined;
    }, tripleEquals);
    expect(pull(parent)).toBe(true);
    expect(pull(nothing)).toBe(undefined);

    writeSource(count, 2);
    expect(pull(parent)).toBe(true);
    expect(calls).toEqual({ nothing: 2, parent: 1 });
  });

  test('a verifying reader does not rerun over a NaN its dependency recomputed, under either equals', () => {
    const calls = { strictParent: 0, objectIsParent: 0 };
    const count = createSourceNode(1, tripleEquals);
    const strictNaN = createDerivedNode(
      (consumer) => readSource(consumer, count) * Number.NaN,
      tripleEquals,
    );
    const objectIsNaN = createDerivedNode(
      (consumer) => readSource(consumer, count) * Number.NaN,
      objectIs,
    );
    const strictParent = createDerivedNode((consumer) => {
      calls.strictParent += 1;
      return Number.isNaN(readDerived(consumer, strictNaN));
    }, tripleEquals);
    const objectIsParent = createDerivedNode((consumer) => {
      calls.objectIsParent += 1;
      return Number.isNaN(readDerived(consumer, objectIsNaN));
    }, tripleEquals);
    expect(pull(strictParent)).toBe(true);
    expect(pull(objectIsParent)).toBe(true);

    writeSource(count, 2);
    expect(pull(strictParent)).toBe(true);
    expect(pull(objectIsParent)).toBe(true);
    expect(calls).toEqual({ strictParent: 1, objectIsParent: 1 });
  });

  test('the supplied equals decides cutoff for 0 and -0', () => {
    const sign = createSourceNode(1, tripleEquals);
    const strictZero = createDerivedNode((consumer) => readSource(consumer, sign) * 0, tripleEquals);
    const objectIsZero = createDerivedNode((consumer) => readSource(consumer, sign) * 0, objectIs);
    const describeZero = (value: number) => (Object.is(value, -0) ? 'negative zero' : 'zero');
    const strictParent = createDerivedNode(
      (consumer) => describeZero(readDerived(consumer, strictZero)),
      tripleEquals,
    );
    const objectIsParent = createDerivedNode(
      (consumer) => describeZero(readDerived(consumer, objectIsZero)),
      tripleEquals,
    );
    expect(pull(strictParent)).toBe('zero');
    expect(pull(objectIsParent)).toBe('zero');

    writeSource(sign, -1);
    expect(pull(strictParent)).toBe('zero');
    expect(pull(objectIsParent)).toBe('negative zero');
  });
});
```

### `graph.test-d.ts`

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  type DerivedNode,
  pull,
  readDerived,
  readSource,
  tripleEquals,
  writeSource,
} from '../src';

test('readSource, readDerived, and pull return the node’s value type', () => {
  const count = createSourceNode(1, tripleEquals);
  const doubled = createDerivedNode((consumer) => {
    expectTypeOf(readSource(consumer, count)).toEqualTypeOf<number>();
    return readSource(consumer, count) * 2;
  }, tripleEquals);
  const label = createDerivedNode((consumer) => {
    expectTypeOf(readDerived(consumer, doubled)).toEqualTypeOf<number>();
    return `${readDerived(consumer, doubled)}`;
  }, tripleEquals);
  expectTypeOf(pull(label)).toEqualTypeOf<string>();
});

test('a derived node of any value type is a DerivedNode<unknown>', () => {
  expectTypeOf<DerivedNode<number>>().toMatchTypeOf<DerivedNode<unknown>>();
  expectTypeOf<DerivedNode<ReadonlyArray<string>>>().toMatchTypeOf<DerivedNode<unknown>>();
});

test('a source node is not read with readDerived', () => {
  const count = createSourceNode(1, tripleEquals);
  createDerivedNode((consumer) => {
    // @ts-expect-error readDerived does not take a source node.
    return readDerived(consumer, count);
  }, tripleEquals);
});

test('writes and equalities are typed by the node', () => {
  const count = createSourceNode(1, tripleEquals);
  // @ts-expect-error writeSource does not write a string to a number source.
  writeSource(count, 'one');
  createDerivedNode(
    () => 1,
    // @ts-expect-error a number node does not take an equality over strings.
    (previous: string, next: string) => previous === next,
  );
});
```

## The package

### Location and files

The engine is a workspace library at `libs/pico-js`, covered by `pnpm-workspace.yaml`'s `libs/*`. Its build and test setup copies `libs/isograph-runtime`'s without the Babel plugin, which only the runtime's fixtures need.

```text
libs/pico-js/
  package.json
  README.md                 The package README
  tsconfig.json             references tsconfig.package.json and tsconfig.test.json
  tsconfig.package.json     extends ../../tsconfig.build.json, includes src/**/*.ts
  tsconfig.test.json        extends ../../tsconfig.build.json, includes test/**/*.ts
  tsdown.config.ts          merges ../../tsdown.config.ts, dts.build true
  vitest.config.ts          typecheck enabled with tsconfig.test.json
  src/equality.ts
  src/graph.ts
  src/index.ts
  test/*.test.ts, test/graph.test-d.ts   Tests
```

Adding the package changes `pnpm-lock.yaml` (a new importer, written by `pnpm install`), which the same commit includes.

### `package.json`

```json
// from libs/pico-js/package.json
{
  "name": "@isograph/pico-js",
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
    "directory": "libs/pico-js"
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
// from libs/pico-js/tsconfig.test.json
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
// from libs/pico-js/vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { typecheck: { enabled: true, tsconfig: './tsconfig.test.json' } },
});
```

Deltas of `tsconfig.test.json` from the runtime's: no `paths` entry and no `fixtures/**/*.ts`, because the tests import `../src` and there are no fixtures. Deltas of `vitest.config.ts`: no Babel plugin. With `typecheck.enabled`, vitest type-checks `graph.test-d.ts` as the runtime's `read.test-d.ts` is.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`; every type field is `readonly` except `SourceNode.value`, `DerivedNode.memo`, `DerivedNode.sourceReads`, `DerivedNode.derivedReads`, and the fields of `Memo`, which the engine writes; arrays are `ReadonlyArray` except the dependency lists and the reads buffers, which the engine reuses as buffers (Pull). The engine has no class (decision 7), so the ES private field rule does not apply. oxlint's `switch-exhaustiveness-check` and `strict-boolean-expressions` (`.oxlintrc.json`) hold for every `switch` and condition in the body.

### Pre-commit hooks

`.pre-commit-config.yaml` gains two hooks, after `disposable-types-typecheck`:

```yaml
# from .pre-commit-config.yaml
      - id: pico-js-test
        name: engine test
        entry: pnpm --filter @isograph/pico-js test
        language: system
        files: '^libs/pico-js/'
        pass_filenames: false
      - id: pico-js-typecheck
        name: engine typecheck
        entry: pnpm --filter @isograph/pico-js tsc
        language: system
        files: '^(libs/pico-js/|tsconfig\.build\.json$)'
        pass_filenames: false
```

`prettier` and `oxlint` already cover every file in the workspace. When the Isograph runtime gains its dependency on the engine (pico-runtime.md core change 5), the runtime's three hooks also match the engine's directory:

```yaml
# from .pre-commit-config.yaml
      # runtime-dependencies-build and runtime-typecheck, files:
        files: '^(libs/isograph-runtime/|libs/isograph-disposable-types/|libs/pico-js/|tsconfig\.build\.json$)'
      # runtime-test, files:
        files: '^libs/(isograph-runtime|isograph-disposable-types|pico-js)/'
```

### CI

`.github/workflows/js.yml`'s `babel-plugin` job runs `pnpm compile-libs`, which turbo runs for every workspace package with a `compile-libs` script; turbo's `compile-libs` task depends on `tsc` (`turbo.json`), so the engine is type-checked and built there with no change. The job gains one step after "Test @isograph/disposable-types":

```yaml
# from .github/workflows/js.yml
      - name: Test @isograph/pico-js
        run: pnpm --filter @isograph/pico-js test
```

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal). pico-runtime.md core change 4 ships the package before its first workspace caller on that ground; core change 5 is the first caller of the source functions and core change 8 of the derived-node functions. The package ships alone, as its own change (decision 11).

## What this doc changes in pico-runtime.md

pico-runtime.md holds no copy of the engine. It references this doc by section for every engine type, function body, test, and package file, and it specifies only the Isograph layer and the calls that layer makes into the engine (pico-runtime.md, The engine). Its core change 4 implements this doc. The engine behaviors pico-runtime.md relies on are this doc's body:

- The engine has no write-during-computation check and no graph object; a computation or an equality that writes breaks the contract and leaves values silently stale (Writes during a computation, decision 2). pico-runtime.md's `callResolver` catches every throw of a resolver into `ResolverThrew` (pico-runtime.md open decision 30).
- Marking marks the direct dependents `Dirty` in one loop and walks a work list for the transitive ones, in `markDependentsDirty` (Mark).
- A computation is synchronous, and the node it receives is used only as the first argument of `readSource` and `readDerived` while it runs; any other use breaks the contract, and the engine does not check it (The consumer, decision 2).
- Disposal resets the node to never computed (Disposal, open decision 9).
- A computation and an equality must not throw, and for a computation's throw the engine guarantees only that its own state is not corrupted (Throws, decision 3).
- The derived nodes form a directed acyclic graph, and the engine has no cycle check (Cycles, decision 4).
- Pull depth is a contract on the caller; a pull that overflows the call stack throws `RangeError`, after which the engine does not guarantee its state; marking is iterative (Pull, decision 12). pico-runtime.md relies on client field nesting to bound it.
- The engine has no default equality; `createSourceNode` and `createDerivedNode` take a required `equals` (decisions 6 and 8). pico-runtime.md's store passes `isUnchanged(key)` to each source node and `resolverResultsEqual(artifact, tripleEquals)` to each resolver node.
- A read of `source.value` or a `pull` inside a computation is an untracked read (decision 10).

pico-runtime.md's open decisions that restate this doc's state the same status: its 2 is this doc's 2; its 3 is 3; 8 is 4; 18 is 1; 19 is 5; 20 is 6; 21 is 7; 23 is 8; 24 is 9; 25 is 10; 26 is 11. When one of these is decided, both docs change in one commit.

## Open decisions

### 1. The package's name (decided)

The package is `@isograph/pico-js` and its directory is `libs/pico-js` (pico-runtime.md open decision 18). The `-js` suffix keeps the npm name distinct from the Rust crate `pico` (`crates/pico`), which the engine ports. The directory matches the existing `tsconfig.build.json` mapping `"@isograph/*": ["./libs/*"]` (`tsconfig.build.json:22`), which maps `@isograph/pico-js` to `libs/pico-js`. Whether the name is free on the npm registry was not verified, because the registry was not reachable offline.

### 2. A write during a computation (decided)

Decided by the user: no guard and no database object. The engine gives up no performance to prevent misuse, and misuse is a documented contract (Goal). A computation and an equality must not write a source or dispose a node; a violation leaves the cache silently stale (Writes during a computation), and the engine runs no check to catch it. There is no `Graph`: every public function takes the node it acts on (`writeSource(source, value)`, `markSourceDirty(source)`, `pull(node)`, `disposeDerivedNode(node)`), and the two read functions also take the node being computed as `consumer: DerivedNode<unknown>` (`readSource(consumer, source)`, `readDerived(consumer, node)`). Deviation from pico, which rejects every write while a memo runs (`assert_empty_dependency_stack`, `crates/pico/src/database.rs:193-198`) and passes the database to every call (`Database::set`, `:151`): the check costs a counter raised and lowered around every computation and every equality and a test on every write, and the database parameter exists in the engine only to carry that counter.

### 3. A throw out of a computation or an equality (decided)

A computation and an equality must not throw; a throw is a programmer error (Throws). For a computation's throw, the engine guarantees only that its own state is not corrupted, which holds because the throw leaves before any state is written (pico-runtime.md open decision 3). A `RangeError` from a pull that overflows the call stack is outside this guarantee (decision 12).

### 4. A cycle of derived nodes (decided)

The derived nodes must form a directed acyclic graph; a cycle is a programmer error, and the engine has no cycle check (Cycles). In the Isograph layer, `InlineCycle` rejects static field-selection cycles, so no runtime node cycle forms (pico-runtime.md open decision 8).

### 5. How the engine types the links between nodes of different value types (decided)

pico-runtime.md open decision 19, restated.

Decided by the user: (a).

- (a) No alias: a link is written `DerivedNode<unknown>`, `SourceNode<unknown>`, or `SourceNode<unknown> | DerivedNode<unknown>` at each use, and `equals` is a method signature so that `DerivedNode<T>` is assignable to `DerivedNode<unknown>`. tsc 5.6 under `strict` accepts pushing a `DerivedNode<number>` into a dependency array, adding a `Memo<number>` to a `Set<Memo<unknown>>`, and passing one as `consumer`, and rejects all three when `equals` is a function-typed property. Reason: no cast and no wrapper; the engine calls `equals` only with two values the node's own computation produced. Cost: method bivariance also accepts a call such as `(node as DerivedNode<unknown>).equals('a', 'b')` on a `DerivedNode<number>`; only code outside the engine could make it.
- (b) One value type per graph. Cost: a caller whose nodes have different value types declares a union and narrows every `readDerived` result.
- (c) An existential encoding: a link holds `<R>(visit: <T>(node: DerivedNode<T>) => R) => R`. Cost: one closure per link and a call per traversal step; it is sound.

### 6. The engine's identity equality (decided)

The engine has no default equality. `createDerivedNode` and `createSourceNode` require `equals`, with no optional parameter and no default. The package exports two comparisons, `tripleEquals` (`===`) and `objectIs` (`Object.is`), which a caller passes explicitly; a shared function avoids allocating a closure per node creation. `tripleEquals` is the Isograph layer's default, which it passes to each node it creates (pico-runtime.md open decision 20; upstream Isograph's comparison of resolver values, `areEqualWithDeepComparison.ts:84-95` in `../isograph`). Under `===`, a node whose value is `NaN` never cuts off: each recomputation stores the new `NaN` and marks its dependents `Dirty`. A reader whose verify pulled it sees a value `Object.is` the previous one and does not recompute (Verification); a reader pulled later finds its `Dirty` mark and recomputes. A change from `0` to `-0` cuts off, so its readers keep `0`. A caller that supplies its own `equals` gets that comparison and is not affected by the Isograph default. `degenerate.test.ts`'s last test asserts that the supplied `equals` decides cutoff for `0` and `-0`, and the test before it asserts that a verifying reader does not rerun over a dependency that recomputed to `NaN` under either comparison.

### 7. Whether a caller can write a node's fields (decided)

pico-runtime.md open decision 21, restated. Decided by the user: nodes are plain objects of plain object types, and the engine has no class. A caller that writes `dependents`, `memo`, `sourceReads`, or `derivedReads` breaks the contract silently, and the engine does not check it (decision 2). This deviates from AGENTS.md's "Never rely on discipline what we can enforce with newtypes" for the node types. Reason: classes with ES private fields make `DerivedNode<number>` not assignable to `DerivedNode<unknown>` through a private `#equals`, which needs decision 5 (c) or a cast per link, and opaque node types cost a cast in every engine function that receives a node.

### 8. Whether `writeSource` compares the new value with the old (decided)

Writing an equal value does not propagate. A source takes a required equality, as a derived node does: `createSourceNode(value, equals)`, and `writeSource` compares the stored value with the new one by that `equals`; when it accepts, `writeSource` stores nothing and marks nothing. The engine still has no default equality (decision 6). `markSourceDirty` stays, for a value object changed in place. This matches pico (`set_source`, `crates/pico/src/database.rs:310-329`), Reactively's `set`, and Preact's setter, with the equality supplied by the caller.

### 9. A pull of a disposed node

A disposal that leaves the node's state as it was is not an option: a `Clean` node pulled after it returns its old value although no source lists it (Disposal). pico removes a collected memo's revision, and the next call of the memo computes it again (`crates/pico/tests/absent_source/garbage_collection.rs:46`).

- (a) Disposal resets the node to never computed; a later pull computes it from scratch and reattaches its edges. This doc's body describes it. This option is held. Reason: pico's behavior. Cost: a caller that pulls a node it disposed gets a working node again instead of an error.
- (b) A fourth state, `Disposed`; `pull` throws on it, and so does a dependent's recomputation that reads it. Cost: every `switch` on the mark gains a case, and a dependent marked `Dirty` by the disposal throws on its next pull when its computation still reads the node.

### 10. An untracked read inside a computation (decided)

An untracked read is allowed. A read of `source.value`, and a `pull` called inside a computation, read without recording a dependency of the running computation, like pico's untracked lookup (`MemoRef::lookup`, `crates/pico/src/memo_ref.rs:57`, against `lookup_tracked`, `:80`), Preact's `peek()`, and Solid's `untrack`. An untracked read writes no source and marks nothing. `dependencies.test.ts` "source.value inside a computation records nothing" and "pull inside a computation records nothing" assert that a later write to what was read untracked does not mark the node.

### 11. Whether the package ships before its first caller (decided)

The package ships alone, as its own change (pico-runtime.md core change 4), with its tests and before the Isograph runtime uses it. Its reader is the user's stated goal that the engine is usable by projects outside Isograph (Readers before the Isograph runtime depends on it).

### 12. Pull depth (decided)

Pull depth is a contract on the caller. `pull`, `verify`, `recompute`, and `compute` recurse once per level of derived nodes below the pulled node, so a graph's pull depth must stay well below the host's call stack limit, an estimated 2,000 derived levels (about five frames per level; an estimate, not a measurement) in Node 24 with its default stack (Pull). Marking is iterative and never overflows (Mark). A pull that overflows throws `RangeError`, and in that case alone the engine does not guarantee its state: at the limit, a node's edges may be partly moved by `setDependencies` (Throws). The engine has no iterative pull. In the Isograph runtime, pull depth is bounded by client field nesting.
