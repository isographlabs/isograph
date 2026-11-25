Proposal: benchmark cases for observer lifetimes, multi-write batches, and cutoffs

We are developing a push-pull reactive library. While comparing it against alien-signals and Preact signals, we found operation sequences that applications perform and that no case in this suite runs. In several of them, libraries differ in mechanism: one library recomputes a computed where another keeps its value, or one library visits a node on every write where another never links it. The current cases do not show those differences.

Would you accept PRs that add these cases? Below are 17 cases, grouped by what they need from the suite, followed by a separate list of harness changes. The list includes every case we found, including the cases where our own library does worse than the others.

## Conventions

- Every snippet uses the existing `ReactiveFramework` in `packages/core/src/util/reactiveFramework.ts`. Each case builds its graph inside `withBuild` and returns an iteration function, the same shape the kairo cases use.
- `work(seed, n)` stands for a computation whose cost the case controls, like `busy()` in `kairo/util`:

  ```ts
  function work(seed: number, n: number): number {
    let hash = seed;
    for (let i = 0; i < n; i++) hash = (hash * 31 + i) | 0;
    return hash;
  }
  ```

- Each case counts computation runs with a `Counter` inside the computed, the way `kairo/broad.ts` counts effect runs. For many of these cases the libraries legitimately differ in count, so the case asserts values and reports counts (harness proposal H2) instead of asserting one count.

## Group 1: cases that fit the current interface

These use only `signal`, `computed`, `effect`, `withBatch`, `withBuild`, and `cleanup`.

### 1. Reader swap within one flush

Pattern: two panels exchange the derived values they show on one toggle, such as a split view whose sides swap.

```ts
const s = framework.signal(1);
const x = framework.signal(true);
const a = framework.computed(() => work(s.read(), cost));
const b = framework.computed(() => work(s.read() + 1, cost));
framework.effect(() => void (x.read() ? a.read() : b.read()));
framework.effect(() => void (x.read() ? b.read() : a.read()));
let flag = true;
return () => framework.withBatch(() => x.write((flag = !flag)));
```

A variant makes `a` and `b` the heads of chains of depth `d`.

- Measures: time per toggle, and runs of `a` and `b` per toggle. No input of `a` or `b` changes, so the minimum is 0.
- Why it differs: the first effect drops `a` before the second effect picks it up in the same flush. Some libraries discard the value of a computed that loses its last subscriber and recompute it on the next read; others keep it and revalidate.
- Nearest existing case: kairo `unstable` switches one reader between two computeds, but `head` both drives the switch and feeds both branches, so every switch must recompute anyway.

### 3. A row moves between group nodes

Pattern: a kanban card moves between columns, or a re-sort moves a row into another section.

- Shape: `N` row computeds over `N` sources, `G` member signals holding arrays of row ids, `G` group computeds that read their members' rows, and one effect per group.
- Iteration: one batch removes the first id from group `k`'s members and prepends it to group `k + 1`'s members.
- Measures: row runs per move (minimum 0), and time.
- Runs in both write orders (losing group first, gaining group first). In some libraries the result depends on which group's effect runs first: the row is recomputed only when the losing group drops it before the gaining group reads it.

### 4. Reorder a dependency list

Pattern: a list re-sort that keeps its members, or a page prepended to a feed.

- Shape: `N` item computeds, an `order` signal holding item ids, a `list` computed that reads the items in `order`, and one effect.
- Iteration: one batch writes `order` rotated by `k`. Variants: a full reversal, and a prepend of `k` new items.
- Measures: time per reorder. Libraries differ in whether a computation that reads the same dependencies in a new order reuses its existing edges or allocates new ones.
- Nearest existing case: a dynamic node in `reactively/dependencyGraph.ts` skips one source and reads the rest in the same order. No case changes the order of reads.

### 5. Branch flip across batches

Pattern: a tab switch, where each tab's panel is a chain of computeds over sources that do not change while the tab is hidden.

```ts
const panels = Array.from({ length: tabCount }, (_, t) => {
  let head: Computed<number> = framework.signal(t);
  for (let level = 0; level < depth; level++) {
    const below = head;
    head = framework.computed(() => below.read() + 1);
  }
  return head;
});
const tab = framework.signal(0);
framework.effect(() => void panels[tab.read()].read());
let current = 0;
return () =>
  framework.withBatch(() => tab.write((current = (current + 1) % tabCount)));
```

A variant adds one effect per panel, so every panel stays observed, to measure what keeping them alive costs.

- Measures: runs per switch after each tab's first visit (minimum 0), and time.
- Why it differs: some libraries keep the values of the abandoned chain and revalidate it on return; others discard them and recompute the chain.
- Nearest existing case: `unstable` flips on every write, but its branch inputs change on the same write. No case returns to a branch whose inputs did not change.

