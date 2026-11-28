import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { at } from './at';
import { internal } from './internal';
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  endWriteBatch,
  readDerived,
  readDerivedNode,
  readSource,
  releaseRetain,
  retainDerivedNode,
  scheduler,
  startWriteBatch,
  writeSource,
  type CallingNode,
  type DerivedNode,
  type Retain,
  type SourceNode,
} from '../src/index';
import { MAYBE_DIRTY, CLEAN, DIRTY, UNCOMPUTED } from '../src/nodes';
import { statusOf } from './status';
import { countStatusReads } from './status-reads';

beforeEach(() => {
  scheduler.schedule = disposeAll;
});

afterEach(disposeAll);

function writeInBatch<TValue>(source: SourceNode<TValue>, value: TValue): void {
  startWriteBatch();
  writeSource(source, value);
  endWriteBatch();
}

const retainedByRead = new WeakSet<DerivedNode<unknown>>();

function read<TValue>(node: DerivedNode<TValue>): TValue {
  if (!retainedByRead.has(node)) {
    retainedByRead.add(node);
    retainDerivedNode(node);
  }
  return readDerivedNode(node);
}

/** The first read retains the node with an `onChange` that does nothing, so the flush recomputes the node. */
function readObserved<TValue>(node: DerivedNode<TValue>): TValue {
  if (!retainedByRead.has(node)) {
    retainedByRead.add(node);
    retainDerivedNode(node, () => {});
  }
  return readDerivedNode(node);
}

const writers: ReadonlyArray<
  readonly [string, (source: SourceNode<number>, value: number) => void]
> = [
  ['writeSource', writeSource],
  ['writeSource in a batch', writeInBatch],
];

describe('tracked reads and top-level reads', () => {
  test('a derived node recomputes nothing until a top-level read reaches it, and a second top-level read of the CLEAN node recomputes nothing', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const double = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) * 2;
    });
    expect(runs).toBe(0);
    expect(statusOf(double)).toBe(UNCOMPUTED);
    expect(read(double)).toBe(2);
    expect(statusOf(double)).toBe(CLEAN);
    expect(read(double)).toBe(2);
    expect(runs).toBe(1);
  });

  test('a derived node created inside a computation and passed to readDerived is a dependency of the node being recomputed', () => {
    const s1 = createSourceNode(1);
    let inner: DerivedNode<number> | undefined;
    const d1 = createDerivedNode((callingNode) => {
      inner ??= createDerivedNode(
        (innerCallingNode) => readSource(innerCallingNode, s1) * 10,
      );
      return readDerived(callingNode, inner);
    });
    expect(read(d1)).toBe(10);
    writeInBatch(s1, 2);
    expect(statusOf(d1)).toBe(MAYBE_DIRTY);
    expect(read(d1)).toBe(20);
  });

  test('the first recompute receives undefined as previous and a later recompute receives the previous value', () => {
    const previousValues: Array<number | undefined> = [];
    const s1 = createSourceNode(1);
    const double = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, s1) * 2;
    });
    read(double);
    writeInBatch(s1, 2);
    read(double);
    expect(previousValues).toEqual([undefined, 2]);
  });
});

describe('marking', () => {
  test('a write marks the dependents of a source DIRTY and the dependents of those MAYBE_DIRTY, and recomputes nothing', () => {
    const runs = { sum: 0, label: 0 };
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, s1) + readSource(callingNode, s2);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    expect(read(label)).toBe('big');
    writeInBatch(s1, 5);
    expect(statusOf(sum)).toBe(DIRTY);
    expect(statusOf(label)).toBe(MAYBE_DIRTY);
    expect(runs).toEqual({ sum: 1, label: 1 });
  });

  test('in a diamond, a write marks the shared node DIRTY and every node that depends on it, directly or through other nodes, MAYBE_DIRTY', () => {
    const s1 = createSourceNode(1);
    const shared = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) + 1,
    );
    const d1 = createDerivedNode(
      (callingNode) => readDerived(callingNode, shared) * 2,
    );
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, shared) * 3,
    );
    const d3 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d1) + readDerived(callingNode, d2),
    );
    read(d3);
    writeInBatch(s1, 2);
    expect([
      statusOf(shared),
      statusOf(d1),
      statusOf(d2),
      statusOf(d3),
    ]).toEqual([DIRTY, MAYBE_DIRTY, MAYBE_DIRTY, MAYBE_DIRTY]);
  });

  test('a write marks a chain of 100,000 derived nodes', () => {
    const s1 = createSourceNode(0);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    read(d1);
    const chainEnd = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (dependency) => {
        const dependent = createDerivedNode(
          (callingNode) => readDerived(callingNode, dependency) + 1,
        );
        read(dependent);
        return dependent;
      },
      d1,
    );
    writeInBatch(s1, 1);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(statusOf(chainEnd)).toBe(MAYBE_DIRTY);
  });
});

describe('cutoff', () => {
  test('a dependency that recomputes to the same value leaves its dependent CLEAN without recomputing it', () => {
    const runs = { sum: 0, label: 0 };
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, s1) + readSource(callingNode, s2);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    read(label);
    writeInBatch(s1, 0);
    writeInBatch(s2, 3);
    expect(read(label)).toBe('big');
    expect(statusOf(label)).toBe(CLEAN);
    expect(runs).toEqual({ sum: 2, label: 1 });
  });

  test('a computation that returns previous leaves its dependents CLEAN without recomputing them', () => {
    let readerRuns = 0;
    const s1 = createSourceNode(1);
    const parity = createDerivedNode<{ readonly even: boolean }>(
      (callingNode, previous) => {
        const even = readSource(callingNode, s1) % 2 === 0;
        return previous !== undefined && previous.even === even
          ? previous
          : { even };
      },
    );
    const d1 = createDerivedNode((callingNode) => {
      readerRuns += 1;
      return readDerived(callingNode, parity).even;
    });
    const first = read(parity);
    read(d1);
    writeInBatch(s1, 3);
    expect(read(d1)).toBe(false);
    expect(statusOf(d1)).toBe(CLEAN);
    expect(read(parity)).toBe(first);
    expect(readerRuns).toBe(1);
  });

  test('a node whose computation returns undefined is CLEAN, and a recompute that returns undefined again leaves its dependent CLEAN without recomputing it', () => {
    const runs = { d1: 0, d2: 0 };
    const previousValues: Array<undefined> = [];
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode<undefined>((callingNode, previous) => {
      runs.d1 += 1;
      previousValues.push(previous);
      readSource(callingNode, s1);
      return undefined;
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1) === undefined;
    });
    expect(read(d2)).toBe(true);
    expect(statusOf(d1)).toBe(CLEAN);
    expect(read(d1)).toBeUndefined();
    expect(runs).toEqual({ d1: 1, d2: 1 });
    writeInBatch(s1, 2);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(read(d2)).toBe(true);
    expect(statusOf(d2)).toBe(CLEAN);
    expect(runs).toEqual({ d1: 2, d2: 1 });
    expect(previousValues).toEqual([undefined, undefined]);
  });

  test('a projection of a new object that recomputes to the same value leaves its dependent CLEAN without recomputing it', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return { a: readSource(callingNode, s1), b: readSource(callingNode, s2) };
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1).b;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d2) * 2;
    });
    expect(read(d3)).toBe(4);
    writeInBatch(s1, 10);
    expect(read(d3)).toBe(4);
    expect(runs).toEqual({ d1: 2, d2: 2, d3: 1 });
  });
});

describe('verification', () => {
  test('in a diamond each node recomputes once per change', () => {
    const s1 = createSourceNode(1);
    const runs = { shared: 0, d1: 0, d2: 0, d3: 0 };
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, s1) + 1;
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readDerived(callingNode, shared) * 2;
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, shared) * 3;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d1) + readDerived(callingNode, d2);
    });
    expect(read(d3)).toBe(10);
    writeInBatch(s1, 2);
    expect(read(d3)).toBe(15);
    expect(runs).toEqual({ shared: 2, d1: 2, d2: 2, d3: 2 });
  });

  test('verification stops at the first derived dependency whose value changed', () => {
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const runs = { gate: 0, d1: 0, d2: 0 };
    const gate = createDerivedNode((callingNode) => {
      runs.gate += 1;
      return readSource(callingNode, s1);
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s2);
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, gate) === 1
        ? readDerived(callingNode, d1)
        : 0;
    });
    expect(read(d2)).toBe(1);
    writeInBatch(s1, 2);
    writeInBatch(s2, 2);
    expect(read(d2)).toBe(0);
    expect(runs).toEqual({ gate: 2, d1: 1, d2: 2 });
  });
});

describe('source writes and verification', () => {
  test('a node whose source dependency changed recomputes without bringing its derived dependency up to date', () => {
    const runs = { d1: 0, d2: 0 };
    const s1 = createSourceNode(true);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s2);
    });
    const d2 = createDerivedNode(() => {
      runs.d2 += 1;
      return 0;
    });
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, s1)
        ? readDerived(callingNode, d1)
        : readDerived(callingNode, d2),
    );
    expect(read(d3)).toBe(1);
    writeInBatch(s2, 2);
    writeInBatch(s1, false);
    expect(statusOf(d3)).toBe(DIRTY);
    expect(read(d3)).toBe(0);
    expect(runs).toEqual({ d1: 1, d2: 1 });
  });
});

