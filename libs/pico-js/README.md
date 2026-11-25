# pico-js

pico-js is a TypeScript library for incremental computation. You write functions that compute values from inputs and from each other. pico-js remembers each function's result and which inputs and functions it read. When an input changes, a function reruns the next time its result is needed, and only if at least one of the nodes it read last time has changed. A function whose node is retained with a change callback reruns before the update returns. A function whose node pico-js has disposed reruns the next time its result is needed.

## Priorities

pico-js is designed to be the fastest JS reactivity library, even at the expense of ease of use. To achieve this performance, it does not enforce anything at runtime, and requires the user to follow contracts. For example, sources should only be updated when their value has changed, and derived nodes should return their previous value when the new result is equal to it, so that `===` detects no change. "Contracts" lists every contract and what happens when you break it.

## What pico-js does not support

- Computations and change callbacks must not throw. Code that can throw wraps itself in `try`/`catch` inside the computation or callback, and after a throw reaches pico-js its state is unspecified. "Wrapping pico-js for an easier API" has an error-handling wrapper.
- Computations must be synchronous, and read other nodes only through `readSource` and `readDerived`.
- No node may read itself, directly or through other nodes.
- A write batch body must not read or retain a derived node, or call `disposeNext` or `disposeAll`.
- A change callback may call `readDerivedNode` and nothing else in pico-js, so it must not write, open a batch, retain, or release.
- pico-js compares values with `===` only, and counts `===`-equal values as identical.

"Contracts for correctness" lists what the current version does when you do any of these.

## How it works

pico-js is the package `@isograph/pico-js`:

```ts
import {
  createDerivedNode,
  createSourceNode,
  readDerived,
  readDerivedNode,
  readSource,
  retainDerivedNode,
  writeSource,
} from '@isograph/pico-js';
```

A source node holds an input, and a derived node holds a computed value:

```ts
const a = createSourceNode(1);
const b = createSourceNode(2);

const sum = createDerivedNode(
  (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
);
```

pico-js has no top-level read of a source, so keep the value you wrote if you need it elsewhere.

To read a derived node from outside a computation, call `readDerivedNode`. The first read computes the node. While the node is retained (see "Retaining nodes"), later reads return the cached value until something the node read changes:

```ts
let doubledComputationCount = 0;
const doubled = createDerivedNode((callingNode) => {
  // Counting computations is for demonstration only. Real computations should have no side effects.
  doubledComputationCount += 1;
  return readDerived(callingNode, sum) * 2;
});
retainDerivedNode(doubled);

readDerivedNode(doubled); // 6, doubledComputationCount === 1
readDerivedNode(doubled); // 6, doubledComputationCount === 1
```

Updating a source recomputes nothing right away, unless a node retained with a change callback depends on it (see "Reacting to changes"). The nodes that depend on it recompute when they are next read, if a value they read changed:

```ts
writeSource(a, 10); // doubledComputationCount === 1
readDerivedNode(doubled); // 24, doubledComputationCount === 2
```

A node depends on whatever its computation read on its latest run, so its dependencies can change from one run to the next:

```ts
const useA = createSourceNode(true);
const chosen = createDerivedNode((callingNode) =>
  readSource(callingNode, useA)
    ? readSource(callingNode, a)
    : readSource(callingNode, b),
);
retainDerivedNode(chosen);
```

Retaining `chosen` computes it and keeps its result. While `useA` is true, `chosen` reads only `useA` and `a`. After an update to `b`, reading `chosen` returns its cached value without rerunning its computation.

## API

- `createSourceNode(value)` creates a source node.
- `writeSource(source, value)` updates a source.
- `createDerivedNode(computation)` creates a derived node. `computation(callingNode, previous)` returns its value. `previous` has the type `TValue | undefined`. It is `undefined` the first time the node computes, and again the first time it computes after pico-js has disposed it.
- `readSource(callingNode, source)` and `readDerived(callingNode, node)` read a node from inside a computation.
- `readDerivedNode(node)` reads a derived node from outside a computation.
- `retainDerivedNode(node, onChange?)` computes the node if it is not up to date, keeps it cached between reads, and returns a retain. `releaseRetain(retain)` releases the retain.
- `startWriteBatch()` and `endWriteBatch()` group updates.
- `scheduler`, `disposeNext()`, and `disposeAll()` control when unused nodes are disposed. See "Disposing unused nodes".
- `SourceNode`, `DerivedNode`, and `Retain` are opaque handles. You pass them to the functions above. `CallingNode` is the type of the `callingNode` a computation receives.

