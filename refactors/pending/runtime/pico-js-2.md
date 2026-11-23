# pico-js: implementation

This doc is the complete source of `@isograph/pico-js` in `libs/pico-js`. The package layout, `package.json`, build, hooks, and CI are as pico-js.md (The package) specifies; this doc gives the source files and the tests.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and, once computed, a `DerivedNodeMemo`: the value, a status, and the nodes the computation read.
- A write marks the memos of the source's direct readers `Dirty` and the memos above them `Check`. It runs no computation.
- `pull` brings a node up to date. A `Clean` memo returns its value. A `Check` memo pulls its derived dependencies in the order they were read and recomputes at the first whose value changed; if none changed it becomes `Clean` without running. A `Dirty` memo recomputes. A node with no memo computes for the first time.
- After a recompute, a value that the node's `equals` accepts keeps the previous value object and marks nothing.
- A computation records its reads in two buffers on its node. After it returns, the buffers become the memo's dependency lists and the reverse edges are added, in one synchronous step with the new value.
- Marking a memo `Dirty` removes it from the `dependents` set of everything it read. Its next computation adds it to the sets of what it reads then.
- `dependents` sets hold memos. A memo exists only after its node has computed, so a `dependents` set never holds an uncomputed node.

Contracts the engine does not check:

- A computation and an `equals` do not throw, do not write a source, and do not dispose a node.
- A computation is synchronous and reads only through the node it receives.
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
  equals(previous: TValue, next: TValue): boolean;
  readonly dependents: Set<DerivedNodeMemo<unknown>>;
};

export type DerivedNode<TValue> = {
  computation(consumer: DerivedNode<unknown>): TValue;
  equals(previous: TValue, next: TValue): boolean;
  readonly dependents: Set<DerivedNodeMemo<unknown>>;
  memo: Option<DerivedNodeMemo<TValue>>;
  sourceReads: Array<SourceNode<unknown>>;
  derivedReads: Array<DerivedNode<unknown>>;
};

export type DerivedNodeMemo<TValue> = {
  value: TValue;
  status: Status;
  sourceDependencies: Array<SourceNode<unknown>>;
  derivedDependencies: Array<DerivedNode<unknown>>;
  readonly node: DerivedNode<TValue>;
};

export function createSourceNode<TValue>(
  value: TValue,
  equals: (previous: TValue, next: TValue) => boolean,
): SourceNode<TValue> {
  return { value, equals, dependents: new Set() };
}

export function createDerivedNode<TValue>(
  computation: (consumer: DerivedNode<unknown>) => TValue,
  equals: (previous: TValue, next: TValue) => boolean,
): DerivedNode<TValue> {
  return {
    computation,
    equals,
    dependents: new Set(),
    memo: MISSING,
    sourceReads: [],
    derivedReads: [],
  };
}

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

export function readSource<TValue>(consumer: DerivedNode<unknown>, source: SourceNode<TValue>): TValue {
  consumer.sourceReads.push(source);
  return source.value;
}

