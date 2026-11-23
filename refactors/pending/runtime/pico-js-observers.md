# pico-js: observers and batches

Status: under discussion. Not approved for implementation. Every decision below is open for the user.

This doc proposes two additions to the engine that `refactors/pending/runtime/pico-js.md` specifies:

- `onDirty`, a callback on a derived node that the engine calls after the node leaves `Clean`;
- `batch(db, fn)`, so that the callbacks of a group of writes run once, after every write in the group has marked.

A database object, `Database`, holds the batch depth and the queue of nodes whose callbacks are pending. `writeSource`, `disposeDerivedNode`, and `batch` take it as their first parameter.

## Prior art

Versions read: `@preact/signals-core@1.14.5`, `alien-signals@3.2.1`, `signal-polyfill@0.2.3` (the TC39 proposal's polyfill), `@reactively/core@0.0.8`, and `react-reconciler@0.31.0` (React 19), each from its npm tarball. alien-signals ships compiled JavaScript only, so its paths are under `esm/`. Reactively ships `dist/core.js` only.

Preact signals:

- A computed node and an effect node have a `_notify` method. `Computed.prototype._notify` sets `OUTDATED | NOTIFIED` and recurses into its targets, and skips a node that already has `NOTIFIED` (`src/index.ts:737-749`). `Computed.prototype._refresh` clears `NOTIFIED` when the node is pulled (`:646-647`).
- `Effect.prototype._notify` links the effect into the module-level list `batchedEffect` through the effect's own `_nextBatchedEffect` field and sets `NOTIFIED`, so a notified effect is not linked twice (`:940-946`). The queue costs no allocation.
- `startBatch` increments the module-level `batchDepth` (`:37-39`, `:144-146`). `endBatch` decrements it when it is above 1 and returns (`:41-45`). At the outermost level it drains `batchedEffect` in a loop that runs until no effect is linked, clears `NOTIFIED` on each effect before running it, and skips a disposed effect (`:51-73`). It decrements `batchDepth` only after the loop (`:75-76`), so a write inside an effect runs at depth 1, links its effects, and the same loop runs them.
- `endBatch` catches each effect's throw, keeps the first error, runs the remaining effects, and rethrows the first error at the end (`:61-70`, `:78-80`).
- `batch(fn)` runs `fn` directly when a batch is open, and otherwise wraps it in `startBatch` and a `finally` that calls `endBatch` (`:95-106`).
- A signal write is a batch of one: the setter calls `startBatch`, notifies its targets, and calls `endBatch` in a `finally` (`:461-485`). The setter throws `Cycle detected` when `batchIteration` exceeds 100, which catches effects that keep writing (`:463-465`).

alien-signals:

- `createReactiveSystem({ update, notify, unwatched })` takes one shared `notify` function (`esm/system.mjs:10`). `propagate` calls `notify(sub)` for a node whose flags include `Watching` (2) when marking reaches it (`:114`). `shallowPropagate`, which a pull calls after a recompute changed a value, calls `notify(sub)` for a `Pending` node that becomes `Dirty` while it has `Watching` and not `RecursedCheck` (`:202-217`).
- The library's `notify` appends the effect to the module-level array `queued` and clears the effect's `Watching` flag, so a queued effect is not queued again until it runs and its flags are reset (`esm/index.mjs:21-38`, `:9`, `:235`, `:250`).
- `startBatch` and `endBatch` move the module-level `batchDepth`, and `endBatch` calls `flush` when the depth reaches 0 (`esm/index.mjs:67-74`). A signal write calls `flush` when `batchDepth` is 0 (`:301-312`).
- `flush` runs `queued` by index until `notifyIndex` reaches `queuedLength`, so an effect queued by a write during the flush runs in the same flush (`:253-260`). It catches nothing. Its `finally` clears the rest of the queue and sets `Watching | Recursed` on each effect that did not run (`:261-269`).

TC39 signal-polyfill:

- `new Signal.subtle.Watcher(notify)` stores `notify` as the watcher node's `consumerMarkedDirty` field (`src/wrapper.ts:185-193`). Every reactive node has that field, with a no-op default (`src/graph.ts:71`, `:176`).
- `consumerMarkDirty` sets `dirty`, notifies the node's consumers, and then calls `consumerMarkedDirty` (`src/graph.ts:330-334`). It runs inside `producerNotifyConsumers`, which sets `inNotificationPhase` while marking (`:303-320`), so the watcher's `notify` runs before marking completes.
- During `notify`, a read throws (`producerAccessed`, `src/graph.ts:214-220`) and a write throws `Writes to signals not permitted during Watcher callback` (`src/wrapper.ts:70-75`). The constructor's comment states the rule: "No signals may be read or written during the notify." (`src/wrapper.ts:184`).
- `notify` is called once until `watch()` is called again: `watch` resets `dirty` (`src/wrapper.ts:207-219`), and `producerNotifyConsumers` skips a consumer that is already `dirty` (`src/graph.ts:313-315`). The polyfill has no batch. Its README tells library authors to schedule from `notify`, with `queueMicrotask` in its example (`README.md:48-73`).

Reactively:

- A node has a boolean `effect` field (`dist/core.js:29`, `:37`). `stale` pushes an effect node onto the module-level `EffectQueue` when the node leaves `CacheClean` (`:99-104`, `:5`). A recompute whose value changed sets its observers `CacheDirty` directly and pushes nothing (`:154-159`).
- `stabilize()` pulls every queued effect and empties the queue (`:194-199`). `autoStabilize` runs `stabilize` in a microtask (`:200-211`). Reactively has no batch: nothing runs until `stabilize` runs.
- Reactively's `CacheClean`, `CacheCheck`, and `CacheDirty` and its `updateIfNecessary` (`:162-175`) are the engine's `Clean`, `Check`, `Dirty`, and `verify`.

React:

- `useSyncExternalStore`'s subscription callback calls `getSnapshot` synchronously and schedules a render when the snapshot changed (`subscribeToStore` and `checkIfSnapshotChanged`, `cjs/react-reconciler.development.js:4489-4503`). A callback that is React's `onStoreChange` therefore pulls inside the callback.

Upstream pico has no comparable hook. `Database::set` stores the source and advances `current_epoch` (`crates/pico/src/database.rs:152`, `:312-331`), and a memo checks epochs when it is next called. No code runs on a write. pico's database object owns the shared write state (`current_epoch`, `database.rs:82`), and every write takes it (`db.set(..)`).

## The held design

```ts
// from libs/pico-js/src/graph.ts
export type Database = {
  batchDepth: number;
  readonly queue: Array<DerivedNode<unknown>>;
};

export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  onDirty: (() => void) | undefined;
  readonly sourceDependencies: Array<SourceNode<unknown>>;
  readonly derivedDependencies: Array<DerivedNode<unknown>>;
  readonly dependents: Set<DerivedNode<unknown>>;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

export function createDatabase(): Database;
export function batch<TValue>(db: Database, fn: () => TValue): TValue;
export function setOnDirty(node: DerivedNode<unknown>, onDirty: () => void): void;
export function clearOnDirty(node: DerivedNode<unknown>): void;
export function writeSource<TValue>(db: Database, source: SourceNode<TValue>, value: TValue): void;
export function disposeDerivedNode(db: Database, node: DerivedNode<unknown>): void;
```

- Marking pushes a node onto `db.queue` when the node leaves `Clean` and its `onDirty` is set. Marking calls no callback.
- When the batch depth is 0 after a write or a disposal, or when the outermost `batch` ends, the engine runs the queue: it calls `onDirty` of each queued node, in queue order, synchronously.
- A write outside any batch is a batch of one. Its callbacks run before `writeSource` returns, after its marking completes.
- A callback may pull, write, dispose, and call `batch`. A write inside a callback pushes onto the same queue, and the same run calls those callbacks after the current callback returns.
- A callback does not throw (contract).
- `pull`, `readSource`, and `readDerived` keep their signatures: a pull never makes a node leave `Clean` (question 6).
- Disposal clears `onDirty` (question 7).

## 1. Where the callback lives

The cost that matters is marking's hot path: the two `case 'Clean':` arms of `markDependentsDirty` (pico-js.md, `src/graph.ts`), which run once per node a write takes out of `Clean`.

- (a) An optional field `onDirty: (() => void) | undefined` on `DerivedNode`. The TC39 polyfill's `consumerMarkedDirty` is this shape (`src/graph.ts:176`). Code change: one field on the type and in `createDerivedNode`, and in each `Clean` arm `if (node.onDirty !== undefined) db.queue.push(node)`. Cost: one field per derived node (every node has it, set to `undefined`, so all nodes keep one object shape); per node leaving `Clean`, one property load and one comparison; per observed node leaving `Clean`, one array push. A node holds one callback, so a caller with several listeners on one node multiplexes them in the function it sets. Breaks nothing.
- (b) A `Set` of observed nodes on the database, scanned after each write, calling each node that is not `Clean`. This is pico-runtime.md's `notify` (Later: keeping values up to date automatically). Code change: no change to marking; a scan in `writeSource`, `disposeDerivedNode`, and `batch`. Cost: O(observers) per write, including writes that reach none of them. Unless each entry records that it was already called, an observer whose node stays marked is called on every write, and that record is the per-node state of (a) kept in a second place.
- (c) A flag on the node and one shared callback on the database that receives the node. alien-signals' `Watching` flag with the `notify` passed to `createReactiveSystem` (`esm/system.mjs:10`, `:114`) and Reactively's `effect` boolean with `EffectQueue` (`dist/core.js:29`, `:101-104`) are this shape. Cost in the hot path: the same as (a). The flag is a `bool`, which AGENTS.md, Booleans, asks to replace by an enum or by a payload-bearing field, and (a)'s `onDirty | undefined` is that payload-bearing field. The shared callback must find the node's listeners from the node, which is a `Map` lookup per notified node in the caller, and one callback per database means two independent consumers of one graph (the React bindings and an effect reconciler) share one function.

Held: (a). Reason: it costs the same as (c) in marking, needs no lookup from node to callback, and is the polyfill's shape. Open for the user.

The engine writes `onDirty` only through `setOnDirty` and `clearOnDirty`, because a caller does not write a field of a node (pico-js.md, Model). `setOnDirty` on a node whose `onDirty` is set replaces it.

## 2. When the callback runs

- (a) During marking, at the moment the node leaves `Clean`, in the arm that sets its status. This is the polyfill's `Watcher` (`consumerMarkDirty`, `src/graph.ts:330-334`). A callback that pulls during marking sees a half-marked graph. Two traces against pico-js.md's `markDependentsDirty`:

  ```text
  write S, S.dependents = {A}; A observed, its callback pulls A
  first loop visits A: Clean -> detach(A) removes A from S.dependents -> Dirty -> callback
    -> pull(A) recomputes, reads S -> attach(A) adds A to S.dependents -> A Clean
  the Set iterator of S.dependents visits A again, because a deleted and re-added entry is appended
    -> A Clean -> Dirty -> callback -> pull(A) -> re-added -> visited again, without end
  ```

  ```text
  write S, S.dependents = {A}, A.dependents = {T}; T observed through A only after A
  first loop: A Clean -> Dirty, A pushed on the stack; T is still Clean
  a callback that runs now and pulls T gets T's stored value over a Dirty A: a stale value
  the stack then marks T Check, and T's callback runs a second time for one write
  ```

  The polyfill makes this safe by forbidding reads and writes during `notify` and checking both (`src/graph.ts:214-220`, `src/wrapper.ts:70-75`).
- (b) After marking completes, before the write returns. Every invariant of pico-js.md's `DerivedNode` holds again, so a pull is a normal pull.
- (c) At the end of the outermost batch.

Held: (b), with (c) inside a batch. A write outside a batch is a batch of one, so its callbacks run as soon as its marking completes. Inside a batch, they run when the outermost batch ends, which is after the marking of every write in it completes. Reason: a callback that pulls sees a consistent graph and the values of every write in the batch, and React's `onStoreChange` pulls (`checkIfSnapshotChanged`, `react-reconciler.development.js:4494-4503`). Open for the user.

Rejected alternative: (a) with the polyfill's rule and no batch. A callback must not read, write, or dispose, and only schedules work; with no batch, the engine needs no depth, no queue, and no database. It cannot apply several writes atomically for a callback that does its work synchronously: a callback that pulls after the first of two writes sees the first value and not the second. Every callback would have to defer its pull (a microtask, as the polyfill's README does), and React's `onStoreChange` does not defer.

