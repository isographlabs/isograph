import { at } from '../test/at';
import type {
  CallingNode,
  DerivedNode,
  Retain,
  SourceNode,
} from '../src/index';
import type { MutableDerivedNode } from '../src/nodes';

/*
 * Exhaustive bounded model checker for pico-js consistency.
 *
 * The checker enumerates small graphs, node kinds, and operation sequences,
 * runs each against the engine through its public API (plus `internal`, the engine's state of a
 * derived node), and
 * compares every value the engine hands out with a reference evaluator. The
 * reference evaluator recomputes a derived node from the current source
 * values by calling the node's computation with its reads redirected to the
 * reference evaluator. It shares no code with the engine.
 *
 * Checked points:
 * - every value a computation receives from `readSource` or `readDerived`;
 * - every value `readDerivedNode` returns, including the
 *   reads an `onChange` makes and the reads after the sequence ends;
 * - at the end of every outermost operation, the value of every `CLEAN` node
 *   (an internal check: no reader has yet received a `CLEAN` node's stale value);
 * - at the end of every outermost operation, that each retain with an
 *   `onChange` was called exactly once if its node's value changed since the
 *   retain last saw it, and not at all otherwise.
 */

/** The engine operations the checker uses. */
export type Engine = {
  createSource(value: number): SourceNode<number>;
  createDerived(
    computation: (
      callingNode: CallingNode,
      previous: number | undefined,
    ) => number,
  ): DerivedNode<number>;
  readSource(callingNode: CallingNode, source: SourceNode<number>): number;
  readDerived(callingNode: CallingNode, node: DerivedNode<number>): number;
  retainDerivedNode(
    node: DerivedNode<number>,
    onChange?: (value: number) => void,
  ): Retain<number>;
  releaseRetain(retain: Retain<number>): void;
  readDerivedNode(node: DerivedNode<number>): number;
  startBatch(): void;
  endBatch(): void;
  writeSource(source: SourceNode<number>, value: number): void;
  disposeNext(): boolean;
  disposeAll(): void;
  setSchedule(schedule: () => void): void;
  /** The engine's state of `node`, which the public `DerivedNode` type does not expose. */
  internal(node: DerivedNode<number>): MutableDerivedNode<number>;
};

export type Read =
  | { readonly kind: 's'; readonly index: number }
  | { readonly kind: 'd'; readonly index: number };

export type Reads =
  | { readonly kind: 'static'; readonly reads: ReadonlyArray<Read> }
  | {
      readonly kind: 'branch';
      readonly source: number;
      readonly even: ReadonlyArray<Read>;
      readonly odd: ReadonlyArray<Read>;
    };

/** `inj` returns a value that differs for every distinct input list; `cut` returns the parity of the sum of the inputs. */
export type Mode = 'inj' | 'cut';

export type NodeSpec = {
  readonly reads: Reads;
  readonly mode: Mode;
};

export type Graph = {
  readonly sourceCount: number;
  readonly nodes: ReadonlyArray<NodeSpec>;
};

/** The retains the setup makes on each node, `true` for a retain with an `onChange`. */
export type Setup = {
  readonly retains: ReadonlyArray<ReadonlyArray<boolean>>;
  readonly order: 'desc' | 'asc';
};

export type Op =
  | { readonly k: 'write'; readonly s: number; readonly delta: 0 | 1 | 2 }
  | { readonly k: 'start' }
  | { readonly k: 'end' }
  | { readonly k: 'read'; readonly node: number }
  | { readonly k: 'retain'; readonly node: number; readonly onChange: boolean }
  | { readonly k: 'release'; readonly node: number; readonly onChange: boolean }
  | { readonly k: 'releaseAgain'; readonly node: number }
  | { readonly k: 'disposeNext' }
  | { readonly k: 'disposeAll' };

export type ViolationKind =
  | 'computation-input'
  | 'top-level-read'
  | 'onChange-read'
  | 'onChange-missed'
  | 'onChange-duplicate'
  | 'onChange-spurious'
  | 'onChange-in-batch'
  | 'clean-stale'
  | 'leak'
  | 'engine-exception';

export type Violation = {
  readonly kind: ViolationKind;
  /** The operation that was running: an Op kind, 'setup', or 'drain-*'. */
  readonly phase: string;
  readonly inBatch: boolean;
  readonly contractBroken: boolean;
  /** The node whose value was wrong, or whose computation received a wrong value. */
  readonly node: number;
  readonly detail: string;
  /** A short classification used to group violations. */
  readonly tag: string;
};

/** Thrown by a computation when a run has recomputed more than RUNAWAY_LIMIT times, which only an engine that loops does. */
class Runaway extends Error {}
const RUNAWAY_LIMIT = 2000;

const internTable = new Map<string, number>();
function intern(key: string): number {
  let value = internTable.get(key);
  if (value === undefined) {
    value = internTable.size + 2;
    internTable.set(key, value);
  }
  return value;
}

