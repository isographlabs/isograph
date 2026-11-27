Proposal: benchmark cases for graph operations the suite does not test

The suite has no case for the six operations below, and each of them occurs in large apps. Would you accept PRs that add them?

| Case | Operation | Needs |
| --- | --- | --- |
| 1 | A computed briefly has no reader, then is read again | effect returns a disposer (variant b) |
| 2 | Disposed effects leave nothing behind while the input keeps changing | effect returns a disposer |
| 3 | A computed that was read once and has no effect, while its input changes | - |
| 4 | A computed reads the same computeds in a different order | - |
| 5 | One batch writes the same input many times | - |
| 6 | Many writes with no effect, then one read | - |

## Cases

### 1. A computed briefly has no reader, then is read again

`derived` loses its last reader and gains a new one, with no change to `input`, so `derived` should not be recomputed. The three variants differ only in how the reader goes away.

Failure mode: an implementation that tears a computed down as soon as it has no reader throws away its value and recomputes it when the next reader reads it. Tearing down early frees memory, which is a reasonable choice, but no existing case measures what it costs.

```js
// a. The reader reruns and stops reading derived, in the same batch.
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

```js
// b. The reader is disposed and a new one is created.
const input = framework.signal(1);
const derived = framework.computed(() => input.read() * 2);
const dispose = framework.effect(() => derived.read());
dispose();
framework.effect(() => derived.read());
// derived should not rerun.
```

```js
// c. The reader stops reading derived in one batch and reads it again in a later one.
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

### 2. Disposed effects leave nothing behind

Effects reading `shared` are created and disposed, over and over, while `input` keeps changing. After an effect is disposed, a write should cost nothing for it, and its memory should be reclaimable.

Failure mode: an implementation that keeps disposed effects linked to `shared` walks every one of them on each later write, and that cost and the retained memory grow with the number of effects ever created.

```js
const input = framework.signal(0);
const shared = framework.computed(() => input.read() * 2);
for (let round = 0; round < rounds; round++) {
  const disposers = [];
  for (let i = 0; i < count; i++) disposers.push(framework.effect(() => shared.read()));
  for (const dispose of disposers) dispose();
  framework.withBatch(() => input.write(round));
}
// The cost of each write should not grow with round, and retained memory should not grow with rounds.
```

### 3. A computed read once with no effect

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

### 4. A computed reads the same computeds in a different order

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

### 5. One batch writes the same input many times

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

### 6. Many writes with no effect, then one read

`derived` has no effect. `input` is written `count` times, each write in its own batch, and then `derived` is read once. `derived` should run once, however many writes came before the read.

Failure mode: an implementation that does work per write for nodes nobody is watching pays that work `count` times for one read. Every write in the suite today is followed by an effect run or a read, so no case shows how a read's cost grows with the number of writes since the last read.

```js
const input = framework.signal(0);
const derived = framework.computed(() => input.read() * 2);
derived.read();

for (let i = 1; i <= count; i++) framework.withBatch(() => input.write(i));
derived.read();
// derived should run once.
```

## Harness

- Report retained heap per node and after dropping everything.
- Run each library in its own process.
- Report the median and a high percentile, not only the minimum.
- Pass `--expose-gc` in the Node runner; the `gc()` calls currently do nothing.
- Report computation counts next to times.
- Report the first, unwarmed run.

Happy to open PRs for any of these. Case 1b and case 2 need `effect` to return its disposer.
