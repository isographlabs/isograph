Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the ten graph operations below, and each of them occurs in large apps. Would you accept PRs that add them?

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

Each case is written against the suite's `ReactiveFramework` (`signal`, `computed`, `effect`, `withBatch`, `withBuild`). `work(n)` stands for a computation of tunable cost. Each function builds the graph and returns one iteration.

### 1. Two effects swap which of two computeds they read

When `x` flips, each effect stops reading one computed and starts reading the other. Nothing `a` or `b` reads has changed, so neither should be recomputed.

```js
function readerSwap(fw) {
  const s = fw.signal(1);
  const x = fw.signal(true);
  const a = fw.computed(() => work(s.read()));
  const b = fw.computed(() => work(s.read() + 1));
  fw.effect(() => (x.read() ? a.read() : b.read()));
  fw.effect(() => (x.read() ? b.read() : a.read()));
  let flag = true;
  return () => fw.withBatch(() => x.write((flag = !flag)));
  // s never changes, so a and b should run 0 times per iteration.
}
```

### 2. Dispose the effects that read a computed, then create new ones that read it

The old effects go away and new ones read the same computeds, with no write in between. Nothing the computeds read has changed, so they should not be recomputed, even in the order where they briefly have no reader.

```js
function replaceReaders(fw, order /* 'disposeFirst' | 'createFirst' */) {
  const s = fw.signal(1);
  const shared = Array.from({ length: S }, (_, i) => fw.computed(() => work(s.read() + i)));
  const mount = () => Array.from({ length: R }, (_, j) => fw.effect(() => shared[j % S].read()));
  let current = mount();
  return () => {
    if (order === 'disposeFirst') {
      current.forEach((dispose) => dispose());
      current = mount();
    } else {
      const next = mount();
      current.forEach((dispose) => dispose());
      current = next;
    }
  };
  // s never changes, so the shared computeds should run 0 times per iteration, in both orders.
}
```

### 3. Create and dispose readers of a shared computed while writes happen

Readers of `shared` come and go while `s` keeps changing. `shared` should be recomputed once per write and never because a reader was added or removed, and disposed readers should not leave memory behind.

```js
function churn(fw) {
  const s = fw.signal(0);
  const shared = fw.computed(() => work(s.read()));
  return () => {
    const disposers = Array.from({ length: R }, (_, i) => {
      const row = fw.computed(() => shared.read() + i);
      return fw.effect(() => row.read());
    });
    for (let w = 0; w < W; w++) fw.withBatch(() => s.write(s.read() + 1));
    disposers.forEach((dispose) => dispose());
  };
  // shared should run W times per iteration. Also report heap retained after many iterations.
}
```

### 4. One batch writes many signals

One batch writes many signals at once. Each computed and effect that depends on them should run at most once per batch, no matter how many of its signals were written, and the cost should grow with the number of writes and effects in a measurable way.

```js
function wideBatch(fw) {
  const signals = Array.from({ length: S }, (_, i) => fw.signal(i));
  const computeds = Array.from({ length: M }, (_, i) =>
    fw.computed(() => signals[i % S].read() + signals[(i * 7) % S].read()));
  for (let e = 0; e < E; e++) {
    fw.effect(() => computeds[e % M].read() + computeds[(e * 3) % M].read() + computeds[(e * 5) % M].read());
  }
  let v = 0;
  return () => fw.withBatch(() => { v++; for (let i = 0; i < W; i++) signals[i].write(v); });
  // Each out-of-date computed and effect should run at most once per batch. Sweep W from 1 to S, E from 1 to 1,000.
}
```

### 5. Many computeds over one signal, only some with effects

Only the computeds that have an effect need to be kept up to date. Computeds that nobody reads anymore should not be recomputed on writes, and effects should only rerun for computeds whose value changed.

```js
function partialObservation(fw) {
  const s = fw.signal(0);
  const computeds = Array.from({ length: K }, (_, i) =>
    fw.computed(() => (i < c * K ? s.read() : (s.read(), 0))));
  computeds.forEach((computed, i) => {
    computed.read();
    if (i < o * K) fw.effect(() => computed.read());
  });
  return () => fw.withBatch(() => s.write(s.read() + 1));
  // Ideal per write: o * K computed runs, and o * c * K effect runs. Sweep K to 10,000, o over 0, 0.1, 1, c over 0, 0.1, 0.5, 1.
}
```

### 6. Many writes with no effect, then one read

Nobody is watching while the writes happen, so nothing should be recomputed until the read. The read should then recompute each out-of-date computed at most once, however many writes came before it.

```js
function manyWritesThenRead(fw) {
  const signals = Array.from({ length: 100 }, (_, i) => fw.signal(i));
  const layer1 = Array.from({ length: 1000 }, (_, i) =>
    fw.computed(() => signals[i % 100].read() + signals[(i + 1) % 100].read() + signals[(i + 2) % 100].read()));
  const layer2 = Array.from({ length: 100 }, (_, i) =>
    fw.computed(() => { let t = 0; for (let k = 0; k < 10; k++) t += layer1[i * 10 + k].read(); return t; }));
  const root = fw.computed(() => layer2.reduce((t, c) => t + c.read(), 0));
  root.read();
  let v = 0;
  return () => {
    for (let w = 0; w < W; w++) fw.withBatch(() => signals[w % 100].write(++v));
    root.read();
  };
  // At most 1,101 computed runs per iteration, however large W is. Sweep W from 1 to 1,000.
}
```

