# pico-js: implementation

This doc is the complete source of `@isograph/pico-js` in `libs/pico-js`. The package layout, `package.json`, build, hooks, and CI are as pico-js.md (The package) specifies, with two deltas. `src/` holds `graph.ts` and `index.ts`, and `test/` holds `graph.test.ts` and `graph.test-d.ts`. Every type field is `readonly` except `SourceNode.value`, `DerivedNode.value`, and `DerivedNode.status`, and the two dependency arrays are `Array`, because reads append to them and `detach` empties them.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation, its last value, a status, the source and derived nodes its computation read, and the derived nodes that read it.
- A write marks the source's direct readers `Dirty` and the nodes above them `Check`. It runs no computation.
- `pull` brings a node up to date. A `Clean` node returns its value. A `Check` node pulls its derived dependencies in the order they were read and recomputes at the first whose value changed; if none changed it becomes `Clean` without running. A `Dirty` node recomputes. A new node is `Dirty`.
- A computation receives its node and the previous value, which is `undefined` on the first computation and on the first computation after a disposal, as Solid's `createMemo` passes it. A recompute whose value is `===` the previous value marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- Under `===`, a `NaN` value always counts as changed. Under `===`, `0` and `-0` count as equal.
- Each read during a computation records the node it read in the calling node's dependency arrays. After the computation returns, the node is added to the `dependents` set of each source and derived node it read.
- Marking a node `Dirty` removes it from every `dependents` set it is in and empties its dependency arrays. Its next computation adds it to the sets of what it reads then.
- Disposing a node marks its readers `Dirty` and returns the node to the state of a new node.

The engine does not check these contracts:

- The caller writes a source only when its value changed.
- A computation, and any function it calls, does not throw.
- A computation does not write a source and does not dispose a node.
- A computation is synchronous and reads only through the node it receives.
- The derived nodes form a directed acyclic graph.
- The chains of derived nodes that a pull descends and that a write marks stay well below the call stack limit. In Node 24 with its default stack, a pull of a chain of about 3,000 derived nodes with one-line computations overflows.

## `src/graph.ts`

```ts
// from libs/pico-js/src/graph.ts
export type Status = 'Clean' | 'Check' | 'Dirty';

export type SourceNode<TValue> = {
  value: TValue;
  readonly dependents: Set<DerivedNode<unknown>>;
};

export type DerivedNode<TValue> = {
  value: TValue;
  status: Status;
  readonly sourceDependencies: Array<SourceNode<unknown>>;
  readonly derivedDependencies: Array<DerivedNode<unknown>>;
  readonly dependents: Set<DerivedNode<unknown>>;
  computation(callingNode: DerivedNode<unknown>, previous: TValue | undefined): TValue;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return { value, dependents: new Set() };
}

export function createDerivedNode<TValue>(
  computation: (callingNode: DerivedNode<unknown>, previous: TValue | undefined) => TValue,
): DerivedNode<TValue> {
  return {
    value: undefined as unknown as TValue,
    status: 'Dirty',
    sourceDependencies: [],
    derivedDependencies: [],
    dependents: new Set(),
    computation,
  };
}

export function writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
  source.value = value;
  markDependentsDirty(source.dependents);
}

export function readSource<TValue>(callingNode: DerivedNode<unknown>, source: SourceNode<TValue>): TValue {
  callingNode.sourceDependencies.push(source);
  return source.value;
}

export function readDerived<TValue>(callingNode: DerivedNode<unknown>, node: DerivedNode<TValue>): TValue {
  const value = pull(node);
  callingNode.derivedDependencies.push(node);
  return value;
}

export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  switch (node.status) {
    case 'Clean':
      return node.value;
    case 'Check':
      return verify(node);
    case 'Dirty':
      return recompute(node);
    default:
      return assertNever(node.status);
  }
}

export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  markDependentsDirty(node.dependents);
  detach(node);
  node.status = 'Dirty';
  node.value = undefined;
}

function markDependentsDirty(dependents: Set<DerivedNode<unknown>>): void {
  for (const node of dependents) {
    switch (node.status) {
      case 'Clean':
        detach(node);
        node.status = 'Dirty';
        markDependentsCheck(node.dependents);
        break;
      case 'Check':
        detach(node);
        node.status = 'Dirty';
        break;
      case 'Dirty':
        throw new Error('A Dirty node is in a dependents set');
      default:
        assertNever(node.status);
    }
  }
}

function markDependentsCheck(dependents: Set<DerivedNode<unknown>>): void {
  for (const node of dependents) {
    switch (node.status) {
      case 'Clean':
        node.status = 'Check';
        markDependentsCheck(node.dependents);
        break;
      case 'Check':
        break;
      case 'Dirty':
        throw new Error('A Dirty node is in a dependents set');
      default:
        assertNever(node.status);
    }
  }
}

function verify<TValue>(node: DerivedNode<TValue>): TValue {
  for (const dependency of node.derivedDependencies) {
    pull(dependency);
    if (node.status === 'Dirty') {
      return recompute(node);
    }
  }
  node.status = 'Clean';
  return node.value;
}

function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const value = node.computation(node, node.value);
  attach(node);
  node.status = 'Clean';
  if (value === node.value) {
    return node.value;
  }
  node.value = value;
  markDependentsDirty(node.dependents);
  return value;
}

function attach(node: DerivedNode<unknown>): void {
  for (const source of node.sourceDependencies) {
    source.dependents.add(node);
  }
  for (const dependency of node.derivedDependencies) {
    dependency.dependents.add(node);
  }
}

function detach(node: DerivedNode<unknown>): void {
  for (const source of node.sourceDependencies) {
    source.dependents.delete(node);
  }
  for (const dependency of node.derivedDependencies) {
    dependency.dependents.delete(node);
  }
  node.sourceDependencies.length = 0;
  node.derivedDependencies.length = 0;
}

function assertNever(value: never): never {
  throw new Error(`Unhandled variant: ${String(value)}`);
}
```

