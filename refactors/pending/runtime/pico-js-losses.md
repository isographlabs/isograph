# pico-js: where POC-B loses

This doc explains each test where pico-js POC-B (the engine of PR 1027, `libs/pico-js/src/graph.ts` at `eeea06bf` in `../i2-worktrees/pico-js-perf`) is slower than alien-signals 3.1.2 or Preact Signals (signals-core 1.13.0) in pico-js-performance.md's final comparison. For each test it states the mechanism from operation counts and CPU profiles, splits the time into adapter, effect shim, representation, and algorithm, and lists the engine changes that close the gap.

## Status of the numbers

Every timing in this doc is v0, untrusted. The machine was running several benchmark agents at once while these were taken. Load averages were not recorded for the case matrix. At the end of the one-rep full suite, `uptime` reported `load averages: 5.76 22.43 19.11`. Operation counts are exact and do not depend on load. Profile shares are relative within one process. Conclusions rest on counts and profile shares where they can. A time difference below about 10% is not evidence of anything here.

## Method

The tooling lives in the session scratchpad, `bench/losses/` (`run.ts`, `cases.ts`, `direct.ts`, `adapters.ts`, `prewarm.ts`, `genall.py`, `instr/`).

- Each measurement is one Node process (24.12.0) that first runs a shortened copy of every suite test through the same framework (`prewarm.ts`), so the measured test starts from the type feedback a full-suite run gives it. Without the prewarm, alien runs broadPropagation in 49.6 ms instead of the suite's 65.2, because its call sites stay monomorphic. With the prewarm, pico-js, alien, and Preact reproduce the suite's broadPropagation times within 2 ms.
- The harness per test copies the suite's: kairo is 500 `iter()` per timed unit, best of 10; extra cases are `inner` iterations, best of 10; `3-5x500` is one `runGraph`, best of 5; sbench copies `run()` from `sBench.ts`.
- Variants, best of 5 processes each:
  - `pico`: POC-B with `libs/pico-js/bench/adapter.ts` (ambient `current`, effects pulled after every write).
  - `direct`: the same graph written against the pico-js API (`direct.ts`). A computation receives its calling node and calls `readSource` / `readDerived`. There is no `current`, no `track` wrapper, and no read or write closure. `pico − direct` is the adapter's share.
  - `pico-ondirty`: pico-js-batch.md applied to POC-B (`Batch` is a `Set`), with effects that set `onDirty` to a callback that pulls the effect. `ondirty-arr` is the same with `Batch` as an array.
  - `pico-fast2` and `pico-flag2`: the engine changes under Fixes.
