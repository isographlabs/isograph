import {
  currentBatchQueue,
  flushOrQueue,
  scheduleIfNotFlushing,
  scheduleIfOutermost,
} from './batch';
import { markMaybeDirtyQueue } from './marking';
import {
  type MAYBE_DIRTY,
  type DIRTY,
  type OBSERVED,
  type UNCOMPUTED,
  type UNOBSERVED,
  type DerivedNode,
  type MutableDerivedNode,
  type MutableRetain,
  type Retain,
} from './nodes';
import { addToCleanupQueue, scheduleIfPending } from './cleanup';
import { pull } from './pull';

/**
 * Brings `node` up to date, adds a retain to the head of its retain list, and
 * returns the retain. A retained node keeps its value and its edges to its
 * dependencies until its last retain is released. A retain with an
 * `onChange` gives the node a callback.
 *
 * Guarantee: the flush calls `onChange` with the node's value once for each
 * update or outermost batch that changed the node's value, after the flush has
 * refreshed every node on the flush queue. The first recompute of a node does
 * not call `onChange`.
 *
 * Contract: called outside computations, `onChange` callbacks, and write
 * batch bodies, because retaining computes the node. `onChange` does not
 * write, start or end a batch, retain, release, or dispose. It may call
 * `readDerivedNode`.
 */
export function retainDerivedNode<TValue>(
  node: DerivedNode<TValue>,
  onChange?: (value: TValue) => void,
): Retain<TValue> {
  const mutableNode = node as MutableDerivedNode<TValue>;
  // Internal invariant: retaining brings the node up to date, so it is `CLEAN`
  // when its first retain with an `onChange` gives it a callback.
  pull(mutableNode);
  const head = mutableNode.firstRetain;
  const retainFields: Omit<MutableRetain<TValue>, keyof Retain<TValue>> = {
    node: mutableNode,
    onChange,
    previous: undefined,
    next: head,
  };
  const retain = retainFields as MutableRetain<TValue>;
  if (head !== undefined) {
    head.previous = retain;
  }
  mutableNode.firstRetain = retain;
  if (onChange !== undefined) {
    observe(mutableNode);
  }
  // Internal invariant: by the contract, a retain is an outermost operation.
  scheduleIfPending();
  return retain;
}

/**
 * Releases `retain`. A release that leaves the node unneeded and not
 * `UNCOMPUTED` puts the node on the cleanup queue. The node keeps its value and
 * its edges until a disposal, or the marking of a write, reaches it.
 *
 * Contract: called once per retain, outside computations and `onChange`
 * callbacks.
 */
export function releaseRetain<TValue>(retain: Retain<TValue>): void {
  const mutableRetain = retain as MutableRetain<TValue>;
  const mutableNode = mutableRetain.node;
  const previous = mutableRetain.previous;
  const next = mutableRetain.next;
  if (previous === undefined) {
    mutableNode.firstRetain = next;
  } else {
    previous.next = next;
  }
  if (next !== undefined) {
    next.previous = previous;
  }
  if (mutableRetain.onChange !== undefined) {
    unobserveIfLast(mutableNode);
  }
  if (
    mutableNode.firstRetain === undefined &&
    mutableNode.firstDependent === undefined &&
    mutableNode.status !== (0 satisfies typeof UNCOMPUTED)
  ) {
    addToCleanupQueue(mutableNode);
  }
  scheduleIfOutermost();
}

/**
 * Top-level read of `node`: brings `node` up to date and returns its value. A
 * recompute links edges as in a tracked read. A node left unneeded and not
 * `UNCOMPUTED` goes on the cleanup queue, so the scheduler can dispose it and
 * each dependency that its disposal leaves unneeded.
 *
 * Contract: called outside computations and write batch bodies. Inside an
 * `onChange`, it is allowed.
 */
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue {
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  if (
    mutableNode.firstRetain === undefined &&
    mutableNode.firstDependent === undefined &&
    mutableNode.status !== (0 satisfies typeof UNCOMPUTED)
  ) {
    addToCleanupQueue(mutableNode);
  }
  scheduleIfNotFlushing();
  return value;
}

function observe(node: MutableDerivedNode<unknown>): void {
  node.evaluation = 1 satisfies typeof OBSERVED;
}

/**
 * Internal invariant: a node that loses its last callback while `DIRTY` or
 * `MAYBE_DIRTY` has no `CLEAN` dependents, and its dependents may be
 * `MAYBE_DIRTY` with `MARKED_BEYOND`, waiting for its refresh to put the
 * nodes with callbacks among them on the flush queue. The walk through lazy nodes
 * from the node clears that bit on them, as the dependents of a lazy node that
 * is `DIRTY` or `MAYBE_DIRTY` require, and puts those nodes with callbacks on the
 * flush queue. A node that gets a callback was brought
 * up to date first, so it is `CLEAN`.
 */
function unobserveIfLast(node: MutableDerivedNode<unknown>): void {
  if (hasOnChangeRetain(node)) {
    return;
  }
  node.evaluation = 0 satisfies typeof UNOBSERVED;
  if (
    node.status === (1 satisfies typeof DIRTY) ||
    node.status === (2 satisfies typeof MAYBE_DIRTY)
  ) {
    flushOrQueue(
      markMaybeDirtyQueue(currentBatchQueue(), node, undefined, undefined),
    );
  }
}

function hasOnChangeRetain(node: MutableDerivedNode<unknown>): boolean {
  let retain = node.firstRetain;
  while (retain !== undefined) {
    if (retain.onChange !== undefined) {
      return true;
    }
    retain = retain.next;
  }
  return false;
}
