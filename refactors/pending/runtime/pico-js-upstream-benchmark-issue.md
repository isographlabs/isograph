Proposal: benchmark cases for observer lifetimes, multi-write batches, and cutoffs

We should add benchmarks for the cases below. The suite does not run them today. Would you accept PRs that add them?

| Case | App pattern | What it does | Needs |
| --- | --- | --- | --- |
| 1 | Split view swaps its panels | Two effects exchange the computeds they read on one toggle | - |
| 2 | Peek at the current selection | Reader swap where one effect reads a computed without subscribing | untracked read |
| 3 | Kanban card moves between columns | A row leaves one group node and joins another in one batch | - |
| 4 | List re-sort, page prepended to a feed | One computed reads the same items in a different order | - |
| 5 | Tab switch | Returns to a branch whose inputs did not change | - |
| 6 | Route change | Unmounts one screen's effects and mounts the next screen's over shared computeds, in both orders | effect returns a disposer |
| 7 | Virtualized list scroll | Creates and disposes row effects on a live graph while data arrives | effect returns a disposer |
| 8 | Normalized response, bulk edit | One batch writes many sources | - |
| 9 | Drag or typing events in one frame | One batch writes the same source many times | - |
| 10 | Refetch with equal values, optimistic write rolled back | Writes that equal or restore the current value | - |
| 11 | Selector that returns the previous object, day-formatted timestamp | A computed cuts off by returning its previous value, at varying chain positions | - |
| 12 | Retained screens | Computeds that were read once and never read again depend on a changing source | - |
| 13 | Data arrives while a view is hidden | Many separate writes, then one read, with no effect | - |
| 14 | Selectors on one source, some on screen, some changing | Sweeps the observed fraction and the changed fraction over fan-out size | - |
| 15 | Items that share a record | One computed reads the same sources repeatedly, not adjacently | - |
| 16 | Fallback rows until a record arrives | Dependency lists of many effects grow and shrink by a large factor | - |
| 17 | Memory per node | Retained heap per source, computed, and effect, and what stays after dropping them | memory measurement |

## Cases

In every diagram, `[x]` is a signal, `(x)` is a computed, `<x>` is an effect, and an arrow points from a node to a node that reads it. A `..>` arrow is an untracked read, and `==>` marks the one edge that changes in a before and after pair.

### 1. Split view swaps its panels

A split or compare view shows two derived values, one per panel, and a toggle makes each panel show the value the other panel showed.

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
- The case reports time per toggle and runs of `a` and `b` per toggle. `s` never changes, so the minimum is 0 runs: both values are still valid and only the edges move.
- The closest existing case is kairo `unstable`, which switches one reader between two computeds. A single `head` both drives the switch and feeds both branches there, so every switch needs a recompute and a dropped value costs nothing extra. The dynamic family never has two readers exchange a computed.

### 2. Peek at the current selection

An effect reads the current selection without subscribing to it, for logging or an imperative call, while a second effect subscribes to the computed that the selection drops when it changes.

```text
Before (x = true)                  After (x = false)

[s] --+--> (a) --> (r) ..> <E1>    [s] --+--> (a) ==> <E2>
      |                                  |
      +--> (b) ==> <E2>                  +--> (b) --> (r) ..> <E1>

[x] --> (r), <E1>, <E2>            [x] --> (r), <E1>, <E2>
```

- The graph is case 1 with an extra computed `r` that reads `x` and then `a` when `x` is true, otherwise `b`. `E1` reads `x` with a tracked read and reads `r` with an untracked read, so `E1` is not a subscriber of `r`. `E2` reads `x` and then `b` when `x` is true, otherwise `a`.
- Each iteration writes `x` once in a batch. `r` and `E2` are stale, `E1` reruns because of `x`, and the untracked read of `r` pulls its value. `E2` moves from `b` to `a`, and `r` moves from `a` to `b`.
- A reader that holds no subscription still pulls `r`, so `a` and `b` each have a different set of live readers after the toggle. This affects whether `a` keeps its value when its last subscriber leaves.
- The case reports time per toggle and runs of `a` and `b` per toggle. `s` never changes, so the minimum is 0 runs.
- No existing case uses an untracked read, because the suite interface has none. The closest case is again `unstable`, which has tracked reads only.

### 3. Kanban card moves between columns

A card moves from one column to the next, or a re-sort moves a row into another section, and each column or section is a computed over its members' row computeds.

