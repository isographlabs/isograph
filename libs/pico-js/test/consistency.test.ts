import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { internal } from './internal';
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  endWriteBatch,
  readDerived,
  readDerivedNode,
  readSource,
  retainDerivedNode,
  scheduler,
  startWriteBatch,
  writeSource,
  type CallingNode,
  type DerivedNode,
  type SourceNode,
} from '../src/index';

beforeEach(() => {
  scheduler.schedule = disposeAll;
});

afterEach(disposeAll);

/** The reads a computation makes, through which it records what it saw. */
type Reads = {
  readonly source: (source: SourceNode<number>) => number;
  readonly derived: (node: DerivedNode<number>) => number;
};

/**
 * A graph whose computations are pure functions of what they read. Every
 * derived value a computation reads, and every top-level read, is compared
 * with the value computed from the current sources, and each mismatch is
 * recorded in `stale` with the node that saw it.
 */
function consistencyGraph() {
  const current = new Map<DerivedNode<number>, () => number>();
  const names = new Map<DerivedNode<number>, string>();
  const stale: Array<string> = [];
  const runs = new Map<string, number>();
  const currentValue = (node: DerivedNode<number>): number => {
    const reference = current.get(node);
    if (reference === undefined) {
      throw new Error('the node was not created by this graph');
    }
    return reference();
  };
  const node = (
    name: string,
    compute: (reads: Reads) => number,
  ): DerivedNode<number> => {
    const computation = (callingNode: CallingNode): number => {
      runs.set(name, (runs.get(name) ?? 0) + 1);
      return compute({
        source: (source) => readSource(callingNode, source),
        derived: (dependency) => {
          const value = readDerived(callingNode, dependency);
          const expected = currentValue(dependency);
          if (value !== expected) {
            stale.push(
              `${name} read ${names.get(dependency)} as ${value}, and its current value is ${expected}`,
            );
          }
          return value;
        },
      });
    };
    const created = createDerivedNode(computation);
    names.set(created, name);
    current.set(created, () =>
      compute({
        source: (source) => internal(source).value,
        derived: (dependency) => currentValue(dependency),
      }),
    );
    return created;
  };
  /** A top-level read, checked against the value from the current sources. */
  const read = (readNode: DerivedNode<number>, where: string): number => {
    const value = readDerivedNode(readNode);
    const expected = currentValue(readNode);
    if (value !== expected) {
      stale.push(
        `${where}: ${names.get(readNode)} read as ${value}, and its current value is ${expected}`,
      );
    }
    return value;
  };
  return { node, read, stale, runs };
}

function write(source: SourceNode<number>, value: number): void {
  startWriteBatch();
  try {
    writeSource(source, value);
  } finally {
    endWriteBatch();
  }
}