/** The computation of node `index`: reads through `read`, returns the node's value. */
export function evaluate(
  node: NodeSpec,
  index: number,
  read: (r: Read) => number,
): number {
  const inputs: Array<number> = [];
  let list: ReadonlyArray<Read>;
  if (node.reads.kind === 'static') {
    list = node.reads.reads;
  } else {
    const selector = read({ kind: 's', index: node.reads.source });
    inputs.push(selector);
    list = selector % 2 === 0 ? node.reads.even : node.reads.odd;
  }
  for (const r of list) {
    inputs.push(read(r));
  }
  return node.mode === 'inj'
    ? intern(`${index}:${inputs.join(',')}`)
    : inputs.reduce((a, b) => a + b, 0) % 2;
}

/** The reference evaluator: the value of every derived node, computed from `sourceValues` alone. */
export function referenceValues(
  graph: Graph,
  sourceValues: ReadonlyArray<number>,
): Array<number> {
  const values: Array<number> = [];
  graph.nodes.forEach((node, index) => {
    values.push(
      evaluate(node, index, (r) =>
        r.kind === 's' ? at(sourceValues, r.index) : at(values, r.index),
      ),
    );
  });
  return values;
}

export function readName(r: Read): string {
  return `${r.kind}${r.index + 1}`;
}

export function describeGraph(graph: Graph): string {
  return graph.nodes
    .map((node, index) => {
      const name = `d${index + 1}${node.mode === 'cut' ? '[parity]' : ''}`;
      if (node.reads.kind === 'static') {
        return `${name} -> ${node.reads.reads.map(readName).join(', ')}`;
      }
      const sel = `s${node.reads.source + 1}`;
      return `${name} -> ${sel}, then ${sel} even: [${node.reads.even.map(readName).join(', ')}], odd: [${node.reads.odd.map(readName).join(', ')}]`;
    })
    .join('; ');
}

export function describeOp(op: Op): string {
  switch (op.k) {
    case 'write':
      return `write s${op.s + 1} (+${op.delta})`;
    case 'start':
      return 'startBatch';
    case 'end':
      return 'endBatch';
    case 'read':
      return `readDerivedNode d${op.node + 1}`;
    case 'retain':
      return `retain d${op.node + 1}${op.onChange ? ' with onChange' : ''}`;
    case 'release':
      return `release d${op.node + 1}${op.onChange ? ' (onChange retain)' : ''}`;
    case 'releaseAgain':
      return `release a released retain of d${op.node + 1}`;
    case 'disposeNext':
      return 'disposeNext';
    case 'disposeAll':
      return 'disposeAll';
  }
}

export function describeSetup(setup: Setup): string {
  const order = setup.retains.map((_, i) => i);
  if (setup.order === 'desc') {
    order.reverse();
  }
  const parts: Array<string> = [];
  for (const i of order) {
    for (const onChange of at(setup.retains, i)) {
      parts.push(`retain d${i + 1}${onChange ? ' with onChange' : ''}`);
    }
  }
  return parts.length === 0 ? '(none)' : parts.join(', ');
}

type RetainRecord = {
  readonly handle: Retain<number>;
  readonly onChange: boolean;
  lastSeen: number;
  calls: number;
};

/** One run: a fresh graph on an engine, the setup, and a sequence of ops. */
export class Run {
  readonly sources: Array<SourceNode<number>> = [];
  readonly nodes: Array<DerivedNode<number>> = [];
  readonly sourceValues: Array<number>;
  readonly live: Array<Array<RetainRecord>>;
  readonly released: Array<Array<RetainRecord>>;
  readonly violations: Array<Violation> = [];
  readonly snapshots: Array<Array<number>> = [];
  readonly retainOfHandle = new Map<object, RetainRecord>();
  depth = 0;
  contractBroken = false;
  aborted = false;
  phase = 'setup';
  recording = true;
  /** The state key at the outermost startBatch, and the ops since then. */
  batchStartKey = '';
  batchStartValues: Array<number> = [];
  batchLog = '';
  cleanupLog = '';

  constructor(
    readonly engine: Engine,
    readonly graph: Graph,
  ) {
    engine.setSchedule(() => {});
    this.sourceValues = Array.from({ length: graph.sourceCount }, () => 0);
    this.live = graph.nodes.map(() => []);
    this.released = graph.nodes.map(() => []);
    for (let s = 0; s < graph.sourceCount; s += 1) {
      this.sources.push(engine.createSource(0));
    }
    graph.nodes.forEach((_, index) => {
      this.nodes.push(
        engine.createDerived((callingNode) => this.compute(index, callingNode)),
      );
    });
    this.snapshots.push([...this.sourceValues]);
  }

  reference(): Array<number> {
    return referenceValues(this.graph, this.sourceValues);
  }