```text
Before                                 After

[src i] --> (row[i]) ==> (group[k])    [src i] --> (row[i]) ==> (group[k+1])

[g[k]]   --> (group[k])   --> <E[k]>   [g[k]]   --> (group[k])   --> <E[k]>
[g[k+1]] --> (group[k+1]) --> <E[k+1]> [g[k+1]] --> (group[k+1]) --> <E[k+1]>

Every other (row[j]) keeps its edge to its own group, and N rows in total.
```

- There are `N` sources, each feeding one row computed `row[i]`. There are `G` signals `g[0]` to `g[G-1]`, where `g[k]` holds the array of row ids in group `k`. Each group computed `group[k]` reads `g[k]` and then `row[i]` for each id in that array. One effect `E[k]` reads each `group[k]`.
- Each iteration removes the first id `i` from `g[k]` and prepends it to `g[k+1]`, with both writes in one batch, and `k` advances each iteration. Both groups rerun and both effects rerun. The edge `row[i]` to `group[k]` is removed and the edge `row[i]` to `group[k+1]` is added.
- The case reports row runs per move and time per move. No source of any row changed, so the minimum is 0 row runs.
- It runs twice, once writing `g[k]` first and once writing `g[k+1]` first, because the order decides whether `row[i]` ever has zero readers.
- The closest existing cases are the dynamic family and `unstable`, but neither moves a computed from one live reader to another.

### 4. List re-sort, page prepended to a feed

A list is re-sorted or a page of items is prepended, so one computed reads the same items in a different order, or the same items plus new ones in front.

```text
[src 1] --> (item[1]) --+
[src 2] --> (item[2]) --+
   ...                  +--> (list) --> <E>
[src N] --> (item[N]) --+
[order] ----------------+

Read order of (list), rotation by 2:
  Before:  order, item[1], item[2], item[3], ..., item[N]
  After:   order, item[3], item[4], ..., item[N], item[1], item[2]
```

- There are `N` item computeds over `N` sources, one `order` signal that holds item ids, one `list` computed that reads `order` and then the items in the order `order` gives, and one effect `E`.
- Each iteration writes `order` rotated by a shift, in one batch. `list` and `E` rerun. The set of edges into `list` is the same, but the order in which `list` reads them differs.
- Variants are a full reversal and a prepend of `k` new items in front. `N` is swept from 10 to 10,000.
- The case reports time per reorder and, where the harness can count them, dependency edges allocated per reorder. No item source changed, so the minimum is 0 item runs for the rotation and the reversal.
- A node in the dynamic family skips one source and reads the rest in the same order, and no existing case changes the order of reads, so none charges for tracking a reordered dependency list.

### 5. Tab switch

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

- There are `T` panels. Panel `t` is a chain `p_t[1]` to `p_t[d]` of depth `d` over its own source. One effect `E` reads `tab` and then the head `p_t[d]` of the selected panel. The chains of hidden panels keep their edges and only lose the edge to `E`.
- Each iteration writes `tab` to the next tab, cyclically, in one batch. `E` reruns, the edge from the old head to `E` is removed, and the edge from the new head to `E` is added.
- A variant adds one effect per panel, so every panel stays observed, and measures what keeping hidden panels alive costs.
- The case reports runs per switch after the first visit to each tab and time per switch. Nothing a hidden panel reads changed, so the minimum is 0 chain runs.
- Kairo `unstable` flips on every write, but the flip and the branch values come from the same `head`, so it never returns to a branch whose inputs did not change.

### 6. Route change

A route change unmounts the old screen's effects and mounts the new screen's effects, and both screens read computeds that are shared, such as the viewer, a workspace, or users that many rows display.

```text
        +--> (c[0])   --> <A[j]> or <B[j]>, for j mod S = 0
[s] ----+--> (c[1])   --> <A[j]> or <B[j]>, for j mod S = 1
        +--> (c[S-1]) --> <A[j]> or <B[j]>, for j mod S = S-1

Live effects after each step (j runs from 0 to R-1):
  Unmount first:  {A} -> {}     -> {B}   (no c[i] has a reader in between)
  Mount first:    {A} -> {A, B} -> {B}   (every c[i] keeps a reader)
```

