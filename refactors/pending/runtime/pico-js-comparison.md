# pico-js compared with Preact Signals and alien-signals

pico-js is the engine of `pico-js.md` plus the batches and `onDirty` of `pico-js-batch.md`. Preact is `@preact/signals-core@1.14.5`. alien is `alien-signals@3.2.1`.

Citations:

- `pico-js.md:N` and `pico-js-batch.md:N` are lines of the docs in `refactors/pending/runtime/`.
- `index.ts:N` is `src/index.ts` in the npm tarball of `@preact/signals-core@1.14.5`.
- `system.mjs:N` and `index.mjs:N` are `esm/system.mjs` and `esm/index.mjs` in the npm tarball of `alien-signals@3.2.1`.

## 1. Tracking

| | How a read finds its reader | Untracked read |
|---|---|---|
| pico-js | The computation receives `callingNode` and passes it to `readSource` / `readDerived` (`pico-js.md:375-390`). | `source.value` or `pull(node)` (`pico-js.md:14`). |
| Preact | A module variable `evalContext` (`index.ts:109`) holds the running computed or effect. `_refresh` and `_start` set it (`index.ts:674-677`, `index.ts:935-936`). `addDependency` reads it (`index.ts:208-211`). | `untracked(fn)` (`index.ts:126-141`), `peek()` (`index.ts:449-451`). |
| alien | A module variable `activeSub` (`index.mjs:8`). Each read calls `link(this, activeSub, cycle)` (`index.mjs:295-298`, `index.mjs:323-326`). | `setActiveSub(undefined)` (`index.mjs:59-63`). |

```ts
// from libs/pico-js/src/graph.ts
export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  callingNode.sourceDependencies.push(source);
  return source.value;
}
```

```ts
// external @preact/signals-core@1.14.5/src/index.ts
function addDependency(signal: Signal): Node | undefined {
	if (evalContext === undefined) {
		return undefined;
	}
```

```js
// external alien-signals@3.2.1/esm/index.mjs
    const sub = activeSub;
    if (sub !== undefined) {
        link(this, sub, cycle);
    }
    return this.value;
```

## 2. States and flags

pico-js stores `status: 'Uncomputed' | 'Dirty' | 'Check' | 'Clean'` (`pico-js.md:318`).

| Library | Flags |
|---|---|
| Preact | `RUNNING`, `NOTIFIED`, `OUTDATED`, `DISPOSED`, `HAS_ERROR`, `TRACKING` (`index.ts:6-11`), plus `_version` and `_globalVersion` numbers. |
| alien | `Mutable` 1, `Watching` 2, `RecursedCheck` 4, `Recursed` 8, `Dirty` 16, `Pending` 32 (`system.mjs:1-9`), `HasChildEffect` 64 (`index.mjs:2`). |

| pico-js status | Preact computed | alien computed |
|---|---|---|
| `Uncomputed` | `_version === 0` with `OUTDATED` (`index.ts:640-641`). `_refresh` skips `needsToRecompute` while `_version` is 0 (`index.ts:669`). | `flags === 0` (`index.mjs:103`, `index.mjs:284-293`). |
| `Dirty` | Preact has no flag for it. `needsToRecompute` finds a source whose `_version` differs from the edge's `_version` (`index.ts:518`). | `Mutable \| Dirty`. `shallowPropagate` sets it during a pull (`system.mjs:206-207`), and `unwatched` sets it (`index.mjs:42`). |
| `Check` | `OUTDATED` on a `TRACKING` computed (`index.ts:739`), or any non-`TRACKING` computed whose `_globalVersion !== globalVersion` (`index.ts:661`). | `Mutable \| Pending` (`system.mjs:95`). |
| `Clean` | `TRACKING` without `OUTDATED` (`index.ts:656`), or `_globalVersion === globalVersion` (`index.ts:661`). | `Mutable` alone. |
| none | `RUNNING` marks a computation in progress. `NOTIFIED` deduplicates marking. `HAS_ERROR` marks a cached error. | `RecursedCheck` marks a computation in progress. `Recursed` marks a node re-marked during its own run. A signal's `Mutable \| Dirty` means its `pendingValue` is not yet committed (`index.mjs:304`, `index.mjs:315-321`). |

