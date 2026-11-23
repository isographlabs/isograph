# pico-js: implementation

This doc is the complete source of `@isograph/pico-js` in `libs/pico-js`. The package layout, `package.json`, build, hooks, and CI are as pico-js.md (The package) specifies; this doc gives the source files and the tests.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and, once computed, a `DerivedNodeMemo`: the value, a status, the nodes the computation read, and the memos that read the node.
- A write marks the memos of the source's direct readers `Dirty` and the memos above them `Check`. It runs no computation.
- `pull` brings a node up to date. A `Clean` memo returns its value. A `Check` memo pulls its derived dependencies in the order they were read and recomputes at the first whose value changed; if none changed it becomes `Clean` without running. A `Dirty` memo recomputes. A node with no memo computes for the first time.
- A computation receives its node's memo and the previous value, which is `undefined` on the first computation, as Solid's `createMemo` passes it. A recompute whose value is `===` the previous value marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- Under `===`, a `NaN` value always counts as changed. Under `===`, `0` and `-0` count as equal.
- A computation pushes its reads onto its memo's dependency lists. After it returns, the memo is added to the `dependents` set of each node it read, in one synchronous step with the new value.
- Marking a memo `Dirty` removes it from every `dependents` set it is in and empties its dependency lists. Its next computation adds it to the sets of what it reads then.

Contracts the engine does not check:

- The caller writes a source only when its value changed.
- A computation does not throw, does not write a source, and does not dispose a node.
- A computation is synchronous and reads only through the memo it receives.
- The derived nodes form a directed acyclic graph.
- The derived nodes below any pulled node stay well below the call stack limit (about 2,000 levels in Node 24, an estimate).

## `src/graph.ts`

```ts
// from libs/pico-js/src/graph.ts
const MISSING: unique symbol = Symbol('Missing');

type Option<TValue> = TValue | typeof MISSING;

export type Status = 'Clean' | 'Check' | 'Dirty';

export type SourceNode<TValue> = {
  value: TValue;
  readonly dependents: Set<DerivedNodeMemo<unknown>>;
};

export type DerivedNode<TValue> = {
  computation(consumer: DerivedNodeMemo<unknown>, previous: TValue | undefined): TValue;
  memo: Option<DerivedNodeMemo<TValue>>;
};

export type DerivedNodeMemo<TValue> = {
  value: TValue;
  status: Status;
  sourceDependencies: Array<SourceNode<unknown>>;
  derivedDependencies: Array<DerivedNode<unknown>>;
  readonly dependents: Set<DerivedNodeMemo<unknown>>;
  readonly node: DerivedNode<TValue>;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return { value, dependents: new Set() };
}

export function createDerivedNode<TValue>(
  computation: (consumer: DerivedNodeMemo<unknown>, previous: TValue | undefined) => TValue,
): DerivedNode<TValue> {
  return { computation, memo: MISSING };
}

export function writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
  source.value = value;
  markDependentsDirty(source.dependents);
}

export function markSourceDirty(source: SourceNode<unknown>): void {
  markDependentsDirty(source.dependents);
}

export function readSource<TValue>(consumer: DerivedNodeMemo<unknown>, source: SourceNode<TValue>): TValue {
  consumer.sourceDependencies.push(source);
  return source.value;
}

export function readDerived<TValue>(consumer: DerivedNodeMemo<unknown>, node: DerivedNode<TValue>): TValue {
  const value = pull(node);
  consumer.derivedDependencies.push(node);
  return value;
}

export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  const memo = node.memo;
  if (memo === MISSING) {
    return computeFirst(node);
  }
  switch (memo.status) {
    case 'Clean':
      return memo.value;
    case 'Check':
      return verify(memo);
    case 'Dirty':
      return recompute(memo);
    default:
      return assertNever(memo.status);
  }
}

export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const memo = node.memo;
  if (memo === MISSING) {
    return;
  }
  markDependentsDirty(memo.dependents);
  if (memo.status !== 'Dirty') {
    detach(memo);
  }
  node.memo = MISSING;
}

function markDependentsDirty(dependents: Set<DerivedNodeMemo<unknown>>): void {
  const stack: Array<DerivedNodeMemo<unknown>> = [];
  for (const memo of dependents) {
    switch (memo.status) {
      case 'Clean':
        detach(memo);
        memo.status = 'Dirty';
        stack.push(memo);
        break;
      case 'Check':
        detach(memo);
        memo.status = 'Dirty';
        break;
      case 'Dirty':
        break;
      default:
        assertNever(memo.status);
    }
  }
  let memo: DerivedNodeMemo<unknown> | undefined;
  while ((memo = stack.pop()) !== undefined) {
    for (const dependent of memo.dependents) {
      if (dependent.status !== 'Clean') {
        continue;
      }
      dependent.status = 'Check';
      stack.push(dependent);
    }
  }
}

function verify<TValue>(memo: DerivedNodeMemo<TValue>): TValue {
  for (const dependency of memo.derivedDependencies) {
    pull(dependency);
    if (memo.status === 'Dirty') {
      return recompute(memo);
    }
  }
  memo.status = 'Clean';
  return memo.value;
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const memo: DerivedNodeMemo<TValue> = {
    value: undefined as unknown as TValue,
    status: 'Clean',
    sourceDependencies: [],
    derivedDependencies: [],
    dependents: new Set(),
    node,
  };
  node.memo = memo;
  const value = node.computation(memo, undefined);
  memo.value = value;
  attach(memo);
  return value;
}

function recompute<TValue>(memo: DerivedNodeMemo<TValue>): TValue {
  const value = memo.node.computation(memo, memo.value);
  attach(memo);
  memo.status = 'Clean';
  if (value === memo.value) {
    return memo.value;
  }
  memo.value = value;
  markDependentsDirty(memo.dependents);
  return value;
}

function attach(memo: DerivedNodeMemo<unknown>): void {
  for (const source of memo.sourceDependencies) {
    source.dependents.add(memo);
  }
  for (const dependency of memo.derivedDependencies) {
    memoOf(dependency).dependents.add(memo);
  }
}

function detach(memo: DerivedNodeMemo<unknown>): void {
  for (const source of memo.sourceDependencies) {
    source.dependents.delete(memo);
  }
  for (const dependency of memo.derivedDependencies) {
    memoOf(dependency).dependents.delete(memo);
  }
  memo.sourceDependencies.length = 0;
  memo.derivedDependencies.length = 0;
}

function memoOf<TValue>(dependency: DerivedNode<TValue>): DerivedNodeMemo<TValue> {
  const memo = dependency.memo;
  if (memo === MISSING) {
    throw new Error('A derived dependency of a memo that is not Dirty has no memo');
  }
  return memo;
}

function assertNever(value: never): never {
  throw new Error(`Unhandled variant: ${String(value)}`);
}
```