- There is one source `s` and `S` shared computeds `c[0]` to `c[S-1]` that each read `s` and cost a tunable amount. Screen A is `R` effects `A[0]` to `A[R-1]`, and screen B is `R` effects `B[0]` to `B[R-1]`. Effect `A[j]` and effect `B[j]` each read `c[j mod S]`.
- Each iteration disposes the effects of the mounted screen and creates the effects of the other screen, in one of two orders. In unmount first, all `A[j]` are disposed and then all `B[j]` are created. In mount first, all `B[j]` are created and then all `A[j]` are disposed.
- Only unmount first leaves a shared computed with zero observers for a moment, so only that order tests whether a computed keeps its value after its last observer leaves.
- The case reports runs of the `c[i]` per navigation and time per navigation. `s` never changes, so the minimum is 0 runs. Each new effect runs once in any case.
- The closest existing cases are sbench `createComputations` and the build-and-dispose families, which end every lifetime at once through `cleanup`. None disposes some observers while others that read the same computeds are created.

### 7. Virtualized list scroll

Rows enter and leave a virtualized list while data keeps arriving, so effects are created and disposed on a graph that stays live.

```text
Between cycles:     [s] --> (shared)

During a cycle, rows 1 to R:
                              +--> (row[1]) --> <Er[1]>
    [s] --> (shared) ---------+--> (row[2]) --> <Er[2]>
                              +--> (row[R]) --> <Er[R]>

Steps of one cycle:
  1 create rows and effects   2 write [s]   3 dispose the effects
```

- There is one source `s` and one computed `shared` that reads `s`. Each cycle creates `R` row computeds `row[1]` to `row[R]`, each reading `shared`, and `R` row effects `Er[1]` to `Er[R]`, each reading its row.
- Each iteration creates the rows and effects, which run once. It then writes `s` once in a batch, which reruns `shared`, every row, and every effect. It then disposes the `R` effects, which leaves `shared` with no observers again.
- `s` is unchanged between the write of one cycle and the creation in the next, so the minimum is 1 run of `shared` per cycle, the one the write causes. A graph that drops the value of `shared` when its last observer leaves runs it a second time during creation.
- The case reports time per cycle and, with the retained-memory measurement from H1, heap retained after many cycles.
- Sbench `createComputations` also creates effects, but over fresh signals, and disposes the whole build with no write between creation and disposal. This case needs `shared` to be subscribed and unsubscribed repeatedly while an upstream write happens in between.

### 8. Normalized response, bulk edit

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
- Each iteration writes `W` distinct sources in one batch. We sweep `W` from 1 to `S` and `E` from 1 to 1,000. Every computed that reads a written source, and every effect that reads such a computed, is stale.
- Each stale computed and each stale effect needs to run at most once per batch, however many of its sources were written. The case reports time per batch and, where the harness can count them, marking visits per batch.
- The most writes in one batch in the suite today are the 4 in cellx and the 2 in mol, and every other batch wraps a single write, so no case shows how the cost of a batch grows with the number of writes.

### 9. Drag or typing events in one frame

Several input events are coalesced into one frame, and each event writes the same source with a new value before the frame is processed.

```text
[s] --> (c) --> <E>

One batch:  s = v1, s = v2, ..., s = vW   (W writes, W distinct values)
```

- The graph is one source `s`, one computed `c` over it, and one effect `E`.
- Each iteration writes `s` `W` times with distinct values inside one batch, with `W` swept from 1 to 1,000. After the batch, `c` and `E` each need one run, on the last value.
- Only the last value is observable, so the ideal cost grows with `W` only through the writes themselves. The case reports time per batch and, where it can count them, marking visits per batch.
- No existing batch writes a source twice, since mol writes each of two sources once and cellx writes four different sources.

### 10. Refetch with equal values, optimistic write rolled back

A refetch returns values equal to what is stored, and an optimistic write is undone by the same batch, such as a slider dragged back to its origin before the frame ends.

```text
Revert shape
[s] --> (c) --> (d) --> <E>
One batch:  s = new value, then s = old value

Equal-writes shape
[s1] --+
[s2] --+--> (sum) --> <E>
 ...   |
[sn] --+
One batch:  every s[i] written with its current value
```

- In the revert shape, `c` has a tunable cost, `d` reads `c`, and `E` reads `d`. Each iteration writes a new value to `s` and then writes the old value back, inside one batch.
- In the equal-writes shape, `sum` reads all `n` sources and `E` reads `sum`. Each iteration writes every source with the value it already holds, in one batch.
- At the end of the batch nothing has a value different from before it, so the minimum is 0 runs of `c`, `d`, `sum`, and `E`. The case reports those runs per batch and time per batch.
- Almost every timed write in kairo, sbench, and the dynamic family differs from the source's last value, and none writes a value back within a batch, so none measures a write that changes nothing.