### 8. One batch writes many sources

Pattern: a response normalized into many records, a form reset, a bulk edit.

- Shape: `S` sources, `M` computeds that each read two sources, and `E` effects that each read three computeds.
- Iteration: one `withBatch` writes `W` sources. Sweep `W` from 1 to `S` and `E` from 1 to 1,000.
- Measures: time per batch.
- Nearest existing case: the most writes in one batch today are cellx's 4 and molBench's 2. Every other batch wraps one write.

### 9. Repeated writes to one source in a batch

Pattern: drag or typing events coalesced into one frame.

- Shape: `s`, a computed over `s`, one effect.
- Iteration: one batch writes `s` `W` times with distinct values.
- Measures: time per batch. Libraries differ in whether each write in the batch walks the subscribers again.
- Nearest existing case: no batch writes the same source twice.

### 10. Equal-value writes, and a write reverted in the same batch

Pattern: a refetch whose values equal what is stored, and an optimistic write that the same batch rolls back.

```ts
// revert
const s = framework.signal(1);
const c = framework.computed(() => work(s.read(), cost));
const d = framework.computed(() => c.read() + 1);
framework.effect(() => void d.read());
return () =>
  framework.withBatch(() => {
    s.write(2);
    s.write(1);
  });

// equal writes
const sources = Array.from({ length: n }, (_, i) => framework.signal(i));
const sum = framework.computed(() =>
  sources.reduce((t, s) => t + s.read(), 0),
);
framework.effect(() => void sum.read());
return () => framework.withBatch(() => sources.forEach((s, i) => s.write(i)));
```

- Measures: runs of `c`, `d`, and the effect per batch (minimum 0), and time.
- Why it differs: some libraries treat a source restored within the batch as unchanged; others recompute `c`, which returns its old value and stops there. The equal-writes variant measures what a write that changes nothing costs.
- Nearest existing case: almost every timed write differs from the source's current value, and no batch writes a value back.

### 11. Cutoff by returning the previous value, and cutoff position

Pattern: a selector that builds an object and returns the previous object when the fields are equal, and a formatter whose output rarely changes (a timestamp formatted to a day).

```ts
const title = framework.signal('a');
const updatedAt = framework.signal(0);
let previous: { title: string; day: number } | undefined;
const view = framework.computed(() => {
  const next = {
    title: title.read(),
    day: Math.floor(updatedAt.read() / 86_400_000),
  };
  if (
    previous !== undefined &&
    previous.title === next.title &&
    previous.day === next.day
  ) {
    return previous;
  }
  return (previous = next);
});
for (let r = 0; r < readerCount; r++) {
  const reader = framework.computed(() => view.read().title.length + r);
  framework.effect(() => void reader.read());
}
let time = 0;
return () => framework.withBatch(() => updatedAt.write((time += 1_000)));
```

A second shape is a chain of depth `D` with the node that returns an unchanged value at the first, middle, or last position.

- Measures: time per write, and runs below `view` (minimum 0).
- The case keeps the previous value in a closure, so `computed` needs no new parameter.
- Nearest existing case: `avoidablePropagation` cuts off with a constant, and `mux` returns a new object on every run, so its readers never cut off. No case returns a previous object, and no case varies the cutoff position.

### 12. Stale readers that are never read again

Pattern: retained screens whose computeds were read once while visible and still depend on sources that later writes change.

- Shape: `K` computeds over one source, each read once after the build. A fraction `f` of them also has an effect.
- Iteration: one batch writes the source. Run `f` at 0, 0.1, and 1.
- Measures: time per write. Libraries differ in whether a computed read outside any effect stays linked to its sources, and so in whether each later write visits it.
- Nearest existing case: in the dynamic family, the unread leaves are chosen before the first read and are never read, so no library ever links them.

### 13. Many writes, then one read, with no effect

Pattern: data that arrives while a view is hidden and is read once when the view returns.

- Shape: 100 sources, 1,000 computeds that each read three sources, 100 computeds that each read ten of those, and one root computed. The root is read once after the build. No effect.
- Iteration: `W` writes, each in its own `withBatch`, then one `root.read()`. Sweep `W` from 1 to 1,000.
- Measures: time per iteration.
- Nearest existing case: every iteration in the suite makes one write before the next read or effect flush.

### 14. Fan-out sweep over the observed and changed fractions

Pattern: one source read by `K` selectors, of which a fraction is on screen and a fraction changes value on a write.

- Shape: `K` computeds over one source. A fraction `c` returns a value that changes on each write, and the rest return a constant. A fraction `o` has an effect. All are read once after the build.
- Iteration: one batch writes the source. Sweep `K` from 1 to 10,000, `o` over 0, 0.1, and 1, and `c` over 0, 0.1, 0.5, and 1.
- Measures: time per write.
- Nearest existing case: `update1to1000`, `update1to2`, `update1to4`, `broadPropagation`, and `mux`. All observe every reader, and all but `mux` change every reader.