- Counts come from instrumented copies of each engine (`instr/gen.py`, `instr/alien-src`, `instr/preact-src.ts` built from signals-core's `src/index.ts`). They are per iteration of the test: one `iter()` for kairo and extra cases, one write-and-read step for `3-5x500`, the whole timed run for sbench. The columns:
  - mark: dependents-list entries visited by marking (pico-js `markDirty`; alien `propagate` and `shallowPropagate`; Preact `_notify` calls);
  - verify: dependency edges visited by verification (pico-js `verify`; alien `checkDirty`; Preact `needsToRecompute`);
  - recompute: computations run;
  - stack: arrays allocated by pico-js `markDirty` and `verify`, or stack frames allocated by alien `propagate` and `checkDirty`;
  - relink: pico-js edges reused for a different dependency.
- Profiles are taken with `node:inspector` around the timed part only, with `--no-turbo-inlining` so that self time lands on the function that does the work. Shares exclude `console.assert`, which the inspector slows, and idle time.

## Summary

- Against alien, every loss is the adapter plus stack allocation in `markDirty` and `verify`. The direct graphs with fast2 and flag2 run at or below alien in every kairo test and in `3-5x500`. The engine's algorithm does more marking visits than alien only in deepPropagation (101 against 51 per write), and that difference does not survive in the direct timings.
- Against Preact, the losses with no effect (fanOutRepeat, noObservers, manyWritesThenPull, largeGraphSmallPull, isograph w10000 read10%) come from the algorithm: pico-js marks every reader on every write, and Preact marks none because no computed has a subscriber. A source flag (flag2) removes the repeated marking of readers that are already `Dirty`, which covers fanOutRepeat with M > 1 and noObservers. manyWritesThenPull and largeGraphSmallPull keep their marking with any change that keeps eager marking.
- The effect shim costs nothing in these tests. Every write in them takes every effect out of `Clean`, so pulling all effects pulls the ones `onDirty` would call. Replacing the shim with pico-js-batch.md's `onDirty` makes every effect test slower: the `Set` batch adds 1.7 to 44 ms, an array batch 1.1 to 17 ms.
- Isograph calls the explicit API, so the adapter's share does not apply to it.

## Per test

Times are ms, v0. The "suite" column is pico-js-performance.md's final run (pico-js / alien / Preact). "pico − direct" is the adapter, "fast2" and "flag2" are the gain of each fix applied in that order, "onDirty" is `pico-ondirty − pico`, all from the case matrix.

| test | suite | mechanism (counts per iteration) | pico − direct | fast2 | flag2 | onDirty | cause |
|---|---|---|---|---|---|---|---|
| createSignals | 0.78 / 0.69 / 0.64 | No graph work: every count is 0. The adapter allocates a read closure, a write closure, and a wrapper object per source; the engine allocates one 3-field object. Profile: garbage collector 29%, adapter `signal` 19%, `createSourceNode` 7%. | 0.43 of 0.59 | 0 | 0 | 0 | adapter, 73% of pico-js time. The suite gap (0.09 ms) is inside the spread between processes: in the matrix Preact took 1.17. |
| createComputations | 27.6 / 26.4 / 45.2 | 675,100 effects, 800,000 edges allocated and unlinked. alien allocates 1,475,100 edges, because each effect also links into the effect scope. Profile: garbage collector 24%, then `readSource`, `unlinkAll`, adapter `effect` and `read` at 5-6% each. | 6.7 of 23.4 | 0 | 0 | +11.4 | adapter, 29% of pico-js time (one `track` closure per effect). pico-js allocates fewer edges than alien. |
| avoidablePropagation | 82.5 / 73.0 / 75.7 | 1,001 writes. Per write pico-js: 7 mark, 5 verify, 2 recompute, 3 stack; alien 6, 6, 2, 0. The chain `computed1..computed5` and the effect have one dependent each, so every array pico-js allocates holds one node or one edge. | 6.9 | 11.0 | −0.8 | +18.8 | representation (stack arrays), then adapter. pico-flag2 73.9 against alien 74.2; direct with both fixes 71.2. |
| broadPropagation | 76.3 / 65.2 / 68.2 | 51 writes. Per write pico-js: 250 mark, 100 verify, 150 recompute, 151 stack; alien 200, 101, 150, 0; Preact 150, 150, 150. pico-js's 50 extra mark visits come from `recompute` marking its one dependent `Dirty` after a changed value; alien passes the change up `checkDirty`'s walk instead. | 15.0 | 6.3 | 1.4 | +44.3 | adapter, 91% of the gap to alien (150 computations and 150 reads per write go through `track` and the read closures). Direct with both fixes 61.5 against alien 60.2. |
| deepPropagation | 28.7 / 28.7 / 27.1 | 51 writes down a 50-node chain. Per write pico-js: 101 mark, 50 verify, 51 recompute, 52 stack; alien 51, 51, 51, 0; Preact 51, 51, 51. | 7.3 | 5.3 | 0.2 | −3.2 | adapter and representation, each larger than the gap to Preact. The 50 extra mark visits are the algorithm's, but direct with both fixes runs in 22.1 against Preact 28.6. |
| diamond | 61.7 / 60.4 / 62.4 | 501 writes. Per write pico-js: 17 mark, 2 verify, 7 recompute, 8 stack; alien 20, 3, 7, 0. | 7.3 | 3.8 | −0.3 | +7.1 | adapter and representation. The suite gap to alien is 2%, inside the noise. |
| mux | 63.4 / 59.4 / 61.7 | 20 writes, each one recomputing `mux` (a new object every time), so every effect leaves `Clean` on every write. Per write pico-js: 363 mark, 181 verify, 93 recompute, 94 stack; alien 361, 186, 93, 2. Profile: the `Object.fromEntries` computation is 33% in all three engines. | 6.5 | 5.5 | 0.6 | +31.4 | adapter and representation. pico-flag2 60.8, direct 60.6, alien 58.9. |
| repeatedObservers | 9.7 / 10.5 / 9.0 | 101 writes; the computed reads `head` 30 times. Per write pico-js: 3 mark, 1 verify, 2 recompute; alien and Preact 2, 2, 2. Profile: `readSource` 21% and the adapter's read closure 18%. | 1.9 | 0.2 | 0.2 | +1.7 | adapter: 30 read-closure calls per recompute. Direct 8.7 against Preact 10.1. |
| 3-5x500 | 71.3 / 70.0 / 78.7 | Per step pico-js: 29,882 mark, 1,002 verify, 4,988 recompute, 4,991 stack; alien 29,886, 1,006, 4,988, and 5,972 stack frames. The counts match alien. | 18.1 | 4.4 | 0.8 | −5.5 | adapter, 15,000 reads per step through the read closures. Direct 56.6 against alien 72.3. |
| isograph w10000 read10% | 3.2 / 6.6 / 3.1 | 10,000 writes, no effect. pico-js: 7,717 mark, 129 verify, 596 recompute, 5,899 stack; Preact 0 mark (no computed has a subscriber), 1,442 verify, 596 recompute. Profile: `markDirty` 35% for pico-js. flag2: 2,276 mark, 104 stack. | 1.45 | 0.1 | 0.4 | +0.4 | algorithm (eager marking of readers nobody observes), reduced by flag2; the 2% suite gap is inside the noise. |
| fanOutRepeat K10 M10, K10 M100, K100 M10, K100 M100, K1000 M10, K1000 M100 | e.g. K1000 M100: 5.9 / 12.2 / 0.8 | One source with K readers, M writes, then one read of each reader. pico-js marks K entries per write (K·M per iteration): after the first write every reader is `Dirty`, and each later write walks K `Dirty` readers and changes nothing. alien walks K·M + 3K. Preact: 0 mark, K verify, K recompute. Profile: `markDirty` 68% (K100 M100) and 84% (K1000 M100) of pico-js time. flag2: K mark per iteration. | 0.3 to 2.6 | 0 to 0.9 | 1.6 to 3.5 | ±1 | algorithm (repeated marking of `Dirty` readers), all of it removed by flag2: K100 M100 4.44 → 0.78 (Preact 0.80), K1000 M100 5.17 → 0.99 (0.73), K1000 M10 9.70 → 6.23 (6.44), K100 M10 8.49 → 5.53 (5.43), K10 M100 4.83 → 1.90 (3.31), K10 M10 9.20 → 7.63 (8.23). |
| fanOutRepeat K100 M1, K1000 M1 | 33.7 / 52.2 / 32.5; 45.7 / 63.9 / 35.2 | One write per read. pico-js: K mark, K recompute, 1 stack; Preact: 0 mark, K verify, K recompute. The work per reader is one visit for pico-js against one version comparison for Preact. | 7.9; 10.5 | 0 | 0 | +3 | adapter. In the matrix pico-js is at or below Preact (28.5 against 31.9, 34.9 against 34.8), and direct runs in 20.7 and 24.5. |
| noObservers | 1.00 / 1.87 / 0.65 | 100 writes to sources whose readers are never read again. pico-js: 200 mark, 100 stack; Preact: 0. Profile: `markDirty` 37%, `writeSource` 18%. flag2: 0.7 mark per iteration. | 0.26 | 0.1 | 0.2 | +0.3 | algorithm, removed by flag2 (0.96 → 0.66; Preact 1.01 in the matrix). |
| manyWritesThenPull | 21.5 / 50.3 / 13.5 | 100 writes, each to a different source that all 1,000 nodes read, then one read of each node. pico-js: 105,000 mark (the first write marks 1,000 nodes `Dirty`, the other 99 walk 1,000 `Dirty` nodes each), 1,050 recompute; alien 212,100 mark; Preact 0 mark, 1,050 verify, 1,050 recompute. Profile: `markDirty` 55% of pico-js time. | −1.0 | 0.1 | −0.4 | 0 | algorithm, 100% of the gap. flag2 does not apply, because each source is written once per iteration. |
| largeGraphSmallPull | 0.68 / 0.83 / 0.06 | One source, 100 mids, 10,000 leaves, 10 leaves read per write. pico-js: 2,142 mark (100 mids, about 1,000 leaves set `Check` under the mids the previous read left `Clean`, about 1,000 leaves set `Dirty` when those mids recompute), 20 recompute; Preact: 0 mark, 20 verify, 20 recompute. Profile: `markDirty` 67%. | 0 | 0 | 0 | 0 | algorithm, 100% of the gap. Eager marking reaches the 100 leaves under each recomputed mid, of which one is read. |
| deepChainCutoff | 2.41 / 2.11 / 2.02 | One write, then a read of the top of a 1,001-node chain whose bottom returns a constant. pico-js: 1,006 mark, 1,005 verify, 1 recompute, 2 stack (one push and pop per chain node on each walk); alien 1,007, 1,006, 1; Preact 0 mark, 1,006 refresh. Profile: `markDirty` 47%, `verify` 41%. | 0.37 | 0.12 | 0 | +0.2 | representation (the per-node push and pop on both walks; fast2 removes them). The counts equal alien's. The `pico − direct` difference is not adapter work, because the adapter runs three closures per iteration; it is a difference in the code V8 compiles in the two processes. Direct with both fixes 1.57 against alien 1.50 and Preact 1.65. |
| dynamicSwitch | 2.35 / 2.39 / 1.74 | Each write of the selector makes all 1,000 nodes read the other source. pico-js: 1,021 mark, 1,021 recompute, 1,020 relink (unlink from one source's list and link into the other's), 0 allocations; Preact: 0 mark, 1,020 edges allocated and linked only into the computed's own list, because an unobserved computed is not in its sources' lists. | 0.74 | 0.1 | −0.2 | +0.6 | adapter in the matrix (direct 1.39 against Preact 1.97); the marking of 1,021 nodes is the algorithm's share and is smaller than the adapter's. |

## Fixes

Both fixes pass the 23 tests in `libs/pico-js/test/graph.test.ts` (run against each engine file in `bench/losses/vt/`), and the benchmark's own assertions.

### fast2: no stack allocation on single-dependent paths

`markDirty` holds the last node it must descend into in a local, allocates its stack on the first push, and descends into a dependent that is the last entry of its list without pushing it. `recompute` marks through pico-js-batch.md's `markCheckDependentsDirty`, which needs no stack. `verify` pushes an edge only when its dependency has more than one dependent; a node with one dependent returns through its only dependents-list edge (alien's `checkDirty` does the same with `hasMultipleSubs`).

