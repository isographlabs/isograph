Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the seven operations below, and each of them occurs in large apps. Would you accept PRs that add them?

| Case | Operation | Needs |
| --- | --- | --- |
| 1 | A computed moves from one effect to another in one batch | - |
| 2 | The effect reading a computed is disposed and a new one created, with no write in between | effect returns a disposer |
| 3 | Effects reading a shared computed are created and disposed while the input keeps changing | effect returns a disposer |
| 4 | A computed that was read once and has no effect, while its input changes | - |
| 5 | A computed reads the same computeds in a different order | - |
| 6 | An effect stops reading a computed and later reads it again | - |
| 7 | One batch writes the same input many times | - |

Cases 2 and 3 need `effect` to return its disposer.

## Cases

### 1. A computed moves from one effect to another in one batch

`derived` moves from the first effect to the second. `input` did not change, so `derived` should not be recomputed.

Failure mode: the first effect reruns and stops reading `derived`, so for a moment `derived` has no reader. An implementation that tears a computed down as soon as it has no reader throws away its value, and recomputes it when the second effect reads it.

```js
const input = framework.signal(1);
const inFirst = framework.signal(true);
const derived = framework.computed(() => input.read() * 2);
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

Failure mode: once the old effect is disposed, `derived` has no reader. Tearing a computed down as soon as it has no reader is a reasonable strategy, since it frees memory right away, but here it costs a recomputation when the new effect reads `derived`. No existing case measures that cost.

```js
const input = framework.signal(1);
const derived = framework.computed(() => input.read() * 2);
const dispose = framework.effect(() => derived.read());

dispose();
framework.effect(() => derived.read());
// derived should not rerun.
```

### 3. Effects come and go while the input keeps changing

Effects reading `shared` are created, `input` is written, and the effects are disposed, over and over. `shared` should run once when first read and once per write, never because effects were added or removed, and disposed effects should not keep memory alive.

Failure mode: an implementation that tears `shared` down when its last effect is disposed recomputes it when the next effects read it. An implementation that never tears anything down keeps every disposed effect alive, and memory grows.

```js
const input = framework.signal(0);
const shared = framework.computed(() => input.read() * 2);

const disposers = [];
for (let i = 0; i < count; i++) disposers.push(framework.effect(() => shared.read()));
framework.withBatch(() => input.write(1));
for (const dispose of disposers) dispose();

for (let i = 0; i < count; i++) framework.effect(() => shared.read());
// shared should not rerun when the new effects read it.
```

### 4. Only some computeds have an effect

`unobserved` was read once and has no effect. Nobody will read it again, so it should not be recomputed when `input` changes.

Failure mode: an implementation that keeps `unobserved` linked to `input` walks to it on every write, even though it does not recompute it. With many such computeds, every write pays a walk over all of them.

```js
const input = framework.signal(0);
const observed = framework.computed(() => input.read() * 2);
const unobserved = framework.computed(() => input.read() * 2);
framework.effect(() => observed.read());
unobserved.read();

framework.withBatch(() => input.write(1));
// observed should rerun; unobserved should not.
```

### 5. A computed reads the same computeds in a different order

`list` reads `count` item computeds in the order stored in `order`, and the order is reversed. No item's input changed, so no item should be recomputed. Only `list` and its effect should run.

Failure mode: an implementation that rechecks each item a computed reads, instead of reusing what it read last time, does `count` checks on every reorder, even though nothing an item reads has changed.

```js
const items = [];
for (let i = 0; i < count; i++) items.push(framework.computed(() => i));
const order = framework.signal(items);
const list = framework.computed(() => order.read().map((item) => item.read()));
framework.effect(() => list.read());

framework.withBatch(() => order.write([...items].reverse()));
// items should not rerun.
```

### 6. An effect stops reading a computed and later reads it again

`derived` is hidden in one batch and shown again in a later one. `input` did not change, so `derived` should not be recomputed when it is shown.

Failure mode: while hidden, `derived` has no reader. An implementation that tears it down then recomputes it when it is shown.

```js
const input = framework.signal(1);
const visible = framework.signal(true);
const derived = framework.computed(() => input.read() * 2);
framework.effect(() => {
  if (visible.read()) derived.read();
});

framework.withBatch(() => visible.write(false));
framework.withBatch(() => visible.write(true));
// derived should not rerun.
```

### 7. One batch writes the same input many times

One batch writes `input` `count` times. Only the last value can be seen, so `derived` and its effect should run once.

Failure mode: an implementation that propagates on every write walks the graph `count` times, and one that runs effects per write runs the effect `count` times.

```js
const input = framework.signal(0);
const derived = framework.computed(() => input.read() * 2);
framework.effect(() => derived.read());

framework.withBatch(() => {
  for (let i = 1; i <= count; i++) input.write(i);
});
// derived and the effect should run once.
```

## Harness

- Report retained heap per node and after dropping everything.
- Run each library in its own process.
- Report the median and a high percentile, not only the minimum.
- Pass `--expose-gc` in the Node runner; the `gc()` calls currently do nothing.
- Report computation counts next to times.
- Report the first, unwarmed run.

Happy to open PRs for any of these.
