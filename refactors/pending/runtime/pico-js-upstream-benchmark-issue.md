Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the ten graph operations below, and each of them occurs in large apps. Would you accept PRs that add them?

| Case | Operation | What it measures | Needs |
| --- | --- | --- | --- |
| 1 | Two effects swap which of two computeds they read, in one batch | Recomputations of the two computeds when only edges move | - |
| 2 | Dispose every effect that reads a computed, then create new effects that read it, with no write in between, and the opposite order | Recomputations of the computeds, and time | effect returns a disposer |
| 3 | Create and dispose computeds and effects over a shared computed while upstream writes happen between | Time, recomputations of the shared computed, and retained memory | effect returns a disposer |
| 4 | One batch writes N sources | Time and marking visits against writes per batch and observer count | - |
| 5 | Many computeds read one source, only a fraction have effects, the rest are read once and never again, and a fraction change value per write | Computation counts and time against the observed and changed fractions | - |
| 6 | Many writes with no effect, then one read of a computed over many layers | Time and visits against writes before the read | - |
| 7 | One computed reads the same computeds in a different order, with new ones in front, or one computed moves from one reading computed to another | Recomputations of the moved computeds, and time | - |
| 8 | One effect switches among T computed chains whose inputs never change | Recomputations of the chains on return to a branch, and time | - |
| 9 | A computed returns its previous object so its readers cut off, at varying chain position | Recomputations below the cutoff, and time | - |
| 10 | One batch writes the same source N times | Time and marking visits against N | - |

## Interface

The only extension is that `effect` returns its disposer, which cases 2 and 3 need. `cleanup()` ends every effect of a run at once, so it cannot express disposing some effects while other effects that read the same computeds stay alive. An adapter whose library cannot dispose a single effect skips these two cases.

## Cases

In every diagram, `[x]` is a signal, `(x)` is a computed, `<x>` is an effect, and an arrow points from a node to a node that reads it. `==>` marks the one edge that changes in a before and after pair.

### 1. Two effects swap which of two computeds they read

```text
Before (x = true)                  After (x = false)

[s] --+--> (a) ==> <E1>            [s] --+--> (a) ==> <E2>
      |                                  |
      +--> (b) ==> <E2>                  +--> (b) ==> <E1>

[x] --> <E1>, <E2>                 [x] --> <E1>, <E2>
```

- `s` is the one source of `a` and `b`, and each of `a` and `b` costs a tunable amount. `x` is the toggle. `E1` reads `x` and then `a` when `x` is true, otherwise `b`. `E2` reads `x` and then `b` when `x` is true, otherwise `a`.
- Each iteration writes `x` once in a batch. Both effects rerun, the edges `a` to `E1` and `b` to `E2` are removed, and the edges `b` to `E1` and `a` to `E2` are added. In the same flush each of `a` and `b` loses its only reader and gains the other effect as its new reader.
- A variant makes `a` and `b` the heads of chains of depth `d`, swept over a few values, so the value a node drops when its reader leaves is worth more as `d` grows.
- It measures time per toggle and runs of `a` and `b` per toggle. `s` never changes, so both values are still valid and the minimum is 0 runs.
- The nearest existing case is `unstable`, which switches one reader between two computeds. A single `head` both drives the switch and feeds both branches there, so every switch needs a recompute and a dropped value costs nothing extra. The dynamic family never has two readers exchange a computed.

### 2. Dispose the effects that read a computed, then create new effects that read it

```text
        +--> (c[0])   --> <A[j]> or <B[j]>, for j mod S = 0
[s] ----+--> (c[1])   --> <A[j]> or <B[j]>, for j mod S = 1
        +--> (c[S-1]) --> <A[j]> or <B[j]>, for j mod S = S-1

Live effects after each step (j runs from 0 to R-1):
  Dispose first:  {A} -> {}     -> {B}   (no c[i] has a reader in between)
  Create first:   {A} -> {A, B} -> {B}   (every c[i] keeps a reader)
```

- There is one source `s` and `S` computeds `c[0]` to `c[S-1]` that each read `s` and cost a tunable amount. There are `R` effects `A[0]` to `A[R-1]` and `R` effects `B[0]` to `B[R-1]`. Effects `A[j]` and `B[j]` each read `c[j mod S]`.
- Each iteration disposes every effect of one set and creates every effect of the other set, with no write in between, in one of two orders. In dispose first, all old effects are disposed and then all new effects are created. In create first, all new effects are created and then all old effects are disposed.
- Only dispose first leaves a computed with no readers for a moment, so only that order tests whether a computed keeps its value after its last reader leaves.
- We sweep `S` and `R`, and the cost of each `c[i]`.
- It measures runs of the `c[i]` per iteration and time per iteration. `s` never changes, so the minimum is 0 runs of the `c[i]`. Each new effect runs once in any case.
- The nearest existing cases are `createComputations` and the build-and-dispose cases, which end every lifetime at once through `cleanup`. None disposes some readers of a computed while others are created.

### 3. Create and dispose computeds and effects over a shared computed while upstream writes happen

```text
Between cycles:     [s] --> (shared)

During a cycle, rows 1 to R:
                              +--> (row[1]) --> <Er[1]>
    [s] --> (shared) ---------+--> (row[2]) --> <Er[2]>
                              +--> (row[R]) --> <Er[R]>

Steps of one cycle:
  1 create rows and effects   2 write [s] W times   3 dispose the effects
```

