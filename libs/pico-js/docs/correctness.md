# pico-js correctness

This document proves the first guarantee of the README: if the caller follows the contracts, every computation, top-level read, and change callback receives current values, and each `onChange` runs once for each change of its node's value in a flush, after every node on the flush queue is refreshed. The proof is an argument over the code in `src/`. No step depends on a test. Functions are named by file and function, and "the clear in `readDerived`" means the code in `readDerived` that removes `BEYOND_DONE` from the node it read.

## Contract and terms

The proof assumes the contracts for correctness in the README: computations are synchronous and pure and read only through `readSource` and `readDerived` with their `callingNode`; the graph has no cycles; a write batch body writes, releases, and opens nested batches, and does not read a derived node or retain one; an `onChange` calls only `readDerivedNode`; no computation or `onChange` throws; no computation calls anything else in pico-js.

- `X -> Y` means that X's dependency list holds an edge to Y, so X read Y in its last run.
- Below and above follow edges: Y is below X if there is a path `X -> ... -> Y`.
- A node with a callback is a node with a retain that has an `onChange` (`evaluation === OBSERVED`), as in the README. Every other node is lazy.
- A node is unrefreshed if it has a callback, is on the flush queue of the running flush, and has not been refreshed since it was put there.
- A step is one engine function call that runs to completion with no computation of another node inside it. The invariants below hold between steps. Inside a running computation of node N, they hold with N counted as not `CLEAN`.

The claim (I): no read returns the cached value of a `CLEAN` node that has an unrefreshed node below it.

## Paths that return a derived value or let a computation continue without recomputing

1. `readDerived` (pull.ts). It calls `pull` before it touches edges, for a new edge, for an edge moved to another node, and for an edge reused by position. Its value is the value of path 2, 3, or 4.
2. `pull` of a `CLEAN` node returns `node.value`.
3. `pull` of a lazy `MAYBE_DIRTY` node: `pullMaybeDirty` returns `node.value` when `verify` returns `CLEAN`.
4. `pull` of a `DIRTY` or `MAYBE_DIRTY` node with a callback: `pullObserved`. While the flush refreshes its flush queue (`refreshQueue !== undefined`), it calls `refreshQueued` and returns `node.value`. Otherwise it takes the lazy path.
5. `verify`. It descends into a lazy `MAYBE_DIRTY` dependency, calls `pull` on every other dependency, sets a node `CLEAN` when it has visited every edge of the node, and returns `CLEAN` for the root.
6. `refreshObserved` returns `UNCHANGED` without a recompute for a node that has no callback, for a `CLEAN` or `UNCOMPUTED` node, and for a `MAYBE_DIRTY` node that `verify` makes `CLEAN`.
7. `recompute` and `recomputeObserved` keep the previous value when the rerun returns a value `===` to it.
8. The refresh walk `refreshFlushQueue` calls `refreshQueued` on each entry in insertion order, including the entries appended during the walk.
9. `readDerivedNode` and `retainDerivedNode` (retain.ts) call `pull` and return or keep its value.
10. Reads in an `onChange`: `callOnChanges` runs after `refreshFlushQueue` returns (in `flush`, batch.ts), with `refreshQueue === undefined` and the flush state running. An `onChange` reaches the engine only through `readDerivedNode`, so this is path 9, and path 4 takes the lazy branch.
11. First computes and recomputes nested inside any of the above: `computeFirst` and `rerun` run a computation whose reads are path 1.
12. The walk that sets `MARKED_BEYOND`, the loop over `beyond` in `markMaybeDirtyQueue` (marking.ts), with its skip of a node that has `BEYOND_DONE`. It returns no value, but it decides which nodes stay `CLEAN`, so it is where I can fail.

Every path ends in path 2, 3, 4, or 6. Paths 2, 3, and 6 return a cached value. Path 4 during the walk recomputes or verifies first. So I reduces to this: whenever path 2 runs, or path 3 or 5 sets a node `CLEAN`, no unrefreshed node is below that node.

