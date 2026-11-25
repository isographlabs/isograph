import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { internal } from './internal';
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  disposeNext,
  endWriteBatch,
  readDerived,
  readDerivedNode,
  readSource,
  releaseRetain,
  retainDerivedNode,
  scheduler,
  startWriteBatch,
  writeSource,
  type DerivedNode,
  type SourceNode,
} from '../src/index';
import { CLEAN, DIRTY, OBSERVED, UNCOMPUTED, UNOBSERVED } from '../src/nodes';
import { statusOf } from './status';

let scheduleCalls = 0;

beforeEach(() => {
  scheduleCalls = 0;
  scheduler.schedule = () => {
    scheduleCalls += 1;
  };
});

afterEach(() => {
  disposeAll();
});

describe('retains and the cleanup queue', () => {
  test('a release of the last retain puts the node on the cleanup queue, and a retain before disposal keeps the node from being disposed', () => {
    const s1 = createSourceNode(1);
    let runs = 0;
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const r1 = retainDerivedNode(d1);
    readDerivedNode(d1);
    releaseRetain(r1);
    expect([statusOf(d1), scheduleCalls]).toEqual([CLEAN, 1]);
    const r2 = retainDerivedNode(d1);
    expect(disposeNext()).toBe(false);
    expect([statusOf(d1), internal(d1).value, runs]).toEqual([CLEAN, 1, 1]);
    releaseRetain(r2);
    expect(disposeNext()).toBe(false);
    expect([statusOf(d1), internal(d1).value]).toEqual([UNCOMPUTED, undefined]);
  });

  test('a release that leaves another retain puts nothing on the cleanup queue', () => {
    const d1 = createDerivedNode(() => 1);
    const r1 = retainDerivedNode(d1);
    retainDerivedNode(d1);
    readDerivedNode(d1);
    releaseRetain(r1);
    expect(scheduleCalls).toBe(0);
    expect(internal(d1).firstRetain).not.toBe(undefined);
    expect(disposeNext()).toBe(false);
    expect(statusOf(d1)).toBe(CLEAN);
  });

  test('a node whose retain is released and that a new dependent then reads before disposal is not recomputed', () => {
    const s1 = createSourceNode(1);
    let runs = 0;
    const shared = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const d1 = createDerivedNode((callingNode) =>
      readDerived(callingNode, shared),
    );
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, shared) + 1,
    );
    const r1 = retainDerivedNode(d1);
    readDerivedNode(d1);
    releaseRetain(r1);
    retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(2);
    expect(runs).toBe(1);
    disposeAll();
    expect([statusOf(d1), statusOf(shared), statusOf(d2)]).toEqual([
      UNCOMPUTED,
      CLEAN,
      CLEAN,
    ]);
  });

  test('the flush skips a node on the flush queue that lost its last callback, which stays DIRTY, and a top-level read recomputes it as a lazy node', () => {
    const s1 = createSourceNode(1);
    let runs = 0;
    const observed = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const r1 = retainDerivedNode(observed, () => {});
    expect([internal(observed).evaluation, runs]).toEqual([OBSERVED, 1]);
    startWriteBatch();
    writeSource(s1, 2);
    releaseRetain(r1);
    endWriteBatch();
    expect([internal(observed).evaluation, statusOf(observed), runs]).toEqual([
      UNOBSERVED,
      DIRTY,
      1,
    ]);
    expect(scheduleCalls).toBe(1);
    retainDerivedNode(observed, () => {});
    expect([
      readDerivedNode(observed),
      runs,
      internal(observed).evaluation,
    ]).toEqual([2, 2, OBSERVED]);
    expect(disposeNext()).toBe(false);
    expect(statusOf(observed)).toBe(CLEAN);
  });

  test('the flush calls the onChange of a retain once when the value changed, not on the first recompute, and a lazy node retained without onChange is DIRTY until a top-level read recomputes it', () => {
    const calls: Array<number> = [];
    const s1 = createSourceNode(1);
    const observed = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const unobserved = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    retainDerivedNode(unobserved);
    retainDerivedNode(observed, () => calls.push(readDerivedNode(observed)));
    expect(calls).toEqual([]);
    startWriteBatch();
    writeSource(s1, 2);
    writeSource(s1, 3);
    endWriteBatch();
    expect(calls).toEqual([3]);
    expect([statusOf(observed), statusOf(unobserved)]).toEqual([CLEAN, DIRTY]);
    expect(readDerivedNode(unobserved)).toBe(3);
  });

  test('releaseRetain puts the node on the cleanup queue and leaves it CLEAN, and disposeAll disposes it and the dependency only it depended on, but not a retained dependency', () => {
    const s1 = createSourceNode(1);
    const shared = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const only = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const d1 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, shared) + readDerived(callingNode, only),
    );
    const r1 = retainDerivedNode(d1);
    const sharedRetain = retainDerivedNode(shared);
    releaseRetain(r1);
    expect([statusOf(d1), statusOf(shared), statusOf(only)]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(scheduleCalls).toBe(1);
    disposeAll();
    expect([statusOf(d1), statusOf(shared), statusOf(only)]).toEqual([
      UNCOMPUTED,
      CLEAN,
      UNCOMPUTED,
    ]);
    expect(internal(d1).value).toBe(undefined);
    expectSame(dependentsOf(s1), [shared]);
    expectSame(dependentsOf(shared), []);
    expect(internal(shared).firstRetain === sharedRetain).toBe(true);
    expect(disposeNext()).toBe(false);
  });

  test('a node that a disposal leaves unneeded, and that a new retain reaches before the node is disposed, keeps its value and does not recompute', () => {
    const runs = { shared: 0, d1: 0, d2: 0 };
    const s1 = createSourceNode(1);
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, s1);
    });
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return readDerived(callingNode, shared) + 1;
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, shared) + 2;
    });
    releaseRetain(retainDerivedNode(d1));
    expect(disposeNext()).toBe(true);
    expect([statusOf(d1), statusOf(shared)]).toEqual([UNCOMPUTED, CLEAN]);
    retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(3);
    expect(runs).toEqual({ shared: 1, d1: 1, d2: 1 });
    disposeAll();
    expect([statusOf(d1), statusOf(shared), statusOf(d2)]).toEqual([
      UNCOMPUTED,
      CLEAN,
      CLEAN,
    ]);
    retainDerivedNode(d1);
    expect(readDerivedNode(d1)).toBe(2);
    expect(runs).toEqual({ shared: 1, d1: 2, d2: 1 });
  });

  test('a node that one node with a callback stops reading in a flush, and a later node with a callback reads in the same flush, keeps its value and does not recompute', () => {
    let estimateRuns = 0;
    const flag = createSourceNode(true);
    const estimate = createDerivedNode(() => {
      estimateRuns += 1;
      return 5;
    });
    const d1 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, estimate) : 0,
    );
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? 0 : readDerived(callingNode, estimate),
    );
    retainDerivedNode(d1, () => {});
    retainDerivedNode(d2, () => {});
    startWriteBatch();
    writeSource(flag, false);
    endWriteBatch();
    expect([readDerivedNode(d1), readDerivedNode(d2)]).toEqual([0, 5]);
    expect(estimateRuns).toBe(1);
    expectSame(dependentsOf(estimate), [d2]);
    expect(disposeNext()).toBe(false);
    expect(statusOf(estimate)).toBe(CLEAN);
  });

  test('a branch that flips and flips back before disposal keeps both branches from being disposed, and disposeAll disposes the branch that no node depends on', () => {
    const runs = { d1: 0, d2: 0 };
    const flag = createSourceNode(true);
    const d1 = createDerivedNode(() => {
      runs.d1 += 1;
      return 'b';
    });
    const d2 = createDerivedNode(() => {
      runs.d2 += 1;
      return 'c';
    });
    const d3 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, d1)
        : readDerived(callingNode, d2),
    );
    retainDerivedNode(d3);
    startWriteBatch();
    writeSource(flag, false);
    endWriteBatch();
    expect(readDerivedNode(d3)).toBe('c');
    startWriteBatch();
    writeSource(flag, true);
    endWriteBatch();
    expect(readDerivedNode(d3)).toBe('b');
    expect(runs).toEqual({ d1: 1, d2: 1 });
    disposeAll();
    expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, UNCOMPUTED]);
    startWriteBatch();
    writeSource(flag, false);
    endWriteBatch();
    expect(readDerivedNode(d3)).toBe('c');
    expect(runs).toEqual({ d1: 1, d2: 2 });
  });

  test('disposeNext disposes one node and returns whether the cleanup queue is non-empty, and later disposeNext calls dispose the dependencies that each disposal put on the cleanup queue', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const d3 = createDerivedNode((callingNode) => readDerived(callingNode, d2));
    releaseRetain(retainDerivedNode(d3));
    expect(disposeNext()).toBe(true);
    expect([statusOf(d3), statusOf(d2), statusOf(d1)]).toEqual([
      UNCOMPUTED,
      CLEAN,
      CLEAN,
    ]);
    expect(disposeNext()).toBe(true);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, CLEAN]);
    expect(disposeNext()).toBe(false);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(s1), []);
  });

  test('disposal skips a node that gained a dependent after the node was put on the cleanup queue, and the node goes on the cleanup queue again when the node loses that dependent', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    releaseRetain(retainDerivedNode(d1));
    const r1 = retainDerivedNode(d2);
    expect(disposeNext()).toBe(false);
    expect(statusOf(d1)).toBe(CLEAN);
    releaseRetain(r1);
    expect(disposeNext()).toBe(true);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, CLEAN]);
    disposeAll();
    expect(statusOf(d1)).toBe(UNCOMPUTED);
  });

  test('a node left unneeded twice before disposal is on the cleanup queue once per time, and disposeAll disposes it once', () => {
    let runs = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    releaseRetain(retainDerivedNode(d1));
    releaseRetain(retainDerivedNode(d1));
    expect(runs).toBe(1);
    expect(statusOf(d1)).toBe(CLEAN);
    expect(disposeNext()).toBe(true);
    expect(statusOf(d1)).toBe(UNCOMPUTED);
    expect(disposeNext()).toBe(false);
    expect(statusOf(d1)).toBe(UNCOMPUTED);
  });

  test('a lazy node, which an earlier refresh in the flush stops reading, is brought up to date by a later refresh that reads the node', () => {
    const runs = { observed: 0 };
    const s1 = createSourceNode(1);
    const flag = createSourceNode(true);
    const observed = createDerivedNode((callingNode) => {
      runs.observed += 1;
      return readSource(callingNode, s1) * 10;
    });
    const d1 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, observed) : 0,
    );
    const d2 = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? 0
        : readSource(callingNode, s1) + readDerived(callingNode, observed),
    );
    retainDerivedNode(d1, () => {});
    retainDerivedNode(d2, () => {});
    startWriteBatch();
    writeSource(flag, false);
    writeSource(s1, 2);
    endWriteBatch();
    expect(readDerivedNode(d2)).toBe(22);
    expect(runs.observed).toBe(2);
    expect(statusOf(observed)).toBe(CLEAN);
  });

  test('disposeAll disposes a chain of 100,000 nodes without overflowing the call stack', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    // The first recompute of the whole chain from its last node would overflow
    // the call stack, so each node is recomputed while its dependency is
    // CLEAN.
    let chainEnd = d1;
    let r1 = retainDerivedNode(chainEnd);
    for (let index = 0; index < 100_000; index += 1) {
      const dependency = chainEnd;
      chainEnd = createDerivedNode((callingNode) =>
        readDerived(callingNode, dependency),
      );
      const previousRetain = r1;
      r1 = retainDerivedNode(chainEnd);
      releaseRetain(previousRetain);
    }
    expect(scheduleCalls).toBe(0);
    releaseRetain(r1);
    disposeAll();
    expect([statusOf(chainEnd), statusOf(d1)]).toEqual([
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    expectSame(dependentsOf(s1), []);
  });
});

