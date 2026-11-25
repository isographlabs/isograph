import { test, vi } from 'vitest';
import { at } from '../test/at';
import { internal } from '../test/internal';
import {
  type CallingNode,
  type DerivedNode,
  type Retain,
  type SourceNode,
} from '../src/index';
import {
  MAYBE_DIRTY,
  CLEAN,
  DIRTY,
  MARKED_BEYOND,
  OBSERVED,
  UNCOMPUTED,
  type Status,
} from '../src/nodes';
import { statusOf } from '../test/status';
import type * as EngineModule from '../src/index';

/** A module instance of the engine, imported again for each case. */
type Engine = typeof EngineModule;

type Read =
  | { readonly kind: 'source'; readonly source: number }
  | { readonly kind: 'derived'; readonly node: number };

/** How a computation turns its number into the value it returns. */
type ResultShape = 'number' | 'reused-object' | 'fresh-object';

type NodeSpec = {
  readonly selector: Read;
  readonly branches: ReadonlyArray<ReadonlyArray<Read>>;
  readonly modulus: number;
  readonly shape: ResultShape;
};

/** A derived node reads only sources and derived nodes of a smaller index. */
type GraphSpec = {
  readonly sourceValues: ReadonlyArray<number>;
  readonly nodes: ReadonlyArray<NodeSpec>;
};

type WriteStep = {
  readonly kind: 'write';
  readonly source: number;
  readonly value: number;
};

/** A top-level read of any node. A release of a node that the run has no retain of is skipped. */
type ReadStep = { readonly kind: 'read'; readonly node: number };

type RetainStep = { readonly kind: 'retain'; readonly node: number };

type ReleaseStep = { readonly kind: 'release'; readonly node: number };

type Step = WriteStep | ReadStep | RetainStep | ReleaseStep;

/** A callback calls `readDerivedNode` on each node of `reads`. */
type CallbackSpec = {
  readonly id: number;
  readonly reads: ReadonlyArray<number>;
};

type Batch = {
  readonly kind: 'batch';
  readonly items: ReadonlyArray<BatchItem>;
};

type BatchItem =
  | Step
  | Batch
  | { readonly kind: 'single'; readonly write: WriteStep };

/**
 * Retains the node with an `onChange` that runs `callback`, then releases the
 * retain with an `onChange` that the run made earlier on the node, so the node
 * stays retained. Without `callback`, the step only releases.
 */
type ReplaceOnChangeRetainStep = {
  readonly kind: 'replaceOnChangeRetain';
  readonly node: number;
  readonly callback: CallbackSpec | undefined;
};

/**
 * Under the `deferred` scheduler setting, only this step and the end of the
 * case call `disposeAll`.
 */
type DisposeAllStep = { readonly kind: 'disposeAll' };

type Op = Step | Batch | ReplaceOnChangeRetainStep | DisposeAllStep;

/**
 * `sync` makes the scheduler `disposeAll`, so the cleanup queue is empty
 * after each outermost operation. `deferred` makes it do nothing, so a node
 * left unneeded stays linked until a `disposeAll` step, unless a write's
 * marking reaches it.
 */
type SchedulerSetting = 'sync' | 'deferred';

type Case = { readonly graph: GraphSpec; readonly ops: ReadonlyArray<Op> };

/**
 * `single` makes every write a `writeSource` outside a batch, and
 * `one-op-batch` makes every write a batch of one `writeSource`, so the two
 * runs must log the same events. `grouped` runs each `Batch` as one batch and
 * its `single` items as `writeSource` calls inside it.
 */
type Mode = 'single' | 'one-op-batch' | 'grouped';

type Value = number | { readonly n: number };

type Synchronization = 'in-sync' | 'out-of-sync';

/** A read of a node's latest computation, with the counters at that read. */
type ReadRecord =
  | {
      readonly kind: 'source';
      readonly source: number;
      readonly writes: number;
    }
  | {
      readonly kind: 'derived';
      readonly node: number;
      readonly changes: number;
    };

/**
 * `flushQueue` is the model's flush queue of the batch. `expected` is the
 * callbacks of the nodes with callbacks that the flush recomputed to a
 * changed value, once per node, in the order of the first recompute of each
 * node that changed its value, and `invoked` is the callbacks the flush called.
 */
type Frame = {
  readonly flushQueue: Array<number>;
  readonly invoked: Array<number>;
  readonly expected: Array<number>;
  readonly expectedNodes: Array<number>;
};

type LinkedEdge = {
  readonly dependency: unknown;
  readonly dependent: unknown;
  readonly nextDependent: LinkedEdge | undefined;
};

type NodeWithDependents = { readonly firstDependent: LinkedEdge | undefined };

/** The retain with an `onChange` that the run made on a node. */
type OnChangeRetain = {
  readonly retain: Retain<Value>;
  readonly callback: number;
};

const MAXIMUM_RUNS = 1_000_000;

function isObservedNode(node: DerivedNode<unknown>): boolean {
  return internal(node).evaluation === OBSERVED;
}

function isMarkedBeyond(node: DerivedNode<unknown>): boolean {
  return (internal(node).flags & MARKED_BEYOND) !== 0;
}

function retainList(node: DerivedNode<Value>): ReadonlyArray<Retain<Value>> {
  const retains: Array<Retain<Value>> = [];
  let retain = internal(node).firstRetain;
  while (retain !== undefined) {
    retains.push(retain);
    retain = retain.next;
  }
  return retains;
}

function valueNumber(value: Value): number {
  return typeof value === 'number' ? value : value.n;
}

function sameRead(left: Read, right: Read): boolean {
  switch (left.kind) {
    case 'source':
      return right.kind === 'source' && right.source === left.source;
    case 'derived':
      return right.kind === 'derived' && right.node === left.node;
  }
}

function evaluate(spec: NodeSpec, read: (read: Read) => number): number {
  const selected = read(spec.selector);
  const sum = at(spec.branches, selected % spec.branches.length).reduce(
    (total, branchRead) => total + read(branchRead),
    selected,
  );
  return sum % spec.modulus;
}

function shapeResult(
  shape: ResultShape,
  n: number,
  previous: Value | undefined,
): Value {
  switch (shape) {
    case 'number':
      return n;
    case 'reused-object':
      return typeof previous === 'object' && previous.n === n
        ? previous
        : { n };
    case 'fresh-object':
      return { n };
  }
}

/** The value of every derived node, computed from the source values alone. */
function scratchValues(
  graph: GraphSpec,
  sourceValues: ReadonlyArray<number>,
): ReadonlyArray<number> {
  return graph.nodes.reduce<ReadonlyArray<number>>(
    (values, spec) => [
      ...values,
      evaluate(spec, (read) =>
        read.kind === 'source'
          ? at(sourceValues, read.source)
          : at(values, read.node),
      ),
    ],
    [],
  );
}

/**
 * A naive model of the README's model and guarantees over arrays. Each node's
 * derived dependency list is an array of node indices that a run edits by
 * position, as the engine edits its edge list, and a node is needed while it
 * is retained, has a retain with an `onChange`, or is in another node's list of
 * dependencies. A flush that refreshes a node with a callback depends on the
 * order of the flush queue, which the
 * guarantees leave open, so the model goes out of sync there and the run
 * adopts the engine's state at the end of the operation, after checking it
 * against the guarantees.
 */