describe('dependencies', () => {
  test('a source that a recompute stopped reading does not mark the node', () => {
    let runs = 0;
    const useA = createSourceNode(true);
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const pick = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, useA)
        ? readSource(callingNode, s1)
        : readSource(callingNode, s2);
    });
    expect(read(pick)).toBe(1);
    writeInBatch(useA, false);
    expect(read(pick)).toBe(2);
    writeInBatch(s1, 10);
    expect(statusOf(pick)).toBe(CLEAN);
    expect(read(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of those nodes', () => {
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, s1) + readSource(callingNode, s2),
    );
    read(sum);
    writeInBatch(s1, 3);
    expect(read(sum)).toBe(5);
    writeInBatch(s2, 4);
    expect(statusOf(sum)).toBe(DIRTY);
    expect(read(sum)).toBe(7);
    writeInBatch(s1, 5);
    expect(statusOf(sum)).toBe(DIRTY);
    expect(read(sum)).toBe(9);
  });

  test('a DIRTY node stays in the list of dependents of its other sources, and a second write leaves it DIRTY', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) + readSource(callingNode, s2);
    });
    const label = createDerivedNode(
      (callingNode) => `${readDerived(callingNode, sum)}`,
    );
    read(label);
    writeInBatch(s1, 3);
    expect(hasDependent(s2, sum)).toBe(true);
    writeInBatch(s2, 4);
    expect(statusOf(sum)).toBe(DIRTY);
    expect(statusOf(label)).toBe(MAYBE_DIRTY);
    expect(read(label)).toBe('7');
    expect(runs).toBe(2);
  });

  test('a computation that reads s1, s2, s1 recomputes once per write of s1 and of s2', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(10);
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return (
        readSource(callingNode, s1) +
        readSource(callingNode, s2) +
        readSource(callingNode, s1)
      );
    });
    expect(read(d1)).toBe(12);
    writeInBatch(s1, 2);
    expect(read(d1)).toBe(14);
    writeInBatch(s2, 20);
    expect(read(d1)).toBe(24);
    writeInBatch(s1, 3);
    writeInBatch(s2, 30);
    expect(read(d1)).toBe(36);
    expect(runs).toBe(4);
  });

  test('a recompute that reads fewer and then more sources is marked by exactly the sources it read last', () => {
    let runs = 0;
    const count = createSourceNode(3);
    const s1 = createSourceNode(0);
    const s2 = createSourceNode(1);
    const s3 = createSourceNode(2);
    const sources = [s1, s2, s3];
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return sources
        .slice(0, readSource(callingNode, count))
        .reduce((sum, source) => sum + readSource(callingNode, source), 0);
    });
    expect(read(d1)).toBe(3);
    writeInBatch(count, 1);
    expect(read(d1)).toBe(0);
    writeInBatch(s3, 100);
    expect(statusOf(d1)).toBe(CLEAN);
    writeInBatch(count, 3);
    expect(read(d1)).toBe(101);
    writeInBatch(s3, 200);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(read(d1)).toBe(201);
    expect(runs).toBe(4);
  });

  test('a computation that reads a source twice recomputes once per change', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const twice = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) + readSource(callingNode, s1);
    });
    expect(read(twice)).toBe(2);
    writeInBatch(s1, 2);
    expect(read(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('batches and onChange', () => {
  test('a write calls nothing until the batch is flushed', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const double = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 2,
    );
    readObserved(double);
    retainDerivedNode(double, () => calls.push('double'));
    startWriteBatch();
    writeSource(s1, 2);
    expect(calls).toEqual([]);
    endWriteBatch();
    expect(calls).toEqual(['double']);
  });

  test('two writes in one batch call each onChange once, after both writes', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    readObserved(d1);
    readObserved(d2);
    retainDerivedNode(d1, () => calls.push('readsA'));
    retainDerivedNode(d2, () => calls.push('readsB'));
    startWriteBatch();
    writeSource(s1, 2);
    writeSource(s2, 2);
    expect(calls).toEqual([]);
    endWriteBatch();
    expect(calls).toEqual(['readsA', 'readsB']);
  });

  test('an onChange that reads its node sees every write of the batch', () => {
    const seen: Array<number> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, s1) + readSource(callingNode, s2),
    );
    readObserved(sum);
    retainDerivedNode(sum, () => seen.push(readDerivedNode(sum)));
    startWriteBatch();
    writeSource(s1, 10);
    writeSource(s2, 20);
    endWriteBatch();
    expect(seen).toEqual([30]);
  });

  test('onChange receives the value of its node after the flush, which a read inside onChange also returns', () => {
    const received: Array<number> = [];
    const read: Array<number> = [];
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 2,
    );
    readObserved(d1);
    retainDerivedNode(d1, (value) => {
      received.push(value);
      read.push(readDerivedNode(d1));
    });
    writeSource(s1, 2);
    writeInBatch(s1, 3);
    expect(received).toEqual([4, 6]);
    expect(read).toEqual([4, 6]);
  });

  test('two nodes that share one callback call the callback once per node', () => {
    let calls = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) + 1,
    );
    readObserved(d1);
    readObserved(d2);
    const callback = () => {
      calls += 1;
    };
    retainDerivedNode(d1, callback);
    retainDerivedNode(d2, callback);
    writeInBatch(s1, 2);
    expect(calls).toBe(2);
  });

  test('a batch that writes nothing calls nothing', () => {
    let calls = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    readObserved(d1);
    retainDerivedNode(d1, () => {
      calls += 1;
    });
    startWriteBatch();
    endWriteBatch();
    expect(calls).toBe(0);
    expect(statusOf(d1)).toBe(CLEAN);
  });

  test('a node that changes in each of two batches calls onChange once per batch', () => {
    let calls = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    readObserved(d1);
    retainDerivedNode(d1, () => {
      calls += 1;
    });
    writeInBatch(s1, 2);
    startWriteBatch();
    writeSource(s1, 3);
    writeSource(s1, 4);
    endWriteBatch();
    expect(calls).toBe(2);
    expect(readObserved(d1)).toBe(4);
  });
});

describe('nodes with callbacks', () => {
  test('a node with a callback that recomputes to the previous value marks and verifies the nodes beyond it, and saves them the same recomputes as a lazy node in its place', () => {
    const measure = (kind: 'observed' | 'lazy') => {
      const runs = { isOne: 0, dependents: 0 };
      const s1 = createSourceNode(2);
      const isOne = createDerivedNode((callingNode) => {
        runs.isOne += 1;
        return readSource(callingNode, s1) === 1;
      });
      const dependents = [0, 1, 2].map((index) =>
        createDerivedNode((callingNode) => {
          runs.dependents += 1;
          return readDerived(callingNode, isOne) ? index : -index;
        }),
      );
      const retains = dependents.map((node) => retainDerivedNode(node));
      const observer =
        kind === 'observed' ? retainDerivedNode(isOne, () => {}) : undefined;
      writeSource(s1, 3);
      const marked = dependents.map(statusOf);
      const runsBeforeReads = runs.isOne;
      const values = dependents.map(read);
      const verified = dependents.map(statusOf);
      retains.forEach(releaseRetain);
      if (observer !== undefined) {
        releaseRetain(observer);
      }
      return { marked, runsBeforeReads, values, verified, runs };
    };
    const observed = measure('observed');
    const lazy = measure('lazy');
    expect(observed.marked).toEqual([MAYBE_DIRTY, MAYBE_DIRTY, MAYBE_DIRTY]);
    expect(observed.verified).toEqual([CLEAN, CLEAN, CLEAN]);
    expect(observed.runsBeforeReads).toBe(2);
    expect(lazy.runsBeforeReads).toBe(1);
    expect(observed).toEqual({ ...lazy, runsBeforeReads: 2 });
    expect(observed.runs).toEqual({ isOne: 2, dependents: 3 });
  });

  test('a node with a callback that recomputes to the previous value leaves its dependents MAYBE_DIRTY, a read verifies them without a recompute, and no onChange runs', () => {
    const runs = { isOne: 0, d1: 0 };
    let calls = 0;
    const s1 = createSourceNode(2);
    const isOne = createDerivedNode((callingNode) => {
      runs.isOne += 1;
      return readSource(callingNode, s1) === 1;
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readDerived(callingNode, isOne) ? 'one' : 'other';
    });
    read(d1);
    retainDerivedNode(isOne, () => {
      calls += 1;
    });
    writeInBatch(s1, 3);
    writeInBatch(s1, 4);
    expect(statusOf(isOne)).toBe(CLEAN);
    expect(statusOf(d1)).toBe(MAYBE_DIRTY);
    expect(calls).toBe(0);
    expect(read(d1)).toBe('other');
    expect(runs).toEqual({ isOne: 3, d1: 1 });
  });

  test('a changed value of a node with a callback marks its lazy dependents DIRTY, and onChange runs after the flush has refreshed every node on the flush queue', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    const d3 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d2) + 1,
    );
    readObserved(d1);
    read(d3);
    retainDerivedNode(d1, () =>
      calls.push(`observedA, observedB ${statusOf(d2)} ${internal(d2).value}`),
    );
    retainDerivedNode(d2, () => calls.push('observedB'));
    startWriteBatch();
    writeSource(s1, 2);
    writeSource(s2, 2);
    expect(statusOf(d3)).toBe(MAYBE_DIRTY);
    endWriteBatch();
    expect(calls).toEqual([`observedA, observedB ${CLEAN} 2`, 'observedB']);
    expect(statusOf(d3)).toBe(DIRTY);
    expect(read(d3)).toBe(3);
  });

  test('a node with a callback behind a derived node becomes MAYBE_DIRTY, and the flush verifies and recomputes it', () => {
    const runs = { d1: 0, observed: 0 };
    let calls = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1) % 2;
    });
    const observed = createDerivedNode((callingNode) => {
      runs.observed += 1;
      return readDerived(callingNode, d1) === 0;
    });
    const d2 = createDerivedNode((callingNode) =>
      readDerived(callingNode, observed),
    );
    read(d2);
    retainDerivedNode(observed, () => {
      calls += 1;
    });
    startWriteBatch();
    writeSource(s1, 3);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(statusOf(observed)).toBe(MAYBE_DIRTY);
    expect(statusOf(d2)).toBe(MAYBE_DIRTY);
    endWriteBatch();
    expect(runs).toEqual({ d1: 2, observed: 1 });
    expect(calls).toBe(0);
    writeInBatch(s1, 4);
    expect(runs).toEqual({ d1: 3, observed: 2 });
    expect(calls).toBe(1);
    expect(read(d2)).toBe(true);
  });

  test('several writes in one batch recompute a node with a callback once', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const observed = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) + readSource(callingNode, s2);
    });
    readObserved(observed);
    startWriteBatch();
    writeSource(s1, 10);
    writeSource(s2, 20);
    writeSource(s1, 30);
    endWriteBatch();
    expect(runs).toBe(2);
    expect(readObserved(observed)).toBe(50);
  });
});

describe('single calls', () => {
  type Graph = {
    readonly s1: SourceNode<number>;
    readonly calls: Array<string>;
    readonly d1: DerivedNode<number>;
    readonly observed: DerivedNode<number>;
    readonly d2: DerivedNode<number>;
  };

  function build(): Graph {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const observed = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) * 2,
    );
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, observed) + 1,
    );
    read(d2);
    retainDerivedNode(observed, () =>
      calls.push(`observed ${readDerivedNode(observed)} ${statusOf(d1)}`),
    );
    return { s1, calls, d1, observed, d2 };
  }

  function statuses(graph: Graph): ReadonlyArray<number> {
    return [statusOf(graph.d1), statusOf(graph.observed), statusOf(graph.d2)];
  }

  test('writeSource calls the same onChange and leaves the same statuses and values as a batch of one write', () => {
    const single = build();
    const batched = build();
    writeSource(single.s1, 2);
    startWriteBatch();
    writeSource(batched.s1, 2);
    endWriteBatch();
    expect(single.calls).toEqual([`observed 4 ${CLEAN}`]);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(single)).toEqual([CLEAN, CLEAN, DIRTY]);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(
      [single.d1, single.observed, single.d2].map((node) => read(node)),
    ).toEqual([2, 4, 5]);
    expect(
      [batched.d1, batched.observed, batched.d2].map((node) => read(node)),
    ).toEqual([2, 4, 5]);
  });

  test('a single-call write whose marking reaches only lazy nodes recomputes nothing and calls nothing, and a later write puts a dependent with a callback on the flush queue and refreshes it', () => {
    let runs = 0;
    const calls: Array<number> = [];
    const s1 = createSourceNode(1);
    const lazy = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const d1 = createDerivedNode((callingNode) =>
      readDerived(callingNode, lazy),
    );
    const observed = createDerivedNode((callingNode) => {
      runs += 1;
      return readDerived(callingNode, d1) * 10;
    });
    read(d1);
    writeSource(s1, 2);
    expect([statusOf(lazy), statusOf(d1)]).toEqual([DIRTY, MAYBE_DIRTY]);
    expect(read(d1)).toBe(2);
    readObserved(observed);
    retainDerivedNode(observed, () => calls.push(readDerivedNode(observed)));
    writeSource(s1, 3);
    expect(runs).toBe(2);
    expect(calls).toEqual([30]);
  });
});