```ts
// from libs/pico-js/src/graph.ts
function markDirty<TDependency>(first: Edge<TDependency>): void {
  let stack: Array<DerivedNode<unknown>> | undefined;
  let held: DerivedNode<unknown> | undefined;
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3:
        node.status = 1;
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        }
        break;
      case 2:
        node.status = 1;
        break;
      case 1:
        break;
      case 0:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
  let marked = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3:
          dependent.status = 2;
          if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          }
          break;
        case 2:
        case 1:
          break;
        case 0:
          throw new Error(
            'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
          );
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
}
```

`verify`, before (POC-B):

```ts
// from libs/pico-js/src/graph.ts
      if (dependency.status === 2) {
        verifyStack.push(edge);
```

```ts
// from libs/pico-js/src/graph.ts
    for (;;) {
      const popped = verifyStack.pop();
      if (popped === undefined) {
        return computedValue(root);
      }
      edge = popped;
```

After:

```ts
// from libs/pico-js/src/graph.ts
      if (dependency.status === 2) {
        if (edge.previousDependent !== undefined || edge.nextDependent !== undefined) {
          (verifyStack ??= []).push(edge);
        }
```

```ts
// from libs/pico-js/src/graph.ts
    for (;;) {
      if (node === root) {
        return computedValue(root);
      }
      const only = node.firstDependent!;
      edge = only.nextDependent === undefined ? only : verifyStack!.pop()!;
```

