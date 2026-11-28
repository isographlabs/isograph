/*
 * An edge connects a derived node, the dependent, to a node its computation
 * read, the dependency. Each edge is in two lists: the dependent's list of
 * dependencies, in the order its computation read them (a derived node keeps
 * one list of source dependencies and one of derived dependencies), and the
 * dependency's list of dependents.
 *
 * A recompute reuses edges by position: the n-th dependency the computation
 * reads takes the dependent's n-th edge. For example, d1's computation reads
 * s1 then s2, so d1's list of source dependencies is [e1, e2], where e1 points
 * at s1 and e2 at s2. If d1 recomputes and its computation reads:
 * - s1 then s2: e1 and e2 are kept, and nothing is allocated.
 * - s1 then s3: e2 is taken off s2's dependents and put on s3's.
 * - only s1: e2 is removed from d1's dependencies and from s2's dependents.
 *
 * A list of dependents is doubly linked through the edges, so removing e2
 * takes constant time. A list of dependencies is singly linked, because a
 * recompute only walks it from the start and a trim cuts its end. A node's
 * retain list is doubly linked the same way, through the `Retain` objects.
 *
 * Internal invariant: an edge is in exactly one list of dependencies and one
 * list of dependents. A node's lists of dependencies stay linked while it is
 * `DIRTY`. A disposed node has no edges.
 */

/*
 * Each use of a status or flag constant in src/ is written
 * `<literal> satisfies typeof <NAME>`. The type checker rejects a literal that
 * differs from the constant, and esbuild and tsdown emit the literal instead of
 * a load of a module-scope binding, which neither inlines.
 */
export const UNCOMPUTED = 0;
export const DIRTY = 1;
export const MAYBE_DIRTY = 2;
export const CLEAN = 3;

export type Status =
  | typeof UNCOMPUTED
  | typeof DIRTY
  | typeof MAYBE_DIRTY
  | typeof CLEAN;

declare const sourceBrand: unique symbol;
declare const derivedBrand: unique symbol;
declare const retainBrand: unique symbol;
declare const callingNodeBrand: unique symbol;

/**
 * The node whose computation is running. A computation receives it and passes
 * it to `readSource` and `readDerived`. It has no runtime value of its own:
 * the engine passes the derived node being computed.
 */
export type CallingNode = { readonly [callingNodeBrand]: true };

export type MutableEdge<TDependency> = {
  dependency: TDependency;
  readonly dependent: MutableDerivedNode<unknown>;
  nextDependency: MutableEdge<TDependency> | undefined;
  previousDependent: MutableEdge<TDependency> | undefined;
  nextDependent: MutableEdge<TDependency> | undefined;
};

export type NodeWithDependents<TDependency> = {
  firstDependent: MutableEdge<TDependency> | undefined;
  lastDependent: MutableEdge<TDependency> | undefined;
};

export type MutableSourceNode<TValue> = SourceNode<TValue> & {
  value: TValue;
  firstDependent: MutableEdge<MutableSourceNode<unknown>> | undefined;
  lastDependent: MutableEdge<MutableSourceNode<unknown>> | undefined;
  dependentsStatus: DependentsStatus;
};

/**
 * A derived node is the `CallingNode` its own computation receives.
 *
 * Internal invariant: `computation` is a method, which keeps
 * `MutableDerivedNode<TValue>` assignable to `MutableDerivedNode<unknown>`.
 */
export type MutableDerivedNode<TValue> = DerivedNode<TValue> &
  CallingNode & {
    value: TValue;
    firstRetain: MutableRetain<TValue> | undefined;
    status: Status;
    evaluation: EvaluationState;
    // The bits `ON_CHANGE_QUEUED`, `MARKED_BEYOND`, and `BEYOND_DONE`.
    flags: number;
    firstSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
    lastSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
    firstDerivedDependency:
      | MutableEdge<MutableDerivedNode<unknown>>
      | undefined;
    lastDerivedDependency: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    firstDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    lastDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
    computation(callingNode: CallingNode, previous: TValue | undefined): TValue;
  };

/**
 * Internal invariant: `onChange` has the type of a method, which keeps
 * `MutableRetain<TValue>` assignable to `MutableRetain<unknown>`.
 */
export type MutableRetain<TValue> = Retain<TValue> & {
  readonly node: MutableDerivedNode<TValue>;
  readonly onChange: OnChange<TValue> | undefined;
  previous: MutableRetain<TValue> | undefined;
  next: MutableRetain<TValue> | undefined;
};

type OnChange<TValue> = { method(value: TValue): void }['method'];

export const DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY = 0;
export const DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY = 1;

export type DependentsStatus =
  | typeof DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY
  | typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;