### 11. Selector that returns the previous object, day-formatted timestamp

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
- In the chain shape the cutoff node `n[p]` is at the first, middle, or last position. Nodes above it must run to find that its inputs are unchanged. Nodes below it should not.
- The case reports time per write and runs below `view` or below `n[p]`, with a minimum of 0.
- Kairo `avoidablePropagation` cuts off with a constant, and `mux` builds a new object on every run so its readers never cut off. No case returns the previous object, and none varies the cutoff position, so none shows how the cost of a cutoff depends on how much graph lies below it.

### 12. Retained screens

A user leaves screens in memory, and the computeds of those screens were read once while visible and still depend on sources that later writes change.

```text
        +--> (c[1]) --> <E1>        has an effect
        +--> (c[2]) --> <E2>        has an effect
[s] ----+--> (c[3])                 read once, no reader since
        +--> (c[4])                 read once, no reader since
        +--> (c[K])                 read once, no reader since

A fraction f of the K computeds has an effect.
```

- There is one source `s` and `K` computeds that each read `s`, each read once after the build. A fraction `f` of the computeds also has an effect, and we run `f` at 0, 0.1, and 1.
- Each iteration writes `s` once in a batch. A computed with an effect must rerun, and its effect reruns if the value changed. A computed without an effect is not read again, so the minimum is 0 runs for it.
- The case reports time per write and, where it can count them, marking visits per write.
- In the dynamic family the unread leaves are chosen before the first read and are never read, so no node is both read once and then abandoned. This case covers a node that was read once, has no observer, and still depends on a changing source.

### 13. Data arrives while a view is hidden

Data arrives in many separate writes while a view is hidden, and the view is read once when it returns, with no effect observing it.

```text
[s1..s3]     --> (m1)    --+
[s4..s6]     --> (m2)    --+--> (g1)   --+
  ...                      |             |
[s98..s100]  --> (m1000) --+             +--> (root)    no effect
  ...                                    |
                           (g2..g100) ---+

100 sources s, 1,000 computeds m (3 sources each), 100 computeds g
(10 of the m each), 1 computed root (all 100 g).
Per iteration:  W writes, each in its own batch, then one read of (root).
```

- There are 100 sources, 1,000 layer-1 computeds `m` that each read three sources, 100 layer-2 computeds `g` that each read ten of the `m`, and one `root` that reads all 100 `g`. `root` is read once after the build. No effect exists, so a write has no observer to notify.
- Each iteration makes `W` writes, each in its own batch, and then one read of `root`, with `W` swept from 1 to 1,000. The read has to find which of the 1,101 computeds below `root` are out of date.
- The read runs each out-of-date node at most once, so the number of runs is bounded by 1,101 however large `W` is. The case reports time per iteration and, where it can count them, marking and verification visits.
- Every iteration in the suite makes one write before the next read or effect flush, so no case shows how the cost of a read grows with the number of writes that preceded it.

### 14. Selectors on one source, some on screen, some changing

A source is read by `K` selectors, of which a fraction is on screen and a fraction returns a different value after a write.

```text
        +--> (c[1]) --> <E1>     changes on each write, on screen
        +--> (c[2])              changes on each write, off screen
[s] ----+--> (c[3]) --> <E3>     constant, on screen
        +--> (c[4])              constant, off screen
        +--> (c[K])

c = fraction of K computeds that change.  o = fraction that have an effect.
```

- There is one source `s` and `K` computeds over it. A fraction `c` of them returns a value that changes on each write, and the rest return a constant. A fraction `o` of the computeds has an effect, and all are read once after the build.
- Each iteration writes `s` once in a batch. We sweep `K` from 1 to 10,000, `o` over 0, 0.1, and 1, and `c` over 0, 0.1, 0.5, and 1.
- A computed with an effect must rerun to learn whether its value changed. An effect reruns only when its computed changed. A computed with no effect needs no run, so the minimum is `o * K` computed runs and `o * c * K` effect runs per write.
- The case reports time per write. Sbench `update1to1000`, `update1to2`, and `update1to4`, kairo `broadPropagation`, and `mux` all observe every reader, and all but `mux` change every reader, so none varies either fraction.

### 15. Items that share a record

List items share an author record, or a selector reads the same collection several times, so one computed reads the same sources more than once and not on adjacent reads.

```text
[f1] --+
[f2] --+--> (c) --> <E>
 ...   |
[fF] --+

Read order of (c), P passes:
  f1, f2, ..., fF, f1, f2, ..., fF, ...   (F * P reads, F distinct sources)
```

