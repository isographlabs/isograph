Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the ten operations below, and each of them occurs in large apps. Would you accept PRs that add them?

| Case | Operation | Needs |
| --- | --- | --- |
| 1 | A computed moves from one effect to another in one batch | - |
| 2 | The effect reading a computed is disposed and a new one created, with no write in between | effect returns a disposer |
| 3 | Effects reading a shared computed are created and disposed while the input keeps changing | effect returns a disposer |
| 4 | One batch writes many inputs | - |
| 5 | A computed that was read once and has no effect, while its input changes | - |
| 6 | Many writes with no effect, then one read | - |
| 7 | A computed reads the same computeds in a different order | - |
| 8 | An effect stops reading a computed and later reads it again | - |
| 9 | A computed returns its previous object, so its readers should not rerun | - |
| 10 | One batch writes the same input many times | - |

## Interface

The only extension is that `effect` returns its disposer, which cases 2 and 3 need. `cleanup()` ends every effect at once, so it cannot dispose one effect while others that read the same computed stay alive. An adapter whose library cannot dispose a single effect skips these two cases.

## Cases

Each case is a minimal demonstration of the operation, written against the suite's `ReactiveFramework`. `work(value)` stands for a computation of tunable cost. A benchmark repeats the operation and scales `count`.

### 1. A computed moves from one effect to another in one batch

`derived` moves from the first effect to the second. `input` did not change, so `derived` should not be recomputed.

Failure mode: the first effect reruns and stops reading `derived`, so for a moment `derived` has no reader. An implementation that tears a computed down as soon as it has no reader throws away its value, and recomputes it when the second effect reads it.

```js
const input = framework.signal(1);
const inFirst = framework.signal(true);
const derived = framework.computed(() => work(input.read()));
framework.effect(() => {
  if (inFirst.read()) derived.read();
});
framework.effect(() => {
  if (!inFirst.read()) derived.read();
});

framework.withBatch(() => inFirst.write(false));
// derived should not rerun.
```

### 2. The effect reading a computed is replaced

The old effect is disposed, then a new one reads `derived`, with no write in between. `input` did not change, so `derived` should not be recomputed. The same holds if the new effect is created first.

Failure mode: once the old effect is disposed, `derived` has no reader. An implementation that tears it down at that point recomputes it when the new effect reads it.

```js
const input = framework.signal(1);
const derived = framework.computed(() => work(input.read()));
const dispose = framework.effect(() => derived.read());

dispose();
framework.effect(() => derived.read());
// derived should not rerun.
```

### 3. Effects come and go while the input keeps changing

Effects reading `shared` are created, `input` is written, and the effects are disposed, over and over. `shared` should run once per write, never because effects were added or removed, and disposed effects should not keep memory alive.

Failure mode: an implementation that tears `shared` down when its last effect is disposed recomputes it when the next effects read it. An implementation that never tears anything down keeps every disposed effect alive, and memory grows.

```js
const input = framework.signal(0);
const shared = framework.computed(() => work(input.read()));

const disposers = [];
for (let i = 0; i < count; i++) disposers.push(framework.effect(() => shared.read()));
framework.withBatch(() => input.write(1));
for (const dispose of disposers) dispose();

for (let i = 0; i < count; i++) framework.effect(() => shared.read());
// shared should not rerun when the new effects read it.
```

### 4. One batch writes many inputs

`sum` reads `count` inputs, and one batch writes all of them. `sum` and its effect should run once.

Failure mode: an implementation that propagates each write separately walks from every written input to `sum`, `count` times instead of once.

```js
const inputs = [];
for (let i = 0; i < count; i++) inputs.push(framework.signal(0));
const sum = framework.computed(() => {
  let total = 0;
  for (const input of inputs) total += input.read();
  return total;
});
framework.effect(() => sum.read());

framework.withBatch(() => {
  for (const input of inputs) input.write(1);
});
// sum and the effect should run once.
```

### 5. Only some computeds have an effect

`unobserved` was read once and has no effect. Nobody will read it again, so it should not be recomputed when `input` changes.

Failure mode: an implementation that keeps every computed it has read up to date recomputes `unobserved` on every write. With many such computeds, every write pays for all of them.

