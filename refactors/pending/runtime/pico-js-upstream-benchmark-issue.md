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

### 1. Split view swaps its panels

A split or compare view shows two derived values, one per panel, and a toggle makes each panel show the value the other panel showed. The graph has one source `s`, a second source `x` for the toggle, two computeds `a` and `b` that each read `s` and cost a tunable amount, and two effects that read `x` and then one of `a` or `b`, in opposite order. Each iteration writes `x` once in a batch. A variant makes `a` and `b` the heads of chains of depth `d`, swept over a few values, so the cost of a dropped value grows with `d`. The case reports time per toggle and runs of `a` and `b` per toggle. No source of `a` or `b` changed, so the minimum is 0 runs. The closest existing case is kairo `unstable`, which switches one reader between two computeds, but a single `head` both drives the switch and feeds both branches, so every switch needs a recompute and a value dropped at the switch costs nothing extra. The dynamic family also never has two readers exchange a computed.

### 2. Peek at the current selection

An effect reads the current selection through an untracked read, for logging or an imperative call, while a second effect subscribes to the computed that the selection drops when it changes. The graph is the one in case 1 with an extra computed `r` that reads `x` and then `a` or `b`. The first effect reads `x` and then reads `r` untracked, and the second effect reads `x` and then `b` or `a`. Each iteration writes `x` once in a batch. The case reports time per toggle and runs of `a` and `b` per toggle, and the minimum is 0 runs because no source of `a` or `b` changed. No existing case uses an untracked read, because the suite interface has none, so the closest case is again `unstable`, which has a tracked read only. This case differs from case 1 in that a reader that is not subscribed still reads `r`, which affects whether `a` keeps its value when its last subscriber leaves.

### 3. Kanban card moves between columns

A card moves from one column to the next, or a re-sort moves a row into another section, and each column or section is a computed over its members' row computeds. The graph has `N` row computeds over `N` sources, `G` signals that each hold the array of row ids in one group, `G` group computeds that read their members' rows, and one effect per group. Each iteration removes the first id from group `k` and prepends it to group `k + 1`, with both writes in one batch, and `k` advances each iteration. The case reports row runs per move and time per move. The minimum is 0 row runs, because no row's own inputs changed. It runs twice, once writing the losing group first and once the gaining group first, because the order decides whether the row ever has zero readers. The closest existing cases are the dynamic family and `unstable`, but neither moves a computed from one live reader to another.

### 4. List re-sort, page prepended to a feed

A list is re-sorted or a page of items is prepended, so one computed reads the same items in a different order, or the same items plus new ones in front. The graph has `N` item computeds over `N` sources, an `order` signal holding item ids, one `list` computed that reads the items in the order of `order`, and one effect. Each iteration writes `order` rotated by a shift, in one batch. Variants are a full reversal and a prepend of `k` new items, and `N` is swept from 10 to 10,000. The case reports time per reorder and, where the harness can count them, dependency edges allocated per reorder. Item runs have a minimum of 0 for the rotation and reversal. A node in the dynamic family skips one source and reads the rest in the same order, and no existing case changes the order of reads, so none charges for tracking a reordered dependency list.

### 5. Tab switch

A tab bar shows one panel at a time, and each panel is a chain of computeds over sources that do not change while the tab is hidden. The graph has `T` panels, each a chain of depth `d` over its own source, a `tab` signal, and one effect that reads `tab` and then the head of the selected panel. Each iteration writes `tab` to the next tab, cyclically, in one batch. A variant adds one effect per panel, so every panel stays observed, and measures what keeping hidden panels alive costs. The case reports runs per switch after the first visit to each tab and time per switch. The minimum is 0 runs, because nothing a hidden panel reads changed. Kairo `unstable` flips on every write, but the flip and the branch values come from the same `head`, so it never returns to a branch whose inputs did not change.

### 6. Route change

