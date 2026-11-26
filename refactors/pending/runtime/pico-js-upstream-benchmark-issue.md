Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the ten operations below, and each of them occurs in large apps. Would you accept PRs that add them?

| Case | Operation | Needs |
| --- | --- | --- |
| 1 | A computed moves from one effect to another in one batch | - |
| 2 | The effect reading a computed is disposed and a new one created, with no write in between | effect returns a disposer |
| 3 | Effects reading a shared computed are created and disposed while the input keeps changing | effect returns a disposer |
| 4 | One batch writes many inputs | - |
| 5 | Of many computeds over one input, only some have an effect | - |
| 6 | Many writes with no effect, then one read | - |
| 7 | A computed reads the same computeds in a different order | - |
| 8 | An effect stops reading a computed and later reads it again | - |
| 9 | A computed returns its previous object, so its readers should not rerun | - |
| 10 | One batch writes the same input many times | - |

## Interface

The only extension is that `effect` returns its disposer, which cases 2 and 3 need. `cleanup()` ends every effect at once, so it cannot dispose one effect while others that read the same computed stay alive. An adapter whose library cannot dispose a single effect skips these two cases.

## Cases

Each case is written against the suite's `ReactiveFramework`. `work(value)` stands for a computation of tunable cost. Each function builds the graph and returns one iteration. Sizes such as `count` are swept.

### 1. A computed moves from one effect to another in one batch

Both effects rerun, and `derived` moves from the first to the second. `input` did not change, so `derived` should not be recomputed.

Failure mode: the first effect reruns and stops reading `derived`, so for a moment `derived` has no reader. An implementation that tears a computed down as soon as it has no reader throws away its value. When the second effect then reads `derived`, it recomputes it from scratch, on every iteration.

```js
function moveBetweenEffects(framework) {
  const input = framework.signal(1);
  const inFirst = framework.signal(true);
  const derived = framework.computed(() => work(input.read()));
  framework.effect(() => {
    if (inFirst.read()) derived.read();
  });
  framework.effect(() => {
    if (!inFirst.read()) derived.read();
  });
  return () => {
    framework.withBatch(() => inFirst.write(false));
    framework.withBatch(() => inFirst.write(true));
  };
  // derived should run 0 times per iteration.
}
```

### 2. The effect reading a computed is replaced

The old effect is disposed and a new one reads `derived`, with no write in between. `input` did not change, so `derived` should not be recomputed, in either order.

Failure mode: when the old effect is disposed first, `derived` briefly has no reader. An implementation that tears it down at that point recomputes it when the new effect reads it.

```js
function replaceEffect(framework, disposeFirst) {
  const input = framework.signal(1);
  const derived = framework.computed(() => work(input.read()));
  let dispose = framework.effect(() => derived.read());
  return () => {
    if (disposeFirst) {
      dispose();
      dispose = framework.effect(() => derived.read());
    } else {
      const next = framework.effect(() => derived.read());
      dispose();
      dispose = next;
    }
  };
  // derived should run 0 times per iteration, in both orders.
}
```

### 3. Effects come and go while the input keeps changing

Each iteration creates `count` effects that read `shared`, writes `input` once, and disposes the effects. `shared` should run once per iteration, for the write, and never because effects were added or removed. Disposed effects should not keep memory alive.

Failure mode: an implementation that tears `shared` down when its last effect is disposed recomputes it again when the next iteration's effects read it. An implementation that never tears anything down keeps every disposed effect alive, and memory grows with the number of iterations.

```js
function effectChurn(framework) {
  const input = framework.signal(0);
  const shared = framework.computed(() => work(input.read()));
  return () => {
    const disposers = [];
    for (let i = 0; i < count; i++) {
      disposers.push(framework.effect(() => shared.read()));
    }
    framework.withBatch(() => input.write(input.read() + 1));
    for (const dispose of disposers) dispose();
  };
  // shared should run once per iteration. Also report heap retained after many iterations.
}
```

### 4. One batch writes many inputs

`sum` reads `count` inputs, and one batch writes all of them. `sum` and its effect should run once per batch.

Failure mode: an implementation that propagates each write separately walks from every written input to `sum`, so the batch costs `count` propagations instead of one, and may run `sum` or the effect more than once.

```js
function wideBatch(framework) {
  const inputs = [];
  for (let i = 0; i < count; i++) inputs.push(framework.signal(0));
  const sum = framework.computed(() => {
    let total = 0;
    for (const input of inputs) total += input.read();
    return total;
  });
  framework.effect(() => sum.read());
  return () =>
    framework.withBatch(() => {
      for (const input of inputs) input.write(input.read() + 1);
    });
  // sum and the effect should run once per batch.
}
```

### 5. Only some computeds have an effect

`count` computeds read `input`. Each is read once, and only `observedCount` of them have an effect. Computeds without an effect are never read again, so they should not be recomputed when `input` changes.