## Invariants

### C: `CLEAN` is closed downward

Every derived dependency of a `CLEAN` node is `CLEAN`. Equivalently, every node above a node that is not `CLEAN` is not `CLEAN`.

C implies I at once. An unrefreshed node is not `CLEAN`, so no node above it is `CLEAN`.

C also covers source dependencies. A write to a source marks every reader of the source (`markDirtyQueue`), so a `CLEAN` node has no source dependency written since it last ran.

### BD: `BEYOND_DONE` means that no dependent is `CLEAN`

If a node has `BEYOND_DONE`, none of its dependents is `CLEAN` (the doc comment of `BEYOND_DONE` in nodes.ts). With C, none of the nodes above it is `CLEAN`, so the walk that sets `MARKED_BEYOND` marks nothing less when it skips the node.

### W: a clear inside a computation stays in effect

A `BEYOND_DONE` bit that is cleared inside a computation of node N stays clear at least until N becomes `CLEAN`. The walk that sets `MARKED_BEYOND` is the only code that sets the bit, and that walk starts only from a write (`writeSource` through `markDirtyQueue`) or from a release (`unobserveIfLast` through `markMaybeDirtyQueue`). The contract excludes both inside a computation, and inside the `pull` that a computation's read makes, because a computation does not write or release, and nested computations are computations too.

### L: a lazy node that is not `CLEAN` has no `CLEAN` dependent

This is the lazy case of C. The proof states it separately because the comments in the code rely on it (the `DIRTY` case of `markMaybeDirtyQueue`, and `recompute`).

### M: the dependents of a lazy node that is `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, are in one of those states