## 3. Write path

| | Equality on write | Nodes visited | What it sets | Node already marked |
|---|---|---|---|---|
| pico-js | None. The caller writes only when the value changed (contract, `pico-js.md:20`). | Every derived node that read the source, transitively, observed or not, through `dependents` sets with an explicit stack (`pico-js.md:412-456`). | Direct readers `Dirty`, and each is removed from every `dependents` set and its arrays emptied (`detach`, `pico-js.md:417-418`). Nodes above become `Check`. With batches, a node leaving `Clean` adds its `onDirty` (`pico-js-batch.md:143-145`, `pico-js-batch.md:168-170`). | A `Check` direct reader becomes `Dirty` and is not descended (`pico-js.md:421-424`). A `Check` node above is skipped (`pico-js.md:443-444`). A `Dirty` node is in no `dependents` set, so no write reaches it. Reaching one throws (`pico-js.md:425-428`). |
| Preact | `value !== this._value` (`index.ts:462`). | `_targets` lists only, which hold effects and `TRACKING` computeds (`index.ts:246-248`). An unobserved computed is not visited. Recursion through `_notify` (`index.ts:737-749`). | `_version++`, `globalVersion++` on the signal (`index.ts:469-470`). Computeds get `OUTDATED \| NOTIFIED`. Effects get `NOTIFIED` and are pushed on `batchedEffect` (`index.ts:940-946`). Edges stay subscribed. | `NOTIFIED` set: `_notify` returns without descending (`index.ts:738`). |
| alien | `pendingValue !== value` (`index.mjs:303`). | Every computed that read the signal while it was the active sub, and their readers, through `subs` lists, with a linked-list stack (`system.mjs:88-141`). | The signal gets `Mutable \| Dirty` and keeps `currentValue` until a read commits it (`index.mjs:304`). Every reached node gets `Pending`, including direct readers (`system.mjs:94-95`). `Watching` nodes are queued (`system.mjs:113-115`). Edges stay linked. | `Dirty` or `Pending` without `RecursedCheck`/`Recursed`: stops, no descent, no queue (`system.mjs:100-101`). With `Recursed` only: clears it, sets `Pending`, descends (`system.mjs:103-104`). During its own run: sets `Recursed \| Pending` if the edge was read in this run, and does not queue it (`system.mjs:106-108`). |

```ts
// from libs/pico-js/src/graph.ts
      case 'Check':
        detach(node);
        node.status = 'Dirty';
        break;
```

```ts
// external @preact/signals-core@1.14.5/src/index.ts
Computed.prototype._notify = function () {
	if (!(this._flags & NOTIFIED)) {
		this._flags |= OUTDATED | NOTIFIED;
```

```js
// external alien-signals@3.2.1/esm/system.mjs
        if (!(flags & (4 | 8 | 16 | 32))) {
            sub.flags = flags | 32;
            if (innerWrite) {
                sub.flags |= 8;
            }
        }
        else if (!(flags & (4 | 8))) {
            flags = 0;
        }
```

pico-js pays for detach-on-dirty at write time: each newly `Dirty` node deletes itself from the `dependents` set of every dependency (`pico-js.md:508-517`), and its recompute adds itself back (`pico-js.md:499-506`). Preact and alien touch no edge on write.

## 4. Read and verify path

| | Fast path | Verification | Recursion |
|---|---|---|---|
| pico-js | `Clean` returns `value` (`pico-js.md:396-397`). | `verify` pulls `derivedDependencies` in read order and recomputes as soon as the node is `Dirty` (`pico-js.md:458-467`). A dependency's `recompute` marks its readers `Dirty` when its value changed (`pico-js.md:486`). Sources are never checked, because a write already made their direct readers `Dirty`. | `pull` recurses per level. About 3,000 levels overflow (`pico-js.md:29`). |
| Preact | `TRACKING` without `OUTDATED` (`index.ts:656`), then `_globalVersion === globalVersion` (`index.ts:661`). | `needsToRecompute` walks `_sources` (signals and computeds in one list) in read order: compares `_version`, calls `_refresh()`, compares again (`index.ts:504-531`). | `_refresh` recurses per level. |
| alien | `Mutable` without `Dirty`/`Pending` returns `value` (`index.mjs:271-276`). | `checkDirty` walks `deps` in read order, updates a `Dirty` dependency and descends into a `Pending` one (`system.mjs:142-201`). A dependency that changed and has more readers marks them `Dirty` with `shallowPropagate` (`system.mjs:155-157`, `system.mjs:179-183`). | `checkDirty` descends with a linked-list stack. Only running a getter recurses. |