### 15. Repeated reads of one dependency that are not adjacent

Pattern: list items that share an author record, and a selector that reads the same collection several times.

- Shape: one computed that reads `F` sources in sequence, `P` times over (`a, b, c, a, b, c, ...`), and one effect.
- Iteration: one batch writes the first source.
- Measures: time per write. Libraries differ in whether a repeated read that is not adjacent to the previous read of the same dependency adds another edge.
- Nearest existing case: `repeatedObservers` reads `head` 30 times in a row, which a check against the immediately preceding read already deduplicates.

### 16. Missing data arrives

Pattern: rows that render a fallback while a record is absent, then read its fields when a response delivers it.

```ts
const present = framework.signal(false);
const fields = Array.from({ length: fieldCount }, (_, i) =>
  framework.signal(i),
);
const formatted = fields.map((f) => framework.computed(() => `${f.read()}`));
for (let r = 0; r < rowCount; r++) {
  framework.effect(() => {
    if (present.read()) formatted.forEach((c) => c.read());
  });
}
return () => framework.withBatch(() => present.write(!present.read()));
```

- Measures: time per arrival and per removal, and runs of the formatted computeds.
- Nearest existing case: a dynamic node changes one source per run, so no dependency list grows or shrinks by a large factor.

## Group 2: cases that need an interface extension

### Extension A: `effect` returns its disposer

```ts
effect(fn: () => void): () => void;
```

`cleanup()` disposes everything a build created, and the adapters hold one build at a time (the alien-signals adapter keeps one `scope`; the Solid, Pota, and x-reactivity adapters overwrite `cleanup` on each `withBuild`). A case therefore cannot dispose some effects while other effects that read the same computeds stay alive, and cannot have two sets of effects alive at once.

#### 6. Navigation: unmount a screen, mount the next

Pattern: a route change. The old screen's components unsubscribe and the new screen's subscribe, and both read shared computeds (the viewer, a workspace, users shared by many rows).

```ts
const s = framework.signal(1);
const shared = Array.from({ length: sharedCount }, (_, i) =>
  framework.computed(() => work(s.read() + i, 100)),
);
const mount = () =>
  Array.from({ length: readersPerScreen }, (_, i) =>
    framework.effect(() => void shared[i % sharedCount].read()),
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
```

- Measures: runs of shared computeds per navigation (minimum 0), and time.
- Why it differs: with unmount first, some libraries discard a shared computed's value when its last subscriber is disposed and recompute it for the next screen; others keep it. With mount first, no library we checked recomputes.
- Nearest existing case: lifetimes in the suite end all at once through `cleanup()`. `sBench` disposes the whole graph inside its timed region.

#### 7. Observer churn on a live graph

Pattern: rows scroll into and out of a virtualized list while data keeps arriving.

```ts
const s = framework.signal(0);
const shared = framework.computed(() => s.read() * 2);
return () => {
  const disposers = Array.from({ length: rowCount }, (_, i) => {
    const row = framework.computed(() => shared.read() + i);
    return framework.effect(() => void row.read());
  });
  framework.withBatch(() => s.write(s.read() + 1));
  disposers.forEach((dispose) => dispose());
};
```

- Measures: time per cycle, and, with H1, retained heap after many cycles.
- Nearest existing case: `createComputations` creates effects over fresh signals and disposes the whole build, with no write in between.

#### Adapters that change

- alien-signals: the adapter already returns `effect(fn)`, which is the disposer. Only the type changes.
- Preact, tansu, MobX, compostate, uSignal, TC39 polyfill: the adapter already pushes a disposer onto `toCleanup` and would also return it.
- Angular: `effect` returns an `EffectRef`, and the adapter would return `() => ref.destroy()`.
- Vue: `effect` returns a runner, and the adapter would return `() => stop(runner)`.
- $mol_wire: the adapter would return `() => atom.destructor()`.
- Solid, x-reactivity, Pota, s-js: the effect function returns nothing, so the adapter would create each effect in its own root (`createRoot`, `root`, `S.root`) and return that root's dispose.
- Svelte: the adapter would create each effect inside `effect_root` and return its result.
- Reactively: the adapter notes that it does not support cleaning up effects, and `@reactively/core` 0.0.8 exports no disposal API. Reactively would skip these cases.

Each adapter must also keep `cleanup()` correct after some of its effects were disposed individually.

### Extension B: an untracked read

```ts
untracked<T>(fn: () => T): T;
```

#### 2. Reader swap through an untracked read