class Model {
  readonly #graph: GraphSpec;
  #synchronization: Synchronization = 'in-sync';
  readonly #sourceValues: Array<number>;
  readonly #status: Array<Status>;
  readonly #value: Array<number | undefined>;
  readonly #reads: Array<ReadonlyArray<Read>>;
  readonly #derivedEdges: Array<Array<number>>;
  readonly #retainCount: Array<number>;
  readonly #hasOnChangeRetain: Array<boolean>;
  /**
   * Whether each `MAYBE_DIRTY` node has `MARKED_BEYOND`, because marking
   * reached it only beyond a node with a callback on the flush queue. Read only while
   * the node is `MAYBE_DIRTY`.
   */
  readonly #markedBeyond: Array<boolean>;
  /** The nodes that the runs of the current read left unneeded. */
  #pendingDisposals: Array<number> = [];
  readonly predictedRuns: Array<number> = [];
  readonly #schedulerSetting: SchedulerSetting;
  /**
   * The cleanup queue under the `deferred` scheduler setting: one entry per
   * push, and a node that is needed again when it is popped is not disposed.
   */
  #cleanupQueue: Array<number> = [];
  /** The nodes that marking newly marked DIRTY or MAYBE_DIRTY while they were unneeded. */
  #marked: Array<number> = [];

  constructor(graph: GraphSpec, schedulerSetting: SchedulerSetting) {
    this.#graph = graph;
    this.#schedulerSetting = schedulerSetting;
    this.#sourceValues = [...graph.sourceValues];
    this.#status = graph.nodes.map(() => UNCOMPUTED);
    this.#value = graph.nodes.map(() => undefined);
    this.#reads = graph.nodes.map(() => []);
    this.#derivedEdges = graph.nodes.map(() => []);
    this.#retainCount = graph.nodes.map(() => 0);
    this.#hasOnChangeRetain = graph.nodes.map(() => false);
    this.#markedBeyond = graph.nodes.map(() => false);
  }

  get synchronization(): Synchronization {
    return this.#synchronization;
  }

  get sourceValues(): ReadonlyArray<number> {
    return this.#sourceValues;
  }