## 3. What a callback may do

- (a) Nothing in the graph: no pull, no write, no disposal. The polyfill's rule, checked there by throws (`src/graph.ts:214-220`, `src/wrapper.ts:70-75`). It exists because the polyfill calls `notify` during marking (question 2 (a)), which this design does not.
- (b) Pull only. A write or disposal inside a callback breaks the contract. Cost: an effect reconciler whose effect writes a source (a loader writing its item's source node) must defer that write.
- (c) Pull, write, dispose, and `batch`. The queue run keeps `db.batchDepth` at 1 while it runs, as Preact's `endBatch` keeps `batchDepth` at 1 until its loop ends (`src/index.ts:51-76`). A write inside a callback sees a depth above 0, so it marks, pushes its observed nodes onto `db.queue`, and returns without running the queue. The run iterates `db.queue` with the array iterator, which reads the array's length at each step, so it reaches the pushed nodes after the current callback returns. alien-signals' `flush` does the same by index (`esm/index.mjs:253-260`).

Held: (c). Reason: once callbacks run after marking completes, a write inside one is an ordinary write, and queuing it costs nothing beyond the run's loop. Open for the user.

A callback that writes a source its own node reads, and pulls that node, makes the node leave `Clean` again, so its callback runs again, without end. Preact counts the runs and throws `Cycle detected` past 100 (`src/index.ts:463-465`). The engine checks no misuse (pico-runtime.md, open decision 2), so held: no check, and the contract states that the writes callbacks make reach a state in which no callback writes. Cost: such a loop hangs instead of throwing. Open for the user.

## 4. Once per transition or once per batch

A node is pushed onto the queue when it leaves `Clean`. A `Check` or `Dirty` node takes the `Check` arm or no arm, and an `Uncomputed` node is in no `dependents` set, so a node does not leave `Clean` again until something pulls it. The behavior follows with no flag:

- N writes, in a batch or not, push an observed node at most once while nothing pulls it, so its callback runs at most once for them.
- After its callback runs, an observed node that nobody pulls is not called again for any number of later writes. The polyfill behaves the same until `watch()` is called (`src/wrapper.ts:207-219`), and alien-signals until the effect runs (`esm/index.mjs:26`, `:235`). pico-runtime.md's sentence "an observer whose node stays marked is called on every write until something pulls it" no longer holds.
- A node pulled inside a batch, between two writes that both reach it, leaves `Clean` twice and is pushed twice, so its callback runs twice when the batch ends.
- A node queued and then pulled before the queue runs is `Clean` when its callback runs. The callback still runs, because the node's value may have changed since its observer last read it.

Options for the duplicate push:

- (a) An `Array` queue, duplicates allowed. Held. Cost: one push per observed node leaving `Clean`; a second call only after a pull inside the batch.
- (b) A `Set` queue. Cost: a hash insertion per push. Iteration while deleting and adding is defined for `Set`, so the run works the same.
- (c) Preact's intrusive list: a `queued` flag and a next-node field on every derived node (`_nextBatchedEffect` and `NOTIFIED`, `src/index.ts:940-946`). Cost: two fields per node, and a `bool`.

Held: (a). Reason: the second call reports a second change, which a callback such as React's handles by comparing snapshots. Open for the user.

## 5. The batching API

- (a) `batch(db, fn)`, which returns what `fn` returns. Preact's `batch` (`src/index.ts:95-106`). Code change: one exported function with a `try`/`finally`. Cost: a closure per batch. A throw inside `fn` still ends the batch, because the `finally` decrements the depth and runs the queue.
- (b) `startBatch(db)` and `endBatch(db)`. alien-signals' exports (`esm/index.mjs:67-74`). Cost: a caller that throws between the two calls without its own `finally` leaves the depth above 0, and no callback of that database runs again.

Both nest by depth: an inner batch changes the depth and runs nothing.

Held: (a) alone. Reason: an unbalanced depth is not representable through it. Open for the user.

## 6. Where the depth and the queue live, and which functions take the database

The depth and the queue are shared across every write to a graph, so they cannot live on a node. Preact, alien-signals, and Reactively hold them in module-level variables (`src/index.ts:144-146`; `esm/index.mjs:5-9`; `dist/core.js:5`). The user rejected module-level state and asked for a database object as the first parameter. pico's `Database` is the precedent (`database.rs:82`, `:152`).

This reverses part of two statements in pico-runtime.md: "The engine has no graph object" and "Every engine call takes only the node it acts on" (The engine), and the "no database object" half of open decision 2 (decided). Open decision 2's other half, no write-during-computation check, stays.

Options for the database's placement:

- (a) A parameter of every function that marks or runs the queue. Held, by the user's instruction. Cost: one parameter on each such call. A caller can pass two different databases to writes on one graph, and then a write's queue runs in the wrong database; the contract forbids it and the engine does not check it.
- (b) A field on every node, given at creation (`createSourceNode(db, value)`, `createDerivedNode(db, computation)`). Cost: one field per node. A write cannot use the wrong database.
- (c) Module-level state. Rejected by the user.

The fields:

- `batchDepth: number`. A nested batch must not run the queue when it ends, so the database counts open batches; a boolean would end the outer batch at the inner batch's end. The queue run holds it at 1, so a write inside a callback queues instead of starting a second run inside the first.
- `queue: Array<DerivedNode<unknown>>`. It holds the nodes that left `Clean` with `onDirty` set and whose callbacks have not run. Question 4 holds an `Array`.

Which functions mark, and so need the database:

- `writeSource` calls `markDependentsDirty(source.dependents)`. A source's readers can be `Clean`. It needs `db`.
- `disposeDerivedNode` calls `markDependentsDirty(node.dependents)`. A `Clean` node's readers can be `Clean`. It needs `db`.
- `recompute` calls `markDependentsDirty(node.dependents)` when the new value is not `===` the previous value. `recompute` runs only on a `Dirty` node: `pull` calls it in `case 'Dirty'`, and `verify` calls it after a pull left the node `Dirty`. pico-js.md's invariant for a `Dirty` node is that every node in its `dependents` is `Check` (`src/graph.ts`, the `DerivedNode` doc comment). That holds when `recompute` reaches `markDependentsDirty`: a dependent `C` of the recomputing node `N` becomes `Clean` only by a pull of `C`, and a pull of `C` during `N`'s computation pulls `N` again (`C` read `N`), which recurses without end, so the contracts (an acyclic graph, a bounded call stack) exclude it. So `recompute`'s marking takes only `Check` nodes to `Dirty`, pushes nothing onto the marking stack, and no node leaves `Clean`. A `Check` node left `Clean` at an earlier write or disposal, and its callback was queued then.
- `computeFirst` marks nothing: an `Uncomputed` node has no readers (pico-js.md, `src/graph.ts`).
- `readSource` records a dependency and marks nothing. `readDerived` calls `pull`.

So `pull`, `readSource`, and `readDerived` need no database, provided `recompute` calls a marking function without the queue. Options:

- (a) `recompute` calls a new `markCheckDependentsDirty(dependents)` that handles `Check` and throws on every other status as a bug in pico-js. Held. Cost: a second marking function. It also drops the empty stack array `markDependentsDirty` allocates on each changed recompute.
- (b) `markDependentsDirty(db, ..)` everywhere, and `pull(db, node)`, `readDerived(db, callingNode, node)`. Cost: every pull and every tracked read carries a parameter that its marking never reads.
- (c) Question 6 (b), where `recompute` reads `node.db`.

Held: (a). Open for the user.

`setOnDirty` and `clearOnDirty` read and write only the node, so they take no database. The user's instruction names "the observer functions" among the functions that take `db`; a parameter they do not read is a field AGENTS.md says we do not have, and `noUnusedParameters` in `tsconfig.build.json` rejects it. Open for the user.

Every signature change:

```ts
// from libs/pico-js/src/graph.ts, before
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void;
export function disposeDerivedNode(node: DerivedNode<unknown>): void;
function markDependentsDirty(dependents: Set<DerivedNode<unknown>>): void;
```

```ts
// from libs/pico-js/src/graph.ts, after
export function writeSource<TValue>(
  db: Database,
  source: SourceNode<TValue>,
  value: TValue,
): void;
export function disposeDerivedNode(
  db: Database,
  node: DerivedNode<unknown>,
): void;
function markDependentsDirty(
  db: Database,
  dependents: Set<DerivedNode<unknown>>,
): void;
```

New functions: `createDatabase()`, `batch(db, fn)`, `setOnDirty(node, onDirty)`, `clearOnDirty(node)`, and the unexported `runQueue(db)` and `markCheckDependentsDirty(dependents)`. `createSourceNode`, `createDerivedNode`, `readSource`, `readDerived`, and `pull` are unchanged.

## 7. Disposal and observed nodes

- (a) Disposal clears `onDirty`. The node returns to the state of a new node (pico-js.md, Model), and a new node has no callback. A node queued in an open batch and then disposed is skipped when the queue runs, because the run reads `onDirty` when it reaches the node. Disposal calls no callback of the node itself. The readers it marks are queued as for a write. Cost: an observer whose node is disposed by someone else is never called again, and learns of it only from whoever disposed the node.
- (b) Disposal keeps `onDirty`, and a disposed `Clean` node is queued, because it leaves `Clean`. Its callback pulls and recomputes the node from scratch. Cost: an observer keeps a disposed node computing.
- (c) An observed node is a root, and disposal of a node with `onDirty` set throws. Cost: a check per disposal, and pico-runtime's `deleteRecord` disposes every node of a deleted record without asking who observes it.

Held: (a). Reason: it keeps disposal's meaning, and the run already reads `onDirty` at the time it calls it. Open for the user.

Root holds: pico-runtime.md, Later: retention, plans `rootCount` and `retainRoot` on the engine's derived node, and `rootCount` "counts retentions and observers"; `listen` adds an observer and a root hold (Later: keeping values up to date automatically). `onDirty` is not a root hold in this design. The options for `collect`:

- (a) The runtime's `listen` calls `retainRoot` beside `setOnDirty`, and `collect` reads `rootCount` only. Held. Reason: one root mechanism, and `onDirty` stays a notification.
- (b) `collect` treats `onDirty !== undefined` as a root, and `rootCount` counts retentions only. Cost: one more check per node in the reachability walk; a node with a callback that nobody retains is kept.

Open for the user.

## 8. A callback that throws

pico-runtime's `reportCallbackError` calls a function and rethrows its throw in a microtask (`libs/isograph-runtime/src/subscribe.ts:157-167`).

- (a) Contract: a callback does not throw, as a computation does not. The queue run is in a `try`/`finally` whose `finally` removes the entries it reached from `db.queue` and sets `db.batchDepth` to 0, so after a throw the database works, and the callbacks queued after the throwing one run at the next run. Cost: if the contract is broken, those callbacks wait for the next write or disposal; a throw during `batch` replaces the error `fn` threw.
- (b) Preact: catch each throw, run the rest, rethrow the first at the end (`src/index.ts:61-80`). Cost: a `try`/`catch` per callback, and an error policy in the engine.
- (c) Catch each throw and rethrow it in a microtask, which is `reportCallbackError` moved into the engine. Cost: the engine takes the runtime's error policy.
- (d) alien-signals: catch nothing, and on a throw drop the rest of the queue and re-arm the nodes that did not run (`esm/index.mjs:261-269`). Cost: the dropped nodes are not `Clean`, so they are not called until something pulls them.

Held: (a). Reason: the engine catches nothing elsewhere (pico-js.md, Model), and the Isograph runtime already wraps every callback in `reportCallbackError` (pico-runtime.md, Later: keeping values up to date automatically). Open for the user.

## `src/graph.ts`

The new type and functions, in full:

```ts
// from libs/pico-js/src/graph.ts
/**
 * The batch depth and the nodes whose `onDirty` has not run.
 * - `batchDepth` is the number of open `batch` calls, and 1 while the queue runs.
 * - `queue` holds each node that left `Clean` while its `onDirty` was set, once per time it left `Clean`, in the order it left.
 */
export type Database = {
  batchDepth: number;
  readonly queue: Array<DerivedNode<unknown>>;
};

export function createDatabase(): Database {
  return { batchDepth: 0, queue: [] };
}

export function batch<TValue>(db: Database, fn: () => TValue): TValue {
  db.batchDepth += 1;
  try {
    return fn();
  } finally {
    db.batchDepth -= 1;
    if (db.batchDepth === 0) {
      runQueue(db);
    }
  }
}

export function setOnDirty(
  node: DerivedNode<unknown>,
  onDirty: () => void,
): void {
  node.onDirty = onDirty;
}

export function clearOnDirty(node: DerivedNode<unknown>): void {
  node.onDirty = undefined;
}

function runQueue(db: Database): void {
  db.batchDepth = 1;
  let reached = 0;
  try {
    for (const node of db.queue) {
      reached += 1;
      node.onDirty?.();
    }
  } finally {
    db.queue.splice(0, reached);
    db.batchDepth = 0;
  }
}

function markCheckDependentsDirty(
  dependents: Set<DerivedNode<unknown>>,
): void {
  for (const node of dependents) {
    switch (node.status) {
      case 'Check':
        detach(node);
        node.status = 'Dirty';
        break;
      case 'Clean':
        throw new Error(
          'A Clean node is in the dependents set of a Dirty node. This is indicative of a bug in pico-js.',
        );
      case 'Dirty':
        throw new Error(
          'A Dirty node is in a dependents set. This is indicative of a bug in pico-js.',
        );
      case 'Uncomputed':
        throw new Error(
          'An Uncomputed node is in a dependents set. This is indicative of a bug in pico-js.',
        );
    }
  }
}
```

- `runQueue` reads `onDirty` when it reaches a node, so a node whose callback was cleared, or that was disposed, after it was queued is skipped.
- An array iterator compares its index with the array's current length at each step, so the loop reaches nodes a callback's write pushes.
- On a normal end `reached` is the queue's length, and `splice` empties it. On a throw, `splice` removes the entries the loop reached, including the one that threw.
- A callback's `batch` call moves the depth from 1 to 2 and back to 1, so it does not start a second run.

`DerivedNode` gains `onDirty`. The doc comment's invariants are unchanged.

```ts
// from libs/pico-js/src/graph.ts, before
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  readonly sourceDependencies: Array<SourceNode<unknown>>;
```

```ts
// from libs/pico-js/src/graph.ts, after
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
  onDirty: (() => void) | undefined;
  readonly sourceDependencies: Array<SourceNode<unknown>>;
```

```ts
// from libs/pico-js/src/graph.ts, createDerivedNode, before
  return {
    value: NOT_SET,
    status: 'Uncomputed',
    sourceDependencies: [],
```

```ts
// from libs/pico-js/src/graph.ts, createDerivedNode, after
  return {
    value: NOT_SET,
    status: 'Uncomputed',
    onDirty: undefined,
    sourceDependencies: [],
```

```ts
// from libs/pico-js/src/graph.ts, before
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  markDependentsDirty(source.dependents);
}
```

```ts
// from libs/pico-js/src/graph.ts, after
export function writeSource<TValue>(
  db: Database,
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  markDependentsDirty(db, source.dependents);
  if (db.batchDepth === 0) {
    runQueue(db);
  }
}
```

Marking calls no code outside the engine, so `writeSource` does not raise the depth around it. A write outside a batch runs the queue after its marking, which is a batch of one.

```ts
// from libs/pico-js/src/graph.ts, before
export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  markDependentsDirty(node.dependents);
  detach(node);
  node.status = 'Uncomputed';
  node.value = NOT_SET;
}
```

```ts
// from libs/pico-js/src/graph.ts, after
export function disposeDerivedNode(
  db: Database,
  node: DerivedNode<unknown>,
): void {
  markDependentsDirty(db, node.dependents);
  detach(node);
  node.status = 'Uncomputed';
  node.value = NOT_SET;
  node.onDirty = undefined;
  if (db.batchDepth === 0) {
    runQueue(db);
  }
}
```

```ts
// from libs/pico-js/src/graph.ts, before
function markDependentsDirty(dependents: Set<DerivedNode<unknown>>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  for (const node of dependents) {
    switch (node.status) {
      case 'Clean':
        detach(node);
        node.status = 'Dirty';
        stack.push(node);
        break;
```

```ts
// from libs/pico-js/src/graph.ts, after
function markDependentsDirty(
  db: Database,
  dependents: Set<DerivedNode<unknown>>,
): void {
  const stack: Array<DerivedNode<unknown>> = [];
  for (const node of dependents) {
    switch (node.status) {
      case 'Clean':
        detach(node);
        node.status = 'Dirty';
        if (node.onDirty !== undefined) {
          db.queue.push(node);
        }
        stack.push(node);
        break;
```

```ts
// from libs/pico-js/src/graph.ts, markDependentsDirty's second loop, before
      switch (dependent.status) {
        case 'Clean':
          dependent.status = 'Check';
          stack.push(dependent);
          break;
```

```ts
// from libs/pico-js/src/graph.ts, markDependentsDirty's second loop, after
      switch (dependent.status) {
        case 'Clean':
          dependent.status = 'Check';
          if (dependent.onDirty !== undefined) {
            db.queue.push(dependent);
          }
          stack.push(dependent);
          break;
```

```ts
// from libs/pico-js/src/graph.ts, recompute, before
  node.value = value;
  markDependentsDirty(node.dependents);
  return value;
```

```ts
// from libs/pico-js/src/graph.ts, recompute, after
  node.value = value;
  markCheckDependentsDirty(node.dependents);
  return value;
```

pico-js.md's notes under `src/graph.ts` gain:

- Marking pushes a node onto `db.queue` only in a `Clean` arm, so a node is queued at most once per time it leaves `Clean`.
- `recompute` runs only on a `Dirty` node, whose `dependents` are all `Check`, so `markCheckDependentsDirty` throws on any other status as a bug in pico-js, and a pull queues nothing.

pico-js.md's TypeScript rules sentence lists `DerivedNode.onDirty` among the fields that are not `readonly`, with `SourceNode.value`, `DerivedNode.value`, `DerivedNode.status`, and `Database.batchDepth`.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts, before
export {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  writeSource,
} from './graph';
export type { DerivedNode, SourceNode, Status } from './graph';
```

```ts
// from libs/pico-js/src/index.ts, after
export {
  batch,
  clearOnDirty,
  createDatabase,
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  setOnDirty,
  writeSource,
} from './graph';
export type { Database, DerivedNode, SourceNode, Status } from './graph';
```

## `README.md`

The Example section, before:

````markdown
<!-- from libs/pico-js/README.md -->
```ts
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  writeSource,
} from '@isograph/pico-js';