Pattern: an effect reads a derived value without subscribing to it (a peek at the current selection for logging or an imperative call), while another effect subscribes to a node that the peeked value drops.

```ts
const s = framework.signal(1);
const x = framework.signal(true);
const a = framework.computed(() => work(s.read(), cost));
const b = framework.computed(() => work(s.read() + 1, cost));
const r = framework.computed(() => (x.read() ? a.read() : b.read()));
framework.effect(() => {
  x.read();
  framework.untracked(() => r.read());
});
framework.effect(() => void (x.read() ? b.read() : a.read()));
let flag = true;
return () => framework.withBatch(() => x.write((flag = !flag)));
```

- Measures: runs of `a` and `b` per toggle, and time.
- Why it differs: libraries differ in whether a computed that is only ever read untracked links itself to its dependencies, and so in whether its recompute counts as `a` losing a subscriber.

#### Adapters that change

- `untracked`: Preact, Angular (the adapter already imports it), MobX.
- `untrack`: Solid, x-reactivity, Pota, Svelte (`svelte/internal/client`), tansu, compostate.
- s-js: `S.sample`. Vue: `pauseTracking()` and `resetTracking()`. alien-signals 3.x: `setActiveSub(undefined)` around the read, then restore the previous subscriber.
- Reactively: `@reactively/core` 0.0.8 exports no untracked read. Reactively would skip this case.

## Group 3: harness and methodology changes

Each of these is a separate proposal.

### H1. Retained memory

Case 17, retained memory per node and collection of dropped nodes: after a forced GC, read the heap before and after creating `N` sources, `N` computeds read once, and `N` effects, then call `cleanup()`, drop every reference, force GC again, and read the heap once more.

- Reports: bytes per source, per computed read once, per effect, and bytes still retained after the drop.
- Needs: `gc()` and a heap reading (`process.memoryUsage().heapUsed` in Node; the web runner's Chrome script already passes `--expose-gc` and `--enable-precise-memory-info`), a result unit other than milliseconds, and, to keep other frameworks' heaps out of the reading, one process per framework (H3).
- The case uses `cleanup()`, so it needs no interface extension.

### H2. Report computation counts

Counts are checked today, never reported: the kairo cases `console.assert` on their `callCounter`, and the dynamic family checks `count` only when `readFraction` is 1 or `testPullCounts` is set (`util/perfTests.ts`). Proposal: an optional `count` on `PerfResult`, printed next to `time`. Several Group 1 cases differ across libraries mainly in count, and a time alone does not show why.

### H3. One process per framework

`runTests` runs every framework in one process, and `kairoBench` warms every case under every framework before it times any. Harness call sites and shared closures therefore see many libraries' object shapes before a library is timed, and a library's timings depend on which libraries ran before it. In our runs, removing the other frameworks' warmup changed one library's kairo timing noticeably because its call sites stayed monomorphic. Proposal: an option that runs each framework, or each framework and family, in its own process, and an option to repeat that process several times and report the median process, since background load slows whole processes.

### H4. Report more than the fastest run

`fastestTest` returns the fastest run, `sBench` keeps the fastest of 10, and `cellx` sums 10 runs. The minimum discards GC pauses and tails, so a library that allocates more pays for it in GC that the reported number omits. Proposal: report the median and a high percentile (or every sample) next to the minimum, and GC time per test, for example from `PerformanceObserver` `gc` entries in Node. The README lists "Tracks garbage collection overhead per test" as a feature, but no code measures GC time today.

### H5. Expose GC in the Node runner

Every family calls `gc()` only `if (globalThis.gc)`, and the Node package's `run` script is `node dist/index.js`, without `--expose-gc`, so those calls do nothing in that runner unless the person running it adds the flag. Proposal: add `--expose-gc` to that script.

### H6. A first, unwarmed run

Every family warms up before it times. Applications run many computations only once or a few times, and cold costs differ between libraries. Proposal: in a fresh process (H3), record the first iteration of each case before any warmup, as its own result.

### H7. Costlier computations and more distinct function literals

Most computations in the suite are one-line sums, and few distinct function literals are hot, so the inline-cache state is unlike an application with hundreds of distinct computations. Proposal: an option that generates many distinct computation functions (for example with `new Function`) and a per-computation cost parameter, applied to the dynamic family.

## Proposed PRs

We would be glad to open PRs, split as follows:

- One PR with the Group 1 cases (1, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16).
- One PR for Extension A, with cases 6 and 7 and the adapter changes.
- One PR for Extension B, with case 2 and the adapter changes.
- One PR per harness change, H1 (with case 17) through H7.

Which of these would you accept? We are happy to adjust the case names, the parameters, or where the cases live (a new family under `benches/`, or additions to the kairo list) to fit the suite.