## 5. Dependency storage

| | Storage | Duplicate reads | Reuse across recomputes |
|---|---|---|---|
| pico-js | Two arrays on the reader, `sourceDependencies` and `derivedDependencies`, and a `Set` of `dependents` on each node (`pico-js.md:320-345`). | Kept twice in the array, once in the `Set` (`pico-js.md:528`). | The arrays are emptied with `length = 0` on dirty, and every `Set` entry is deleted and re-added (`pico-js.md:499-517`). |
| Preact | Doubly-linked `Node` edges: `_prevSource`/`_nextSource`, `_prevTarget`/`_nextTarget`, `_version`, `_rollbackNode` (`index.ts:15-35`). | `signal._node._target === evalContext` detects the second read (`index.ts:213-214`, `index.ts:285`). | `prepareSources` sets every edge's `_version` to -1 (`index.ts:533-563`). A read reuses its edge and moves it to the tail (`index.ts:250-283`). `cleanupSources` unsubscribes edges still at -1 (`index.ts:565-617`). |
| alien | Doubly-linked `Link` edges: `version`, `dep`, `sub`, `prevSub`, `nextSub`, `prevDep`, `nextDep` (`system.d.ts`). | `prevDep.dep === dep` (`system.mjs:20-21`) and `dep.subsTail` with the same `cycle` (`system.mjs:29-31`). | A recompute resets `depsTail` (`index.mjs:194`). `link` reuses the next edge if it has the same dependency (`system.mjs:23-28`). `purgeDeps` unlinks the edges after `depsTail` (`index.mjs:364-370`). |

```ts
// external @preact/signals-core@1.14.5/src/index.ts
type Node = {
	_source: Signal;
	_prevSource?: Node;
	_nextSource?: Node;
	_target: Computed | Effect;
	_prevTarget?: Node;
	_nextTarget?: Node;
	_version: number;
	_rollbackNode?: Node;
};
```

The excerpt omits the source comments. A recompute that reads the same dependencies allocates nothing in Preact or alien. In pico-js it does one `Set.delete` and one `Set.add` per edge.

## 6. Cutoff and equality

| | Derived cutoff | Previous value | Source write |
|---|---|---|---|
| pico-js | `value === previous` (`pico-js.md:482`). | The computation receives `previous` and returns it to signal no change (`pico-js.md:11`). | No check. The caller writes only on change (`pico-js.md:20`). |
| Preact | `this._value !== value` bumps `_version` (`index.ts:679-687`). No custom equality option (`index.ts:355-359`). | `_fn()` takes no argument (`index.ts:678`). | `!==` (`index.ts:462`). A batch that restores a signal's pre-batch value fast-forwards readers' versions (`index.ts:180-206`). |
| alien | `oldValue !== c.getter(oldValue)` (`index.mjs:200`). | The getter receives the old value (`index.mjs:200`), as in pico-js. | `!==` on `pendingValue` (`index.mjs:303`). A commit compares `currentValue` with `pendingValue` (`index.mjs:210`), so a write followed by a write back to the old value, with no read between, changes nothing. |

All three use `===` or `!==`, so `NaN` always counts as changed and `0` equals `-0`.

## 7. Effects and batching

