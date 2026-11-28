import { unlinkAll, unlinkEdge } from './edges';
import {
  clearMarkedUnneeded,
  markedUnneeded,
  markedUnneededCount,
} from './marking';
import { type CLEAN, type UNCOMPUTED, type MutableDerivedNode } from './nodes';

/**
 * Internal invariant: the cleanup queue holds one entry for each release of a
 * last retain, top-level read, recompute, or disposal that left a derived node
 * that is not `UNCOMPUTED` unneeded, also when the node already has an entry,
 * so a loop of top-level reads of a node that nothing retains adds one entry
 * per read. Each entry stays until `disposeNext` or `disposeAll` pops it, also
 * when its node became needed again.
 */
const cleanupQueue: Array<MutableDerivedNode<unknown>> = [];

const SCHEDULE_NOT_PENDING = 0;
const SCHEDULE_PENDING = 1;

type ScheduleStatus = typeof SCHEDULE_NOT_PENDING | typeof SCHEDULE_PENDING;

let scheduleStatus: ScheduleStatus = 0 satisfies typeof SCHEDULE_NOT_PENDING;

/**
 * `scheduler.schedule` is called with no arguments at the end of an outermost
 * operation after the cleanup queue went from empty to non-empty. A
 * `writeSource` to a source that has no dependents, or that no computation
 * read since the source was created or last written, returns before that
 * call. `schedule` may call `disposeAll` synchronously. The default disposes
 * during `requestIdleCallback` time where that exists, and for 5 ms per
 * `setTimeout` task elsewhere, while the cleanup queue is non-empty. Assigning
 * `scheduler.schedule` replaces the default.
 *
 * Contract: after a call, the scheduler calls `disposeNext` until it returns
 * false, or calls `disposeAll`.
 */
export const scheduler: { schedule: () => void } = {
  schedule: defaultSchedule,
};

const TIMEOUT_DISPOSAL_MS = 5;

function defaultSchedule(): void {
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(disposeWhileIdle);
  } else {
    setTimeout(disposeForTimeout, 0);
  }
}

function disposeWhileIdle(deadline: IdleDeadline): void {
  let remaining = true;
  while (remaining && deadline.timeRemaining() > 0) {
    remaining = disposeNext();
  }
  if (remaining) {
    requestIdleCallback(disposeWhileIdle);
  }
}

function disposeForTimeout(): void {
  const end = performance.now() + TIMEOUT_DISPOSAL_MS;
  let remaining = true;
  while (remaining && performance.now() < end) {
    remaining = disposeNext();
  }
  if (remaining) {
    setTimeout(disposeForTimeout, 0);
  }
}

export function addToCleanupQueue(node: MutableDerivedNode<unknown>): void {
  if (cleanupQueue.length === 0) {
    scheduleStatus = 1 satisfies typeof SCHEDULE_PENDING;
  }
  cleanupQueue.push(node);
}

export function scheduleIfPending(): void {
  if (scheduleStatus === (1 satisfies typeof SCHEDULE_PENDING)) {
    scheduleStatus = 0 satisfies typeof SCHEDULE_NOT_PENDING;
    scheduler.schedule();
  }
}

/**
 * Pops one entry off the cleanup queue and disposes its node if the node is
 * still unneeded and not `UNCOMPUTED`. Disposing a node puts each of its
 * derived dependencies that becomes unneeded on the cleanup queue. Returns
 * whether the cleanup queue is non-empty.
 *
 * Contract: called outside computations, batches, and `onChange` callbacks.
 */
export function disposeNext(): boolean {
  const node = cleanupQueue.pop();
  if (
    node !== undefined &&
    node.firstRetain === undefined &&
    node.firstDependent === undefined &&
    node.status !== (0 satisfies typeof UNCOMPUTED)
  ) {
    disposeNode(node);
  }
  if (cleanupQueue.length > 0) {
    return true;
  }
  scheduleStatus = 0 satisfies typeof SCHEDULE_NOT_PENDING;
  return false;
}

/**
 * Disposes every unneeded node on the cleanup queue, and each node that a
 * disposal leaves unneeded.
 *
 * Guarantee: after `disposeNext` returns false, and after `disposeAll`
 * returns, the cleanup queue is empty and no scheduler call is pending.
 *
 * Contract: as `disposeNext`.
 */
export function disposeAll(): void {
  while (true) {
    const node = cleanupQueue.pop();
    if (node === undefined) {
      scheduleStatus = 0 satisfies typeof SCHEDULE_NOT_PENDING;
      return;
    }
    if (
      node.firstRetain === undefined &&
      node.firstDependent === undefined &&
      node.status !== (0 satisfies typeof UNCOMPUTED)
    ) {
      disposeNode(node);
    }
  }
}

function disposeNode(node: MutableDerivedNode<unknown>): void {
  unlinkAll(node.firstSourceDependency);
  let edge = node.firstDerivedDependency;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency;
    if (
      dependency.firstDependent === undefined &&
      dependency.firstRetain === undefined
    ) {
      addToCleanupQueue(dependency);
    }
    edge = edge.nextDependency;
  }
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = 0 satisfies typeof UNCOMPUTED;
  node.value = undefined;
}

/**
 * Internal invariant: a dependency that a disposal here leaves unneeded is
 * disposed too if it is not `CLEAN`, because the same write marked it. If it
 * is `CLEAN`, it goes on the cleanup queue with its value.
 */
export function disposeMarked(): void {
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  for (let index = 0; index < markedUnneededCount; index += 1) {
    let node: MutableDerivedNode<unknown> | undefined = markedUnneeded[index];
    while (node !== undefined) {
      if (
        node.firstRetain === undefined &&
        node.firstDependent === undefined &&
        node.status !== (0 satisfies typeof UNCOMPUTED) &&
        node.status !== (3 satisfies typeof CLEAN)
      ) {
        unlinkAll(node.firstSourceDependency);
        let edge = node.firstDerivedDependency;
        while (edge !== undefined) {
          unlinkEdge(edge);
          const dependency = edge.dependency;
          if (
            dependency.firstDependent === undefined &&
            dependency.firstRetain === undefined
          ) {
            if (dependency.status === (3 satisfies typeof CLEAN)) {
              addToCleanupQueue(dependency);
            } else {
              (stack ??= []).push(dependency);
            }
          }
          edge = edge.nextDependency;
        }
        node.firstSourceDependency = undefined;
        node.lastSourceDependency = undefined;
        node.firstDerivedDependency = undefined;
        node.lastDerivedDependency = undefined;
        node.status = 0 satisfies typeof UNCOMPUTED;
        node.value = undefined;
      }
      node = stack?.pop();
    }
  }
  clearMarkedUnneeded();
}
