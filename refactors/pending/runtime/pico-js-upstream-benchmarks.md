# pico-js: benchmarks the upstream reactivity suites lack

This doc proposes benchmark cases for the public reactivity suites. Each case is a graph shape and an operation sequence that no reviewed suite runs. For each case the doc gives the app pattern it models, the shape against a neutral API, what it measures, which suites lack it and why their nearest case does not cover it, and how alien-signals 3.2.1, `@preact/signals-core` 1.14.5, and pico-js differ mechanically. pico-js is `libs/pico-js/src` at engine commit `ba3daedf7` in `../i2-worktrees/pico-js-perf`.

## Conventions

- A snippet under `js-reactivity-benchmark/src/proposed/` is a proposed file. That directory does not exist upstream.
- Library citations are `alien-signals@3.2.1/esm/index.mjs` and `esm/system.mjs`, `@preact/signals-core@1.14.5/src/index.ts`, and pico-js `libs/pico-js/src/graph.ts` and `libs/pico-js/README.md` at `ba3daedf7`.
- A count marked "probe" is a number of computation runs from a scratch script in the session scratchpad, `upstream-probe/` (`probe.ts`, `nav.ts`, `dyn.ts`, `dup.ts`), bundled against those three versions. Each probe runs 10 operations. A probe counts runs and edges. It times nothing.
- Every timing is quoted from the file named next to it, with the engine commit that file measured. No timing in this doc was taken at `ba3daedf7`.
- Each mechanical claim cites library source or a probe. A claim that rests on neither is marked unverified.

## Suites reviewed

- js-reactivity-benchmark, milomg, commit `56eb45e8`, the clone at `libs/pico-js/node_modules/.cache/js-reactivity-benchmark` in the perf worktree. Families under `packages/core/src/benches/`: sbench (`sBench.ts`), kairo (`kairo/*.ts`, including `molBench.ts`), cellx (`cellxBench.ts`), and dynamic (`reactively/dynamicBench.ts`, `reactively/dependencyGraph.ts`, shapes in `config.ts`). pico-js-performance.md runs this suite.
- js-reactivity-benchmark, transitive-bullshit fork, commit `b792428`, scratchpad `external/transitive-bullshit_js-reactivity-benchmark/repo`. The same families under `src/`. Its `updateComputations*` cases read computeds with no effect (`sBench.ts:78-131`), its dynamic family has an `--unobserved` mode (`util/dependencyGraph.ts:57`), and its cellx runs depths 10, 20, and 30 unless `--stress` is passed (`cellxBench.ts:106`).
- TanStack/store, commit `3ee69b9`, `packages/store/tests/derived.bench.ts`. Each iteration builds a 7-node diamond, subscribes once, and writes once.
- artalar/reactive-computed-bench, commit `499c876`, `computers_bench.ts`. One 9-node graph with one effect, and each timed update writes `i % 2` (`computers_bench.ts:716-726`). `dynamic_deps_bench.ts` imports only `mol_wire_lib`, `@reatom/core`, a local reatom v4 build, `mobx`, and `@artalar/act`, so it has no alien or Preact arm.
- marcisbee/js-store-benchmark, commit `5c0e4db`, `react/{counter,fields,workspace}`. React in headless Chrome. The Preact target holds the whole workspace state in one signal and wraps each selector in a `computed` (`react/workspace/src/preact-signals.tsx:11-16`).
- jotai, commit `5fe0e82`, `benchmarks/*.ts` (benny). These cases run only jotai.
- KvashaIhor/state-management-benchmark and Andycko/gqlc-bench were not reviewed for gaps. The first counts React renders across Redux, Zustand, Jotai, Valtio, MobX, and Context (its README, "Configurations"). The second times GraphQL cache reads and writes (`benchmarks/01` to `20`). Neither has an alien or Preact arm.

## Neutral API

The suites' `ReactiveFramework` has `signal`, `computed(fn: () => T)`, `effect`, `withBatch`, `withBuild`, and, in milomg, `cleanup` (`util/reactiveFramework.ts:5-13` in both forks). The proposed cases need three more members: `previous` in `computed`, a disposer from `effect`, and `untracked`.

```ts
// external js-reactivity-benchmark/src/proposed/framework.ts
export interface ProposedFramework {
  signal<T>(initial: T): ProposedSignal<T>;
  computed<T>(fn: (previous: T | undefined) => T): ProposedComputed<T>;
  /** Returns the disposer. */
  effect(fn: () => void): () => void;
  untracked<T>(fn: () => T): T;
  batch(fn: () => void): void;
}

export interface ProposedSignal<T> {
  read(): T;
  write(value: T): void;
}

export interface ProposedComputed<T> {
  read(): T;
}

/** A computation whose cost the case controls. */
export function work(seed: number, iterations: number): number {
  let hash = seed;
  for (let i = 0; i < iterations; i++) {
    hash = (hash * 31 + i) | 0;
  }
  return hash;
}
```

- alien: `signal`, `computed(getter)`, `effect` (returns its disposer), `setActiveSub(undefined)` around an untracked read, `startBatch`/`endBatch`. The getter receives the old value (`index.mjs:199-200`).
- Preact: `.value`, `effect` (returns its disposer), `untracked`, `batch`. `computed` passes no argument (`index.ts:678` calls `this._fn()`), so the adapter keeps `previous` in a closure.
- pico-js: the shape of `libs/pico-js/bench/adapter.ts` at `9bf3c5a5d`. An effect is an eager derived node that the adapter retains and reads once, and its disposer calls `releaseDerivedNode`. A computed read outside a computation is retained on that first read. `untracked` is `readDerivedNode` of a retained node (README, Contracts). `write` compares with `source.value` before it writes (README, Contracts, first bullet).
- Every case reports computation runs per computed, from a counting build of each engine, next to its time. Cases 1 to 6 and 10 differ between engines in count, and a time alone does not show that.

## 1. Reader swap within one flush

