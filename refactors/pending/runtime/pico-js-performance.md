# pico-js: performance

This doc measures `libs/pico-js` (pico-js.md's `graph.ts`) against alien-signals 3.1.2 and Preact Signals 2.8.0 (signals-core 1.13.0) in js-reactivity-benchmark, names the causes of the gap, and proposes engine changes. POC-A keeps every pico-js semantic. POC-B changes internal invariants and is labeled as such.

## Reproduce

- Branch: detached worktree `../i2-worktrees/pico-js-perf`, commits listed under Branch. `pnpm --filter @isograph/pico-js bench [frameworks] [upstream|isograph|all] [runs]` clones js-reactivity-benchmark at `56eb45e8` into `libs/pico-js/node_modules/.cache`, runs each framework in its own Node process, and prints the minimum per test over the runs.
- Node 24.12.0, Apple Silicon, 18 cores. Every number below is the minimum of 3 runs, one process per engine per run.
- Run-to-run drift between sessions is about 3%. A change counts only when a same-session comparison moves the upstream total by more than that.
- The adapter (`bench/adapter.ts`) is the same for every pico-js variant. Its `===` check before `writeSource` is pico-js's contract (a caller writes a source only when its value changed); alien and Preact make the same comparison inside their setters. It is not counted as adapter overhead.

## Results

Upstream suites (sbench, kairo, cellx, dynamic), ms:

| test | baseline | POC-A | POC-B | alien | Preact |
|---|---|---|---|---|---|
| createSignals | 5.3 | 0.8 | 0.8 | 0.7 | 0.6 |
| createComputations | 148.2 | 27.3 | 27.6 | 26.3 | 45.2 |
| updateSignals | 1153.6 | 253.0 | 174.0 | 234.8 | 196.3 |
| avoidablePropagation | 184.4 | 96.1 | 82.5 | 73.0 | 75.7 |
| broadPropagation | 415.0 | 91.0 | 76.3 | 65.2 | 68.2 |
| deepPropagation | 141.7 | 35.8 | 28.7 | 28.7 | 27.1 |
| diamond | 222.1 | 73.2 | 61.7 | 60.4 | 62.4 |
| mux | 191.8 | 72.7 | 63.4 | 59.4 | 61.7 |
| repeatedObservers | 34.5 | 14.7 | 9.7 | 10.5 | 9.0 |
| triangle | 76.2 | 23.5 | 19.2 | 19.9 | 21.2 |
| unstable | 44.1 | 16.5 | 8.8 | 11.6 | 11.2 |
| molBench | 13.0 | 12.5 | 11.8 | 12.0 | 12.0 |
| cellx1000 | 11.5 | 4.0 | 3.5 | 3.6 | 8.2 |
| cellx2500 | 31.1 | 11.2 | 8.7 | 10.8 | 15.7 |
| 2-10x5 - lazy80% | 575.1 | 174.7 | 126.2 | 140.1 | 143.2 |
| 6-10x10 - dyn25% - lazy80% | 373.5 | 109.1 | 80.6 | 104.0 | 102.0 |
| 4-1000x12 - dyn5% | 735.3 | 300.5 | 191.3 | 218.5 | 273.1 |
| 25-1000x5 | 1063.8 | 350.9 | 222.6 | 294.3 | 350.0 |
| 3-5x500 | 272.5 | 90.0 | 71.3 | 70.0 | 78.7 |
| 6-100x15 - dyn50% | 439.5 | 147.9 | 115.9 | 136.9 | 145.5 |
| total | 6132.5 | 1905.4 | 1384.3 | 1580.7 | 1706.8 |

`bench/cases.ts`, ms (Isograph-shaped case and the workload cases below):

| test | baseline | POC-A | POC-B | alien | Preact |
|---|---|---|---|---|---|
| isograph w10 read100% | 460.2 | 162.5 | 106.0 | 116.7 | 1628.4 |
| isograph w10 read10% | 20.9 | 7.6 | 6.0 | 8.5 | 135.0 |
| isograph w100 read100% | 230.8 | 78.7 | 48.0 | 55.1 | 217.8 |
| isograph w100 read10% | 4.8 | 1.9 | 1.9 | 3.3 | 15.9 |
| isograph w1000 read100% | 95.0 | 35.2 | 23.5 | 29.8 | 37.9 |
| isograph w1000 read10% | 8.6 | 3.7 | 3.5 | 6.0 | 4.9 |
| isograph w10000 read100% | 38.6 | 14.1 | 9.7 | 15.7 | 10.5 |
| isograph w10000 read10% | 5.6 | 3.3 | 3.2 | 6.6 | 3.1 |
| fanOutRepeat K10 M1 | 186.8 | 33.4 | 27.0 | 47.5 | 30.5 |
| fanOutRepeat K10 M10 | 37.2 | 9.8 | 9.8 | 17.2 | 7.7 |
| fanOutRepeat K10 M100 | 6.9 | 4.1 | 5.8 | 9.8 | 2.8 |
| fanOutRepeat K100 M1 | 214.1 | 43.6 | 33.7 | 52.2 | 32.5 |
| fanOutRepeat K100 M10 | 37.0 | 8.2 | 9.0 | 13.1 | 6.1 |
| fanOutRepeat K100 M100 | 4.4 | 1.2 | 4.2 | 5.1 | 0.9 |
| fanOutRepeat K1000 M1 | 228.2 | 52.5 | 45.7 | 63.9 | 35.2 |
| fanOutRepeat K1000 M10 | 40.1 | 9.7 | 14.4 | 20.7 | 6.4 |
| fanOutRepeat K1000 M100 | 4.4 | 1.1 | 5.9 | 12.2 | 0.8 |
| sameRecordWrites | 31.8 | 10.5 | 7.3 | 10.8 | 9.0 |
| writeThenDispose | 32.8 | 5.6 | 5.8 | 6.8 | 8.5 |
| writeThenDispose control | 23.3 | 6.0 | 5.8 | 5.8 | 6.6 |
| noObservers | 1.4 | 0.6 | 1.0 | 1.9 | 0.6 |
| fanIn F100 | 14.4 | 4.1 | 1.6 | 4.5 | 2.9 |
| fanIn F1000 | 14.6 | 5.3 | 1.9 | 4.7 | 4.1 |
| manyWritesThenPull | 69.8 | 19.4 | 21.5 | 50.3 | 13.5 |
| largeGraphSmallPull | 0.2 | 0.1 | 0.7 | 0.8 | 0.1 |
| deepChainCutoff | 4.7 | 3.1 | 2.4 | 2.1 | 2.0 |
| dynamicSwitch | 7.6 | 2.3 | 2.4 | 2.4 | 1.7 |
| total | 1824.4 | 527.5 | 407.5 | 573.3 | 2225.7 |

POC-B beats alien and Preact on the upstream total and on the `bench/cases.ts` total. POC-A, which keeps detach-on-dirty, is 21% slower than alien and 12% slower than Preact on the upstream total, and beats both on the `bench/cases.ts` total.

## Measured causes, ranked

Baseline profile, sbench only (`node --cpu-prof`, self time; `graph.ts` is engine, `adapter` is `bench/adapter.ts`, `harness` is js-reactivity-benchmark):

| # | function | self | owner |
|---|---|---|---|
| 1 | `detach` (`Set.delete` per dependency) | 40.2% | engine |
| 2 | `attach` (`Set.add` per dependency) | 15.4% | engine |
| 3 | garbage collector | 9.1% | engine (Sets per node, arrays) |
| 4 | `readSource` (`Array.push`, inlined computation) | 8.2% | engine |
| 5 | `markDependentsDirty` (Set iteration) | 7.6% | engine |
| 6 | `flushEffects` (inlined `pull`, `recompute`) | 6.6% | adapter |
| 7 | `writeSource` | 2.9% | engine |
| 8 | (program) | 1.9% | runtime |
| 9 | update closure `update1to1000` | 1.8% | harness |
| 10 | `withBatch` | 1.1% | adapter |
| 11 | `createSignals` | 0.7% | harness |
| 12 | `run` | 0.7% | harness |
| 13 | `cleanup` | 0.4% | adapter |
| 14 | signal `read` | 0.3% | adapter |
| 15 | effect closure `update1to1` | 0.3% | harness |

`update1to1000` (1000 effects over one source, 10,000 writes) is 1060 of the baseline's 1216 ms `updateSignals`; in that test `detach` is 48.6% and `attach` 18.7%.

Causes of the baseline's gap to alien, ranked by measured share:

| # | cause | measurement | share |
|---|---|---|---|
| 1 | `Set` dependents: `Set.delete` in `detach` on every mark, `Set.add` in `attach` on every recompute, one `Set` per node | edge lists in place of Sets (change 1): upstream 6198.4 -> 1950.5 | 69% of baseline time |
| 2 | Detach-on-dirty (algorithm): every recompute unlinks and relinks each edge, where alien keeps edges | POC-A -> POC-B before changes 4 and 5 (change 3 alone): 1894.6 -> 1638.0 | 13.5% of POC-A time |
| 3 | Recursive verification (`verify` -> `pull` -> `verify`) | iterative verify (change 4): POC-B 1638.0 -> 1566.5; POC-A 1894.6 -> 1901.8 | 4.4% of POC-B |
| 4 | String statuses | numeric statuses (change 5): POC-B 1566.5 -> 1494.5; POC-A 1913.5 -> 1985.0; baseline no change (1214 -> 1218 updateSignals) | 4.6% of POC-B, none in POC-A |
| 5 | Invariant throws and the `NOT_SET` check | POC-B without them: 1481.2 -> 1416.0; throws moved into a helper function: 1472.1 | 4.4% of POC-B, kept |
| 6 | Pushing nodes without dependents onto the marking stack | change 2: POC-A 1950.5 -> 1894.6 | 2.9% of POC-A |
| 7 | Per-write stack allocation in `markDependentsDirty` | module-level stack, baseline: updateSignals 1214.1 -> 1173.0; POC-A local vs module stack 1954.6 vs 1970.8 | at most 3% in the baseline, none in POC-A |
| 8 | Adapter ambient tracking | direct API vs adapter, baseline: `update1to1000` 1043 -> 950 ms, `create1to1` 35 -> 10 ms; deep chain on POC-B 25.8 -> 22.3 ms, alien 20.3 ms; removing `try`/`finally` from `track`: no change | 9-14% of a test, outside the engine |
| 9 | Deleting from the `Set` being iterated | baseline variant that clears the written source's `Set` after the loop instead of deleting each member: 1214 -> 1212 | none |

Hypotheses from the brief: (a) Set churn in `detach`/`attach` is cause 1; (b) Set iteration in marking is part of cause 1 (7.6% self) and disappears with edge lists, and deleting during iteration costs nothing (cause 9); (c) the adapter costs 9-14% of a test (cause 8); (d) the per-write stack costs at most 3% (cause 7).

### Operation counts

Counts per test over the whole run, in millions: marking visits / edge-list operations (link, unlink, allocate) / recomputes / verification steps. Recomputes are equal in every engine on every test: pico-js runs exactly the computations alien runs. alien's counts come from an instrumented copy of alien-signals 3.1.2 (`propagate`, `shallowPropagate`, `link`, `unlink`, `checkDirty`, `updateComputed`, `run`). Warmup of the kairo cases is counted under `avoidablePropagation`.

| test | baseline | POC-A | POC-B | alien |
|---|---|---|---|---|
| createComputations | 0.0 / 24.1 / 0.0 / 0.0 | 0.0 / 24.1 / 0.0 / 0.0 | 0.0 / 24.1 / 0.0 / 0.0 | 0.0 / 29.6 / 0.0 / 0.0 |
| updateSignals | 113.3 / 367.1 / 113.3 / 0.0 | 113.3 / 131.4 / 113.3 / 0.0 | 113.3 / 0.1 / 113.3 / 0.0 | 219.6 / 0.1 / 113.3 / 9.1 |
| avoidablePropagation | 35.2 / 30.3 / 10.1 / 25.1 | 35.2 / 10.1 / 10.1 / 25.1 | 35.2 / 0.0 / 10.1 / 25.1 | 30.2 / 0.0 / 10.1 / 30.1 |
| broadPropagation | 63.8 / 114.8 / 38.3 / 25.5 | 63.8 / 38.3 / 38.3 / 25.5 | 63.8 / 0.0 / 38.3 / 25.5 | 51.0 / 0.0 / 38.3 / 25.8 |
| deepPropagation | 25.8 / 39.0 / 13.0 / 12.8 | 25.8 / 13.0 / 13.0 / 12.8 | 25.8 / 0.0 / 13.0 / 12.8 | 13.0 / 0.0 / 13.0 / 13.0 |
| diamond | 32.6 / 82.7 / 17.5 / 5.0 | 32.6 / 37.6 / 17.5 / 5.0 | 42.6 / 0.0 / 17.5 / 5.0 | 50.1 / 0.0 / 17.5 / 7.5 |
| mux | 36.3 / 54.6 / 9.3 / 18.1 | 36.3 / 27.1 / 9.3 / 18.1 | 36.3 / 0.0 / 9.3 / 18.1 | 36.1 / 0.0 / 9.3 / 18.6 |
| repeatedObservers | 1.5 / 47.0 / 1.0 / 0.5 | 1.5 / 15.7 / 1.0 / 0.5 | 1.5 / 0.0 / 1.0 / 0.5 | 1.0 / 0.0 / 1.0 / 1.0 |
| triangle | 10.1 / 30.3 / 5.6 / 0.5 | 10.1 / 14.7 / 5.6 / 0.5 | 19.2 / 0.0 / 5.6 / 0.5 | 19.7 / 0.0 / 5.6 / 1.0 |
| unstable | 2.0 / 63.7 / 1.5 / 0.5 | 2.0 / 21.7 / 1.5 / 0.5 | 3.0 / 1.0 / 1.5 / 0.5 | 4.0 / 3.0 / 1.5 / 1.0 |
| molBench | 0.2 / 0.6 / 0.1 / 0.0 | 0.2 / 0.3 / 0.1 / 0.0 | 0.3 / 0.1 / 0.1 / 0.0 | 0.3 / 0.1 / 0.1 / 0.0 |
| cellx1000 | 0.2 / 0.6 / 0.1 / 0.0 | 0.2 / 0.4 / 0.1 / 0.0 | 0.2 / 0.3 / 0.1 / 0.0 | 0.2 / 0.3 / 0.1 / 0.0 |
| cellx2500 | 0.4 / 1.4 / 0.2 / 0.1 | 0.4 / 0.9 / 0.2 / 0.1 | 0.5 / 0.6 / 0.2 / 0.1 | 0.5 / 0.7 / 0.2 / 0.1 |
| 2-10x5 - lazy80% | 67.2 / 190.1 / 31.7 / 24.5 | 67.2 / 95.0 / 31.7 / 24.5 | 77.8 / 0.0 / 31.7 / 24.5 | 69.6 / 0.0 / 31.7 / 31.7 |
| 6-10x10 - dyn25% - lazy80% | 57.8 / 167.0 / 9.4 / 1.2 | 57.8 / 102.0 / 9.4 / 1.2 | 97.7 / 0.0 / 9.4 / 1.2 | 98.2 / 0.0 / 9.4 / 1.7 |
| 4-1000x12 - dyn5% | 52.9 / 308.5 / 11.8 / 28.5 | 52.9 / 194.0 / 11.8 / 28.5 | 82.4 / 0.1 / 11.8 / 28.5 | 82.6 / 0.1 / 11.8 / 28.7 |
| 25-1000x5 | 96.4 / 511.4 / 5.9 / 11.6 | 96.4 / 335.1 / 5.9 / 11.6 | 181.6 / 0.2 / 5.9 / 11.6 | 182.2 / 0.2 / 5.9 / 12.2 |
| 3-5x500 | 39.8 / 89.8 / 10.0 / 2.0 | 39.8 / 49.9 / 10.0 / 2.0 | 59.7 / 0.0 / 10.0 / 2.0 | 59.8 / 0.0 / 10.0 / 2.0 |
| 6-100x15 - dyn50% | 54.7 / 160.0 / 8.6 / 1.4 | 54.7 / 98.0 / 8.6 / 1.4 | 92.2 / 0.1 / 8.6 / 1.4 | 92.3 / 0.1 / 8.6 / 1.5 |
| isograph w10 read100% | 5.6 / 326.5 / 3.2 / 60.0 | 5.6 / 214.4 / 3.2 / 60.0 | 5.8 / 0.0 / 3.2 / 60.0 | 9.4 / 0.0 / 3.2 / 62.2 |
| isograph w10 read10% | 1.0 / 10.2 / 0.6 / 1.4 | 1.0 / 6.2 / 0.6 / 1.4 | 2.4 / 0.0 / 0.6 / 1.4 | 3.2 / 0.0 / 0.6 / 2.0 |
| isograph w100 read100% | 4.8 / 134.1 / 2.5 / 14.1 | 4.8 / 86.9 / 2.5 / 14.1 | 6.0 / 0.0 / 2.5 / 14.1 | 9.2 / 0.0 / 2.5 / 16.3 |
| isograph w100 read10% | 0.2 / 2.1 / 0.1 / 0.2 | 0.2 / 1.3 / 0.1 / 0.2 | 1.3 / 0.0 / 0.1 / 0.2 | 1.4 / 0.0 / 0.1 / 0.3 |
| isograph w1000 read100% | 4.0 / 35.2 / 2.0 / 1.2 | 4.0 / 21.5 / 2.0 / 1.2 | 5.5 / 0.0 / 2.0 / 1.2 | 7.9 / 0.0 / 2.0 / 3.3 |
| isograph w1000 read10% | 0.4 / 3.2 / 0.2 / 0.3 | 0.4 / 1.9 / 0.2 / 0.3 | 1.7 / 0.0 / 0.2 / 0.3 | 2.0 / 0.0 / 0.2 / 0.5 |
| isograph w10000 read100% | 1.8 / 11.2 / 0.9 / 0.2 | 1.8 / 6.6 / 0.9 / 0.2 | 3.5 / 0.0 / 0.9 / 0.2 | 4.9 / 0.0 / 0.9 / 1.0 |
| isograph w10000 read10% | 0.2 / 1.3 / 0.1 / 0.0 | 0.2 / 0.8 / 0.1 / 0.0 | 1.5 / 0.0 / 0.1 / 0.0 | 2.0 / 0.0 / 0.1 / 0.1 |
| fanOutRepeat K10 M1 | 20.0 / 60.0 / 20.0 / 0.0 | 20.0 / 20.0 / 20.0 / 0.0 | 20.0 / 0.0 / 20.0 / 0.0 | 80.0 / 0.0 / 20.0 / 2.0 |
| fanOutRepeat K10 M10 | 3.6 / 10.9 / 3.6 / 0.0 | 3.6 / 3.6 / 3.6 / 0.0 | 36.4 / 0.0 / 3.6 / 0.0 | 47.3 / 0.0 / 3.6 / 0.4 |
| fanOutRepeat K10 M100 | 0.4 / 1.2 / 0.4 / 0.0 | 0.4 / 0.4 / 0.4 / 0.0 | 39.6 / 0.0 / 0.4 / 0.0 | 40.8 / 0.0 / 0.4 / 0.0 |
| fanOutRepeat K100 M1 | 20.0 / 60.0 / 20.0 / 0.0 | 20.0 / 20.0 / 20.0 / 0.0 | 20.0 / 0.0 / 20.0 / 0.0 | 80.0 / 0.0 / 20.0 / 0.2 |
| fanOutRepeat K100 M10 | 3.6 / 10.9 / 3.6 / 0.0 | 3.6 / 3.6 / 3.6 / 0.0 | 36.4 / 0.0 / 3.6 / 0.0 | 47.3 / 0.0 / 3.6 / 0.0 |
| fanOutRepeat K100 M100 | 0.4 / 1.2 / 0.4 / 0.0 | 0.4 / 0.4 / 0.4 / 0.0 | 39.6 / 0.0 / 0.4 / 0.0 | 40.8 / 0.0 / 0.4 / 0.0 |
| fanOutRepeat K1000 M1 | 20.0 / 60.0 / 20.0 / 0.0 | 20.0 / 20.0 / 20.0 / 0.0 | 20.0 / 0.0 / 20.0 / 0.0 | 80.0 / 0.0 / 20.0 / 0.0 |
| fanOutRepeat K1000 M10 | 3.6 / 10.9 / 3.6 / 0.0 | 3.6 / 3.6 / 3.6 / 0.0 | 36.4 / 0.0 / 3.6 / 0.0 | 47.4 / 0.0 / 3.6 / 0.0 |
| fanOutRepeat K1000 M100 | 0.4 / 1.2 / 0.4 / 0.0 | 0.4 / 0.4 / 0.4 / 0.0 | 40.4 / 0.0 / 0.4 / 0.0 | 41.6 / 0.0 / 0.4 / 0.0 |
| sameRecordWrites | 3.0 / 13.6 / 1.5 / 0.0 | 3.0 / 7.6 / 1.5 / 0.0 | 5.5 / 0.0 / 1.5 / 0.0 | 7.1 / 0.0 / 1.5 / 2.0 |
| writeThenDispose | 2.0 / 6.0 / 0.0 / 0.0 | 2.0 / 4.0 / 0.0 / 0.0 | 2.0 / 6.0 / 0.0 / 0.0 | 2.0 / 8.0 / 0.0 / 0.0 |
| writeThenDispose control | 0.0 / 6.0 / 0.0 / 0.0 | 0.0 / 6.0 / 0.0 / 0.0 | 0.0 / 6.0 / 0.0 / 0.0 | 0.0 / 8.0 / 0.0 / 0.0 |
| noObservers | 0.0 / 0.0 / 0.0 / 0.0 | 0.0 / 0.0 / 0.0 / 0.0 | 4.0 / 0.0 / 0.0 / 0.0 | 4.0 / 0.0 / 0.0 / 0.0 |
| fanIn F100 | 0.1 / 15.0 / 0.1 / 0.0 | 0.1 / 10.0 / 0.1 / 0.0 | 0.1 / 0.0 / 0.1 / 0.0 | 0.2 / 0.0 / 0.1 / 2.5 |
| fanIn F1000 | 0.0 / 15.0 / 0.0 / 0.0 | 0.0 / 10.0 / 0.0 / 0.0 | 0.0 / 0.0 / 0.0 / 0.0 | 0.0 / 0.0 / 0.0 / 2.5 |
| manyWritesThenPull | 0.2 / 61.4 / 0.2 / 0.0 | 0.2 / 40.8 / 0.2 / 0.0 | 20.4 / 0.2 / 0.2 / 0.0 | 41.2 / 0.2 / 0.2 / 0.0 |
| largeGraphSmallPull | 0.0 / 0.1 / 0.0 / 0.0 | 0.0 / 0.0 / 0.0 / 0.0 | 1.1 / 0.0 / 0.0 / 0.0 | 1.1 / 0.0 / 0.0 / 0.0 |
| deepChainCutoff | 2.0 / 0.0 / 0.0 / 2.0 | 2.0 / 0.0 / 0.0 / 2.0 | 2.0 / 0.0 / 0.0 / 2.0 | 2.0 / 0.0 / 0.0 / 2.0 |
| dynamicSwitch | 0.5 / 3.0 / 0.5 / 0.0 | 0.5 / 1.5 / 0.5 / 0.0 | 0.5 / 1.0 / 0.5 / 0.0 | 2.0 / 1.0 / 0.5 / 0.0 |

- Baseline and POC-A do 100-500M edge-list operations on the dynamic and kairo tests where POC-B and alien do almost none. That is detach-on-dirty: each mark unlinks every edge of the node and each recompute relinks it.
- Baseline and POC-A visit fewer nodes while marking (25-1000x5: 96M against 182M), because a Dirty node is in no dependents list and a later write does not reach it.
- With the representation matched (POC-A against POC-B: same edge lists, same marking, same stacks), the remaining gap is the edge-list column: 13.5% of POC-A's upstream time (1894.6 -> 1638.0). Changes 4 and 5 then help POC-B and not POC-A, so the final gap is 27%.

## Where each engine wins

One line per case from `bench/cases.ts` (POC-B against POC-A; alien and Preact for reference):

| case | winner | why |
|---|---|---|
| isograph w10 read100% | POC-B (106.0; alien 116.7, POC-A 162.5, Preact 1628.4) | few writes and many pulls of `Check` fragments; POC-B verifies without relinking; Preact's computeds that no effect observes are unsubscribed and re-check every dependency on each read |
| isograph w10, w100 read10% | POC-B (6.0, 1.9), POC-A (7.6, 1.9; alien 8.5, 3.3) | only the pulled tops recompute; POC-A reaches an unpulled `Dirty` fragment on its first write only, POC-B skips it in O(1), alien sets its `Pending` flag again |
| isograph w100, w1000 read100% | POC-B (48.0, 23.5; alien 55.1, 29.8) | as w10; POC-A pays one unlink and one relink per edge per recompute |
| isograph w10000 read100% | POC-B (9.7; Preact 10.5, POC-A 14.1, alien 15.7) | a write-heavy batch then one read; alien walks every write's readers again, POC-B skips nodes that are already `Dirty` in O(1) |
| isograph w10000 read10% | Preact (3.1; POC-B 3.2, POC-A 3.3, alien 6.6) | 90% of the graph is never read again; Preact keeps unobserved computeds unsubscribed, pico-js marks them once and then skips them, alien walks them on every write |
| fanOutRepeat M1 (K10, K100, K1000) | POC-B at K10 (27.0), Preact at K100, K1000 (32.5, 35.2) | one write then one pull; same work in every engine, POC-A adds the relink of each reader |
| fanOutRepeat M10 | Preact (6.1-7.7; POC-A 8.2-9.8, POC-B 9.0-14.4) | Preact's readers are unobserved computeds, so a write does not visit them; POC-A visits them on the first write only |
| fanOutRepeat M100 | Preact (0.8-2.8), then POC-A (1.1-4.1; POC-B 4.2-5.9, alien 5.1-12.2) | detach-on-dirty: in POC-A the writes after the first reach no reader; POC-B and alien visit K readers per write |
| sameRecordWrites | POC-B (7.3; Preact 9.0, POC-A 10.5, alien 10.8) | every field of every record is written: POC-B marks each resolver once and skips it on its other fields; POC-A pays the unlink of the other fields' edges on the first one |
| writeThenDispose (minus control) | POC-A, POC-B (-0.4, 0.0; alien 1.0, Preact 1.9) | the write marks 1000 effects without running anything, and the dispose in the same batch drops them; alien and Preact queue each effect |
| noObservers | POC-A, Preact (0.6; POC-B 1.0, alien 1.9) | expected loss, measured win for POC-A: after the first write every computed is `Dirty` and detached, so later writes reach nothing |
| fanIn F100, F1000 | POC-B (1.6, 1.9; Preact 2.9, 4.1, POC-A 4.1, 5.3, alien 4.5, 4.7) | expected loss, measured win for POC-B: a recompute that reads the same F sources compares one pointer per read; POC-A unlinks and relinks F edges per write |
| manyWritesThenPull | Preact (13.5; POC-A 19.4, POC-B 21.5, alien 50.3) | 100 writes to sources shared by 1000 nodes before one pull; POC-A visits the nodes on the first write only, POC-B skips `Dirty` nodes per write, alien re-walks |
| largeGraphSmallPull | POC-A, Preact (0.1; POC-B 0.7, alien 0.8) | 10 of 10,000 leaves are read per write; in POC-A the unread leaves are `Dirty` and detached after the first write |
| deepChainCutoff | Preact (2.0; alien 2.1, POC-B 2.4, POC-A 3.1) | eager `Check` marking of 1000 nodes then verification down the chain; Preact's version check skips the walk |
| dynamicSwitch | Preact (1.7; POC-A 2.3, POC-B 2.4, alien 2.4) | every node changes its dependency each write; edge replacement costs about the same in all |

The reasons given for Preact come from its design (a computed with no subscriber does not subscribe to its sources and re-checks them on read by version). Preact was not instrumented.

## Proposed engine changes

Before snippets are pico-js.md's `graph.ts`. After snippets are POC-A (changes 1, 2) and POC-B (changes 3-5). The full files are under POC sources.

### Change 1: intrusive edge lists (POC-A, semantics kept; branch commit `e533cdd66`)

Measured: upstream 6198.4 -> 1950.5 ms, updateSignals 1200.9 -> 241.9 ms, createComputations 150.1 -> 25.5 ms, createSignals 5.9 -> 0.6 ms.

```ts
// from libs/pico-js/src/graph.ts, before
export type SourceNode<TValue> = {
  value: TValue;
  readonly dependents: Set<DerivedNode<unknown>>;
};

export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  readonly sourceDependencies: Array<SourceNode<unknown>>;
  readonly derivedDependencies: Array<DerivedNode<unknown>>;
  readonly dependents: Set<DerivedNode<unknown>>;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

function attach(node: DerivedNode<unknown>): void {
  for (const source of node.sourceDependencies) {
    source.dependents.add(node);
  }
  for (const dependency of node.derivedDependencies) {
    dependency.dependents.add(node);
  }
}

function detach(node: DerivedNode<unknown>): void {
  for (const source of node.sourceDependencies) {
    source.dependents.delete(node);
  }
  for (const dependency of node.derivedDependencies) {
    dependency.dependents.delete(node);
  }
  node.sourceDependencies.length = 0;
  node.derivedDependencies.length = 0;
}
```

```ts
// from libs/pico-js/src/graph.ts, after
type Edge<TDependency> = {
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  nextDependency: Edge<TDependency> | undefined;
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

type SourceEdge = Edge<SourceNode<unknown>>;
type DerivedEdge = Edge<DerivedNode<unknown>>;

export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
};

export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  firstSourceDependency: SourceEdge | undefined;
  lastSourceDependency: SourceEdge | undefined;
  firstDerivedDependency: DerivedEdge | undefined;
  lastDerivedDependency: DerivedEdge | undefined;
  firstDependent: DerivedEdge | undefined;
  lastDependent: DerivedEdge | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};
```

`readSource` reuses the next edge of the dependency list and allocates only past its end; `attach` links each read edge at its dependency's tail and drops an edge whose dependency's last dependent is already this node (a dependency read twice); marking cuts off the written node's whole dependents list and unlinks the marked node's other edges in O(1) each (`unlinkSources`, `unlinkDerived`). Full code under POC sources.

Costs and breaks:
- API: `SourceNode.dependents` and `DerivedNode.dependents` (`Set`) become `firstDependent`/`lastDependent`; `sourceDependencies`/`derivedDependencies` become edge lists. The two tests that call `dependents.has` walk the list with a test helper.
- Invariants: unchanged. Every throw stays, worded "dependents list".
- Memory: one edge object (5 fields) per dependency in place of one array slot plus one Set entry.

### Change 2: push only nodes that have dependents (POC-A; branch commit `2b8c508c0`)

Measured: 1950.5 -> 1894.6 ms (-2.9%).

```ts
// from libs/pico-js/src/graph.ts, before
      case 'Clean':
        detach(node);
        node.status = 'Dirty';
        stack.push(node);
        break;
```

```ts
// from libs/pico-js/src/graph.ts, after
      case 'Clean':
        unlinkSources(node.firstSourceDependency, emptied);
        unlinkDerived(node.firstDerivedDependency, emptied);
        node.status = 'Dirty';
        if (node.firstDependent !== undefined) {
          stack.push(node);
        }
        break;
```

The `Check` loop gets the same guard. Breaks nothing.

### Change 3: keep edges linked while Dirty (POC-B; changes invariants; decision 2)

Measured: POC-A 1894.6 -> POC-B 1638.0 ms (-13.5%) with changes 1 and 2 otherwise equal; the snippets show POC-B's final numeric statuses (change 5); `bench/cases.ts` 527.5 -> 407.5 ms in the final run (POC-B with changes 4 and 5).

```ts
// from libs/pico-js/src/graph.ts, before
function markDependentsDirty(dependents: Set<DerivedNode<unknown>>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  for (const node of dependents) {
    switch (node.status) {
      case 'Clean':
        detach(node);
        node.status = 'Dirty';
        stack.push(node);
        break;
      case 'Check':
        detach(node);
        node.status = 'Dirty';
        break;
      case 'Dirty':
        throw new Error(
          'A Dirty node is in a dependents set. This is indicative of a bug in pico-js.',
        );
```

```ts
// from libs/pico-js/src/graph.ts, after
function markDirty<TDependency>(first: Edge<TDependency>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3:
        node.status = 1;
        if (node.firstDependent !== undefined) {
          stack.push(node);
        }
        break;
      case 2:
        node.status = 1;
        break;
      case 1:
        break;
```

```ts
// from libs/pico-js/src/graph.ts, after
export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  const last = callingNode.lastSourceDependency;
  const next =
    last === undefined
      ? callingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== source) {
      unlinkEdge(next, next.dependency);
      next.dependency = source as SourceNode<unknown>;
      linkEdge(next, source as SourceNode<unknown>);
    }
    callingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== source) {
    // allocate and link a new edge (full code under POC sources)
  }
  return source.value;
}
```

Costs and breaks:
- Invariant: "Marking a node `Dirty` removes it from every dependents set" (pico-js.md Model) no longer holds. A dependents list may hold `Dirty` nodes, which marking skips, so the throw "A Dirty node is in a dependents set" is removed. The throws for `Uncomputed` stay.
- A dependency read at two non-adjacent positions (`a, b, a`) gets two edges and two entries in `a`'s list; marking visits the node twice, the second visit a no-op.
- A node links an edge when it reads, not after its computation returns.
- Observable through the public functions: nothing. Values, statuses, and runs per node equal the baseline on 200 random dynamic graphs with writes, pulls, and disposals (`pocB/test/differential.test.ts`). The test "a Dirty node is not in the dependents set of its other sources" changes its `has` expectation to `true`.
- Performance: repeated writes to the sources of an unpulled node visit it every time (fanOutRepeat M100, noObservers, manyWritesThenPull, largeGraphSmallPull, table above).
- POC-C (`pocC/`) added lazy detach on top (a write that reaches a Dirty node unlinks that edge): upstream 1720.8 against POC-B 1484.7, cases 453.7 against 431.5. Rejected.

### Change 4: iterative verification (POC-B)

Measured: POC-B 1638.0 -> 1566.5 ms (-4.4%; dynamic tests -10%, cellx +5 ms). POC-A 1894.6 -> 1901.8, rejected for POC-A.

```ts
// from libs/pico-js/src/graph.ts, before
function verify<TValue>(node: DerivedNode<TValue>): TValue {
  for (const dependency of node.derivedDependencies) {
    pull(dependency);
    if (node.status === 'Dirty') {
      return recompute(node);
    }
  }
  node.status = 'Clean';
  return computedValue(node);
}
```

```ts
// from libs/pico-js/src/graph.ts, after
function verify<TValue>(root: DerivedNode<TValue>): TValue {
  const verifyStack: Array<DerivedEdge> = [];
  let node: DerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  for (;;) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      if (dependency.status === 2) {
        verifyStack.push(edge);
        node = dependency;
        edge = dependency.firstDerivedDependency;
        continue;
      }
      pull(dependency);
      if (node.status !== 1) {
        edge = edge.nextDependency;
        continue;
      }
      recompute(node);
    } else {
      node.status = 3;
    }
    for (;;) {
      const popped = verifyStack.pop();
      if (popped === undefined) {
        return computedValue(root);
      }
      edge = popped;
      node = edge.dependent;
      if (node.status !== 1) {
        break;
      }
      recompute(node);
    }
    edge = edge.nextDependency;
  }
}
```

Breaks nothing: the order of pulls and recomputes is the recursive version's. A pull of a 100,000-node `Check` chain no longer recurses through `verify`, though `recompute` still recurses through computations.

### Change 5: numeric statuses (POC-B; decision 3)

Measured: POC-B 1566.5 -> 1494.5 ms (-4.6%). POC-A 1913.5 -> 1985.0, rejected for POC-A.

```ts
// from libs/pico-js/src/graph.ts, before
export type Status = 'Uncomputed' | 'Dirty' | 'Check' | 'Clean';
```

```ts
// from libs/pico-js/src/graph.ts, after
/** 0 Uncomputed, 1 Dirty, 2 Check, 3 Clean. */
export type Status = 0 | 1 | 2 | 3;
```

Breaks the public `Status` type and every test that compares `status` to a string.

### Rejected

| change | engine | measured | why |
|---|---|---|---|
| marking a recomputed node's dependents with a dedicated `Check` -> `Dirty` loop and no stack (pico-js-batch.md's `markCheckDependentsDirty`) | A, B | A 1894.6 -> 1894.8, B 1638.0 -> 1633.9 | within drift |
| module-level marking and verification stacks | A, B | A 1970.8 vs local 1954.6, B 1479.5 vs local 1469.6 | no gain, and module state |
| throws moved into one helper function | B | 1479.5 -> 1472.1 | within drift |
| `track` without `try`/`finally` (adapter) | B | no change on six cases | none |
| lazy detach (POC-C) | B | 1484.7 -> 1720.8 | slower |

