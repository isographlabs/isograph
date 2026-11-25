import { markDirtyQueue, markedUnneededCount, storeValue } from './marking';
import type {
  MutableDerivedNode,
  MutableSourceNode,
  SourceNode,
} from './nodes';
import { callOnChanges, refreshFlushQueue } from './pull';
import { disposeMarked, scheduleIfPending } from './cleanup';

let batchDepth = 0;

/**
 * Internal invariant: the flush queue of the open batch is created on the
 * first push, so a batch that adds nothing to the flush queue allocates
 * nothing.
 */
let batchQueue: Array<MutableDerivedNode<unknown>> | undefined;

const FLUSH_IDLE = 0;
const FLUSH_RUNNING = 1;

type FlushState = typeof FLUSH_IDLE | typeof FLUSH_RUNNING;

let flushState: FlushState = 0 satisfies typeof FLUSH_IDLE;

/**
 * Opens a batch. Each write until the matching `endWriteBatch` marks at once, and
 * the flush of the outermost batch refreshes the nodes with callbacks. A
 * nested batch joins the open batch.
 *
 * Contract: each `startWriteBatch` is matched by one `endWriteBatch`, called outside
 * computations and `onChange` callbacks. Between them, the caller writes,
 * releases, and opens nested batches, and does not call `readDerivedNode` or
 * `retainDerivedNode`.
 */
export function startWriteBatch(): void {
  batchDepth += 1;
}

/**
 * Closes a batch. The outermost `endWriteBatch` flushes: it refreshes the nodes on
 * the flush queue, then calls the `onChange` of each retain of each node whose
 * value a refresh changed.
 *
 * Contract: as `startWriteBatch`.
 */
export function endWriteBatch(): void {
  batchDepth -= 1;
  if (batchDepth === 0) {
    const queue = batchQueue;
    batchQueue = undefined;
    flushOrDisposeMarked(queue);
  }
  scheduleIfOutermost();
}

/**
 * Stores `value` and marks the dependents of `source`. Outside a batch it
 * flushes before it returns, as a batch of one write does.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  const first = storeValue(source as MutableSourceNode<TValue>, value);
  if (first === undefined) {
    return;
  }
  flushOrQueue(markDirtyQueue(currentBatchQueue(), first));
  scheduleIfOutermost();
}

export function currentBatchQueue():
  | Array<MutableDerivedNode<unknown>>
  | undefined {
  return batchDepth > 0 ? batchQueue : undefined;
}

export function flushOrQueue(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  if (batchDepth > 0) {
    batchQueue = queue;
  } else {
    flushOrDisposeMarked(queue);
  }
}

function flushOrDisposeMarked(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  if (queue !== undefined) {
    flush(queue);
  } else if (markedUnneededCount > 0) {
    disposeMarked();
  }
}

/**
 * An outermost operation is a call to `writeSource`, `endWriteBatch`,
 * `retainDerivedNode`, `releaseRetain`, or `readDerivedNode` made with no
 * batch open and no flush running, or the `endWriteBatch` that closes the
 * outermost batch.
 *
 * Internal invariant: a top-level read never runs inside a computation or a
 * batch body (its contract), so a top-level read inside an `onChange` is the
 * only nesting of operations outside a batch, and the flush state covers it.
 */
export function scheduleIfOutermost(): void {
  if (batchDepth === 0 && flushState === (0 satisfies typeof FLUSH_IDLE)) {
    scheduleIfPending();
  }
}

/**
 * `scheduleIfOutermost` for `readDerivedNode`, which runs outside batch bodies
 * (its contract), so only the flush state decides whether it is outermost.
 */
export function scheduleIfNotFlushing(): void {
  if (flushState === (0 satisfies typeof FLUSH_IDLE)) {
    scheduleIfPending();
  }
}

/**
 * Guarantee: every `onChange` runs after the flush has refreshed every node
 * on the flush queue, in the order of each node's first recompute that
 * changed its value.
 *
 * Internal invariant: an `onChange` writes nothing, so it puts no node on the
 * flush queue and no node on the marked-unneeded list.
 */
function flush(observedNodes: Array<MutableDerivedNode<unknown>>): void {
  flushState = 1 satisfies typeof FLUSH_RUNNING;
  refreshFlushQueue(observedNodes);
  if (markedUnneededCount > 0) {
    disposeMarked();
  }
  callOnChanges();
  flushState = 0 satisfies typeof FLUSH_IDLE;
}
