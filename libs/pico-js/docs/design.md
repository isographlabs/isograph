# pico-js design

pico-js keeps a graph of source nodes and derived nodes. The graph's edges connect each derived node to the nodes its computation read on its latest run. A write invalidates the nodes that depend on the written source, without computing anything. A read recomputes lazily, and only what it needs. A node with a callback, which is a node with a retain that has an `onChange`, is brought up to date at the end of each update, so that its callback can run. The examples use the README's graph, without the computation counter.

```ts
const a = createSourceNode(1);
const b = createSourceNode(2);
const useA = createSourceNode(true);

const sum = createDerivedNode(
  (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
);
const doubled = createDerivedNode(
  (callingNode) => readDerived(callingNode, sum) * 2,
);
const chosen = createDerivedNode((callingNode) =>
  readSource(callingNode, useA)
    ? readSource(callingNode, a)
    : readSource(callingNode, b),
);

const doubledRetain = retainDerivedNode(doubled);
retainDerivedNode(chosen);
// sum is 3, doubled is 6, and chosen is 1
```

Each example starts from this state unless it says otherwise.

## Invalidate upward

`writeSource(a, 10)` stores 10 and walks from `a` to the nodes that depend on it, directly or transitively. It runs no computation. A node that read `a` directly is marked `DIRTY`, which means it must recompute before its value is used, because a source it read was written. A write is not compared with the old value. Every other node on the way is marked `MAYBE_DIRTY`, which means a dependency might have changed, so its cached value might be out of date. `sum` and `chosen` read `a`, so they become `DIRTY`. `doubled` reads `sum`, so it becomes `MAYBE_DIRTY`. A write to `b` instead would mark `sum` `DIRTY` and `doubled` `MAYBE_DIRTY`, and leave `chosen` alone.

The walk stops at a node that is already marked, because marking already walked from that node or put it on the flush queue (see "Nodes with callbacks"), except at a node that is `MAYBE_DIRTY` with `MARKED_BEYOND` (see "Marking past a node on the flush queue"). A node that read the source directly and is already `MAYBE_DIRTY` is upgraded to `DIRTY`, and the walk does not go further from it. Starting from the initial state, take `writeSource(a, 10)` followed by `writeSource(b, 3)` with no read in between. The second write reaches `sum`, which is already `DIRTY`, and stops there.

## Lazily recompute

A node is brought up to date only when something reads it, and then only as far as needed. The exception is a node with a callback, which "Nodes with callbacks" describes. A node whose cached value is known to be current is `CLEAN`. A read of a `CLEAN` node returns the cached value.

A read of a `DIRTY` node recomputes it. `readDerivedNode(chosen)` after `writeSource(a, 10)` runs the computation of `chosen`, which returns 10 and makes `chosen` `CLEAN`.

A read of a `MAYBE_DIRTY` node first brings its derived dependencies up to date, one at a time in the order the node read them. This check is called verification. It stops at the first derived dependency that changed, and then recomputes the node. A derived dependency changed if its recompute returned a value not `===` to its previous value. If none changed, the node becomes `CLEAN` without running. `readDerivedNode(doubled)` after `writeSource(a, 10)` brings `sum` up to date first. `sum` is `DIRTY`, so it recomputes to 12. Its value changed, so `doubled` recomputes to 24.

Now start from `a` at 10, `b` at 2, `sum` at 12, and `doubled` at 24, all `CLEAN`. `writeSource(a, 11)` and then `writeSource(b, 1)` mark `sum` `DIRTY` and `doubled` `MAYBE_DIRTY`. `readDerivedNode(doubled)` recomputes `sum`, which returns 12 again. The value did not change, so `doubled` becomes `CLEAN` without running. A value that did not change stops the propagation like this, and this is a cutoff.

## Nodes with callbacks