Every dependent of a lazy node that is `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, is itself `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`. Every node with a callback in one of those two states is on the flush queue of the open batch or of the running flush, and the flush has not refreshed it since marking put it there, because a refresh leaves a node `CLEAN` (F).

M is needed only for the proof about callbacks, and for the claim that a `MAYBE_DIRTY` node with `MARKED_BEYOND` holds a current value once no node below it is unrefreshed.

### F: what a pull leaves behind

After `pull(n)` returns, `n` is `CLEAN`. By cases on `pull`: a `CLEAN` node returns at once; `computeFirst` and `rerun` set `CLEAN`; `verify` returns `CLEAN` only after it set the root `CLEAN`, and returns `DIRTY` only before a `recompute`; during the walk `pullObserved` calls `refreshObserved` on a node with a callback, which recomputes it, verifies it `CLEAN`, or finds it `CLEAN`. No call that follows the contract reaches a node whose computation is running, because that needs a cycle.

## Proof of C

C holds for the empty graph. Every operation that changes a status, an edge, or a flag keeps it.

### Status changes to `CLEAN`

Three stores set a node `CLEAN`: in `computeFirst`, in `rerun`, and in `verify` when the node's edges are exhausted.

- `rerun` sets `lastSourceDependency` and `lastDerivedDependency` to `undefined` before it runs the computation, so every derived edge that the node keeps after `trim` was moved or reused by a `readDerived` of this run, and `trim` drops the rest. That `readDerived` pulled the dependency, so by F the dependency was `CLEAN` when read. It is still `CLEAN` when the computation returns. A later read in the same computation can change statuses only by marking, and marking starts at a node that is recomputed or refreshed. Such a node is not `CLEAN`. If it were below an earlier dependency D, C applied to D, which was `CLEAN`, would have failed before this step. So no marking reaches D.
- `computeFirst` does not reset either cursor and does not call `trim`. An `UNCOMPUTED` node N enters `computeFirst` with no edges and with both cursors `undefined`. `createDerivedNode` creates it so, and disposal (`disposeNode` and `disposeMarked` in cleanup.ts) unlinks every edge and resets both cursors before it sets `UNCOMPUTED`. No other code makes a node `UNCOMPUTED`. While the computation of N runs, only `readSource` and `readDerived` called with N as `callingNode` write N's dependency lists or N's cursors. The computation reads only through those two functions with the `callingNode` it receives, and does not keep it (the contract of `createDerivedNode`). A nested computation writes the lists of its own node. `unlinkEdge` and `linkEdge` write lists of dependents and never `nextDependency`. No disposal of N runs inside a computation: `disposeNext` and `disposeAll` run outside computations (their contract), and `disposeMarked` runs only from `writeSource`, `endWriteBatch`, and `releaseRetain` (through `flushOrQueue` and `flushOrDisposeMarked` in batch.ts), none of which a computation calls. By induction over N's reads, each cursor is `undefined` while its list is empty and otherwise points at the tail of its list, and every edge in the list was appended in this run. At a read, the cursor is `undefined` and the list is empty, so `next` is `firstSourceDependency` or `firstDerivedDependency`, which is `undefined`; or the cursor is the tail, so `next` is its `nextDependency`, which is `undefined`. The read therefore never takes the reuse branch. It either finds the cursor's edge already on the same dependency and changes nothing, or appends a new edge whose `nextDependency` is `undefined` and moves the cursor to it. When the computation returns, each cursor is `undefined` with its list empty, or is the tail of its list, so `trim` would unlink nothing and store nothing new. No computation throws, so every `computeFirst` returns, every edge N keeps was linked by a `readDerived` of this run, and the argument of the first bullet holds for N.
- `verify`: each edge of the node was either descended into (the dependency was set `CLEAN` by `verify` or recomputed, and the ascent resumed with the next edge) or pulled (by F, the dependency is `CLEAN`). The argument of the first bullet shows that the earlier dependencies stay `CLEAN`. If a pull made the node `DIRTY`, `verify` recomputes the node instead of setting it `CLEAN`.

### Status changes away from `CLEAN`

- `markDirtyQueue` for a write. Its first loop sets each `CLEAN` reader of the source `DIRTY`. `markMaybeDirtyQueue` walks from a lazy one through lazy nodes, sets every `CLEAN` node it meets `MAYBE_DIRTY` without `MARKED_BEYOND`, and goes on through the lazy ones. A one with a callback goes on the flush queue and on `beyond`, and the walk that sets `MARKED_BEYOND` sets every `CLEAN` node above it `MAYBE_DIRTY` with `MARKED_BEYOND` and goes on through it. Each walk stops only at a node that is not `CLEAN`, above which nothing is `CLEAN` by C before the step, or at a node with `BEYOND_DONE`, above which nothing is `CLEAN` by BD and C. After the step, every node above a node that the step took out of `CLEAN` is not `CLEAN`.
- `markFromChange` after the refresh of a node P with a callback changed its value. P was not `CLEAN` before its refresh, so by C no dependent of P was `CLEAN` then. During P's computation only nodes below P run, so no dependent of P becomes `CLEAN`. So under the contract `markFromChange` never takes its `CLEAN` branches. Its other branches change `MAYBE_DIRTY` to `DIRTY` or clear `MARKED_BEYOND`, which does not change whether a node is `CLEAN`.
- `markMaybeDirtyDependentsDirty` changes only `MAYBE_DIRTY` to `DIRTY`.
- `unobserveIfLast` (retain.ts) runs `markMaybeDirtyQueue` from the node, and its walk through lazy nodes marks the node's dependents as the walk of a write does.
- Disposal sets `UNCOMPUTED` only on a node with no dependents.

### Edge changes

- Only `readDerived` links a derived edge or moves one to another dependency, from the running node, which is not `CLEAN`, to a node that `pull` just made `CLEAN` (F). C constrains only the dependencies of `CLEAN` nodes, so an edge from a node that is not `CLEAN` cannot break it, and the running node becomes `CLEAN` only under the rules above.
- `trim`, `unlinkDropped`, and disposal remove edges. Removing an edge cannot break C.

## Proof of BD

The walk that sets `MARKED_BEYOND` sets `BEYOND_DONE` on a node and then, in the same call, sets every `CLEAN` dependent of the node `MAYBE_DIRTY`. After that loop no dependent is `CLEAN`. BD can break only if a dependent becomes `CLEAN`, or a `CLEAN` dependent is linked, while the bit is set.

The bit is cleared at three sites. `readDerived` loads the flags of the node it read after its `pull`, and stores them without `BEYOND_DONE` if the bit is set. `verify` does the same for the dependency of each edge it visits, before it descends into the dependency or pulls it. `markFromChange` clears the bit on the changed node. A store that removes a clear bit would leave the flags as they were, so the conditional stores leave the same nodes with the bit as unconditional stores would. Each load is followed by its store with no call between them, so no bit that `pull` sets, such as `ON_CHANGE_QUEUED` during a refresh, is overwritten.

- A dependent becomes `CLEAN` by `computeFirst` or `rerun`, which read each kept dependency through `readDerived` (see the proof of C), and that read cleared the bit. By W, no walk that sets `MARKED_BEYOND` sets the bit again before the store of `CLEAN`. Or the dependent becomes `CLEAN` by `verify`, which cleared the bit on every dependency it visited, and it visits every edge before it sets the node `CLEAN`. `verify` runs no walk that sets `MARKED_BEYOND`, so the clears stay in effect.
- A new dependent links only through `readDerived`, which cleared the bit, and by W the bit stays clear until that dependent is `CLEAN`.
- `markFromChange` clears the bit on the changed node. C makes this clear redundant.

The clears of `ON_CHANGE_QUEUED` and `MARKED_BEYOND` use masks that keep `BEYOND_DONE`. Disposal keeps the bit, but a disposed node has no dependents, and a new dependent links through `readDerived`.

## Proof of M

M breaks only if a lazy node becomes `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, while a dependent is `MAYBE_DIRTY` with `MARKED_BEYOND`, or if a node with a callback enters one of those two states without going on the flush queue.

