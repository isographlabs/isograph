import { expect, test } from 'vitest';
import * as pico from '../src/index';
import { internal } from '../test/internal';
import {
  ALL_RETAIN_OPTIONS,
  describeFinding,
  search,
  type Bounds,
  type Engine,
  type Run,
} from './checker';

/*
 * PICO_EXHAUSTIVE_BOUND selects the bound: `ci` (the default, about 40 s),
 * `medium`, `full`, or a JSON object of Bounds, or a JSON array of them.
 * PICO_EXHAUSTIVE_SHARD and PICO_EXHAUSTIVE_SHARDS run one shard of the graphs.
 */

const base = {
  maxSources: 2,
  maxBranchReads: 2,
  setupOrders: ['desc', 'asc'],
  maxBatchDepth: 2,
  maxRetainsPerNode: 2,
  disposeNext: false,
  contractViolations: false,
} as const;

const ABSTRACT_RETAIN_OPTIONS: Bounds['retainOptions'] = [[], [false], [true]];

const PRESETS: Record<string, ReadonlyArray<Bounds>> = {
  ci: [
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 3,
      dynamic: true,
      modes: 'oneCut',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 2,
    },
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 3,
      dynamic: false,
      modes: 'oneCut',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 3,
    },
    {
      ...base,
      minDerived: 3,
      maxDerived: 3,
      maxReads: 3,
      dynamic: false,
      modes: 'inj',
      retainOptions: ABSTRACT_RETAIN_OPTIONS,
      maxOps: 1,
    },
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 2,
      dynamic: false,
      modes: 'inj',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 2,
      disposeNext: true,
    },
  ],
  medium: [
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 3,
      dynamic: true,
      modes: 'all',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 3,
    },
    {
      ...base,
      minDerived: 3,
      maxDerived: 3,
      maxReads: 3,
      dynamic: false,
      modes: 'oneCut',
      retainOptions: ABSTRACT_RETAIN_OPTIONS,
      maxOps: 2,
    },
  ],
  full: [
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 3,
      dynamic: true,
      modes: 'all',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 4,
    },
    {
      ...base,
      minDerived: 1,
      maxDerived: 2,
      maxReads: 3,
      dynamic: true,
      modes: 'all',
      retainOptions: ALL_RETAIN_OPTIONS,
      maxOps: 3,
      disposeNext: true,
    },
    {
      ...base,
      minDerived: 3,
      maxDerived: 3,
      maxReads: 3,
      dynamic: false,
      modes: 'oneCut',
      retainOptions: ABSTRACT_RETAIN_OPTIONS,
      maxOps: 3,
    },
    {
      ...base,
      minDerived: 3,
      maxDerived: 3,
      maxReads: 3,
      dynamic: true,
      modes: 'inj',
      retainOptions: ABSTRACT_RETAIN_OPTIONS,
      maxOps: 2,
    },
    {
      ...base,
      minDerived: 4,
      maxDerived: 4,
      maxReads: 2,
      dynamic: false,
      modes: 'inj',
      retainOptions: ABSTRACT_RETAIN_OPTIONS,
      maxOps: 2,
    },
  ],
};

function bounds(): ReadonlyArray<Bounds> {
  const value = process.env['PICO_EXHAUSTIVE_BOUND'] ?? 'ci';
  const preset = PRESETS[value];
  if (preset !== undefined) {
    return preset;
  }
  const parsed: Bounds | Array<Bounds> = JSON.parse(value);
  return Array.isArray(parsed) ? parsed : [parsed];
}

/** The public API as the checker's Engine; the engine's module state is shared by every run. */
const engine: Engine = {
  createSource: (value) => pico.createSourceNode(value),
  createDerived: (computation) => pico.createDerivedNode(computation),
  readSource: (callingNode, source) => pico.readSource(callingNode, source),
  readDerived: (callingNode, node) => pico.readDerived(callingNode, node),
  retainDerivedNode: (node, onChange) => pico.retainDerivedNode(node, onChange),
  releaseRetain: (retain) => pico.releaseRetain(retain),
  readDerivedNode: (node) => pico.readDerivedNode(node),
  startBatch: () => pico.startWriteBatch(),
  endBatch: () => pico.endWriteBatch(),
  writeSource: (source, value) => pico.writeSource(source, value),
  disposeNext: () => pico.disposeNext(),
  disposeAll: () => pico.disposeAll(),
  setSchedule: (schedule) => {
    pico.scheduler.schedule = schedule;
  },
  internal: (node) => internal(node),
};

/** Returns the shared engine to an idle state: no open batch, no retain, an empty cleanup queue. */
function releaseEngine(run: Run): void {
  while (run.depth > 0) {
    run.depth -= 1;
    pico.endWriteBatch();
  }
  for (const records of run.live) {
    for (const record of records.splice(0)) {
      pico.releaseRetain(record.handle);
    }
  }
  pico.disposeAll();
}

test('every value a computation, a top-level read, or an onChange receives is current', () => {
  const shard = process.env['PICO_EXHAUSTIVE_SHARD'];
  const shards = process.env['PICO_EXHAUSTIVE_SHARDS'];
  const failing = new Map<string, string>();
  for (const b of bounds()) {
    const { findings, stats } = search(() => engine, b, {
      ...(shards === undefined
        ? {}
        : { shard: Number(shard ?? 0), shards: Number(shards) }),
      releaseEngine,
    });
    console.log(`${JSON.stringify(b)}\n  ${JSON.stringify(stats)}`);
    for (const finding of findings.values()) {
      if (!failing.has(finding.signature)) {
        failing.set(finding.signature, describeFinding(finding));
      }
    }
  }
  expect([...failing.values()].join('\n\n')).toBe('');
});