## Returning the previous value

Derived nodes should return the previous value if their results are unchanged. When a derived node recomputes to a value `===` to its previous value, the nodes that depend on it do not recompute on its account. They still recompute if something else they read changed. A computation receives its previous value as its second argument:

```ts
const numbers = createSourceNode([1, 2, 3, 4]);

const evens = createDerivedNode((callingNode, previous) => {
  const next = readSource(callingNode, numbers).filter((n) => n % 2 === 0);
  return deepEqual(previous, next) ? previous : next;
});
```

`filter` returns a new array every time, so without the check every update to `numbers` would give `evens` a new value. With it, updating `numbers` to `[1, 2, 5, 4]` leaves `evens` as the same `[2, 4]` array, and a node that reads only `evens` does not recompute.

## Nodes with parameters

A computation receives only `callingNode` and `previous`. For a value that depends on a parameter, create one derived node per parameter value, and look each node up however suits your app. For example, with a map:

```ts
const multipleNodes = new Map<number, DerivedNode<number>>();

function multipleOfA(factor: number): DerivedNode<number> {
  let node = multipleNodes.get(factor);
  if (node === undefined) {
    node = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * factor,
    );
    multipleNodes.set(factor, node);
  }
  return node;
}
```

## Retaining nodes

pico-js disposes a node that nothing retains and no other node reads, after a while, and drops its cached result. Retain any node whose result should stay cheap to read, for example because you will read it again. `retainDerivedNode` computes the node right away if it is not up to date, so it counts as a read. It keeps the node and everything it depends on cached until you call `releaseRetain`. A React component, for example, typically retains the nodes it renders when it mounts, and releases them when it unmounts.

## Reacting to changes

Pass a callback to `retainDerivedNode` to run code when a node's value changes:

```ts
retainDerivedNode(sum, (value) => console.log(value));
```

`onChange` receives the node's value after the update. pico-js does not call it when the retain is created. A node with at least one retain that has an `onChange` is a node with a callback. pico-js updates a node with a callback without waiting for a read. After each update that may change its value, pico-js brings it up to date right away, which recomputes it only if something it read changed, and calls the callback if the value changed. The call happens synchronously, before `writeSource` returns, or before the `endWriteBatch` that closes the outermost batch returns. After the last retain with a callback is released, the node is no longer observed and recomputes only when it is read. The step at the end of an update that brings nodes with callbacks up to date and calls their callbacks is the flush.

## Grouping updates

pico-js brings nodes with callbacks up to date once, for all the updates between `startWriteBatch()` and the matching `endWriteBatch()`, and runs their callbacks once. Batches can nest. Only the `endWriteBatch` that closes the outermost batch does this:

```ts
startWriteBatch();
writeSource(a, 3);
startWriteBatch();
writeSource(b, 4);
endWriteBatch(); // no flush: the outer batch is still open
endWriteBatch(); // flushes both writes
```

Inside a batch, write sources, release retains, and open nested batches. Do not read derived nodes, retain them, or call `disposeNext` or `disposeAll` there (see "Contracts").

## Disposing unused nodes

[Disposing nodes](./docs/disposing-nodes.md) explains how this works.

A node becomes unneeded when nothing retains it and no other node reads it, after you release its last retain, after its last dependent stops reading it or is disposed, or after a read of a node nothing retains. pico-js does not dispose an unneeded node right away. It adds the node to a cleanup queue and keeps its cached value until a cleanup runs, so a read or a retain before then reuses the cached value, and recomputes it only if something the node read has changed. Each of these events adds an entry, also for a node that is already on the cleanup queue, so a loop of top-level reads of a node that nothing retains adds one entry per read. An update that takes an unneeded node out of date disposes it when the update ends, or when the outermost batch ends, if the node is still unneeded then.

