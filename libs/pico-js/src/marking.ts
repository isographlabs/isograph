import {
  type BEYOND_DONE,
  type MAYBE_DIRTY,
  type CLEAN,
  type DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  type DIRTY,
  type MARKED_BEYOND,
  type OBSERVED,
  type UNCOMPUTED,
  type MutableDerivedNode,
  type MutableEdge,
  type MutableSourceNode,
} from './nodes';

/**
 * Internal invariant: the first `markedUnneededCount` entries of
 * `markedUnneeded` are the nodes that marking took out of `CLEAN` while they
 * were unneeded, and every later entry is `undefined`. `markedUnneededCount`
 * is 0 when an outermost operation returns.
 */
export const markedUnneeded: Array<MutableDerivedNode<unknown> | undefined> =
  [];
export let markedUnneededCount = 0;

function addMarkedUnneeded(node: MutableDerivedNode<unknown>): void {
  markedUnneeded[markedUnneededCount] = node;
  markedUnneededCount += 1;
}

/**
 * Sets each entry in use to `undefined` and the count to 0, so the array keeps
 * its length and its capacity.
 */
export function clearMarkedUnneeded(): void {
  for (let index = 0; index < markedUnneededCount; index += 1) {
    markedUnneeded[index] = undefined;
  }
  markedUnneededCount = 0;
}

export function storeValue<TValue>(
  source: MutableSourceNode<TValue>,
  value: TValue,
): MutableEdge<MutableSourceNode<unknown>> | undefined {
  source.value = value;
  if (
    source.dependentsStatus ===
    (1 satisfies typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY)
  ) {
    return undefined;
  }
  source.dependentsStatus = 1 satisfies typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;
  return source.firstDependent;
}

/**
 * Marks a write's readers and every node above them. A `CLEAN` reader becomes
 * `DIRTY`, and a `CLEAN` node above becomes `MAYBE_DIRTY`. A node with a callback that this
 * takes out of `CLEAN` goes on the flush queue, unless marking reached it only
 * beyond another node with a callback: such a node is `MAYBE_DIRTY` with
 * `MARKED_BEYOND`, the change of the node with a callback below it puts it on the
 * flush queue, and if that node keeps its value, a read verifies it.
 */
export function markDirtyQueue<TDependency>(
  initialQueue: Array<MutableDerivedNode<unknown>> | undefined,
  first: MutableEdge<TDependency>,
): Array<MutableDerivedNode<unknown>> | undefined {
  let queue = initialQueue;
  // `held` is the latest marked lazy node with dependents, and the stack holds
  // the earlier ones, so a write that reaches one allocates no array.
  // `beyond` holds the nodes with callbacks that have dependents and that this
  // marking put on the flush queue.
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let held: MutableDerivedNode<unknown> | undefined;
  let beyond: Array<MutableDerivedNode<unknown>> | undefined;
  let edge: MutableEdge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3 satisfies typeof CLEAN:
        node.status = 1 satisfies typeof DIRTY;
        if (node.evaluation === (1 satisfies typeof OBSERVED)) {
          queue = queueObservedNode(queue, node);
          if (node.firstDependent !== undefined) {
            (beyond ??= []).push(node);
          }
        } else if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        } else if (node.firstRetain === undefined) {
          addMarkedUnneeded(node);
        }
        break;
      // Internal invariant: a `MAYBE_DIRTY` node marked its dependents when it
      // left `CLEAN`, or has a callback and went on the flush queue then. One with
      // `MARKED_BEYOND` loses the bit and is marked again by the walk through
      // lazy nodes. It goes on the flush queue if it has a callback, and the walk
      // goes on through it if it is lazy.
      case 2 satisfies typeof MAYBE_DIRTY:
        node.status = 1 satisfies typeof DIRTY;
        if ((node.flags & (2 satisfies typeof MARKED_BEYOND)) !== 0) {
          node.flags &= ~(2 satisfies typeof MARKED_BEYOND);
          if (node.evaluation === (1 satisfies typeof OBSERVED)) {
            queue = queueObservedNode(queue, node);
          } else if (node.firstDependent !== undefined) {
            if (held !== undefined) {
              (stack ??= []).push(held);
            }
            held = node;
          }
        }
        break;
      // Internal invariant: a `DIRTY` node was marked earlier. An `UNCOMPUTED`
      // node has no edge to a dependency, so no list of dependents has it.
      case 1 satisfies typeof DIRTY:
      case 0 satisfies typeof UNCOMPUTED:
        break;
    }
  }
  return held === undefined && beyond === undefined
    ? queue
    : markMaybeDirtyQueue(queue, held, stack, beyond);
}

/**
 * Marks the `CLEAN` dependents of `held` and of each node on the stack
 * `MAYBE_DIRTY`, putting each one with a callback on the flush queue, and the `CLEAN`
 * dependents of each node on `beyond` `MAYBE_DIRTY` with `MARKED_BEYOND`, putting
 * none on the flush queue. It continues through every node it marks. The
 * walk over `beyond` skips a node with `BEYOND_DONE` and sets the bit on each
 * node it walks.
 */