export function readDerived<TValue>(consumer: DerivedNode<unknown>, node: DerivedNode<TValue>): TValue {
  const value = pull(node);
  consumer.derivedReads.push(node);
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
  if (memo.status !== 'Dirty') {
    detach(memo);
  }
  node.memo = MISSING;
  markDependentsDirty(node.dependents);
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
    for (const dependent of memo.node.dependents) {
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
  const value = compute(node);
  const memo: DerivedNodeMemo<TValue> = {
    value,
    status: 'Clean',
    sourceDependencies: node.sourceReads,
    derivedDependencies: node.derivedReads,
    node,
  };
  node.sourceReads = [];
  node.derivedReads = [];
  node.memo = memo;
  attach(memo);
  return value;
}

function recompute<TValue>(memo: DerivedNodeMemo<TValue>): TValue {
  const node = memo.node;
  const value = compute(node);
  const previousSources = memo.sourceDependencies;
  memo.sourceDependencies = node.sourceReads;
  node.sourceReads = previousSources;
  const previousDerived = memo.derivedDependencies;
  memo.derivedDependencies = node.derivedReads;
  node.derivedReads = previousDerived;
  attach(memo);
  memo.status = 'Clean';
  if (node.equals(memo.value, value)) {
    return memo.value;
  }
  memo.value = value;
  markDependentsDirty(node.dependents);
  return value;
}

function compute<TValue>(node: DerivedNode<TValue>): TValue {
  node.sourceReads.length = 0;
  node.derivedReads.length = 0;
  return node.computation(node);
}

function attach(memo: DerivedNodeMemo<unknown>): void {
  for (const source of memo.sourceDependencies) {
    source.dependents.add(memo);
  }
  for (const dependency of memo.derivedDependencies) {
    dependency.dependents.add(memo);
  }
}

function detach(memo: DerivedNodeMemo<unknown>): void {
  for (const source of memo.sourceDependencies) {
    source.dependents.delete(memo);
  }
  for (const dependency of memo.derivedDependencies) {
    dependency.dependents.delete(memo);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled variant: ${String(value)}`);
}
```

- A `Dirty` memo is in no `dependents` set. `markDependentsDirty` detaches a memo when it marks it `Dirty`, and `computeFirst` and `recompute` attach the memo's new dependencies when they make it `Clean`. Writes to the dependencies of a `Dirty` node therefore never reach it. Deleting the memo from the set being iterated is safe, because a `Set` iterator continues past a deleted current element.
- `markDependentsDirty` pushes a memo only when it leaves `Clean`, so the marks are the visited set: each memo is pushed at most once and each `dependents` set is walked at most once per write. It allocates one stack array per call and no closure.
- `verify` recomputes when pulling a dependency marked its memo `Dirty`. A dependency's `recompute` marks its dependents `Dirty` exactly when its `equals` rejected the new value, so the memo's status is the dependency's verdict, and the verified node is `Dirty` and therefore detached whenever it recomputes.
- `recompute` swaps each reads buffer with the memo's previous dependency list, so a computation allocates no array, and then attaches every dependency in the new lists. A dependency read twice is one member of its set.
- `computeFirst` marks nothing. A node with no memo has no dependents that are not already `Dirty`: a memo enters a `dependents` set only after its node computed, and disposal marks the dependents of the disposed node `Dirty`.
- `disposeDerivedNode` detaches the memo unless it is already `Dirty`, then resets `node.memo`. A disposed node's next pull computes it for the first time.

## `src/equality.ts`

```ts
// from libs/pico-js/src/equality.ts
export function tripleEquals<TValue>(previous: TValue, next: TValue): boolean {
  return previous === next;
}

export function objectIs<TValue>(previous: TValue, next: TValue): boolean {
  return Object.is(previous, next);
}
```

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
export { objectIs, tripleEquals } from './equality';
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
  objectIs,
  pull,
  readDerived,
  readSource,
  tripleEquals,
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
    const a = createSourceNode(1, tripleEquals);
    const double = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) * 2;
    }, tripleEquals);
    expect(runs).toBe(0);
    expect(statusOf(double)).toBe('NotComputed');
    expect(pull(double)).toBe(2);
    expect(pull(double)).toBe(2);
    expect(runs).toBe(1);
  });

  test('a node created inside a computation and read through readDerived is a dependency of its reader', () => {
    const source = createSourceNode(1, tripleEquals);
    let inner: DerivedNode<number> | undefined;
    const outer = createDerivedNode((consumer) => {
      inner ??= createDerivedNode((innerConsumer) => readSource(innerConsumer, source) * 10, tripleEquals);
      return readDerived(consumer, inner);
    }, tripleEquals);
    expect(pull(outer)).toBe(10);
    writeSource(source, 2);
    expect(statusOf(outer)).toBe('Check');
    expect(pull(outer)).toBe(20);
  });
});

describe('marking', () => {
  test('a write marks direct readers Dirty and their readers Check and runs nothing', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const sum = createDerivedNode((consumer) => {
      runs.sum += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    }, tripleEquals);
    const label = createDerivedNode((consumer) => {
      runs.label += 1;
      return readDerived(consumer, sum) > 2 ? 'big' : 'small';
    }, tripleEquals);
    expect(pull(label)).toBe('big');
    writeSource(a, 5);
    expect(statusOf(sum)).toBe('Dirty');
    expect(statusOf(label)).toBe('Check');
    expect(runs).toEqual({ sum: 1, label: 1 });
  });

  test('writing an equal value marks nothing', () => {
    const a = createSourceNode(1, tripleEquals);
    const double = createDerivedNode((consumer) => readSource(consumer, a) * 2, tripleEquals);
    pull(double);
    writeSource(a, 1);
    expect(statusOf(double)).toBe('Clean');
  });

  test('markSourceDirty marks readers without comparing values', () => {
    let runs = 0;
    const a = createSourceNode(1, tripleEquals);
    const double = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) * 2;
    }, tripleEquals);
    pull(double);
    markSourceDirty(a);
    expect(statusOf(double)).toBe('Dirty');
    expect(pull(double)).toBe(2);
    expect(runs).toBe(2);
  });

  test('in a diamond, a write marks the shared node Dirty and every node above it Check', () => {
    const source = createSourceNode(1, tripleEquals);
    const shared = createDerivedNode((consumer) => readSource(consumer, source) + 1, tripleEquals);
    const left = createDerivedNode((consumer) => readDerived(consumer, shared) * 2, tripleEquals);
    const right = createDerivedNode((consumer) => readDerived(consumer, shared) * 3, tripleEquals);
    const top = createDerivedNode(
      (consumer) => readDerived(consumer, left) + readDerived(consumer, right),
      tripleEquals,
    );
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
    const source = createSourceNode(0, tripleEquals);
    const bottom = createDerivedNode((consumer) => readSource(consumer, source), tripleEquals);
    pull(bottom);
    let top = bottom;
    for (let index = 1; index < 100_000; index += 1) {
      const below = top;
      top = createDerivedNode((consumer) => readDerived(consumer, below), tripleEquals);
      pull(top);
    }
    writeSource(source, 1);
    expect(statusOf(bottom)).toBe('Dirty');
    expect(statusOf(top)).toBe('Check');
  });
});

