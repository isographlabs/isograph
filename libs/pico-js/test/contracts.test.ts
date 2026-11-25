import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
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
  type CallingNode,
  type DerivedNode,
} from '../src/index';
import { CLEAN, DIRTY, MAYBE_DIRTY, UNCOMPUTED } from '../src/nodes';
import type * as EngineModule from '../src/index';
import { at } from './at';
import { statusOf } from './status';

beforeEach(() => {
  scheduler.schedule = disposeAll;
});

afterEach(disposeAll);

describe('contracts for performance', () => {
  test('updating a source with its current value recomputes its dependents to the same value and calls no onChange', () => {
    let runs = 0;
    let calls = 0;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, s1) * 2;
    });
    const d2 = createDerivedNode((callingNode) => readDerived(callingNode, d1));
    const r1 = retainDerivedNode(d1, () => {
      calls += 1;
    });
    const r2 = retainDerivedNode(d2);
    writeSource(s1, 1);
    expect([runs, calls, readDerivedNode(d1), readDerivedNode(d2)]).toEqual([
      2, 0, 2, 2,
    ]);
    releaseRetain(r1);
    releaseRetain(r2);
  });

  test('a computation that returns an equal new value instead of previous makes its dependents recompute and calls its onChange, and one that returns previous does not', () => {
    for (const returnPrevious of [false, true]) {
      let dependentRuns = 0;
      let calls = 0;
      const s1 = createSourceNode(1);
      const equal = createDerivedNode<Array<number>>(
        (callingNode, previous) => {
          readSource(callingNode, s1);
          return returnPrevious && previous !== undefined ? previous : [0];
        },
      );
      const dependent = createDerivedNode((callingNode) => {
        dependentRuns += 1;
        return readDerived(callingNode, equal).length;
      });
      const r1 = retainDerivedNode(equal, () => {
        calls += 1;
      });
      const r2 = retainDerivedNode(dependent);
      writeSource(s1, 2);
      readDerivedNode(dependent);
      expect([returnPrevious, dependentRuns, calls]).toEqual(
        returnPrevious ? [true, 1, 0] : [false, 2, 1],
      );
      releaseRetain(r1);
      releaseRetain(r2);
      disposeAll();
    }
  });
});