| | Collection | Deduplication | Order | Flush | Writes during flush | Errors |
|---|---|---|---|---|---|---|
| pico-js | Marking adds `node.onDirty` to a caller-held `Batch` (`Set<() => void>`) when a node leaves `Clean` (`pico-js-batch.md:83`, `pico-js-batch.md:133-186`). A pull collects nothing (`pico-js-batch.md:216-237`). | By callback identity, in the `Set`. | The order in which marking reached the nodes. | The caller calls `flushBatch`, which deletes then calls each callback (`pico-js-batch.md:89-94`). The engine does not pull. The callback decides. | Writes through the same batch append to the `Set`, and the same flush calls them. | Contract: a callback does not throw (`pico-js-batch.md:19`). |
| Preact | `Effect._notify` pushes onto the `batchedEffect` list (`index.ts:940-946`). | `NOTIFIED`. | Last notified runs first, because the list is prepended (`index.ts:943-944`). | A global `batchDepth`. `batch(fn)` (`index.ts:95-106`), and every write wraps itself in `startBatch`/`endBatch` (`index.ts:472-483`). `endBatch` runs an effect only if `needsToRecompute` (`index.ts:62`). | `endBatch` loops until no effect is queued (`index.ts:51`). Over 100 iterations, a write throws (`index.ts:463-464`). | Every effect runs, and the first error is rethrown at the end (`index.ts:63-80`). |
| alien | `notify` appends the effect and its `Watching` parent effects to `queued`, reversed so the outer effect runs first (`index.mjs:21-38`). | `notify` clears `Watching`, and `propagate` stops at a node already `Pending` or `Dirty`. | Notification order, parents before children. | `startBatch`/`endBatch` on a global `batchDepth` (`index.mjs:67-74`). An unbatched write flushes (`index.mjs:308-310`). `run` reruns only if `Dirty`, or `Pending` and `checkDirty` (`index.mjs:212-216`). | A nested `flush` shares `notifyIndex` and continues the same queue (`index.mjs:253-258`). | `finally` drops the rest of the queue and sets `Watching \| Recursed` on each dropped effect (`index.mjs:261-268`). |

## 8. Unobserved nodes

| | Unsubscribes on last observer | Explicit release |
|---|---|---|
| pico-js | pico-js has no observer concept. A `Clean` or `Check` node stays in its dependencies' `dependents` sets, and every write marks it. A node leaves the sets when it becomes `Dirty` (`pico-js.md:15`). | `disposeDerivedNode` marks readers `Dirty`, detaches, and returns the node to `Uncomputed` (`pico-js.md:405-410`). |
| Preact | Yes. `_unsubscribe` clears `TRACKING` and unsubscribes from every source when `_targets` empties (`index.ts:716-735`). The first subscriber resubscribes (`index.ts:699-714`). The computed keeps `_sources` and verifies by version on read. `watched`/`unwatched` callbacks (`index.ts:394-396`, `index.ts:419-421`). | `effect()` returns `dispose` (`index.ts:982-984`). |
| alien | Yes, for a node that had readers. `unlink` calls `unwatched` when `subs` empties (`system.mjs:83-84`). A computed becomes `Mutable \| Dirty` and drops all its edges (`index.mjs:39-45`), so its next read recomputes without a cutoff check. A computed read only outside any sub never had readers and stays linked. | `effect()` and `effectScope()` return a disposer (`index.mjs:131`, `index.mjs:152`). |

## 9. Errors and cycles

| | Computation throws | Cycle |
|---|---|---|
| pico-js | Contract: it does not (`pico-js.md:21`). If it does, the node stays `Uncomputed` or `Dirty`, unattached, and its arrays keep the reads made before the throw. The next run appends to them. | Contract: the graph is a DAG (`pico-js.md:28`). A node that reads itself recurses until `RangeError`, because its status stays `Uncomputed` or `Dirty` while it runs (`pico-js.md:469-488`). |
| Preact | The error is cached as the value with `HAS_ERROR` (`index.ts:688-692`) and rethrown on read (`index.ts:761-763`). | A computed read while `RUNNING` throws `Cycle detected` (`index.ts:753-754`). So does an effect started while `RUNNING` (`index.ts:926-927`). |
| alien | No catch. `finally` restores `activeSub`, clears `RecursedCheck`, and purges unread edges (`index.mjs:201-206`). The node keeps its old value with flags `Mutable`, which is clean. | No detection. A computed that reads itself while running returns its current value (`index.mjs:271-299`). |

