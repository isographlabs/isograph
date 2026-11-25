Proposal: benchmark cases for workloads of large apps

These workloads are common in large apps and the suite does not model them. Would you accept PRs that add them?

| Case | App pattern | What it measures | Needs |
| --- | --- | --- | --- |
| 1 | Route navigation | Effects of one screen unmount and effects of the next screen mount over shared computeds, in both orders | effect returns a disposer |
| 2 | Virtualized list churn | Row computeds and effects are created and disposed while data keeps arriving | effect returns a disposer |
| 3 | Normalized response | One batch writes many sources, swept over writes per batch and observer count | - |
| 4 | Partial observation | Many computeds over shared sources, where only a fraction have effects, the rest are read once, and a fraction change value per write | - |
| 5 | Data arrives while a view is hidden | Many writes, then one read, with no effect | - |
| 6 | List mutations | Re-sort, regroup, and prepend over item computeds | - |
| 7 | Tab switch | Return to panels whose inputs did not change while hidden | - |
| 8 | Structural-sharing selectors | A selector returns its previous object when its fields are equal, so readers cut off, at varying chain positions | - |
| 9 | Coalesced input | Many writes to one source within one batch | - |

## Interface

The only extension is that `effect` returns its disposer, which cases 1 and 2 need. `cleanup()` ends every effect of a run at once, so it cannot express disposing some effects while other effects that read the same computeds stay alive. An adapter whose library cannot dispose a single effect skips these two cases.

## Cases

In every diagram, `[x]` is a signal, `(x)` is a computed, `<x>` is an effect, and an arrow points from a node to a node that reads it. `==>` marks the one edge that changes in a before and after pair.

### 1. Route navigation

A route change unmounts the old screen's effects and mounts the new screen's effects, and both screens read computeds that are shared, such as the viewer, a workspace, or users that many rows display.

```text
        +--> (c[0])   --> <A[j]> or <B[j]>, for j mod S = 0
[s] ----+--> (c[1])   --> <A[j]> or <B[j]>, for j mod S = 1
        +--> (c[S-1]) --> <A[j]> or <B[j]>, for j mod S = S-1

Live effects after each step (j runs from 0 to R-1):
  Unmount first:  {A} -> {}     -> {B}   (no c[i] has a reader in between)
  Mount first:    {A} -> {A, B} -> {B}   (every c[i] keeps a reader)
```

- There is one source `s` and `S` shared computeds `c[0]` to `c[S-1]` that each read `s` and cost a tunable amount. Screen A is `R` effects `A[0]` to `A[R-1]`, and screen B is `R` effects `B[0]` to `B[R-1]`. Effects `A[j]` and `B[j]` each read `c[j mod S]`.
- Each iteration disposes the effects of the mounted screen and creates the effects of the other screen, in one of two orders. In unmount first, all effects of the old screen are disposed and then all effects of the new screen are created. In mount first, all effects of the new screen are created and then all effects of the old screen are disposed.
- Only unmount first leaves a shared computed with no observers for a moment, so only that order tests whether a computed keeps its value after its last observer leaves.
- We sweep `R` and `S`, and the cost of each `c[i]`.
- It measures time per navigation and runs of the `c[i]` per navigation. `s` never changes, so the minimum is 0 runs of the `c[i]`. Each new effect runs once in any case.
- The closest existing cases are `createComputations` and the build-and-dispose cases, which end every lifetime at once through `cleanup`. None disposes some observers while others that read the same computeds are created.

### 2. Virtualized list churn

Rows enter and leave a virtualized list while data keeps arriving, so row computeds and effects are created and disposed on a graph that stays live.

```text
Between cycles:     [s] --> (shared)

During a cycle, rows 1 to R:
                              +--> (row[1]) --> <Er[1]>
    [s] --> (shared) ---------+--> (row[2]) --> <Er[2]>
                              +--> (row[R]) --> <Er[R]>

Steps of one cycle:
  1 create rows and effects   2 write [s] W times   3 dispose the effects
```