  status(node: number): Status {
    return at(this.#status, node);
  }

  value(node: number): number | undefined {
    return this.#value[node];
  }

  markedBeyond(node: number): boolean {
    return at(this.#markedBeyond, node);
  }

  reads(node: number): ReadonlyArray<Read> {
    return at(this.#reads, node);
  }

  derivedEdges(node: number): ReadonlyArray<number> {
    return at(this.#derivedEdges, node);
  }

  retainCount(node: number): number {
    return at(this.#retainCount, node);
  }

  inCleanupQueue(node: number): boolean {
    return this.#cleanupQueue.includes(node);
  }

  desynchronize(): void {
    this.#synchronization = 'out-of-sync';
  }

  resynchronize(
    status: ReadonlyArray<Status>,
    value: ReadonlyArray<number | undefined>,
    reads: ReadonlyArray<ReadonlyArray<Read>>,
    derivedEdges: ReadonlyArray<ReadonlyArray<number>>,
    markedBeyond: ReadonlyArray<boolean>,
  ): void {
    this.#marked = [];
    status.forEach((nodeStatus, node) => {
      this.#status[node] = nodeStatus;
      this.#markedBeyond[node] = at(markedBeyond, node);
      this.#value[node] = value[node];
      this.#reads[node] = at(reads, node);
      this.#derivedEdges[node] = [...at(derivedEdges, node)];
    });
    // The guarantees put every computed unneeded node on the cleanup queue,
    // and a popped entry whose node is needed is not disposed, so these
    // entries dispose what the engine's entries dispose.
    this.#cleanupQueue = this.#graph.nodes.flatMap((_, node) =>
      this.#unneeded(node) ? [node] : [],
    );
    this.#synchronization = 'in-sync';
  }

  write(write: WriteStep, flushQueue: Array<number>): void {
    this.#sourceValues[write.source] = write.value;
    if (this.#synchronization === 'in-sync') {
      this.#markDirty(
        this.#dependents({ kind: 'source', source: write.source }),
        flushQueue,
      );
    }
  }

  retain(node: number): void {
    if (this.#synchronization === 'in-sync') {
      this.#update(node);
    }
    this.#retainCount[node] = at(this.#retainCount, node) + 1;
  }

  release(node: number): void {
    const wasObserved = this.#observed(node);
    this.#retainCount[node] = at(this.#retainCount, node) - 1;
    this.#deactivate(node, wasObserved);
    if (this.#synchronization === 'in-sync') {
      this.#disposeMarkedIfOutermost();
      this.#dispose([node]);
    }
  }

  /** A node with a retain with an `onChange` is retained and has a callback. Adding that retain brings the node up to date first, and removing it disposes the node as a release does. */
  setOnChangeRetain(node: number, withOnChange: boolean): void {
    if (withOnChange && this.#synchronization === 'in-sync') {
      this.#update(node);
    }
    const wasObserved = this.#observed(node);
    this.#hasOnChangeRetain[node] = withOnChange;
    this.#deactivate(node, wasObserved);
    if (this.#synchronization === 'in-sync') {
      this.#disposeMarkedIfOutermost();
    }
    if (!withOnChange && this.#synchronization === 'in-sync') {
      this.#dispose([node]);
    }
  }

  /** Marking stops at a node that has a retain with an `onChange`. */
  #observed(node: number): boolean {
    return at(this.#hasOnChangeRetain, node);
  }

  /** A node that loses its last callback while DIRTY or MAYBE_DIRTY marks its dependents, and the engine flushes the nodes on the flush queue that this marking produces at once, which the model does not order. */
  #deactivate(node: number, wasObserved: boolean): void {
    if (
      !wasObserved ||
      this.#observed(node) ||
      this.#synchronization === 'out-of-sync' ||
      (at(this.#status, node) !== DIRTY &&
        at(this.#status, node) !== MAYBE_DIRTY)
    ) {
      return;
    }
    const flushQueue: Array<number> = [];
    this.#markMaybeDirty([node], flushQueue);
    if (this.refreshes(flushQueue)) {
      this.desynchronize();
    }
  }

  #retained(node: number): boolean {
    return at(this.#retainCount, node) > 0 || at(this.#hasOnChangeRetain, node);
  }

  /** Whether the flush of a batch with `flushQueue` refreshes a node with a callback. */
  refreshes(flushQueue: ReadonlyArray<number>): boolean {
    return flushQueue.some(
      (node) =>
        this.#observed(node) &&
        (at(this.#status, node) === DIRTY ||
          at(this.#status, node) === MAYBE_DIRTY),
    );
  }

  /** A top-level read, after which a node that has no retain and no dependent is disposed with each dependency that this leaves unneeded. */
  read(node: number): number {
    const value = this.#update(node);
    if (this.#unneeded(node)) {
      this.#dispose([node]);
    }
    return value;
  }

  #update(node: number): number {
    const value = this.#pull(node);
    const left = this.#pendingDisposals;
    this.#pendingDisposals = [];
    this.#dispose(left);
    return value;
  }

  /**
   * A node with a callback that is not CLEAN, outside a flush and outside a batch,
   * is on no flush queue: marking left it MAYBE_DIRTY with MARKED_BEYOND above
   * a node with a callback that kept its value. A pull brings it up to date as a lazy node.
   */
  #pull(node: number): number {
    switch (at(this.#status, node)) {
      case CLEAN:
        return this.#stored(node);
      case DIRTY:
        return this.#recompute(node);
      case MAYBE_DIRTY:
        return this.#verify(node) === DIRTY
          ? this.#recompute(node)
          : this.#stored(node);
      case UNCOMPUTED:
        return this.#computeFirst(node);
    }
  }

  #dependents(target: Read): ReadonlyArray<number> {
    switch (target.kind) {
      case 'source':
        return this.#reads.flatMap((reads, node) =>
          at(this.#status, node) !== UNCOMPUTED &&
          reads.some((read) => sameRead(read, target))
            ? [node]
            : [],
        );
      case 'derived':
        return this.#derivedEdges.flatMap((edges, node) =>
          edges.includes(target.node) ? [node] : [],
        );
    }
  }

  #hasDependents(node: number): boolean {
    return this.#derivedEdges.some((edges) => edges.includes(node));
  }

  #unneeded(node: number): boolean {
    return (
      at(this.#status, node) !== UNCOMPUTED &&
      !this.#retained(node) &&
      !this.#hasDependents(node)
    );
  }

  /** The engine disposes when the outermost operation ends, so a batch or a `writeSource` outside a batch defers the disposals inside it. */
  #opDepth = 0;
  #deferred: Array<number> = [];

  enterOp(): void {
    this.#opDepth += 1;
  }

  exitOp(): void {
    this.#opDepth -= 1;
    if (this.#synchronization === 'in-sync') {
      this.#disposeMarkedIfOutermost();
    }
    if (this.#opDepth === 0 && this.#deferred.length > 0) {
      const deferred = this.#deferred;
      this.#deferred = [];
      if (this.#synchronization === 'in-sync') {
        this.#dispose(deferred);
      }
    }
  }

  #dispose(start: ReadonlyArray<number>): void {
    if (this.#opDepth > 0) {
      this.#deferred.push(...start);
      return;
    }
    const schedulerSetting: SchedulerSetting = this.#schedulerSetting;
    switch (schedulerSetting) {
      case 'sync':
        this.#disposeNow(start);
        break;
      case 'deferred':
        this.#cleanupQueue.push(...start);
        break;
    }
  }

  disposeAll(): void {
    const cleanupQueue = this.#cleanupQueue;
    this.#cleanupQueue = [];
    this.#disposeNow(cleanupQueue);
  }

  /** At the end of the outermost operation, each marked node that is still unneeded is disposed, with each dependency that this leaves unneeded and that is not CLEAN, and the CLEAN ones go to the cleanup queue. */
  #disposeMarkedIfOutermost(): void {
    if (this.#opDepth > 0 || this.#marked.length === 0) {
      return;
    }
    const stack = this.#marked;
    this.#marked = [];
    const clean: Array<number> = [];
    let node = stack.pop();
    while (node !== undefined) {
      if (this.#unneeded(node) && at(this.#status, node) !== CLEAN) {
        const dependencies = at(this.#derivedEdges, node);
        this.#derivedEdges[node] = [];
        this.#reads[node] = [];
        this.#status[node] = UNCOMPUTED;
        this.#value[node] = undefined;
        dependencies.forEach((dependency) => {
          if (!this.#unneeded(dependency)) {
            return;
          }
          if (at(this.#status, dependency) === CLEAN) {
            clean.push(dependency);
          } else {
            stack.push(dependency);
          }
        });
      }
      node = stack.pop();
    }
    this.#dispose(clean);
  }

  #disposeNow(start: ReadonlyArray<number>): void {
    const stack = [...start];
    let node = stack.pop();
    while (node !== undefined) {
      if (this.#unneeded(node)) {
        stack.push(...at(this.#derivedEdges, node));
        this.#derivedEdges[node] = [];
        this.#reads[node] = [];
        this.#status[node] = UNCOMPUTED;
        this.#value[node] = undefined;
      }
      node = stack.pop();
    }
  }

  /**
   * A CLEAN reader becomes DIRTY, and a CLEAN node above becomes MAYBE_DIRTY.
   * A node with a callback that marking takes out of CLEAN goes on the flush queue,
   * and the walk past it makes each CLEAN node above it MAYBE_DIRTY with
   * MARKED_BEYOND and puts none on the flush queue. A MAYBE_DIRTY node with
   * MARKED_BEYOND that marking reaches through readers and lazy nodes loses
   * the bit and is marked again that way.
   */
  #markDirty(
    dependents: ReadonlyArray<number>,
    flushQueue: Array<number>,
  ): void {
    const lazy: Array<number> = [];
    const beyond: Array<number> = [];
    dependents.forEach((node) => {
      switch (at(this.#status, node)) {
        case CLEAN:
          this.#status[node] = DIRTY;
          this.#markedBeyond[node] = false;
          this.#reachInLine(node, flushQueue, lazy, beyond);
          break;
        case MAYBE_DIRTY:
          this.#status[node] = DIRTY;
          if (at(this.#markedBeyond, node)) {
            this.#markedBeyond[node] = false;
            this.#reachMarkedBeyond(node, flushQueue, lazy);
          }
          break;
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    });
    this.#markInLine(lazy, flushQueue, beyond);
  }

  /** Marks the dependents of each node of `lazy` with the walk through lazy nodes. */
  #markMaybeDirty(lazy: Array<number>, flushQueue: Array<number>): void {
    this.#markInLine(lazy, flushQueue, []);
  }

  #markInLine(
    lazy: Array<number>,
    flushQueue: Array<number>,
    beyond: Array<number>,
  ): void {
    let marked = lazy.pop();
    while (marked !== undefined) {
      this.#dependents({ kind: 'derived', node: marked }).forEach(
        (dependent) => {
          switch (at(this.#status, dependent)) {
            case CLEAN:
              this.#status[dependent] = MAYBE_DIRTY;
              this.#markedBeyond[dependent] = false;
              this.#reachInLine(dependent, flushQueue, lazy, beyond);
              break;
            case MAYBE_DIRTY:
              if (at(this.#markedBeyond, dependent)) {
                this.#markedBeyond[dependent] = false;
                this.#reachMarkedBeyond(dependent, flushQueue, lazy);
              }
              break;
            case DIRTY:
            case UNCOMPUTED:
              break;
          }
        },
      );
      marked = lazy.pop();
    }
    let beyondNode = beyond.pop();
    while (beyondNode !== undefined) {
      this.#dependents({ kind: 'derived', node: beyondNode }).forEach(
        (dependent) => {
          if (at(this.#status, dependent) !== CLEAN) {
            return;
          }
          this.#status[dependent] = MAYBE_DIRTY;
          this.#markedBeyond[dependent] = true;
          this.#recordIfUnneeded(dependent);
          beyond.push(dependent);
        },
      );
      beyondNode = beyond.pop();
    }
  }

  /** A node that the walk through lazy nodes took out of CLEAN. */
  #reachInLine(
    node: number,
    flushQueue: Array<number>,
    lazy: Array<number>,
    beyond: Array<number>,
  ): void {
    if (this.#observed(node)) {
      flushQueue.push(node);
      beyond.push(node);
    } else {
      this.#recordIfUnneeded(node);
      lazy.push(node);
    }
  }

  /** A MAYBE_DIRTY node with MARKED_BEYOND that the walk through lazy nodes reached. */
  #reachMarkedBeyond(
    node: number,
    flushQueue: Array<number>,
    lazy: Array<number>,
  ): void {
    if (this.#observed(node)) {
      flushQueue.push(node);
    } else {
      lazy.push(node);
    }
  }

  #recordIfUnneeded(node: number): void {
    if (!this.#retained(node) && !this.#hasDependents(node)) {
      this.#marked.push(node);
    }
  }

  #stored(node: number): number {
    const value = this.#value[node];
    if (value === undefined) {
      throw new Error(`the model has no value for node ${node}`);
    }
    return value;
  }

  #verify(node: number): typeof DIRTY | typeof CLEAN {
    for (const dependency of [...at(this.#derivedEdges, node)]) {
      this.#pull(dependency);
      if (at(this.#status, node) === DIRTY) {
        return DIRTY;
      }
    }
    this.#status[node] = CLEAN;
    return CLEAN;
  }

  /**
   * Each `readDerived` reuses the entry at its position, repoints it, or
   * appends one unless it repeats the previous `readDerived` past the end of
   * the list. A node repointed away from, and the entries past the last read,
   * which the run drops, are checked when the read returns.
   */
  #run(node: number): number {
    this.predictedRuns.push(node);
    const spec = at(this.#graph.nodes, node);
    const edges = at(this.#derivedEdges, node);
    const candidates: Array<number> = [];
    const reads: Array<Read> = [];
    let cursor = 0;
    const value = evaluate(spec, (read) => {
      reads.push(read);
      if (read.kind === 'source') {
        return at(this.#sourceValues, read.source);
      }
      const readValue = this.#pull(read.node);
      const next = edges[cursor];
      if (next !== undefined) {
        if (next !== read.node) {
          edges[cursor] = read.node;
          if (!this.#retained(next) && !this.#hasDependents(next)) {
            candidates.push(next);
          }
        }
        cursor += 1;
      } else if (cursor === 0 || edges[cursor - 1] !== read.node) {
        edges.push(read.node);
        cursor += 1;
      }
      return readValue;
    });
    const dropped = edges.splice(cursor);
    this.#reads[node] = reads;
    this.#status[node] = CLEAN;
    this.#pendingDisposals.push(...dropped, ...candidates);
    return value;
  }

  #computeFirst(node: number): number {
    const value = this.#run(node);
    this.#value[node] = value;
    return value;
  }

  #recompute(node: number): number {
    const previous = this.#stored(node);
    const value = this.#run(node);
    const changed =
      at(this.#graph.nodes, node).shape === 'fresh-object' ||
      value !== previous;
    if (!changed) {
      return previous;
    }
    this.#value[node] = value;
    this.#dependents({ kind: 'derived', node }).forEach((dependent) => {
      if (at(this.#status, dependent) === MAYBE_DIRTY) {
        this.#status[dependent] = DIRTY;
      }
    });
    return value;
  }
}

/** One run of a case through pico-js in one mode, checked against the model. */
class Run {
  readonly #mode: Mode;
  readonly #graph: GraphSpec;
  readonly #model: Model;
  readonly #sources: ReadonlyArray<SourceNode<number>>;
  readonly #nodes: ReadonlyArray<DerivedNode<Value>>;
  readonly #indexOf = new Map<unknown, Read>();
  readonly log: Array<string> = [];
  readonly #frames: Array<Frame> = [];
  readonly #recordedReads: Array<ReadonlyArray<Read>>;
  readonly #lastReads: Array<ReadonlyArray<ReadRecord> | undefined>;
  readonly #writeCount: Array<number>;
  readonly #changeCount: Array<number>;
  readonly #onChangeRetains: Array<OnChangeRetain | undefined>;
  /** The retains without an `onChange` that the run made on each node. */
  readonly #retains: ReadonlyArray<Array<Retain<Value>>>;
  readonly #actualRuns: Array<number> = [];
  #totalRuns = 0;
  #bodyDepth = 0;
  #where = 'setup';

  readonly #engine: Engine;
  readonly #schedulerSetting: SchedulerSetting;

  constructor(
    engine: Engine,
    graph: GraphSpec,
    mode: Mode,
    schedulerSetting: SchedulerSetting,
  ) {
    this.#engine = engine;
    this.#mode = mode;
    this.#schedulerSetting = schedulerSetting;
    this.#graph = graph;
    this.#model = new Model(graph, schedulerSetting);
    this.#sources = graph.sourceValues.map((value) =>
      this.#engine.createSourceNode(value),
    );
    this.#nodes = graph.nodes.map((_, index) => {
      const computation = (
        callingNode: CallingNode,
        previous: Value | undefined,
      ): Value => this.#compute(index, callingNode, previous);
      return this.#engine.createDerivedNode<Value>(computation);
    });
    this.#sources.forEach((source, index) =>
      this.#indexOf.set(source, { kind: 'source', source: index }),
    );
    this.#nodes.forEach((node, index) =>
      this.#indexOf.set(node, { kind: 'derived', node: index }),
    );
    this.#recordedReads = graph.nodes.map(() => []);
    this.#lastReads = graph.nodes.map(() => undefined);
    this.#writeCount = graph.sourceValues.map(() => 0);
    this.#changeCount = graph.nodes.map(() => 0);
    this.#onChangeRetains = graph.nodes.map(() => undefined);
    this.#retains = graph.nodes.map(() => []);
  }

  execute(op: Op, index: number): void {
    this.#where = `op ${index} (${JSON.stringify(op)})`;
    this.log.push(`op ${index}`);
    switch (op.kind) {
      case 'write':
        this.#topLevelWrite(op);
        break;
      case 'read':
        this.#read(op.node);
        break;
      case 'retain':
        this.#retain(op.node);
        break;
      case 'release':
        this.#release(op.node);
        break;
      case 'replaceOnChangeRetain':
        this.#replaceOnChangeRetain(op);
        break;
      case 'disposeAll':
        this.#disposeAll();
        break;
      case 'batch':
        switch (this.#mode) {
          case 'single':
          case 'one-op-batch':
            this.#flattened(op.items);
            break;
          case 'grouped':
            this.#batch(op.items);
            break;
        }
        break;
    }
    this.#endOp();
  }

  #disposeAll(): void {
    const synchronization = this.#model.synchronization;
    if (synchronization === 'in-sync') {
      this.#model.disposeAll();
    }
    this.#engine.disposeAll();
    this.log.push('disposeAll');
    if (this.#actualRuns.length !== 0) {
      this.#fail('disposeAll ran a computation');
    }
  }

  /**
   * When every retain is released, every derived node is disposed, and no
   * source has a dependent.
   */
  finish(): void {
    this.#where = 'after the last operation';
    this.#engine.disposeAll();
    if (
      this.#retains.some((retains) => retains.length !== 0) ||
      this.#onChangeRetains.some(
        (onChangeRetain) => onChangeRetain !== undefined,
      )
    ) {
      return;
    }
    this.#nodes.forEach((node, index) => {
      if (
        statusOf(node) !== UNCOMPUTED ||
        internal(node).firstDependent !== undefined
      ) {
        this.#fail(
          `every retain is released, and node ${index} has status ${statusOf(node)}`,
        );
      }
    });
    this.#sources.forEach((source, index) => {
      if (internal(source).firstDependent !== undefined) {
        this.#fail(
          `every retain is released, and source ${index} has a dependent`,
        );
      }
    });
  }

  #fail(message: string): never {
    throw new Error(
      `${this.#mode} mode, ${this.#schedulerSetting} scheduler, ${this.#where}: ${message}`,
    );
  }

  #flattened(items: ReadonlyArray<BatchItem>): void {
    items.forEach((item) => {
      switch (item.kind) {
        case 'write':
          this.#topLevelWrite(item);
          break;
        case 'read':
          this.#read(item.node);
          break;
        case 'retain':
          this.#retain(item.node);
          break;
        case 'release':
          this.#release(item.node);
          break;
        case 'batch':
          this.#flattened(item.items);
          break;
        case 'single':
          this.#topLevelWrite(item.write);
          break;
      }
    });
  }

  #topLevelWrite(write: WriteStep): void {
    switch (this.#mode) {
      case 'single':
      case 'grouped':
        this.#single(write);
        break;
      case 'one-op-batch':
        this.#batch([write]);
        break;
    }
  }

  #replaceOnChangeRetain(op: ReplaceOnChangeRetainStep): void {
    const node = at(this.#nodes, op.node);
    const previous = this.#onChangeRetains[op.node];
    const callback = op.callback;
    if (callback === undefined) {
      if (previous !== undefined) {
        this.#engine.releaseRetain(previous.retain);
      }
      this.#onChangeRetains[op.node] = undefined;
      this.#model.setOnChangeRetain(op.node, false);
      return;
    }
    const synchronization = this.#model.synchronization;
    const predictedStart = this.#model.predictedRuns.length;
    const actualStart = this.#actualRuns.length;
    this.#model.setOnChangeRetain(op.node, true);
    const retain = this.#engine.retainDerivedNode(node, (value) =>
      this.#invoke(callback, op.node, value),
    );
    if (previous !== undefined) {
      this.#engine.releaseRetain(previous.retain);
    }
    this.#onChangeRetains[op.node] = { retain, callback: callback.id };
    if (synchronization === 'in-sync') {
      const runs = this.#actualRuns.slice(actualStart);
      const predictedRuns = this.#model.predictedRuns.slice(predictedStart);
      if (runs.join(',') !== predictedRuns.join(',')) {
        this.#fail(
          `retaining ${op.node} with an onChange ran [${runs.join(', ')}], the model [${predictedRuns.join(', ')}]`,
        );
      }
      this.#compareModel(`after retaining ${op.node} with an onChange`);
    }
  }

  #invoke(callback: CallbackSpec, node: number, value: Value): void {
    const frame = this.#frames.at(-1);
    if (frame === undefined) {
      this.#fail(`callback ${callback.id} ran outside a flush`);
    }
    if (value !== internal(at(this.#nodes, node)).value) {
      this.#fail(
        `callback ${callback.id} received a value other than the value of node ${node}`,
      );
    }
    frame.invoked.push(callback.id);
    this.log.push(`callback ${callback.id}`);
    if (this.#bodyDepth === 0) {
      this.#checkObservedSettled(`in callback ${callback.id}`);
    }
    callback.reads.forEach((node) => this.#read(node));
  }

  #retain(node: number): void {
    const synchronization = this.#model.synchronization;
    const predictedStart = this.#model.predictedRuns.length;
    const actualStart = this.#actualRuns.length;
    this.#model.retain(node);
    at(this.#retains, node).push(
      this.#engine.retainDerivedNode(at(this.#nodes, node)),
    );
    this.log.push(`retain ${node}`);
    if (synchronization === 'in-sync') {
      const runs = this.#actualRuns.slice(actualStart);
      const predictedRuns = this.#model.predictedRuns.slice(predictedStart);
      if (runs.join(',') !== predictedRuns.join(',')) {
        this.#fail(
          `retainDerivedNode(${node}) ran [${runs.join(', ')}], the model [${predictedRuns.join(', ')}]`,
        );
      }
    }
  }

  #release(node: number): void {
    const retain = at(this.#retains, node).pop();
    if (retain === undefined) {
      this.log.push(`skip release ${node}`);
      return;
    }
    const runs = this.#actualRuns.length;
    const synchronization = this.#model.synchronization;
    // A release that leaves a DIRTY or MAYBE_DIRTY node with no callback left flushes the nodes that its marking put on the flush queue, as a flush of its own.
    // The model goes out of sync first when the engine's release will flush, so the callbacks of that flush are not checked against it.
    this.#model.release(node);
    const frame = this.#bodyDepth > 0 ? this.#pushFrame() : undefined;
    this.#engine.releaseRetain(retain);
    if (frame !== undefined) {
      this.#popFrame(frame);
    }
    this.log.push(`release ${node}`);
    if (
      this.#actualRuns.length !== runs &&
      this.#model.synchronization === 'in-sync'
    ) {
      this.#fail(`releaseRetain of node ${node} ran a computation`);
    }
    if (synchronization === 'in-sync') {
      this.#compareModel(`after releaseRetain of node ${node}`);
    }
  }

  #skips(write: WriteStep): boolean {
    const skips =
      internal(at(this.#sources, write.source)).value === write.value;
    if (skips) {
      this.log.push('skip');
    }
    return skips;
  }

  #pushFrame(): Frame {
    const frame: Frame = {
      flushQueue: [],
      invoked: [],
      expected: [],
      expectedNodes: [],
    };
    this.#frames.push(frame);
    return frame;
  }

  #predictWrite(frame: Frame, write: WriteStep): Synchronization {
    const synchronization = this.#model.synchronization;
    this.#model.write(write, frame.flushQueue);
    this.#writeCount[write.source] = at(this.#writeCount, write.source) + 1;
    return synchronization;
  }

  #predictFlush(frame: Frame): void {
    if (
      this.#model.synchronization === 'in-sync' &&
      this.#model.refreshes(frame.flushQueue)
    ) {
      this.#model.desynchronize();
    }
  }

  #single(write: WriteStep): void {
    if (this.#skips(write)) {
      return;
    }
    const frame = this.#pushFrame();
    this.#model.enterOp();
    this.#predictWrite(frame, write);
    this.#predictFlush(frame);
    this.#engine.writeSource(at(this.#sources, write.source), write.value);
    this.#model.exitOp();
    this.#popFrame(frame);
  }

  #batch(items: ReadonlyArray<BatchItem>): void {
    const frame = this.#pushFrame();
    this.#model.enterOp();
    this.#engine.startWriteBatch();
    this.#bodyDepth += 1;
    items.forEach((item) => this.#bodyItem(frame, item));
    this.#predictFlush(frame);
    this.#bodyDepth -= 1;
    this.#engine.endWriteBatch();
    this.#model.exitOp();
    this.#popFrame(frame);
  }

  #bodyItem(frame: Frame, item: BatchItem): void {
    switch (item.kind) {
      case 'write':
        this.#writeInBatch(frame, item);
        break;
      case 'read':
        this.#read(item.node);
        break;
      case 'retain':
        this.#retain(item.node);
        break;
      case 'release':
        this.#release(item.node);
        break;
      // A nested batch and a `writeSource` inside a batch join the open batch.
      case 'batch':
        this.#engine.startWriteBatch();
        item.items.forEach((nested) => this.#bodyItem(frame, nested));
        this.#engine.endWriteBatch();
        break;
      case 'single':
        this.#writeInBatch(frame, item.write);
        break;
    }
  }

  #writeInBatch(frame: Frame, write: WriteStep): void {
    if (this.#skips(write)) {
      return;
    }
    const runs = this.#actualRuns.length;
    const invoked = frame.invoked.length;
    const synchronization = this.#predictWrite(frame, write);
    this.#engine.writeSource(at(this.#sources, write.source), write.value);
    if (this.#actualRuns.length !== runs || frame.invoked.length !== invoked) {
      this.#fail('writeSource inside a batch ran a computation or a callback');
    }
    if (synchronization === 'in-sync') {
      this.#compareModel('after writeSource inside a batch');
    }
  }

  /**
   * The flush calls the `onChange` of each node with a callback that it
   * recomputed to a changed value, once per node, in the order of the first
   * recompute of each node that changed its value, and no other callback.
   */
  #popFrame(frame: Frame): void {
    if (this.#frames.pop() !== frame) {
      this.#fail('the batch frames are unbalanced');
    }
    if (frame.invoked.join(',') !== frame.expected.join(',')) {
      this.#fail(
        `the flush recomputed nodes with callbacks [${frame.expectedNodes.join(', ')}] to changed values, whose callbacks are [${frame.expected.join(', ')}], and called [${frame.invoked.join(', ')}]`,
      );
    }
  }

  #read(node: number): void {
    const synchronization = this.#model.synchronization;
    const predictedStart = this.#model.predictedRuns.length;
    const actualStart = this.#actualRuns.length;
    const predicted =
      synchronization === 'in-sync' ? this.#model.read(node) : undefined;
    const value = valueNumber(
      this.#engine.readDerivedNode(at(this.#nodes, node)),
    );
    this.log.push(`read ${node} ${value}`);
    this.#checkOracle(
      `readDerivedNode(node ${node})`,
      value,
      at(scratchValues(this.#graph, this.#model.sourceValues), node),
    );
    const runs = this.#actualRuns.slice(actualStart);
    if (synchronization === 'in-sync') {
      const predictedRuns = this.#model.predictedRuns.slice(predictedStart);
      if (value !== predicted) {
        this.#fail(
          `readDerivedNode(${node}) returned ${value}, the model ${predicted}`,
        );
      }
      if (runs.join(',') !== predictedRuns.join(',')) {
        this.#fail(
          `readDerivedNode(${node}) ran [${runs.join(', ')}], the model [${predictedRuns.join(', ')}]`,
        );
      }
      this.#compareModel(`after readDerivedNode(${node})`);
    }
    // A second read of a node without a retain can follow the disposal of the node.
    if (at(this.#retains, node).length === 0) {
      return;
    }
    const runsBefore = this.#actualRuns.length;
    const again = valueNumber(
      this.#engine.readDerivedNode(at(this.#nodes, node)),
    );
    if (again !== value || this.#actualRuns.length !== runsBefore) {
      this.#fail(
        `a second readDerivedNode(${node}) ran a computation or changed value`,
      );
    }
  }

  /**
   * The oracle: compares a value that a computation receives from
   * `readSource` or `readDerived`, or that a top-level `readDerivedNode`
   * returns, with the value computed from the current source values alone.
   */
  #checkOracle(operation: string, value: number, expected: number): void {
    if (value !== expected) {
      this.#fail(
        `oracle: ${operation} returned ${value}, and computing from the current source values gives ${expected}`,
      );
    }
  }

  #compute(
    index: number,
    callingNode: CallingNode,
    previous: Value | undefined,
  ): Value {
    this.#totalRuns += 1;
    if (this.#totalRuns > MAXIMUM_RUNS) {
      this.#fail(`more than ${MAXIMUM_RUNS} computations ran`);
    }
    const status = statusOf(internal(callingNode));
    this.#actualRuns.push(index);
    this.log.push(`run ${index}`);
    if (status !== UNCOMPUTED) {
      this.#checkJustified(index);
    }
    const spec = at(this.#graph.nodes, index);
    const reads: Array<Read> = [];
    const records: Array<ReadRecord> = [];
    const n = evaluate(spec, (read) => {
      reads.push(read);
      switch (read.kind) {
        case 'source': {
          records.push({
            kind: 'source',
            source: read.source,
            writes: at(this.#writeCount, read.source),
          });
          const value = this.#engine.readSource(
            callingNode,
            at(this.#sources, read.source),
          );
          this.#checkOracle(
            `readSource(source ${read.source}) in the computation of node ${index}`,
            value,
            at(this.#model.sourceValues, read.source),
          );
          return value;
        }
        case 'derived': {
          const value = valueNumber(
            this.#engine.readDerived(callingNode, at(this.#nodes, read.node)),
          );
          this.#checkOracle(
            `readDerived(node ${read.node}) in the computation of node ${index}`,
            value,
            at(scratchValues(this.#graph, this.#model.sourceValues), read.node),
          );
          records.push({
            kind: 'derived',
            node: read.node,
            changes: at(this.#changeCount, read.node),
          });
          return value;
        }
      }
    });
    const result = shapeResult(spec.shape, n, previous);
    const changed = result !== previous;
    if (changed) {
      this.#changeCount[index] = at(this.#changeCount, index) + 1;
    }
    // Only the flush recomputes a node with a callback and is not UNCOMPUTED,
    // and the flush refreshes every node on the flush queue before it calls the
    // first callback.
    if (isObservedNode(internal(callingNode)) && status !== UNCOMPUTED) {
      const frame = this.#frames.at(-1);
      if (frame === undefined) {
        this.#fail(
          `node ${index}, which has a callback, recomputed outside a flush`,
        );
      }
      if (frame.invoked.length !== 0) {
        this.#fail(
          `node ${index}, which has a callback, recomputed after the flush called a callback`,
        );
      }
      const callback = this.#onChangeRetains[index]?.callback;
      if (
        changed &&
        callback !== undefined &&
        !frame.expectedNodes.includes(index)
      ) {
        frame.expectedNodes.push(index);
        frame.expected.push(callback);
      }
    }
    this.#recordedReads[index] = reads;
    this.#lastReads[index] = records;
    return result;
  }

  /**
   * A recompute runs only when a source its latest computation read was
   * written, or a derived node it read changed value.
   */
  #checkJustified(index: number): void {
    const records = this.#lastReads[index];
    if (records === undefined) {
      this.#fail(`node ${index} recomputed with no earlier computation`);
    }
    const justified = records.some((record) =>
      record.kind === 'source'
        ? at(this.#writeCount, record.source) > record.writes
        : at(this.#changeCount, record.node) > record.changes,
    );
    if (!justified) {
      this.#fail(
        `node ${index} recomputed, and no node its latest computation read changed`,
      );
    }
  }

  /**
   * A node with a callback is CLEAN or UNCOMPUTED, or MAYBE_DIRTY with MARKED_BEYOND
   * and its current value: marking leaves a node with a callback beyond another
   * node with a callback MAYBE_DIRTY, and it stays so when the node below it keeps its
   * value.
   */
  #observedSettledNode(
    node: DerivedNode<Value>,
    index: number,
    scratch: ReadonlyArray<number>,
  ): boolean {
    return (
      !isObservedNode(node) ||
      statusOf(node) === CLEAN ||
      statusOf(node) === UNCOMPUTED ||
      (statusOf(node) === MAYBE_DIRTY &&
        isMarkedBeyond(node) &&
        valueNumber(internal(node).value) === at(scratch, index))
    );
  }

  #checkObservedSettled(where: string): void {
    const scratch = scratchValues(this.#graph, this.#model.sourceValues);
    this.#nodes.forEach((node, index) => {
      if (!this.#observedSettledNode(node, index, scratch)) {
        this.#fail(
          `${where}, node ${index}, which has a callback, has status ${statusOf(node)}`,
        );
      }
    });
  }

  #engineDerivedEdges(node: DerivedNode<Value>): ReadonlyArray<number> {
    const edges: Array<number> = [];
    let edge = internal(node).firstDerivedDependency;
    while (edge !== undefined) {
      const dependency = this.#indexOf.get(edge.dependency);
      if (dependency === undefined || dependency.kind !== 'derived') {
        this.#fail('a list of derived dependencies has an edge to no node');
      }
      edges.push(dependency.node);
      edge = edge.nextDependency;
    }
    return edges;
  }

  #compareModel(where: string): void {
    if (this.#model.synchronization === 'out-of-sync') {
      return;
    }
    this.#nodes.forEach((node, index) => {
      const status = this.#model.status(index);
      if (statusOf(node) !== status) {
        this.#fail(
          `${where}, node ${index} has status ${statusOf(node)}, the model ${status}`,
        );
      }
      if (
        status === MAYBE_DIRTY &&
        isMarkedBeyond(node) !== this.#model.markedBeyond(index)
      ) {
        this.#fail(
          `${where}, MAYBE_DIRTY node ${index} has MARKED_BEYOND ${isMarkedBeyond(node)}, the model ${this.#model.markedBeyond(index)}`,
        );
      }
      const edges = this.#engineDerivedEdges(node);
      if (edges.join(',') !== this.#model.derivedEdges(index).join(',')) {
        this.#fail(
          `${where}, node ${index} has derived edges [${edges.join(', ')}], the model [${this.#model.derivedEdges(index).join(', ')}]`,
        );
      }
      if (status === UNCOMPUTED) {
        return;
      }
      const value = valueNumber(internal(node).value);
      if (value !== this.#model.value(index)) {
        this.#fail(
          `${where}, node ${index} has the value ${value}, the model ${this.#model.value(index)}`,
        );
      }
      if (
        JSON.stringify(this.#model.reads(index)) !==
        JSON.stringify(this.#recordedReads[index])
      ) {
        this.#fail(
          `${where}, node ${index} read ${JSON.stringify(this.#recordedReads[index])}, the model ${JSON.stringify(this.#model.reads(index))}`,
        );
      }
    });
  }

  /**
   * After a computation, its node is in the list of dependents of each node it
   * read and of no other node. A source whose `dependentsStatus` is 1 has only
   * `DIRTY` dependents.
   */
  #checkDependents(): void {
    const listedIn: Array<Array<Read>> = this.#graph.nodes.map(() => []);
    const dependencies: ReadonlyArray<readonly [Read, NodeWithDependents]> = [
      ...this.#sources.map(
        (source, index) =>
          [{ kind: 'source', source: index }, internal(source)] as const,
      ),
      ...this.#nodes.map(
        (node, index) =>
          [{ kind: 'derived', node: index }, internal(node)] as const,
      ),
    ];
    dependencies.forEach(([dependencyRead, dependency]) => {
      let edge = dependency.firstDependent;
      let count = 0;
      while (edge !== undefined) {
        count += 1;
        if (count > 100_000) {
          this.#fail(
            `the list of dependents of ${JSON.stringify(dependencyRead)} has a cycle`,
          );
        }
        const dependent = this.#indexOf.get(edge.dependent);
        if (
          edge.dependency !== dependency ||
          dependent === undefined ||
          dependent.kind !== 'derived'
        ) {
          this.#fail(
            `the list of dependents of ${JSON.stringify(dependencyRead)} has an edge of another node`,
          );
        }
        listedIn[dependent.node]?.push(dependencyRead);
        edge = edge.nextDependent;
      }
    });
    this.#nodes.forEach((node, index) => {
      const listed = at(listedIn, index);
      const read =
        statusOf(node) === UNCOMPUTED ? [] : at(this.#recordedReads, index);
      const missing = read.filter(
        (target) => !listed.some((entry) => sameRead(entry, target)),
      );
      const extra = listed.filter(
        (entry) => !read.some((target) => sameRead(entry, target)),
      );
      if (missing.length !== 0 || extra.length !== 0) {
        this.#fail(
          `node ${index} (status ${statusOf(node)}) is missing from the dependents of ${JSON.stringify(missing)} and is extra in those of ${JSON.stringify(extra)}`,
        );
      }
    });
    this.#sources.forEach((source, sourceIndex) => {
      if (internal(source).dependentsStatus !== 1) {
        return;
      }
      this.#nodes.forEach((node, index) => {
        if (
          statusOf(node) !== UNCOMPUTED &&
          statusOf(node) !== DIRTY &&
          at(this.#recordedReads, index).some((target) =>
            sameRead(target, { kind: 'source', source: sourceIndex }),
          )
        ) {
          this.#fail(
            `source ${sourceIndex} has dependentsStatus 1, and its dependent ${index} has status ${statusOf(node)}`,
          );
        }
      });
    });
  }

  /**
   * Each node's retain list contains exactly the retains the run has not
   * released. A node that is not `UNCOMPUTED` is retained, has a dependent, or
   * is on the model's cleanup queue under the `deferred` scheduler setting.
   * An `UNCOMPUTED` node has no value, no dependency, and no dependent.
   */
  #checkLifetimes(): void {
    this.#nodes.forEach((node, index) => {
      const retains = at(this.#retains, index);
      const onChangeRetain = this.#onChangeRetains[index];
      const expectedRetains =
        onChangeRetain === undefined
          ? retains
          : [...retains, onChangeRetain.retain];
      const listed = retainList(node);
      if (
        listed.length !== expectedRetains.length ||
        expectedRetains.some((retain) => !listed.includes(retain))
      ) {
        this.#fail(
          `node ${index} has ${listed.length} retains in its retain list, and the run has ${expectedRetains.length}`,
        );
      }
      if (retains.length !== this.#model.retainCount(index)) {
        this.#fail(
          `node ${index} has ${retains.length} retains without an onChange, the model ${this.#model.retainCount(index)}`,
        );
      }
      if (statusOf(node) === UNCOMPUTED) {
        if (
          internal(node).value !== undefined ||
          internal(node).firstSourceDependency !== undefined ||
          internal(node).firstDerivedDependency !== undefined ||
          internal(node).firstDependent !== undefined
        ) {
          this.#fail(
            `UNCOMPUTED node ${index} has a value, a dependency, or a dependent`,
          );
        }
      } else if (
        internal(node).firstRetain === undefined &&
        internal(node).firstDependent === undefined &&
        (this.#schedulerSetting === 'sync' ||
          (this.#model.synchronization === 'in-sync' &&
            !this.#model.inCleanupQueue(index)))
      ) {
        this.#fail(
          `node ${index} has status ${statusOf(node)}, an empty retain list, no dependent, and is not on the model's cleanup queue`,
        );
      }
    });
  }

  #endOp(): void {
    if (this.#bodyDepth !== 0 || this.#frames.length !== 0) {
      this.#fail('a batch is still open after the operation');
    }
    this.#checkObservedSettled('after the operation');
    const scratch = scratchValues(this.#graph, this.#model.sourceValues);
    this.#nodes.forEach((node, index) => {
      if (
        statusOf(node) === CLEAN &&
        valueNumber(internal(node).value) !== at(scratch, index)
      ) {
        this.#fail(
          `CLEAN node ${index} has the value ${valueNumber(internal(node).value)}, and recomputing from the sources gives ${at(scratch, index)}`,
        );
      }
    });
    this.#checkDependents();
    this.#checkLifetimes();
    switch (this.#model.synchronization) {
      case 'in-sync':
        this.#compareModel('after the operation');
        if (
          this.#actualRuns.join(',') !== this.#model.predictedRuns.join(',')
        ) {
          this.#fail(
            `the operation ran [${this.#actualRuns.join(', ')}], the model [${this.#model.predictedRuns.join(', ')}]`,
          );
        }
        break;
      case 'out-of-sync':
        this.#model.resynchronize(
          this.#nodes.map((node) => statusOf(node)),
          this.#nodes.map((node) =>
            statusOf(node) === UNCOMPUTED
              ? undefined
              : valueNumber(internal(node).value),
          ),
          this.#nodes.map((node, index) =>
            statusOf(node) === UNCOMPUTED ? [] : at(this.#recordedReads, index),
          ),
          this.#nodes.map((node) => this.#engineDerivedEdges(node)),
          this.#nodes.map((node) => isMarkedBeyond(node)),
        );
        break;
    }
    this.#actualRuns.length = 0;
    this.#model.predictedRuns.length = 0;
    this.log.push(
      this.#nodes
        .map(
          (node, index) =>
            `${statusOf(node)}:${statusOf(node) === UNCOMPUTED ? '-' : valueNumber(internal(node).value)}:${at(this.#retains, index).length}`,
        )
        .join(' '),
    );
  }
}

