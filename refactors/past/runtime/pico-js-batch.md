# pico-js: batches and onDirty

Status: under discussion. Not approved for implementation. This doc is a follow-up to the first pico-js PR, which ships `refactors/pending/runtime/pico-js.md` without batches or `onDirty`, and changes that engine.

A caller that observes a derived node needs to learn when the node stops being up to date, and a caller that writes several sources needs those writes to land before any observer runs. This change adds both. A `Batch` collects the `onDirty` callbacks of the observed nodes that the batch's writes take out of `Clean`. `flushBatch` calls them, once each, in the order the nodes left `Clean`. A callback runs after every write of its batch, so it may pull, write, and dispose.

## Model

- A `DerivedNode` has a writable field `onDirty: (() => void) | undefined`. A caller sets it to observe the node.
- `writeSource` and `disposeDerivedNode` take a `Batch` as their first parameter. Marking adds a node's `onDirty` to the batch when the node goes from `Clean` to `Dirty` or from `Clean` to `Check`. A node that is already `Check` or `Dirty` is not added again, so a node's callback is collected at most once between two pulls of the node.
- A `Batch` is a `Set<() => void>`. The set keeps insertion order, which is the order in which marking found the nodes, and it holds each callback once, so two nodes that share one callback function call it once per flush.
- `flushBatch(batch)` deletes each callback from the set and then calls it, in insertion order. A callback that writes through the same batch adds callbacks to the end of the set, and the same flush calls them. A callback that pulls its node and is then marked again in the same flush is added again and called again, because it was deleted before it ran. After `flushBatch` returns, the batch is empty and can be reused.
- A callback that is in the batch is called even when an earlier callback in the same flush disposes its node. Removing it would need a lookup from node to batch, and nothing needs that behavior.
- A pull collects no callbacks. A recompute whose value changed marks its node's dependents `Dirty`, and every dependent of a `Dirty` node is `Check` or `Dirty` (the `DerivedNode` invariants), so no node leaves `Clean` during a pull. `pull`, `readSource`, and `readDerived` therefore take no batch. `recompute` marks through `markCheckDependentsDirty`, which sets a `Check` dependent `Dirty`, leaves a `Dirty` dependent `Dirty`, and throws on a `Clean` or `Uncomputed` dependent.
- Setting `onDirty` on a node that is not `Clean` collects nothing until the node is pulled and marked again. An observer that starts observing pulls its node to make it `Clean`.

Contracts, added to pico-js.md's list:

- A callback does not throw.
- A caller does not write a node's fields, except `onDirty`.
- A callback that writes through the batch being flushed eventually stops writing.

## `src/graph.ts`

`DerivedNode` gains `onDirty`. Before (pico-js.md, `src/graph.ts`):

```ts
// from libs/pico-js/src/graph.ts
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
```

After:

```ts
// from libs/pico-js/src/graph.ts
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  onDirty: (() => void) | undefined;
  status: Status;
```

`createDerivedNode` initializes it, so every node has the same shape. After:

```ts
// from libs/pico-js/src/graph.ts
export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return {
    value: NOT_SET,
    onDirty: undefined,
    status: 0,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
}
```

The batch type and its two functions are new, and `writeSource` takes the batch. Before:

```ts
// from libs/pico-js/src/graph.ts
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
}
```

After:

```ts
// from libs/pico-js/src/graph.ts
export type Batch = Set<() => void>;

export function createBatch(): Batch {
  return new Set();
}

export function flushBatch(batch: Batch): void {
  for (const callback of batch) {
    batch.delete(callback);
    callback();
  }
}

export function writeSource<TValue>(
  batch: Batch,
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined) {
    markDirty(batch, first);
  }
}
```

`disposeDerivedNode` takes the batch. Before:

```ts
// from libs/pico-js/src/graph.ts
export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
```

After:

```ts
// from libs/pico-js/src/graph.ts
export function disposeDerivedNode(
  batch: Batch,
  node: DerivedNode<unknown>,
): void {
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(batch, first);
  }
  unlinkAll(node.firstSourceDependency);
  unlinkAll(node.firstDerivedDependency);
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = 0;
  node.value = NOT_SET;
}
```

`markDirty` takes the batch and adds `onDirty` in both arms where a node leaves `Clean`. After, in full:

