Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the ten graph operations below, and each of them occurs in large apps. Would you accept PRs that add them?

## Terms

- A signal is a value that the benchmark writes.
- A computed is a value derived from signals and other computeds. It runs a function to get its value, and a run is one call of that function.
- An effect is a function that reads signals and computeds and reruns when something it read has changed.
- A batch is a group of writes after which effects rerun once.
- A disposer is a function that stops one effect, so that it never reruns and no longer reads anything. Disposing an effect means calling its disposer.
- A reader of a node is a computed or effect that reads it.

| Case | Operation | What it measures | Needs |
| --- | --- | --- | --- |
| 1 | Two effects swap which of two computeds they read, in one batch | Recomputations of the two computeds when only the readers change | - |
| 2 | Dispose every effect that reads a computed, then create new effects that read it, with no write in between, and the opposite order | Recomputations of the computeds, and time | effect returns a disposer |
| 3 | Create and dispose computeds and effects over a shared computed while upstream writes happen between | Time, recomputations of the shared computed, and retained memory | effect returns a disposer |
| 4 | One batch writes N sources | Time and visits that mark nodes out of date against writes per batch and observer count | - |
| 5 | Many computeds read one source, only a fraction have effects, the rest are read once and never again, and a fraction change value per write | Computation counts and time against the observed and changed fractions | - |
| 6 | Many writes with no effect, then one read of a computed over many layers | Time and visits against writes before the read | - |
| 7 | One computed reads the same computeds in a different order, with new ones in front, or one computed moves from one reading computed to another | Recomputations of the moved computeds, and time | - |
| 8 | One effect switches among T computed chains whose inputs never change | Recomputations of the chains on return to a branch, and time | - |
| 9 | A computed returns its previous object so its readers cut off, at varying chain position | Recomputations below the cutoff, and time | - |
| 10 | One batch writes the same source N times | Time and visits that mark nodes out of date against N | - |

## Interface

The only extension is that `effect` returns its disposer, which cases 2 and 3 need. `cleanup()` ends every effect of a run at once, so it cannot express disposing some effects while other effects that read the same computeds stay alive. An adapter whose library cannot dispose a single effect skips these two cases.

## Cases

### 1. Two effects swap which of two computeds they read

- The benchmark has one signal `s`, two computeds `a` and `b` that both read `s`, a signal `x` that is true or false, and two effects. The first effect shows `a` when `x` is true and `b` otherwise. The second effect does the opposite.
- Each iteration flips `x` in one batch. Both effects rerun, and each now reads the computed that the other one read before. A variant makes `a` and `b` the tops of chains of `d` computeds, for a few values of `d`.
- It measures time per flip and the number of runs of `a` and `b` per flip. Since `s` never changes, `a` and `b` should never rerun.
- The nearest existing case is `unstable`, which switches one reader between two computeds, but there one computed drives the switch and feeds both branches, so every switch needs a rerun. Here nothing the computeds read changes, and two readers exchange computeds.

### 2. Dispose the effects that read a computed, then create new effects that read it

- The benchmark has one signal `s` and `S` computeds that each read `s`. It also has two sets of `R` effects, set A and set B, and effect number `j` of each set reads computed number `j` modulo `S`. Disposing an effect stops it and removes it as a reader.
- Each iteration disposes every effect of one set and creates every effect of the other set, with no write to `s`. In one order it disposes first and creates afterwards, so for a moment the computeds have no readers. In the other order it creates first and disposes afterwards, so each computed always has a reader.
- It measures time per iteration and the number of runs of the computeds per iteration. Since `s` never changes, the computeds should never rerun, even after they lost their last reader. We sweep `S`, `R`, and the cost of each computed.
- The nearest existing cases are `createComputations` and the build-and-dispose cases, which end every effect at once with `cleanup`. None disposes some readers of a computed while others are created.

### 3. Create and dispose computeds and effects over a shared computed while writes happen