Counts: stack allocations per write fall from 151 to 1 (broadPropagation), 52 to 0 (deepPropagation), 3 to 0 (avoidablePropagation), 4,991 to 4 per step (`3-5x500`). Measured gain (v0): avoidablePropagation 11.0 ms, broadPropagation 6.3, deepPropagation 5.3, mux 5.5, `3-5x500` 4.4, diamond 3.8.

### flag2: a write skips a source whose readers are all `Dirty`

A source records that its last write left every node in its dependents list `Dirty`. After `markDirty` from a source, every direct dependent is `Dirty` (its first loop sets `Clean` and `Check` to `Dirty`). A direct dependent leaves `Dirty` only through a recompute, which either reads the source again (`readSource` clears the record) or does not (`trim` removes the edge). A new dependent reaches the list only through `readSource`. A dispose removes the node from the list. So the record holds until the next `readSource` of that source.

```ts
// from libs/pico-js/src/graph.ts
export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
  dependentsDirty: 0 | 1;
};

export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined && source.dependentsDirty === 0) {
    source.dependentsDirty = 1;
    markDirty(first);
  }
}

export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  if (source.dependentsDirty === 1) {
    source.dependentsDirty = 0;
  }
  // POC-B's body follows unchanged
```

With pico-js-batch.md, a skipped write collects no callback, which is the same result marking would give, because marking collects a callback only when a node leaves `Clean`.

