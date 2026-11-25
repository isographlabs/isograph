import type { MutableEdge, NodeWithDependents } from './nodes';

/** Contract: `edge.nextDependent` is `undefined`. */
export function linkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: MutableEdge<TDependency>,
): void {
  const dependency = edge.dependency;
  const tail = dependency.lastDependent;
  edge.previousDependent = tail;
  if (tail === undefined) {
    dependency.firstDependent = edge;
  } else {
    tail.nextDependent = edge;
  }
  dependency.lastDependent = edge;
}

/**
 * Internal invariant: the edge keeps its own links, so `unlinkAll` can follow
 * its `nextDependency` after the unlink. A reuse of the edge clears its
 * `nextDependent` before `linkEdge`, which overwrites its `previousDependent`.
 */
export function unlinkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: MutableEdge<TDependency>,
): void {
  const dependency = edge.dependency;
  const previous = edge.previousDependent;
  const next = edge.nextDependent;
  if (previous === undefined) {
    dependency.firstDependent = next;
  } else {
    previous.nextDependent = next;
  }
  if (next === undefined) {
    dependency.lastDependent = previous;
  } else {
    next.previousDependent = previous;
  }
}

export function unlinkAll<TDependency extends NodeWithDependents<TDependency>>(
  first: MutableEdge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    edge = edge.nextDependency;
  }
}