Failure mode: an implementation that keeps every computed it has read up to date recomputes all `count` computeds on every write, including the ones nobody will read again.

```js
function partialObservation(framework) {
  const input = framework.signal(0);
  for (let i = 0; i < count; i++) {
    const derived = framework.computed(() => work(input.read()));
    derived.read();
    if (i < observedCount) framework.effect(() => derived.read());
  }
  return () => framework.withBatch(() => input.write(input.read() + 1));
  // observedCount computed runs per write, not count.
}
```

### 6. Many writes with no effect, then one read

`derived` has no effect. `input` is written `count` times, then `derived` is read once. `derived` should run once per iteration, however many writes came before the read.

Failure mode: an implementation that recomputes on each write does `count` rounds of work for one read. One that walks the graph on every write pays that walk `count` times.

```js
function manyWritesThenRead(framework) {
  const input = framework.signal(0);
  const derived = framework.computed(() => work(input.read()));
  derived.read();
  return () => {
    for (let i = 0; i < count; i++) {
      framework.withBatch(() => input.write(input.read() + 1));
    }
    derived.read();
  };
  // derived should run once per iteration.
}
```

### 7. A computed reads the same computeds in a different order

`list` reads `count` item computeds in the order stored in `order`. Each iteration reverses the order. No item's input changed, so no item should be recomputed. Only `list` and its effect should run.

Failure mode: an implementation that matches what a computed read last time by position sees every position change. It relinks every item, and it may tear items down and recompute them.

```js
function reorder(framework) {
  const items = [];
  for (let i = 0; i < count; i++) {
    const input = framework.signal(i);
    items.push(framework.computed(() => work(input.read())));
  }
  const order = framework.signal(items.map((_, i) => i));
  const list = framework.computed(() => order.read().map((i) => items[i].read()));
  framework.effect(() => list.read());
  return () => framework.withBatch(() => order.write([...order.read()].reverse()));
  // items should run 0 times per iteration.
}
```

### 8. An effect stops reading a computed and later reads it again

Each iteration hides `derived`, then shows it again, in two separate batches. `input` did not change, so `derived` should not be recomputed when it is shown again.

Failure mode: when the effect stops reading `derived`, `derived` has no reader. An implementation that tears it down then recomputes it on every show.

```js
function hideAndShow(framework) {
  const input = framework.signal(1);
  const visible = framework.signal(true);
  const derived = framework.computed(() => work(input.read()));
  framework.effect(() => {
    if (visible.read()) derived.read();
  });
  return () => {
    framework.withBatch(() => visible.write(false));
    framework.withBatch(() => visible.write(true));
  };
  // derived should run 0 times per iteration.
}
```

### 9. A computed returns its previous object

`view` builds an object from `input` and returns its previous object when the new one has the same fields. Each write changes `input` without changing the field `view` keeps, so `view` reruns and returns the same object. `reader` and its effect should not rerun.

Failure mode: an implementation that reruns everything below a node that reran, without checking whether the node's value changed, reruns `reader` and the effect on every write.

```js
function previousObject(framework) {
  const input = framework.signal(0);
  let previous;
  const view = framework.computed(() => {
    const next = { even: input.read() % 2 === 0 };
    if (previous !== undefined && previous.even === next.even) return previous;
    previous = next;
    return next;
  });
  const reader = framework.computed(() => work(view.read().even));
  framework.effect(() => reader.read());
  return () => framework.withBatch(() => input.write(input.read() + 2));
  // view runs once per write; reader and the effect should run 0 times.
}
```

### 10. One batch writes the same input many times

One batch writes `input` `count` times. Only the last value can be seen, so `derived` and its effect should run once per batch.

Failure mode: an implementation that propagates on every write walks the graph `count` times per batch, and one that runs effects per write runs the effect `count` times.

```js
function repeatedWrites(framework) {
  const input = framework.signal(0);
  const derived = framework.computed(() => work(input.read()));
  framework.effect(() => derived.read());
  return () =>
    framework.withBatch(() => {
      for (let i = 0; i < count; i++) input.write(input.read() + 1);
    });
  // derived and the effect should run once per batch.
}
```

## Harness

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC before and after building `count` inputs, computeds, and effects, and again after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop. Case 3 would also report heap retained after many iterations.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and slow runs. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which check that `gc` exists, do nothing. We propose adding the flag to the run script.
- Counts: computation counts are asserted in some cases but never reported. We propose reporting them next to the time, because most cases above are about how many times something runs.
- First run: every case is warmed up before it is timed, so no result shows a first run. We propose recording the first iteration of each case in a fresh process as its own result.

We can open one PR with the cases that need no interface change (1 and 4 to 10), one PR with the disposer extension and cases 2 and 3, and one PR per harness change. Which of these would you accept?
