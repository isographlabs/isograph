// Node has no requestIdleCallback, so the default scheduler disposes in setTimeout tasks.
import { afterEach, expect, test, vi } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  disposeNext,
  readDerived,
  readDerivedNode,
  readSource,
  releaseRetain,
  retainDerivedNode,
  type DerivedNode,
  type Retain,
} from '../src/index';
import { CLEAN, UNCOMPUTED } from '../src/nodes';
import { statusOf } from './status';
import { at } from './at';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  disposeAll();
});

function nextMacrotask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

test('a release schedules a disposal that disposes the node after the current task, unless the node is retained again first', async () => {
  const s1 = createSourceNode(1);
  const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
  const r1 = retainDerivedNode(d1);
  readDerivedNode(d1);
  releaseRetain(r1);
  expect(statusOf(d1)).toBe(CLEAN);
  await nextMacrotask();
  expect(statusOf(d1)).toBe(UNCOMPUTED);
  expect(disposeNext()).toBe(false);
  const r2 = retainDerivedNode(d1);
  readDerivedNode(d1);
  releaseRetain(r2);
  retainDerivedNode(d1);
  await nextMacrotask();
  expect(statusOf(d1)).toBe(CLEAN);
});

test('the default scheduler disposes for a few milliseconds per task and reschedules while entries remain', async () => {
  const s1 = createSourceNode(1);
  const dependencies = Array.from({ length: 2500 }, () =>
    createDerivedNode((callingNode) => readSource(callingNode, s1)),
  );
  const d1 = createDerivedNode((callingNode) =>
    dependencies.reduce((sum, node) => sum + readDerived(callingNode, node), 0),
  );
  const r1 = retainDerivedNode(d1);
  readDerivedNode(d1);
  releaseRetain(r1);
  await nextMacrotask();
  expect(statusOf(d1)).toBe(UNCOMPUTED);
  expect(dependencies.some((node) => statusOf(node) === UNCOMPUTED)).toBe(true);
  await nextMacrotask();
  await nextMacrotask();
  await nextMacrotask();
  expect(dependencies.every((node) => statusOf(node) === UNCOMPUTED)).toBe(
    true,
  );
  expect(disposeNext()).toBe(false);
});

/** A release of `retain` puts `dependent` on the cleanup queue, and its disposal puts each node of `dependencies` on the cleanup queue. */
function retainFanIn(count: number): {
  readonly retain: Retain<number>;
  readonly dependent: DerivedNode<number>;
  readonly dependencies: ReadonlyArray<DerivedNode<number>>;
} {
  const source = createSourceNode(1);
  const dependencies = Array.from({ length: count }, () =>
    createDerivedNode((callingNode) => readSource(callingNode, source)),
  );
  const dependent = createDerivedNode((callingNode) =>
    dependencies.reduce((sum, node) => sum + readDerived(callingNode, node), 0),
  );
  return { retain: retainDerivedNode(dependent), dependent, dependencies };
}

test('where requestIdleCallback exists, the default scheduler disposes while the deadline has time and requests another idle period while entries remain', () => {
  const callbacks: Array<(deadline: IdleDeadline) => void> = [];
  vi.stubGlobal(
    'requestIdleCallback',
    (callback: (deadline: IdleDeadline) => void) => {
      callbacks.push(callback);
      return callbacks.length;
    },
  );
  const { retain, dependent, dependencies } = retainFanIn(2);
  releaseRetain(retain);
  expect(callbacks.length).toBe(1);
  let budget = 2;
  const deadline: IdleDeadline = {
    didTimeout: false,
    timeRemaining: () => {
      budget -= 1;
      return budget >= 0 ? 1 : 0;
    },
  };
  at(callbacks, 0)(deadline);
  expect([
    statusOf(dependent),
    statusOf(at(dependencies, 1)),
    statusOf(at(dependencies, 0)),
  ]).toEqual([UNCOMPUTED, UNCOMPUTED, CLEAN]);
  expect(callbacks.length).toBe(2);
  budget = 1;
  at(callbacks, 1)(deadline);
  expect(statusOf(at(dependencies, 0))).toBe(UNCOMPUTED);
  expect(callbacks.length).toBe(2);
});

test('the default setTimeout task disposes until its time runs out and schedules another task while entries remain', async () => {
  let now = 0;
  vi.spyOn(performance, 'now').mockImplementation(() => {
    now += 3;
    return now;
  });
  const { retain, dependent, dependencies } = retainFanIn(3);
  releaseRetain(retain);
  const statuses = (): ReadonlyArray<number> =>
    [dependent, ...dependencies].map((node) => statusOf(node));
  await nextMacrotask();
  expect(statuses()).toEqual([UNCOMPUTED, CLEAN, CLEAN, CLEAN]);
  await nextMacrotask();
  expect(statuses()).toEqual([UNCOMPUTED, CLEAN, CLEAN, UNCOMPUTED]);
  await nextMacrotask();
  await nextMacrotask();
  expect(statuses()).toEqual([UNCOMPUTED, UNCOMPUTED, UNCOMPUTED, UNCOMPUTED]);
});