const person = createSourceNode({ first: 'Ada', last: 'Lovelace', age: 36 });
````

After:

````markdown
<!-- from libs/pico-js/README.md -->
```ts
import {
  createDatabase,
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  writeSource,
} from '@isograph/pico-js';

const db = createDatabase();
const person = createSourceNode({ first: 'Ada', last: 'Lovelace', age: 36 });
````

The example's three `writeSource(person, ..)` calls become `writeSource(db, person, ..)`.

The API section, before:

```markdown
<!-- from libs/pico-js/README.md -->
- `writeSource(source, value)` stores `value` in `source` and marks the derived nodes that read it. It runs no computation. `writeSource` marks the source's readers whether or not the value changed, and a caller that wants no recompute for an equal value compares before it writes.
```

```markdown
<!-- from libs/pico-js/README.md -->
- `disposeDerivedNode(node)` marks the readers of `node` `Dirty` and returns `node` to `Uncomputed`.
```

After:

```markdown
<!-- from libs/pico-js/README.md -->
- `createDatabase()` returns a database, which holds the batch depth and the nodes whose `onDirty` has not run. Every write, disposal, and batch on one graph receives the same database.
- `writeSource(db, source, value)` stores `value` in `source` and marks the derived nodes that read it. It runs no computation. `writeSource` marks the source's readers whether or not the value changed, and a caller that wants no recompute for an equal value compares before it writes. Outside a batch, it calls the `onDirty` of each node it took out of `Clean` before it returns.
- `batch(db, fn)` calls `fn` and returns its value. The `onDirty` callbacks of the writes and disposals inside `fn` run when the outermost batch ends, including when `fn` throws.
- `setOnDirty(node, onDirty)` sets the function the engine calls after `node` leaves `Clean`, and replaces one set before. `clearOnDirty(node)` removes it.
```

```markdown
<!-- from libs/pico-js/README.md -->
- `disposeDerivedNode(db, node)` marks the readers of `node` `Dirty`, returns `node` to `Uncomputed`, and clears its `onDirty`. Outside a batch, it calls the `onDirty` of each reader it took out of `Clean` before it returns.
- `Database` is the type of a database.
```

A new README section after Disposal:

````markdown
<!-- from libs/pico-js/README.md -->
## Observing a node

A node with `onDirty` set is observed. When a write or a disposal takes an observed node out of `Clean`, the engine queues the node, and after the marking completes it calls the node's `onDirty`. Outside a batch that happens before `writeSource` returns. Inside a batch it happens when the outermost `batch` call ends, so a callback sees every write of the batch.

A node leaves `Clean` once until it is pulled, so its `onDirty` runs once for any number of writes before the next pull. A callback that does not pull its node is not called again until something does. A node pulled inside a batch and marked again is queued again, and its `onDirty` runs twice when the batch ends.

A callback may pull, write, dispose, and call `batch`. Callbacks run in the order their nodes left `Clean`. A write inside a callback queues the callbacks it causes, and they run after the current callback returns, before the outermost write or batch returns.

```ts
const db = createDatabase();
const count = createSourceNode(1);
const doubled = createDerivedNode((callingNode) => readSource(callingNode, count) * 2);
pull(doubled);
setOnDirty(doubled, () => console.log(pull(doubled)));