```ts
// from libs/pico-js/src/graph.ts
function markDirty<TDependency>(batch: Batch, first: Edge<TDependency>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3:
        node.status = 1;
        if (node.onDirty !== undefined) {
          batch.add(node.onDirty);
        }
        if (node.firstDependent !== undefined) {
          stack.push(node);
        }
        break;
      case 2:
        node.status = 1;
        break;
      case 1:
        break;
      case 0:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
  let marked: DerivedNode<unknown> | undefined;
  while ((marked = stack.pop()) !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3:
          dependent.status = 2;
          if (dependent.onDirty !== undefined) {
            batch.add(dependent.onDirty);
          }
          if (dependent.firstDependent !== undefined) {
            stack.push(dependent);
          }
          break;
        case 2:
        case 1:
          break;
        case 0:
          throw new Error(
            'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
          );
      }
    }
  }
}
```

`recompute` marks its dependents through a new function that accepts only `Check` and `Dirty` dependents. Before:

```ts
// from libs/pico-js/src/graph.ts
  node.value = value;
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
  return value;
}
```

After:

```ts
// from libs/pico-js/src/graph.ts
function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const previous = computedValue(node);
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = 3;
  if (value === previous) {
    return previous;
  }
  node.value = value;
  markCheckDependentsDirty(node.firstDependent);
  return value;
}

function markCheckDependentsDirty(first: DerivedEdge | undefined): void {
  let edge = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 2:
        node.status = 1;
        break;
      case 1:
        break;
      case 3:
        throw new Error(
          'A Clean node depends on a Dirty node. This is indicative of a bug in pico-js.',
        );
      case 0:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
}
```

`markCheckDependentsDirty` needs no stack, because the dependents of a `Check` or `Dirty` node are already `Check` or `Dirty`. A `Dirty` dependent is a node that earlier marking set `Dirty` and that kept its edge to `node`. It stays `Dirty`.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export {
  createBatch,
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  flushBatch,
  pull,
  readDerived,
  readSource,
  writeSource,
} from './graph';
export type { Batch, DerivedNode, SourceNode, Status } from './graph';
```

## README

The API list gains these entries, and the `writeSource` and `disposeDerivedNode` entries take `batch` first:

```markdown
<!-- from libs/pico-js/README.md -->
- `createBatch()` returns an empty batch.
- `writeSource(batch, source, value)` stores `value` in `source`, marks the derived nodes that read it, and adds to `batch` the `onDirty` of each observed node that leaves `Clean`. It runs no computation.
- `disposeDerivedNode(batch, node)` marks the readers of `node` `Dirty`, adds their `onDirty` callbacks to `batch`, and returns `node` to `Uncomputed`.
- `flushBatch(batch)` calls each collected callback once, in the order the nodes left `Clean`, and empties `batch`.
- `node.onDirty` is a callback that a write or disposal collects when `node` leaves `Clean`.
- `Batch` is the type of a batch, a `Set<() => void>`.
```

The README example's writes become:

```ts
// from libs/pico-js/README.md
const batch = createBatch();
writeSource(batch, person, { first: 'Ada', last: 'Lovelace', age: 37 });
flushBatch(batch);
```

## Tests

The existing tests in pico-js.md call two helpers that write or dispose through a batch of one. Every `writeSource(source, value)` in them becomes `write(source, value)`, and every `disposeDerivedNode(node)` becomes `dispose(node)`; their assertions do not change.

```ts
// from libs/pico-js/test/graph.test.ts
import { describe, expect, test } from 'vitest';
import {
  createBatch,
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  flushBatch,
  pull,
  readDerived,
  readSource,
  writeSource,
  type DerivedNode,
  type SourceNode,
} from '../src/index';

function write<TValue>(source: SourceNode<TValue>, value: TValue): void {
  const batch = createBatch();
  writeSource(batch, source, value);
  flushBatch(batch);
}