describe('a retain with an onChange gives a node a callback', () => {
  test('a retain with an onChange gives a lazy node a callback, the flush refreshes the node and calls the onChange, and the last release makes the node lazy again', () => {
    const s1 = createSourceNode(1);
    let runs = 0;
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    const calls: Array<number> = [];
    const r1 = retainDerivedNode(d1, () => calls.push(readDerivedNode(d1)));
    expect([statusOf(d1), internal(d1).evaluation, runs]).toEqual([
      CLEAN,
      OBSERVED,
      1,
    ]);
    retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(2);
    startWriteBatch();
    writeSource(s1, 2);
    endWriteBatch();
    expect([calls, runs, statusOf(d2)]).toEqual([[2], 2, DIRTY]);
    startWriteBatch();
    writeSource(s1, 3);
    endWriteBatch();
    expect(calls).toEqual([2, 3]);
    releaseRetain(r1);
    startWriteBatch();
    writeSource(s1, 4);
    endWriteBatch();
    expect([calls, runs, statusOf(d1)]).toEqual([[2, 3], 3, DIRTY]);
    expect(readDerivedNode(d2)).toBe(5);
    expect(internal(d1).evaluation).toBe(UNOBSERVED);
  });
});

describe('a synchronous scheduler', () => {
  test('a scheduler that disposes inside schedule disposes when the outermost operation ends, as immediate cleanup does', () => {
    scheduler.schedule = disposeAll;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const r1 = retainDerivedNode(d2);
    expect(readDerivedNode(d2)).toBe(1);
    releaseRetain(r1);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect(disposeNext()).toBe(false);
    const kept = retainDerivedNode(d2);
    const r2 = retainDerivedNode(d1);
    startWriteBatch();
    writeSource(s1, 2);
    releaseRetain(r2);
    expect(statusOf(d1)).toBe(DIRTY);
    endWriteBatch();
    expect([readDerivedNode(d2), statusOf(d1)]).toEqual([2, CLEAN]);
    releaseRetain(kept);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect(disposeNext()).toBe(false);
  });
});

