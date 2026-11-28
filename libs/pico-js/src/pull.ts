import { markFromChange, markMaybeDirtyDependentsDirty } from './marking';
import {
  type BEYOND_DONE,
  type MAYBE_DIRTY,
  type CLEAN,
  type DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY,
  type DIRTY,
  type OBSERVED,
  type ON_CHANGE_QUEUED,
  type UNCOMPUTED,
  type CallingNode,
  type DerivedNode,
  type MutableDerivedNode,
  type MutableEdge,
  type MutableSourceNode,
  type SourceNode,
} from './nodes';
import { addToCleanupQueue } from './cleanup';
import { linkEdge, unlinkAll, unlinkEdge } from './edges';

/**
 * Returns the value of `source` and records it as a dependency of
 * `callingNode`.
 *
 * Contract: called only while the computation of `callingNode` runs.
 */
export function readSource<TValue>(
  callingNode: CallingNode,
  source: SourceNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableSource = source as MutableSourceNode<TValue>;
  // A write records that every node in the source's dependents list is
  // `DIRTY`, so the next write skips marking. The calling node is `CLEAN` in
  // that list once this computation finishes, so the read clears the record.
  mutableSource.dependentsStatus =
    0 satisfies typeof DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY;
  const last = mutableCallingNode.lastSourceDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== mutableSource) {
      unlinkEdge(next);
      next.dependency = mutableSource;
      next.nextDependent = undefined;
      linkEdge(next);
    }
    mutableCallingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== mutableSource) {
    const edge: MutableEdge<MutableSourceNode<unknown>> = {
      dependency: mutableSource,
      dependent: mutableCallingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge);
    if (last === undefined) {
      mutableCallingNode.firstSourceDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    mutableCallingNode.lastSourceDependency = edge;
  }
  return mutableSource.value;
}

/**
 * Brings `node` up to date, returns its value, and records it as a dependency
 * of `callingNode`.
 *
 * Contract: called only while the computation of `callingNode` runs.
 */
export function readDerived<TValue>(
  callingNode: CallingNode,
  node: DerivedNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  // The calling node becomes `CLEAN` when its computation returns, and may be
  // a new dependent of `node`.
  const flags = mutableNode.flags;
  if ((flags & (4 satisfies typeof BEYOND_DONE)) !== 0) {
    mutableNode.flags = flags & ~(4 satisfies typeof BEYOND_DONE);
  }
  const last = mutableCallingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== mutableNode) {
      const previousDependency = next.dependency;
      unlinkEdge(next);
      next.dependency = mutableNode;
      next.nextDependent = undefined;
      linkEdge(next);
      if (
        previousDependency.firstDependent === undefined &&
        previousDependency.firstRetain === undefined
      ) {
        addToCleanupQueue(previousDependency);
      }
    }
    mutableCallingNode.lastDerivedDependency = next;
  } else if (last === undefined || last.dependency !== mutableNode) {
    const edge: MutableEdge<MutableDerivedNode<unknown>> = {
      dependency: mutableNode,
      dependent: mutableCallingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge);
    if (last === undefined) {
      mutableCallingNode.firstDerivedDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    mutableCallingNode.lastDerivedDependency = edge;
  }
  return value;
}

/**
 * Guarantee: a pull of a `DIRTY` or `MAYBE_DIRTY` node with a callback during the
 * refreshes of the running flush refreshes it at once, so the reader sees its
 * new value, and its `onChange` runs in the flush after every node on the
 * flush queue is refreshed.
 */
export function pull<TValue>(node: MutableDerivedNode<TValue>): TValue {
  switch (node.status) {
    case 3 satisfies typeof CLEAN:
      return node.value;
    case 1 satisfies typeof DIRTY:
      return node.evaluation === (1 satisfies typeof OBSERVED)
        ? pullObserved(node)
        : recompute(node);
    case 2 satisfies typeof MAYBE_DIRTY:
      return node.evaluation === (1 satisfies typeof OBSERVED)
        ? pullObserved(node)
        : pullMaybeDirty(node);
    case 0 satisfies typeof UNCOMPUTED:
      return computeFirst(node);
  }
}

function pullMaybeDirty<TValue>(node: MutableDerivedNode<TValue>): TValue {
  return verify(node) === (1 satisfies typeof DIRTY)
    ? recompute(node)
    : node.value;
}

/**
 * Internal invariant: no pull runs in a batch body, because a computation runs
 * only in a flush or in a top-level read or retain, and neither of those runs
 * in a batch body (their contracts). Outside the refreshes of a flush, a
 * node with a callback that is not `CLEAN` is on no flush queue. Marking left it
 * `MAYBE_DIRTY` with `MARKED_BEYOND` above a node with a callback that kept its
 * value. A pull brings it up to date as a lazy node.
 */