Pattern: two panels exchange the derived values they show on one toggle (a split view whose sides swap, a compare view). Case 3 is the list form of the same mechanism.

```ts
// external js-reactivity-benchmark/src/proposed/readerSwap.ts
export function readerSwap(fw: ProposedFramework, cost: number): () => void {
  const s = fw.signal(1);
  const x = fw.signal(true);
  const a = fw.computed(() => work(s.read(), cost));
  const b = fw.computed(() => work(s.read() + 1, cost));
  fw.effect(() => void (x.read() ? a.read() : b.read()));
  fw.effect(() => void (x.read() ? b.read() : a.read()));
  let flag = true;
  return () => {
    flag = !flag;
    fw.batch(() => x.write(flag));
  };
}
```

A second variant makes `a` and `b` the heads of chains of depth `d`, so the cost of a lost value grows with `d`.

- Measures: time per toggle, and runs of `a` and `b` per toggle. No source of `a` or `b` changes, so the minimum is 0.
- Gap: no case in either js-reactivity-benchmark fork has two readers exchange a computed in one flush. kairo `unstable` (`kairo/unstable.ts:12`) switches one reader between `double` and `inverse`, but `head` is both the switch and the input of both branches, so every switch requires a recompute and a lost value costs nothing extra. In the dynamic family, alien, Preact, and pico-js run the same number of computations in all six shapes at a tenth of their iterations (probe `dyn.ts`: 348,000, 115,423, 146,091, 72,956, 122,157, and 107,073 runs), so the family never charges an engine for a dropped value. TanStack and artalar graphs are static.
- alien: the first effect's run ends with `purgeDeps` (`index.mjs:246`), which unlinks its edge to `a`. `unlink` calls `unwatched` when `a.subs` empties (`system.mjs:83-84`). `unwatched` sets `a.flags = 1 | 16` (Mutable, Dirty) and unlinks every dependency of `a` (`index.mjs:39-45`). The second effect's read finds `a` Dirty and recomputes it (`index.mjs:273-277`). Probe: 1 run per toggle. With chains, `disposeAllDepsInReverse` reaches each computed dependency through `unlink`, so the whole private chain recomputes (from source; the probe used depth 1).
- Preact: the first effect's `cleanupSources` (`index.ts:565-617`) unsubscribes `a`. `a._unsubscribe` clears `TRACKING` and unsubscribes `a` from its sources, and keeps `_value` and `_sources` (`index.ts:716-735`). The second effect's `addDependency` subscribes `a` again, which walks `a`'s sources (`index.ts:699-714`). Its `_refresh` sees a new `globalVersion` and runs `needsToRecompute`, which compares the versions of `a`'s sources and finds no change (`index.ts:504-531`, `index.ts:646-697`). Probe: 0 runs. The cost is three walks of `a`'s source list per toggle.
- pico-js: the first effect's recompute moves its edge from `a` to `b` and pushes `a` onto its `disposalCandidates` (`graph.ts`, `readDerived`). The flush disposes candidates after its last refresh and skips a node that has a reader again (`graph.ts`, `flush`; README, Guarantees: "A derived node that a run stops reading, and that a later run started by the same `readDerivedNode` or flush reads, keeps its value and does not run for that read"). Probe: 0 runs. The cost is one edge move per effect.

## 2. Reader swap through an untracked read

Pattern: an effect reads a derived value without subscribing to it (a `peek` of the current selection for logging or for an imperative call), while another effect subscribes to a node that value drops.

```ts
// external js-reactivity-benchmark/src/proposed/readerSwapUntracked.ts
export function readerSwapUntracked(
  fw: ProposedFramework,
  cost: number,
): () => void {
  const s = fw.signal(1);
  const x = fw.signal(true);
  const a = fw.computed(() => work(s.read(), cost));
  const b = fw.computed(() => work(s.read() + 1, cost));
  const r = fw.computed(() => (x.read() ? a.read() : b.read()));
  fw.effect(() => {
    x.read();
    fw.untracked(() => r.read());
  });
  fw.effect(() => void (x.read() ? b.read() : a.read()));
  let flag = true;
  return () => {
    flag = !flag;
    fw.batch(() => x.write(flag));
  };
}
```

- Measures: runs of `a` and `b` per toggle, and time.
- Gap: neither fork's `ReactiveFramework` has an untracked read (`util/reactiveFramework.ts:5-13`), so no case uses one.
- alien: `r` is read with no active subscriber, so `r` gains no subscriber, but its first run links its own dependencies (`index.mjs:284-293`). `r`'s recompute purges its edge to `a`, `a.subs` empties, and `unwatched` marks `a` Dirty. The second effect recomputes `a`. Probe: 1 run per toggle.
- Preact: `r` has no target, so it is not `TRACKING`, and `addDependency` subscribes a dependency only for a `TRACKING` context (`index.ts:246-248`). `a` was never subscribed through `r` and loses nothing. Probe: 0 runs.
- pico-js: `readDerivedNode` inside a computation disposes the candidates of its own run when it returns (`graph.ts`, `readDerivedNode`; README, Model: "A `readDerivedNode` inside a computation starts its own outermost run"). `a` is `Uncomputed` before the second effect refreshes, and that refresh computes `a` from scratch and allocates its edges. Probe: 1 run per toggle. pico-js loses this case to Preact and matches alien's count.

## 3. Regroup: a row moves between group nodes

Pattern: a kanban card moves between columns, or a list re-sort moves a row into another section. Each group node reads its members' row selectors.