/**
 * Internal invariant: `dependentsStatus` is `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`
 * from the source's creation or from a write until the next `readSource`.
 * A node leaves `DIRTY` only by a recompute, and a node joins the source's list
 * of dependents only through `readSource`.
 */
export type SourceNode<TValue> = { readonly [sourceBrand]: TValue };

export const UNOBSERVED = 0;
export const OBSERVED = 1;

/**
 * Internal invariant: a node is `OBSERVED` exactly while it has a retain with
 * an `onChange`. Marking puts a node with a callback that it takes out of `CLEAN` on
 * the flush queue, unless it reached the node only beyond another node
 * with a callback. Every other node is lazy.
 */
export type EvaluationState = typeof UNOBSERVED | typeof OBSERVED;

/**
 * A bit of `flags`: the node is on the list of changed nodes of the running
 * flush, so a second changed recompute in that flush does not add it again.
 *
 * Internal invariant: no node has the bit outside a flush, because only the
 * refreshes of a flush add to that list, and the end of the flush clears it.
 */
export const ON_CHANGE_QUEUED = 1;
/**
 * A bit of `flags`, read only while the node is `MAYBE_DIRTY`: marking reached
 * the node only beyond a node with a callback that it put on the flush queue, so a
 * node with a callback above it may be `MAYBE_DIRTY` and on no flush queue.
 */
export const MARKED_BEYOND = 2;
/**
 * A bit of `flags`: no dependent of the node is `CLEAN`, so the walk that sets
 * `MARKED_BEYOND` would mark nothing past the node and skips it.
 *
 * Internal invariant: that walk sets the bit when it walks the node. A
 * dependent becomes `CLEAN` only by a computation, which reads the node through
 * `readDerived`, or by `verify`, which visits the node; a dependent joins the
 * node's list of dependents only through `readDerived`. `readDerived` and
 * `verify` clear the bit, and `markFromChange` of the node clears it.
 */
export const BEYOND_DONE = 4;

/**
 * Internal invariants of a derived node whose computation is not running:
 * - `UNCOMPUTED`: `value` is `undefined`, both dependency lists are empty, and
 *   the node has no dependents.
 * - `DIRTY`: `value` is the value of the last recompute, and no dependent is
 *   `CLEAN`. A lazy node has `MAYBE_DIRTY` or `DIRTY` dependents. A node
 *   with a callback is on the flush queue of the open batch or of the running flush.
 *   Only inside `markDirtyQueue`, or inside `markMaybeDirtyQueue` for a
 *   release, can a node with a callback that the marking has just put on the flush
 *   queue have `CLEAN` dependents, until the marking's walk that sets
 *   `MARKED_BEYOND` runs.
 * - `MAYBE_DIRTY`: as `DIRTY`. A node with a callback that marking reached only
 *   beyond another node with a callback is also on no flush queue: the change of the
 *   node below it puts it there, and if that node keeps its value, the node's
 *   value is current and a read verifies it.
 * - `CLEAN`: `value` is up to date, and every derived dependency is `CLEAN`.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's list of dependents. A node that is not `UNCOMPUTED` and is
 * unneeded is on the cleanup queue, or on the marked-unneeded list of the
 * running write, except a node that `readDerived` has brought up to date but
 * has not yet linked to the calling node by an edge.
 */
export type DerivedNode<TValue> = { readonly [derivedBrand]: TValue };

export type Retain<TValue> = { readonly [retainBrand]: TValue };

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  const source: Omit<MutableSourceNode<TValue>, keyof SourceNode<TValue>> = {
    value,
    firstDependent: undefined,
    lastDependent: undefined,
    dependentsStatus: 1 satisfies typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  };
  return source as MutableSourceNode<TValue>;
}

/**
 * Returns an `UNCOMPUTED` derived node.
 *
 * Contract: `computation` is synchronous. `computation` reads other nodes
 * only through `readSource` and `readDerived`, called with the `callingNode`
 * that `computation` receives, and does not keep that `callingNode` for use
 * after it returns. `computation` does not write, start or end a batch, retain,
 * release, dispose, or call `readDerivedNode`.
 */
export function createDerivedNode<TValue>(
  computation: (
    callingNode: CallingNode,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  const node: Omit<
    MutableDerivedNode<TValue>,
    keyof DerivedNode<TValue> | keyof CallingNode
  > = {
    // An `UNCOMPUTED` node has no value. Every read of `value` follows a
    // status check that excludes `UNCOMPUTED`, so `value` is read only after a
    // computation stored a `TValue` in it. The type cannot make `value` depend
    // on `status`, and typing it `TValue | undefined` would put a cast at
    // every read.
    value: undefined as TValue,
    firstRetain: undefined,
    flags: 0,
    status: 0 satisfies typeof UNCOMPUTED,
    evaluation: 0 satisfies typeof UNOBSERVED,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
  return node as MutableDerivedNode<TValue>;
}