A route change unmounts the old screen's effects and mounts the new screen's effects, and both screens read computeds that are shared, such as the viewer, a workspace, or users that many rows display. The graph has one source, `S` shared computeds that each cost a tunable amount, and `R` effects per screen that each read one shared computed. Each iteration disposes the old screen and mounts the new one, in one of two orders, unmount first and mount first. The case reports runs of the shared computeds per navigation, with a minimum of 0 because no source changed, and time per navigation. The closest existing cases are sbench `createComputations` and the build-and-dispose families, which end every lifetime at once through `cleanup`. None disposes some observers while others reading the same computeds are created. The two orders matter because only unmount first leaves a shared computed with zero observers for a moment.

### 7. Virtualized list scroll

Rows enter and leave a virtualized list while data keeps arriving, so effects are created and disposed on a graph that stays live. The graph has one source, one shared computed over it, and per cycle `R` row computeds and `R` row effects created over the shared computed. Each iteration creates the rows, writes the source once in a batch so the new effects run, and disposes the row effects. The case reports time per cycle and, with the retained-memory measurement from H1, heap retained after many cycles. Sbench `createComputations` also creates effects, but over fresh signals, and disposes the whole build with no write between creation and disposal. This case needs the shared computed to be subscribed and unsubscribed repeatedly while an upstream write happens in between.

### 8. Normalized response, bulk edit

A response is normalized into many records and written in one batch, or a form reset or bulk edit writes many fields at once. The graph has `S` sources, `M` computeds that each read two sources, and `E` effects that each read three computeds. Each iteration writes `W` distinct sources in one batch. We sweep `W` from 1 to `S` and `E` from 1 to 1,000. The case reports time per batch and, where the harness can count them, marking visits per batch. The most writes in one batch in the suite today are the 4 in cellx and the 2 in mol, and every other batch wraps a single write, so no case shows how the cost of a batch grows with the number of writes.

### 9. Drag or typing events in one frame

Several input events are coalesced into one frame, and each event writes the same source with a new value before the frame is processed. The graph is one source, one computed over it, and one effect. Each iteration writes the source `W` times with distinct values inside one batch, with `W` swept from 1 to 1,000. The case reports time per batch and, where it can count them, marking visits per batch. Only the last value is observable, so the ideal cost grows with `W` only through the writes themselves. No existing batch writes a source twice, since mol writes each of two sources once and cellx writes four different sources.

### 10. Refetch with equal values, optimistic write rolled back

A refetch returns values equal to what is stored, and an optimistic write is undone by the same batch, such as a slider dragged back to its origin before the frame ends. There are two shapes. In the revert shape one source feeds a computed `c` of tunable cost, a computed `d` over `c`, and one effect, and each iteration writes a new value and then the old value inside one batch. In the equal-writes shape `n` sources feed one summing computed with one effect, and each iteration writes every source with its current value. The case reports runs of `c`, `d`, and the effect per batch, with a minimum of 0, and time per batch. Almost every timed write in kairo, sbench, and the dynamic family differs from the source's last value, and none writes a value back within a batch, so none measures a write that changes nothing.

### 11. Selector that returns the previous object, day-formatted timestamp

A selector builds an object from a title and a timestamp formatted to a day, and returns the previous object when the fields are equal, so readers below it should not run. The graph has two sources, one `view` computed that returns its previous value when both fields are unchanged, and `R` reader computeds each with an effect. Each iteration advances the timestamp by an amount that stays within one day. The second shape is a chain of depth `D` where one node returns an unchanged value, placed at the first, middle, or last position. The case reports time per write and runs below `view`, with a minimum of 0. Kairo `avoidablePropagation` cuts off with a constant, and `mux` builds a new object on every run so its readers never cut off. No case returns the previous object, and none varies the cutoff position, so none shows how the cost of a cutoff depends on how much graph lies below it.

### 12. Retained screens

A user leaves screens in memory, and the computeds of those screens were read once while visible and still depend on sources that later writes change. The graph has one source and `K` computeds that read it, each read once after the build. A fraction `f` of the computeds also has an effect, and we run `f` at 0, 0.1, and 1. Each iteration writes the source once in a batch. The case reports time per write and, where it can count them, marking visits per write. In the dynamic family the unread leaves are chosen before the first read and are never read, so no node is both read once and then abandoned. This case covers a node that was read once, has no observer, and still depends on a changing source.

### 13. Data arrives while a view is hidden