### 7a. A computed reads the same computeds in a different order

The list reads the same items in a new order. The items themselves have not changed, so they should not be recomputed. Only `list` and its effect should run.

```js
function reorder(fw) {
  const items = Array.from({ length: N }, (_, i) => { const s = fw.signal(i); return fw.computed(() => work(s.read())); });
  const order = fw.signal(items.map((_, i) => i));
  const list = fw.computed(() => order.read().map((i) => items[i].read()));
  fw.effect(() => list.read());
  return () => fw.withBatch(() => { const o = order.read(); order.write([...o.slice(k), ...o.slice(0, k)]); });
  // No item signal changes, so items should run 0 times per iteration. Variants: reversal, and prepending k new items.
}
```

### 7b. One computed moves from one reading computed to another

An item moves from one group to another. The item itself has not changed, so it should not be recomputed. Only the two groups and their effects should run.

```js
function move(fw, order /* 'sourceFirst' | 'targetFirst' */) {
  const items = Array.from({ length: N }, (_, i) => { const s = fw.signal(i); return fw.computed(() => work(s.read())); });
  const groups = Array.from({ length: G }, (_, g) => fw.signal(items.map((_, i) => i).filter((i) => i % G === g)));
  const groupComputeds = groups.map((ids) => fw.computed(() => ids.read().map((i) => items[i].read())));
  groupComputeds.forEach((group) => fw.effect(() => group.read()));
  let k = 0;
  return () => {
    const from = groups[k % G];
    const to = groups[(k + 1) % G];
    const [moved, ...rest] = from.read();
    fw.withBatch(() => {
      if (order === 'sourceFirst') { from.write(rest); to.write([moved, ...to.read()]); }
      else { to.write([moved, ...to.read()]); from.write(rest); }
    });
    k++;
  };
  // No item signal changes, so items should run 0 times per iteration, in both orders.
}
```

### 8. One effect switches among chains whose inputs never change

The effect switches to a chain it showed before. Nothing that chain reads has changed since then, so the chain should not be recomputed.

```js
function tabSwitch(fw) {
  const chains = Array.from({ length: T }, (_, t) => {
    let head = fw.signal(t);
    for (let level = 0; level < d; level++) { const below = head; head = fw.computed(() => below.read() + 1); }
    return head;
  });
  const tab = fw.signal(0);
  fw.effect(() => chains[tab.read()].read());
  let current = 0;
  return () => fw.withBatch(() => tab.write((current = (current + 1) % T)));
  // After each chain has been shown once, chains should run 0 times per switch.
}
```

### 9. A computed returns its previous object so its readers skip rerunning

`view` recomputes but returns the same object as before. Its readers see no change, so they and their effects should not be recomputed.

```js
function previousObjectCutoff(fw) {
  const title = fw.signal('a');
  const updatedAt = fw.signal(0);
  let previous;
  const view = fw.computed(() => {
    const next = { title: title.read(), day: Math.floor(updatedAt.read() / 86400000) };
    if (previous && previous.title === next.title && previous.day === next.day) return previous;
    return (previous = next);
  });
  for (let r = 0; r < R; r++) {
    const reader = fw.computed(() => view.read().title.length + r);
    fw.effect(() => reader.read());
  }
  let time = 0;
  return () => fw.withBatch(() => updatedAt.write((time += 1000)));
  // view runs once per write; the R readers and their effects should run 0 times.
  // Chain variant: a chain of D computeds where the one at position p returns its previous value; nothing below p should run.
}
```

### 10. One batch writes the same signal many times

Only the last value written in the batch can be seen. `c` and the effect should run once per batch, and the cost should grow with the number of writes only through the writes themselves.

```js
function repeatedWrites(fw) {
  const s = fw.signal(0);
  const c = fw.computed(() => work(s.read()));
  fw.effect(() => c.read());
  let v = 0;
  return () => fw.withBatch(() => { for (let w = 0; w < W; w++) s.write(++v); });
  // c and the effect should run once per batch. Sweep W from 1 to 1,000.
}
```

## Harness

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC at each stage of a build of `N` sources, then `N` computeds each read once, then `N` effects, then after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop, with a result unit other than milliseconds. Case 3 would also report heap retained after many cycles.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process, and an option that repeats the processes and reports the median process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and tails. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which are guarded by a check that `gc` exists, do nothing. We propose adding the flag to the package's run script.
- Counts: computation counts are asserted in some cases but never reported. We propose an optional count on each result, printed next to the time, because several cases above are about counts that a time alone does not show.
- First run: every case is warmed up before it is timed, so no result shows a first run, while applications run many computations once or a few times. We propose recording, in a fresh process, the first iteration of each case before any warmup as its own result.

We can open one PR with the cases that need no interface change (1 and 4 to 10), one PR with the disposer extension and cases 2 and 3, and one PR per harness change, and we will adjust names, parameters, and placement to fit the suite. Which of these would you accept?