```ts
// external js-reactivity-benchmark/src/proposed/regroup.ts
export function regroup(
  fw: ProposedFramework,
  rowCount: number,
  groupCount: number,
  cost: number,
): () => void {
  const data = Array.from({ length: rowCount }, (_, i) => fw.signal(i));
  const rows = data.map((d) => fw.computed(() => work(d.read(), cost)));
  const members = Array.from({ length: groupCount }, (_, g) =>
    fw.signal<ReadonlyArray<number>>(
      rows.map((_, id) => id).filter((id) => id % groupCount === g),
    ),
  );
  const groups = members.map((m) =>
    fw.computed(() => m.read().map((id) => rows[id]!.read())),
  );
  groups.forEach((group) => fw.effect(() => void group.read()));
  let step = 0;
  return () => {
    const from = members[step % groupCount]!;
    const to = members[(step + 1) % groupCount]!;
    step++;
    const [moved, ...rest] = from.read();
    fw.batch(() => {
      from.write(rest);
      to.write([moved!, ...to.read()]);
    });
  };
}
```

- Measures: row runs per move (minimum 0), and time.
- Gap: no upstream case moves a computed from one live reader to another (case 1). The marcisbee workspace's Preact target recomputes every selector from one state signal (`react/workspace/src/preact-signals.tsx:11-16`), so it has no row selector to move.
- Mechanics are those of case 1. Probe (10 rows, 2 groups, 10 moves): alien 10 row runs, Preact 0, pico-js 0. The probe writes the losing group's members first, so alien queues and runs the losing group's effect first. If the gaining group's effect ran first, the row would still have a subscriber when the losing group drops it, `unwatched` would not run (`system.mjs:83-84`), and alien would not recompute the row. The case runs both write orders. Preact and pico-js keep the row in either order.
- pico-js with eager rows: a queued eager node that no node reads and no holder retains when the flush reaches it is disposed, and a later read in that flush computes it again (README, Guarantees). When the batch also writes the moved row's data and the flush reaches the queued row after the losing group's refresh and before the gaining group's, the row is computed from scratch instead of recomputed.

## 4. Reorder a dependency list

Pattern: a list re-sort that keeps its members, or a page prepended to a feed.

```ts
// external js-reactivity-benchmark/src/proposed/reorder.ts
export function reorder(
  fw: ProposedFramework,
  itemCount: number,
  shift: number,
): () => void {
  const data = Array.from({ length: itemCount }, (_, i) => fw.signal(i));
  const items = data.map((d) => fw.computed(() => d.read() * 2));
  const order = fw.signal<ReadonlyArray<number>>(items.map((_, id) => id));
  const list = fw.computed(() => order.read().map((id) => items[id]!.read()));
  fw.effect(() => void list.read());
  return () => {
    const current = order.read();
    fw.batch(() =>
      order.write([...current.slice(shift), ...current.slice(0, shift)]),
    );
  };
}
```

Variants: a full reversal, and a prepend of `k` new items.

- Measures: time per reorder, and edges allocated per reorder from the counting build.
- Gap: no upstream computation changes the order of its reads. A dynamic node skips one source and reads the rest in order (`reactively/dependencyGraph.ts:159-164` in milomg). gg2-benchmark-plan-review.md, "Scenarios that favor pico-js, and missing scenarios that disfavor it", lists the same gap for the earlier app plan.
- pico-js: `readDerived` reuses the edge at each read position and moves it to the node read there when the node differs, which allocates nothing (`graph.ts`, `readDerived`). A node moved away from is a disposal candidate and survives, because the same run reads it again.
- alien: `link` reuses the next edge only when it points at the same dependency (`system.mjs:23-28`), and otherwise allocates a link (`system.mjs:33-43`). `purgeDeps` unlinks the edges left after the run (`index.mjs:364-370`). A rotation therefore allocates about one link per moved read. This count is inferred from source and not probed.
- Preact: `addDependency` finds the previous run's edge through `signal._node` and moves it to the tail of the source list (`index.ts:249-283`), which allocates nothing.
- App: S3b (prepend 25 items) has p50 0.651 ms for pico-js, 0.741 ms for alien, and 0.783 ms for Preact at engine `b46f6a870` (`bench/app/RESULTS.md` in the scratchpad). That event also includes normalize and 25 mounts.

## 5. Branch flip across batches

Pattern: a tab switch, where each tab's panel is a chain of computeds over sources that do not change while the tab is hidden.

```ts
// external js-reactivity-benchmark/src/proposed/tabSwitch.ts
export function tabSwitch(
  fw: ProposedFramework,
  tabCount: number,
  depth: number,
): () => void {
  const panels = Array.from({ length: tabCount }, (_, t) => {
    const source = fw.signal(t);
    let head: ProposedComputed<number> = source;
    for (let level = 0; level < depth; level++) {
      const below = head;
      head = fw.computed(() => below.read() + 1);
    }
    return head;
  });
  const tab = fw.signal(0);
  fw.effect(() => void panels[tab.read()]!.read());
  let current = 0;
  return () => {
    current = (current + 1) % tabCount;
    fw.batch(() => tab.write(current));
  };
}
```

A second variant holds every panel alive (an effect per panel in alien and Preact, `retainDerivedNode` in pico-js) to measure what the keep-alive costs.

- Measures: runs per switch after the first visit to each tab (minimum 0), and time.
- Gap: kairo `unstable` flips every write, but the flip and the branch values come from the same `head` (`kairo/unstable.ts:12`). No upstream case returns to a branch whose inputs did not change. The dynamic family charges no engine for a dropped value (probe `dyn.ts`, case 1).
- alien: the abandoned chain is unwatched and each computed in it is marked Dirty and unlinked (`index.mjs:39-45`, `index.mjs:356-363`). Returning recomputes the chain. Probe (depth 1): 1 run per switch.
- Preact: `_unsubscribe` clears `TRACKING` and keeps values and sources (`index.ts:716-735`). Returning resubscribes and compares versions. Probe: 0 runs after the first computation.
- pico-js: "A computation whose branch flips ... disposes the branch it stopped reading and computes it from scratch when the flag flips back. A caller that retains both `b` and `c` keeps both computed." (README, Model). Probe: 1 run per switch. In `retain/RESULTS.md` (prototype `4c465dbe3`, two 100-node chains), a toggle costs 2,142 ns against 11.3 ns for the engine without lifetimes (`1c624e97e`), and 11.5 ns with both heads retained.
- pico-js and alien lose this case to Preact.