- There are `F` sources, one computed `c` that reads all `F` in sequence `P` times over, and one effect `E`. `c` has `F * P` reads and only `F` distinct sources.
- Each iteration writes `f1` once in a batch. `c` and `E` each run once. A graph that records one edge per distinct source holds `F` edges for `c`, and a graph that records one edge per read holds `F * P`.
- We sweep `F` from 1 to 100 and `P` from 1 to 30. The case reports time per write and, where it can count them, the dependency edges held by `c`.
- Kairo `repeatedObservers` reads `head` 30 times in a row, which a check against the immediately preceding read already deduplicates, so it never has a read of one dependency separated from the previous read of it.

### 16. Fallback rows until a record arrives

Rows render a fallback while a record is absent, and read its fields and their formatting computeds once a response delivers it.

```text
[field 1] --> (fmt[1]) --+--> <Row[1]>
[field 2] --> (fmt[2]) --+--> <Row[2]>
   ...                   |      ...
[field F] --> (fmt[F]) --+--> <Row[R]>

[present] --> <Row[1]>, <Row[2]>, ..., <Row[R]>

Before (present = false):  each <Row[r]> reads [present] only.
After  (present = true):   each <Row[r]> reads [present] and fmt[1..F].
The F edges from fmt to each row appear on arrival and vanish on removal.
```

- There is a `present` signal, `F` field sources, one formatting computed `fmt[j]` per field, and `R` row effects `Row[1]` to `Row[R]`. Each row effect reads `present` and, when it is true, every `fmt[j]`.
- Each iteration toggles `present` in one batch. On arrival each row's dependency list grows from 1 to `F + 1`, and on removal it shrinks back to 1. We sweep `F` from 1 to 1,000 and `R` from 1 to 1,000.
- No field source changes. The minimum is `F` runs of the formatting computeds on the first arrival, when they are first read, and 0 on later arrivals if they keep their values and 0 on every removal.
- The case reports time per arrival and per removal, and runs of the formatting computeds.
- A node in the dynamic family changes one source per run, so no existing dependency list grows or shrinks by a large factor.

### 17. Memory per node

A long session retains many nodes, and the heap each node costs and what stays after the program drops its references matter.

```text
[s[1]] --> (c[1]) --> <e[1]>
[s[2]] --> (c[2]) --> <e[2]>
  ...
[s[N]] --> (c[N]) --> <e[N]>

Heap readings, each after a forced GC:
  H0 empty    H1 after N sources    H2 after N computeds, each read once
  H3 after N effects    H4 after cleanup and dropping every reference
```

- The case builds `N` sources, then `N` computeds that each read one source and are each read once, then `N` effects. It then calls `cleanup` and drops every reference. After each stage it forces GC and reads the heap, with `N` swept from 1,000 to 100,000.
- Bytes per source is `(H1 - H0) / N`, bytes per computed read once is `(H2 - H1) / N`, bytes per effect is `(H3 - H2) / N`, and bytes retained after the drop is `H4 - H0`. The ideal retained after the drop is 0.
- The suite calls `gc()` between runs and reads no heap figure, and one fork's own README lists memory among what it does not measure, so no case reports retained bytes per node.

## Harness gaps

- H1: no retained-memory measurement, which case 17 needs. The suite calls `gc()` between runs but never reads the heap. We propose reading the heap after a forced GC at each stage of case 17, reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds.
- H2: computation counts are checked in some cases but never reported. The kairo cases assert on a counter, and the dynamic family checks one total count in some configurations. We propose an optional count on each result, printed next to the time, because several of the cases above are about counts that a time alone does not show.
- H3: every framework runs in one process, and every case is warmed up under every framework before any is timed, so a framework's timings depend on which frameworks ran before it. We propose an option that runs each framework, or each framework and family, in its own process, and an option to repeat the process and report the median process.
- H4: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- H5: the Node runner does not pass `--expose-gc`, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- H6: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.
- H7: computations are cheap one-line sums and few distinct function literals are hot, so call-site feedback is unlike an application with hundreds of distinct computations. We propose an option that generates many distinct computation functions and a per-computation cost parameter, applied to the dynamic family.

Cases 6 and 7 need `effect` to return its disposer, case 2 needs an untracked read, and case 17 needs a heap reading. We can open one PR with the cases that need nothing, one PR per interface addition with its cases and adapter changes, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite.