Counts: fanOutRepeat mark visits per iteration fall from K·M to K, noObservers from 200 to 0.7, isograph w10000 read10% from 7,717 to 2,276. Measured gain (v0) over fast2: fanOutRepeat K100 M100 3.5 ms (to 0.78, Preact 0.80), K1000 M100 3.3, K1000 M10 3.1, K100 M10 2.9, K10 M100 2.3; noObservers 0.2; elsewhere within ±1.4 ms.

### Not fixed

- manyWritesThenPull and largeGraphSmallPull keep their marking. A node that no observer holds still gets visited on each write that reaches it. Preact avoids this by not linking an unobserved computed into its sources' dependents lists and comparing source versions on read. That is a change of algorithm (pico-js-comparison.md, "unobserved nodes"), not a cheap fix.
- The adapter share has no engine fix. The benchmark's API is ambient, so the adapter adds one `track` call per computation and one closure call per read. Removing `try/finally` from `track` changed nothing measurable (broadPropagation 74.9 → 76.3).

### Effect shim

The effect shim (pull every effect after each write) costs nothing in these tests, because every write in them takes every effect out of `Clean`: pulls of `Clean` effects that `onDirty` would skip are 0 per write in every test except mux, where they are 10 per write and O(1) each. pico-js-batch.md's `onDirty` with a `Set` batch adds 44.3 ms to broadPropagation (2,550 callbacks per iteration, about 35 ns each), 31.4 to mux, 18.8 to avoidablePropagation. An array batch cuts that to 6.7, 7.1, and 16.7. The remaining cost of the array batch in avoidablePropagation (about 33 ns per callback) is not explained; the profile was not taken.

## Full suite, one rep (v0)

One process per engine, one rep, while the machine was loaded (`load averages: 5.76 22.43 19.11` at the end). alien and Preact were not completed in this run.

| | POC-B | fast2 | fast2 + flag2 | fast2 + flag2 + onDirty (array batch) |
|---|---|---|---|---|
| upstream total (20 tests) | 1818.0 | 1574.5 | 1508.5 | 1557.0 |
| extra total (27 tests) | 462.3 | 442.6 | 423.7 | 452.0 |
| geometric mean of ratios to POC-B | 1.000 | 0.927 | 0.798 | 0.891 |

In the fast2 and flag2 columns, no test got slower by more than the noise except fanOutRepeat K10 M1 (29.6 → 44.1 with fast2, 42.0 with flag2), which is not in the case matrix. Its counts cannot have risen (fast2 only removes allocations), so this is unexplained and needs the serial rerun. The onDirty column is slower than flag2 in writeThenDispose (6.1 → 17.1), createComputations (25.2 → 41.1), updateSignals (196.3 → 239.3), and the kairo effect tests.

## Decisions

pico-js.md adopts fast2 and flag2, with these decisions.

- The source field is `dependentsStatus: DependentsStatus`. `DependentsStatus` is the union of two numbers with a named constant for each, private to `graph.ts`: `DEPENDENTS_MAY_NOT_BE_DIRTY` (`0`, a node in the dependents list may be `Check` or `Clean`, so a write marks the list) and `DEPENDENTS_ALL_DIRTY` (`1`, every node in the dependents list is `Dirty`, so a write skips the list). The constants cost nothing measurable against the literals (fanOutRepeat K1000 M100 0.69 ms against 0.93, noObservers 0.64 against 0.62, v0).
- fast2's `verify` has no `!` assertion. A node `verify` descended into with no dependents, or with more than one and no edge on the stack, throws an error that ends "This is indicative of a bug in pico-js.", as the other invariant throws do.
- pico-js-batch.md keeps its `Set` batch.