## 6. Navigation: unmount a screen, mount the next

Pattern: a route change. The old screen's components unsubscribe and the new screen's subscribe. Both read shared computeds (the viewer, a workspace, users shared by many rows).

```ts
// external js-reactivity-benchmark/src/proposed/navigate.ts
export function navigate(
  fw: ProposedFramework,
  sharedCount: number,
  readersPerScreen: number,
  order: 'unmountFirst' | 'mountFirst',
): () => void {
  const s = fw.signal(1);
  const shared = Array.from({ length: sharedCount }, (_, i) =>
    fw.computed(() => work(s.read() + i, 100)),
  );
  const mount = () =>
    Array.from({ length: readersPerScreen }, (_, i) =>
      fw.effect(() => void shared[i % sharedCount]!.read()),
    );
  let screen = mount();
  return () => {
    if (order === 'unmountFirst') {
      screen.forEach((dispose) => dispose());
      screen = mount();
    } else {
      const next = mount();
      screen.forEach((dispose) => dispose());
      screen = next;
    }
  };
}
```

- Measures: runs of shared computeds per navigation (minimum 0), and time.
- Gap: upstream lifetimes end all at once. milomg's sbench disposes the whole graph inside the timed region (`sBench.ts:96-98`), and the transitive-bullshit fork disposes outside it (`sBench.ts:28`, `sBench.ts:124`, README.md:91). No case disposes some observers while others that read the same computeds are created. jotai `sub-unsub` subscribes and unsubscribes primitive atoms with no derived atom and no write (`benchmarks/subscription.ts:15-28`).
- Probe `nav.ts` (one shared computed, 10 navigations): unmount first, alien 10 runs, Preact 0, pico-js 10. Mount first, 0 in all three.
- alien: the last disposer's unlink calls `unwatched`, which marks the computed Dirty (`index.mjs:39-45`).
- Preact: the value survives. With no write between the unsubscribe and the next read, `_refresh` returns at `this._globalVersion === globalVersion` (`index.ts:661-663`) after the resubscribe walk.
- pico-js: `releaseDerivedNode` disposes the node and the dependencies it leaves unneeded when the count reaches 0 and no node reads it (README, API). The deferral covers runs inside one `readDerivedNode` or flush, and two separate top-level calls do not share it.
- App: S2 at `b46f6a870`, whose pico-js had no lifetimes, ran 2,007.82 computations per navigation for alien against 1,476.2 for pico-js and Preact, "because c1 recomputes a computed that lost its last subscriber" (`bench/app/RESULTS.md`). At `9bf3c5a5d` the app's `engine-pico.ts` retains each resolver node from its first read until `deleteRecord` (that commit's message), so this count was not re-measured for `ba3daedf7`.

## 7. Observer churn on a live graph

Pattern: rows scroll into and out of a virtualized list while data keeps arriving.

```ts
// external js-reactivity-benchmark/src/proposed/churn.ts
export function churn(fw: ProposedFramework, rowCount: number): () => void {
  const s = fw.signal(0);
  const shared = fw.computed(() => s.read() * 2);
  return () => {
    const disposers = Array.from({ length: rowCount }, (_, i) => {
      const row = fw.computed(() => shared.read() + i);
      return fw.effect(() => void row.read());
    });
    fw.batch(() => s.write(s.read() + 1));
    disposers.forEach((dispose) => dispose());
  };
}
```

- Measures: time per cycle, and retained heap after many cycles with forced GC.
- Gap: as in case 6. sbench `createComputations` creates effects over fresh signals and disposes the whole root, with no write between creation and disposal (`sBench.ts:24-38` in milomg).
- `pico-js-where-it-wins.md`, section 7 (engine `686cb9419`, effects through `onDirty` and `disposeDerivedNode`): alien is 1.5 to 1.7 times faster at N from 10 to 1,000, with equal counts (2 edges and 2 computations per pair). At `ba3daedf7` an effect is a retained eager node and its disposal is a release with the cascade, and this case has not been timed there.
- pico-js loses this case to alien at `686cb9419`.

## 8. One batch writes many sources

Pattern: a response that normalizes into many records, a form reset, a bulk edit.

```ts
// external js-reactivity-benchmark/src/proposed/bulkWrite.ts
export function bulkWrite(
  fw: ProposedFramework,
  sourceCount: number,
  midCount: number,
  effectCount: number,
  writesPerBatch: number,
): () => void {
  const sources = Array.from({ length: sourceCount }, (_, i) => fw.signal(i));
  const mids = Array.from({ length: midCount }, (_, i) =>
    fw.computed(
      () =>
        sources[i % sourceCount]!.read() +
        sources[(i * 7 + 1) % sourceCount]!.read(),
    ),
  );
  for (let e = 0; e < effectCount; e++) {
    fw.effect(() => {
      for (let k = 0; k < 3; k++) {
        mids[(e * 3 + k) % midCount]!.read();
      }
    });
  }
  let next = sourceCount;
  return () =>
    fw.batch(() => {
      for (let w = 0; w < writesPerBatch; w++) {
        sources[(next + w) % sourceCount]!.write(next++);
      }
    });
}
```

Sweep `writesPerBatch` from 1 to `sourceCount` and `effectCount` from 1 to 1,000.

- Measures: time per batch, and marking visits per batch.
- Gap: the most writes in one upstream batch are cellx's 4 (`cellxBench.ts`, `withBatch` around 4 writes) and mol's 2 (`kairo/molBench.ts`). Every other kairo, sbench, and dynamic batch wraps one write. TanStack and artalar write once per iteration.
- Preact allocates a `BatchSnapshot` for the first write of each source in a batch (`index.ts:163-178`) and walks the targets of each restored source at the end (`index.ts:180-206`). alien allocates nothing per write in a batch and stops propagation at a node already Pending or Dirty (`system.mjs:88-141`). pico-js walks each written source's dependents and stops at a node already `Dirty` (README, Guarantees).
- `pico-js-where-it-wins.md`, section 9 (5 sources written, 100 computeds, `686cb9419`): batched, E=100, pico-js 3.30 µs, alien 4.63, Preact 5.39. E=1,000, alien 37.2 µs, pico-js 47.9, Preact 47.2.
- In `retain/RESULTS.md`, the eager-effect adapter costs +38% and +60% on cellx1000 and cellx2500 and +11% on broadPropagation against the onDirty adapter, which the doc attributes to queuing and verifying each marked effect in the flush.
- pico-js loses at 1,000 effects at `686cb9419`.