Cleanup is controlled by `scheduler`, an object with a `schedule` function field that you can assign. pico-js calls `scheduler.schedule()` with no arguments at the end of the first call into pico-js that completes after the cleanup queue went from empty to non-empty. That call is a `writeSource`, `retainDerivedNode`, `releaseRetain`, or `readDerivedNode` made outside a batch and outside a change callback, or the `endWriteBatch` that closes the outermost batch. A `writeSource` to a source that has no dependents, or that no computation has read since the source was created or last written, returns without calling `schedule`, and the next of these calls calls it. Adding a node to a non-empty cleanup queue does not cause another call to `schedule`. `disposeNext()` can empty the cleanup queue and then refill it by disposing a node, so `schedule` can be called again while an earlier drain is still in progress.

`disposeNext()` takes the node most recently added to the cleanup queue, disposes it if it is still unneeded, and returns whether the cleanup queue still has entries. Disposing a node can add the nodes it depended on to the cleanup queue. `disposeAll()` disposes every node on the cleanup queue that is still unneeded, and every node that this leaves unneeded.

The default `schedule` calls `disposeNext()` during idle time with `requestIdleCallback`, for as long as the idle period has time left, and asks for another idle period if nodes are left. Where `requestIdleCallback` does not exist, for example on a server, it disposes nodes for 5 ms per `setTimeout` task.

Your own `schedule` must not throw (see "Contracts for correctness"). After pico-js calls it, `schedule` must call `disposeAll()`, or call `disposeNext()` until that returns `false`, at once or later, for example in idle time or in chunks between frames. To dispose everything at the end of each call into pico-js:

```ts
scheduler.schedule = disposeAll;
```

To dispose in chunks, with the time budget of each idle callback:

```ts
function disposeInIdleTime(deadline: IdleDeadline) {
  while (deadline.timeRemaining() > 0) {
    if (!disposeNext()) {
      return;
    }
  }
  requestIdleCallback(disposeInIdleTime);
}
scheduler.schedule = () => requestIdleCallback(disposeInIdleTime);
```

To dispose only when you choose, make `schedule` do nothing and call `disposeAll()` yourself, outside a write batch, a computation, and a change callback, for example when your app is idle. The contract then requires you to call `disposeAll()`, or to drain `disposeNext()`, later. Until you do, unneeded nodes stay cached, except that an update still disposes the unneeded nodes it marks:

```ts
scheduler.schedule = () => {};
```

## Servers

pico-js's state is module-level, so every caller that loads the same copy of the module shares it. On a server, every request shares the graph, the cached values in it, and the open write batch.

## Wrapping pico-js for an easier API

pico-js keeps its API minimal for speed, and an app can wrap it. These wrappers are examples. They use the exports in "API", and each compiles against them.

A module-level current node lets a computation read without passing `callingNode`, and lets sources and derived nodes be functions that read when called. `derived` saves the current node, sets it to its own `callingNode`, and restores it in a `finally`. `read(node)` reads a source or a derived node, inside or outside a computation, and calling a node, as in `price()`, calls `read` on it. `read` checks `kind` to tell a source from a derived node, then checks the current node. Inside a computation it calls `readSource` or `readDerived` with the current node. Outside one it calls `readDerivedNode` for a derived node, and returns `latest` for a source, because pico-js has no top-level read of a source. `derived` takes an `equal` function that returns `previous` when the new result is equal to it, which keeps the `===` cutoff for structurally equal results, for example with `deepEqual`. A source's `set` takes an `equal` function that skips `writeSource` when the value did not change, and is the only code that writes `latest`. Both `equal` functions default to `===`, the comparison pico-js itself uses.