- The first loops of `markDirtyQueue` and `markFromChange`, and the walks through lazy nodes of `markMaybeDirtyQueue` and `markFromChange`, set `CLEAN` to `DIRTY` or to `MAYBE_DIRTY` without `MARKED_BEYOND`, or clear `MARKED_BEYOND`. In each case they go on through a lazy node to its dependents and put a one with a callback on the flush queue. A walk stops at a node that is already `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, whose dependents are in one of those states by M.
- `markMaybeDirtyDependentsDirty` sets a node `DIRTY`. Take the first time this breaks M: it is called for the dependents of a lazy node K whose recompute changed its value (in `recompute`), and one dependent J is `MAYBE_DIRTY` with `MARKED_BEYOND`. K's value changed, so a dependency of K changed since K last ran. If that dependency is a source, the write made K `DIRTY`. If it is a node with a callback whose refresh in the flush changed its value, its `markFromChange` made K `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`. If it is a lazy node, or a node with a callback that a pull outside the flush recomputed through `recompute` (the lazy branch of `pullObserved`), its `markMaybeDirtyDependentsDirty` set K `DIRTY`, and by the choice of the first break that left M intact. So K was `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, and by M its dependent J was in one of those states, not `MAYBE_DIRTY` with `MARKED_BEYOND`. The break cannot happen.
- The walk that sets `MARKED_BEYOND` sets it only on a `CLEAN` node, and by C every node below a `CLEAN` node is `CLEAN`.

## Proof of I for each path

By C, I holds between steps. For each path:

- Path 1, new edge: the value comes from `pull`. By F the node is `CLEAN`, and by C nothing unrefreshed is below it.
- Path 1, edge reused by position or moved: the same, because `readDerived` pulls before it looks at the edge.
- Path 2: by C.
- Path 3: `verify` set the node `CLEAN` under the rule proved for `verify` in the proof of C, so C holds for it.
- Path 4 during the walk: `refreshQueued` refreshes the node at once, and its computation's reads are path 1. Outside the walk no node is unrefreshed, because a batch body makes no reads (contract), and outside a batch and a flush `batchQueue` is `undefined` (`endWriteBatch` and `flushOrQueue` in batch.ts).
- Path 5: each `pull` in `verify` is path 2, 3, or 4. The descent changes no status before `verify` sets a node `CLEAN`.
- Path 6: `refreshObserved` returns `UNCHANGED` before its switch for a node that has no callback. That covers a node whose last `onChange` retain was released after marking put it on the flush queue, a release that the contracts allow inside a batch. Such a node stays `DIRTY` or `MAYBE_DIRTY`, `unobserveIfLast` marked its dependents, and a later read recomputes or verifies it as a lazy node. A node that reaches the switch has a callback, so it is retained and never disposed, so the `UNCOMPUTED` case cannot occur. The `CLEAN` case returns no value to a reader, and a later read is path 2.
- Path 7: the value is the result of a rerun whose reads are path 1.
- Path 8: each refresh is path 4 or 6. The order of the walk does not matter for I, because a pull of an unrefreshed node refreshes it at once.
- Path 9: outside a flush no node is unrefreshed. `retainDerivedNode` runs outside flushes (contract). `readDerivedNode` inside a flush runs only from an `onChange`, which is path 10.
- Path 10: when `callOnChanges` runs, the walk has ended at an `undefined` entry, so every entry was refreshed or skipped, and no node is unrefreshed. An `onChange` read can recompute lazy nodes (path 4, lazy branch), whose `recompute` calls only `markMaybeDirtyDependentsDirty`, which puts nothing on a flush queue. An `onChange` writes nothing (contract). So no node joins a flush queue while callbacks run.
- Path 11: each read of a nested computation is path 1, with the running node counted as not `CLEAN`. A computation runs only on a node that is `DIRTY` or `UNCOMPUTED`. `computeFirst` runs on an `UNCOMPUTED` node, and `recompute` and `recomputeObserved` run on a node that its status or `verify` showed to be `DIRTY`. The node keeps that status until `computeFirst` or `rerun` sets it `CLEAN`. A nested refresh's `markFromChange` can reach the running node N through N's edge from its previous run, and both loops of `markFromChange` leave a `DIRTY` or `UNCOMPUTED` node as it is, so that marking changes nothing on N. N's value is current, because each value it read was current when read and, by the argument for `rerun` in the proof of C, did not change afterwards.
- Path 12: proved under C and BD.

## Change callbacks

### Each `onChange` runs after the flush has refreshed every node on the flush queue

`flush` calls `refreshFlushQueue` and then `callOnChanges`. `refreshFlushQueue` returns only when its index reaches an `undefined` entry, so every entry, including those appended during the walk, has been refreshed. Path 10 shows that nothing is appended while callbacks run. No flush runs inside another, because no `onChange` writes or ends a batch (contract).

### Each `onChange` runs exactly when its node's value changed in the update