## 9. Repeated writes to one source in a batch

Pattern: drag or typing events coalesced into one frame.

```ts
// external js-reactivity-benchmark/src/proposed/coalescedWrites.ts
export function coalescedWrites(
  fw: ProposedFramework,
  writesPerBatch: number,
): () => void {
  const s = fw.signal(0);
  const c = fw.computed(() => s.read() * 2);
  fw.effect(() => void c.read());
  let next = 1;
  return () =>
    fw.batch(() => {
      for (let w = 0; w < writesPerBatch; w++) {
        s.write(next++);
      }
    });
}
```

- Measures: time per batch, and marking visits per batch.
- Gap: no upstream batch writes a source twice. mol writes `A` and `B` once each per batch (`molBench.ts:43-49` in the transitive-bullshit fork), and cellx writes four different sources.
- alien calls `propagate` on every write whose value differs from `pendingValue` (`index.mjs:301-313`). Preact runs its `_notify` loop on every write (`index.ts:461-484`), and `NOTIFIED` stops it at the first node. pico-js records that a write left every reader `Dirty` and skips the list on the next write (README, Design choices).
- `pico-js-where-it-wins.md`, section 5 (`686cb9419`): M=100, batched, observed, pico-js 134 ns, alien 282, Preact 302, with 3, 101, and 101 marking visits. Unbatched and observed, alien is 1.4 times faster than pico-js at every M.

## 10. Equal-value writes, and a write reverted in the same batch

Pattern: a refetch whose values equal the store, and an optimistic write that the same batch rolls back (a slider dragged back to its origin before the frame ends).

```ts
// external js-reactivity-benchmark/src/proposed/revert.ts
export function revert(fw: ProposedFramework, cost: number): () => void {
  const s = fw.signal(1);
  const c = fw.computed(() => work(s.read(), cost));
  const d = fw.computed(() => c.read() + 1);
  fw.effect(() => void d.read());
  return () =>
    fw.batch(() => {
      s.write(2);
      s.write(1);
    });
}

// external js-reactivity-benchmark/src/proposed/equalWrites.ts
export function equalWrites(
  fw: ProposedFramework,
  sourceCount: number,
): () => void {
  const sources = Array.from({ length: sourceCount }, (_, i) => fw.signal(i));
  const sum = fw.computed(() => sources.reduce((t, s) => t + s.read(), 0));
  fw.effect(() => void sum.read());
  return () =>
    fw.batch(() => sources.forEach((s, i) => s.write(i)));
}
```

- Measures: runs of `c`, `d`, and the effect per batch, and time.
- Gap: every timed write in kairo and dynamic differs from the source's last value, except the first write of a run, which can equal the initial value (sbench `update*` writes `i` from 0 to `sources[0]`, initialized to 0, `sBench.ts:235-244`). artalar writes `i % 2` and alternates (`computers_bench.ts:724`), and TanStack writes 2 over 1. No upstream batch writes a value back.
- Probe `revertInBatch` (10 batches): alien 0 runs of `c`, Preact 0, pico-js 10. In pico-js, `c` recomputes to its old value and cuts off, so `d` and the effect run 0 times in all three.
- alien commits `pendingValue` on read and reports a change only when it differs from `currentValue` (`index.mjs:208-211`).
- Preact's `reconcileBatchSnapshots` moves a restored source's targets to the new version, so they do not recompute (`index.ts:180-206`). The same probe against signals-core 1.13.0, the version the clone's own adapter resolves, ran 10 runs of `c`.
- pico-js has no snapshot. A computation runs when a source it read was written (README, Guarantees), and the caller compares each write only with the value at that moment (README, Contracts).
- Equal writes: alien (`index.mjs:303`), Preact (`index.ts:462`), and the pico-js caller each compare once and do nothing else. The case measures the cost of a write that changes nothing. In the app, S7 skips 51,921 writes per event (`bench/app/RESULTS.md`), and normalize dominates its time.
- pico-js loses the revert variant to alien and Preact 1.14.5.

## 11. Cutoff by returning previous, and cutoff position

Pattern: a selector that builds an object and returns the previous one when the fields are equal, and a formatter whose output often does not change (S8, a timestamp formatted to a day).

```ts
// external js-reactivity-benchmark/src/proposed/structuralCutoff.ts
export function structuralCutoff(
  fw: ProposedFramework,
  readerCount: number,
): () => void {
  const title = fw.signal('a');
  const updatedAt = fw.signal(0);
  const view = fw.computed<{ title: string; day: number }>((previous) => {
    const next = {
      title: title.read(),
      day: Math.floor(updatedAt.read() / 86_400_000),
    };
    return previous !== undefined &&
      previous.title === next.title &&
      previous.day === next.day
      ? previous
      : next;
  });
  for (let r = 0; r < readerCount; r++) {
    const reader = fw.computed(() => view.read().title.length + r);
    fw.effect(() => void reader.read());
  }
  let time = 0;
  return () => fw.batch(() => updatedAt.write((time += 1_000)));
}
```

A second shape is a chain of depth D with the constant-returning node at the first, middle, or last position.