describe('cutoff', () => {
  test('a dependency that recomputes to an equal value leaves its reader Clean without running it', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const sum = createDerivedNode((consumer) => {
      runs.sum += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    }, tripleEquals);
    const label = createDerivedNode((consumer) => {
      runs.label += 1;
      return readDerived(consumer, sum) > 2 ? 'big' : 'small';
    }, tripleEquals);
    pull(label);
    writeSource(a, 0);
    writeSource(b, 3);
    expect(pull(label)).toBe('big');
    expect(statusOf(label)).toBe('Clean');
    expect(runs).toEqual({ sum: 2, label: 1 });
  });

  test('a recompute whose value equals the previous one returns the previous object', () => {
    const a = createSourceNode(1, tripleEquals);
    const pair = createDerivedNode(
      (consumer) => ({ even: readSource(consumer, a) % 2 === 0 }),
      (previous, next) => previous.even === next.even,
    );
    const first = pull(pair);
    writeSource(a, 3);
    expect(pull(pair)).toBe(first);
  });

  test('the supplied equals decides whether a change from 0 to -0 reaches readers', () => {
    const strict = createSourceNode(0, tripleEquals);
    const exact = createSourceNode(0, objectIs);
    const strictReader = createDerivedNode((consumer) => readSource(consumer, strict), tripleEquals);
    const exactReader = createDerivedNode((consumer) => readSource(consumer, exact), objectIs);
    pull(strictReader);
    pull(exactReader);
    writeSource(strict, -0);
    writeSource(exact, -0);
    expect(statusOf(strictReader)).toBe('Clean');
    expect(statusOf(exactReader)).toBe('Dirty');
    expect(Object.is(pull(exactReader), -0)).toBe(true);
  });
});