```ts
let current: CallingNode | undefined;

type Source<T> = (() => T) & {
  readonly kind: 'source';
  readonly handle: SourceNode<T>;
  latest: T;
  readonly set: (value: T, equal?: (a: T, b: T) => boolean) => void;
};

type Derived<T> = (() => T) & {
  readonly kind: 'derived';
  readonly handle: DerivedNode<T>;
};

function read<T>(node: Source<T> | Derived<T>): T {
  if (node.kind === 'source') {
    return current === undefined
      ? node.latest
      : readSource(current, node.handle);
  }
  return current === undefined
    ? readDerivedNode(node.handle)
    : readDerived(current, node.handle);
}

function source<T>(initial: T): Source<T> {
  const handle = createSourceNode(initial);
  const node: Source<T> = Object.assign(() => read(node), {
    kind: 'source' as const,
    handle,
    latest: initial,
    set: (value: T, equal: (a: T, b: T) => boolean = (a, b) => a === b) => {
      if (!equal(node.latest, value)) {
        node.latest = value;
        writeSource(handle, value);
      }
    },
  });
  return node;
}

function derived<T>(
  compute: (previous: T | undefined) => T,
  equal: (a: T, b: T) => boolean = (a, b) => a === b,
): Derived<T> {
  const handle = createDerivedNode<T>((callingNode, previous) => {
    const saved = current;
    current = callingNode;
    try {
      const next = compute(previous);
      return previous !== undefined && equal(previous, next) ? previous : next;
    } finally {
      current = saved;
    }
  });
  const node: Derived<T> = Object.assign(() => read(node), {
    kind: 'derived' as const,
    handle,
  });
  return node;
}

const price = source(10);
const qty = source(3);
const total = derived(() => price() * qty());
const evens = derived(
  () => [price(), qty()].filter((n) => n % 2 === 0),
  deepEqual,
);

total(); // 30
price.set(12);
read(total); // 36
read(price); // 12
evens(); // [12]
```

A wrapper that catches errors returns a result value, so nothing throws into pico-js. The reader decides what an error means. Inside a computation or a change callback, the reader checks `ok` and uses a fallback. Outside them, `unwrap` rethrows the error:

```ts
type Result<T> = { ok: true; value: T } | { ok: false; error: unknown };

function safeDerived<T>(
  compute: (previous: T | undefined) => T,
): Derived<Result<T>> {
  return derived<Result<T>>((previous) => {
    try {
      return {
        ok: true,
        value: compute(previous?.ok === true ? previous.value : undefined),
      };
    } catch (error) {
      return { ok: false, error };
    }
  });
}

function unwrap<T>(result: Result<T>): T {
  if (!result.ok) {
    throw result.error;
  }
  return result.value;
}
```

`batch` closes the batch in a `finally`, and `subscribe` retains a node with a callback and returns a function that releases the retain. The wrappers do not lift the batch contract. A `batch` body may write sources, read sources, release retains, and open nested batches. Reading a derived node there, with `total()` or `read(total)`, calls `readDerivedNode` inside a write batch, and calling `subscribe` there retains inside one, which the contracts forbid:

```ts
function batch<T>(fn: () => T): T {
  startWriteBatch();
  try {
    return fn();
  } finally {
    endWriteBatch();
  }
}

function subscribe<T>(
  node: Derived<T>,
  onChange: (value: T) => void,
): () => void {
  const retain = retainDerivedNode(node.handle, onChange);
  return () => releaseRetain(retain);
}
```

Each wrapper has a cost, which is why pico-js does not do it itself. An engine build that tracked a current node itself was within noise of `callingNode` on a write followed by a top-level read of a retained chain of three nodes, 4.7% slower on a write followed by a top-level read that recomputes a node reading 1,000 derived nodes, and 1.4% to 8.2% slower in three of the 22 app scenarios that were measured, and within noise in the others. The wrapper above also makes each read a call through a closure, plus the dispatch between source and derived: the same two measurements took 1.34 and 1.91 times as long through it. `batch(fn)` was 3% to 27% slower than `startWriteBatch` and `endWriteBatch` in 8 of the 59 measured cases, most of them repeated write batches and effect churn, faster in 3, and within noise in the others. A `try` adds work to every run, and an equality check adds a comparison to every write or every recompute. pico-js leaves these to the app, so that an app that does not need one does not pay for it.

## Contracts

pico-js does not check these contracts at run time.

### Contracts for correctness

The results listed for violating a contract describe what the current version does, and may change in any release. Where a contract lists no result for a violation, or calls it unspecified, pico-js's behavior after that violation is unspecified.