- Measures: time per write, and runs below `view` (minimum 0).
- Gap: `computed(fn: () => T)` (`util/reactiveFramework.ts:8` in both forks) passes no previous value. The upstream cutoffs are `computed2` returning the constant 0 in `avoidablePropagation` (`kairo/avoidable.ts`), and `mux`, which returns a new object on every run (`kairo/mux.ts`), so its readers never cut off. No case returns `previous`, and no case varies the cutoff position. `deepPropagation` has no cutoff.
- All three skip the readers of a node whose new value is `===` the old one (`index.mjs:200`, `index.ts:679-687`, README, Guarantees). alien and pico-js pass the old value to the computation. A Preact adapter keeps it in a closure. In pico-js, marking stops at an eager node, so an eager `view` whose value did not change marks nothing above it (README, Model).
- `pico-js-where-it-wins.md`, section 4 (`686cb9419`), D=1,000, cutoff at the first node: lazy pico-js 12.0 µs, alien 10.6, Preact 18.8, eager pico-js 20 ns. Section 2, K=10,000 readers, none changing: eager pico-js 108 µs, alien 201, Preact 217.
- App S8 (500 timestamps that all cut off, `b46f6a870`): 501, 488, and 488 computations, p50 1.712, 1.933, and 2.238 ms (pico-js, alien, Preact).
- Lazy pico-js loses the first-position cutoff to alien by 7 to 13% at D of 100 or more (`pico-js-where-it-wins.md`, section 4).

## 12. Stale readers that are never read again

Pattern: retained screens. Their nodes were read once when the screen was visible and still read sources that later events write (S4b, a hot record read by every row of every retained screen).

```ts
// external js-reactivity-benchmark/src/proposed/staleReaders.ts
export function staleReaders(
  fw: ProposedFramework,
  readerCount: number,
  observedFraction: number,
): () => void {
  const s = fw.signal(0);
  const readers = Array.from({ length: readerCount }, (_, i) =>
    fw.computed(() => s.read() + i),
  );
  readers.forEach((reader) => reader.read());
  readers
    .slice(0, Math.round(readerCount * observedFraction))
    .forEach((reader) => fw.effect(() => void reader.read()));
  return () => fw.batch(() => s.write(s.read() + 1));
}
```

- Measures: time per write, and marking visits per write.
- Gap: in both forks the unread leaves of a dynamic graph are chosen before the first read (`reactively/dependencyGraph.ts:52` in milomg, `util/dependencyGraph.ts:56` in the transitive-bullshit fork) and are never read, so no engine ever links them. The `--unobserved` mode reads the same fixed set of leaves on every iteration (`util/dependencyGraph.ts:64-71`).
- Preact subscribes a dependency only for a `TRACKING` context (`index.ts:246-248`), so a write visits only observed readers. alien and pico-js keep every reader that was read once in the source's list and visit it on each write.
- `pico-js-where-it-wins.md`, section 2 (`686cb9419`), K=1,000, 10% observed: Preact 2.77 µs, pico-js 5.10, alien 7.61.
- `pico-js-losses.md`, `largeGraphSmallPull` (POC-B, alien 3.1.2, signals-core 1.13.0, v0 timings the doc marks untrusted): pico-js 0.68 ms, alien 0.83, Preact 0.06.
- App S4b (`b46f6a870`): pico-js ran 2,785.1 computations per event against 277.1, because eager ItemBadge nodes on unmounted retained screens recomputed in the flush, and p50 was 1.650 ms against 0.266 (alien) and 0.285 (Preact).
- pico-js loses this case to Preact.

## 13. Many writes, then one read, with no effect

Pattern: data that arrives while a view is hidden and is read once when the view returns.

```ts
// external js-reactivity-benchmark/src/proposed/writesThenPull.ts
export function writesThenPull(
  fw: ProposedFramework,
  writesBeforeRead: number,
): () => void {
  const sources = Array.from({ length: 100 }, (_, i) => fw.signal(i));
  const level1 = Array.from({ length: 1000 }, (_, i) =>
    fw.computed(
      () =>
        sources[i % 100]!.read() +
        sources[(i + 1) % 100]!.read() +
        sources[(i + 2) % 100]!.read(),
    ),
  );
  const level2 = Array.from({ length: 100 }, (_, i) =>
    fw.computed(() =>
      level1.slice(i * 10, i * 10 + 10).reduce((t, c) => t + c.read(), 0),
    ),
  );
  const root = fw.computed(() => level2.reduce((t, c) => t + c.read(), 0));
  root.read();
  let next = 100;
  return () => {
    for (let w = 0; w < writesBeforeRead; w++) {
      sources[w % 100]!.write(next++);
    }
    root.read();
  };
}
```

- Measures: time per iteration, marking visits, and verification visits.
- Gap: the transitive-bullshit `updateComputations*` cases and the `--unobserved` dynamic mode read the leaves after every write (`sBench.ts:99-102`, `util/dependencyGraph.ts:64-71`). milomg's sbench `update*` cases observe through effects. No case makes several writes before a read.
- `pico-js-where-it-wins.md`, section 8 (`686cb9419`, the graph above): W=1, pico-js 2.74 µs, alien 4.19, Preact 12.8, with Preact checking 3,846 edges on the read. W=1,000, pico-js 30.6 µs, alien 126, Preact 33.6, with 5,200 marking visits for pico-js and 35,099 for alien.
- `pico-js-losses.md`, `manyWritesThenPull` (100 writes to sources that all 1,000 nodes read, POC-B, v0): pico-js 21.5 ms, alien 50.3, Preact 13.5. pico-js loses that shape to Preact.

## 14. Fan-out sweep over the observed and changed fractions

Pattern: one source read by K selectors, of which a fraction is on screen and a fraction changes value on a write.

```ts
// external js-reactivity-benchmark/src/proposed/fanOutSweep.ts
export function fanOutSweep(
  fw: ProposedFramework,
  readerCount: number,
  observedFraction: number,
  changedFraction: number,
): () => void {
  const s = fw.signal(0);
  const changing = Math.round(readerCount * changedFraction);
  const readers = Array.from({ length: readerCount }, (_, i) =>
    fw.computed(() => (i < changing ? s.read() + i : (s.read(), 0))),
  );
  readers.forEach((reader) => reader.read());
  readers
    .slice(0, Math.round(readerCount * observedFraction))
    .forEach((reader) => fw.effect(() => void reader.read()));
  return () => fw.batch(() => s.write(s.read() + 1));
}
```