A derived node with at least one retain that `retainDerivedNode` made with an `onChange` is a node with a callback. Every other node is lazy, which means it recomputes only when something reads it. A lazy node cannot call back when its value changes, because no read would report the change, so pico-js does not leave a node with a callback for a later read. Marking puts a node with a callback that it takes out of `CLEAN` on the flush queue, and goes on past it. It marks the nodes above it `MAYBE_DIRTY`, as it marks the nodes above a lazy node, so a read of a node above a node on the flush queue verifies it. Marking does not put the nodes with callbacks above a node on the flush queue on the flush queue. At the end of the update, pico-js runs the flush, which brings each node on the flush queue up to date in the way a read would, so that its callback can run. A refresh that reads a node on the flush queue, directly or through the nodes it depends on, refreshes that node first, so every computation in the flush reads current values.

Retain `doubled` with a callback, `retainDerivedNode(doubled, (value) => console.log(value))`. `writeSource(a, 10)` marks `sum` `DIRTY`, and when marking reaches `doubled` it makes `doubled` `MAYBE_DIRTY` and puts it on the flush queue. The flush recomputes `sum` to 12 and `doubled` to 24, and the callback logs 24.

If the refresh of a node on the flush queue changed its value, pico-js marks its dependents `DIRTY`, and each node with a callback that this reaches joins the same flush queue. If its value did not change, the nodes above it stay `MAYBE_DIRTY`, the nodes with callbacks among them stay off the flush queue, and a read verifies each of them without running it. A node on the flush queue stops no marking and no verification. It saves the nodes above it only from recomputing on its account, and a lazy node in its place saves them that too. Give `sum` a callback that does nothing instead, and `doubled` no callback. `writeSource(a, 1)` stores the value `a` already has, puts `sum` on the flush queue, and marks `doubled` `MAYBE_DIRTY`. The flush recomputes `sum` to 3 and does not touch `doubled`. A read of `doubled` finds `sum` unchanged and makes `doubled` `CLEAN` without running it. With `sum` lazy, the same write marks `doubled` `MAYBE_DIRTY`, and the read recomputes `sum` to 3 and makes `doubled` `CLEAN` without running it.

### Marking past a node on the flush queue

Marking starts from a written source with a walk through lazy nodes. That walk takes each `CLEAN` node it reaches to `DIRTY`, or to `MAYBE_DIRTY` without `MARKED_BEYOND`, and goes on through the lazy nodes among them. When it takes a node with a callback out of `CLEAN`, it puts that node on the flush queue and does not go on through it. After the walk through lazy nodes, marking walks past each such node with a callback. This second walk marks each `CLEAN` node above the node with a callback `MAYBE_DIRTY` with the `MARKED_BEYOND` bit, goes on through every node it marks, lazy or with a callback, and puts none of them on the flush queue. After marking, no node above a node on the flush queue is `CLEAN`.

A node that is `MAYBE_DIRTY` with `MARKED_BEYOND` waits for the refresh of the node on the flush queue below it. If that refresh changes the value, `markFromChange` marks that node's dependents with a walk through lazy nodes: a `CLEAN` dependent becomes `DIRTY`, a dependent with `MARKED_BEYOND` becomes `DIRTY` and loses the bit, the walk goes on through lazy nodes and clears the bit on each `MAYBE_DIRTY` node with `MARKED_BEYOND` that it reaches, and each node with a callback that it marks joins the flush queue. If the refresh keeps the value, the nodes with `MARKED_BEYOND` stay `MAYBE_DIRTY` with current values, and a read verifies them without running them. A later write whose walk through lazy nodes reaches a `MAYBE_DIRTY` node with `MARKED_BEYOND` clears the bit and goes on through it, because that write can change the node through a path that no node on the flush queue is on.

A refresh in the flush can read a node with a callback that is still on the flush queue, directly or through the nodes it depends on. The pull of a node with a callback that is `DIRTY` or `MAYBE_DIRTY`, while the flush refreshes its flush queue, refreshes that node at once, appends the nodes with callbacks that its change marks to the running flush's queue, and adds the node to the list of changed nodes if its value changed. So every computation in the flush reads current values, whatever the order of the flush queue. Outside the flush's refreshes, a node with a callback that is not `CLEAN` is on no flush queue that a read can reach. Marking left it `MAYBE_DIRTY` with `MARKED_BEYOND` above a node on the flush queue that kept its value, and a pull brings it up to date as a lazy node.

### Skipping repeated walks past a node