- `computation` is a method signature. TypeScript checks a method's parameters bivariantly, so `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`.
- A new node is in no `dependents` set and has empty dependency arrays, and marking and disposal detach a node before they set it `Dirty`, so a `Dirty` node is in no `dependents` set and writes to its dependencies never reach it. Deleting a node from the set being iterated is safe, because a `Set` iterator continues past a deleted current element.
- Only a `Dirty` node recomputes, and `recompute` attaches the node after its computation returns, so a node is in no `dependents` set while its computation runs. Nothing marks the node during its computation, so its dependency arrays hold exactly the computation's reads when `recompute` attaches it.
- Marking recurses into a node's dependents only when the node leaves `Clean`, so a write recurses into each node's dependents at most once.
- Because a write marks the source's direct readers `Dirty`, a `Check` node has no changed source, and `verify` pulls only its derived dependencies. A dependency's `recompute` marks its readers `Dirty` exactly when its new value is not `===` its previous value, so after a pull in `verify` the node is `Dirty` exactly when a dependency it read changed value.
- A `Clean` or `Check` node's `value` is a value its computation returned. A node becomes `Clean` only at the end of a computation or a verification, and `Check` only from `Clean`. A `Dirty` node's `value` is its previous value, or the cast `undefined` from `createDerivedNode` or disposal, and the engine reads it only as `previous`, which is typed `TValue | undefined`.
- A node read twice appears twice in the reader's dependency array and once in its `dependents` set.
- `disposeDerivedNode` takes a `DerivedNode<unknown>`, so writing `undefined` to `value` type-checks without a cast. Marking every reader `Dirty` detaches each reader, which empties the node's `dependents` set. A node that was never computed has an empty `dependents` set and empty dependency arrays, because a reader adds itself to a node's set only after pulling it, so its disposal changes nothing. An already `Dirty` node has empty dependency arrays, and its `dependents` set holds its `Check` readers, which its disposal marks `Dirty`.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  writeSource,
} from './graph';
export type { DerivedNode, SourceNode, Status } from './graph';
```

## Tests

```ts
// from libs/pico-js/test/graph.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  writeSource,
  type DerivedNode,
} from '../src/index';

describe('pull', () => {
  test('a derived node runs nothing until it is pulled, and a second pull of the Clean node runs nothing', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    expect(runs).toBe(0);
    expect(double.status).toBe('Dirty');
    expect(pull(double)).toBe(2);
    expect(pull(double)).toBe(2);
    expect(runs).toBe(1);
  });

  test('a node created inside a computation and read through readDerived is a dependency of its reader', () => {
    const source = createSourceNode(1);
    let inner: DerivedNode<number> | undefined;
    const outer = createDerivedNode((callingNode) => {
      inner ??= createDerivedNode((innerCallingNode) => readSource(innerCallingNode, source) * 10);
      return readDerived(callingNode, inner);
    });
    expect(pull(outer)).toBe(10);
    writeSource(source, 2);
    expect(outer.status).toBe('Check');
    expect(pull(outer)).toBe(20);
  });

  test('the first computation receives undefined as previous and a recompute receives the previous value', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, a) * 2;
    });
    pull(double);
    writeSource(a, 2);
    pull(double);
    expect(previousValues).toEqual([undefined, 2]);
  });
});