- Measures: time per write over K from 1 to 10,000, observed fraction 0, 0.1, and 1, and changed fraction 0, 0.1, 0.5, and 1.
- Gap: the upstream fan-out points are sbench `update1to1000` (1,000 effects on the source, `sBench.ts:321-334` in milomg), `update1to2` and `update1to4`, kairo `broadPropagation` (50 chains with an effect each), and `mux` (100 effects). All are observed fraction 1 and changed fraction 1, except `mux`, where each write changes one of 100 readers. No case varies either fraction.
- `pico-js-where-it-wins.md`, section 2 (`686cb9419`): observed 1, changed 1, K=10,000, alien 304 µs, pico-js 377, Preact 380. Observed 0.1, Preact wins from K=10 on. Changed 0.5 or less with eager readers, pico-js wins.
- pico-js loses the all-observed, all-changing point to alien, and the 10%-observed points to Preact.

## 15. Repeated reads of one dependency that are not adjacent

Pattern: list items that share an author record, and a selector that reads the same collection several times.

```ts
// external js-reactivity-benchmark/src/proposed/interleavedReads.ts
export function interleavedReads(
  fw: ProposedFramework,
  sourceCount: number,
  passes: number,
): () => void {
  const sources = Array.from({ length: sourceCount }, (_, i) => fw.signal(i));
  const c = fw.computed(() => {
    let total = 0;
    for (let pass = 0; pass < passes; pass++) {
      for (const s of sources) {
        total += s.read();
      }
    }
    return total;
  });
  fw.effect(() => void c.read());
  return () => fw.batch(() => sources[0]!.write(sources[0]!.read() + 1));
}
```

- Measures: time per write, and edges held by `c`.
- Gap: kairo `repeatedObservers` reads `head` 30 times in a row (`kairo/repeated.ts:11-14`), which a check against the previous read alone deduplicates.
- pico-js: `readSource` deduplicates only against the run's previous `readSource` (`graph.ts:543`), so `passes` passes over F sources hold `passes · F` edges. Probe `dup.ts`: reads of `a, b, a, b, a, b` leave 6 edges.
- alien: `link` returns early when the dependency's last subscriber link belongs to this subscriber in the current cycle (`system.mjs:29-31`), unless another subscriber linked that dependency in between.
- Preact: `addDependency` returns without a new edge when `signal._node` already targets the running context (`index.ts:213-214`, `index.ts:249-283`).
- `external/RESULTS.md`, section 4: the first marcisbee workspace run recorded 6,000 edges per recompute because a selector read 1,000 task sources six times, and pico-js ranked 41st at 57.8 ms. After the adapter cached the array, it ranked 25th at 50.5 ms.
- pico-js loses this case.

## 16. Missing data arrives

Pattern: rows that render a fallback while a record is absent, then read its fields and nested computeds when a response delivers it (S13).

```ts
// external js-reactivity-benchmark/src/proposed/dataArrives.ts
export function dataArrives(
  fw: ProposedFramework,
  rowCount: number,
  fieldCount: number,
): () => void {
  const present = fw.signal(false);
  const fields = Array.from({ length: fieldCount }, (_, i) => fw.signal(i));
  const formatted = fields.map((f) => fw.computed(() => `${f.read()}`));
  for (let r = 0; r < rowCount; r++) {
    fw.effect(() => {
      if (present.read()) {
        formatted.forEach((c) => c.read());
      }
    });
  }
  return () => fw.batch(() => present.write(!present.read()));
}
```

- Measures: time per arrival and per removal, edges allocated, and runs of the formatted computeds.
- Gap: a dynamic node changes one source per run (`reactively/dependencyGraph.ts:155-164`), so no upstream dependency list grows by a large factor. artalar's `dynamic_deps_bench.ts` grows and shrinks an aggregate's dependency list, with no alien or Preact arm.
- All three allocate an edge per new dependency (alien `system.mjs:33-43`, Preact `index.ts:215-245`, pico-js `graph.ts`, `readSource` and `readDerived`). The removal path is cases 1 and 5: alien marks the formatted computeds Dirty when the last row stops reading them, and pico-js disposes them at the end of the flush.
- App S13 (`b46f6a870`): alien ran 66 computations per event against 25 for pico-js and Preact, and p50 was 0.152, 0.195, and 0.180 ms (pico-js, alien, Preact). `bench/app/RESULTS.md` lists the count difference and states no cause for it.

## 17. Retained memory per node, and collection of dropped nodes

Pattern: a long session with 10 retained screens (`bench/app/RESULTS.md`, Model).

```ts
// external js-reactivity-benchmark/src/proposed/retainedBytes.ts
export function retainedBytes(
  fw: ProposedFramework,
  nodeCount: number,
  gc: () => void,
): Record<string, number> {
  const heap = () => {
    gc();
    gc();
    return process.memoryUsage().heapUsed;
  };
  const base = heap();
  const sources = Array.from({ length: nodeCount }, (_, i) => fw.signal(i));
  const afterSources = heap();
  const computeds = sources.map((s) => fw.computed(() => s.read() + 1));
  computeds.forEach((c) => c.read());
  const afterComputeds = heap();
  const disposers = computeds.map((c) => fw.effect(() => void c.read()));
  const afterEffects = heap();
  disposers.forEach((dispose) => dispose());
  computeds.length = 0;
  const afterDrop = heap();
  return {
    perSource: (afterSources - base) / nodeCount,
    perComputedReadOnce: (afterComputeds - afterSources) / nodeCount,
    perEffect: (afterEffects - afterComputeds) / nodeCount,
    retainedAfterDrop: afterDrop - afterSources,
  };
}
```