A node on the flush queue that keeps its value still costs the walk past it. A projection that keeps its value on most writes and has N dependents with callbacks would have all N dependents visited on every such write, although after the first walk none of them is `CLEAN`. The walk that sets `MARKED_BEYOND` therefore sets the `BEYOND_DONE` bit on each node it walks, in the same step in which it marks the node's `CLEAN` dependents, and skips a node that has the bit. The bit means that no dependent of the node is `CLEAN`. Every derived dependency of a `CLEAN` node is `CLEAN`, so no node above a node with the bit is `CLEAN` either, and the skip marks nothing less than the walk would.

A dependent becomes `CLEAN` only when its computation returns, after it read the node through `readDerived`, or when verification makes it `CLEAN`, after verification visited the dependent's edge to the node. A dependent joins the node's dependents only through `readDerived`. The bit is cleared at three sites:

- `readDerived` clears the bit on the node it reads, after the pull. This covers a first compute, a recompute, a new edge, an edge moved to another node, and an edge reused by position.
- `verify` clears the bit on each dependency whose edge it visits, before it descends into the dependency or pulls it.
- `markFromChange` clears the bit on the node whose value changed.

`readDerived` and `verify` load the flags and store them only when the bit is set, so a read of a node whose bit is clear costs a branch and no store. An unlink or a disposal only removes dependents, so it keeps the bit. The bit sits in `flags` beside `ON_CHANGE_QUEUED` and `MARKED_BEYOND`, and the clears of those two bits keep it, so a derived node keeps its 12 fields.

A clear by any one dependent removes the bit from the node, so a single dependent that reruns, or that verification visits, between two writes that leave the node unchanged makes the second of those writes walk all N dependents again. A clear made inside a computation stays in effect until the computation returns, because the walk that sets `MARKED_BEYOND` and `BEYOND_DONE` starts only from a write or a release, and neither runs inside a computation. [Correctness](./correctness.md) proves that the engine gives no stale read with these clears.

### When callbacks run

Callbacks run last, for every node on the flush queue whose value changed, and only after the flush has refreshed every node on the flush queue. They never run at the moment the value of a node on the flush queue changes. Retain `doubled` with a callback, as in "Nodes with callbacks", and retain `sum` with the callback `() => console.log(readDerivedNode(doubled))`. `writeSource(a, 10)` puts `sum` on the flush queue and marks `doubled` `MAYBE_DIRTY` with `MARKED_BEYOND`, without putting `doubled` on the flush queue. The flush recomputes `sum` to 12, and only then marks `doubled` `DIRTY` and appends it to the flush queue behind `sum`. The flush refreshes `doubled` to 24 before it calls any callback, and the callback of `sum` then logs 24. Running callbacks after every refresh means each callback reads the values of the whole update, and the flush has recorded every node with a callback whose value changed before any callback runs.

## Dynamic dependencies

A node depends on what its computation read on its latest run, so its dependencies can change from one run to the next. After `writeSource(useA, false)`, the next read of `chosen` recomputes it, and it reads `useA` and then `b`, where it read `a` before. A write to `a` no longer reaches `chosen`, and a write to `b` does.

## Batches

Marking is cheap and the flush is not, so a batch separates them. A write inside a batch marks at once and puts nodes with callbacks on the flush queue, and the flush waits for the end of the outermost batch. Starting from the initial state, with a callback on `doubled`, a batch that writes 10 to `a` and 20 to `b` puts `doubled` on the flush queue once, because the second write finds `sum` already `DIRTY`. The flush brings the nodes on the flush queue up to date once, and `sum` recomputes once, to 30. Batches nest, and the flush runs in the `endWriteBatch` call that closes the outermost batch. Inside a batch, the caller writes, releases, and opens nested batches, and does not read or retain a derived node, so no computation runs until the flush. A read in the batch recomputes a node on the flush queue as a lazy node, and the flush then finds it `CLEAN` and calls no callback for the change (the README lists the full result).

## Other documents

[Disposing nodes](./disposing-nodes.md) describes when pico-js disposes a cached node, [performance](./performance.md) describes the data structures and techniques that make the algorithm fast, and [correctness](./correctness.md) proves that no computation, top-level read, or change callback receives a stale value.