  violation(
    kind: ViolationKind,
    node: number,
    detail: string,
    tag: string,
  ): void {
    if (!this.recording) {
      return;
    }
    this.violations.push({
      kind,
      phase: this.phase,
      inBatch: this.depth > 0,
      contractBroken: this.contractBroken,
      node,
      detail,
      tag,
    });
  }

  nodeClass(index: number): string {
    const node = this.engine.internal(at(this.nodes, index));
    const observed = node.evaluation === 1;
    const status = ['UNCOMPUTED', 'DIRTY', 'MAYBE_DIRTY', 'CLEAN'][node.status];
    return `${observed ? 'with a callback' : 'lazy'}/${status}`;
  }

  computeCount = 0;
  /** The reads of each node's last computation that returned, for telling a new dependency from a kept one. */
  readonly lastReads: Array<ReadonlyArray<string>> = [];

  compute(index: number, callingNode: CallingNode): number {
    this.computeCount += 1;
    if (this.computeCount > RUNAWAY_LIMIT) {
      throw new Runaway();
    }
    if (callingNode !== this.engine.internal(at(this.nodes, index))) {
      this.violation(
        'engine-exception',
        index,
        'wrong callingNode',
        'callingNode',
      );
    }
    const seen: Array<{ r: Read; value: number; before: string }> = [];
    const spec = at(this.graph.nodes, index);
    const value = evaluate(spec, index, (r) => {
      if (r.kind === 's') {
        const v = this.engine.readSource(
          callingNode,
          at(this.sources, r.index),
        );
        seen.push({ r, value: v, before: 'source' });
        return v;
      }
      const before = this.nodeClass(r.index);
      const v = this.engine.readDerived(callingNode, at(this.nodes, r.index));
      seen.push({ r, value: v, before });
      return v;
    });
    const previousReads = this.lastReads[index];
    this.lastReads[index] = seen.map((x) => readName(x.r));
    const want = this.reference();
    const stale = seen.filter(
      (x) =>
        x.value !==
        (x.r.kind === 's' ? this.sourceValues[x.r.index] : want[x.r.index]),
    );
    if (stale.length > 0) {
      const consistent = this.snapshots.some((snapshot) => {
        const ref = referenceValues(this.graph, snapshot);
        return seen.every(
          (x) =>
            x.value ===
            (x.r.kind === 's' ? snapshot[x.r.index] : ref[x.r.index]),
        );
      });
      const self = this.nodeClass(index).split('/')[0];
      const edge = stale.some(
        (x) =>
          previousReads === undefined || !previousReads.includes(readName(x.r)),
      )
        ? 'new-dependency'
        : 'kept-dependency';
      this.violation(
        'computation-input',
        index,
        `d${index + 1} (${self}) read stale ${stale.map((x) => `${readName(x.r)} [${x.before} before the read]`).join(', ')}; inputs ${consistent ? 'match one earlier source state' : 'are mutually inconsistent'}`,
        `self=${self} stale=${[...new Set(stale.map((x) => x.before.split('/')[0] ?? ''))].sort((a, b) => a.localeCompare(b)).join('+')} ${edge} ${consistent ? 'consistent-old' : 'mixed'}`,
      );
    }
    return value;
  }

  checkTopLevel(
    kind: ViolationKind,
    index: number,
    got: unknown,
    before: string,
  ): void {
    const want = this.reference()[index];
    if (got !== want) {
      this.violation(
        kind,
        index,
        `d${index + 1} returned a stale value [${before} before the read]`,
        `stale=${before}`,
      );
    }
  }

  onChangeFor(
    index: number,
    record: () => RetainRecord,
  ): (value: number) => void {
    return (value) => {
      const rec = record();
      rec.calls += 1;
      if (this.depth > 0) {
        this.violation(
          'onChange-in-batch',
          index,
          `onChange of d${index + 1} ran inside a batch`,
          'in-batch',
        );
      }
      const saved = this.phase;
      this.phase = `${saved}/onChange`;
      const before = this.nodeClass(index);
      this.checkTopLevel('onChange-read', index, value, before);
      const got = this.engine.readDerivedNode(at(this.nodes, index));
      this.checkTopLevel('onChange-read', index, got, before);
      this.phase = saved;
    };
  }

  retain(index: number, onChange: boolean): void {
    let rec: RetainRecord | undefined;
    const handle = this.engine.retainDerivedNode(
      at(this.nodes, index),
      onChange
        ? this.onChangeFor(index, () => {
            if (rec === undefined) {
              throw new Error('onChange ran inside retainDerivedNode');
            }
            return rec;
          })
        : undefined,
    );
    // A retain made inside a batch has seen the value the node had when the outermost batch started.
    const lastSeen =
      this.depth > 0
        ? at(this.batchStartValues, index)
        : at(this.reference(), index);
    rec = { handle, onChange, lastSeen, calls: 0 };
    this.retainOfHandle.set(handle, rec);
    at(this.live, index).push(rec);
  }