describe('marking', () => {
  test('a write marks direct readers Dirty and their readers Check and runs nothing', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    expect(pull(label)).toBe('big');
    writeSource(a, 5);
    expect(sum.status).toBe('Dirty');
    expect(label.status).toBe('Check');
    expect(runs).toEqual({ sum: 1, label: 1 });
  });

  test('a write marks readers even when the value is the same', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    pull(double);
    writeSource(a, 1);
    expect(double.status).toBe('Dirty');
    expect(pull(double)).toBe(2);
    expect(runs).toBe(2);
  });

  test('in a diamond, a write marks the shared node Dirty and every node above it Check', () => {
    const source = createSourceNode(1);
    const shared = createDerivedNode((callingNode) => readSource(callingNode, source) + 1);
    const left = createDerivedNode((callingNode) => readDerived(callingNode, shared) * 2);
    const right = createDerivedNode((callingNode) => readDerived(callingNode, shared) * 3);
    const top = createDerivedNode((callingNode) => readDerived(callingNode, left) + readDerived(callingNode, right));
    pull(top);
    writeSource(source, 2);
    expect([shared.status, left.status, right.status, top.status]).toEqual([
      'Dirty',
      'Check',
      'Check',
      'Check',
    ]);
  });
});

describe('cutoff', () => {
  test('a dependency that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    pull(label);
    writeSource(a, 0);
    writeSource(b, 3);
    expect(pull(label)).toBe('big');
    expect(label.status).toBe('Clean');
    expect(runs).toEqual({ sum: 2, label: 1 });
  });

  test('a computation that returns previous leaves its readers Clean without running them', () => {
    let readerRuns = 0;
    const a = createSourceNode(1);
    const parity = createDerivedNode<{ readonly even: boolean }>((callingNode, previous) => {
      const even = readSource(callingNode, a) % 2 === 0;
      return previous !== undefined && previous.even === even ? previous : { even };
    });
    const reader = createDerivedNode((callingNode) => {
      readerRuns += 1;
      return readDerived(callingNode, parity).even;
    });
    const first = pull(parity);
    pull(reader);
    writeSource(a, 3);
    expect(pull(reader)).toBe(false);
    expect(reader.status).toBe('Clean');
    expect(pull(parity)).toBe(first);
    expect(readerRuns).toBe(1);
  });

  test('a projection of a new object that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return { a: readSource(callingNode, a), b: readSource(callingNode, b) };
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1).b;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d2) * 2;
    });
    expect(pull(d3)).toBe(4);
    writeSource(a, 10);
    expect(pull(d3)).toBe(4);
    expect(runs).toEqual({ d1: 2, d2: 2, d3: 1 });
  });
});

describe('verification', () => {
  test('in a diamond each node runs once per change', () => {
    const source = createSourceNode(1);
    const runs = { shared: 0, left: 0, right: 0, top: 0 };
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, source) + 1;
    });
    const left = createDerivedNode((callingNode) => {
      runs.left += 1;
      return readDerived(callingNode, shared) * 2;
    });
    const right = createDerivedNode((callingNode) => {
      runs.right += 1;
      return readDerived(callingNode, shared) * 3;
    });
    const top = createDerivedNode((callingNode) => {
      runs.top += 1;
      return readDerived(callingNode, left) + readDerived(callingNode, right);
    });
    expect(pull(top)).toBe(10);
    writeSource(source, 2);
    expect(pull(top)).toBe(15);
    expect(runs).toEqual({ shared: 2, left: 2, right: 2, top: 2 });
  });

  test('verification stops at the first derived dependency whose value changed', () => {
    const gateSource = createSourceNode(1);
    const otherSource = createSourceNode(1);
    const runs = { gate: 0, other: 0, node: 0 };
    const gate = createDerivedNode((callingNode) => {
      runs.gate += 1;
      return readSource(callingNode, gateSource);
    });
    const other = createDerivedNode((callingNode) => {
      runs.other += 1;
      return readSource(callingNode, otherSource);
    });
    const node = createDerivedNode((callingNode) => {
      runs.node += 1;
      return readDerived(callingNode, gate) === 1 ? readDerived(callingNode, other) : 0;
    });
    expect(pull(node)).toBe(1);
    writeSource(gateSource, 2);
    writeSource(otherSource, 2);
    expect(pull(node)).toBe(0);
    expect(runs).toEqual({ gate: 2, other: 1, node: 2 });
  });
});