function pullObserved<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const queue = refreshQueue;
  if (queue !== undefined) {
    refreshQueued(queue, node as unknown as MutableDerivedNode<unknown>);
    return node.value;
  }
  return node.status === (1 satisfies typeof DIRTY)
    ? recompute(node)
    : pullMaybeDirty(node);
}

function verify(
  root: MutableDerivedNode<unknown>,
): typeof DIRTY | typeof CLEAN {
  let verifyStack: Array<MutableEdge<MutableDerivedNode<unknown>>> | undefined;
  let node: MutableDerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  while (true) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      // `node` may become `CLEAN` with this edge kept.
      const flags = dependency.flags;
      if ((flags & (4 satisfies typeof BEYOND_DONE)) !== 0) {
        dependency.flags = flags & ~(4 satisfies typeof BEYOND_DONE);
      }
      if (
        dependency.status === (2 satisfies typeof MAYBE_DIRTY) &&
        dependency.evaluation !== (1 satisfies typeof OBSERVED)
      ) {
        // Internal invariant: a dependency with one dependent has one edge back
        // to that dependent, so only an edge into a dependency with several
        // dependents is pushed onto the verify stack.
        if (
          edge.previousDependent !== undefined ||
          edge.nextDependent !== undefined
        ) {
          (verifyStack ??= []).push(edge);
        }
        node = dependency;
        edge = dependency.firstDerivedDependency;
        continue;
      }
      pull(dependency);
      if (node.status !== (1 satisfies typeof DIRTY)) {
        edge = edge.nextDependency;
        continue;
      }
      if (node === root) {
        return 1 satisfies typeof DIRTY;
      }
      recompute(node);
    } else {
      // Internal invariant: a write to a source dependency of a `MAYBE_DIRTY` node
      // makes the node `DIRTY`, so a `MAYBE_DIRTY` node whose derived dependencies are
      // all unchanged becomes `CLEAN`.
      node.status = 3 satisfies typeof CLEAN;
    }
    while (true) {
      if (node === root) {
        return 3 satisfies typeof CLEAN;
      }
      const only = node.firstDependent!;
      edge = only.nextDependent === undefined ? only : verifyStack!.pop()!;
      node = edge.dependent;
      if (node.status !== (1 satisfies typeof DIRTY)) {
        break;
      }
      if (node === root) {
        return 1 satisfies typeof DIRTY;
      }
      recompute(node);
    }
    edge = edge.nextDependency;
  }
}

/**
 * The flush queue that a pull of a node with a callback that is not `CLEAN`
 * refreshes the node into: the running flush's during its refreshes.
 * `undefined` otherwise.
 */
let refreshQueue: Array<MutableDerivedNode<unknown>> | undefined;

/**
 * Internal invariant: no flush runs inside another, because no `onChange`
 * writes or ends a batch. One module-level array therefore serves every
 * flush. The flush clears each entry of the array before it calls the
 * `onChange` callbacks of the entry's node, and `changedCount` is 0 outside a
 * flush. The list has no duplicates, because a node on it has the bit
 * `ON_CHANGE_QUEUED`.
 */
const changedNodes: Array<MutableDerivedNode<unknown> | undefined> = [];
let changedCount = 0;

/**
 * Refreshes each node on the flush queue, and records each node whose refresh
 * changed its value for its `onChange`.
 *
 * Internal invariant: marking puts a node on the flush queue each time it
 * takes the node out of `CLEAN`, and a pull refreshes a node on the flush
 * queue at once. A node is on the flush queue twice when a recompute made it
 * `CLEAN` after its first entry and a later marking put it there again, and
 * `refreshObserved` skips the later entry because the node is `CLEAN`.
 */
export function refreshFlushQueue(
  observedNodes: Array<MutableDerivedNode<unknown>>,
): void {
  refreshQueue = observedNodes;
  // An index walks the array, because a refresh appends to it during the
  // walk.
  let index = 0;
  while (true) {
    const node = observedNodes[index];
    if (node === undefined) {
      break;
    }
    index += 1;
    refreshQueued(observedNodes, node);
  }
  refreshQueue = undefined;
}

/** Refreshes `node` and records it for its `onChange` if its value changed. */
function refreshQueued(
  observedNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): void {
  const result = refreshObserved(observedNodes, node);
  if (
    result === (0 satisfies typeof CHANGED) &&
    (node.flags & (1 satisfies typeof ON_CHANGE_QUEUED)) === 0
  ) {
    node.flags |= 1 satisfies typeof ON_CHANGE_QUEUED;
    changedNodes[changedCount] = node;
    changedCount += 1;
  }
}