- The benchmark has one signal `s` and one computed `shared` that reads `s`. Each cycle creates `R` computeds that each read `shared`, and one effect per computed that reads it.
- In each cycle the effects run once at creation. Then `s` is written `W` times, each write in its own batch, so `shared`, every row computed, and every effect rerun after each write. Then the effects are disposed, which leaves `shared` with no readers again, and the row computeds are dropped.
- It measures time per cycle, the number of runs of `shared` per cycle, and the memory still held after many cycles. Since `s` does not change between cycles, `shared` should run exactly `W` times per cycle, and no more when the next cycle creates new readers. We sweep `R` from 10 to 1,000 and `W` from 1 to 10.
- The nearest existing case is `createComputations`, which also creates effects, but over new signals, and ends them all at once with no write between creation and disposal. Here `shared` gains and loses readers many times while writes happen between.

### 4. One batch writes N sources

- The benchmark has `S` signals, `M` computeds that each read two signals, and `E` effects that each read three computeds.
- Each iteration writes `W` different signals in one batch. Every computed that reads a written signal, and every effect that reads such a computed, is out of date.
- It measures time per batch and, where the harness can count them, the number of visits that mark nodes out of date. Each out-of-date computed and effect should run at most once per batch, however many of its signals were written. We sweep `W` from 1 to `S` and `E` from 1 to 1,000.
- The largest batch in the suite today writes 4 signals, and every other batch wraps a single write, so no case shows how the cost of a batch grows with the number of writes or the number of effects.

### 5. Many computeds over one signal, only some with effects

- The benchmark has one signal `s` and `K` computeds that each read `s`. A fraction `o` of them have an effect. All of them are read once after setup, and the ones without an effect are never read again. A fraction `c` of them return a new value after each write, and the rest always return the same value.
- Each iteration writes `s` once in one batch.
- It measures time per write, computed runs per write, and effect runs per write. A computed with an effect must rerun to find out whether its value changed, and its effect reruns only if it did. A computed with no effect is not read again, so it need not rerun. The ideal is `o * K` computed runs and `o * c * K` effect runs per write.
- We sweep `K` from 1 to 10,000, `o` over 0, 0.1, and 1, and `c` over 0, 0.1, 0.5, and 1. The nearest existing cases are `update1to1000`, `update1to2`, `update1to4`, `broadPropagation`, and `mux`, which all have an effect on every computed, and all but `mux` change every computed on each write. None varies either fraction, and none has a computed that was read once and then left alone while its signal keeps changing.

### 6. Many writes with no effect, then one read

- The benchmark has 100 signals and 1,000 computeds that each read three signals. It has 100 more computeds that each read ten of those, and one top computed `root` that reads all 100 of them. The benchmark reads `root` once after setup and has no effects, so a write has nobody to notify.
- Each iteration makes `W` writes, each in its own batch, and then reads `root` once.
- It measures time per iteration and, where the harness can count them, visits that mark nodes out of date and visits that check them. The read must find which of the 1,101 computeds are out of date and run each at most once, so the number of runs cannot exceed 1,101, however large `W` is. We sweep `W` from 1 to 1,000.
- In every existing case, each write is followed by an effect rerun or a read, so no case shows how the cost of a read grows with the number of writes since the last read.

### 7a. A computed reads the same computeds in a different order

- The benchmark has `N` item computeds, each reading its own signal, a signal `order` that holds a list of item numbers, one computed `list` that reads `order` and then the items in that order, and one effect that reads `list`. The prepend variant draws new item numbers from a pool of further items, so the window of `N` items moves along the pool.
- Each iteration writes a new `order` in one batch. In the reorder variant the list is rotated by some amount or reversed, so `list` reads the same items in a new order. In the prepend variant `k` new items are added at the front and `k` are removed from the end. In both, `list` and the effect rerun.
- It measures time per iteration and the number of runs of the item computeds. No item signal changes, so the items should never rerun. We sweep `N` from 10 to 10,000 and `k` from 1 to `N`.
- The nearest existing cases are the dynamic family, where a computed skips one signal and reads the rest in the same order, and `unstable`. None changes the order of reads, so none charges for tracking a reordered read.