## Decisions

1. Public node types: decided. The node types expose the edge-list fields as POC-B does (`firstDependent`, `lastDependent`, and the dependency-list fields), and the tests walk the dependents list with a test helper.
2. Detach-on-dirty: decided. pico-js adopts POC-B, which drops detach-on-dirty (1384.3 ms upstream, against alien 1580.7 and Preact 1706.8).
3. Numeric `Status`: decided. The public `Status` is `0 | 1 | 2 | 3`, with no exported constants.
4. Invariant checks cost 4.4% in POC-B. They are kept, as the brief says.
5. pico-js.md specifies POC-B's `graph.ts`, `index.ts`, tests, and README.

## Branch

`../i2-worktrees/pico-js-perf`, detached, not pushed:

- `ba0ea92ea` pico-js: this commit adds libs/pico-js as refactors/pending/runtime/pico-js.md specifies, with the engine test and typecheck hooks and the CI test step. The switches have no default arm ...
- `805f3e780` pico-js: this commit adds bench/, run by `pnpm bench`. The script clones js-reactivity-benchmark at 56eb45e8 into node_modules/.cache, runs its sbench, kairo, cellx, and dynamic suites a...
- `e533cdd66` pico-js: this commit replaces the dependency arrays and dependents Sets with intrusive edge lists. Each edge sits in its dependent's dependency list in read order and, while the dependen...
- `2b8c508c0` pico-js: this commit pushes a node onto the marking stack only when its dependents list is not empty, so a write to the readers of a source that have no readers of their own, such as eff...