function hasDependent(
  dependency: SourceNode<unknown> | DerivedNode<unknown>,
  node: DerivedNode<unknown>,
): boolean {
  let dependentEdge = internal(dependency).firstDependent;
  while (dependentEdge !== undefined) {
    if (dependentEdge.dependent === node) {
      return true;
    }
    dependentEdge = dependentEdge.nextDependent;
  }
  return false;
}

describe('verification of chains', () => {
  test('a chain of three derived nodes returns the new value after a write, and each node recomputes once', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1) + 1;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d2) + 1;
    });
    expect(read(d3)).toBe(3);
    writeSource(s1, 2);
    expect([statusOf(d1), statusOf(d2), statusOf(d3)]).toEqual([
      DIRTY,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
    ]);
    expect(read(d3)).toBe(4);
    expect([statusOf(d1), statusOf(d2), statusOf(d3)]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ d1: 2, d2: 2, d3: 2 });
  });

  test('a top-level read of the last node of a chain of 100,000 MAYBE_DIRTY nodes recomputes every node without overflowing the call stack', () => {
    const s1 = createSourceNode(0);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    read(d1);
    const chainEnd = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (dependency) => {
        const dependent = createDerivedNode(
          (callingNode) => readDerived(callingNode, dependency) + 1,
        );
        read(dependent);
        return dependent;
      },
      d1,
    );
    expect(read(chainEnd)).toBe(99_999);
    writeSource(s1, 1);
    expect(statusOf(chainEnd)).toBe(MAYBE_DIRTY);
    expect(read(chainEnd)).toBe(100_000);
    expect(statusOf(chainEnd)).toBe(CLEAN);
  });

  test('a top-level read of the last node of a chain of 100,000 MAYBE_DIRTY nodes that depend on a node that cuts off recomputes none of them and leaves them CLEAN', () => {
    let runs = 0;
    const s1 = createSourceNode(0);
    const d1 = createDerivedNode((callingNode) => {
      readSource(callingNode, s1);
      return 0;
    });
    read(d1);
    const nodes: Array<DerivedNode<number>> = [];
    const chainEnd = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (dependency) => {
        const dependent = createDerivedNode((callingNode) => {
          runs += 1;
          return readDerived(callingNode, dependency) + 1;
        });
        read(dependent);
        nodes.push(dependent);
        return dependent;
      },
      d1,
    );
    expect(runs).toBe(99_999);
    writeSource(s1, 1);
    expect(statusOf(chainEnd)).toBe(MAYBE_DIRTY);
    expect(read(chainEnd)).toBe(99_999);
    expect(runs).toBe(99_999);
    expect(nodes.every((dependent) => statusOf(dependent) === CLEAN)).toBe(
      true,
    );
  });

  test('a MAYBE_DIRTY node whose derived dependencies all come back unchanged becomes CLEAN after every one of them was brought up to date', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1) > 0;
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readSource(callingNode, s2) > 0;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d1) && readDerived(callingNode, d2);
    });
    expect(read(d3)).toBe(true);
    writeSource(s1, 2);
    writeSource(s2, 2);
    expect(statusOf(d3)).toBe(MAYBE_DIRTY);
    expect(read(d3)).toBe(true);
    expect([statusOf(d1), statusOf(d2), statusOf(d3)]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ d1: 2, d2: 2, d3: 1 });
  });
});

describe('verification of a diamond of dependencies of a MAYBE_DIRTY node', () => {
  type Diamond = {
    readonly s1: SourceNode<number>;
    readonly runs: {
      readonly d1: number;
      readonly shared: number;
      readonly d2: number;
      readonly d3: number;
      readonly d4: number;
    };
    readonly d1: DerivedNode<number>;
    readonly shared: DerivedNode<number>;
    readonly d2: DerivedNode<number>;
    readonly d3: DerivedNode<number>;
    readonly d4: DerivedNode<number>;
  };

  // d4 depends on d2 and d3, which depend on shared, which depends on d1, which
  // depends on s1. After a write, shared is a MAYBE_DIRTY node with two
  // dependents.
  function build(bottomValue: (sourceValue: number) => number): Diamond {
    const runs = { d1: 0, shared: 0, d2: 0, d3: 0, d4: 0 };
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return bottomValue(readSource(callingNode, s1));
    });
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readDerived(callingNode, d1) + 1;
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, shared) * 2;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, shared) * 3;
    });
    const d4 = createDerivedNode((callingNode) => {
      runs.d4 += 1;
      return readDerived(callingNode, d2) + readDerived(callingNode, d3);
    });
    expect(read(d4)).toBe(10);
    return { s1, runs, d1, shared, d2, d3, d4 };
  }

  test('a top-level read of d4 recomputes each node once when the value changed', () => {
    const graph = build((value) => value);
    writeSource(graph.s1, 2);
    expect([
      statusOf(graph.d1),
      statusOf(graph.shared),
      statusOf(graph.d2),
      statusOf(graph.d3),
      statusOf(graph.d4),
    ]).toEqual([DIRTY, MAYBE_DIRTY, MAYBE_DIRTY, MAYBE_DIRTY, MAYBE_DIRTY]);
    expect(read(graph.d4)).toBe(15);
    expect(graph.runs).toEqual({
      d1: 2,
      shared: 2,
      d2: 2,
      d3: 2,
      d4: 2,
    });
  });

  test('a top-level read of d4 recomputes only d1 when d1 cuts off, and leaves every node CLEAN', () => {
    const graph = build(() => 1);
    writeSource(graph.s1, 2);
    expect(read(graph.d4)).toBe(10);
    expect(graph.runs).toEqual({
      d1: 2,
      shared: 1,
      d2: 1,
      d3: 1,
      d4: 1,
    });
    expect([
      statusOf(graph.d1),
      statusOf(graph.shared),
      statusOf(graph.d2),
      statusOf(graph.d3),
      statusOf(graph.d4),
    ]).toEqual([CLEAN, CLEAN, CLEAN, CLEAN, CLEAN]);
  });

  test('a top-level read of d3, which read the shared node last, recomputes each node once, and a later top-level read of d4 recomputes the rest', () => {
    const graph = build((value) => value);
    writeSource(graph.s1, 2);
    expect(read(graph.d3)).toBe(9);
    expect(graph.runs).toEqual({
      d1: 2,
      shared: 2,
      d2: 1,
      d3: 2,
      d4: 1,
    });
    expect([statusOf(graph.d2), statusOf(graph.d4)]).toEqual([DIRTY, DIRTY]);
    expect(read(graph.d4)).toBe(15);
    expect(graph.runs).toEqual({
      d1: 2,
      shared: 2,
      d2: 2,
      d3: 2,
      d4: 2,
    });
  });

  test('a top-level read of d3, which read the shared node last, leaves d2 MAYBE_DIRTY when d1 cuts off', () => {
    const graph = build(() => 1);
    writeSource(graph.s1, 2);
    expect(read(graph.d3)).toBe(6);
    expect([
      statusOf(graph.shared),
      statusOf(graph.d2),
      statusOf(graph.d3),
      statusOf(graph.d4),
    ]).toEqual([CLEAN, MAYBE_DIRTY, CLEAN, MAYBE_DIRTY]);
    expect(read(graph.d4)).toBe(10);
    expect(graph.runs).toEqual({
      d1: 2,
      shared: 1,
      d2: 1,
      d3: 1,
      d4: 1,
    });
  });

  for (const order of ['cutoff node first', 'changed node first'] as const) {
    test(`a node that depends on a derived node directly and through a node that cuts off recomputes once per write, ${order}`, () => {
      const runs = { d1: 0, parity: 0, d2: 0 };
      const s1 = createSourceNode(1);
      const d1 = createDerivedNode((callingNode) => {
        runs.d1 += 1;
        return readSource(callingNode, s1);
      });
      const parity = createDerivedNode((callingNode) => {
        runs.parity += 1;
        return readDerived(callingNode, d1) % 2;
      });
      const d2 = createDerivedNode((callingNode) => {
        runs.d2 += 1;
        if (order === 'cutoff node first') {
          const parityValue = readDerived(callingNode, parity);
          return `${parityValue}:${readDerived(callingNode, d1)}`;
        }
        const baseValue = readDerived(callingNode, d1);
        return `${readDerived(callingNode, parity)}:${baseValue}`;
      });
      expect(read(d2)).toBe('1:1');
      writeSource(s1, 3);
      expect([statusOf(d1), statusOf(parity), statusOf(d2)]).toEqual([
        DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
      ]);
      expect(read(d2)).toBe('1:3');
      expect(runs).toEqual({ d1: 2, parity: 2, d2: 2 });
      expect([statusOf(d1), statusOf(parity), statusOf(d2)]).toEqual([
        CLEAN,
        CLEAN,
        CLEAN,
      ]);
    });
  }
});