- Computations must be synchronous and pure, and read other nodes only through `readSource` and `readDerived`, called with the `callingNode` the computation received. pico-js reruns a computation only when something it read that way has changed, or after it has disposed the node, so a computation that reads anything else keeps returning its cached value after that other state changes. A read with a `callingNode` that a computation kept after it returned, or that belongs to another node, records the dependency on that `callingNode`'s node. The computation that made the read keeps its cached value when the read node changes, and the other node recomputes the next time it is read. If that other node's computation is running, the behavior is unspecified. An asynchronous computation's value is the promise it returns. Its reads after an `await` are reads with a `callingNode` kept after the computation returned, so they record dependencies of its own node, and a change to what they read makes the node recompute the next time it is read.
- A computation must not call anything else in pico-js. `readDerivedNode` inside a computation returns the node's value without recording a dependency, as a read of state outside pico-js does. If it took the cleanup queue from empty to non-empty, it calls `schedule` while the computation runs, except during a flush, where the call into pico-js that ran the flush calls `schedule` before it returns. If a `schedule` called while the computation runs disposes nodes, the behavior is unspecified. The behavior of a write, a batch, a retain, a release, or a disposal inside a computation is unspecified.
- Computations and change callbacks must not throw. If your code can throw, wrap it in `try`/`catch` inside the computation or callback, so nothing throws into pico-js. After a throw reaches pico-js, its state is unspecified. `safeDerived` in "Wrapping pico-js for an easier API" is an example. It catches inside the computation and returns the error as a value.
- The graph must not have cycles. A computation that reads its own node, or a node that depends on its node, recurses until the call stack overflows and throws a `RangeError`. After that, pico-js's state is unspecified.
- A change callback may call `readDerivedNode` and must not do anything else with pico-js. A write inside a callback to a source that no node with a callback depends on, directly or through other nodes, does what that write does outside a callback: it marks the nodes that depend on the source and runs no computation, and the flush goes on calling callbacks. A write to a source that a node with a callback depends on starts a nested flush, which brings the nodes with callbacks that the write reached up to date and then throws a `TypeError` before it calls any callback, whether or not a value changed. After that throw, pico-js's state is unspecified. A batch inside a callback does the same at its `endWriteBatch`, for the sources written in it. The behavior of a retain or a release inside a callback is unspecified. Compute a value that can be computed from the graph in a derived node, not in a source that a callback writes. To reset a value when another changes, keep the raw value in the source and compute the effective value in a derived node:

  ```ts
  const query = createSourceNode('a');
  const requested = createSourceNode({ page: 3, query: 'a' });
  const page = createDerivedNode((callingNode) => {
    const { page, query: requestedFor } = readSource(callingNode, requested);
    return requestedFor === readSource(callingNode, query) ? page : 1;
  });
  ```

  Starting work outside the graph from a callback, for example a fetch, is fine, and the `writeSource` from its response runs outside the flush. A write that must follow a change can be deferred with `queueMicrotask(() => writeSource(...))`, which runs after the flush returns.