## Step log

```text
step 0  baseline pico-js (pico-js.md graph.ts) vs POC-A v1 (intrusive edge lists, edges kept on the node and relinked on recompute, detach-on-dirty kept, module stack, string statuses)
  full upstream suite total ms: pico-js 6587.3 | poc v1 2020.1 | preact 1911.5 | alien 1944.8   (profile/out/r1.json)
measurement from here: each engine runs in its own node process, 3 runs, minimum per test (profile/runall.sh); totals are the upstream suite (sbench, kairo, cellx, dynamic)
ref (profile/out/ref): alien 1673.9 | preact 1826.5 | pico-js baseline 6198.4 | poc v1 1950.5
step 1  push a marked node onto the marking stack only when it has dependents: 1950.5 -> 1894.6 (-2.9%). kept. tests pass.
step 2  dedicated marking of a recomputed node's dependents (Check -> Dirty, no stack): A 1894.6 -> 1894.8, B 1638.0 -> 1633.9. rejected (within noise).
POC-B (changes semantics, see pocB/src/graph.ts header): edges stay linked while Dirty, reused in place when the read at the same position is the same node
step B0 POC-B = POC-A step 1 with kept edges: upstream 1638.0 (alien 1673.9, preact 1826.5)
step 3  iterative verify with an explicit edge stack (alien's checkDirty shape): A 1894.6 -> 1901.8 rejected; B 1638.0 -> 1566.5 (-4.4%) kept (dynamic graphs -10%, cellx +5 ms)
lean adapter (no try/finally in track): no measurable change on 6 cases; not pursued
step 4  numeric statuses (0 Uncomputed, 1 Dirty, 2 Check, 3 Clean; Status becomes 0|1|2|3): B 1566.5 -> 1494.5 (-4.6%) kept; tests adapted
step 4a numeric statuses in A: 1913.5 -> 1985.0 (same-session rerun), rejected for A
POC-C = POC-B + lazy detach (a write that reaches a Dirty node unlinks that edge): upstream B 1484.7 vs C 1720.8, extra B 431.5 vs C 453.7 (profile/out/cC). C wins only fanOutRepeat M100, manyWritesThenPull, largeGraphSmallPull. rejected.
step 5  A with a per-call marking stack (no module state), same session: 1970.8 -> 1954.6, kept
measure: B with no invariant throws and no NOT_SET check: 1481.2 -> 1416.0 (the throws cost 4.4% in B)
step 6  B with per-call stacks (no module state): 1479.5 -> 1469.6 kept; B with throws moved into a helper: 1472.1, rejected (noise)
```