describe('marking breadth', () => {
  for (const [name, writeWith] of writers) {
    test(`${name} marks three dependents of a source DIRTY, the three dependents of those MAYBE_DIRTY, and the three dependents of those MAYBE_DIRTY`, () => {
      const s1 = createSourceNode(1);
      const d1s = [0, 1, 2].map((offset) =>
        createDerivedNode(
          (callingNode) => readSource(callingNode, s1) + offset,
        ),
      );
      const d2s = d1s.map((dependency) =>
        createDerivedNode((callingNode) =>
          readDerived(callingNode, dependency),
        ),
      );
      const d3s = d2s.map((dependency) =>
        createDerivedNode((callingNode) =>
          readDerived(callingNode, dependency),
        ),
      );
      d3s.forEach((node) => read(node));
      writeWith(s1, 2);
      expect(d1s.map((node) => statusOf(node))).toEqual([DIRTY, DIRTY, DIRTY]);
      expect(d2s.map((node) => statusOf(node))).toEqual([
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
      ]);
      expect(d3s.map((node) => statusOf(node))).toEqual([
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
      ]);
      expect(d3s.map((node) => read(node))).toEqual([2, 3, 4]);
    });

    test(`${name} marks every node that depends, directly or through other nodes, on a node with three dependents that each have a dependent`, () => {
      const s1 = createSourceNode(1);
      const d1 = createDerivedNode((callingNode) =>
        readSource(callingNode, s1),
      );
      const d2s = [0, 1, 2].map((offset) =>
        createDerivedNode(
          (callingNode) => readDerived(callingNode, d1) + offset,
        ),
      );
      const d3s = d2s.map((dependency) =>
        createDerivedNode((callingNode) =>
          readDerived(callingNode, dependency),
        ),
      );
      const d4s = d3s.map((dependency) =>
        createDerivedNode((callingNode) =>
          readDerived(callingNode, dependency),
        ),
      );
      d4s.forEach((node) => read(node));
      writeWith(s1, 2);
      expect(statusOf(d1)).toBe(DIRTY);
      expect([...d2s, ...d3s, ...d4s].map((node) => statusOf(node))).toEqual([
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
        MAYBE_DIRTY,
      ]);
      expect(d4s.map((node) => read(node))).toEqual([2, 3, 4]);
    });
  }

  test('writeSource marks a MAYBE_DIRTY dependent DIRTY, and the dependent recomputes once', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(10);
    const d1 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) > 0,
    );
    const d2 = createDerivedNode((callingNode) => {
      runs += 1;
      return readDerived(callingNode, d1) ? readSource(callingNode, s2) : 0;
    });
    read(d2);
    writeSource(s1, 2);
    expect(statusOf(d2)).toBe(MAYBE_DIRTY);
    writeSource(s2, 20);
    expect(statusOf(d2)).toBe(DIRTY);
    expect(read(d2)).toBe(20);
    expect(runs).toBe(2);
  });

  test('writeSource leaves a MAYBE_DIRTY node and its MAYBE_DIRTY dependent MAYBE_DIRTY when a second write reaches them', () => {
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    const d3 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d1) + readDerived(callingNode, d2),
    );
    const d4 = createDerivedNode((callingNode) => readDerived(callingNode, d3));
    read(d4);
    writeSource(s1, 2);
    expect([statusOf(d3), statusOf(d4)]).toEqual([MAYBE_DIRTY, MAYBE_DIRTY]);
    writeSource(s2, 2);
    expect([statusOf(d3), statusOf(d4)]).toEqual([MAYBE_DIRTY, MAYBE_DIRTY]);
    expect(read(d4)).toBe(4);
  });

  test('writeSource puts a dependent with a callback, and a dependent with a callback of a lazy dependent, on the flush queue, and marks the nodes that depend on them MAYBE_DIRTY, which stay so when neither changes', () => {
    const runs = { d1: 0, d3: 0 };
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1) > 0;
    });
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const lazy = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) > 0,
    );
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, lazy);
    });
    const d4 = createDerivedNode((callingNode) => readDerived(callingNode, d3));
    read(d2);
    read(d4);
    let calls = 0;
    retainDerivedNode(d1, () => {
      calls += 1;
    });
    retainDerivedNode(d3, () => {
      calls += 1;
    });
    writeSource(s1, 2);
    expect(runs).toEqual({ d1: 2, d3: 1 });
    expect([statusOf(d1), statusOf(d3), statusOf(d2), statusOf(d4)]).toEqual([
      CLEAN,
      CLEAN,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
    ]);
    expect(calls).toBe(0);
  });

  test('writeSource refreshes two dependents with callbacks of a source, and two dependents with callbacks of a lazy dependent, in the order they left CLEAN', () => {
    const refreshed: Array<string> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1s = ['d1s 0', 'd1s 1'].map((name) =>
      createDerivedNode((callingNode) => {
        refreshed.push(name);
        return readSource(callingNode, s1);
      }),
    );
    const lazy = createDerivedNode((callingNode) =>
      readSource(callingNode, s2),
    );
    const d2s = ['d2s 0', 'd2s 1'].map((name) =>
      createDerivedNode((callingNode) => {
        refreshed.push(name);
        return readDerived(callingNode, lazy);
      }),
    );
    [...d1s, ...d2s].forEach((node) => readObserved(node));
    refreshed.length = 0;
    writeSource(s1, 2);
    expect(refreshed).toEqual(['d1s 0', 'd1s 1']);
    writeSource(s2, 3);
    expect(refreshed).toEqual(['d1s 0', 'd1s 1', 'd2s 0', 'd2s 1']);
    expect([...d1s, ...d2s].map((node) => internal(node).value)).toEqual([
      2, 2, 3, 3,
    ]);
  });

  test('writeSource marks a chain of four nodes, the first DIRTY and the rest MAYBE_DIRTY', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const d3 = createDerivedNode((callingNode) => readDerived(callingNode, d2));
    const d4 = createDerivedNode((callingNode) => readDerived(callingNode, d3));
    read(d4);
    writeSource(s1, 2);
    expect([statusOf(d1), statusOf(d2), statusOf(d3), statusOf(d4)]).toEqual([
      DIRTY,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
    ]);
  });
});

describe('writes', () => {
  test('a write of the value the source stores marks its dependent DIRTY, the dependent recomputes once, and the dependent of that dependent does not recompute', () => {
    const runs = { d1: 0, d2: 0 };
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1) + 1;
    });
    read(d2);
    writeSource(s1, 1);
    expect([statusOf(d1), statusOf(d2)]).toEqual([DIRTY, MAYBE_DIRTY]);
    expect(read(d2)).toBe(2);
    expect(runs).toEqual({ d1: 2, d2: 1 });
  });

  test('dependentsStatus is 1 for a new source, 0 after a readSource, and 1 after a write, and a write after a new dependent called readSource marks that dependent', () => {
    const s1 = createSourceNode(1);
    expect(internal(s1).dependentsStatus).toBe(1);
    writeSource(s1, 2);
    expect(internal(s1).dependentsStatus).toBe(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    expect(read(d1)).toBe(2);
    expect(internal(s1).dependentsStatus).toBe(0);
    writeSource(s1, 3);
    expect(internal(s1).dependentsStatus).toBe(1);
    expect(statusOf(d1)).toBe(DIRTY);
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    expect(read(d2)).toBe(3);
    expect(internal(s1).dependentsStatus).toBe(0);
    startWriteBatch();
    writeSource(s1, 4);
    endWriteBatch();
    expect(internal(s1).dependentsStatus).toBe(1);
    expect([statusOf(d1), statusOf(d2)]).toEqual([DIRTY, DIRTY]);
    expect([read(d1), read(d2)]).toEqual([4, 4]);
  });
  for (const [name, writeWith] of writers) {
    test(`${name} that reaches a DIRTY dependent leaves that dependent DIRTY, marks the other dependent, and the DIRTY dependent recomputes once`, () => {
      const runs = { d1: 0, d2: 0 };
      const s1 = createSourceNode(1);
      const s2 = createSourceNode(10);
      const d1 = createDerivedNode((callingNode) => {
        runs.d1 += 1;
        return readSource(callingNode, s1) + readSource(callingNode, s2);
      });
      const d2 = createDerivedNode((callingNode) => {
        runs.d2 += 1;
        return readSource(callingNode, s2) * 2;
      });
      read(d1);
      read(d2);
      writeWith(s1, 2);
      expect([statusOf(d1), statusOf(d2)]).toEqual([DIRTY, CLEAN]);
      writeWith(s2, 20);
      expect([statusOf(d1), statusOf(d2)]).toEqual([DIRTY, DIRTY]);
      expect([read(d1), read(d2)]).toEqual([22, 40]);
      expect(runs).toEqual({ d1: 2, d2: 2 });
    });
  }
});

describe('values', () => {
  test('a recompute that returns -0 over a stored 0 cuts off and keeps 0', () => {
    let readerRuns = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 0,
    );
    const d2 = createDerivedNode((callingNode) => {
      readerRuns += 1;
      return readDerived(callingNode, d1);
    });
    expect(Object.is(read(d2), 0)).toBe(true);
    writeSource(s1, -1);
    expect(Object.is(read(d1), 0)).toBe(true);
    expect(Object.is(internal(d1).value, 0)).toBe(true);
    expect(Object.is(read(d2), 0)).toBe(true);
    expect(readerRuns).toBe(1);
  });
});

