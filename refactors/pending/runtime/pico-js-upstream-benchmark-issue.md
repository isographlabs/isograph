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

Harness gaps that affect these and the existing cases:

- H1: no retained-memory measurement (needed by case 17).
- H2: computation counts are asserted in some cases but never reported.
- H3: every framework runs in one process, so timings depend on which frameworks ran before.
- H4: only the fastest run is reported, which leaves out GC pauses and tails.
- H5: the Node runner does not pass `--expose-gc`, so the `gc()` calls do nothing.
- H6: every case is warmed up, so no result shows a first run.
- H7: computations are cheap one-line sums with few distinct function literals.

Cases 6 and 7 need `effect` to return its disposer, case 2 needs an untracked read, and case 17 needs a heap reading. We can open one PR with the cases that need nothing, one PR per interface addition with its cases and adapter changes, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite.