describe('no computation and no top-level read sees a stale value', () => {
  for (const [label, d1Value] of [
    ['changes', (s1: number) => s1 * 10],
    ['keeps its value', (s1: number) => (s1 > 0 ? 1 : 0)],
  ] as const) {
    test(`d3 reads s1, then d2 over d1, which has a callback, then d1, and d1 ${label}`, () => {
      const { node, read, stale } = consistencyGraph();
      const s1 = createSourceNode(1);
      const d1 = node('d1', (reads) => d1Value(reads.source(s1)));
      const d2 = node('d2', (reads) => reads.derived(d1) + 1);
      // d3 reads s1 first, so its edge leads s1's dependents list and the
      // write queues it before d1.
      const d3 = node(
        'd3',
        (reads) =>
          reads.source(s1) * 1000 + reads.derived(d2) * 100 + reads.derived(d1),
      );
      retainDerivedNode(d3, () => {});
      retainDerivedNode(d1, () => {});
      read(d3, 'before the write');
      write(s1, 2);
      read(d3, 'after the write');
      read(d2, 'after the write');
      expect(stale).toEqual([]);
    });
  }

  for (const length of [2, 3, 10]) {
    test(`a chain of ${length} links with callbacks, each reading s1 and then the link below`, () => {
      const { node, read, stale } = consistencyGraph();
      const s1 = createSourceNode(1);
      let top = node('link 0', (reads) => reads.source(s1));
      const links = [top];
      for (let index = 1; index < length; index += 1) {
        const below = top;
        top = node(
          `link ${index}`,
          (reads) => reads.source(s1) + reads.derived(below),
        );
        links.push(top);
      }
      retainDerivedNode(top, () => {});
      links.forEach((link) => retainDerivedNode(link, () => {}));
      read(top, 'before the write');
      write(s1, 2);
      read(top, 'after the write');
      expect(stale).toEqual([]);
    });
  }

  test('a node with a callback that reads s1 and then two nodes with callbacks over s1', () => {
    const { node, read, stale } = consistencyGraph();
    const s1 = createSourceNode(0);
    const left = node('left', (reads) => reads.source(s1) + 1);
    const right = node('right', (reads) => reads.source(s1) + 2);
    const below = node(
      'below',
      (reads) =>
        reads.source(s1) +
        reads.derived(left) * 10 +
        reads.derived(right) * 100,
    );
    retainDerivedNode(below, () => {});
    retainDerivedNode(left, () => {});
    retainDerivedNode(right, () => {});
    write(s1, 1);
    read(below, 'after the write');
    expect(stale).toEqual([]);
  });

  // Contract: no top-level read of a derived node inside a batch. A source's
  // value inside a batch is the value last written.
  test('a source read inside a batch returns the value last written', () => {
    const s1 = createSourceNode(1);
    const seen: Array<number> = [];
    startWriteBatch();
    writeSource(s1, 2);
    seen.push(internal(s1).value);
    writeSource(s1, 3);
    seen.push(internal(s1).value);
    endWriteBatch();
    expect(seen).toEqual([2, 3]);
  });

  test('an onChange that reads a lazy node over another node with a callback of the same flush', () => {
    const { node, read, stale } = consistencyGraph();
    const s1 = createSourceNode(1);
    const first = node('first', (reads) => reads.source(s1) * 2);
    const second = node('second', (reads) => reads.source(s1) * 3);
    const overSecond = node('overSecond', (reads) => reads.derived(second));
    retainDerivedNode(overSecond);
    retainDerivedNode(second, () => {});
    read(overSecond, 'before the write');
    retainDerivedNode(first, () => {
      read(overSecond, 'in the onChange of first');
      read(second, 'in the onChange of first');
    });
    write(s1, 2);
    expect(stale).toEqual([]);
  });

  for (const target of [
    'the node on the flush queue',
    'a lazy node over the node on the flush queue',
  ] as const) {
    test(`a refresh whose branch switch newly reads ${target}`, () => {
      const { node, read, stale } = consistencyGraph();
      const s0 = createSourceNode(0);
      const s1 = createSourceNode(5);
      const queued = node('queued', (reads) => reads.source(s0) * 10);
      const over = node('over', (reads) => reads.derived(queued) + 1);
      // switcher reads s0 first, so the write puts it on the flush queue before
      // queued.
      const switcher = node('switcher', (reads) =>
        reads.source(s0) % 2 === 0
          ? reads.source(s1)
          : target === 'the node on the flush queue'
            ? reads.derived(queued)
            : reads.derived(over),
      );
      retainDerivedNode(switcher, () => {});
      retainDerivedNode(queued, () => {});
      read(over, 'before the write');
      write(s0, 1);
      read(switcher, 'after the write');
      expect(stale).toEqual([]);
    });
  }

  test('a node with a callback whose branch switch newly reads a node with a callback that is higher and on the flush queue', () => {
    const { node, read, stale } = consistencyGraph();
    const s0 = createSourceNode(0);
    const s1 = createSourceNode(3);
    const base = node('base', (reads) => reads.source(s0) + 1);
    const middle = node('middle', (reads) => reads.derived(base) * 2);
    const deep = node('deep', (reads) => reads.derived(middle) + 7);
    const below = node('below', (reads) => reads.derived(deep) * 3);
    // The switching node reads s0 first and only sources until the switch.
    const switching = node('switching', (reads) =>
      reads.source(s0) === 0 ? reads.source(s1) : reads.derived(below),
    );
    retainDerivedNode(switching, () => {});
    retainDerivedNode(deep, () => {});
    read(below, 'before the write');
    write(s0, 1);
    read(switching, 'after the write');
    read(below, 'after the write');
    expect(stale).toEqual([]);
  });

  test('a branch switch that links a reader to a node above it, so the next write through the reused edges reads nothing stale', () => {
    const { node, read, stale } = consistencyGraph();
    const s0 = createSourceNode(0);
    const s1 = createSourceNode(1);
    const queued = node('queued', (reads) => reads.source(s1) * 10);
    const over = node('over', (reads) => reads.derived(queued) + 1);
    // switcher reads only sources until the switch.
    const switcher = node('switcher', (reads) =>
      reads.source(s0) % 2 === 0 ? 0 : reads.derived(over),
    );
    const lazyOverSwitcher = node(
      'lazyOverSwitcher',
      (reads) => reads.derived(switcher) + 2,
    );
    // top reads s1 first, so the second write queues it directly.
    const top = node(
      'top',
      (reads) => reads.source(s1) + reads.derived(lazyOverSwitcher) * 100,
    );
    retainDerivedNode(queued, () => {});
    retainDerivedNode(switcher, () => {});
    retainDerivedNode(top, () => {});
    read(top, 'before the writes');
    // The switch links switcher to over, which is above it.
    write(s0, 1);
    read(top, 'after the switch');
    // The write puts queued and top on the flush queue, and queued's refresh
    // puts switcher there through the reused edges.
    write(s1, 2);
    read(top, 'after the second write');
    expect(stale).toEqual([]);
  });

  // `projection` keeps its value on each cutoff write. `beforeProjection`
  // reads `t` first, so a batch that writes `t` and then `s` queues it before
  // `projection`, and its refresh reads the lazy `reader` before the refresh
  // of `projection`.
  function projectionGraph() {
    const graph = consistencyGraph();
    const { node } = graph;
    const s = createSourceNode(0);
    const t = createSourceNode(0);
    const projection = node('projection', (reads) =>
      Math.floor(reads.source(s) / 100),
    );
    const changes: Array<string> = [];
    retainDerivedNode(projection, (value) =>
      changes.push(`projection ${value}`),
    );
    const subscribers = [1, 2, 3].map((index) => {
      const subscriber = node(
        `subscriber ${index}`,
        (reads) => reads.derived(projection) * 10 + index,
      );
      retainDerivedNode(subscriber, (value) =>
        changes.push(`subscriber ${index} ${value}`),
      );
      return subscriber;
    });
    const cutoffWrites = (from: number) => {
      for (let value = from; value < from + 20; value += 1) {
        write(s, value);
      }
    };
    const changeAfterT = (beforeProjection: DerivedNode<number>) => {
      startWriteBatch();
      writeSource(t, 1);
      writeSource(s, 100);
      endWriteBatch();
      graph.read(beforeProjection, 'after the change');
    };
    return {
      ...graph,
      s,
      t,
      projection,
      subscribers,
      changes,
      cutoffWrites,
      changeAfterT,
    };
  }

  test('a projection with a callback whose lazy reader a read makes CLEAN between cutoff writes', () => {
    const {
      node,
      read,
      stale,
      t,
      projection,
      subscribers,
      changes,
      cutoffWrites,
      changeAfterT,
    } = projectionGraph();
    const reader = node('reader', (reads) => reads.derived(projection) + 7);
    const beforeProjection = node(
      'beforeProjection',
      (reads) => reads.source(t) + reads.derived(reader) * 1000,
    );
    // The retain without an onChange keeps the lazy reader linked to
    // projection across the top-level reads.
    retainDerivedNode(reader);
    cutoffWrites(1);
    read(reader, 'after the first cutoff writes');
    retainDerivedNode(beforeProjection, (value) =>
      changes.push(`beforeProjection ${value}`),
    );
    cutoffWrites(21);
    expect(changes).toEqual([]);
    changeAfterT(beforeProjection);
    read(reader, 'after the change');
    subscribers.forEach((subscriber) => read(subscriber, 'after the change'));
    expect(stale).toEqual([]);
    // The refresh of beforeProjection refreshes projection at once, so the
    // change of projection comes first.
    expect(changes).toEqual([
      'projection 1',
      'beforeProjection 8001',
      'subscriber 1 11',
      'subscriber 2 12',
      'subscriber 3 13',
    ]);
  });

  test('a subscriber that links to a projection with a callback after cutoff writes', () => {
    const {
      node,
      read,
      stale,
      t,
      projection,
      subscribers,
      changes,
      cutoffWrites,
      changeAfterT,
    } = projectionGraph();
    const late = node('late', (reads) => reads.derived(projection) + 5);
    // beforeProjection reads only `t` until the change, so it does not link
    // `late` up front.
    const beforeProjection = node('beforeProjection', (reads) =>
      reads.source(t) === 0 ? 0 : 1 + reads.derived(late) * 1000,
    );
    retainDerivedNode(beforeProjection, (value) =>
      changes.push(`beforeProjection ${value}`),
    );
    cutoffWrites(1);
    retainDerivedNode(late, (value) => changes.push(`late ${value}`));
    read(late, 'after the link');
    cutoffWrites(21);
    expect(changes).toEqual([]);
    changeAfterT(beforeProjection);
    read(late, 'after the change');
    subscribers.forEach((subscriber) => read(subscriber, 'after the change'));
    expect(stale).toEqual([]);
    // The refresh of beforeProjection refreshes late, and the refresh of late
    // refreshes projection, at once.
    expect(changes).toEqual([
      'projection 1',
      'late 6',
      'beforeProjection 6001',
      'subscriber 1 11',
      'subscriber 2 12',
      'subscriber 3 13',
    ]);
  });

  test('a reader of two nodes with callbacks at different depths', () => {
    const { node, read, stale } = consistencyGraph();
    const s1 = createSourceNode(1);
    const shallow = node('shallow', (reads) => reads.source(s1) + 1);
    const mid = node('mid', (reads) => reads.source(s1) * 2);
    const lazyOverMid = node('lazyOverMid', (reads) => reads.derived(mid) + 3);
    const deep = node('deep', (reads) => reads.derived(lazyOverMid) * 5);
    const reader = node(
      'reader',
      (reads) =>
        reads.source(s1) +
        reads.derived(deep) * 100 +
        reads.derived(shallow) * 10000,
    );
    retainDerivedNode(reader, () => {});
    retainDerivedNode(shallow, () => {});
    retainDerivedNode(mid, () => {});
    retainDerivedNode(deep, () => {});
    write(s1, 2);
    read(reader, 'after the write');
    write(s1, 3);
    read(reader, 'after the second write');
    expect(stale).toEqual([]);
  });
});

describe('onChange', () => {
  test('onChange does not run when the node ends the flush with its previous value', () => {
    const s1 = createSourceNode(0);
    const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
    const d2 = createDerivedNode(
      (callingNode) =>
        (readSource(callingNode, s1) + readDerived(callingNode, d1)) % 2,
    );
    let calls = 0;
    retainDerivedNode(d2, () => {
      calls += 1;
    });
    retainDerivedNode(d1, () => {});
    write(s1, 1);
    expect(readDerivedNode(d2)).toBe(0);
    expect(calls).toBe(0);
  });
});