describe('source writes and verification', () => {
  test('a node whose source changed recomputes without pulling the derived node it read before', () => {
    const runs = { b: 0, c: 0 };
    const a = createSourceNode(true);
    const bInput = createSourceNode(1);
    const b = createDerivedNode((callingNode) => {
      runs.b += 1;
      return readSource(callingNode, bInput);
    });
    const c = createDerivedNode(() => {
      runs.c += 1;
      return 0;
    });
    const foo = createDerivedNode((callingNode) =>
      readSource(callingNode, a) ? readDerived(callingNode, b) : readDerived(callingNode, c),
    );
    expect(pull(foo)).toBe(1);
    writeSource(bInput, 2);
    writeSource(a, false);
    expect(foo.status).toBe('Dirty');
    expect(pull(foo)).toBe(0);
    expect(runs).toEqual({ b: 1, c: 1 });
  });
});

describe('dependencies', () => {
  test('a source that a recompute stopped reading does not mark the node', () => {
    let runs = 0;
    const useA = createSourceNode(true);
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const pick = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, useA) ? readSource(callingNode, a) : readSource(callingNode, b);
    });
    expect(pull(pick)).toBe(1);
    writeSource(useA, false);
    expect(pull(pick)).toBe(2);
    writeSource(a, 10);
    expect(pick.status).toBe('Clean');
    expect(pull(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of them', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => readSource(callingNode, a) + readSource(callingNode, b));
    pull(sum);
    writeSource(a, 3);
    expect(pull(sum)).toBe(5);
    writeSource(b, 4);
    expect(sum.status).toBe('Dirty');
    expect(pull(sum)).toBe(7);
    writeSource(a, 5);
    expect(sum.status).toBe('Dirty');
    expect(pull(sum)).toBe(9);
  });

  test('a Dirty node is not in the dependents set of its other sources, so a second write does not reach it', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode((callingNode) => `${readDerived(callingNode, sum)}`);
    pull(label);
    writeSource(a, 3);
    expect(b.dependents.has(sum)).toBe(false);
    writeSource(b, 4);
    expect(sum.status).toBe('Dirty');
    expect(label.status).toBe('Check');
    expect(pull(label)).toBe('7');
    expect(runs).toBe(2);
  });

  test('a computation that reads a source twice recomputes once per change', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const twice = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, a);
    });
    expect(pull(twice)).toBe(2);
    writeSource(a, 2);
    expect(pull(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('disposal', () => {
  test('disposing a node leaves it Dirty with no value, marks its readers Dirty, and its next pull computes it with previous undefined', () => {
    const runs = { inner: 0, outer: 0 };
    const innerPreviousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const inner = createDerivedNode<number>((callingNode, previous) => {
      runs.inner += 1;
      innerPreviousValues.push(previous);
      return readSource(callingNode, source);
    });
    const outer = createDerivedNode((callingNode) => {
      runs.outer += 1;
      return readDerived(callingNode, inner) + 1;
    });
    expect(pull(outer)).toBe(2);
    disposeDerivedNode(inner);
    expect(inner.status).toBe('Dirty');
    expect(inner.value).toBe(undefined);
    expect(outer.status).toBe('Dirty');
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
    writeSource(source, 2);
    expect(outer.status).toBe('Check');
    expect(pull(outer)).toBe(3);
    expect(innerPreviousValues).toEqual([undefined, undefined, 1]);
  });

  test('a disposed node is in no dependents set of the sources it read', () => {
    const source = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => readSource(callingNode, source));
    pull(inner);
    disposeDerivedNode(inner);
    expect(source.dependents.has(inner)).toBe(false);
    writeSource(source, 2);
    expect(pull(inner)).toBe(2);
  });

  test('disposing a Dirty node marks its Check readers Dirty', () => {
    const source = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => readSource(callingNode, source));
    const outer = createDerivedNode((callingNode) => readDerived(callingNode, inner) + 1);
    pull(outer);
    writeSource(source, 2);
    expect(inner.status).toBe('Dirty');
    expect(outer.status).toBe('Check');
    disposeDerivedNode(inner);
    expect(inner.value).toBe(undefined);
    expect(outer.status).toBe('Dirty');
    expect(pull(outer)).toBe(3);
  });

  test('disposing a node that was never computed changes nothing', () => {
    const previousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const node = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, source);
    });
    disposeDerivedNode(node);
    expect(node.status).toBe('Dirty');
    expect(node.value).toBe(undefined);
    expect(pull(node)).toBe(1);
    expect(previousValues).toEqual([undefined]);
  });
});
```

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import { createDerivedNode, readDerived, type DerivedNode } from '../src/index';

test('a derived node that reads another derived node has the type of the value its computation returns', () => {
  const count = createDerivedNode(() => 1);
  const label = createDerivedNode((callingNode) => `${readDerived(callingNode, count)}`);
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});
```