  /** At the end of an outermost operation: onChange expectations and CLEAN nodes. */
  endOfOutermost(): void {
    const want = this.reference();
    this.live.forEach((records, index) => {
      for (const rec of records) {
        if (!rec.onChange) {
          continue;
        }
        const changed = want[index] !== rec.lastSeen;
        if (changed && rec.calls === 0) {
          this.violation(
            'onChange-missed',
            index,
            `onChange of d${index + 1} not called although its value changed`,
            this.nodeClass(index),
          );
        } else if (rec.calls > 1) {
          this.violation(
            'onChange-duplicate',
            index,
            `onChange of d${index + 1} called ${rec.calls} times`,
            'dup',
          );
        } else if (!changed && rec.calls > 0) {
          this.violation(
            'onChange-spurious',
            index,
            `onChange of d${index + 1} called although its value did not change`,
            'spurious',
          );
        }
        rec.lastSeen = at(want, index);
        rec.calls = 0;
      }
    });
    this.nodes.forEach((node, index) => {
      const state = this.engine.internal(node);
      if (state.status === 3 && state.value !== want[index]) {
        const observed = state.evaluation === 1;
        this.violation(
          'clean-stale',
          index,
          `d${index + 1} is CLEAN with a stale value after the operation`,
          observed ? 'with a callback' : 'lazy',
        );
      }
    });
  }

  write(s: number, delta: 0 | 1 | 2): void {
    this.sourceValues[s] = (at(this.sourceValues, s) + delta) % 4;
    this.snapshots.push([...this.sourceValues]);
    this.engine.writeSource(at(this.sources, s), at(this.sourceValues, s));
  }

  /** Runs one op; returns false if the engine threw an unexpected exception. */
  apply(op: Op, key: () => string): void {
    this.phase = op.k;
    const outermostBefore = this.depth === 0;
    try {
      this.applyInner(op, key);
    } catch (error) {
      this.violation(
        'engine-exception',
        -1,
        String(error),
        error instanceof Runaway ? 'runaway' : String(error).slice(0, 60),
      );
      this.aborted = true;
      return;
    }
    if (this.depth > 0) {
      this.batchLog += `${describeOp(op)};`;
    }
    if (op.k !== 'disposeAll' || this.depth > 0) {
      this.cleanupLog += `${describeOp(op)};`;
    } else {
      this.cleanupLog = '';
    }
    if (this.depth === 0 && (outermostBefore || op.k === 'end')) {
      this.endOfOutermost();
    }
  }

  private applyInner(op: Op, key: () => string): void {
    const e = this.engine;
    switch (op.k) {
      case 'write':
        if (op.delta === 0) {
          this.contractBroken = true;
        }
        this.write(op.s, op.delta);
        return;
      case 'start':
        if (this.depth === 0) {
          this.batchStartKey = key();
          this.batchStartValues = this.reference();
          this.batchLog = '';
        }
        this.depth += 1;
        e.startBatch();
        return;
      case 'end':
        if (this.depth === 0) {
          this.contractBroken = true;
        }
        this.depth -= 1;
        e.endBatch();
        return;
      case 'read': {
        if (this.depth > 0) {
          this.contractBroken = true;
        }
        const before = this.nodeClass(op.node);
        this.checkTopLevel(
          'top-level-read',
          op.node,
          e.readDerivedNode(at(this.nodes, op.node)),
          before,
        );
        return;
      }
      case 'retain':
        if (this.depth > 0) {
          this.contractBroken = true;
        }
        this.retain(op.node, op.onChange);
        return;
      case 'release': {
        const records = at(this.live, op.node);
        let position = -1;
        for (let i = records.length - 1; i >= 0; i -= 1) {
          if (at(records, i).onChange === op.onChange) {
            position = i;
            break;
          }
        }
        const rec = at(records.splice(position, 1), 0);
        at(this.released, op.node).push(rec);
        e.releaseRetain(rec.handle);
        return;
      }
      case 'releaseAgain': {
        this.contractBroken = true;
        const released = at(this.released, op.node);
        const rec = at(released, released.length - 1);
        e.releaseRetain(rec.handle);
        return;
      }
      case 'disposeNext':
        if (this.depth > 0) {
          this.contractBroken = true;
        }
        e.disposeNext();
        return;
      case 'disposeAll':
        if (this.depth > 0) {
          this.contractBroken = true;
        }
        e.disposeAll();
        return;
    }
  }

  setup(setup: Setup): void {
    const order = this.graph.nodes.map((_, i) => i);
    if (setup.order === 'desc') {
      order.reverse();
    }
    this.phase = 'setup';
    try {
      for (const i of order) {
        for (const onChange of at(setup.retains, i)) {
          this.retain(i, onChange);
        }
      }
    } catch (error) {
      this.violation(
        'engine-exception',
        -1,
        String(error),
        error instanceof Runaway ? 'runaway' : String(error).slice(0, 60),
      );
      this.aborted = true;
    }
    this.cleanupLog = `${describeSetup(setup)}|`;
  }