- There is one source `s` and one computed `shared` that reads `s`. Each cycle creates `R` row computeds `row[1]` to `row[R]`, each reading `shared`, and `R` row effects `Er[1]` to `Er[R]`, each reading its row.
- Each cycle creates the rows and effects, which run once. It then writes `s` `W` times, each write in its own batch, which reruns `shared`, every row, and every effect after each write. It then disposes the `R` effects, which leaves `shared` with no observers again, and drops the rows.
- We sweep `R` from 10 to 1,000 and `W` from 1 to 10.
- It measures time per cycle, runs of `shared` per cycle, and, with the memory measurement in the harness list, heap retained after many cycles.
- `s` does not change between the last write of one cycle and the creation in the next, so the minimum is `W` runs of `shared` per cycle. A graph that drops the value of `shared` when its last observer leaves runs it once more during creation.
- `createComputations` also creates effects, but over fresh signals, and disposes the whole build with no write between creation and disposal. This case needs `shared` to be subscribed and unsubscribed repeatedly while upstream writes happen in between.

### 3. Normalized response

A response is normalized into many records and written in one batch, or a form reset or bulk edit writes many fields at once.

```text
[s1] --+
       +--> (m1) --+
[s2] --+           |
                   +--> <e1>
[s3] --+           |
       +--> (m2) --+--> <e2>
[s4] --+
  ...      ...        ...
[sS] ----> (mM) ----> <eE>

Written in one batch: W distinct sources among s1 to sS.
```

- There are `S` sources, `M` computeds that each read two sources, and `E` effects that each read three computeds.
- Each iteration writes `W` distinct sources in one batch. Every computed that reads a written source, and every effect that reads such a computed, is stale.
- We sweep `W` from 1 to `S`, and `E`, the number of observers, from 1 to 1,000.
- It measures time per batch and, where the harness can count them, marking visits per batch. Each stale computed and each stale effect needs to run at most once per batch, however many of its sources were written.
- The largest batch in the suite today writes 4 sources and every other batch wraps a single write, so no case shows how the cost of a batch grows with the number of writes or with the number of observers.

### 4. Partial observation

A source is read by many computeds, such as selectors. Only the ones on screen have effects, the others were read once while visible and are not read again, and only some of them return a different value after a write.

```text
        +--> (c[1]) --> <E1>     observed, changes on each write
        +--> (c[2]) --> <E2>     observed, constant
[s] ----+--> (c[3])              read once, not observed, changes
        +--> (c[4])              read once, not observed, constant
        +--> (c[K])

o = fraction of the K computeds that have an effect.
c = fraction of the K computeds that change on each write.
```

- There is one source `s` and `K` computeds that each read `s`. A fraction `o` of them has an effect. All of them are read once after the build, and the ones without an effect are never read again. A fraction `c` of them returns a value that changes on each write, and the rest return a constant.
- Each iteration writes `s` once in a batch.
- We sweep `K` from 1 to 10,000, `o` over 0, 0.1, and 1, and `c` over 0, 0.1, 0.5, and 1.
- It measures time per write, computed runs per write, and effect runs per write.
- A computed with an effect reads `s`, so it must rerun to learn whether its value changed, and its effect reruns only when it did. A computed with no effect is not read again. The minimum is `o * K` computed runs and `o * c * K` effect runs per write.
- `update1to1000`, `update1to2`, `update1to4`, `broadPropagation`, and `mux` all observe every reader, and all but `mux` change every reader. None varies either fraction, and none has a node that was read once and then abandoned while still depending on a changing source.

### 5. Data arrives while a view is hidden

Data arrives in many separate writes while a view is hidden, and the view is read once when it returns, with no effect observing it.

```text
[s]  100 sources
 |   each read by 30 of the m on average
 v
(m)  1,000 computeds, 3 sources each
 |   each read by 1 of the g
 v
(g)  100 computeds, 10 of the m each
 |
 v
(root)  1 computed, all 100 g          no effect

Per iteration:  W writes, each in its own batch, then one read of (root).
```