Data arrives in many separate writes while a view is hidden, and the view is read once when it returns, with no effect observing it. The graph has 100 sources, 1,000 computeds that each read three sources, 100 computeds that each read ten of those, and one root computed that reads all hundred. The root is read once after the build, and no effect exists. Each iteration makes `W` writes, each in its own batch, and then one read of the root, with `W` swept from 1 to 1,000. The case reports time per iteration and, where it can count them, marking and verification visits. Every iteration in the suite makes one write before the next read or effect flush, so no case shows how the cost of a read grows with the number of writes that preceded it.

### 14. Selectors on one source, some on screen, some changing

A source is read by `K` selectors, of which a fraction is on screen and a fraction returns a different value after a write. The graph has one source and `K` computeds over it, where a fraction `c` returns a value that changes on each write and the rest return a constant. A fraction `o` of the computeds has an effect, and all are read once after the build. Each iteration writes the source once in a batch. We sweep `K` from 1 to 10,000, `o` over 0, 0.1, and 1, and `c` over 0, 0.1, 0.5, and 1. The case reports time per write. Sbench `update1to1000`, `update1to2`, and `update1to4`, kairo `broadPropagation`, and `mux` all observe every reader, and all but `mux` change every reader, so none varies either fraction.

### 15. Items that share a record

List items share an author record, or a selector reads the same collection several times, so one computed reads the same sources more than once and not on adjacent reads. The graph has `F` sources, one computed that reads all `F` in sequence `P` times over, and one effect. Each iteration writes the first source once in a batch. We sweep `F` from 1 to 100 and `P` from 1 to 30. The case reports time per write and, where it can count them, the dependency edges held by the computed. Kairo `repeatedObservers` reads `head` 30 times in a row, which a check against the immediately preceding read already deduplicates, so it never has a read of one dependency separated from the previous read of it.

### 16. Fallback rows until a record arrives

Rows render a fallback while a record is absent, and read its fields and their formatting computeds once a response delivers it. The graph has a `present` signal, `F` field sources, one formatting computed per field, and `R` row effects that read `present` and, when it is true, every formatting computed. Each iteration toggles `present` in one batch, so the dependency list of each row grows from 1 to `F + 1` and then shrinks back. We sweep `F` from 1 to 1,000 and `R` from 1 to 1,000. The case reports time per arrival and per removal, and runs of the formatting computeds, which have a minimum of 0 on removal. A node in the dynamic family changes one source per run, so no existing dependency list grows or shrinks by a large factor.

### 17. Memory per node

A long session retains many nodes, and the heap each node costs and what stays after the program drops its references matter. The case builds `N` sources, `N` computeds that are each read once, and `N` effects, and calls `cleanup`, drops every reference, and forces GC. It reads the heap after forced GC at each stage, with `N` swept from 1,000 to 100,000. The case reports bytes per source, per computed read once, per effect, and bytes still retained after the drop, where the ideal after the drop is 0. The suite calls `gc()` between runs and reads no heap figure, and one fork's own README lists memory among what it does not measure, so no case reports retained bytes per node.

## Harness gaps

- H1: no retained-memory measurement, which case 17 needs. The suite calls `gc()` between runs but never reads the heap. We propose reading the heap after a forced GC at each stage of case 17, reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds.
- H2: computation counts are checked in some cases but never reported. The kairo cases assert on a counter, and the dynamic family checks one total count in some configurations. We propose an optional count on each result, printed next to the time, because several of the cases above are about counts that a time alone does not show.
- H3: every framework runs in one process, and every case is warmed up under every framework before any is timed, so a framework's timings depend on which frameworks ran before it. We propose an option that runs each framework, or each framework and family, in its own process, and an option to repeat the process and report the median process.
- H4: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- H5: the Node runner does not pass `--expose-gc`, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- H6: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.
- H7: computations are cheap one-line sums and few distinct function literals are hot, so call-site feedback is unlike an application with hundreds of distinct computations. We propose an option that generates many distinct computation functions and a per-computation cost parameter, applied to the dynamic family.

Cases 6 and 7 need `effect` to return its disposer, case 2 needs an untracked read, and case 17 needs a heap reading. We can open one PR with the cases that need nothing, one PR per interface addition with its cases and adapter changes, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite.