describe('dependency lists', () => {
  test('a computation that reads one source and one derived node 100 times in a row has one edge to each', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => {
      let sum = 0;
      for (let index = 0; index < 100; index += 1) {
        sum += readSource(callingNode, s1);
      }
      for (let index = 0; index < 100; index += 1) {
        sum += readDerived(callingNode, d1);
      }
      return sum;
    });
    expect(read(d2)).toBe(200);
    expectSame(sourceDependenciesOf(d2), [s1]);
    expectSame(derivedDependenciesOf(d2), [d1]);
    expectSame(dependentsOf(s1), [d2, d1]);
    expectSame(dependentsOf(d1), [d2]);
    writeSource(s1, 2);
    expect(read(d2)).toBe(400);
    expectSame(sourceDependenciesOf(d2), [s1]);
    expectSame(derivedDependenciesOf(d2), [d1]);
    expectSame(dependentsOf(s1), [d2, d1]);
    expectSame(dependentsOf(d1), [d2]);
  });

  test('a recompute that reads the same sources and derived nodes in the same order keeps the same edge objects', () => {
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(2);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    const d3 = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, s1) +
        readSource(callingNode, s2) +
        readDerived(callingNode, d1) +
        readDerived(callingNode, d2),
    );
    read(d3);
    const firstSourceEdge = internal(d3).firstSourceDependency;
    const lastSourceEdge = internal(d3).lastSourceDependency;
    const firstDerivedEdge = internal(d3).firstDerivedDependency;
    const lastDerivedEdge = internal(d3).lastDerivedDependency;
    writeSource(s1, 3);
    expect(read(d3)).toBe(10);
    expect(internal(d3).firstSourceDependency === firstSourceEdge).toBe(true);
    expect(internal(d3).lastSourceDependency === lastSourceEdge).toBe(true);
    expect(internal(d3).firstDerivedDependency === firstDerivedEdge).toBe(true);
    expect(internal(d3).lastDerivedDependency === lastDerivedEdge).toBe(true);
    expectSame(sourceDependenciesOf(d3), [s1, s2]);
    expectSame(derivedDependenciesOf(d3), [d1, d2]);
    expectSame(dependentsOf(s1), [d3, d1]);
    expectSame(dependentsOf(s2), [d3, d2]);
  });

  test('a recompute that reads a different derived node at the same position moves the edge, and the node that the recompute stopped reading does not mark the recomputing node', () => {
    let runs = 0;
    const useLeft = createSourceNode(true);
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(10);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    const pick = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, useLeft)
        ? readDerived(callingNode, d1)
        : readDerived(callingNode, d2);
    });
    expect(read(pick)).toBe(1);
    const e1 = internal(pick).firstDerivedDependency;
    writeSource(useLeft, false);
    expect(read(pick)).toBe(10);
    expect(internal(pick).firstDerivedDependency === e1).toBe(true);
    expectSame(derivedDependenciesOf(pick), [d2]);
    expectSame(dependentsOf(d1), []);
    expectSame(dependentsOf(d2), [pick]);
    writeSource(s1, 2);
    expect(statusOf(pick)).toBe(CLEAN);
    writeSource(s2, 20);
    expect(statusOf(pick)).toBe(MAYBE_DIRTY);
    expect(read(pick)).toBe(20);
    expect(runs).toBe(3);
  });

  test('a recompute that reads fewer, none, and then more derived nodes is marked by exactly the derived nodes it read last', () => {
    let runs = 0;
    const count = createSourceNode(3);
    const inputs = [
      createSourceNode(1),
      createSourceNode(2),
      createSourceNode(4),
    ];
    const derived = inputs.map((input) =>
      createDerivedNode((callingNode) => readSource(callingNode, input)),
    );
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return derived
        .slice(0, readSource(callingNode, count))
        .reduce((sum, below) => sum + readDerived(callingNode, below), 0);
    });
    expect(read(d1)).toBe(7);
    expectSame(derivedDependenciesOf(d1), derived);
    writeSource(count, 1);
    expect(read(d1)).toBe(1);
    expectSame(derivedDependenciesOf(d1), [derived[0]]);
    expectSame(dependentsOf(at(derived, 1)), []);
    expectSame(dependentsOf(at(derived, 2)), []);
    writeSource(at(inputs, 2), 8);
    expect(statusOf(d1)).toBe(CLEAN);
    writeSource(count, 0);
    expect(read(d1)).toBe(0);
    expectSame(derivedDependenciesOf(d1), []);
    expect(internal(d1).lastDerivedDependency === undefined).toBe(true);
    expectSame(dependentsOf(at(derived, 0)), []);
    writeSource(at(inputs, 0), 16);
    expect(statusOf(d1)).toBe(CLEAN);
    writeSource(count, 3);
    expect(read(d1)).toBe(26);
    expectSame(derivedDependenciesOf(d1), derived);
    writeSource(at(inputs, 2), 32);
    expect(statusOf(d1)).toBe(MAYBE_DIRTY);
    expect(read(d1)).toBe(50);
    expect(runs).toBe(5);
  });

  test('a recompute that reads fewer, none, and then more sources is marked by exactly the sources it read last', () => {
    let runs = 0;
    const s1 = createSourceNode(3);
    const count = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const sources = [
      createSourceNode(1),
      createSourceNode(2),
      createSourceNode(4),
    ];
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return sources
        .slice(0, readDerived(callingNode, count))
        .reduce((sum, source) => sum + readSource(callingNode, source), 0);
    });
    expect(read(d1)).toBe(7);
    expectSame(sourceDependenciesOf(d1), sources);
    writeSource(s1, 1);
    expect(read(d1)).toBe(1);
    expectSame(sourceDependenciesOf(d1), [sources[0]]);
    expectSame(dependentsOf(at(sources, 1)), []);
    expectSame(dependentsOf(at(sources, 2)), []);
    writeSource(s1, 0);
    expect(read(d1)).toBe(0);
    expectSame(sourceDependenciesOf(d1), []);
    expect(internal(d1).lastSourceDependency === undefined).toBe(true);
    expectSame(dependentsOf(at(sources, 0)), []);
    writeSource(at(sources, 0), 16);
    expect(statusOf(d1)).toBe(CLEAN);
    writeSource(s1, 3);
    expect(read(d1)).toBe(22);
    expectSame(sourceDependenciesOf(d1), sources);
    writeSource(at(sources, 1), 32);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(read(d1)).toBe(52);
    expect(runs).toBe(5);
  });

  test('a recompute that reads its two sources in the other order is marked by both', () => {
    const forward = createSourceNode(true);
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(10);
    const d1 = createDerivedNode((callingNode) => {
      if (readSource(callingNode, forward)) {
        const first = readSource(callingNode, s1);
        return first - readSource(callingNode, s2);
      }
      const first = readSource(callingNode, s2);
      return first - readSource(callingNode, s1);
    });
    expect(read(d1)).toBe(-9);
    writeSource(forward, false);
    expect(read(d1)).toBe(9);
    expectSame(sourceDependenciesOf(d1), [forward, s2, s1]);
    expectSame(dependentsOf(s1), [d1]);
    expectSame(dependentsOf(s2), [d1]);
    writeSource(s1, 2);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(read(d1)).toBe(8);
    writeSource(s2, 20);
    expect(statusOf(d1)).toBe(DIRTY);
    expect(read(d1)).toBe(18);
  });

  test('removing the first, the middle, and the last dependent of a source leaves the others in its list of dependents in the order of their readSource calls', () => {
    const s1 = createSourceNode(1);
    const readers = [0, 1, 2, 3].map((offset) =>
      createDerivedNode((callingNode) => readSource(callingNode, s1) + offset),
    );
    const retains = readers.map((node) => retainDerivedNode(node));
    expectSame(dependentsOf(s1), readers);
    releaseRetain(at(retains, 0));
    expectSame(dependentsOf(s1), [readers[1], readers[2], readers[3]]);
    releaseRetain(at(retains, 2));
    expectSame(dependentsOf(s1), [readers[1], readers[3]]);
    releaseRetain(at(retains, 3));
    expectSame(dependentsOf(s1), [readers[1]]);
    expect(internal(s1).lastDependent?.dependent === readers[1]).toBe(true);
    const r1 = retainDerivedNode(at(readers, 0));
    expectSame(dependentsOf(s1), [readers[1], readers[0]]);
    writeSource(s1, 2);
    expect(readers.map((node) => statusOf(node))).toEqual([
      DIRTY,
      DIRTY,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    releaseRetain(at(retains, 1));
    releaseRetain(r1);
    expectSame(dependentsOf(s1), []);
    expect(internal(s1).firstDependent === undefined).toBe(true);
    expect(internal(s1).lastDependent === undefined).toBe(true);
  });
});

