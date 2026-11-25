# Disposing nodes

pico-js eventually disposes a cached node that nobody needs. This document uses the graph of the README. Each example continues from the previous one unless it names its starting state, and the first one starts from the state after the last line.

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

A node is needed while it has a retain or a dependent, which is a node that read it on its latest run. A node with neither is unneeded. Disposing a node removes its edges, drops its cached value, and makes it `UNCOMPUTED`, which means it has no cached value, so the next read or retain computes it again. A dependency that loses its last dependent this way and has no retain becomes unneeded in turn. A recompute that stops reading a derived node has the same effect on that node, which joins the cleanup queue if it then has no dependent and no retain.

## The cleanup queue and the scheduler

An unneeded node is not disposed at once. After `releaseRetain(doubledRetain)`, `doubled` has no retain and no dependent, so it keeps its value and edges and goes on a cleanup queue. A top-level read of a node that nothing needs, such as `readDerivedNode(doubled)` after the release, also adds an entry for the node to the cleanup queue when the read returns, whether or not the node already has one there. A read or a retain of `doubled` before the scheduler disposes it reuses its cached value, so `readDerivedNode(doubled)` still returns 6 without running its computation, unless something it read has changed. Waiting means a node that is used again soon after it becomes unneeded does not recompute.

A scheduler decides when the nodes on the cleanup queue are disposed. The README describes how to write one and when pico-js calls it. If the caller follows the contracts, pico-js never calls `scheduler.schedule` inside a batch, a computation, or a change callback, so no node is disposed while an operation that might still read it is running. A read or a retain inside a batch, and a `readDerivedNode` inside a computation that runs outside a flush, break the contracts and can call `scheduler.schedule` there, and the README states those results. Each disposal first checks that the node is still unneeded, so if `doubled` is retained again before the scheduler reaches it, the disposal skips it. When the scheduler disposes `doubled`, `sum` loses its last dependent and has no retain, so it joins the cleanup queue and is disposed next. `chosen` stays, because it has a retain.

## Updates dispose the unneeded nodes they mark

Marking takes a node out of `CLEAN` and leaves its cached value invalid or doubtful. If nobody needs that node, keeping it only means that later updates walk it. Marking therefore lists a node that it takes out of `CLEAN` while the node has no dependents and no retain, and the update disposes each listed node that is still unneeded and not `CLEAN`, and each dependency that this leaves unneeded and not `CLEAN`. A dependency that is `CLEAN` still has a current value, so it goes on the cleanup queue. When the update put nodes with callbacks on the flush queue, the flush does this after it brings them up to date and before it calls callbacks, and otherwise the update does it at its end. Inside a batch, both happen at the end of the outermost batch. An unneeded node that was already `DIRTY` or `MAYBE_DIRTY` was not taken out of `CLEAN` by this update, so it waits on the cleanup queue.

Starting from the state after the release, and before the scheduler disposes `doubled`, suppose `writeSource(a, 10)` runs. Marking makes `sum` `DIRTY` and `doubled` `MAYBE_DIRTY`, and lists `doubled`, which has no dependents and no retain. The update disposes `doubled`, and then `sum`, which is `DIRTY` and has no dependents left. `chosen` is retained and stays `DIRTY`.