## 10. API surface

- pico-js: `createSourceNode`, `writeSource(batch, source, value)`, `createDerivedNode((callingNode, previous) => value)`, `readSource`, `readDerived`, `pull`, `disposeDerivedNode(batch, node)`, `createBatch`, `flushBatch`, `node.onDirty`.
- Preact: `signal`, `computed`, `effect`, `batch`, `untracked`, `action`, `createModel`, the `Signal`/`Computed`/`Effect` classes with `.value`, `.peek()`, `.subscribe()`, and `watched`/`unwatched`/`name` options (`index.ts:1162-1173`).
- alien: `signal` (one function for read and write), `computed(getter(previousValue))`, `effect`, `effectScope`, `trigger`, `startBatch`/`endBatch`/`getBatchDepth`, `getActiveSub`/`setActiveSub`, `isSignal`/`isComputed`/`isEffect`/`isEffectScope`, and `createReactiveSystem` for a custom node set (`index.d.ts`, `system.d.ts`).

## 11. Performance

Harness: js-reactivity-benchmark, Node, times in ms, lower is better. The pico-js adapters (`bench/mine/picoFramework.ts`, `bench/profile/makeAdapter.ts`) add ambient tracking (`current` and a `try`/`finally` wrapper per computation) and pull every effect node after each unbatched write or batch. The `pico-js onDirty` adapter (`bench/mine/pico2Framework.ts`) sets each effect's `onDirty` to a pull and creates a batch per unbatched write. POC v1 (`bench/poc/src/graph.ts`) replaces the arrays and `Set`s with intrusive edge lists that stay on the node and are relinked on recompute, keeps detach-on-dirty and string statuses, and uses one module-level marking stack.

`bench/out/run1.txt`, sBench:

| Test | pico-js | pico-js onDirty | Preact | alien |
|---|---|---|---|---|
| createSignals | 5.63 | 5.66 | 0.95 | 0.98 |
| createComputations | 148.31 | 159.26 | 49.35 | 27.31 |
| updateSignals | 1215.97 | 1650.33 | 223.23 | 292.56 |

`bench/poc/log.txt`, full upstream suite, per test from `bench/profile/out/r1.json`, which the log cites:

| Test | pico-js | POC v1 | Preact | alien |
|---|---|---|---|---|
| createSignals | 6.2 | 0.9 | 1.0 | 1.5 |
| createComputations | 157.0 | 40.5 | 48.0 | 35.4 |
| updateSignals | 1216.1 | 322.5 | 230.5 | 299.3 |
| avoidablePropagation | 199.5 | 96.8 | 91.3 | 92.7 |
| broadPropagation | 466.1 | 101.3 | 80.7 | 86.3 |
| deepPropagation | 154.6 | 40.1 | 32.1 | 31.7 |
| diamond | 253.7 | 86.7 | 78.9 | 81.4 |
| mux | 205.3 | 80.0 | 75.8 | 73.5 |
| repeatedObservers | 40.7 | 22.3 | 14.5 | 21.9 |
| triangle | 77.1 | 24.0 | 24.0 | 22.9 |
| unstable | 50.1 | 20.6 | 15.0 | 22.3 |
| molBench | 13.4 | 12.8 | 12.9 | 12.6 |
| cellx1000 | 15.7 | 6.0 | 10.4 | 5.9 |
| cellx2500 | 40.2 | 19.8 | 30.0 | 24.3 |
| 2-10x5 - lazy80% | 586.1 | 174.9 | 160.3 | 168.1 |
| 6-10x10 - dyn25% - lazy80% | 383.2 | 98.0 | 107.2 | 113.4 |
| 4-1000x12 - dyn5% | 807.8 | 297.1 | 280.5 | 275.7 |
| 25-1000x5 | 1150.7 | 348.8 | 381.7 | 339.0 |
| 3-5x500 | 286.7 | 83.5 | 86.6 | 87.1 |
| 6-100x15 - dyn50% | 477.0 | 143.4 | 150.2 | 149.7 |
| total | 6587.3 | 2020.1 | 1911.5 | 1944.8 |

Reasons for the gap:

| Reason | Evidence | Status |
|---|---|---|
| `Set.add`/`Set.delete` per edge in `attach`/`detach`, and the array pushes in `readSource`. | CPU profile of pico-js on sBench (`bench/profile/out/prof-pico`), self time: `detach` 40.2%, `attach` 15.4%, GC 9.1%, `readSource` 8.2%, `markDependentsDirty` 7.6%, adapter `flushEffects` 6.6%. | Measured. |
| The arrays and `Set`s as a whole. | POC v1 replaces them and the total drops from 6587.3 to 2020.1 (3.26x). | Measured as one change. The log does not split the gain between edge reuse, the module-level stack, and the removal of `Set`s. |
| POC v1 against Preact and alien: 2020.1 against 1911.5 and 1944.8 in total, slower on broadPropagation (101.3 against 80.7), deepPropagation, repeatedObservers, and updateSignals (322.5 against 230.5), faster on cellx and 25-1000x5. | POC v1 still unlinks every edge of a node it marks `Dirty` and relinks them on recompute (`bench/poc/src/graph.ts:179-234`, `bench/poc/src/graph.ts:289-356`). Preact and alien leave edges linked. POC v1 compares string statuses where they test bit flags. Its `verify` recurses where alien's `checkDirty` iterates. | Not measured. |
| The adapter pulls every effect after each write. Preact and alien run only notified effects. | The POC v1 profile (`bench/profile/out/prof-poc`) gives `flushEffects` 36.1% self time. V8 inlined engine calls into it (`verify` has no profile entry), so that share includes engine work. | Measured as a profile share. The adapter's own part is not measured. |
| `pico-js onDirty` is slower than `pico-js` on updateSignals (1650.33 against 1215.97). | Its adapter creates a `Set` per unbatched write and calls `batch.add` per observed node that leaves `Clean`. | Not measured. |

## 12. What pico-js would adopt, and what it would give up

Adopt without changing results, statuses, or `onDirty` collection:

- From both: doubly-linked edges kept on the node and reused in read order, with a tail cursor and a trim of unread edges after the run (`index.ts:533-617`, `system.mjs:18-59`, `index.mjs:364-370`). POC v1 measures this direction.
- From both: deduplicate a repeated read at read time (`index.ts:213-214`, `system.mjs:20-21`) instead of keeping duplicates in the dependency arrays.
- From alien: iterative `checkDirty` with an explicit stack (`system.mjs:142-201`), which lifts the verification part of the call-stack contract (`pico-js.md:29`). Computations that read derived nodes still recurse.
- From alien: integer bit flags in place of status strings, behind the same `Status` names. The benefit is not measured.
- From alien: one module-level marking stack instead of an array per write. POC v1 does this.
- From both: leave a `Dirty` node's edges linked instead of detaching on dirty. Results and statuses stay the same, but the `dependents` invariant (`pico-js.md:331`) and the test at `pico-js.md:886-905` change, and a later write then visits the `Dirty` node and must skip it.

Give up to match Preact fully:

- `callingNode`, for an ambient `evalContext`.
- The `Dirty`/`Check` distinction, for `OUTDATED` plus per-edge version numbers.
- Marking of unobserved nodes, for version polling and automatic unsubscription on the last observer. This needs an observer concept.
- `previous` in the computation, because `_fn()` takes no argument.
- The write contract, for `!==` on every write and batch-revert snapshots.
- Caller-held `Batch` and `onDirty` in marking order, for a global batch depth and effects run last-notified-first.
- Contract-only errors and cycles, for cached errors and `Cycle detected`.

Give up to match alien fully:

- `callingNode`, for an ambient `activeSub`.
- Direct readers marked `Dirty` on write, for `Pending` on every reached node and a lazily committed `pendingValue`.
- Detach-on-dirty, for edges that stay linked.
- Explicit `disposeDerivedNode` as the only release, for `unwatched`, which drops a node's edges and resets it to `Dirty` when its last reader unlinks.
- Caller-held `Batch` and `onDirty`, for a global batch depth and a queue that runs parent effects before children.
- Contract-only errors, for a thrown getter that leaves the node clean with its old value.