describe('contracts for correctness', () => {
  test('a computation that reads state other than through readSource and readDerived keeps its value when that state changes, and recomputes with the new state when a source it read changes', () => {
    let external = 1;
    const s1 = createSourceNode(1);
    const d1 = createDerivedNode(
      (callingNode) => readSource(callingNode, s1) * external,
    );
    const r1 = retainDerivedNode(d1);
    external = 10;
    expect(readDerivedNode(d1)).toBe(1);
    writeSource(s1, 2);
    expect(readDerivedNode(d1)).toBe(20);
    releaseRetain(r1);
  });

  describe('write batches', () => {
    test('a batch with no matching endWriteBatch leaves later writes unflushed, and the endWriteBatch that closes it flushes them', () => {
      const calls: Array<number> = [];
      const s1 = createSourceNode(1);
      const d1 = createDerivedNode((callingNode) =>
        readSource(callingNode, s1),
      );
      const r1 = retainDerivedNode(d1, () => calls.push(readDerivedNode(d1)));
      startWriteBatch();
      writeSource(s1, 2);
      writeSource(s1, 3);
      expect(calls).toEqual([]);
      endWriteBatch();
      expect(calls).toEqual([3]);
      releaseRetain(r1);
    });

    test('a write made by other code while a batch is open is flushed when that batch ends', () => {
      const calls: Array<string> = [];
      const s1 = createSourceNode(1);
      const s2 = createSourceNode(1);
      const d1 = createDerivedNode((callingNode) =>
        readSource(callingNode, s1),
      );
      const d2 = createDerivedNode((callingNode) =>
        readSource(callingNode, s2),
      );
      const r1 = retainDerivedNode(d1, () => calls.push('d1'));
      const r2 = retainDerivedNode(d2, () => calls.push('d2'));
      startWriteBatch();
      writeSource(s1, 2);
      writeSource(s2, 2);
      expect(calls).toEqual([]);
      endWriteBatch();
      expect(calls).toEqual(['d1', 'd2']);
      releaseRetain(r1);
      releaseRetain(r2);
    });
  });

  describe('retains and the scheduler', () => {
    test('a retain that is never released keeps its node and its dependencies cached after disposeAll', () => {
      const s1 = createSourceNode(1);
      const d1 = createDerivedNode((callingNode) =>
        readSource(callingNode, s1),
      );
      const d2 = createDerivedNode((callingNode) =>
        readDerived(callingNode, d1),
      );
      const r1 = retainDerivedNode(d2);
      disposeAll();
      expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, CLEAN]);
      releaseRetain(r1);
      disposeAll();
      expect([statusOf(d1), statusOf(d2)]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    });

    test('a scheduler that stops after one disposeNext leaves the other unneeded nodes cached, and pico-js does not call it again until the cleanup queue is empty', () => {
      let scheduleCalls = 0;
      scheduler.schedule = () => {
        scheduleCalls += 1;
        disposeNext();
      };
      const s1 = createSourceNode(1);
      const create = () =>
        createDerivedNode((callingNode) => readSource(callingNode, s1));
      const [d1, d2, extra] = [create(), create(), create()];
      const r1 = retainDerivedNode(d1);
      const r2 = retainDerivedNode(d2);
      startWriteBatch();
      releaseRetain(r1);
      releaseRetain(r2);
      endWriteBatch();
      expect(scheduleCalls).toBe(1);
      expect([statusOf(d1), statusOf(d2)]).toEqual([CLEAN, UNCOMPUTED]);
      releaseRetain(retainDerivedNode(extra));
      expect([scheduleCalls, statusOf(extra)]).toEqual([1, CLEAN]);
      disposeAll();
      expect([d1, d2, extra].map((node) => statusOf(node))).toEqual([
        UNCOMPUTED,
        UNCOMPUTED,
        UNCOMPUTED,
      ]);
      releaseRetain(retainDerivedNode(extra));
      expect(scheduleCalls).toBe(2);
    });
  });
});

type Engine = typeof EngineModule;

/**
 * A new instance of the engine's module, with a scheduler that does nothing,
 * so a violation that leaves the state unspecified does not reach other tests.
 */
async function freshEngine(): Promise<Engine> {
  vi.resetModules();
  const engine = await import('../src/index');
  engine.scheduler.schedule = () => {};
  return engine;
}