  /** Closes open batches, then reads every retained node and every node, then disposes and checks for leaks. */
  drain(): void {
    const e = this.engine;
    try {
      this.phase = 'drain-endBatch';
      while (this.depth > 0) {
        this.depth -= 1;
        e.endBatch();
        if (this.depth === 0) {
          this.endOfOutermost();
        }
      }
      this.phase = 'drain-readDerivedNode';
      this.nodes.forEach((node, index) => {
        const before = this.nodeClass(index);
        this.checkTopLevel(
          'top-level-read',
          index,
          e.readDerivedNode(node),
          before,
        );
        this.endOfOutermost();
      });
      this.phase = 'drain-disposeAll';
      e.disposeAll();
      this.nodes.forEach((node, index) => {
        const state = e.internal(node);
        if (
          at(this.live, index).length === 0 &&
          state.firstDependent === undefined &&
          state.status !== 0
        ) {
          this.violation(
            'leak',
            index,
            `d${index + 1} is unneeded and not UNCOMPUTED after disposeAll`,
            'leak',
          );
        }
      });
    } catch (error) {
      this.violation(
        'engine-exception',
        -1,
        String(error),
        error instanceof Runaway ? 'runaway' : String(error).slice(0, 60),
      );
      this.aborted = true;
    }
  }

  /** Every property reachable from the nodes, plus the harness state that later checks depend on. */
  stateKey(): string {
    const ids = new Map<object, number>();
    const parts: Array<string> = [];
    const walk = (object: object): void => {
      if (ids.has(object)) {
        parts.push(`#${ids.get(object)}`);
        return;
      }
      ids.set(object, ids.size);
      const rec = this.retainOfHandle.get(object);
      parts.push(
        rec === undefined
          ? '{'
          : `{R${rec.onChange ? 'o' : 'p'}${rec.lastSeen}`,
      );
      for (const [k, v] of Object.entries(object)) {
        parts.push(k);
        if (v == null || typeof v !== 'object') {
          parts.push(typeof v === 'function' ? 'f' : String(v));
        } else {
          walk(v);
        }
      }
      parts.push('}');
    };
    for (const s of this.sources) {
      walk(s);
    }
    for (const n of this.nodes) {
      walk(n);
    }
    parts.push(`|${this.depth}`);
    return parts.join(',');
  }
}

export type Bounds = {
  readonly maxDerived: number;
  readonly minDerived: number;
  readonly maxSources: number;
  /** Largest number of reads of a static node. */
  readonly maxReads: number;
  /** Whether one node per graph may branch on a source's parity. */
  readonly dynamic: boolean;
  readonly maxBranchReads: number;
  /** 'inj': every node injective; 'oneCut': also each graph with exactly one parity node. */
  readonly modes: 'inj' | 'oneCut' | 'all';
  /** The retain lists a node may have after the setup. */
  readonly retainOptions: ReadonlyArray<ReadonlyArray<boolean>>;
  readonly setupOrders: ReadonlyArray<'desc' | 'asc'>;
  readonly maxOps: number;
  readonly maxBatchDepth: number;
  readonly maxRetainsPerNode: number;
  readonly disposeNext: boolean;
  readonly contractViolations: boolean;
};

export const ALL_RETAIN_OPTIONS: ReadonlyArray<ReadonlyArray<boolean>> = [
  [],
  [false],
  [true],
  [false, false],
  [false, true],
  [true, true],
];

function orderedSubsets<T>(
  items: ReadonlyArray<T>,
  max: number,
  min: number,
): Array<Array<T>> {
  const out: Array<Array<T>> = [];
  const rec = (prefix: Array<T>, rest: ReadonlyArray<T>): void => {
    if (prefix.length >= min) {
      out.push(prefix);
    }
    if (prefix.length === max) {
      return;
    }
    rest.forEach((item, i) => {
      rec([...prefix, item], [...rest.slice(0, i), ...rest.slice(i + 1)]);
    });
  };
  rec([], items);
  return out;
}

type Shape = Array<Reads>;

function encodeReads(r: Read): string {
  return readName(r);
}

function encodeShape(shape: Shape, sourceCount: number): string {
  return (
    `${sourceCount}:` +
    shape
      .map((x) =>
        x.kind === 'static'
          ? x.reads.map(encodeReads).join('.')
          : `b${x.source}(${x.even.map(encodeReads).join('.')}|${x.odd.map(encodeReads).join('.')})`,
      )
      .join(';')
  );
}