```js
const input = framework.signal(0);
const observed = framework.computed(() => work(input.read()));
const unobserved = framework.computed(() => work(input.read()));
framework.effect(() => observed.read());
unobserved.read();

framework.withBatch(() => input.write(1));
// observed should rerun; unobserved should not.
```

### 6. Many writes with no effect, then one read

`derived` has no effect. `input` is written `count` times, then `derived` is read once. `derived` should run once.

Failure mode: an implementation that recomputes on each write does `count` rounds of work for one read. One that walks the graph on every write pays that walk `count` times.

```js
const input = framework.signal(0);
const derived = framework.computed(() => work(input.read()));
derived.read();

for (let i = 1; i <= count; i++) framework.withBatch(() => input.write(i));
derived.read();
// derived should run once.
```

### 7. A computed reads the same computeds in a different order

`list` reads two item computeds, and the order is reversed. Neither item's input changed, so neither item should be recomputed.

Failure mode: an implementation that matches what a computed read last time by position sees both positions change, relinks both items, and may tear them down and recompute them. With `count` items, a reversal pays for all of them.

```js
const first = framework.computed(() => work(1));
const second = framework.computed(() => work(2));
const order = framework.signal([first, second]);
const list = framework.computed(() => order.read().map((item) => item.read()));
framework.effect(() => list.read());

framework.withBatch(() => order.write([second, first]));
// first and second should not rerun.
```

### 8. An effect stops reading a computed and later reads it again

`derived` is hidden in one batch and shown again in a later one. `input` did not change, so `derived` should not be recomputed when it is shown.

Failure mode: while hidden, `derived` has no reader. An implementation that tears it down then recomputes it when it is shown.

```js
const input = framework.signal(1);
const visible = framework.signal(true);
const derived = framework.computed(() => work(input.read()));
framework.effect(() => {
  if (visible.read()) derived.read();
});

framework.withBatch(() => visible.write(false));
framework.withBatch(() => visible.write(true));
// derived should not rerun.
```

### 9. A computed returns its previous object

`view` reruns after the write but returns the same object, because the field it keeps did not change. `reader` and its effect should not rerun.

Failure mode: an implementation that reruns everything below a node that reran, without checking whether its value changed, reruns `reader` and the effect.

```js
const input = framework.signal(0);
let previous;
const view = framework.computed(() => {
  const even = input.read() % 2 === 0;
  if (previous !== undefined && previous.even === even) return previous;
  previous = { even };
  return previous;
});
const reader = framework.computed(() => work(view.read().even));
framework.effect(() => reader.read());

framework.withBatch(() => input.write(2));
// view should rerun; reader and the effect should not.
```

### 10. One batch writes the same input many times

One batch writes `input` `count` times. Only the last value can be seen, so `derived` and its effect should run once.

Failure mode: an implementation that propagates on every write walks the graph `count` times, and one that runs effects per write runs the effect `count` times.

```js
const input = framework.signal(0);
const derived = framework.computed(() => work(input.read()));
framework.effect(() => derived.read());

framework.withBatch(() => {
  for (let i = 1; i <= count; i++) input.write(i);
});
// derived and the effect should run once.
```

## Harness

- Memory: the suite calls `gc()` between runs and reads no heap figure. We propose reading the heap after a forced GC before and after building `count` inputs, computeds, and effects, and again after `cleanup` and dropping every reference, and reporting bytes per node and bytes retained after the drop. Case 3 would also report heap retained after many iterations.
- Processes: every library runs in one process, and every case is warmed up under every library before any is timed, so one library's timings depend on which ran before it. We propose an option that runs each library in its own process.
- Statistics: only the fastest run is reported, which leaves out GC pauses and slow runs. We propose reporting the median and a high percentile next to the minimum, and GC time per test, which the README lists as a feature but no code measures today.
- `--expose-gc`: the Node runner does not pass the flag, so the `gc()` calls, which check that `gc` exists, do nothing. We propose adding the flag to the run script.
- Counts: computation counts are asserted in some cases but never reported. We propose reporting them next to the time, because most cases above are about how many times something runs.
- First run: every case is warmed up before it is timed, so no result shows a first run. We propose recording the first iteration of each case in a fresh process as its own result.

We can open one PR with the cases that need no interface change (1 and 4 to 10), one PR with the disposer extension and cases 2 and 3, and one PR per harness change. Which of these would you accept?