- Measures: bytes per source, per computed read once with its edge, per effect, and the bytes still held after the program drops every computed.
- Gap: the transitive-bullshit README lists memory consumption among what the suite does not measure (README.md:93). milomg calls `gc()` between runs and reads no heap figure. artalar's `start:mem` script records the change in `heapUsed` across one update (`computers_bench.ts:719-726`), which is allocation per update, not retained bytes per node. jotai's `store-size-scaling` times reads and writes per store size.
- App (`b46f6a870`): bytes per source node 64.0 (pico-js), 120.0 (alien), 96.1 (Preact); per derived node read once 218.5, 330.3, 331.0; per edge 63.7, 79.8, 87.9; per observation 510.3, 376.5, 419.2. The observation figure includes the app adapter's `Map` entry, listener array, and closure.
- `pico-js-where-it-wins.md`, section 1: 280 B per computed in pico-js against 312 B in alien and Preact. Section 2: readers that the program dropped after one read retain 2.7 MB at K=10,000 in pico-js and alien, against 53 KB in Preact.
- Preact does not link an unobserved computed into its sources (`index.ts:246-248`), so a dropped one is collectable. An alien computed read only outside a subscriber links its dependencies on its first run (`index.mjs:284-293`) and stays reachable from them. A pico-js computed read at top level must be retained (README, Contracts) and stays linked until it is released.

## Already covered upstream

- Fan-in: milomg sbench `update1000to1` and `create1000to1` (an effect over 1,000 sources), the transitive-bullshit `updateComputations1000to1` (a computed over 1,000 sources with no effect), and the dynamic `25-1000x5` shape. The remaining fan-in gap is case 15.
- One write per batch with effects: every kairo and sbench batch wraps one write. `pico-js-where-it-wins.md`, section 5, has alien 1.4 times faster than pico-js on that shape.
- Deep chains: `deepPropagation` (50), cellx (1,000 and 2,500 layers in milomg), and the dynamic `3-5x500` shape.
- Source-level dynamic dependencies: the dynamic shapes with 5%, 25%, and 50% dynamic nodes, and kairo `unstable`. `pico-js-where-it-wins.md`, section 6, has pico-js ahead from 10% switching on.
- Many effects on one source: sbench `update1to1000`, and cellx with 4 effects per layer.

## Methodology gaps

- The minimum hides GC and tails. milomg's `fastestTest` returns the fastest run (`util/benchRepeat.ts:15`). The transitive-bullshit fork reports the minimum of 3 samples (README.md:91) and calls `gc()` before and after each sample (`util/benchRepeat.ts:15`, `util/benchRepeat.ts:21`). In the app, GC per S2 event was 0.297 ms (pico-js), 0.597 (alien), and 2.083 (Preact), and every S10 event overlapped a GC under Preact (`bench/app/RESULTS.md`). A minimum discards both.
- No suite measures retained memory (case 17).
- Type feedback depends on what ran before. milomg's `runTests` runs every framework in one process (`packages/core/src/index.ts:26-38`), and `kairoBench` warms every case under every framework before it times any (`kairoBench.ts:32-44`). `pico-js-losses.md`, Method: without a prewarm of every suite test, alien runs `broadPropagation` in 49.6 ms instead of the suite's 65.2, because its call sites stay monomorphic. The transitive-bullshit fork starts a process per framework and family (README.md:83) and runs the cases in sequence inside it.
- No suite measures a cold run. Cold app events differ from warm ones by engine: cold S6a took 0.0888 ms for pico-js against 0.0250 (alien) and 0.0290 (Preact) (`bench/app/RESULTS.md`).
- Samples come from one process. `pico-js-where-it-wins.md`, Setup: 10 to 20% of whole processes ran 1.5 to 4 times slower under background load, and switching the statistic from the fastest process to the median process changed the winner in 7 of 86 cells.
- Computations are cheap and few function literals are hot. gg2-benchmark-plan-review.md (D1, and "What to measure") asks for 25 computation literals. The app bench compiles about 280 resolver literals with `new Function` (`bench/app/RESULTS.md`, Model).
- The ambient API puts adapter cost in every number for an engine with an explicit calling node. `pico-js-losses.md` measures the pico-js adapter's share of `broadPropagation` at 15.0 ms of 76.3 (v0). The transitive-bullshit README states that the custom effect schedulers are part of the measured adapters (README.md:93).
- Counts are not reported. kairo asserts values and effect runs (`kairo/broad.ts` `callCounter`). milomg's dynamic family checks one total count when `readFraction` is 1 or `testPullCounts` is set (`util/perfTests.ts`, `verifyBenchResult`). The transitive-bullshit dynamic fixtures hold sums only (`config.ts`, `expected: { sum }`), so its count check never runs on them (`util/perfTests.ts:34`).
- Some timed units are too small or measure something else. artalar times one update of 1 to 3 µs with `performance.now()` (`external/RESULTS.md`, section 3). The transitive-bullshit cellx default depths take 0.02 to 0.03 ms per engine (`external/RESULTS.md`, section 1). TanStack's bench builds the graph in every iteration, so it measures construction more than propagation (`swap/RESULTS-A.md`, TanStack caveats), and its Solid and Angular blocks do not rerun the graph in that environment (`external/RESULTS.md`, section 2).
- React dominates the marcisbee times, and the top 30 targets are a few ms apart (`external/RESULTS.md`, section 4).

## Not verified

- No proposed case has a timing at `ba3daedf7`.
- alien's link allocation on a reorder (case 4) is inferred from `system.mjs` and not counted.
- The recompute of a whole private chain in alien (cases 1 and 5) is inferred from `disposeAllDepsInReverse`. The probes used chains of depth 1.
- The cause of alien's extra computations in S13 (case 16).
- Whether Preact's walks in cases 1, 3, and 6 cost less time than alien's recomputes. The probes count runs only.
- The case 6 and case 7 counts for pico-js in the app at `ba3daedf7`.