describe('release', () => {
  test('a released node with no dependent has no value and no dependency, and the release disposes the derived dependency it read', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, s1) + readDerived(callingNode, d1),
    );
    const r1 = retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(2);
    expectSame(dependentsOf(d1), [d2]);
    releaseRetain(r1);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([internal(d2).value, internal(d1).value]).toEqual([
      undefined,
      undefined,
    ]);
    expectSame(dependentsOf(d1), []);
    expectSame(dependentsOf(s1), []);
    expect(
      [
        internal(d2).firstSourceDependency,
        internal(d2).lastSourceDependency,
        internal(d2).firstDerivedDependency,
        internal(d2).lastDerivedDependency,
        internal(d1).firstSourceDependency,
      ].map((edge) => edge === undefined),
    ).toEqual([true, true, true, true, true]);
  });

  test('a release disposes a chain of dependencies, and a node in the chain that another node still depends on survives with its value', () => {
    const runs = { shared: 0, d1: 0, d3: 0 };
    const s1 = createSourceNode(1);
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, s1);
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readDerived(callingNode, shared) + 1;
    });
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, shared) + 10;
    });
    const r1 = retainDerivedNode(d2);
    retainDerivedNode(d3);
    expect([readDerivedNode(d2), readDerivedNode(d3)]).toEqual([3, 11]);
    releaseRetain(r1);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([statusOf(shared), statusOf(d3)]).toEqual([CLEAN, CLEAN]);
    expectSame(dependentsOf(shared), [d3]);
    expectSame(dependentsOf(s1), [shared]);
    writeInBatch(s1, 2);
    expect(readDerivedNode(d3)).toBe(12);
    expect(runs).toEqual({ shared: 2, d1: 1, d3: 2 });
  });

  test('a recompute that switches from reading d1 to reading d2 disposes d1 and the node only d1 read, and switching back recomputes them', () => {
    const runs = { d3: 0, d2: 0, d4: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readSource(callingNode, s1) * 100;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d2) + 1;
    });
    const d4 = createDerivedNode((callingNode) => {
      runs.d4 += 1;
      return readSource(callingNode, s1) * 1000;
    });
    const d5 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d1) +
        (readSource(callingNode, flag)
          ? readDerived(callingNode, d3)
          : readDerived(callingNode, d4)),
    );
    retainDerivedNode(d5);
    expect(readDerivedNode(d5)).toBe(102);
    writeInBatch(flag, false);
    expect(readDerivedNode(d5)).toBe(1001);
    expect([statusOf(d3), statusOf(d2)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([statusOf(d1), statusOf(d4)]).toEqual([CLEAN, CLEAN]);
    expectSame(derivedDependenciesOf(d5), [d1, d4]);
    expectSame(dependentsOf(s1), [d1, d4]);
    writeInBatch(flag, true);
    expect(readDerivedNode(d5)).toBe(102);
    expect(statusOf(d4)).toBe(UNCOMPUTED);
    expect(runs).toEqual({ d3: 2, d2: 2, d4: 1 });
  });

  test('a branch that flips is disposed and recomputed on each flip, and retaining both branches keeps both branches from being disposed', () => {
    const runs = { d1: 0, d2: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readSource(callingNode, s1) * 10;
    });
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1)
        : readDerived(callingNode, d2),
    );
    retainDerivedNode(d3);
    readDerivedNode(d3);
    writeInBatch(flag, false);
    readDerivedNode(d3);
    writeInBatch(flag, true);
    readDerivedNode(d3);
    expect(runs).toEqual({ d1: 2, d2: 1 });
    expect(statusOf(d2)).toBe(UNCOMPUTED);

    retainDerivedNode(d1);
    retainDerivedNode(d2);
    writeInBatch(flag, false);
    readDerivedNode(d3);
    writeInBatch(flag, true);
    expect(readDerivedNode(d3)).toBe(1);
    writeInBatch(flag, false);
    expect(readDerivedNode(d3)).toBe(10);
    expect(runs).toEqual({ d1: 2, d2: 2 });
    expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, CLEAN]);
  });

  test('a recompute that reads fewer derived nodes disposes the nodes trim drops', () => {
    const count = createSourceNode(3);
    const s1 = createSourceNode(1);
    const nodes = [0, 1, 2].map((offset) =>
      createDerivedNode((callingNode) => readSource(callingNode, s1) + offset),
    );
    const d1 = createDerivedNode((callingNode) =>
      nodes
        .slice(0, readSource(callingNode, count))
        .reduce((sum, node) => sum + readDerived(callingNode, node), 0),
    );
    retainDerivedNode(d1);
    expect(readDerivedNode(d1)).toBe(6);
    writeInBatch(count, 1);
    expect(readDerivedNode(d1)).toBe(1);
    expect(nodes.map((node) => statusOf(node))).toEqual([
      CLEAN,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    expectSame(dependentsOf(s1), [nodes[0]]);
    writeInBatch(count, 0);
    expect(readDerivedNode(d1)).toBe(0);
    expect(statusOf(at(nodes, 0))).toBe(UNCOMPUTED);
    expectSame(dependentsOf(s1), []);
  });

  test('a trim that drops a node, and the dependency that only the dropped node depended on, disposes both, and a dropped node that another dropped edge still reaches is disposed after that edge is dropped', () => {
    const count = createSourceNode(3);
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const reads = [d2, d1];
    const d3 = createDerivedNode((callingNode) =>
      reads
        .slice(0, readSource(callingNode, count) - 1)
        .reduce((sum, node) => sum + readDerived(callingNode, node), 0),
    );
    retainDerivedNode(d3);
    expect(readDerivedNode(d3)).toBe(3);
    writeInBatch(count, 1);
    expect(readDerivedNode(d3)).toBe(0);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(s1), []);
  });

  test('a node that a recompute repoints away from and reads again later in the same recompute survives with its value and does not recompute', () => {
    const runs = { d1: 0, d2: 0 };
    const order = createSourceNode<'xy' | 'yx'>('xy');
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readSource(callingNode, s1) * 10;
    });
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, order) === 'xy'
        ? readDerived(callingNode, d1) * 100 + readDerived(callingNode, d2)
        : readDerived(callingNode, d2) * 100 + readDerived(callingNode, d1),
    );
    retainDerivedNode(d3);
    expect(readDerivedNode(d3)).toBe(110);
    writeInBatch(order, 'yx');
    // The first tracked read repoints the first edge of d3 from d1 to d2, so d1
    // has no dependent until the second tracked read links an edge to d1.
    expect(readDerivedNode(d3)).toBe(1001);
    expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, CLEAN]);
    expectSame(derivedDependenciesOf(d3), [d2, d1]);
    expect(runs).toEqual({ d1: 1, d2: 1 });
  });

  test('a node that a recompute repoints away from and does not read again is disposed after the recompute, with the dependency that only that node depended on', () => {
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const d3 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    let statusOfXDuringRun: number | undefined;
    const d4 = createDerivedNode((callingNode) => {
      if (readSource(callingNode, flag)) {
        return readDerived(callingNode, d2);
      }
      const value = readDerived(callingNode, d3);
      statusOfXDuringRun = statusOf(d2);
      return value;
    });
    retainDerivedNode(d4);
    expect(readDerivedNode(d4)).toBe(2);
    writeInBatch(flag, false);
    expect(readDerivedNode(d4)).toBe(10);
    expect(statusOfXDuringRun).toBe(CLEAN);
    expect([statusOf(d2), statusOf(d1), statusOf(d3)]).toEqual([
      UNCOMPUTED,
      UNCOMPUTED,
      CLEAN,
    ]);
    expectSame(dependentsOf(s1), [d3]);
  });

  test('a node with several retains is disposed only when the last retain is released', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const retains = [0, 1, 2].map(() => retainDerivedNode(d1));
    expectSame(retainsOf(d1), [...retains].reverse());
    expect(
      retains.map((retain) => readDerivedNode(internal(retain).node)),
    ).toEqual([1, 1, 1]);
    expect(runs).toBe(1);
    releaseRetain(at(retains, 2));
    releaseRetain(at(retains, 1));
    expect(statusOf(d1)).toBe(CLEAN);
    expectSame(dependentsOf(s1), [d1]);
    releaseRetain(at(retains, 0));
    expect(statusOf(d1)).toBe(UNCOMPUTED);
    expectSame(dependentsOf(s1), []);
    expectSame(retainsOf(d1), []);
  });

  test('releasing a node that has a dependent keeps the node, and releasing the dependent disposes both', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const r1 = retainDerivedNode(d1);
    const r2 = retainDerivedNode(d2);
    expect([readDerivedNode(d1), readDerivedNode(d2)]).toEqual([1, 2]);
    releaseRetain(r1);
    expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, CLEAN]);
    writeInBatch(s1, 2);
    expect(readDerivedNode(d2)).toBe(3);
    releaseRetain(r2);
    expect([statusOf(d1), statusOf(d2)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(s1), []);
  });

  test('a disposed node that is retained again recomputes with previous undefined and is marked by its dependencies', () => {
    const previousValues: Array<number | undefined> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(10);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    const d2 = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, s1) + readDerived(callingNode, d1);
    });
    const r1 = retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(11);
    releaseRetain(r1);
    writeInBatch(s1, 2);
    retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(12);
    expectSame(sourceDependenciesOf(d2), [s1]);
    expectSame(derivedDependenciesOf(d2), [d1]);
    writeInBatch(s1, 3);
    expect(statusOf(d2)).toBe(DIRTY);
    expect(readDerivedNode(d2)).toBe(13);
    writeInBatch(s2, 20);
    expect(statusOf(d2)).toBe(MAYBE_DIRTY);
    expect(readDerivedNode(d2)).toBe(23);
    expect(previousValues).toEqual([undefined, undefined, 12, 13]);
  });

  test('releasing a MAYBE_DIRTY node disposes it and the DIRTY node it read', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const r1 = retainDerivedNode(d2);
    readDerivedNode(d2);
    writeInBatch(s1, 2);
    expect([statusOf(d1), statusOf(d2)]).toEqual([DIRTY, MAYBE_DIRTY]);
    releaseRetain(r1);
    expect([statusOf(d1), statusOf(d2)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(s1), []);
  });

  test('a node with a callback is disposed when its dependent, which also has a callback, stops reading the node in a flush, and a node on the flush queue that is released before the flush stays UNCOMPUTED', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const flag = createSourceNode(true);
    const observed = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) * 2;
    });
    const d1 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, observed) : 0,
    );
    retainDerivedNode(d1, () => {});
    expect(readDerivedNode(d1)).toBe(2);
    writeInBatch(flag, false);
    expect(statusOf(d1)).toBe(CLEAN);
    expect(internal(d1).value).toBe(0);
    expect(statusOf(observed)).toBe(UNCOMPUTED);
    expectSame(dependentsOf(s1), []);

    const queued = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const queuedRetain = retainDerivedNode(queued, () => {});
    startWriteBatch();
    writeSource(s1, 2);
    expect(statusOf(queued)).toBe(DIRTY);
    releaseRetain(queuedRetain);
    endWriteBatch();
    expect(statusOf(queued)).toBe(UNCOMPUTED);
    expect(hasDependent(s1, queued)).toBe(false);
    expect(runs).toBe(2);
  });

  test('a node is retained, and retained again with an onChange, which the flush calls when the value changed, and after the release a write calls nothing and visits nothing', () => {
    const calls: Array<number> = [];
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => Math.floor(readDerived(callingNode, d1) / 2) + 1,
    );
    const r1 = retainDerivedNode(d2);
    const r2 = retainDerivedNode(d2, () => calls.push(readDerivedNode(d2)));
    expect(readDerivedNode(d2)).toBe(1);
    writeInBatch(s1, 2);
    writeInBatch(s1, 3);
    writeInBatch(s1, 4);
    expect(calls).toEqual([2, 3]);
    releaseRetain(r2);
    releaseRetain(r1);
    expect([statusOf(d1), statusOf(d2)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(s1), []);
    writeInBatch(s1, 6);
    expect(calls).toEqual([2, 3]);
    expect(internal(s1).dependentsStatus).toBe(1);
  });

  test('two retainers each retain a node and retain it again with an onChange, and releasing the retains of one leaves the node retained by the other', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const subscribe = (name: string): ReadonlyArray<Retain<number>> => {
      const d2 = createDerivedNode((callingNode) =>
        readDerived(callingNode, d1),
      );
      const r1 = retainDerivedNode(d2);
      const r2 = retainDerivedNode(d2, () =>
        calls.push(`${name} ${readDerivedNode(d2)}`),
      );
      return [r2, r1];
    };
    const retains1 = subscribe('first');
    subscribe('second');
    writeInBatch(s1, 2);
    retains1.forEach((r1) => releaseRetain(r1));
    expect(statusOf(internal(at(retains1, 0)).node)).toBe(UNCOMPUTED);
    expect(statusOf(d1)).toBe(CLEAN);
    writeInBatch(s1, 3);
    expect(calls).toEqual(['first 2', 'second 2', 'second 3']);
  });

  test('a release cascades through a chain of 100,000 nodes without overflowing the call stack', () => {
    const s1 = createSourceNode(0);
    let previous = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const d1 = previous;
    const nodes: Array<DerivedNode<number>> = [previous];
    for (let i = 0; i < 100_000; i++) {
      const dependency = previous;
      previous = createDerivedNode(
        (callingNode) => readDerived(callingNode, dependency) + 1,
      );
      nodes.push(previous);
    }
    // Recomputing the chain from its last node would overflow the call stack,
    // so each node is recomputed while it is retained, in the order of the
    // chain from its first node, and every retain but that of the last node is
    // released once the node's dependent is retained.
    const retains = nodes.slice(0, -1).map((node) => retainDerivedNode(node));
    const lastRetain = retainDerivedNode(previous);
    retains.forEach((retain) => releaseRetain(retain));
    expect(statusOf(d1)).toBe(CLEAN);
    releaseRetain(lastRetain);
    expect(nodes.every((node) => statusOf(node) === UNCOMPUTED)).toBe(true);
    expectSame(dependentsOf(s1), []);
  });
});