### 7b. One computed moves from one reading computed to another

- The benchmark has `G` signals, where signal `k` holds the list of item numbers in group `k`. Group computed `k` reads signal `k` and then every item computed in that list, and one effect per group reads that group computed. Each item computed reads its own signal.
- Each iteration takes the first item out of group `k` and puts it at the front of group `k+1`, with both writes in one batch, and `k` moves up by one each iteration. Both group computeds and both of their effects rerun. The moved item stops being read by group `k` and starts being read by group `k+1`.
- The benchmark runs this twice, once writing group `k` first and once writing group `k+1` first, because the order decides whether the moved item ever has no reader. It measures time per iteration and the number of runs of the item computeds. No item signal changes, so the items should never rerun. We sweep `N` from 10 to 10,000.
- No existing case moves a computed from one reader to another, so none charges for tracking a moved read.

### 8. One effect switches among T computed chains whose inputs never change

- The benchmark has `T` chains of `d` computeds. Each chain reads its own signal, and each computed in it reads the one before. One signal `tab` says which chain is selected, and one effect reads `tab` and then the last computed of the selected chain. The other chains keep their values and are only not read by the effect.
- Each iteration moves `tab` to the next chain, wrapping around, in one batch. The effect reruns, stops reading the old chain, and starts reading the new one. A variant adds one effect per chain, so every chain stays in use, and measures what keeping the unselected chains alive costs.
- It measures time per switch and the number of chain runs per switch after each chain has been visited once. Nothing a chain reads has changed, so no chain should rerun. We sweep `T` and `d`.
- The nearest existing case is `unstable`, which also flips between two branches on every write, but the flip and the branch values come from the same computed, so it never returns to a branch whose inputs did not change.

### 9. A computed returns its previous object so its readers skip rerunning

- In the fan-out version there are two signals, `title` and `updatedAt`. One computed `view` reads both and returns its previous object when the title and the day of `updatedAt` are equal. `R` computeds read `view`, and each has one effect.
- Each iteration moves `updatedAt` forward by an amount that stays within one day, in one batch. `view` reruns, finds that nothing changed, and returns the same object as before. The `R` computeds and their effects should then not run.
- In the chain version a signal feeds a chain of `D` computeds, and one computed at position `p` returns its previous object. The computeds above it and the one at `p` must run to find that nothing changed, and the ones below should not run. We sweep `R`, `D`, and `p` at the first, middle, and last position, and measure time per write and the number of runs below `view` or below position `p`. The ideal is 0.
- The nearest existing cases are `avoidablePropagation`, which stops propagation by returning a constant, and `mux`, which builds a new object on every run so nothing below it can stop. No case returns the previous object, and none varies how much graph lies below the stop.

### 10. One batch writes the same signal N times

- The benchmark has one signal `s`, one computed `c` that reads `s`, and one effect that reads `c`.
- Each iteration writes `s` `W` times with different values inside one batch. After the batch, `c` and the effect should each run once, on the last value.
- It measures time per batch and, where the harness can count them, visits that mark nodes out of date. Only the last value can be seen, so the cost should grow with `W` only through the writes themselves. We sweep `W` from 1 to 1,000.
- No existing batch writes the same signal twice. The batches in the suite that hold several writes write different signals, and every other batch wraps a single write.

## Harness

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC at each stage of a build of `N` sources, then `N` computeds each read once, then `N` effects, then after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds. Case 3 would also report heap retained after many cycles.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process, and an option that repeats the processes and reports the median process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- Counts: computation counts are asserted in some cases but never reported. We propose an optional count on each result, printed next to the time, because several cases above are about counts that a time alone does not show.
- First run: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.

We can open one PR with the cases that need no interface change (1 and 4 to 10), one PR with the disposer extension and cases 2 and 3, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite. Which of these would you accept?