function permutations(n: number): Array<Array<number>> {
  if (n === 0) {
    return [[]];
  }
  const out: Array<Array<number>> = [];
  for (const p of permutations(n - 1)) {
    for (let i = 0; i <= p.length; i += 1) {
      out.push([...p.slice(0, i), n - 1, ...p.slice(i)]);
    }
  }
  return out;
}

/** The relabeling of `shape` under a permutation of derived nodes and of sources, or undefined if a node would read a later one. */
function relabel(
  shape: Shape,
  perm: ReadonlyArray<number>,
  sourcePerm: ReadonlyArray<number>,
): Shape | undefined {
  // perm[old] = new index
  const placed: Array<{ readonly index: number; readonly reads: Reads }> = [];
  const map = (r: Read): Read =>
    r.kind === 's'
      ? { kind: 's', index: at(sourcePerm, r.index) }
      : { kind: 'd', index: at(perm, r.index) };
  for (const [old, reads] of shape.entries()) {
    const index = at(perm, old);
    const all =
      reads.kind === 'static' ? reads.reads : [...reads.even, ...reads.odd];
    if (all.some((r) => r.kind === 'd' && at(perm, r.index) >= index)) {
      return undefined;
    }
    placed.push({
      index,
      reads:
        reads.kind === 'static'
          ? { kind: 'static', reads: reads.reads.map(map) }
          : {
              kind: 'branch',
              source: at(sourcePerm, reads.source),
              even: reads.even.map(map),
              odd: reads.odd.map(map),
            },
    });
  }
  return placed.sort((a, b) => a.index - b.index).map((entry) => entry.reads);
}

function connected(shape: Shape, sourceCount: number): boolean {
  const parent = Array.from(
    { length: sourceCount + shape.length },
    (_, i) => i,
  );
  const find = (x: number): number => {
    const next = at(parent, x);
    return next === x ? x : (parent[x] = find(next));
  };
  const id = (r: Read) => (r.kind === 's' ? r.index : sourceCount + r.index);
  shape.forEach((reads, i) => {
    const all: ReadonlyArray<Read> =
      reads.kind === 'static'
        ? reads.reads
        : [{ kind: 's', index: reads.source }, ...reads.even, ...reads.odd];
    for (const r of all) {
      parent[find(id(r))] = find(sourceCount + i);
    }
  });
  const root = find(0);
  return parent.every((_, i) => find(i) === root);
}

/** Every canonical connected graph shape within the bounds, smallest first. */
export function* shapes(
  bounds: Bounds,
): Generator<{ shape: Shape; sourceCount: number }> {
  for (let n = bounds.minDerived; n <= bounds.maxDerived; n += 1) {
    for (
      let sourceCount = 1;
      sourceCount <= bounds.maxSources;
      sourceCount += 1
    ) {
      const perms = permutations(n);
      const sourcePerms = permutations(sourceCount);
      const rec = function* (
        prefix: Shape,
        dynamicUsed: boolean,
      ): Generator<Shape> {
        const i = prefix.length;
        if (i === n) {
          yield prefix;
          return;
        }
        const available: Array<Read> = [
          ...Array.from(
            { length: sourceCount },
            (_, s): Read => ({ kind: 's', index: s }),
          ),
          ...Array.from(
            { length: i },
            (_, d): Read => ({ kind: 'd', index: d }),
          ),
        ];
        for (const reads of orderedSubsets(available, bounds.maxReads, 1)) {
          yield* rec([...prefix, { kind: 'static', reads }], dynamicUsed);
        }
        if (bounds.dynamic && !dynamicUsed) {
          for (let s = 0; s < sourceCount; s += 1) {
            const rest = available.filter(
              (r) => !(r.kind === 's' && r.index === s),
            );
            const lists = orderedSubsets(rest, bounds.maxBranchReads, 0);
            for (const even of lists) {
              for (const odd of lists) {
                if (
                  encodeShape([{ kind: 'static', reads: even }], 0) ===
                  encodeShape([{ kind: 'static', reads: odd }], 0)
                ) {
                  continue;
                }
                yield* rec(
                  [...prefix, { kind: 'branch', source: s, even, odd }],
                  true,
                );
              }
            }
          }
        }
      };
      for (const shape of rec([], false)) {
        if (!connected(shape, sourceCount)) {
          continue;
        }
        const own = encodeShape(shape, sourceCount);
        let canonical = true;
        for (const p of perms) {
          for (const sp of sourcePerms) {
            const other = relabel(shape, p, sp);
            if (other !== undefined && encodeShape(other, sourceCount) < own) {
              canonical = false;
              break;
            }
          }
          if (!canonical) {
            break;
          }
        }
        if (canonical) {
          yield { shape, sourceCount };
        }
      }
    }
  }
}