describe('disposal deferred to the end of the outermost operation', () => {
  function countedNode(
    runs: { d1: number },
    source: SourceNode<number>,
  ): DerivedNode<number> {
    return createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, source);
    });
  }

  test('a node that an inner recompute drops, and that the outer recompute read after the inner node before, survives and does not recompute', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d3 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d2) + readDerived(callingNode, d1),
    );
    expect(read(d3)).toBe(2);
    writeInBatch(flag, false);
    expect(read(d3)).toBe(1);
    expect(runs.d1).toBe(1);
    expectSame(derivedDependenciesOf(d3), [d2, d1]);
  });

  test('a node that an inner recompute drops, and that the outer recompute read before the inner node and now reads after it, survives and does not recompute', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1) + readDerived(callingNode, d2)
        : readDerived(callingNode, d2) + readDerived(callingNode, d1),
    );
    expect(read(d3)).toBe(2);
    writeInBatch(flag, false);
    expect(read(d3)).toBe(1);
    expect(runs.d1).toBe(1);
  });

  test('a node whose edge the outer recompute repointed before reading the inner node, which drops it, survives and does not recompute when the outer recompute reads it again', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d4 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1) + readDerived(callingNode, d3)
        : readDerived(callingNode, d2) +
          readDerived(callingNode, d3) +
          readDerived(callingNode, d1),
    );
    expect(read(d4)).toBe(2);
    writeInBatch(flag, false);
    expect(read(d4)).toBe(11);
    expect(runs.d1).toBe(1);
    expectSame(derivedDependenciesOf(d4), [d2, d3, d1]);
  });

  test('a node that an inner recompute drops, and that the outer recompute reads for the first time, survives and does not recompute', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d2)
        : readDerived(callingNode, d2) + readDerived(callingNode, d1),
    );
    expect(read(d3)).toBe(1);
    writeInBatch(flag, false);
    expect(read(d3)).toBe(1);
    expect(runs.d1).toBe(1);
  });

  test('a node that a recompute nested two recomputes deep drops survives and does not recompute when the outer recompute, which repointed its edge to the node, reads the node again', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d4 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d3) + 1,
    );
    const d5 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1) + readDerived(callingNode, d4)
        : readDerived(callingNode, d2) +
          readDerived(callingNode, d4) +
          readDerived(callingNode, d1),
    );
    expect(read(d5)).toBe(3);
    writeInBatch(flag, false);
    expect(read(d5)).toBe(12);
    expect(runs.d1).toBe(1);
  });

  test('a node that one branch of a swap within one node stops reading at its position survives and does not recompute', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1) * 100 + readDerived(callingNode, d2)
        : readDerived(callingNode, d2) * 100 + readDerived(callingNode, d1),
    );
    expect(read(d3)).toBe(110);
    writeInBatch(flag, false);
    expect(read(d3)).toBe(1001);
    expect(runs.d1).toBe(1);
  });

  for (const [name, writesOther] of [
    ['recomputes', true],
    ['verifies', false],
  ] as const) {
    test(`a node that one node of a swap across two nodes drops and the other reads survives and does not recompute when the outer node ${name}`, () => {
      const runs = { d1: 0 };
      const flag = createSourceNode(true);
      const s1 = createSourceNode(0);
      const s2 = createSourceNode(1);
      const d1 = countedNode(runs, s2);
      const d2 = createDerivedNode(
        (callingNode) => readSource(callingNode, s2) * 10,
      );
      const d3 = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? readDerived(callingNode, d1)
          : readDerived(callingNode, d2),
      );
      const d4 = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? readDerived(callingNode, d2)
          : readDerived(callingNode, d1),
      );
      const d5 = createDerivedNode(
        (callingNode) =>
          readSource(callingNode, s1) +
          readDerived(callingNode, d3) * 100 +
          readDerived(callingNode, d4),
      );
      expect(read(d5)).toBe(110);
      startWriteBatch();
      writeSource(flag, false);
      if (writesOther) {
        writeSource(s1, 1);
      }
      endWriteBatch();
      expect(read(d5)).toBe(writesOther ? 1002 : 1001);
      expect(runs.d1).toBe(1);
      expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, CLEAN]);
    });
  }

  test('a node that a node with a callback drops in a flush, and that a later node with a callback reads, survives and does not recompute', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1)
        : readDerived(callingNode, d2),
    );
    const d4 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d2)
        : readDerived(callingNode, d1),
    );
    expect([readObserved(d3), readObserved(d4)]).toEqual([1, 10]);
    writeInBatch(flag, false);
    expect([internal(d3).value, internal(d4).value]).toEqual([10, 1]);
    expect(runs.d1).toBe(1);
  });

  test('a node that an inner node with a callback drops survives and does not recompute when the outer node with a callback reads it after the inner node', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1) + readDerived(callingNode, d2)
        : readDerived(callingNode, d2) + readDerived(callingNode, d1),
    );
    expect(readObserved(d3)).toBe(2);
    writeInBatch(flag, false);
    expect(internal(d3).value).toBe(1);
    expect(runs.d1).toBe(1);
  });

  test('a node that a recompute during verification drops, in a dependency of the node being verified, survives and does not recompute when a later recompute reads it', () => {
    const runs = { d1: 0 };
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = countedNode(runs, s1);
    const selector = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? 1 : 2,
    );
    const d2 = createDerivedNode((callingNode) =>
      readDerived(callingNode, selector) === 1
        ? readDerived(callingNode, d1)
        : 0,
    );
    const d3 = createDerivedNode((callingNode) =>
      readDerived(callingNode, selector) === 1
        ? 0
        : readDerived(callingNode, d1),
    );
    const d4 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d2) * 100 + readDerived(callingNode, d3),
    );
    expect(read(d4)).toBe(100);
    writeInBatch(flag, false);
    expect(read(d4)).toBe(1);
    expect(runs.d1).toBe(1);
    expectSame(derivedDependenciesOf(d3), [selector, d1]);
  });

  for (const [outerDrops, innerDrops] of [
    [1, 2],
    [2, 1],
  ] as const) {
    test(`when the outer recompute leaves ${outerDrops} and an inner recompute leaves ${innerDrops} nodes unneeded, the outermost operation disposes each one that no later recompute read`, () => {
      const flag = createSourceNode(true);
      const s1 = createSourceNode(1);
      const nodes = (count: number) =>
        Array.from({ length: count }, () =>
          createDerivedNode((callingNode) => readSource(callingNode, s1)),
        );
      const outerNodes = nodes(outerDrops);
      const innerNodes = nodes(innerDrops);
      const replacement = createDerivedNode(
        (callingNode) => readSource(callingNode, s1) * 10,
      );
      const sum = (
        callingNode: CallingNode,
        list: ReadonlyArray<DerivedNode<number>>,
      ) =>
        list.reduce((total, node) => total + readDerived(callingNode, node), 0);
      const d1 = createDerivedNode((callingNode) =>
        readSource(callingNode, flag) ? sum(callingNode, innerNodes) : 0,
      );
      const d2 = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? sum(callingNode, outerNodes) + readDerived(callingNode, d1)
          : readDerived(callingNode, replacement) +
            readDerived(callingNode, d1) +
            readDerived(callingNode, at(innerNodes, 0)),
      );
      expect(read(d2)).toBe(outerDrops + innerDrops);
      writeInBatch(flag, false);
      expect(read(d2)).toBe(11);
      expect(outerNodes.map((node) => statusOf(node))).toEqual(
        outerNodes.map(() => UNCOMPUTED),
      );
      expect(innerNodes.map((node) => statusOf(node))).toEqual(
        innerNodes.map((_, index) => (index === 0 ? CLEAN : UNCOMPUTED)),
      );
      expectSame(dependentsOf(s1), [at(innerNodes, 0), replacement]);
    });
  }

  test('a node that an inner recompute drops, and no later recompute reads, is disposed when the outermost operation returns', () => {
    const flag = createSourceNode(true);
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    let statusOfXAfterInner: number | undefined;
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, d1) : 0,
    );
    const d3 = createDerivedNode((callingNode) => {
      const value = readDerived(callingNode, d2);
      statusOfXAfterInner = statusOf(d1);
      return value;
    });
    expect(read(d3)).toBe(1);
    writeInBatch(flag, false);
    expect(read(d3)).toBe(0);
    expect(statusOfXAfterInner).toBe(CLEAN);
    expect(statusOf(d1)).toBe(UNCOMPUTED);
    expectSame(dependentsOf(s1), []);
  });
});

describe('onChange', () => {
  test('the flush calls onChange after it has refreshed every node on the flush queue, in the order of the first recompute of each node that changed its value', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const s2 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readSource(callingNode, s2));
    readObserved(d1);
    readObserved(d2);
    retainDerivedNode(d1, () =>
      calls.push(`first, second ${statusOf(d2)} ${internal(d2).value}`),
    );
    retainDerivedNode(d2, () =>
      calls.push(`second, first ${internal(d1).value}`),
    );
    startWriteBatch();
    writeSource(s2, 2);
    writeSource(s1, 2);
    endWriteBatch();
    expect(calls).toEqual(['second, first 2', `first, second ${CLEAN} 2`]);
  });

  test('a refresh that reads a node later on the flush queue refreshes that node first, so the reader recomputes once, and the flush calls the onChange of that node first', () => {
    const runs = { d1: 0, late: 0 };
    const calls: Array<string> = [];
    const s1 = createSourceNode(1);
    const late = createDerivedNode((callingNode) => {
      runs.late += 1;
      return readSource(callingNode, s1) * 10;
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readSource(callingNode, s1) + readDerived(callingNode, late);
    });
    readObserved(d1);
    retainDerivedNode(d1, (value) => calls.push(`first ${value}`));
    retainDerivedNode(late, (value) => calls.push(`late ${value}`));
    expectSame(dependentsOf(s1), [d1, late]);
    writeInBatch(s1, 2);
    expect(runs).toEqual({ d1: 2, late: 2 });
    expect(calls).toEqual(['late 20', 'first 22']);
    writeInBatch(s1, 3);
    expect(calls).toEqual(['late 20', 'first 22', 'late 30', 'first 33']);
  });

  test('the first recompute of a node calls no onChange, outside or inside a flush', () => {
    const calls: Array<string> = [];
    const s1 = createSourceNode(0);
    const s2 = createSourceNode(5);
    const late = createDerivedNode((callingNode) =>
      readSource(callingNode, s2),
    );
    retainDerivedNode(late, () => calls.push('late'));
    const d1 = createDerivedNode((callingNode) =>
      readSource(callingNode, s1) > 0 ? readDerived(callingNode, late) : 0,
    );
    retainDerivedNode(d1, () => calls.push(`top ${readDerivedNode(d1)}`));
    expect(readObserved(d1)).toBe(0);
    expect(calls).toEqual([]);
    writeInBatch(s1, 1);
    expect(statusOf(late)).toBe(CLEAN);
    expect(calls).toEqual(['top 5']);
    writeInBatch(s2, 6);
    expect(calls).toEqual(['top 5', 'late', 'top 6']);
  });

  test('an onChange that calls readDerivedNode on lazy nodes and on nodes with callbacks gets their current values and leaves them CLEAN', () => {
    const seen: Array<ReadonlyArray<number>> = [];
    const s1 = createSourceNode(1);
    const observed = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 2,
    );
    const lazy = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) + 1,
    );
    const d1 = createDerivedNode(
      (callingNode) => readDerived(callingNode, observed) + 100,
    );
    readObserved(observed);
    read(lazy);
    read(d1);
    retainDerivedNode(observed, () =>
      seen.push([
        readDerivedNode(observed),
        readDerivedNode(lazy),
        readDerivedNode(d1),
        internal(s1).value,
      ]),
    );
    writeInBatch(s1, 5);
    expect(seen).toEqual([[10, 6, 110, 5]]);
    expect([statusOf(observed), statusOf(lazy), statusOf(d1)]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
  });
});

describe('the flush queue', () => {
  for (const [name, writeWith] of writers) {
    test(`${name} refreshes every node it puts on the flush queue in the order of the flush queue, and refreshes the nodes with callbacks that a changed refresh appends to the flush queue in the same flush`, () => {
      const refreshed: Array<string> = [];
      const calls: Array<string> = [];
      const s1 = createSourceNode(1);
      const d1s = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((index) =>
        createDerivedNode((callingNode) => {
          refreshed.push(`d1s ${index}`);
          return readSource(callingNode, s1) + index;
        }),
      );
      const d2s = [0, 9].map((index) =>
        createDerivedNode((callingNode) => {
          refreshed.push(`d2s ${index}`);
          return readDerived(callingNode, at(d1s, index)) * 10;
        }),
      );
      [...d1s, ...d2s].forEach((node) => read(node));
      [...d1s, ...d2s].forEach((node, index) => {
        retainDerivedNode(node, () => calls.push(`${index}`));
      });
      refreshed.length = 0;
      writeWith(s1, 2);
      expect(refreshed).toEqual([
        ...d1s.map((_, index) => `d1s ${index}`),
        'd2s 0',
        'd2s 9',
      ]);
      expect(calls).toEqual([
        '0',
        '1',
        '2',
        '3',
        '4',
        '5',
        '6',
        '7',
        '8',
        '9',
        '10',
        '11',
      ]);
      expect(d2s.map((node) => internal(node).value)).toEqual([20, 110]);
      refreshed.length = 0;
      writeWith(s1, 3);
      expect(refreshed.length).toBe(12);
      expect(calls.length).toBe(24);
    });
  }
});