- A `Dirty` memo is in no `dependents` set and has empty dependency lists. Writes to the dependencies of a `Dirty` node therefore never reach it. Deleting the memo from the set being iterated is safe, because a `Set` iterator continues past a deleted current element.
- A memo is in no `dependents` set while its computation runs, because a recomputing memo is `Dirty` and a new memo is attached only after its first computation returns. Nothing marks the memo during its computation, so `recompute` always receives a detached memo with empty dependency lists and only the computation's own reads fill them.
- `markDependentsDirty` pushes a memo only when it leaves `Clean`, so the marks are the visited set: each memo is pushed at most once and each `dependents` set is walked at most once per write. It allocates one stack array per call and no closure.
- `verify` recomputes when pulling a dependency marked its memo `Dirty`. A dependency's `recompute` marks its dependents `Dirty` exactly when its new value is not `===` its previous value, so the memo's status is the dependency's verdict.
- `computeFirst` creates the memo before computing, so the computation's reads have a memo to push onto. The memo's `value` holds `undefined` until the computation returns. No computation reads it, because the derived nodes form a directed acyclic graph.
- Every derived dependency of a memo that is not `Dirty` has a memo, and that memo's `dependents` set holds the reader. `readDerived` pulls a node before pushing it, a computation does not dispose a node, and disposal marks every reader `Dirty` before it resets `node.memo`. A dependency read twice is one member of its set.
- Disposal empties the memo's `dependents` set, because marking a reader `Dirty` detaches it. A disposed node's next pull computes it for the first time, with `previous` `undefined`.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  markSourceDirty,
  pull,
  readDerived,
  readSource,
  writeSource,
} from './graph';
export type { DerivedNode, DerivedNodeMemo, SourceNode, Status } from './graph';
```

## Tests

```ts
// from libs/pico-js/test/graph.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  markSourceDirty,
  pull,
  readDerived,
  readSource,
  writeSource,
  type DerivedNode,
} from '../src/index';

function statusOf(node: DerivedNode<unknown>): 'NotComputed' | 'Clean' | 'Check' | 'Dirty' {
  const memo = node.memo;
  return typeof memo === 'symbol' ? 'NotComputed' : memo.status;
}