describe('the end of the outermost operation', () => {
  test('a top-level read inside onChange does not call the scheduler, and the flush calls the scheduler once when the flush ends', () => {
    const calls: Array<string> = [];
    let phase = 'outside';
    const s1 = createSourceNode(0);
    const even = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const odd = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const branch = createDerivedNode((callingNode) =>
      readSource(callingNode, s1) % 2 === 0
        ? readDerived(callingNode, even)
        : readDerived(callingNode, odd),
    );
    const r1 = retainDerivedNode(branch, () => {
      phase = 'onChange';
      readDerivedNode(branch);
      phase = 'flush';
    });
    scheduler.schedule = () => calls.push(phase);
    startWriteBatch();
    writeSource(s1, 1);
    endWriteBatch();
    expect(calls).toEqual(['flush']);
    expect(statusOf(even)).toBe(DIRTY);
    expect(disposeNext()).toBe(false);
    expect(statusOf(even)).toBe(UNCOMPUTED);
    releaseRetain(r1);
  });
});

describe('scheduling after disposeAll', () => {
  test('a disposeAll outside an operation that puts dependencies on the cleanup queue while it disposes leaves no scheduler call for the next operation', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    releaseRetain(retainDerivedNode(d2));
    expect(scheduleCalls).toBe(1);
    disposeAll();
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    const d3 = createDerivedNode(() => 0);
    const r1 = retainDerivedNode(d3);
    expect(scheduleCalls).toBe(1);
    releaseRetain(r1);
    expect(scheduleCalls).toBe(2);
  });

  test('a disposeNext that empties the cleanup queue leaves no scheduler call for the next operation', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    releaseRetain(retainDerivedNode(d2));
    expect([disposeNext(), disposeNext()]).toEqual([true, false]);
    retainDerivedNode(createDerivedNode(() => 0));
    expect(scheduleCalls).toBe(1);
  });
});