describe('verification', () => {
  test('in a diamond each node runs once per change', () => {
    const source = createSourceNode(1, tripleEquals);
    const runs = { shared: 0, left: 0, right: 0, top: 0 };
    const shared = createDerivedNode((consumer) => {
      runs.shared += 1;
      return readSource(consumer, source) + 1;
    }, tripleEquals);
    const left = createDerivedNode((consumer) => {
      runs.left += 1;
      return readDerived(consumer, shared) * 2;
    }, tripleEquals);
    const right = createDerivedNode((consumer) => {
      runs.right += 1;
      return readDerived(consumer, shared) * 3;
    }, tripleEquals);
    const top = createDerivedNode((consumer) => {
      runs.top += 1;
      return readDerived(consumer, left) + readDerived(consumer, right);
    }, tripleEquals);
    expect(pull(top)).toBe(10);
    writeSource(source, 2);
    expect(pull(top)).toBe(15);
    expect(runs).toEqual({ shared: 2, left: 2, right: 2, top: 2 });
  });

  test('verification stops at the first derived dependency whose value changed', () => {
    const gateSource = createSourceNode(1, tripleEquals);
    const otherSource = createSourceNode(1, tripleEquals);
    const runs = { gate: 0, other: 0, node: 0 };
    const gate = createDerivedNode((consumer) => {
      runs.gate += 1;
      return readSource(consumer, gateSource);
    }, tripleEquals);
    const other = createDerivedNode((consumer) => {
      runs.other += 1;
      return readSource(consumer, otherSource);
    }, tripleEquals);
    const node = createDerivedNode((consumer) => {
      runs.node += 1;
      return readDerived(consumer, gate) === 1 ? readDerived(consumer, other) : 0;
    }, tripleEquals);
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
    const a = createSourceNode(true, tripleEquals);
    const bInput = createSourceNode(1, tripleEquals);
    const b = createDerivedNode((consumer) => {
      runs.b += 1;
      return readSource(consumer, bInput);
    }, tripleEquals);
    const c = createDerivedNode(() => {
      runs.c += 1;
      return 0;
    }, tripleEquals);
    const foo = createDerivedNode(
      (consumer) => (readSource(consumer, a) ? readDerived(consumer, b) : readDerived(consumer, c)),
      tripleEquals,
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
  test('a computation that stops reading a source is no longer marked by it', () => {
    let runs = 0;
    const useA = createSourceNode(true, tripleEquals);
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const pick = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, useA) ? readSource(consumer, a) : readSource(consumer, b);
    }, tripleEquals);
    expect(pull(pick)).toBe(1);
    writeSource(useA, false);
    expect(pull(pick)).toBe(2);
    writeSource(a, 10);
    expect(statusOf(pick)).toBe('Clean');
    expect(pull(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of them', () => {
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const sum = createDerivedNode((consumer) => readSource(consumer, a) + readSource(consumer, b), tripleEquals);
    pull(sum);
    writeSource(a, 3);
    expect(pull(sum)).toBe(5);
    writeSource(b, 4);
    expect(statusOf(sum)).toBe('Dirty');
    expect(pull(sum)).toBe(7);
  });

  test('a Dirty node is not marked again by a second write to another of its sources', () => {
    let runs = 0;
    const a = createSourceNode(1, tripleEquals);
    const b = createSourceNode(2, tripleEquals);
    const sum = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) + readSource(consumer, b);
    }, tripleEquals);
    const label = createDerivedNode((consumer) => `${readDerived(consumer, sum)}`, tripleEquals);
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

  test('a dependency whose equals rejects a value Object.is accepts makes its reader recompute', () => {
    let runs = 0;
    const source = createSourceNode(Number.NaN, objectIs);
    const value = createDerivedNode((consumer) => readSource(consumer, source), tripleEquals);
    const reader = createDerivedNode((consumer) => {
      runs += 1;
      return readDerived(consumer, value);
    }, objectIs);
    pull(reader);
    markSourceDirty(source);
    pull(reader);
    expect(runs).toBe(2);
    writeSource(source, 1);
    expect(statusOf(reader)).toBe('Check');
    expect(pull(reader)).toBe(1);
  });

  test('a node read twice is marked once and computes once per change', () => {
    let runs = 0;
    const a = createSourceNode(1, tripleEquals);
    const twice = createDerivedNode((consumer) => {
      runs += 1;
      return readSource(consumer, a) + readSource(consumer, a);
    }, tripleEquals);
    expect(pull(twice)).toBe(2);
    writeSource(a, 2);
    expect(pull(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('disposal', () => {
  test('disposing a node marks its readers Dirty and its next pull computes it from scratch', () => {
    const runs = { inner: 0, outer: 0 };
    const source = createSourceNode(1, tripleEquals);
    const inner = createDerivedNode((consumer) => {
      runs.inner += 1;
      return readSource(consumer, source);
    }, tripleEquals);
    const outer = createDerivedNode((consumer) => {
      runs.outer += 1;
      return readDerived(consumer, inner) + 1;
    }, tripleEquals);
    expect(pull(outer)).toBe(2);
    disposeDerivedNode(inner);
    expect(statusOf(inner)).toBe('NotComputed');
    expect(statusOf(outer)).toBe('Dirty');
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
  });

  test('a disposed node is not marked by the sources it read', () => {
    const source = createSourceNode(1, tripleEquals);
    const inner = createDerivedNode((consumer) => readSource(consumer, source), tripleEquals);
    pull(inner);
    disposeDerivedNode(inner);
    writeSource(source, 2);
    expect(statusOf(inner)).toBe('NotComputed');
    expect(pull(inner)).toBe(2);
  });

  test('disposing a node that was never computed changes nothing', () => {
    const source = createSourceNode(1, tripleEquals);
    const node = createDerivedNode((consumer) => readSource(consumer, source), tripleEquals);
    disposeDerivedNode(node);
    expect(statusOf(node)).toBe('NotComputed');
    expect(pull(node)).toBe(1);
  });
});
```

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import { createDerivedNode, createSourceNode, readDerived, tripleEquals, type DerivedNode } from '../src/index';

test('a derived node of any value type can be read by any consumer', () => {
  const count = createDerivedNode(() => 1, tripleEquals);
  const label = createDerivedNode((consumer) => `${readDerived(consumer, count)}`, tripleEquals);
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});

test('an equals over another type is a type error', () => {
  // @ts-expect-error
  createSourceNode(1, (previous: string, next: string) => previous === next);
});
```