describe('pull', () => {
  test('creating a derived node runs nothing until it is pulled', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) * 2;
    });
    expect(runs).toBe(0);
    expect(statusOf(double)).toBe('NotComputed');
    expect(pull(double)).toBe(2);
    expect(pull(double)).toBe(2);
    expect(runs).toBe(1);
  });

  test('a node created inside a computation and read through readDerived is a dependency of its reader', () => {
    const source = createSourceNode(1);
    let inner: DerivedNode<number> | undefined;
    const outer = createDerivedNode((consumer) => {
      inner ??= createDerivedNode((innerConsumer) => readSource(innerConsumer, source) * 10);
      return readDerived(consumer, inner);
    });
    expect(pull(outer)).toBe(10);
    writeSource(source, 2);
    expect(statusOf(outer)).toBe('Check');
    expect(pull(outer)).toBe(20);
  });

  test('the first computation receives undefined as previous and a recompute receives the previous value', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode<number>((consumer, previous) => {
      previousValues.push(previous);
      return readSource(consumer, a) * 2;
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
    const sum = createDerivedNode((consumer) => {
      runs.sum += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    });
    const label = createDerivedNode((consumer) => {
      runs.label += 1;
      return readDerived(consumer, sum) > 2 ? 'big' : 'small';
    });
    expect(pull(label)).toBe('big');
    writeSource(a, 5);
    expect(statusOf(sum)).toBe('Dirty');
    expect(statusOf(label)).toBe('Check');
    expect(runs).toEqual({ sum: 1, label: 1 });
  });

  test('a write marks readers even when the value is the same', () => {
    const a = createSourceNode(1);
    const double = createDerivedNode((consumer) => readSource(consumer, a) * 2);
    pull(double);
    writeSource(a, 1);
    expect(statusOf(double)).toBe('Dirty');
  });

  test('markSourceDirty marks readers without changing the value', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) * 2;
    });
    pull(double);
    markSourceDirty(a);
    expect(statusOf(double)).toBe('Dirty');
    expect(pull(double)).toBe(2);
    expect(runs).toBe(2);
  });

  test('in a diamond, a write marks the shared node Dirty and every node above it Check', () => {
    const source = createSourceNode(1);
    const shared = createDerivedNode((consumer) => readSource(consumer, source) + 1);
    const left = createDerivedNode((consumer) => readDerived(consumer, shared) * 2);
    const right = createDerivedNode((consumer) => readDerived(consumer, shared) * 3);
    const top = createDerivedNode((consumer) => readDerived(consumer, left) + readDerived(consumer, right));
    pull(top);
    writeSource(source, 2);
    expect([statusOf(shared), statusOf(left), statusOf(right), statusOf(top)]).toEqual([
      'Dirty',
      'Check',
      'Check',
      'Check',
    ]);
  });

  test('marking a chain of 100,000 nodes completes', () => {
    const source = createSourceNode(0);
    const bottom = createDerivedNode((consumer) => readSource(consumer, source));
    pull(bottom);
    let top = bottom;
    for (let index = 1; index < 100_000; index += 1) {
      const below = top;
      top = createDerivedNode((consumer) => readDerived(consumer, below));
      pull(top);
    }
    writeSource(source, 1);
    expect(statusOf(bottom)).toBe('Dirty');
    expect(statusOf(top)).toBe('Check');
  });
});