writeSource(db, count, 2); // logs 4
batch(db, () => {
  writeSource(db, count, 3);
  writeSource(db, count, 4);
}); // logs 8, once
```
````

The Contracts section gains, and pico-js.md's Model list gains the same lines word for word:

```markdown
<!-- from libs/pico-js/README.md -->
- An `onDirty` callback does not throw.
- The writes that `onDirty` callbacks make reach a state in which no callback writes.
- Every `writeSource`, `disposeDerivedNode`, and `batch` call on one graph receives the same database.
```

pico-js.md's Model list's line "A computation does not write a source and does not dispose a node." stays: a computation still does not, and a callback is not a computation.

## Tests

Every existing test creates `const db = createDatabase();` and passes it to each `writeSource` and `disposeDerivedNode` call:

```ts
// from libs/pico-js/test/graph.test.ts, before
    pull(double);
    writeSource(a, 2);
```

```ts
// from libs/pico-js/test/graph.test.ts, after
    pull(double);
    writeSource(db, a, 2);
```

New tests in `test/graph.test.ts`, by name and assertion.

`describe('onDirty')`:

- "a write calls onDirty of an observed Clean node once, after every node is marked": the diamond of the marking tests with `onDirty` on `top`, which records the four statuses; after `writeSource`, the callback ran once and recorded `['Dirty', 'Check', 'Check', 'Check']`.
- "onDirty runs for a node the write marks Dirty and for a node it marks Check": `onDirty` on `sum` and on `label` of the marking test, each pushing its name onto a log; after one write the log is `['sum', 'label']`.
- "a second write before a pull does not call onDirty again": two writes, one call.
- "a write after a pull calls onDirty again": write, pull, write; two calls.
- "a callback that pulls receives the written value, and the node is Clean after it": the callback records `pull(node)` and `node.status`; after the write it recorded the new value and `'Clean'`.
- "onDirty set on a node that is already marked is not called by the next write, and is called by a write after a pull": write, `setOnDirty`, write: zero calls; pull, write: one call.
- "onDirty set on an Uncomputed node is not called until the node is computed": `setOnDirty` on a new node, write its source: zero calls; pull, write: one call.
- "a write to a source no node reads calls nothing": `db.queue` is empty and no callback ran.
- "clearOnDirty stops the calls": set, clear, write; zero calls.
- "setOnDirty replaces the previous callback": set `first`, set `second`, write; `first` zero calls, `second` one call.
- "a recompute during a pull calls no onDirty": `onDirty` on `top` of the diamond; write (one call, `top` not pulled); `pull(left)` recomputes `shared` and marks `right` `Dirty`; the call count stays 1 and `db.queue` is empty.

`describe('batch')`:

- "callbacks run once after the outermost batch, and see every write in it": `sum` reads `a` and `b`, its callback records `pull(sum)`; inside `batch` two writes, and the callback count read inside `fn` is 0; after `batch`, one call that recorded the sum of both new values.
- "nested batches run callbacks only when the outermost batch ends": the count is 0 after the inner `batch` returns and 1 after the outer one returns.
- "batch returns the value fn returns": `batch(db, () => 7)` is `7`.
- "a batch whose fn throws runs the queued callbacks and rethrows": `expect(() => batch(db, () => { writeSource(db, a, 2); throw error; })).toThrow(error)`, and the callback ran once.
- "a node pulled inside a batch and marked again has its callback called twice": write, pull, write inside one batch; two calls after it.
- "a node queued and then pulled inside a batch still has its callback called": write, pull inside a batch; one call, and the node is `Clean` when it runs.
- "a write inside a callback queues the callbacks it causes, and they run after the current callback returns": `x`'s callback logs `'x start'`, writes the source of `y`, logs `'x end'`; `y`'s callback logs `'y'`; after the outer write the log is `['x start', 'x end', 'y']`.
- "a batch inside a callback does not run the queue early": as the previous test with the inner write inside `batch`; the same log.
- "a callback that pulls its node and writes its source again runs again": the callback pulls, and on its first run only writes the node's source; two calls, and the node is `Clean` after the outer write.
- "after a callback throws, the callbacks queued after it run at the next write": `first` throws, `second` counts; the write throws; `db.batchDepth` is 0 and `second` has 0 calls; a write to an unrelated source runs `second` once.

`describe('disposal and onDirty')`:

- "disposing an observed node clears onDirty, and a later write calls nothing": dispose, `node.onDirty` is `undefined`, pull, write; zero calls.
- "disposing a node calls onDirty of its observed Clean readers": `outer` reads `inner`, `onDirty` on `outer`; dispose `inner`; one call, `outer` is `Dirty`.
- "an observed node disposed inside a batch after it was queued is skipped": inside a batch, write its source, then dispose it; zero calls after the batch.
- "disposing a node inside a batch runs its readers' callbacks when the batch ends": the reader's count is 0 inside `fn` and 1 after.

`test/graph.test-d.ts` gains:

```ts
// from libs/pico-js/test/graph.test-d.ts
test('batch has the type of the value fn returns', () => {
  const db = createDatabase();
  expectTypeOf(batch(db, () => 'a')).toEqualTypeOf<string>();
});
```

Its import becomes `import { batch, createDatabase, createDerivedNode, readDerived, type DerivedNode } from '../src/index';`.

## How pico-runtime.md's later sections would use it

pico-runtime.md, Later: keeping values up to date automatically, and open decisions 13 and 16, would change as follows. This doc does not edit pico-runtime.md.

- `IsographEnvironment` holds the `Database`. `normalize`'s `writeValue` and `deleteRecord` call `writeSource` and `disposeDerivedNode`, so they need it: either `Store` becomes an object that holds the database beside its record map, or `normalize`, `deleteRecord`, and `garbageCollect` gain a parameter. That choice belongs to pico-runtime.md.
- `listen` and `notify` become `setOnDirty`. A node has one `onDirty`, so the runtime keeps its listeners (a `Subscription`'s `listeners`, `libs/isograph-runtime/src/subscribe.ts:37`) and sets one callback that calls each through `reportCallbackError`. `notify`'s scan of every observer after each write is removed. React's `onStoreChange` pulls through `getSnapshot` inside the callback (`react-reconciler.development.js:4489-4503`); the callback runs after marking, so the pull sees the whole response. An observer whose node stays marked is called once, not on every write (question 4).
- `callSubscriptions` becomes a batch around `normalize`. Core change 9's `try { normalize(..) } finally { callSubscriptions(environment) }` becomes `batch(db, () => normalize(..))`, whose `finally` runs the callbacks, so a response that throws part-way still delivers its writes (open decision 4 (a)). Each subscription's node gets an `onDirty` that pulls the node and calls the subscription's callback when the pulled value is not `===` `delivered`, so a subscription no write reached costs nothing. `garbageCollectEnvironment` wraps `garbageCollect` in `batch`, so a sweep that deletes many records runs each callback once.
- `reconcileEffects` stops pulling every mounted node with an effect after every write. Each mounted node with an effect gets an `onDirty` that pulls the node and, when the value is not `===` the previous one, sets up the new effect. Values with lifecycles requires that "a replaced or disposed effect is torn down after every observer of that write was notified"; the queue runs observers and effect callbacks in the order their nodes left `Clean`, so the effect callback records the old effect's teardown in a list the runtime holds, and the runtime's write site runs that list after its `batch` call returns. That ordering holds only for writes the runtime wraps in `batch`.
- Open decision 16: `setOnDirty` runs in React's `subscribe`, at commit, as `listen` does. A node created at render has no `onDirty` until commit, and a write between render and commit reaches it unobserved; React's `useSyncExternalStore` calls `getSnapshot` after subscribing and compares, which covers that write.
- Root holds stay separate from `onDirty` (question 7 (a)): `listen` calls `retainRoot` and `setOnDirty`, and its cleanup calls both releases.

## Docs this doc amends

- `refactors/pending/runtime/pico-js.md`: `src/graph.ts`, `src/index.ts`, `README.md`, the Model list's contracts, the TypeScript rules sentence, the notes under `src/graph.ts`, and every test's `writeSource` and `disposeDerivedNode` calls.
- `refactors/pending/runtime/pico-runtime.md`: The engine ("no graph object", "takes only the node it acts on", the call list's `writeSource` and `disposeDerivedNode`); open decision 2's "no database object"; Which layer owns each item's Later line (`listen`, `notify`); Later: keeping values up to date automatically; Values with lifecycles; open decisions 13 and 16.