describe('violations with a result the README states', () => {
  test('a write inside an onChange to a source that no node with a callback depends on marks its dependents, runs no computation, and lets the flush call the later callbacks', async () => {
    const e = await freshEngine();
    const s1 = e.createSourceNode(1);
    const s2 = e.createSourceNode(10);
    let lazyRuns = 0;
    const lazy = e.createDerivedNode((callingNode) => {
      lazyRuns += 1;
      return e.readSource(callingNode, s2) * 2;
    });
    e.retainDerivedNode(lazy);
    const first = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s1),
    );
    const second = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s1) + 100,
    );
    const seen: Array<unknown> = [];
    e.retainDerivedNode(first, () => {
      e.writeSource(s2, 20);
      seen.push([statusOf(lazy), lazyRuns]);
    });
    e.retainDerivedNode(second, (value) => seen.push(value));
    e.writeSource(s1, 2);
    expect(seen).toEqual([[DIRTY, 1], 102]);
    expect(e.readDerivedNode(lazy)).toBe(40);
  });

  test('a write inside an onChange to a source that a node with a callback depends on brings that node up to date and throws a TypeError before any callback of the nested flush, whether or not the value changed', async () => {
    for (const compute of [(v: number) => v * 3, (v: number) => v > 0]) {
      const e = await freshEngine();
      const s1 = e.createSourceNode(1);
      const s2 = e.createSourceNode(10);
      const first = e.createDerivedNode((callingNode) =>
        e.readSource(callingNode, s1),
      );
      let reachedRuns = 0;
      const reached = e.createDerivedNode((callingNode) => {
        reachedRuns += 1;
        return compute(e.readSource(callingNode, s2));
      });
      const calls: Array<string> = [];
      e.retainDerivedNode(first, () => {
        calls.push('first');
        e.writeSource(s2, 20);
        calls.push('after the write');
      });
      e.retainDerivedNode(reached, () => calls.push('reached'));
      expect(() => e.writeSource(s1, 2)).toThrow(TypeError);
      expect([calls, reachedRuns, statusOf(reached)]).toEqual([
        ['first'],
        2,
        CLEAN,
      ]);
    }
  });

  test('a batch inside an onChange that writes a source a node with a callback depends on throws a TypeError at its endWriteBatch', async () => {
    const e = await freshEngine();
    const s1 = e.createSourceNode(1);
    const s2 = e.createSourceNode(10);
    const first = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s1),
    );
    const reached = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s2),
    );
    const calls: Array<string> = [];
    e.retainDerivedNode(first, () => {
      e.startWriteBatch();
      e.writeSource(s2, 20);
      calls.push('after the write');
      e.endWriteBatch();
      calls.push('after endWriteBatch');
    });
    e.retainDerivedNode(reached, () => calls.push('reached'));
    expect(() => e.writeSource(s1, 2)).toThrow(TypeError);
    expect(calls).toEqual(['after the write']);
  });

  test('a read inside a batch skips the callback of the read node and of a node with a callback that the writes reach only through it, and keeps the callback of a node they reach through a lazy node', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const read = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 2,
    );
    const beyond = e.createDerivedNode(
      (callingNode) => e.readDerived(callingNode, read) + 1,
    );
    const lazy = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 5,
    );
    const throughLazy = e.createDerivedNode(
      (callingNode) => e.readDerived(callingNode, lazy) + 1,
    );
    const calls: Array<[string, number]> = [];
    e.retainDerivedNode(read, (value) => calls.push(['read', value]));
    e.retainDerivedNode(beyond, (value) => calls.push(['beyond', value]));
    e.retainDerivedNode(throughLazy, (value) =>
      calls.push(['throughLazy', value]),
    );
    e.startWriteBatch();
    e.writeSource(s, 2);
    expect(e.readDerivedNode(read)).toBe(4);
    e.endWriteBatch();
    expect(calls).toEqual([['throughLazy', 11]]);
    expect(e.readDerivedNode(beyond)).toBe(5);
  });

  test('a retain inside a batch skips the callback of the retained node', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const node = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 2,
    );
    const calls: Array<number> = [];
    e.retainDerivedNode(node, (value) => calls.push(value));
    e.startWriteBatch();
    e.writeSource(s, 2);
    e.retainDerivedNode(node);
    e.endWriteBatch();
    expect([calls, e.readDerivedNode(node)]).toEqual([[], 4]);
  });

  test('a computation that reads its own node, or a node that depends on its node, throws a RangeError', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const self: DerivedNode<number> = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s) > 1 ? e.readDerived(callingNode, self) : 0,
    );
    e.retainDerivedNode(self);
    e.writeSource(s, 2);
    expect(() => e.readDerivedNode(self)).toThrow(RangeError);

    const f = await freshEngine();
    const flip = f.createSourceNode(false);
    const above: DerivedNode<number> = f.createDerivedNode(
      (callingNode) => f.readDerived(callingNode, below) * 10,
    );
    const below: DerivedNode<number> = f.createDerivedNode((callingNode) =>
      f.readSource(callingNode, flip) ? f.readDerived(callingNode, above) : 1,
    );
    f.retainDerivedNode(above);
    f.writeSource(flip, true);
    expect(() => f.readDerivedNode(below)).toThrow(RangeError);
  });

  test('after an extra endWriteBatch, writes flush at once even inside a batch, and a release calls schedule only while the batch depth is zero', async () => {
    const e = await freshEngine();
    let scheduleCalls = 0;
    e.scheduler.schedule = () => {
      scheduleCalls += 1;
    };
    const s = e.createSourceNode(1);
    const node = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s),
    );
    const calls: Array<number> = [];
    e.retainDerivedNode(node, (value) => calls.push(value));
    e.endWriteBatch();
    e.writeSource(s, 2);
    const unneeded = () =>
      e.createDerivedNode((callingNode) => e.readSource(callingNode, s));
    e.releaseRetain(e.retainDerivedNode(unneeded()));
    expect([calls, scheduleCalls]).toEqual([[2], 0]);
    e.startWriteBatch();
    e.writeSource(s, 3);
    expect(calls).toEqual([2, 3]);
    e.releaseRetain(e.retainDerivedNode(unneeded()));
    expect(scheduleCalls).toBe(1);
    e.endWriteBatch();
  });

  test('a schedule that throws throws out of the call that called it after the call did its work, and is not called again until the cleanup queue is emptied and refilled', async () => {
    const e = await freshEngine();
    let scheduleCalls = 0;
    e.scheduler.schedule = () => {
      scheduleCalls += 1;
      throw new Error('schedule');
    };
    const s = e.createSourceNode(1);
    const node = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, s),
    );
    expect(() => e.readDerivedNode(node)).toThrow('schedule');
    expect([scheduleCalls, statusOf(node)]).toEqual([1, CLEAN]);
    const other = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) + 1,
    );
    expect(e.readDerivedNode(other)).toBe(2);
    expect(scheduleCalls).toBe(1);
    e.disposeAll();
    const retain = e.retainDerivedNode(other);
    expect(() => e.releaseRetain(retain)).toThrow('schedule');
    expect(scheduleCalls).toBe(2);
  });

  test('readDerivedNode inside a computation records no dependency, so the computation keeps its cached value when the read node changes', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const inner = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 2,
    );
    e.retainDerivedNode(inner);
    const outer = e.createDerivedNode(() => e.readDerivedNode(inner) + 1);
    e.retainDerivedNode(outer);
    expect(e.readDerivedNode(outer)).toBe(3);
    e.writeSource(s, 5);
    expect([e.readDerivedNode(outer), e.readDerivedNode(inner)]).toEqual([
      3, 10,
    ]);
  });

  test("a read with another node's callingNode, kept after its computation returned, records the dependency on that node, which recomputes the next time it is read, and the reading node keeps its value", async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const t = e.createSourceNode(100);
    const kept: Array<CallingNode> = [];
    let ownerRuns = 0;
    const owner = e.createDerivedNode((callingNode) => {
      ownerRuns += 1;
      kept.push(callingNode);
      return e.readSource(callingNode, s);
    });
    e.retainDerivedNode(owner);
    const reader = e.createDerivedNode(() => e.readSource(at(kept, 0), t));
    e.retainDerivedNode(reader);
    expect(e.readDerivedNode(reader)).toBe(100);
    e.writeSource(t, 200);
    expect([e.readDerivedNode(reader), statusOf(owner)]).toEqual([100, DIRTY]);
    expect([e.readDerivedNode(owner), ownerRuns]).toEqual([1, 2]);
  });

  test("an asynchronous computation's value is its promise, and its reads after an await record dependencies of its own node, which recomputes the next time it is read after one of them changes", async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const t = e.createSourceNode(10);
    let runs = 0;
    const node = e.createDerivedNode(async (callingNode) => {
      runs += 1;
      const before = e.readSource(callingNode, s);
      await Promise.resolve();
      return before + e.readSource(callingNode, t);
    });
    e.retainDerivedNode(node);
    const first = e.readDerivedNode(node);
    expect([first instanceof Promise, await first]).toEqual([true, 11]);
    e.writeSource(t, 20);
    expect(statusOf(node)).toBe(DIRTY);
    const second = e.readDerivedNode(node);
    expect([second === first, runs, await second]).toEqual([false, 2, 21]);
  });

  test('readDerivedNode inside a computation that leaves the cleanup queue non-empty calls schedule while the computation runs, and during a flush the write that ran the flush calls schedule before it returns', async () => {
    const e = await freshEngine();
    let running = false;
    const scheduleCalls: Array<string> = [];
    e.scheduler.schedule = () => {
      scheduleCalls.push(running ? 'during the computation' : 'after it');
    };
    const s = e.createSourceNode(1);
    const unretained = e.createDerivedNode(() => 7);
    const node = e.createDerivedNode((callingNode) => {
      running = true;
      e.readDerivedNode(unretained);
      running = false;
      return e.readSource(callingNode, s);
    });
    e.retainDerivedNode(node, () => {});
    expect(scheduleCalls).toEqual(['during the computation']);
    e.disposeAll();
    e.writeSource(s, 2);
    expect(scheduleCalls).toEqual(['during the computation', 'after it']);
  });

  test('a read and a retain inside a batch that leave the cleanup queue non-empty call schedule inside the batch', async () => {
    const e = await freshEngine();
    let scheduleCalls = 0;
    e.scheduler.schedule = () => {
      scheduleCalls += 1;
    };
    const unretained = e.createDerivedNode(() => 1);
    e.startWriteBatch();
    e.readDerivedNode(unretained);
    expect(scheduleCalls).toBe(1);
    e.endWriteBatch();
    e.disposeAll();
    const flag = e.createSourceNode(true);
    const dropped = e.createDerivedNode(() => 2);
    const node = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, flag) ? e.readDerived(callingNode, dropped) : 0,
    );
    e.retainDerivedNode(node);
    expect(scheduleCalls).toBe(1);
    e.startWriteBatch();
    e.writeSource(flag, false);
    e.retainDerivedNode(node);
    expect(scheduleCalls).toBe(2);
    e.endWriteBatch();
  });

  test('a retainDerivedNode whose schedule throws has computed the node and returns no retain, and the retain it made keeps the node cached', async () => {
    const e = await freshEngine();
    e.scheduler.schedule = () => {
      throw new Error('schedule');
    };
    const flag = e.createSourceNode(true);
    const dropped = e.createDerivedNode(() => 2);
    let runs = 0;
    const node = e.createDerivedNode((callingNode) => {
      runs += 1;
      return e.readSource(callingNode, flag)
        ? e.readDerived(callingNode, dropped)
        : 0;
    });
    const r1 = e.retainDerivedNode(node);
    e.writeSource(flag, false);
    expect(() => e.retainDerivedNode(node)).toThrow('schedule');
    expect([runs, statusOf(node)]).toEqual([2, CLEAN]);
    e.scheduler.schedule = () => {};
    e.releaseRetain(r1);
    expect(e.readDerivedNode(node)).toBe(0);
    e.disposeAll();
    expect(statusOf(node)).toBe(CLEAN);
  });

  test('after an extra endWriteBatch, writeSource and endWriteBatch call schedule only while the batch depth is zero', async () => {
    const e = await freshEngine();
    let scheduleCalls = 0;
    e.scheduler.schedule = () => {
      scheduleCalls += 1;
    };
    const flag = e.createSourceNode(true);
    const dropped = e.createDerivedNode(() => 1);
    const node = e.createDerivedNode((callingNode) =>
      e.readSource(callingNode, flag) ? e.readDerived(callingNode, dropped) : 0,
    );
    e.retainDerivedNode(node, () => {});
    e.endWriteBatch();
    e.writeSource(flag, false);
    e.endWriteBatch();
    expect(scheduleCalls).toBe(0);
    e.startWriteBatch();
    e.startWriteBatch();
    e.startWriteBatch();
    e.endWriteBatch();
    expect(scheduleCalls).toBe(1);
    e.disposeAll();
    e.writeSource(flag, true);
    e.writeSource(flag, false);
    expect(scheduleCalls).toBe(2);
  });

  test('after a read inside a batch changes the value of a node with a callback, the node with a callback that reads it keeps its old value and its callback does not run on later writes until a top-level read, and runs after that read', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const read = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 2,
    );
    let beyondRuns = 0;
    const beyond = e.createDerivedNode((callingNode) => {
      beyondRuns += 1;
      return e.readDerived(callingNode, read) + 1;
    });
    const calls: Array<[string, number]> = [];
    e.retainDerivedNode(read, (value) => calls.push(['read', value]));
    e.retainDerivedNode(beyond, (value) => calls.push(['beyond', value]));
    e.startWriteBatch();
    e.writeSource(s, 2);
    e.readDerivedNode(read);
    e.endWriteBatch();
    e.writeSource(s, 3);
    e.writeSource(s, 4);
    expect([calls, beyondRuns, statusOf(beyond)]).toEqual([
      [
        ['read', 6],
        ['read', 8],
      ],
      1,
      DIRTY,
    ]);
    expect(e.readDerivedNode(beyond)).toBe(9);
    e.writeSource(s, 5);
    expect(calls.slice(2)).toEqual([
      ['read', 10],
      ['beyond', 11],
    ]);
  });

  test('after a read inside a batch changes the value of a node with a callback, the flush refreshes the node with a callback that reads it, with its callbacks, when a write reaches a node with a callback above it through another source', async () => {
    const e = await freshEngine();
    const s = e.createSourceNode(1);
    const t = e.createSourceNode(0);
    const read = e.createDerivedNode(
      (callingNode) => e.readSource(callingNode, s) * 2,
    );
    const beyond = e.createDerivedNode(
      (callingNode) => e.readDerived(callingNode, read) + 1,
    );
    const above = e.createDerivedNode(
      (callingNode) =>
        e.readDerived(callingNode, beyond) * 10 + e.readSource(callingNode, t),
    );
    const calls: Array<[string, number]> = [];
    e.retainDerivedNode(read, (value) => calls.push(['read', value]));
    e.retainDerivedNode(beyond, (value) => calls.push(['beyond', value]));
    e.retainDerivedNode(above, (value) => calls.push(['above', value]));
    e.startWriteBatch();
    e.writeSource(s, 2);
    e.readDerivedNode(read);
    e.endWriteBatch();
    e.writeSource(s, 3);
    expect([calls, statusOf(beyond), statusOf(above)]).toEqual([
      [['read', 6]],
      DIRTY,
      MAYBE_DIRTY,
    ]);
    e.writeSource(t, 1);
    expect(calls.slice(1)).toEqual([
      ['beyond', 7],
      ['above', 71],
    ]);
  });

  test('disposeAll and disposeNext inside a batch dispose the unneeded nodes on the cleanup queue, the batch delivers the same callbacks and values as with the disposal after it, and the end of the batch calls schedule for a node that joins the emptied cleanup queue', async () => {
    const drainDisposeNext = (e: Engine): void => {
      if (e.disposeNext()) {
        drainDisposeNext(e);
      }
    };
    const run = async (
      dispose: 'disposeAll' | 'disposeNext',
      when: 'inside the batch' | 'after the batch',
    ) => {
      const e = await freshEngine();
      let scheduleCalls = 0;
      e.scheduler.schedule = () => {
        scheduleCalls += 1;
      };
      const drain = () =>
        dispose === 'disposeAll' ? e.disposeAll() : drainDisposeNext(e);
      const s = e.createSourceNode(1);
      const flag = e.createSourceNode(true);
      const low = e.createDerivedNode(
        (callingNode) => e.readSource(callingNode, s) + 1,
      );
      const mid = e.createDerivedNode(
        (callingNode) => e.readDerived(callingNode, low) * 2,
      );
      const unneeded = e.createDerivedNode(
        (callingNode) => e.readDerived(callingNode, low) + 100,
      );
      const top = e.createDerivedNode((callingNode) =>
        e.readSource(callingNode, flag) ? e.readDerived(callingNode, mid) : -1,
      );
      const calls: Array<[string, number]> = [];
      e.retainDerivedNode(top, (value) => calls.push(['top', value]));
      const midRetain = e.retainDerivedNode(mid, (value) =>
        calls.push(['mid', value]),
      );
      e.readDerivedNode(unneeded);
      e.startWriteBatch();
      e.writeSource(s, 2);
      e.releaseRetain(midRetain);
      e.writeSource(flag, false);
      if (when === 'inside the batch') {
        drain();
      }
      const unneededInBatch = statusOf(unneeded);
      e.endWriteBatch();
      if (when === 'after the batch') {
        drain();
      }
      return {
        calls,
        unneededInBatch,
        scheduleCalls,
        values: [low, mid, unneeded, top].map((node) =>
          e.readDerivedNode(node),
        ),
      };
    };
    for (const dispose of ['disposeAll', 'disposeNext'] as const) {
      const expected = {
        calls: [['top', -1]],
        values: [3, 6, 103, -1],
      };
      expect(await run(dispose, 'inside the batch')).toEqual({
        ...expected,
        unneededInBatch: UNCOMPUTED,
        scheduleCalls: 2,
      });
      expect(await run(dispose, 'after the batch')).toEqual({
        ...expected,
        unneededInBatch: MAYBE_DIRTY,
        scheduleCalls: 1,
      });
    }
  });

  test('a top-level read of the last node of a chain of 1,000 nodes succeeds, and of a chain of 100,000 nodes throws a RangeError, on a first compute, on a recompute of nodes that all read the written source, and on a verification of nodes with callbacks', async () => {
    const chain = (
      e: Engine,
      length: number,
      compute: (
        callingNode: CallingNode,
        below: DerivedNode<number> | undefined,
      ) => number,
    ): Array<DerivedNode<number>> =>
      Array.from({ length }).reduce<Array<DerivedNode<number>>>((nodes) => {
        const below = nodes.at(-1);
        nodes.push(
          e.createDerivedNode((callingNode) => compute(callingNode, below)),
        );
        return nodes;
      }, []);
    const shapes: ReadonlyArray<(e: Engine, length: number) => () => number> = [
      (e, length) => {
        const s = e.createSourceNode(0);
        const nodes = chain(
          e,
          length,
          (callingNode, below) =>
            (below === undefined ? 0 : e.readDerived(callingNode, below)) +
            e.readSource(callingNode, s),
        );
        return () => e.readDerivedNode(at(nodes, length - 1));
      },
      (e, length) => {
        const s = e.createSourceNode(0);
        const nodes = chain(
          e,
          length,
          (callingNode, below) =>
            (below === undefined ? 0 : e.readDerived(callingNode, below)) +
            e.readSource(callingNode, s),
        );
        nodes.forEach((node) => e.retainDerivedNode(node));
        e.writeSource(s, 1);
        return () => e.readDerivedNode(at(nodes, length - 1));
      },
      (e, length) => {
        const s = e.createSourceNode(0);
        const nodes = chain(e, length, (callingNode, below) =>
          below === undefined
            ? Math.floor(e.readSource(callingNode, s) / 1000)
            : e.readDerived(callingNode, below) + 1,
        );
        nodes.forEach((node) => e.retainDerivedNode(node, () => {}));
        e.writeSource(s, 1);
        return () => e.readDerivedNode(at(nodes, length - 1));
      },
    ];
    for (const shape of shapes) {
      expect(shape(await freshEngine(), 1000)()).toBeGreaterThanOrEqual(0);
      expect(shape(await freshEngine(), 100_000)).toThrow(RangeError);
    }
  });
});