describe('cutoff', () => {
  test('a dependency that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((consumer) => {
      runs.sum += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    });
    const label = createDerivedNode((consumer) => {
      runs.label += 1;
      return readDerived(consumer, sum) > 2 ? 'big' : 'small';
    });
    pull(label);
    writeSource(a, 0);
    writeSource(b, 3);
    expect(pull(label)).toBe('big');
    expect(statusOf(label)).toBe('Clean');
    expect(runs).toEqual({ sum: 2, label: 1 });
  });

  test('a computation that returns previous leaves its readers Clean without running them', () => {
    let readerRuns = 0;
    const a = createSourceNode(1);
    const parity = createDerivedNode<{ readonly even: boolean }>((consumer, previous) => {
      const even = readSource(consumer, a) % 2 === 0;
      return previous !== undefined && previous.even === even ? previous : { even };
    });
    const reader = createDerivedNode((consumer) => {
      readerRuns += 1;
      return readDerived(consumer, parity).even;
    });
    const first = pull(parity);
    pull(reader);
    writeSource(a, 3);
    expect(pull(reader)).toBe(false);
    expect(statusOf(reader)).toBe('Clean');
    expect(pull(parity)).toBe(first);
    expect(readerRuns).toBe(1);
  });

  test('a projection of a new object that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const d1 = createDerivedNode((consumer) => {
      runs.d1 += 1;
      return { a: readSource(consumer, a), b: readSource(consumer, b) };
    });
    const d2 = createDerivedNode((consumer) => {
      runs.d2 += 1;
      return readDerived(consumer, d1).b;
    });
    const d3 = createDerivedNode((consumer) => {
      runs.d3 += 1;
      return readDerived(consumer, d2) * 2;
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
    const shared = createDerivedNode((consumer) => {
      runs.shared += 1;
      return readSource(consumer, source) + 1;
    });
    const left = createDerivedNode((consumer) => {
      runs.left += 1;
      return readDerived(consumer, shared) * 2;
    });
    const right = createDerivedNode((consumer) => {
      runs.right += 1;
      return readDerived(consumer, shared) * 3;
    });
    const top = createDerivedNode((consumer) => {
      runs.top += 1;
      return readDerived(consumer, left) + readDerived(consumer, right);
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
    const gate = createDerivedNode((consumer) => {
      runs.gate += 1;
      return readSource(consumer, gateSource);
    });
    const other = createDerivedNode((consumer) => {
      runs.other += 1;
      return readSource(consumer, otherSource);
    });
    const node = createDerivedNode((consumer) => {
      runs.node += 1;
      return readDerived(consumer, gate) === 1 ? readDerived(consumer, other) : 0;
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
    const b = createDerivedNode((consumer) => {
      runs.b += 1;
      return readSource(consumer, bInput);
    });
    const c = createDerivedNode(() => {
      runs.c += 1;
      return 0;
    });
    const foo = createDerivedNode((consumer) =>
      readSource(consumer, a) ? readDerived(consumer, b) : readDerived(consumer, c),
    );
    expect(pull(foo)).toBe(1);
    writeSource(bInput, 2);
    writeSource(a, false);
    expect(statusOf(foo)).toBe('Dirty');
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
    const pick = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, useA) ? readSource(consumer, a) : readSource(consumer, b);
    });
    expect(pull(pick)).toBe(1);
    writeSource(useA, false);
    expect(pull(pick)).toBe(2);
    writeSource(a, 10);
    expect(statusOf(pick)).toBe('Clean');
    expect(pull(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of them', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((consumer) => readSource(consumer, a) + readSource(consumer, b));
    pull(sum);
    writeSource(a, 3);
    expect(pull(sum)).toBe(5);
    writeSource(b, 4);
    expect(statusOf(sum)).toBe('Dirty');
    expect(pull(sum)).toBe(7);
  });

  test('a Dirty node is not marked again by a second write to another of its sources', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    });
    const label = createDerivedNode((consumer) => `${readDerived(consumer, sum)}`);
    pull(label);
    writeSource(a, 3);
    writeSource(b, 4);
    expect(statusOf(sum)).toBe('Dirty');
    expect(statusOf(label)).toBe('Check');
    expect(pull(label)).toBe('7');
    expect(runs).toBe(2);
    writeSource(a, 5);
    expect(statusOf(sum)).toBe('Dirty');
    expect(pull(label)).toBe('9');
  });

  test('a node read twice is marked once and computes once per change', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const twice = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) + readSource(consumer, a);
    });
    expect(pull(twice)).toBe(2);
    writeSource(a, 2);
    expect(pull(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('disposal', () => {
  test('disposing a node marks its readers Dirty and its next pull computes it from scratch', () => {
    const runs = { inner: 0, outer: 0 };
    const innerPreviousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const inner = createDerivedNode<number>((consumer, previous) => {
      runs.inner += 1;
      innerPreviousValues.push(previous);
      return readSource(consumer, source);
    });
    const outer = createDerivedNode((consumer) => {
      runs.outer += 1;
      return readDerived(consumer, inner) + 1;
    });
    expect(pull(outer)).toBe(2);
    disposeDerivedNode(inner);
    expect(statusOf(inner)).toBe('NotComputed');
    expect(statusOf(outer)).toBe('Dirty');
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
    writeSource(source, 2);
    expect(statusOf(outer)).toBe('Check');
    expect(pull(outer)).toBe(3);
    expect(innerPreviousValues).toEqual([undefined, undefined, 1]);
  });

  test('a disposed node is not marked by the sources it read', () => {
    const source = createSourceNode(1);
    const inner = createDerivedNode((consumer) => readSource(consumer, source));
    pull(inner);
    disposeDerivedNode(inner);
    writeSource(source, 2);
    expect(statusOf(inner)).toBe('NotComputed');
    expect(pull(inner)).toBe(2);
  });

  test('disposing a node that was never computed changes nothing', () => {
    const source = createSourceNode(1);
    const node = createDerivedNode((consumer) => readSource(consumer, source));
    disposeDerivedNode(node);
    expect(statusOf(node)).toBe('NotComputed');
    expect(pull(node)).toBe(1);
  });
});
```

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import { createDerivedNode, readDerived, type DerivedNode } from '../src/index';

test('a derived node of any value type can be read by any consumer', () => {
  const count = createDerivedNode(() => 1);
  const label = createDerivedNode((consumer) => `${readDerived(consumer, count)}`);
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});
```