/** Every graph (shape, modes) within the bounds. */
export function* graphs(bounds: Bounds): Generator<Graph> {
  for (const { shape, sourceCount } of shapes(bounds)) {
    const n = shape.length;
    const modeSets: Array<Array<Mode>> = [shape.map(() => 'inj')];
    if (bounds.modes === 'oneCut') {
      for (let i = 0; i < n; i += 1) {
        modeSets.push(shape.map((_, j) => (j === i ? 'cut' : 'inj')));
      }
    } else if (bounds.modes === 'all') {
      for (let mask = 1; mask < 1 << n; mask += 1) {
        modeSets.push(
          shape.map((_, j) => (((mask >> j) & 1) === 1 ? 'cut' : 'inj')),
        );
      }
    }
    for (const modes of modeSets) {
      yield {
        sourceCount,
        nodes: shape.map((reads, j) => ({ reads, mode: at(modes, j) })),
      };
    }
  }
}

export function* setups(graph: Graph, bounds: Bounds): Generator<Setup> {
  const n = graph.nodes.length;
  const rec = function* (
    prefix: Array<ReadonlyArray<boolean>>,
  ): Generator<Array<ReadonlyArray<boolean>>> {
    if (prefix.length === n) {
      yield prefix;
      return;
    }
    for (const option of bounds.retainOptions) {
      yield* rec([...prefix, option]);
    }
  };
  for (const retains of rec([])) {
    const anyRetain = retains.some((r) => r.length > 0);
    for (const order of bounds.setupOrders) {
      yield { retains, order };
      if (!anyRetain || n === 1) {
        break;
      }
    }
  }
}

/** The ops valid after `run` under the README's contracts (plus the contract-violating ones if asked). */
export function nextOps(
  run: Run,
  bounds: Bounds,
  path: ReadonlyArray<Op>,
): Array<Op> {
  const ops: Array<Op> = [];
  const n = run.nodes.length;
  const inBatch = run.depth > 0;
  // A write of +2 keeps the parity, so it differs from a write of +1 only for a parity node or a branch.
  const parityMatters = run.graph.nodes.some(
    (x) => x.mode === 'cut' || x.reads.kind === 'branch',
  );
  for (let s = 0; s < run.sources.length; s += 1) {
    ops.push({ k: 'write', s, delta: 1 });
    if (parityMatters) {
      ops.push({ k: 'write', s, delta: 2 });
    }
  }
  if (run.depth < bounds.maxBatchDepth) {
    ops.push({ k: 'start' });
  }
  if (inBatch) {
    ops.push({ k: 'end' });
  }
  for (let i = 0; i < n; i += 1) {
    if (!inBatch) {
      ops.push({ k: 'read', node: i });
    }
    if (!inBatch && at(run.live, i).length < bounds.maxRetainsPerNode) {
      ops.push(
        { k: 'retain', node: i, onChange: false },
        { k: 'retain', node: i, onChange: true },
      );
    }
    for (const onChange of [false, true]) {
      if (at(run.live, i).some((r) => r.onChange === onChange)) {
        ops.push({ k: 'release', node: i, onChange });
      }
    }
  }
  if (!inBatch) {
    ops.push({ k: 'disposeAll' });
    if (bounds.disposeNext) {
      ops.push({ k: 'disposeNext' });
    }
  }
  if (bounds.contractViolations) {
    for (let s = 0; s < run.sources.length; s += 1) {
      ops.push({ k: 'write', s, delta: 0 });
    }
    for (let i = 0; i < n; i += 1) {
      if (inBatch) {
        ops.push({ k: 'read', node: i });
        if (at(run.live, i).length < bounds.maxRetainsPerNode) {
          ops.push(
            { k: 'retain', node: i, onChange: false },
            { k: 'retain', node: i, onChange: true },
          );
        }
      }
      if (at(run.released, i).length > 0) {
        ops.push({ k: 'releaseAgain', node: i });
      }
    }
    if (inBatch) {
      ops.push({ k: 'disposeAll' }, { k: 'disposeNext' });
    } else {
      ops.push({ k: 'end' });
    }
  }
  void path;
  return ops;
}

export type Finding = {
  readonly signature: string;
  readonly violation: Violation;
  readonly graph: Graph;
  readonly setup: Setup;
  readonly path: ReadonlyArray<Op>;
  readonly size: ReadonlyArray<number>;
  count: number;
};

function sizeOf(
  graph: Graph,
  setup: Setup,
  path: ReadonlyArray<Op>,
): Array<number> {
  const reads = graph.nodes.reduce(
    (a, n) =>
      a +
      (n.reads.kind === 'static'
        ? n.reads.reads.length
        : 1 + n.reads.even.length + n.reads.odd.length),
    0,
  );
  return [
    graph.nodes.length,
    path.length,
    reads,
    graph.sourceCount,
    setup.retains.reduce((a, r) => a + r.length, 0),
    graph.nodes.filter((x) => x.reads.kind === 'branch').length,
    graph.nodes.filter((x) => x.mode === 'cut').length,
  ];
}