/**
 * Calls the `onChange` of each retain of each node whose refresh changed its
 * value, in the order of each node's first refresh that changed it.
 */
export function callOnChanges(): void {
  for (let index = 0; index < changedCount; index += 1) {
    const node = changedNodes[index]!;
    changedNodes[index] = undefined;
    node.flags &= ~(1 satisfies typeof ON_CHANGE_QUEUED);
    // Contract: no computation or `onChange` retains or releases, so the
    // retain list is the one the node had when the refresh changed its value.
    const value = node.value;
    let retain = node.firstRetain;
    while (retain !== undefined) {
      if (retain.onChange !== undefined) {
        retain.onChange(value);
      }
      retain = retain.next;
    }
  }
  changedCount = 0;
}

const CHANGED = 0;
const UNCHANGED = 1;

type RefreshResult = typeof CHANGED | typeof UNCHANGED;

/**
 * Internal invariant: a node that lost its last callback
 * after marking put it on the flush queue is skipped by the flush. The node
 * stays `DIRTY` or `MAYBE_DIRTY`, and the drop marked its dependents, so a
 * tracked read or a top-level read recomputes the node as a lazy node.
 */
function refreshObserved(
  observedNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  if (node.evaluation !== (1 satisfies typeof OBSERVED)) {
    return 1 satisfies typeof UNCHANGED;
  }
  switch (node.status) {
    case 1 satisfies typeof DIRTY:
      return recomputeObserved(observedNodes, node);
    case 2 satisfies typeof MAYBE_DIRTY:
      return verify(node) === (1 satisfies typeof DIRTY)
        ? recomputeObserved(observedNodes, node)
        : (1 satisfies typeof UNCHANGED);
    // Internal invariant: a `CLEAN` node on the flush queue was recomputed after
    // marking put it there, by a pull or by an earlier entry of the same node.
    // A node that reaches this switch has a callback, so it is retained and never
    // `UNCOMPUTED`.
    case 3 satisfies typeof CLEAN:
    case 0 satisfies typeof UNCOMPUTED:
      return 1 satisfies typeof UNCHANGED;
  }
}

/**
 * Internal invariant: the dependents of a node with a callback on the flush queue
 * are not `CLEAN`, and marking left some of them `MAYBE_DIRTY` with
 * `MARKED_BEYOND`, so a changed value marks them through `markFromChange`,
 * which clears that bit on them and puts each node with a callback it marks on the
 * flush queue that the flush is walking.
 */
function recomputeObserved(
  observedNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  const previous = node.value;
  const value = rerun(node, previous);
  if (value === previous) {
    return 1 satisfies typeof UNCHANGED;
  }
  node.value = value;
  if (node.firstDependent !== undefined) {
    markFromChange(observedNodes, node);
  }
  return 0 satisfies typeof CHANGED;
}

/**
 * Internal invariant: an `UNCOMPUTED` node has empty dependency lists and both
 * cursors `undefined`, and only the reads of its own computation append to its
 * lists, so each cursor ends at the tail of its list and `trim` would unlink
 * nothing.
 */
function computeFirst<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  node.status = 3 satisfies typeof CLEAN;
  node.value = value;
  return value;
}

function recompute<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const previous = node.value;
  const value = rerun(node, previous);
  if (value === previous) {
    return previous;
  }
  node.value = value;
  // Internal invariant: the dependents of a lazy node, and of a node with a callback
  // on no flush queue, are already `MAYBE_DIRTY` or `DIRTY`. A `MAYBE_DIRTY` dependent
  // now has a dependency whose value changed, so markMaybeDirtyDependentsDirty
  // makes that dependent `DIRTY`.
  markMaybeDirtyDependentsDirty(node.firstDependent);
  return value;
}

function rerun<TValue>(
  node: MutableDerivedNode<TValue>,
  previous: TValue,
): TValue {
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = 3 satisfies typeof CLEAN;
  return value;
}

function trim(node: MutableDerivedNode<unknown>): void {
  const lastSource = node.lastSourceDependency;
  if (lastSource === undefined) {
    unlinkAll(node.firstSourceDependency);
    node.firstSourceDependency = undefined;
  } else if (lastSource.nextDependency !== undefined) {
    unlinkAll(lastSource.nextDependency);
    lastSource.nextDependency = undefined;
  }
  const lastDerived = node.lastDerivedDependency;
  if (lastDerived === undefined) {
    const first = node.firstDerivedDependency;
    if (first !== undefined) {
      unlinkDropped(first);
      node.firstDerivedDependency = undefined;
    }
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkDropped(lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
  }
}

function unlinkDropped(first: MutableEdge<MutableDerivedNode<unknown>>): void {
  let edge: MutableEdge<MutableDerivedNode<unknown>> | undefined = first;
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
}