- Release each retain exactly once. The behavior of a second release of the same retain is unspecified. A retain that is never released keeps its node and everything the node depends on cached for good.
- Do not read derived nodes inside a write batch, and do not call `retainDerivedNode` there, because retaining computes the node, which is a read. Inside a batch, write sources, release retains, and open nested batches. The `endWriteBatch` that closes the outermost batch brings the nodes with callbacks up to date. A read or a retain inside a batch brings the node up to date as a top-level read does, and calls `schedule` inside the batch if it took the cleanup queue from empty to non-empty. If the batch's writes changed a node with a callback that the read brings up to date, the flush calls no callback for that change. The flush also calls no callback for a node with a callback that depends on the read node and that the batch's writes reach only through nodes with callbacks. If the read changed the value of a node with a callback, each node with a callback that reads that node directly keeps its old value after the flush and is on no flush queue. Marking stops at such a node, so a later update that reaches it brings it up to date and calls its callbacks only if the flush refreshes a node with a callback that depends on it and that the update reached through another source. Otherwise the node keeps its old value, and its callbacks do not run on any later update, until something reads it. A node with a callback above it that an update reaches only through it is not brought up to date by that update either. A top-level read of the node, or a read through a lazy node outside a flush, brings it up to date without calling its callbacks, and the updates after that read call them again. If that read changes the node's value, the read leaves the nodes with callbacks that read the node directly in the same state.
- Call `endWriteBatch` exactly once for each `startWriteBatch`. If the code between them can throw, call `endWriteBatch` in a `finally`. A missing `endWriteBatch` leaves the batch open, so the flush does not run and no callback runs until the outermost batch's `endWriteBatch`. An extra `endWriteBatch` lowers the batch depth below zero. Until `startWriteBatch` calls raise it above zero again, every write flushes at once, even inside a batch, and `writeSource`, `releaseRetain`, and `endWriteBatch` call `schedule` only while the depth is exactly zero.
- A write batch must not span an `await`. Requests share one graph (see "Servers"), so another request's writes join your batch, and nodes with callbacks are brought up to date when you end it.
- Call `disposeNext` and `disposeAll` outside write batches, computations, and change callbacks. Inside a write batch they dispose what they dispose outside one, which is each node on the cleanup queue that is still unneeded and, for `disposeAll`, each node that this leaves unneeded. Every computation, top-level read, and change callback then receives the values it receives when the disposal runs after the batch. A node that the batch's writes marked while it was unneeded is disposed by that call instead of at the end of the outermost batch. After a disposal that empties the cleanup queue, the end of the outermost batch calls `schedule` only if a node joins the cleanup queue after the disposal.
- A custom scheduler must not throw, and once called it must keep calling `disposeNext` until it returns `false`, or call `disposeAll`. If it stops early, the remaining unneeded nodes stay cached until something calls `disposeNext` or `disposeAll` again, and pico-js does not call `schedule` again until the cleanup queue has been emptied and refilled. If `schedule` throws, the error comes out of the call into pico-js that called it, after that call has done its work. `writeSource`, `endWriteBatch`, and `releaseRetain` have flushed. `retainDerivedNode` and `readDerivedNode` have computed the node but return no retain or value, so a retain made that way can never be released. pico-js then does not call `schedule` again until the cleanup queue has been emptied and refilled.
- pico-js compares values with `===` and counts `===`-equal values as identical. A recompute whose result is `===` to the previous value keeps the previous value, the nodes that depend on the node do not recompute on its account, and no callback runs. A computation whose result depends on the sign of zero breaks this contract. A node that reads `x` and returns `1 / x` keeps `Infinity` when `x` recomputes from `0` to `-0`. `NaN` is never `===` to itself, so a node whose value is `NaN` counts as changed on every recompute: the nodes that depend on it recompute, and its callbacks run, after every update that recomputes it.
- Do not mutate values after passing them to pico-js or reading them from it. A value changed in place is still `===` to itself, so nodes that depend on it can keep results computed from its old contents.
- Keep chains of nodes that each read the next to at most 1,000 nodes. The first compute of such a chain recurses through the computations, and so does a recompute of a chain whose nodes each read a written source directly, and a read of a chain of nodes with callbacks that verifies them, so a longer chain can overflow the call stack, which throws a `RangeError`. After that, pico-js's state is unspecified. In Node 24.12 with the default stack size, with nodes that each read the node below and one source and add the two, a first compute threw at 1,358 nodes in a new process and at 1,757 nodes after the engine's code was optimized, and a recompute of a chain whose nodes all read the written source threw at 1,757 to 1,867 nodes. With nodes with callbacks that each read the node below and add 1, over a bottom node that keeps its value, a top-level read after a write, which only verifies the chain and runs no computation, threw at 2,308 nodes. A computation that uses more stack between its reads lowers these limits. A chain of lazy nodes that a write reaches only through the node at its bottom verifies and recomputes with explicit stacks, and a chain of 200,000 such nodes did not overflow.

### Contracts for performance

Breaking one of these costs extra work. The results stay correct.

- Only update a source when its value changed, for example by checking `deepEqual(current, next)` before calling `writeSource`. `writeSource` does not compare values, so an update with the same value marks the nodes that depend on the source, and its direct dependents recompute, usually to the same result. A second write before any computation reads the source again marks nothing.
- When a computation's result is structurally equal to its previous value, return the previous value (`previous`), so that `===` sees no change. A new but equal value counts as a change, so the nodes that depend on the node recompute, and its callbacks run, although nothing changed.

## Guarantees

If you follow the contracts, every computation, top-level read, and change callback reads current values, and a node recomputes only when something it read changed, or after pico-js has disposed it. A change callback runs at most once per update or batch, after every node with a callback is up to date.

How pico-js works is in the [design document](./docs/design.md). [Disposing nodes](./docs/disposing-nodes.md) describes how pico-js decides when to dispose a cached node, [performance](./docs/performance.md) describes the data structures and techniques that make it fast, and [correctness](./docs/correctness.md) proves the first guarantee.