/** mulberry32. */
function prng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * `mixed` reads a source or any lower node. `deep-chains` has more nodes and
 * reads the node just below half the time, so a branch switch can link a node
 * to a long chain during a flush.
 */
type GraphShape = 'mixed' | 'deep-chains';

/**
 * The share of seeds whose graph is `deep-chains`. The choice is drawn from a
 * second generator seeded from the seed, so a `mixed` seed's case does not
 * depend on it.
 */
const DEEP_CHAIN_SHARE = 0.25;

/**
 * The generator tracks the retains it has generated and not released, and
 * releases each of them once, the outstanding ones at the end of the case. A
 * read picks the node of one of those retains, or any node a quarter of the
 * time.
 */
function generateCase(seed: number): Case {
  const random = prng(seed);
  const integer = (bound: number): number => Math.floor(random() * bound);
  const pick = <T>(choices: ReadonlyArray<T>): T =>
    at(choices, integer(choices.length));
  const graphShape: GraphShape =
    prng(seed ^ 0x5bd1e995)() < DEEP_CHAIN_SHARE ? 'deep-chains' : 'mixed';
  const sourceCount = 2 + integer(4);
  const nodeCount =
    graphShape === 'deep-chains' ? 10 + integer(14) : 3 + integer(12);
  const mixedRead = (node: number): Read =>
    node === 0 || random() < 0.4
      ? { kind: 'source', source: integer(sourceCount) }
      : { kind: 'derived', node: integer(node) };
  const readFor = (node: number): Read => {
    switch (graphShape) {
      case 'mixed':
        return mixedRead(node);
      case 'deep-chains':
        return node !== 0 && random() < 0.5
          ? { kind: 'derived', node: node - 1 }
          : mixedRead(node);
    }
  };
  const graph: GraphSpec = {
    sourceValues: Array.from({ length: sourceCount }, () => integer(4)),
    nodes: Array.from({ length: nodeCount }, (_, node) => ({
      selector: readFor(node),
      branches: Array.from({ length: 3 }, () =>
        Array.from({ length: integer(4) }, () => readFor(node)),
      ),
      modulus: pick([2, 3, 1000]),
      shape: pick<ResultShape>(['number', 'reused-object', 'fresh-object']),
    })),
  };
  const retained: Array<number> = [];
  let nextCallbackId = 0;
  const retain = (): RetainStep => {
    const node = integer(nodeCount);
    retained.push(node);
    return { kind: 'retain', node };
  };
  const release = (): ReleaseStep | RetainStep => {
    if (retained.length === 0) {
      return retain();
    }
    const index = integer(retained.length);
    const node = at(retained, index);
    retained.splice(index, 1);
    return { kind: 'release', node };
  };
  const readTarget = (): number =>
    retained.length === 0 || random() < 0.25
      ? integer(nodeCount)
      : pick(retained);
  const read = (): ReadStep => ({ kind: 'read', node: readTarget() });
  const write = (): WriteStep => ({
    kind: 'write',
    source: integer(sourceCount),
    value: integer(4),
  });
  const batch = (depth: number): Batch => ({
    kind: 'batch',
    items: Array.from({ length: 1 + integer(5) }, (): BatchItem => {
      const roll = random();
      if (roll < 0.45) {
        return write();
      }
      if (roll < 0.7) {
        return write();
      }
      if (roll < 0.8) {
        return retained.length === 0 ? write() : release();
      }
      if (roll < 0.9 && depth < 2) {
        return batch(depth + 1);
      }
      return { kind: 'single', write: write() };
    }),
  });
  const op = (): Op => {
    const roll = random();
    if (roll < 0.03) {
      return { kind: 'disposeAll' };
    }
    if (roll < 0.3) {
      return write();
    }
    if (roll < 0.48) {
      return read();
    }
    if (roll < 0.6) {
      return retain();
    }
    if (roll < 0.7) {
      return release();
    }
    if (roll < 0.82) {
      const node = integer(nodeCount);
      if (random() < 0.15) {
        return { kind: 'replaceOnChangeRetain', node, callback: undefined };
      }
      const id = nextCallbackId;
      nextCallbackId += 1;
      return {
        kind: 'replaceOnChangeRetain',
        node,
        callback: {
          id,
          reads: Array.from({ length: integer(3) }, readTarget),
        },
      };
    }
    return batch(0);
  };
  const ops = Array.from({ length: 40 + integer(80) }, op);
  const releases: Array<Op> = [];
  while (retained.length !== 0) {
    releases.push(release());
  }
  return { graph, ops: [...ops, ...releases] };
}