describe('readDerivedNode of a node without a retain', () => {
  test('a top-level read without a retain recomputes the node and leaves it and its dependencies disposable', () => {
    const s1 = createSourceNode(1);
    let runs = 0;
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1);
    });
    const d2 = createDerivedNode(
      (callingNode) => readDerived(callingNode, d1) + 1,
    );
    expect(readDerivedNode(d2)).toBe(2);
    expect([statusOf(d2), statusOf(d1), runs]).toEqual([CLEAN, CLEAN, 1]);
    expect(scheduleCalls).toBe(1);
    expect(readDerivedNode(d2)).toBe(2);
    expect(runs).toBe(1);
    // Each top-level read put d2 on the cleanup queue once. Disposing d2 puts d1 on the cleanup queue.
    expect(disposeNext()).toBe(true);
    expect([statusOf(d2), statusOf(d1)]).toEqual([UNCOMPUTED, CLEAN]);
    expect(disposeNext()).toBe(true);
    expect(statusOf(d1)).toBe(UNCOMPUTED);
    expect(disposeNext()).toBe(false);
    expect(internal(s1).firstDependent).toBe(undefined);
  });

  test('a top-level read of a retained node, or of a node with a dependent, puts nothing on the cleanup queue, and a retain before disposal keeps the node', () => {
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const r1 = retainDerivedNode(d2);
    expect(readDerivedNode(d1)).toBe(1);
    expect(readDerivedNode(d2)).toBe(1);
    expect(scheduleCalls).toBe(0);
    const loose = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    readDerivedNode(loose);
    const kept = retainDerivedNode(loose);
    expect(disposeNext()).toBe(false);
    expect([statusOf(loose), readDerivedNode(loose)]).toEqual([CLEAN, 1]);
    releaseRetain(kept);
    releaseRetain(r1);
    disposeAll();
    expect([statusOf(d2), statusOf(d1), statusOf(loose)]).toEqual([
      UNCOMPUTED,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
  });
});