## POC sources

POC-A (`bench/poc/src/graph.ts`, identical to the branch's `libs/pico-js/src/graph.ts`; tests: pico-js.md's tests with `hasDependent`, `poc.test.ts`, and a differential test against the baseline):

```ts
// from libs/pico-js/src/graph.ts
const NOT_SET: unique symbol = Symbol('NOT_SET');

type Option<TValue> = TValue | typeof NOT_SET;

export type Status = 'Uncomputed' | 'Dirty' | 'Check' | 'Clean';

/**
 * An edge from `dependency` to `dependent`. It sits in two lists: the dependent's dependency list (`nextDependency`), in read order, and, while the dependent is attached, the dependency's dependents list (`previousDependent`, `nextDependent`).
 */
type Edge<TDependency> = {
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  nextDependency: Edge<TDependency> | undefined;
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

type SourceEdge = Edge<SourceNode<unknown>>;
type DerivedEdge = Edge<DerivedNode<unknown>>;

export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
};

/**
 * - When `status` is `Uncomputed`, `value` is `NOT_SET`, both dependency lists are empty, and the dependents list is empty.
 * - When `status` is `Dirty`, `value` is the value the last computation returned, the node's edges are in no dependents list, and every node in its dependents list is `Check`.
 * - When `status` is `Check`, `value` is the value the last computation returned, each edge of its dependency lists is in its dependency's dependents list, and every node in its dependents list is `Check`.
 * - When `status` is `Clean`, `value` is the up-to-date value, and each edge of its dependency lists is in its dependency's dependents list.
 * - Every node in a dependents list is `Clean` or `Check`.
 *
 * A `Dirty` node keeps its edges, unlinked from the dependents lists, and its next computation reuses them in read order.
 */
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  firstSourceDependency: SourceEdge | undefined;
  lastSourceDependency: SourceEdge | undefined;
  firstDerivedDependency: DerivedEdge | undefined;
  lastDerivedDependency: DerivedEdge | undefined;
  firstDependent: DerivedEdge | undefined;
  lastDependent: DerivedEdge | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return { value, firstDependent: undefined, lastDependent: undefined };
}

export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return {
    value: NOT_SET,
    status: 'Uncomputed',
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
}

export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined) {
    source.firstDependent = undefined;
    source.lastDependent = undefined;
    markDirty(first, source);
  }
}

export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  const last = callingNode.lastSourceDependency;
  const next =
    last === undefined
      ? callingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    next.dependency = source;
    callingNode.lastSourceDependency = next;
  } else {
    const edge: SourceEdge = {
      dependency: source,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    if (last === undefined) {
      callingNode.firstSourceDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastSourceDependency = edge;
  }
  return source.value;
}

export function readDerived<TValue>(
  callingNode: DerivedNode<unknown>,
  node: DerivedNode<TValue>,
): TValue {
  const value = pull(node);
  const last = callingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? callingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    next.dependency = node;
    callingNode.lastDerivedDependency = next;
  } else {
    const edge: DerivedEdge = {
      dependency: node,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    if (last === undefined) {
      callingNode.firstDerivedDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastDerivedDependency = edge;
  }
  return value;
}

export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  switch (node.status) {
    case 'Clean':
      return computedValue(node);
    case 'Dirty':
      return recompute(node);
    case 'Check':
      return verify(node);
    case 'Uncomputed':
      return computeFirst(node);
  }
}

export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  if (first !== undefined) {
    node.firstDependent = undefined;
    node.lastDependent = undefined;
    markDirty(first, node);
  }
  if (node.status === 'Clean' || node.status === 'Check') {
    unlinkSources(node.firstSourceDependency, undefined);
    unlinkDerived(node.firstDerivedDependency, undefined);
  }
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = 'Uncomputed';
  node.value = NOT_SET;
}

/**
 * Marks `Dirty` every dependent in the list that starts at `first`, which is the whole dependents list of `emptied`, already cut off from it, and marks `Check` every node above them.
 */
function markDirty<TDependency>(
  first: Edge<TDependency>,
  emptied: SourceNode<unknown> | DerivedNode<unknown>,
): void {
  const stack: Array<DerivedNode<unknown>> = [];
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 'Clean':
        unlinkSources(node.firstSourceDependency, emptied);
        unlinkDerived(node.firstDerivedDependency, emptied);
        node.status = 'Dirty';
        if (node.firstDependent !== undefined) {
          stack.push(node);
        }
        break;
      case 'Check':
        unlinkSources(node.firstSourceDependency, emptied);
        unlinkDerived(node.firstDerivedDependency, emptied);
        node.status = 'Dirty';
        break;
      case 'Dirty':
        throw new Error(
          'A Dirty node is in a dependents list. This is indicative of a bug in pico-js.',
        );
      case 'Uncomputed':
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
  let marked: DerivedNode<unknown> | undefined;
  while ((marked = stack.pop()) !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 'Clean':
          dependent.status = 'Check';
          if (dependent.firstDependent !== undefined) {
            stack.push(dependent);
          }
          break;
        case 'Check':
          break;
        case 'Dirty':
          throw new Error(
            'A Dirty node is in a dependents list. This is indicative of a bug in pico-js.',
          );
        case 'Uncomputed':
          throw new Error(
            'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
          );
      }
    }
  }
}

function verify<TValue>(node: DerivedNode<TValue>): TValue {
  let edge = node.firstDerivedDependency;
  while (edge !== undefined) {
    pull(edge.dependency);
    if (node.status === 'Dirty') {
      return recompute(node);
    }
    edge = edge.nextDependency;
  }
  node.status = 'Clean';
  return computedValue(node);
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  attach(node);
  node.status = 'Clean';
  node.value = value;
  return value;
}

function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const previous = computedValue(node);
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  attach(node);
  node.status = 'Clean';
  if (value === previous) {
    return previous;
  }
  node.value = value;
  const first = node.firstDependent;
  if (first !== undefined) {
    node.firstDependent = undefined;
    node.lastDependent = undefined;
    markDirty(first, node);
  }
  return value;
}

function computedValue<TValue>(node: DerivedNode<TValue>): TValue {
  if (node.value === NOT_SET) {
    throw new Error(
      'A node that is not Uncomputed has no value. This is indicative of a bug in pico-js.',
    );
  }
  return node.value;
}

/**
 * Drops the edges after the last one the computation read, and links each remaining edge into its dependency's dependents list. A second edge to the same dependency is dropped, because the dependency's last dependent is then this node.
 */
function attach(node: DerivedNode<unknown>): void {
  const lastSource = node.lastSourceDependency;
  if (lastSource === undefined) {
    node.firstSourceDependency = undefined;
  } else {
    lastSource.nextDependency = undefined;
    let previous: SourceEdge | undefined = undefined;
    let edge: SourceEdge | undefined = node.firstSourceDependency;
    while (edge !== undefined) {
      const dependency = edge.dependency;
      const tail = dependency.lastDependent;
      if (tail !== undefined && tail.dependent === node) {
        if (previous === undefined) {
          node.firstSourceDependency = edge.nextDependency;
        } else {
          previous.nextDependency = edge.nextDependency;
        }
        if (edge === lastSource) {
          node.lastSourceDependency = previous;
        }
      } else {
        edge.previousDependent = tail;
        edge.nextDependent = undefined;
        if (tail === undefined) {
          dependency.firstDependent = edge;
        } else {
          tail.nextDependent = edge;
        }
        dependency.lastDependent = edge;
        previous = edge;
      }
      edge = edge.nextDependency;
    }
  }
  const lastDerived = node.lastDerivedDependency;
  if (lastDerived === undefined) {
    node.firstDerivedDependency = undefined;
  } else {
    lastDerived.nextDependency = undefined;
    let previous: DerivedEdge | undefined = undefined;
    let edge: DerivedEdge | undefined = node.firstDerivedDependency;
    while (edge !== undefined) {
      const dependency = edge.dependency;
      const tail = dependency.lastDependent;
      if (tail !== undefined && tail.dependent === node) {
        if (previous === undefined) {
          node.firstDerivedDependency = edge.nextDependency;
        } else {
          previous.nextDependency = edge.nextDependency;
        }
        if (edge === lastDerived) {
          node.lastDerivedDependency = previous;
        }
      } else {
        edge.previousDependent = tail;
        edge.nextDependent = undefined;
        if (tail === undefined) {
          dependency.firstDependent = edge;
        } else {
          tail.nextDependent = edge;
        }
        dependency.lastDependent = edge;
        previous = edge;
      }
      edge = edge.nextDependency;
    }
  }
}

/** Unlinks each edge from its dependency's dependents list, except the edges into `emptied`, whose list the caller already cut off. */
function unlinkSources(
  first: SourceEdge | undefined,
  emptied: SourceNode<unknown> | DerivedNode<unknown> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    const dependency = edge.dependency;
    if (dependency !== emptied) {
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
    edge = edge.nextDependency;
  }
}

function unlinkDerived(
  first: DerivedEdge | undefined,
  emptied: SourceNode<unknown> | DerivedNode<unknown> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    const dependency = edge.dependency;
    if (dependency !== emptied) {
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
    edge = edge.nextDependency;
  }
}
```