function smaller(a: ReadonlyArray<number>, b: ReadonlyArray<number>): boolean {
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) {
      return at(a, i) < at(b, i);
    }
  }
  return false;
}

export function signatureOf(v: Violation): string {
  const phase = v.phase.replace(/^drain-endBatch/, 'end');
  return [
    v.kind,
    phase,
    v.inBatch ? 'inBatch' : 'outside',
    v.contractBroken ? 'contractBroken' : 'contractKept',
    v.tag,
  ].join(' | ');
}

export type SearchStats = {
  families: number;
  configurations: number;
  states: number;
  replays: number;
};

/**
 * Explores every op sequence of up to `bounds.maxOps` ops from every setup of
 * `graph`. Two states with the same key have the same future, so a state seen
 * with at least as many ops left is not explored again. The key covers every
 * property reachable from the nodes, the harness state, the ops of the open
 * batch with the key at its start (the flush queue of the batch is module
 * state), and, when disposeNext is an op, the ops since the last disposeAll
 * (the cleanup queue is module state).
 */
export function searchGraph(
  makeEngine: () => Engine,
  graph: Graph,
  bounds: Bounds,
  report: (violation: Violation, setup: Setup, path: ReadonlyArray<Op>) => void,
  stats: SearchStats,
  releaseEngine: (run: Run) => void = () => {},
): void {
  const visited = new Map<string, number>();
  const memo = !bounds.contractViolations;
  stats.families += 1;
  for (const setup of setups(graph, bounds)) {
    stats.configurations += 1;
    const stack: Array<Array<Op>> = [[]];
    for (let path = stack.pop(); path !== undefined; path = stack.pop()) {
      stats.replays += 1;
      const run = new Run(makeEngine(), graph);
      run.recording = path.length === 0;
      run.setup(setup);
      for (let i = 0; i < path.length && !run.aborted; i += 1) {
        run.recording = i === path.length - 1;
        run.apply(at(path, i), () => run.stateKey());
      }
      run.recording = true;
      if (run.aborted) {
        for (const v of run.violations) {
          report(v, setup, path);
        }
        releaseEngine(run);
        continue;
      }
      const left = bounds.maxOps - path.length;
      if (memo) {
        let key = run.stateKey();
        if (run.depth > 0) {
          key += `|B${run.batchStartKey}|${run.batchLog}`;
        }
        if (bounds.disposeNext) {
          key += `|C${run.cleanupLog}`;
        }
        const seen = visited.get(key);
        if (seen !== undefined && seen >= left) {
          for (const v of run.violations) {
            report(v, setup, path);
          }
          releaseEngine(run);
          continue;
        }
        visited.set(key, left);
      }
      stats.states += 1;
      const children = left > 0 ? nextOps(run, bounds, path) : [];
      run.drain();
      for (const v of run.violations) {
        report(v, setup, path);
      }
      releaseEngine(run);
      if (run.aborted) {
        continue;
      }
      [...children].reverse().forEach((child) => stack.push([...path, child]));
    }
  }
}

/** Runs the search over every graph within the bounds and keeps the smallest example of each signature. */
export function search(
  makeEngine: () => Engine,
  bounds: Bounds,
  options: {
    readonly shard?: number;
    readonly shards?: number;
    readonly releaseEngine?: (run: Run) => void;
  } = {},
): { findings: Map<string, Finding>; stats: SearchStats } {
  const findings = new Map<string, Finding>();
  const stats: SearchStats = {
    families: 0,
    configurations: 0,
    states: 0,
    replays: 0,
  };
  let index = 0;
  for (const graph of graphs(bounds)) {
    const mine =
      options.shards === undefined || index % options.shards === options.shard;
    index += 1;
    if (!mine) {
      continue;
    }
    searchGraph(
      makeEngine,
      graph,
      bounds,
      (violation, setup, path) => {
        const signature = signatureOf(violation);
        const size = sizeOf(graph, setup, path);
        const existing = findings.get(signature);
        if (existing === undefined) {
          findings.set(signature, {
            signature,
            violation,
            graph,
            setup,
            path: [...path],
            size,
            count: 1,
          });
        } else {
          existing.count += 1;
          if (smaller(size, existing.size)) {
            findings.set(signature, {
              signature,
              violation,
              graph,
              setup,
              path: [...path],
              size,
              count: existing.count,
            });
          }
        }
      },
      stats,
      options.releaseEngine,
    );
  }
  return { findings, stats };
}

export function describeFinding(f: Finding): string {
  return [
    `${f.signature}  (${f.count} hits)`,
    `  graph: ${describeGraph(f.graph)}`,
    `  setup: ${describeSetup(f.setup)}`,
    `  ops: ${f.path.length === 0 ? '(none)' : f.path.map(describeOp).join(', ')}`,
    `  ${f.violation.detail} [phase ${f.violation.phase}]`,
  ].join('\n');
}