function runCase(
  engine: Engine,
  testCase: Case,
  mode: Mode,
  schedulerSetting: SchedulerSetting,
): ReadonlyArray<string> {
  engine.scheduler.schedule =
    schedulerSetting === 'sync' ? engine.disposeAll : () => {};
  const run = new Run(engine, testCase.graph, mode, schedulerSetting);
  testCase.ops.forEach((op, index) => run.execute(op, index));
  run.finish();
  return run.log;
}

function compareLogs(
  single: ReadonlyArray<string>,
  batched: ReadonlyArray<string>,
): void {
  const length = Math.max(single.length, batched.length);
  for (let index = 0; index < length; index += 1) {
    if (single[index] !== batched[index]) {
      const context = single.slice(Math.max(0, index - 8), index).join(' | ');
      throw new Error(
        `single calls and one-op batches differ at log entry ${index}: ${single[index]} against ${batched[index]}, after ${context}`,
      );
    }
  }
}

/**
 * A failed run can throw inside a computation, a batch, or an `onChange`, so
 * each case gets a new module instance of the engine.
 */
async function failureOf(testCase: Case): Promise<string | undefined> {
  vi.resetModules();
  const engine: Engine = await import('../src/index');
  try {
    for (const schedulerSetting of ['sync', 'deferred'] as const) {
      compareLogs(
        runCase(engine, testCase, 'single', schedulerSetting),
        runCase(engine, testCase, 'one-op-batch', schedulerSetting),
      );
      runCase(engine, testCase, 'grouped', schedulerSetting);
    }
    return undefined;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

function* simplerItems(
  items: ReadonlyArray<BatchItem>,
): Generator<ReadonlyArray<BatchItem>> {
  for (let index = 0; index < items.length; index += 1) {
    yield [...items.slice(0, index), ...items.slice(index + 1)];
  }
  for (let index = 0; index < items.length; index += 1) {
    const item = at(items, index);
    if (item.kind === 'batch') {
      for (const simpler of simplerItems(item.items)) {
        yield [
          ...items.slice(0, index),
          { kind: 'batch', items: simpler },
          ...items.slice(index + 1),
        ];
      }
    }
  }
}

function* simplerOps(op: Op): Generator<Op> {
  switch (op.kind) {
    case 'batch':
      for (const items of simplerItems(op.items)) {
        yield { kind: 'batch', items };
      }
      break;
    case 'replaceOnChangeRetain': {
      const callback = op.callback;
      if (callback === undefined) {
        break;
      }
      for (let index = 0; index < callback.reads.length; index += 1) {
        yield {
          ...op,
          callback: {
            ...callback,
            reads: [
              ...callback.reads.slice(0, index),
              ...callback.reads.slice(index + 1),
            ],
          },
        };
      }
      break;
    }
    case 'write':
    case 'read':
    case 'retain':
    case 'release':
    case 'disposeAll':
      break;
  }
}

function* candidates(testCase: Case): Generator<Case> {
  const ops = testCase.ops;
  for (let size = ops.length >> 1; size >= 1; size >>= 1) {
    for (let start = 0; start < ops.length; start += size) {
      yield {
        ...testCase,
        ops: [...ops.slice(0, start), ...ops.slice(start + size)],
      };
    }
  }
  for (let index = 0; index < ops.length; index += 1) {
    for (const simpler of simplerOps(at(ops, index))) {
      yield {
        ...testCase,
        ops: [...ops.slice(0, index), simpler, ...ops.slice(index + 1)],
      };
    }
  }
}

/** Removes operations, batch items, and callback reads while the case fails. */
async function shrink(
  testCase: Case,
  message: string,
): Promise<{ readonly testCase: Case; readonly message: string }> {
  let current = testCase;
  let currentMessage = message;
  let progress = true;
  while (progress) {
    progress = false;
    for (const candidate of candidates(current)) {
      const failure = await failureOf(candidate);
      if (failure !== undefined) {
        current = candidate;
        currentMessage = failure;
        progress = true;
        break;
      }
    }
  }
  return { testCase: current, message: currentMessage };
}

function integerFromEnvironment(name: string, fallback: number): number {
  const text = process.env[name];
  if (text === undefined || text === '') {
    return fallback;
  }
  const value = Number(text);
  if (!Number.isSafeInteger(value)) {
    throw new Error(`${name} is ${text}, which is not an integer`);
  }
  return value;
}

const seedCount = integerFromEnvironment('PICO_JS_RANDOM_SEEDS', 2000);
const baseSeed = integerFromEnvironment('PICO_JS_RANDOM_BASE_SEED', 1);

test(`random graphs and operations, seeds ${baseSeed} to ${baseSeed + seedCount - 1}`, async () => {
  for (let seed = baseSeed; seed < baseSeed + seedCount; seed += 1) {
    const testCase = generateCase(seed);
    const failure = await failureOf(testCase);
    if (failure !== undefined) {
      const minimal = await shrink(testCase, failure);
      throw new Error(
        `seed ${seed} fails: ${failure}\n\nminimal failing case (${minimal.testCase.ops.length} of ${testCase.ops.length} operations): ${minimal.message}\n${JSON.stringify(minimal.testCase)}`,
      );
    }
  }
});