describe('end-to-end session', () => {
  test('reads, an update that cuts off, a batch, a change callback, a one-off read, and releases return the expected values and recompute counts', () => {
    const featuredEconomist = createSourceNode({
      first: 'John Maynard',
      last: 'Keynes',
      bestBook: 'A Treatise on Money',
    });
    const name = createDerivedNode<{
      readonly first: string;
      readonly last: string;
    }>((callingNode, previous) => {
      const { first, last } = readSource(callingNode, featuredEconomist);
      return previous !== undefined &&
        previous.first === first &&
        previous.last === last
        ? previous
        : { first, last };
    });
    let greetingRuns = 0;
    const greeting = createDerivedNode((callingNode) => {
      greetingRuns += 1;
      const { first, last } = readDerived(callingNode, name);
      return `Hello, ${first} ${last}`;
    });

    const r1 = retainDerivedNode(greeting);
    expect(greetingRuns).toBe(1);
    expect(readDerivedNode(greeting)).toBe('Hello, John Maynard Keynes');

    writeSource(featuredEconomist, {
      first: 'John Maynard',
      last: 'Keynes',
      bestBook: 'The General Theory',
    });
    expect(readDerivedNode(greeting)).toBe('Hello, John Maynard Keynes');
    expect(greetingRuns).toBe(1);

    startWriteBatch();
    writeSource(featuredEconomist, {
      first: 'Milton',
      last: 'Friedman',
      bestBook: 'A Monetary History',
    });
    writeSource(featuredEconomist, {
      first: 'Milton',
      last: 'Friedman',
      bestBook: 'Capitalism and Freedom',
    });
    endWriteBatch();
    expect(readDerivedNode(greeting)).toBe('Hello, Milton Friedman');
    expect(greetingRuns).toBe(2);

    const rendered: Array<string> = [];
    const view = createDerivedNode((callingNode) =>
      readDerived(callingNode, greeting),
    );
    const viewRetain = retainDerivedNode(view, () =>
      rendered.push(readDerivedNode(view)),
    );
    writeSource(featuredEconomist, {
      first: 'Adam',
      last: 'Smith',
      bestBook: 'The Wealth of Nations',
    });
    expect(rendered).toEqual(['Hello, Adam Smith']);

    const initials = createDerivedNode((callingNode) => {
      const { first, last } = readDerived(callingNode, name);
      return `${first[0]}${last[0]}`;
    });
    expect(readDerivedNode(initials)).toBe('AS');
    expect(statusOf(initials)).toBe(UNCOMPUTED);

    releaseRetain(viewRetain);
    releaseRetain(r1);
    expect([statusOf(view), statusOf(greeting), statusOf(name)]).toEqual([
      UNCOMPUTED,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
  });
});

function retainsOf<TValue>(
  node: DerivedNode<TValue>,
): ReadonlyArray<Retain<TValue>> {
  const retains: Array<Retain<TValue>> = [];
  let retain = internal(node).firstRetain;
  while (retain !== undefined) {
    retains.push(retain);
    retain = retain.next;
  }
  return retains;
}

function dependentsOf(
  dependency: SourceNode<unknown> | DerivedNode<unknown>,
): ReadonlyArray<DerivedNode<unknown>> {
  const dependents: Array<DerivedNode<unknown>> = [];
  let dependentEdge = internal(dependency).firstDependent;
  while (dependentEdge !== undefined) {
    dependents.push(dependentEdge.dependent);
    dependentEdge = dependentEdge.nextDependent;
  }
  return dependents;
}

function sourceDependenciesOf(
  node: DerivedNode<unknown>,
): ReadonlyArray<SourceNode<unknown>> {
  const dependencies: Array<SourceNode<unknown>> = [];
  let dependencyEdge = internal(node).firstSourceDependency;
  while (dependencyEdge !== undefined) {
    dependencies.push(dependencyEdge.dependency);
    dependencyEdge = dependencyEdge.nextDependency;
  }
  return dependencies;
}

function derivedDependenciesOf(
  node: DerivedNode<unknown>,
): ReadonlyArray<DerivedNode<unknown>> {
  const dependencies: Array<DerivedNode<unknown>> = [];
  let dependencyEdge = internal(node).firstDerivedDependency;
  while (dependencyEdge !== undefined) {
    dependencies.push(dependencyEdge.dependency);
    dependencyEdge = dependencyEdge.nextDependency;
  }
  return dependencies;
}

// Compares by identity. toEqual on nodes walks the whole graph through the
// edge lists.
function expectSame(
  actual: ReadonlyArray<unknown>,
  expected: ReadonlyArray<unknown>,
): void {
  expect(actual.length).toBe(expected.length);
  actual.forEach((item, index) => {
    expect(item === expected[index]).toBe(true);
  });
}

describe('marking from a refresh in the flush', () => {
  test('the refresh of a node with a callback whose value changed marks its lazy dependents and the nodes that depend on those, which recompute on the next read that reaches them', () => {
    const s1 = createSourceNode(1);
    const observed = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const observedRetain = retainDerivedNode(observed, () => {});
    let firstRuns = 0;
    const d1 = createDerivedNode((callingNode) => {
      firstRuns += 1;
      return readDerived(callingNode, observed) + 1;
    });
    let secondRuns = 0;
    const d2 = createDerivedNode((callingNode) => {
      secondRuns += 1;
      return readDerived(callingNode, observed) + 2;
    });
    const d3 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, d1) + readDerived(callingNode, d2),
    );
    const d4 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) * 2,
    );
    expect([read(d3), read(d4)]).toEqual([23, 22]);
    writeSource(s1, 2);
    expect([statusOf(d1), statusOf(d2), statusOf(d3), statusOf(d4)]).toEqual([
      DIRTY,
      DIRTY,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
    ]);
    expect([read(d3), read(d4)]).toEqual([43, 42]);
    expect([firstRuns, secondRuns]).toEqual([2, 2]);
    releaseRetain(observedRetain);
  });

  test('the refresh of a node with a callback whose value changed marks DIRTY a dependent that the write already marked MAYBE_DIRTY', () => {
    const s1 = createSourceNode(1);
    const lazy = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const observed = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * 10,
    );
    const observedRetain = retainDerivedNode(observed, () => {});
    let runs = 0;
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return (
        readDerived(callingNode, lazy) + readDerived(callingNode, observed)
      );
    });
    expect(read(d1)).toBe(11);
    writeSource(s1, 2);
    expect(statusOf(d1)).toBe(DIRTY);
    expect([read(d1), runs]).toEqual([22, 2]);
    releaseRetain(observedRetain);
  });

  test('the refresh of a node with a callback whose value changed puts a node with a callback that reads it through a lazy node on the flush queue, and the flush disposes an unneeded node that the write marked MAYBE_DIRTY with MARKED_BEYOND', () => {
    const s1 = createSourceNode(1);
    const observed = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const observedRetain = retainDerivedNode(observed, () => {});
    const d1 = createDerivedNode(
      (callingNode) => readDerived(callingNode, observed) * 2,
    );
    const values: Array<number> = [];
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const r1 = retainDerivedNode(d2, () => {
      values.push(readDerivedNode(d2));
    });
    scheduler.schedule = () => {};
    const unneeded = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) - 1,
    );
    expect(readDerivedNode(unneeded)).toBe(1);
    writeSource(s1, 2);
    expect(values).toEqual([5]);
    expect(statusOf(unneeded)).toBe(UNCOMPUTED);
    releaseRetain(r1);
    releaseRetain(observedRetain);
  });
  test('the refresh of a node with a callback whose value changed marks through a chain of lazy dependents with several branches, and the flush disposes an unneeded dependent that the write marked MAYBE_DIRTY with MARKED_BEYOND', () => {
    const s1 = createSourceNode(1);
    const observed = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const observedRetain = retainDerivedNode(observed, () => {});
    const d1 = createDerivedNode(
      (callingNode) => readDerived(callingNode, observed) * 2,
    );
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const d3 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 2,
    );
    const d4 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d2) * 10,
    );
    const d5 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d3) * 10,
    );
    expect([read(d4), read(d5)]).toEqual([30, 40]);
    scheduler.schedule = () => {};
    const unneeded = createDerivedNode(
      (callingNode) => readDerived(callingNode, observed) + 100,
    );
    expect(readDerivedNode(unneeded)).toBe(101);
    writeSource(s1, 2);
    expect([statusOf(d1), statusOf(d2), statusOf(d3)]).toEqual([
      DIRTY,
      MAYBE_DIRTY,
      MAYBE_DIRTY,
    ]);
    expect([statusOf(d4), statusOf(d5), statusOf(unneeded)]).toEqual([
      MAYBE_DIRTY,
      MAYBE_DIRTY,
      UNCOMPUTED,
    ]);
    expect([read(d4), read(d5)]).toEqual([50, 60]);
    releaseRetain(observedRetain);
  });
});

describe('the flush', () => {
  test('a write that puts a node with a callback on the flush queue and reaches an unneeded node disposes the unneeded node in the flush', () => {
    scheduler.schedule = () => {};
    const s1 = createSourceNode(1);
    const unneeded = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    expect(readDerivedNode(unneeded)).toBe(1);
    const observed = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const changes: Array<number> = [];
    const r1 = retainDerivedNode(observed, () => {
      changes.push(readDerivedNode(observed));
    });
    writeSource(s1, 2);
    expect(changes).toEqual([2]);
    expect(statusOf(unneeded)).toBe(UNCOMPUTED);
    releaseRetain(r1);
  });
});

describe('repointed edges', () => {
  test('a recompute that reads a different node at a position puts the node it stopped reading on the cleanup queue', () => {
    const which = createSourceNode(true);
    const d1 = createDerivedNode(() => 'left');
    const d2 = createDerivedNode(() => 'right');
    const d3 = createDerivedNode((callingNode) =>
      readDerived(callingNode, readSource(callingNode, which) ? d1 : d2),
    );
    expect(read(d3)).toBe('left');
    writeSource(which, false);
    expect(read(d3)).toBe('right');
    expect([statusOf(d1), statusOf(d2)]).toEqual([UNCOMPUTED, CLEAN]);
  });
});

describe('the walk past a node with a callback that keeps its value', () => {
  test('with 100 readers, each with a callback, of a node with a callback that keeps its value, the first write reads the status of each reader once and the second write reads none, and each reader verifies to its value', () => {
    const s1 = createSourceNode(0);
    const projection = createDerivedNode((callingNode) =>
      Math.floor(readSource(callingNode, s1) / 1000),
    );
    const projectionRetain = retainDerivedNode(projection, () => {});
    const readers = Array.from({ length: 100 }, (_, index) =>
      createDerivedNode(
        (callingNode) => readDerived(callingNode, projection) + index,
      ),
    );
    const calls: Array<number> = [];
    const retains = readers.map((reader, index) =>
      retainDerivedNode(reader, () => calls.push(index)),
    );
    const statusReads = countStatusReads(readers);
    writeSource(s1, 1);
    const afterFirstWrite = statusReads();
    writeSource(s1, 2);
    expect([afterFirstWrite, statusReads() - afterFirstWrite, calls]).toEqual([
      100,
      0,
      [],
    ]);
    expect(readers.map((reader) => readDerivedNode(reader))).toEqual(
      readers.map((_, index) => index),
    );
    retains.forEach((retain) => releaseRetain(retain));
    releaseRetain(projectionRetain);
  });
});