- A node is recorded only when `recomputeObserved` returns `CHANGED`, that is, when the rerun's value is not `===` to the previous value, and `refreshQueued` records it on the list of changed nodes.
- Each node with a callback is refreshed with a change at most once per flush. After its refresh it is `CLEAN`, so by C every node below it is `CLEAN`. A change in a flush starts only at a node that is not `CLEAN`, because no write happens during a flush, so nothing below it changes again, nothing marks it again, and no second recompute of it runs. The value that `callOnChanges` passes is therefore the value of that one changed recompute.
- No change is missed. A node with a callback whose value changes in the update has a dependency that changed. Following changed dependencies down reaches a written source or a node with a callback whose refresh changed its value. Both mark with a walk through lazy nodes, and by M that walk reaches the node with a callback and puts it on the flush queue, or the node is a direct reader of the written source and goes on the flush queue in the first loop of `markDirtyQueue`. The refresh walk refreshes it through `refreshObserved` and `recomputeObserved`, never through `recompute`, because a pull of a node with a callback during the refresh walk goes through `refreshQueued`. After the refresh walk, a node with a callback that is not `CLEAN` is `MAYBE_DIRTY` with `MARKED_BEYOND`. A lazy node below it that is `DIRTY`, or `MAYBE_DIRTY` without `MARKED_BEYOND`, would put it in one of those states by M, so no such node is below it, and its value is current, and an `onChange` read of it verifies it `CLEAN` with no recompute that changes it.
- The first compute of a node calls no `onChange`, because `computeFirst` records nothing, and a node gets a callback only after `retainDerivedNode` pulled it.

## The ascent of `verify` finds the right edge

`verify` pushes an edge on its stack only when the dependency has more than one dependent, and on the way up it takes the only dependent edge or pops the stack. This is correct only if no node on the descent path gains or loses a dependent before the ascent passes it. A node gains or loses a dependent only through a computation of a node that reads it or read it on its previous run (`readDerived` and `trim`), or through a disposal, and no disposal runs during `verify`. The computations that run during the descent are the pulls and recomputes of nodes below the last node of the path, and the recompute of a path node after the ascent has reached it. No node below the path reads a node on the path. Each derived dependency that a path node read before its edge to the next path node was verified unchanged, and no source that the path node read was written since it last ran, because a write makes each reader of the source `DIRTY`, and a path node is `MAYBE_DIRTY`. A computation is pure, so the path node's computation, run now, would read the next path node again, and through the path the nodes below it. A read of a path node by a node below the path would therefore make a cycle, which the contract excludes. So no node on the path gains or loses a dependent before the ascent passes it.

## Outside the contract

The proof covers only callers that follow the contracts. The README lists what the current version does for the violations whose result the code determines, and calls the others unspecified, including every throw into pico-js. Three of the listed results follow from the code as follows.

- A write inside an `onChange` to a source that a node with a callback depends on reaches `flushOrQueue` (batch.ts) with no batch open, which calls `flush` without looking at the flush state. The nested `flush` refreshes its flush queue, then its `callOnChanges` starts at index 0 of the list of changed nodes. The outer `callOnChanges` cleared that entry before it called the running `onChange`, and `changedCount` is still the outer count, which is at least 1, so the nested loop reads `flags` of `undefined` and throws a `TypeError`. A write to a source that no node with a callback depends on returns from `markDirtyQueue` with no flush queue, so `flushOrDisposeMarked` only disposes the unneeded nodes it marked.
- A read inside a write batch reaches `pullObserved` with `refreshQueue === undefined`, so it recomputes a node on the flush queue through `recompute`, which leaves it `CLEAN`. `refreshObserved` then returns `UNCHANGED` for it. The nodes that marking left above it with `MARKED_BEYOND` are on no flush queue, because only its refresh in the flush would have put them there through `markFromChange`, so the flush calls none of their callbacks either. If the recompute changed the value, `markMaybeDirtyDependentsDirty` sets its direct dependents among them `DIRTY`. Every marking walk stops at a `DIRTY` node without putting it on a flush queue (the `DIRTY` arms of `markDirtyQueue`, `markMaybeDirtyQueue`, and `markFromChange`), so a later write that reaches such a node with a callback neither refreshes it nor calls its callbacks. A pull brings it up to date: a read, which goes through `recompute` and calls no callback, or, in a later flush, the refresh of a node with a callback above it that the write reached through another path, which calls its callbacks.
- A computation that reads its own node, or a node above it, pulls a node that is not `CLEAN` (by C, with the running node counted as not `CLEAN`), and that pull reaches the running node's computation again, so the recursion does not end.