- There is one source `s` and one computed `shared` that reads `s`. Each cycle creates `R` computeds `row[1]` to `row[R]`, each reading `shared`, and `R` effects `Er[1]` to `Er[R]`, each reading its row.
- Each cycle creates the rows and effects, which run once. It then writes `s` `W` times, each write in its own batch, which reruns `shared`, every row, and every effect after each write. It then disposes the `R` effects, which leaves `shared` with no readers again, and drops the rows.
- We sweep `R` from 10 to 1,000 and `W` from 1 to 10.
- It measures time per cycle, runs of `shared` per cycle, and heap retained after many cycles.
- `s` does not change between the last write of one cycle and the creation in the next, so the minimum is `W` runs of `shared` per cycle. A graph that drops the value of `shared` when its last reader leaves runs it once more during creation.
- `createComputations` also creates effects, but over fresh signals, and disposes the whole build with no write between creation and disposal. This case needs `shared` to gain and lose readers repeatedly while upstream writes happen in between.

### 4. One batch writes N sources

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

### 5. Many computeds over one source, only a fraction with effects

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

### 6. Many writes with no effect, then one read

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

### 7. A computed reads the same computeds in a different order, or one computed moves between reading computeds

```text
Reorder and prepend
[src 1] --> (item[1]) --+
[src 2] --> (item[2]) --+
   ...                  +--> (list) --> <E>
[src N] --> (item[N]) --+
[order] ----------------+

Read order of (list), rotation by 2:
  Before:  order, item[1], item[2], item[3], ..., item[N]
  After:   order, item[3], item[4], ..., item[N], item[1], item[2]

Move
Before                                 After

[src i] --> (item[i]) ==> (group[k])   [src i] --> (item[i]) ==> (group[k+1])

[g[k]]   --> (group[k])   --> <E[k]>   [g[k]]   --> (group[k])   --> <E[k]>
[g[k+1]] --> (group[k+1]) --> <E[k+1]> [g[k+1]] --> (group[k+1]) --> <E[k+1]>

Every other (item[j]) keeps its edge to its own group, and N items in total.
```

- For reorder and prepend there are `N` item computeds over `N` sources, one `order` signal that holds item ids, one `list` computed that reads `order` and then the items in the order `order` gives, and one effect `E`. The prepend variant takes its new item ids from a pool of further items, and the window of `N` ids moves by `k` per iteration.
- Each reorder iteration writes `order` rotated by a shift, or reversed, in one batch. `list` and `E` rerun. The set of edges into `list` is the same, but the order in which `list` reads them differs. Each prepend iteration writes `order` with `k` ids added in front and `k` ids removed from the end, in one batch.
- For move there are `G` signals `g[0]` to `g[G-1]`, where `g[k]` holds the array of item ids in group `k`. Each computed `group[k]` reads `g[k]` and then `item[i]` for each id in that array, and one effect `E[k]` reads `group[k]`. Each iteration removes the first id `i` from `g[k]` and prepends it to `g[k+1]`, with both writes in one batch, and `k` advances each iteration. Both groups and both effects rerun. The edge from `item[i]` to `group[k]` is removed and the edge from `item[i]` to `group[k+1]` is added.
- Move runs twice, once writing `g[k]` first and once writing `g[k+1]` first, because the order decides whether `item[i]` ever has no readers.
- We sweep `N` from 10 to 10,000, and `k` from 1 to `N`.
- It measures time per iteration and item runs per iteration. No item source changes, so the minimum for reorder and move is 0 item runs.
- A node in the dynamic family skips one source and reads the rest in the same order, and `unstable` switches one reader between two computeds. None changes the order of reads, and none moves a computed from one live reader to another, so none charges for tracking a reordered or moved dependency.

### 8. One effect switches among T computed chains whose inputs never change

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

- There are `T` chains. Chain `t` is `pt[1]` to `pt[d]` of depth `d` over its own source. One effect `E` reads `tab` and then the head `pt[d]` of the selected chain. The chains that are not selected keep their edges and only lose the edge to `E`.
- Each iteration writes `tab` to the next value, cyclically, in one batch. `E` reruns, the edge from the old head to `E` is removed, and the edge from the new head to `E` is added.
- A variant adds one effect per chain, so every chain stays observed, and measures what keeping unselected chains alive costs.
- We sweep `T` and `d`.
- It measures time per switch and chain runs per switch after the first visit to each chain. Nothing an unselected chain reads changed, so the minimum is 0 chain runs.
- `unstable` flips on every write, but the flip and the branch values come from the same `head`, so it never returns to a branch whose inputs did not change.

### 9. A computed returns its previous object so its readers cut off

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

- In the fan-out shape there are two sources, one `view` computed that reads both and returns its previous object when the title and the day of `updatedAt` are equal, and `R` reader computeds `reader[r]` that read `view`, each with one effect.
- Each iteration advances `updatedAt` by an amount that stays within one day, in one batch. `view` reruns, finds an equal result, and returns the previous object. The readers and effects below it should not run.
- In the chain shape the cutoff node `n[p]` is at the first, middle, or last position. Nodes above it and `n[p]` itself must run to find that the result is unchanged. Nodes below it should not.
- We sweep `R` and `D`, and the position `p`.
- It measures time per write and runs below `view` or below `n[p]`, with a minimum of 0.
- `avoidablePropagation` cuts off with a constant, and `mux` builds a new object on every run so its readers never cut off. No case returns the previous object, and none varies the cutoff position, so none shows how the cost of a cutoff depends on how much graph lies below it.

### 10. One batch writes the same source N times

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

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC at each stage of a build of `N` sources, then `N` computeds each read once, then `N` effects, then after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds. Case 3 would also report heap retained after many cycles.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process, and an option that repeats the processes and reports the median process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- Counts: computation counts are asserted in some cases but never reported. We propose an optional count on each result, printed next to the time, because several cases above are about counts that a time alone does not show.
- First run: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.

We can open one PR with the cases that need no interface change (1 and 4 to 10), one PR with the disposer extension and cases 2 and 3, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite. Which of these would you accept?