export function markMaybeDirtyQueue(
  initialQueue: Array<MutableDerivedNode<unknown>> | undefined,
  held: MutableDerivedNode<unknown> | undefined,
  initialStack: Array<MutableDerivedNode<unknown>> | undefined,
  initialBeyond: Array<MutableDerivedNode<unknown>> | undefined,
): Array<MutableDerivedNode<unknown>> | undefined {
  let queue = initialQueue;
  let stack = initialStack;
  let beyond = initialBeyond;
  let marked: MutableDerivedNode<unknown> | undefined = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3 satisfies typeof CLEAN:
          dependent.status = 2 satisfies typeof MAYBE_DIRTY;
          dependent.flags &= ~(2 satisfies typeof MARKED_BEYOND);
          if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
            queue = queueObservedNode(queue, dependent);
            if (dependent.firstDependent !== undefined) {
              (beyond ??= []).push(dependent);
            }
          } else if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            addMarkedUnneeded(dependent);
          }
          break;
        case 2 satisfies typeof MAYBE_DIRTY:
          if ((dependent.flags & (2 satisfies typeof MARKED_BEYOND)) !== 0) {
            dependent.flags &= ~(2 satisfies typeof MARKED_BEYOND);
            if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
              queue = queueObservedNode(queue, dependent);
            } else if (dependent.firstDependent !== undefined) {
              if (dependentEdge === undefined) {
                dependentEdge = dependent.firstDependent;
              } else {
                (stack ??= []).push(dependent);
              }
            }
          }
          break;
        // Internal invariant: a lazy node that is not `CLEAN` has no `CLEAN`
        // dependents.
        case 1 satisfies typeof DIRTY:
        case 0 satisfies typeof UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
  if (beyond === undefined) {
    return queue;
  }
  marked = beyond.pop();
  while (marked !== undefined) {
    if ((marked.flags & (4 satisfies typeof BEYOND_DONE)) !== 0) {
      marked = beyond.pop();
      continue;
    }
    marked.flags |= 4 satisfies typeof BEYOND_DONE;
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      if (dependent.status === (3 satisfies typeof CLEAN)) {
        dependent.status = 2 satisfies typeof MAYBE_DIRTY;
        dependent.flags |= 2 satisfies typeof MARKED_BEYOND;
        if (dependent.firstDependent !== undefined) {
          beyond.push(dependent);
        } else if (dependent.firstRetain === undefined) {
          addMarkedUnneeded(dependent);
        }
      }
    }
    marked = beyond.pop();
  }
  return queue;
}

/**
 * Marks the dependents of `node`, a node with a callback whose refresh changed
 * its value. A `CLEAN` direct dependent becomes `DIRTY`, and each `CLEAN` node that a
 * walk through lazy nodes reaches beyond them becomes `MAYBE_DIRTY`. A
 * `MAYBE_DIRTY` direct dependent becomes `DIRTY`. A `MAYBE_DIRTY` node with
 * `MARKED_BEYOND` loses the bit, and the walk treats it as it treats a node it
 * marks. Each node with a callback that this marks goes on `observedNodes`, and the
 * walk does not continue through it: its own refresh marks what is beyond it.
 * A node that is `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, has its
 * dependents with callbacks on a flush queue already, so the walk stops at it.
 */
export function markFromChange(
  observedNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): void {
  node.flags &= ~(4 satisfies typeof BEYOND_DONE);
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let held: MutableDerivedNode<unknown> | undefined;
  let edge = node.firstDependent;
  while (edge !== undefined) {
    const dependent = edge.dependent;
    edge = edge.nextDependent;
    switch (dependent.status) {
      case 3 satisfies typeof CLEAN:
        dependent.status = 1 satisfies typeof DIRTY;
        if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
          observedNodes.push(dependent);
        } else if (dependent.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = dependent;
        } else if (dependent.firstRetain === undefined) {
          addMarkedUnneeded(dependent);
        }
        break;
      case 2 satisfies typeof MAYBE_DIRTY:
        dependent.status = 1 satisfies typeof DIRTY;
        if ((dependent.flags & (2 satisfies typeof MARKED_BEYOND)) !== 0) {
          dependent.flags &= ~(2 satisfies typeof MARKED_BEYOND);
          if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
            observedNodes.push(dependent);
          } else if (dependent.firstDependent !== undefined) {
            if (held !== undefined) {
              (stack ??= []).push(held);
            }
            held = dependent;
          }
        }
        break;
      case 1 satisfies typeof DIRTY:
      case 0 satisfies typeof UNCOMPUTED:
        break;
    }
  }
  let marked = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3 satisfies typeof CLEAN:
          dependent.status = 2 satisfies typeof MAYBE_DIRTY;
          dependent.flags &= ~(2 satisfies typeof MARKED_BEYOND);
          if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
            observedNodes.push(dependent);
          } else if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            addMarkedUnneeded(dependent);
          }
          break;
        case 2 satisfies typeof MAYBE_DIRTY:
          if ((dependent.flags & (2 satisfies typeof MARKED_BEYOND)) !== 0) {
            dependent.flags &= ~(2 satisfies typeof MARKED_BEYOND);
            if (dependent.evaluation === (1 satisfies typeof OBSERVED)) {
              observedNodes.push(dependent);
            } else if (dependent.firstDependent !== undefined) {
              if (dependentEdge === undefined) {
                dependentEdge = dependent.firstDependent;
              } else {
                (stack ??= []).push(dependent);
              }
            }
          }
          break;
        case 1 satisfies typeof DIRTY:
        case 0 satisfies typeof UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
}

function queueObservedNode(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
  node: MutableDerivedNode<unknown>,
): Array<MutableDerivedNode<unknown>> {
  if (queue === undefined) {
    return [node];
  }
  queue.push(node);
  return queue;
}

export function markMaybeDirtyDependentsDirty(
  first: MutableEdge<MutableDerivedNode<unknown>> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 2 satisfies typeof MAYBE_DIRTY:
        node.status = 1 satisfies typeof DIRTY;
        break;
      case 1 satisfies typeof DIRTY:
      case 3 satisfies typeof CLEAN:
      case 0 satisfies typeof UNCOMPUTED:
        break;
    }
  }
}