function dispose(node: DerivedNode<unknown>): void {
  const batch = createBatch();
  disposeDerivedNode(batch, node);
  flushBatch(batch);
}
```

New tests:

```ts
// from libs/pico-js/test/graph.test.ts
describe('batches and onDirty', () => {
  test('a write calls nothing until the batch is flushed', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    pull(double);
    double.onDirty = () => calls.push('double');
    const batch = createBatch();
    writeSource(batch, a, 2);
    expect(calls).toEqual([]);
    flushBatch(batch);
    expect(calls).toEqual(['double']);
  });

  test('several writes in one batch call each onDirty once, in the order the nodes left Clean', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsBoth = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, readsA) + readDerived(callingNode, readsB),
    );
    pull(readsBoth);
    readsA.onDirty = () => calls.push('readsA');
    readsB.onDirty = () => calls.push('readsB');
    readsBoth.onDirty = () => calls.push('readsBoth');
    const batch = createBatch();
    writeSource(batch, b, 3);
    writeSource(batch, a, 4);
    writeSource(batch, b, 5);
    flushBatch(batch);
    expect(calls).toEqual(['readsB', 'readsBoth', 'readsA']);
  });

  test('a callback that pulls sees every write of the batch', () => {
    const seen: Array<number> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
    );
    pull(sum);
    sum.onDirty = () => seen.push(pull(sum));
    const batch = createBatch();
    writeSource(batch, a, 10);
    writeSource(batch, b, 20);
    flushBatch(batch);
    expect(seen).toEqual([30]);
  });

  test('a node that is already marked does not call onDirty again until it is pulled', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    pull(double);
    double.onDirty = () => {
      calls += 1;
    };
    write(a, 2);
    write(a, 3);
    expect(calls).toBe(1);
    pull(double);
    write(a, 4);
    expect(calls).toBe(2);
  });

  test('a node that becomes Check calls onDirty', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    outer.onDirty = () => {
      calls += 1;
    };
    write(a, 2);
    expect(outer.status).toBe(2);
    expect(calls).toBe(1);
  });

  test('a pull calls no onDirty, even when a recompute changes a value', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    const batch = createBatch();
    writeSource(batch, a, 2);
    outer.onDirty = () => {
      calls += 1;
    };
    expect(pull(outer)).toBe(3);
    flushBatch(batch);
    expect(calls).toBe(0);
  });

  test('a callback scheduled in a batch is called even if an earlier callback disposed its node', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const first = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const second = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    pull(first);
    pull(second);
    first.onDirty = () => {
      calls.push('first');
      dispose(second);
    };
    second.onDirty = () => calls.push('second');
    write(a, 2);
    expect(calls).toEqual(['first', 'second']);
  });

  test('a callback that writes through the batch being flushed has its new callbacks called in the same flush', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    pull(readsA);
    pull(readsB);
    const batch = createBatch();
    readsA.onDirty = () => {
      calls.push('readsA');
      writeSource(batch, b, 2);
    };
    readsB.onDirty = () => calls.push('readsB');
    writeSource(batch, a, 2);
    flushBatch(batch);
    expect(calls).toEqual(['readsA', 'readsB']);
    expect(batch.size).toBe(0);
  });

  test('a callback that pulls its node and is marked again in the same flush is called again', () => {
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    pull(node);
    const batch = createBatch();
    node.onDirty = () => {
      const value = pull(node);
      calls.push(value);
      if (value < 3) {
        writeSource(batch, a, value + 1);
      }
    };
    writeSource(batch, a, 2);
    flushBatch(batch);
    expect(calls).toEqual([2, 3]);
  });

  test('two nodes that share one callback call it once per flush', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const left = createDerivedNode((callingNode) => readSource(callingNode, a));
    const right = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    pull(left);
    pull(right);
    const callback = () => {
      calls += 1;
    };
    left.onDirty = callback;
    right.onDirty = callback;
    write(a, 2);
    expect(calls).toBe(1);
  });

  test('disposing an observed reader calls its onDirty', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    outer.onDirty = () => {
      calls += 1;
    };
    dispose(inner);
    expect(outer.status).toBe(1);
    expect(calls).toBe(1);
  });

  test('flushing an empty batch calls nothing', () => {
    const batch = createBatch();
    flushBatch(batch);
    expect(batch.size).toBe(0);
  });
});
```

All 35 tests (the 23 of pico-js.md and these 12) and the type test pass under vitest 2.1.9, and `tsc --noEmit` reports no errors, on the code above formatted by the repo's prettier config.

## Effect on pico-runtime.md

- `normalize` writes a response through one batch and flushes it once after the last write, so every observer runs after the whole response has landed. A response that throws part-way still flushes, in the `finally` that core change 9 already places after `normalize`.
- `deleteRecord` and `garbageCollect` take the batch of the write that triggered them, or a batch of their own that they flush.
- Later: `listen(node, onChange)` sets `node.onDirty`, and `notify`'s scan over every observer is deleted. React's `useSyncExternalStore` receives `onChange` as `onDirty` and pulls in `getSnapshot`.
- Later: `reconcileEffects` sets `onDirty` on each mounted node with an effect to a callback that pulls the node and reconciles its effect during the flush.

## Open decisions

1. Callback identity. The batch deduplicates by callback, so two nodes that share a function call it once. The alternative is a `Set<DerivedNode<unknown>>` read at flush time, which calls each node's current `onDirty` and so does not call a callback that was cleared after it was collected. Held: callbacks, because a collected callback is called even if its node changes afterwards, which is the behavior the user asked for.
2. Whether a write outside any batch exists. Held: no. Every write takes a batch, and a single write is `createBatch`, `writeSource`, `flushBatch`. The alternative, an overload without a batch, has nowhere to put the callbacks it collects.