- There are 100 sources, 1,000 computeds `m` that each read three sources, 100 computeds `g` that each read ten of the `m`, and one `root` that reads all 100 `g`. `root` is read once after the build. No effect exists, so a write has no observer to notify.
- Each iteration makes `W` writes, each in its own batch, and then one read of `root`.
- We sweep `W` from 1 to 1,000.
- It measures time per iteration and, where the harness can count them, marking and verification visits per iteration. The read finds which of the 1,101 computeds below and including `root` are out of date and runs each of them at most once, so the number of runs is bounded by 1,101 however large `W` is.
- Every iteration in the suite makes one write before the next read or effect flush, so no case shows how the cost of a read grows with the number of writes that preceded it.

### 6. List mutations

A list is re-sorted, a page of items is prepended to a feed, or an item moves between groups, as a card moves between columns of a board. The item computeds are unchanged, and the computeds that read them see a different order or a different membership.

```text
Re-sort and prepend
[src 1] --> (item[1]) --+
[src 2] --> (item[2]) --+
   ...                  +--> (list) --> <E>
[src N] --> (item[N]) --+
[order] ----------------+

Read order of (list), rotation by 2:
  Before:  order, item[1], item[2], item[3], ..., item[N]
  After:   order, item[3], item[4], ..., item[N], item[1], item[2]

Regroup
Before                                 After

[src i] --> (item[i]) ==> (group[k])   [src i] --> (item[i]) ==> (group[k+1])

[g[k]]   --> (group[k])   --> <E[k]>   [g[k]]   --> (group[k])   --> <E[k]>
[g[k+1]] --> (group[k+1]) --> <E[k+1]> [g[k+1]] --> (group[k+1]) --> <E[k+1]>

Every other (item[j]) keeps its edge to its own group, and N items in total.
```

- For re-sort and prepend there are `N` item computeds over `N` sources, one `order` signal that holds item ids, one `list` computed that reads `order` and then the items in the order `order` gives, and one effect `E`. The prepend variant takes its new item ids from a pool of further items, and the window of `N` ids moves by `k` per iteration.
- Each re-sort iteration writes `order` rotated by a shift, or reversed, in one batch. `list` and `E` rerun. The set of edges into `list` is the same, but the order in which `list` reads them differs. Each prepend iteration writes `order` with `k` ids added in front and `k` ids removed from the end, in one batch.
- For regroup there are `G` signals `g[0]` to `g[G-1]`, where `g[k]` holds the array of item ids in group `k`. Each computed `group[k]` reads `g[k]` and then `item[i]` for each id in that array, and one effect `E[k]` reads `group[k]`. Each iteration removes the first id `i` from `g[k]` and prepends it to `g[k+1]`, with both writes in one batch, and `k` advances each iteration. Both groups and both effects rerun. The edge from `item[i]` to `group[k]` is removed and the edge from `item[i]` to `group[k+1]` is added.
- Regroup runs twice, once writing `g[k]` first and once writing `g[k+1]` first, because the order decides whether `item[i]` ever has no readers.
- We sweep `N` from 10 to 10,000, and `k` from 1 to `N`.
- It measures time per mutation and item runs per mutation. No item source changes, so the minimum for re-sort and regroup is 0 item runs.
- A node in the dynamic family skips one source and reads the rest in the same order, and `unstable` switches one reader between two computeds. None changes the order of reads, and none moves a computed from one live reader to another, so none charges for tracking a reordered or moved dependency.

### 7. Tab switch

A tab bar shows one panel at a time, and each panel is a chain of computeds over sources that do not change while the tab is hidden.