POC-B (`bench/pocB/src/graph.ts`; changes invariants, see change 3):

```ts
// from libs/pico-js/src/graph.ts
// POC-B: changes pico-js semantics. A Dirty node keeps its edges linked in its dependencies' dependents lists,
// and a recompute keeps every edge whose dependency is the same one at the same read position.
// Changed invariants: a dependents list may hold Dirty nodes (marking skips them), and may hold a node twice when a
// computation reads a dependency at two non-adjacent positions. A node links an edge when it reads, not after its
// computation returns. Values, statuses, and runs per node match pico-js.

const NOT_SET: unique symbol = Symbol('NOT_SET');

type Option<TValue> = TValue | typeof NOT_SET;

/** 0 Uncomputed, 1 Dirty, 2 Check, 3 Clean. */
export type Status = 0 | 1 | 2 | 3;

type Edge<TDependency> = {
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  nextDependency: Edge<TDependency> | undefined;
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

type SourceEdge = Edge<SourceNode<unknown>>;
type DerivedEdge = Edge<DerivedNode<unknown>>;

type Producer<TDependency> = {
  firstDependent: Edge<TDependency> | undefined;
  lastDependent: Edge<TDependency> | undefined;
};

export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
};

export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  firstSourceDependency: SourceEdge | undefined;
  lastSourceDependency: SourceEdge | undefined;
  firstDerivedDependency: DerivedEdge | undefined;
  lastDerivedDependency: DerivedEdge | undefined;
  firstDependent: DerivedEdge | undefined;
  lastDependent: DerivedEdge | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return { value, firstDependent: undefined, lastDependent: undefined };
}

export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return {
    value: NOT_SET,
    status: 0,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
}

export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
}

function linkEdge<TDependency extends Producer<TDependency>>(
  edge: Edge<TDependency>,
  dependency: Producer<TDependency>,
): void {
  const tail = dependency.lastDependent;
  edge.previousDependent = tail;
  edge.nextDependent = undefined;
  if (tail === undefined) {
    dependency.firstDependent = edge;
  } else {
    tail.nextDependent = edge;
  }
  dependency.lastDependent = edge;
}

function unlinkEdge<TDependency>(
  edge: Edge<TDependency>,
  dependency: Producer<TDependency>,
): void {
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

export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  const last = callingNode.lastSourceDependency;
  const next =
    last === undefined
      ? callingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== source) {
      unlinkEdge(next, next.dependency);
      next.dependency = source as SourceNode<unknown>;
      linkEdge(next, source as SourceNode<unknown>);
    }
    callingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== source) {
    const edge: SourceEdge = {
      dependency: source as SourceNode<unknown>,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge, source as SourceNode<unknown>);
    if (last === undefined) {
      callingNode.firstSourceDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastSourceDependency = edge;
  }
  return source.value;
}

export function readDerived<TValue>(
  callingNode: DerivedNode<unknown>,
  node: DerivedNode<TValue>,
): TValue {
  const value = pull(node);
  const last = callingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? callingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== node) {
      unlinkEdge(next, next.dependency);
      next.dependency = node as DerivedNode<unknown>;
      linkEdge(next, node as DerivedNode<unknown>);
    }
    callingNode.lastDerivedDependency = next;
  } else if (last === undefined || last.dependency !== node) {
    const edge: DerivedEdge = {
      dependency: node as DerivedNode<unknown>,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge, node as DerivedNode<unknown>);
    if (last === undefined) {
      callingNode.firstDerivedDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastDerivedDependency = edge;
  }
  return value;
}

export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  switch (node.status) {
    case 3:
      return computedValue(node);
    case 1:
      return recompute(node);
    case 2:
      return verify(node);
    case 0:
      return computeFirst(node);
  }
}

export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
  unlinkAll(node.firstSourceDependency);
  unlinkAll(node.firstDerivedDependency);
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = 0;
  node.value = NOT_SET;
}


function markDirty<TDependency>(first: Edge<TDependency>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3:
        node.status = 1;
        if (node.firstDependent !== undefined) {
          stack.push(node);
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
  let marked: DerivedNode<unknown> | undefined;
  while ((marked = stack.pop()) !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3:
          dependent.status = 2;
          if (dependent.firstDependent !== undefined) {
            stack.push(dependent);
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
  }
}


/**
 * Pulls the derived dependencies of `root` in read order, and recomputes a node after the first pull that leaves it `Dirty`. A `Check` dependency is verified the same way before its reader continues, with an explicit stack in place of recursion.
 */
function verify<TValue>(root: DerivedNode<TValue>): TValue {
  const verifyStack: Array<DerivedEdge> = [];
  let node: DerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  for (;;) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      if (dependency.status === 2) {
        verifyStack.push(edge);
        node = dependency;
        edge = dependency.firstDerivedDependency;
        continue;
      }
      pull(dependency);
      if (node.status !== 1) {
        edge = edge.nextDependency;
        continue;
      }
      recompute(node);
    } else {
      node.status = 3;
    }
    for (;;) {
      const popped = verifyStack.pop();
      if (popped === undefined) {
        return computedValue(root);
      }
      edge = popped;
      node = edge.dependent;
      if (node.status !== 1) {
        break;
      }
      recompute(node);
    }
    edge = edge.nextDependency;
  }
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  trim(node);
  node.status = 3;
  node.value = value;
  return value;
}

function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const previous = computedValue(node);
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = 3;
  if (value === previous) {
    return previous;
  }
  node.value = value;
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
  return value;
}

function computedValue<TValue>(node: DerivedNode<TValue>): TValue {
  if (node.value === NOT_SET) {
    throw new Error(
      'A node that is not Uncomputed has no value. This is indicative of a bug in pico-js.',
    );
  }
  return node.value;
}

/** Unlinks and drops the edges after the last one the computation read. */
function trim(node: DerivedNode<unknown>): void {
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
    unlinkAll(node.firstDerivedDependency);
    node.firstDerivedDependency = undefined;
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkAll(lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
  }
}

function unlinkAll<TDependency extends Producer<TDependency>>(
  first: Edge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge, edge.dependency);
    edge = edge.nextDependency;
  }
}
```