describe('disposal of the unneeded nodes that a write reaches', () => {
  test('a write that marks an unneeded node disposes that node when the write ends, disposes the marked dependencies of that node, and puts the CLEAN dependencies of that node on the cleanup queue', () => {
    const marked = createSourceNode(1);
    const untouched = createSourceNode(10);
    const dirtyDependency = createDerivedNode((callingNode) =>
      readSource(callingNode, marked),
    );
    const cleanDependency = createDerivedNode((callingNode) =>
      readSource(callingNode, untouched),
    );
    const d1 = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, dirtyDependency) +
        readDerived(callingNode, cleanDependency),
    );
    const unreached = createDerivedNode((callingNode) =>
      readSource(callingNode, untouched),
    );
    releaseRetain(retainDerivedNode(d1));
    releaseRetain(retainDerivedNode(unreached));
    expect([statusOf(d1), statusOf(unreached), scheduleCalls]).toEqual([
      CLEAN,
      CLEAN,
      1,
    ]);
    startWriteBatch();
    writeSource(marked, 2);
    endWriteBatch();
    expect([
      statusOf(d1),
      statusOf(dirtyDependency),
      statusOf(cleanDependency),
      statusOf(unreached),
    ]).toEqual([UNCOMPUTED, UNCOMPUTED, CLEAN, CLEAN]);
    // The cleanup queue is leaf, unreached, cleanDependency, and pops the last entry first.
    expect(disposeNext()).toBe(true);
    expect([statusOf(cleanDependency), statusOf(unreached)]).toEqual([
      UNCOMPUTED,
      CLEAN,
    ]);
    expect(disposeNext()).toBe(true);
    expect(statusOf(unreached)).toBe(UNCOMPUTED);
    expect(disposeNext()).toBe(false);
  });
});

describe('retains with an onChange', () => {
  test('the flush calls the onChange of every retain of a changed node once, most recent first, and a released retain is not called', () => {
    const s1 = createSourceNode(1);
    const observed = createDerivedNode((callingNode) =>
      readSource(callingNode, s1),
    );
    const calls: Array<string> = [];
    const r1 = retainDerivedNode(observed, () =>
      calls.push(`a${readDerivedNode(observed)}`),
    );
    const r2 = retainDerivedNode(observed, () =>
      calls.push(`b${readDerivedNode(observed)}`),
    );
    const r3 = retainDerivedNode(observed, () =>
      calls.push(`c${readDerivedNode(observed)}`),
    );
    const plain = retainDerivedNode(observed);
    expect(readDerivedNode(observed)).toBe(1);
    expect(calls).toEqual([]);
    startWriteBatch();
    writeSource(s1, 2);
    writeSource(s1, 3);
    endWriteBatch();
    expect(calls).toEqual(['c3', 'b3', 'a3']);
    releaseRetain(r2);
    startWriteBatch();
    writeSource(s1, 4);
    endWriteBatch();
    expect(calls).toEqual(['c3', 'b3', 'a3', 'c4', 'a4']);
    releaseRetain(r3);
    releaseRetain(r1);
    expect(internal(observed).firstRetain).toBe(plain);
    expect(internal(plain).next).toBe(undefined);
    startWriteBatch();
    writeSource(s1, 5);
    endWriteBatch();
    expect(calls.length).toBe(5);
    expect(readDerivedNode(observed)).toBe(5);
  });
});

describe('a finished graph', () => {
  beforeEach(() => {
    scheduler.schedule = disposeAll;
  });

  /**
   * Runs writes, batches, flushes with an `onChange`, and releases on a new graph, and returns weak references to its nodes.
   */
  function runGraph(): ReadonlyArray<
    WeakRef<SourceNode<number> | DerivedNode<number>>
  > {
    const source = createSourceNode(1);
    const lazy = createDerivedNode((callingNode) =>
      readSource(callingNode, source),
    );
    const observed = createDerivedNode(
      (callingNode) => readSource(callingNode, source) * 10,
    );
    const observedRetain = retainDerivedNode(observed, () => {});
    const lazyRetain = retainDerivedNode(lazy);
    startWriteBatch();
    writeSource(source, 2);
    endWriteBatch();
    readDerivedNode(lazy);
    writeSource(source, 3);
    readDerivedNode(lazy);
    releaseRetain(observedRetain);
    releaseRetain(lazyRetain);
    return [new WeakRef(source), new WeakRef(lazy), new WeakRef(observed)];
  }

  test('the engine keeps no node of a finished graph reachable', async () => {
    const references = runGraph();
    await new Promise((resolve) => setTimeout(resolve, 0));
    const { gc } = globalThis;
    if (gc === undefined) {
      throw new Error('vitest.config.ts runs the tests with --expose-gc');
    }
    gc();
    expect(references.map((reference) => reference.deref())).toEqual([
      undefined,
      undefined,
      undefined,
    ]);
  });
});

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