```text
Before (tab = 1)
[src 1] --> (p1[1]) --> ... --> (p1[d]) ==> <E>
[src 2] --> (p2[1]) --> ... --> (p2[d])
[src T] --> (pT[1]) --> ... --> (pT[d])
[tab] ------------------------------------> <E>

After (tab = 2)
[src 1] --> (p1[1]) --> ... --> (p1[d])
[src 2] --> (p2[1]) --> ... --> (p2[d]) ==> <E>
[src T] --> (pT[1]) --> ... --> (pT[d])
[tab] ------------------------------------> <E>
```

- There are `T` panels. Panel `t` is a chain `pt[1]` to `pt[d]` of depth `d` over its own source. One effect `E` reads `tab` and then the head `pt[d]` of the selected panel. The chains of hidden panels keep their edges and only lose the edge to `E`.
- Each iteration writes `tab` to the next tab, cyclically, in one batch. `E` reruns, the edge from the old head to `E` is removed, and the edge from the new head to `E` is added.
- A variant adds one effect per panel, so every panel stays observed, and measures what keeping hidden panels alive costs.
- We sweep `T` and `d`.
- It measures time per switch and chain runs per switch after the first visit to each tab. Nothing a hidden panel reads changed, so the minimum is 0 chain runs.
- `unstable` flips on every write, but the flip and the branch values come from the same `head`, so it never returns to a branch whose inputs did not change.

### 8. Structural-sharing selectors

A selector builds an object from a title and a timestamp formatted to a day, and returns the previous object when the fields are equal, so readers below it should not run.

```text
Fan-out shape
[title] ------+
              +--> (view) --+--> (reader[1]) --> <E1>
[updatedAt] --+             +--> (reader[2]) --> <E2>
                            +--> (reader[R]) --> <ER>

Chain shape (depth D, cutoff at position p)
[s] --> (n1) --> ... --> (n[p]) --> ... --> (n[D]) --> <E>
                           ^
                           this node returns its previous value
```

- In the fan-out shape there are two sources, one `view` computed that reads both and returns its previous object when the title and the day are equal, and `R` reader computeds `reader[r]` that read `view`, each with one effect.
- Each iteration advances `updatedAt` by an amount that stays within one day, in one batch. `view` reruns, finds an equal result, and returns the previous object. The readers and effects below it should not run.
- In the chain shape the cutoff node `n[p]` is at the first, middle, or last position. Nodes above it and `n[p]` itself must run to find that the result is unchanged. Nodes below it should not.
- We sweep `R` and `D`, and the position `p`.
- It measures time per write and runs below `view` or below `n[p]`, with a minimum of 0.
- `avoidablePropagation` cuts off with a constant, and `mux` builds a new object on every run so its readers never cut off. No case returns the previous object, and none varies the cutoff position, so none shows how the cost of a cutoff depends on how much graph lies below it.

### 9. Coalesced input

Several input events are coalesced into one frame, and each event writes the same source with a new value before the frame is processed, as in a drag or typing.

```text
[s] --> (c) --> <E>

One batch:  s = v1, s = v2, ..., s = vW   (W writes, W distinct values)
```

- The graph is one source `s`, one computed `c` over it, and one effect `E`.
- Each iteration writes `s` `W` times with distinct values inside one batch. After the batch, `c` and `E` each need one run, on the last value.
- We sweep `W` from 1 to 1,000.
- It measures time per batch and, where the harness can count them, marking visits per batch. Only the last value is observable, so the ideal cost grows with `W` only through the writes themselves.
- No existing batch writes the same source twice. The multi-write batches in the suite write distinct sources, and every other batch wraps a single write.

## Harness

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC at each stage of a build of `N` sources, then `N` computeds each read once, then `N` effects, then after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds. Case 2 would also report heap retained after many cycles.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process, and an option that repeats the processes and reports the median process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- Counts: computation counts are asserted in some cases but never reported. We propose an optional count on each result, printed next to the time, because several cases above are about counts that a time alone does not show.
- First run: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.

We can open one PR with the cases that need no interface change (3 to 9), one PR with the disposer extension and cases 1 and 2, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite. Which of these would you accept?
