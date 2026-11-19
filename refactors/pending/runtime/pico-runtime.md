# Runtime: resolver results maintained by `normalize`

Status: under discussion. Not approved for implementation. This is the planned runtime step after v1 (`refactors/pending/runtime/store.md` and `refactors/pending/runtime/reader-ast.md`). It describes only what changes from v1, and from upstream where v1 matches upstream.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; `libs/isograph-react/src` is unchanged since `79e2a3c0`, which the other runtime docs cite). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`.

## Goal

Renders do no work. The `normalizeData` that retains a query computes every resolver result its entrypoint reaches. After every later write, every result an observer needs (an observed location's, or one a needed result reads) is recomputed inside that write when one of its inputs changed, lowest height first; every other result the write reaches is marked `Stale` and recomputed when next read. A render reads a precomputed value and nothing else.

Upstream reads and evaluates during render. `useReadAndSubscribe` reads the fragment's data in a `useState` initializer (`react/useReadAndSubscribe.ts:33-35`), `useResult` calls the resolver in the render body (`react/useResult.ts:45-64`), every nested eager resolver is called again on every read (`read.ts:600`), and the component subscribes in an effect after the render (`react/useRerenderOnChange.ts:21-37`, `subscribe.ts:15-40`). After a write, `callSubscriptions` re-reads every subscription whose recorded records overlap the written ones (`subscribe.ts:63-109`, called at `makeNetworkRequest.ts:271`). The deviation is the user's: no read-and-subscribe during render, because a render that reads and evaluates does resolver work on the render path, repeats it for every component that reads the same resolver, and needs a second read in the effect to catch writes between render and subscribe (`subscribe.ts:32-36`).

## Two levels

```text
level 2: observers        listen(location, onChange)  snapshot(location)
                                 ^ notified after level 1 settles
level 1: resolver graph   a GraphNode per (record, resolver store key), in environment.graph, recomputed after normalizeData
                                 ^ reads
store:                    server values on records, in environment.store, written by normalize
```

Level 1 is the resolver graph, held on the environment beside the store (The engine, the store, and the environment). Its nodes are stored resolver results (resolver entries), one per client field, record, and argument values. Its edges are each entry's dependencies: the server values and the other resolver results its reader AST read. How level 1 brings its entries up to date after a write, and which edges it stores to do so, is the Recomputation architecture below. `normalizeData`, the environment's write, owns level 1: it computes the entries a newly retained query reaches, and after every write it recomputes the entries observers need, in height order, and marks the others `Stale`. Level 1 owns ordering. No level 2 code runs while level 1 is computing.

Level 2 is observers. An observer names one location of level 1 (a record and a resolver store key) and a no-argument callback. It holds no value. After level 1 settles, each observer whose location's stored result changed is called once. `snapshot` returns the stored result and computes nothing. This is the shape React's `useSyncExternalStore(subscribe, getSnapshot)` takes (Level 2), so a React package can render from level 2 while `@isograph/runtime` stays React-free.

With React, components are resolver results too (`@component`, Later): a component's entry is a level 1 node like any other, and a mounted component is a level 2 observer of its own entry.

## Recomputation architecture

How level 1 keeps its entries current after a write. pico's algorithm (global epoch, `time_updated` and `time_verified`, verification of dependencies before an entry, backdating) is one candidate, not a given. pico was built for the compiler, where an LSP request demands one memo and everything else stays unverified; the runtime's goal makes every observed entry demanded after every write. This section states the requirements, lays out five candidates against three examples, and compares them. The option it ends on, 3a with compile-time heights and `Stale` marks for entries nobody needs, is held by the user's direction; it is not approved for implementation. The candidates are written against the store's functions: a write is a `normalize` or a `garbageCollect`, and the state a candidate keeps is described without saying where it lives. Under the held design that state lives in `environment.graph`, and the writes are `normalizeData` and `garbageCollectEnvironment`, which call the store's functions (The engine, the store, and the environment).

### Requirements

From the goal and the decisions this doc, store.md, and reader-ast.md settle:

- R1. A render does no work. `snapshot` is a lookup; a render reads no store value and calls no resolver.
- R2. Eager where observed. The `normalize` that retains a query computes its entrypoint's result and every result that result reaches. After every later `normalize` or `garbageCollect`, every needed result is current when it returns: an entry is needed when an observer is at its location or a needed entry reads it. A result nobody needs is current when read: a write that reaches it marks it `Stale`, and the next read recomputes it (open decision 3, held (b)). A `snapshot` of a `Stale` result returns its last computed value; `listen` brings it up to date (change 4).
- R3. Order. After a write, an entry recomputes only after every stored result it reads is current. No entry is computed from a stale input, and no entry is computed twice for one write.
- R4. Equality cutoff is an optimization. It is not a requirement, and no candidate can guarantee it. An entry whose recomputed result is `===` to its previous result does not mark its dependents. This stops propagation at a primitive result and at a result that is a stored or reused object. A result that is a new object is never `===` the previous one, so it always propagates, even when it is structurally equal. A general structural equality over arbitrary JavaScript values (functions, class instances, cycles) cannot be both correct and cheap, so the runtime does not attempt one. Server values keep a check at write time: `normalize` compares each value it writes with the stored one by the shape the AST gives it (change 1), as upstream does: a leaf by `===`, a link by `entityName` and `id`, and a list of links item by item and by length. A leaf whose value is a JSON object or list is a new object on every response, so it always counts as changed. Without the check, every response marks every written field changed and the whole retained tree recomputes on every response. With `Rex` to `Ray`, `Initial` recomputes to `"R"`, a string `===` the previous one, and nothing above it recomputes. With `Rex` to `Ray`, a resolver that returns `{ length: data.name.length }` recomputes to a new `{ length: 3 }`, and every entry that reads it recomputes.

  Upstream never compares resolver outputs either. `callSubscriptionIfDataChanged` (`subscribe.ts:111-162`) re-reads the data the reader AST selects with `readButDoNotEvaluate` (`:117-136`) and compares that data, not the resolver's result, with `mergeObjectsUsingReaderAst(subscription.readerAst, oldItem, newItem)` (`:138-142`), calling the callback when the merge does not return the old item (`:152`). The merge (`areEqualWithDeepComparison.ts:4-121`) walks the reader AST, so it compares only store data: scalars by `===` (`:22-23`), lists item by item (`:33-53`), linked selections recursively (`:24-29`), links by entity and id (`:96-113`), nested resolver values by `===` (`:84-95`), and client pointers as always unequal (`:65-69`). Where a part is equal it writes the old value into the new object (`:77-82`, `:48`, `:110`) and returns the old object when every part is equal (`:120`), so equality also keeps identity stable. The well-defined place for structural equality is therefore a resolver's inputs, which are store data shaped by the reader AST. Under held 3a an entry recomputes only when an input it read changed: a server input is compared by `normalize` at write time by its AST shape (change 1), and a resolver input by `===`. Reading data along the reader AST and keeping the old object where the new data is equal (upstream's merge) is the way to give a snapshot stable identity, which `useSyncExternalStore` requires of `getSnapshot` (network-and-subscriptions.md, The snapshot: `subscriptionSnapshot` returns the same object until a re-read is unequal; react.md's `useSubscriptionSnapshot` relies on it).
- R5. Observers are called once per write, after level 1 settles, and a `snapshot` inside a callback returns a settled result.
- R6. One entry per client field, record, and argument values, shared by every parent and query that reads it (The same resolver selected twice).
- R7. Dependencies follow the data. A recomputation can read different inputs than the previous one (a link moved from `Pet:2` to `Pet:3`, a `MissingData` that became present). The next write is judged against the inputs of the last computation only.
- R8. `normalize` is the only writer of server values. It knows exactly which (record, store key) pairs it changed and which records it created, and returns them (change 1). `garbageCollect` is the only deleter, knows which records it deleted, and returns them (change 2).
- R9. Minimal machinery: the fewest stored fields and functions that meet R1 to R3 and R5 to R8, give R4's optimization, and allow R10.
- R10. Scheduled recalculation can be added later without redesign (Follow-ups 9). It has two halves. The first is laziness: an entry nobody needs is only marked on a write and is recomputed when next read. Changes 2 to 4 build it. The second is deferral: a caller-supplied scheduler decides when, and at which priority, observed entries are brought up to date, with observers notified once per pass that settled them. Changes 1 to 4 do not build it, and the architecture must allow it without undoing anything in them. Deferral requires that marking a write is separate from recomputing, that a pass can leave an entry marked for a later pass, and that the observers needing an entry are counted on the entry.

Cost terms used below, per write:

- `C`: the (record, store key) pairs the write changed and the records it created or deleted.
- `A`: the entries that read something in `C`, directly or through other entries (the affected set).
- `A'`: the entries that recompute, a subset of `A`. An entry is in `A'` when at least one input changed: a server value in `C`, a record created or deleted in `C`, or a stored result whose new result is not `===` its previous one. When every resolver in `A` returns a new object, `A'` is `A`.
- `L` and `E`: the live entries (those a retained or observed location reaches) and the dependency edges among them and to server values.
- `R`: the retained and observed locations.
- `S`: the server values in the store.

### The examples

The fixture of change 2 (Tests).

Diamond. `Badge` (A) reads `Monogram` (B) and `Signature` (C). Both read `Initial` (D). `Initial` reads `Pet:1.name`; `Signature` also reads `Pet:1.nickname`. `BadgePage` on `Query.__ROOT` reads `pet____id___"1"` and `Badge` on `Pet:1`. Two writes: `name` from `Rex` to `Max`, where every entry above `Initial` changes; and `Rex` to `Ray`, where `Initial` recomputes to `"R"`, a string `===` the previous one.

New object. `NameShape` on `Pet:1` reads `Pet:1.name` and returns `{ length: data.name.length }`. `ShapeLabel` reads `NameShape` and returns `` `${data.NameShape.length} letters` ``. `ShapePage` on `Query.__ROOT` reads `pet____id___"1"` and `ShapeLabel` on `Pet:1`. One write: `name` from `Rex` to `Ray`, where `NameShape` recomputes to a new `{ length: 3 }`, structurally equal and not `===`, and `ShapeLabel` recomputes to `"3 letters"`, `===` the previous string.

Chain across fragments. `FriendPage` on `Query.__ROOT` reads `FriendCard` on `Pet:1`, which reads `FriendLabel` on `Pet:1`, which reads `Pet:1.bestFriend` (`Pet:2`) and `PetName` on `Pet:2`. A second retained query, `PetSummary` with `{ id: "2" }`, reads `PetName` on `Pet:2` directly. Two writes: `Pet:2.name` changes; then `Pet:1.bestFriend` moves to `Pet:3`.

R3 excludes push without order: `Initial` changes and notifies `Monogram` and `Signature`; `Monogram` recomputes and notifies `Badge`; `Badge` recomputes reading the old `Signature`; `Signature` recomputes and notifies `Badge` again. `Badge` runs twice, once from a stale input. Every candidate below is a way to avoid this.

### Candidate 1: upstream, re-read every overlapping subscription

No stored resolver results. Each mounted component holds a subscription with the data it last read and the records that read visited:

```ts
// external isograph/libs/isograph-react/src/core/IsographEnvironment.ts
export type FragmentSubscription<TReadFromStore extends UnknownTReadFromStore> =
  {
    readonly kind: 'FragmentSubscription';
    readonly callback: (
      newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
    ) => void;
    /** The value read out from the previous call to readButDoNotEvaluate */
    encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>;
    readonly fragmentReference: FragmentReference<TReadFromStore, any>;
    readonly readerAst: ReaderAst<TReadFromStore>;
  };
```

`WithEncounteredRecords` is `{ encounteredRecords: EncounteredIds; item }` (`read.ts:55-58`), and `EncounteredIds` is `Map<TypeName, Set<DataId>>` (`cache.ts:151`).

A write:

1. `normalizeData` writes the response and adds each record with a changed field to `encounteredIds` (`cache.ts:211-218`).
2. `callSubscriptions` loops over every subscription (`subscribe.ts:67`). For each whose `encounteredRecords` overlap the written records (`subscribe.ts:74-79`), it re-reads the whole fragment with `readButDoNotEvaluate` (`subscribe.ts:117`), which calls every nested eager resolver (`read.ts:600`).
3. It compares the new data with the old by `mergeObjectsUsingReaderAst` (`areEqualWithDeepComparison.ts:55-121`), which walks the reader AST: store data by its shape (scalars `===`, lists item by item, linked selections recursively, links by id), nested resolver values by `===` (`:84-95`). It compares the data the resolver receives, never the resolver's result. When unequal, it calls the callback, which is React's state setter (`react/useRerenderOnChange.ts:21-37`).
4. React re-renders the component, and the render calls the fragment's own resolver (`react/useResult.ts:45-64`, `react/useReadAndSubscribe.ts` `componentFunction`).

Diamond, `Rex` to `Max`, with one component reading `BadgePage`: the re-read calls `Initial` (through `Monogram`), `Monogram`, `Initial` again (through `Signature`), `Signature`, and `Badge`; the render calls `BadgePage`. `Initial` runs twice. `Rex` to `Ray`: the same five calls, because nothing is stored; `BadgePage`'s data holds `Badge`'s result, a new object, so the merge finds it unequal by `===` and the component re-renders.

Chain, `Pet:2.name`: the `FriendPage` component and the `PetSummary` component both overlap `Pet:2`; each re-read calls `PetName` on `Pet:2`, so it runs twice. The link move: the re-read follows the new link; `encounteredRecords` is replaced by the new read's, so R7 holds by construction.

- Ordering: correct trivially. Every re-read runs after the write finished and reads the whole store; nothing is stored, so nothing is stale.
- Cost per write: for each overlapping subscription, its whole reader tree and every resolver under it; a resolver shared by `k` subscriptions runs `k` times. Overlap is by record (`cache.ts:211-218`), so a write to `Pet:1.tagline` re-reads every subscription that read any field of `Pet:1`.
- Memory: per subscription, the last data and the set of records read.
- Needs from `normalize`: the changed records (`EncounteredIds`).
- Equality cutoff: at the subscription only, on the data the reader AST selects, where structural comparison is well-defined; the merge keeps the old object, so a subscription's data keeps its identity across an equal re-read. No resolver output is compared except as a nested value by `===`: a nested resolver that returns a new object makes its parent's data unequal on every re-read.
- Against the requirements: fails R1 (the render calls the fragment's resolver, and the first render reads the store), R2 (nothing is computed at normalization), R3's "nothing twice" (a shared resolver runs once per subscription, and once per path inside one), and R6. Meets R5 for the subscription callbacks and R7. Of R4's optimization it has none below the subscription: every resolver under an overlapping subscription runs.

Step A is this candidate with v1's `read` and no overlap filter.

### Candidate 2: pico's pull, epochs and verification

Data structures:

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = {
  readonly records: Map<EntityName, Map<RecordId, StoreRecord>>;
  /** The epoch of the last `normalize` or `garbageCollect` that changed `records`. */
  epoch: Epoch;
};

export type Epoch = number;

/** A server value and a stored resolver result share the record, under distinct store keys. */
export type StoreRecord = Map<StoreKey, StoreEntry>;
export type StoreEntry = ServerEntry | ResolverEntry;

export type ServerEntry = {
  readonly kind: 'Server';
  readonly value: StoreValue;
  /** The epoch of the `normalize` that last changed `value`. */
  readonly timeUpdated: Epoch;
};

export type ResolverEntry = {
  readonly kind: 'Resolver';
  readonly artifact: ReaderArtifact<ResolverParam, unknown>;
  readonly variables: Variables;
  readonly result: ReadResult<unknown>;
  /** The epoch in which `result` last changed. */
  readonly timeUpdated: Epoch;
  /** The last epoch in which `result` was confirmed current. */
  readonly timeVerified: Epoch;
  /** What the last computation read, in read order. */
  readonly dependencies: ReadonlyArray<EpochDependency>;
};

export type EpochDependency =
  | { readonly kind: 'Record'; readonly record: StoreLink }
  | {
      readonly kind: 'Entry';
      readonly record: StoreLink;
      readonly storeKey: StoreKey;
      /** The entry's `timeUpdated` when read; `null` when the key was absent. */
      readonly observed: Epoch | null;
    };
```

This is pico's model: `ServerEntry` is a source node with `time_updated`, `ResolverEntry` is a derived node with `DerivedNodeRevision` (`time_updated`, `time_verified`) and its dependency list (`crates/pico/src/execute_memoized_function.rs`). Dependencies are forward only.

A write:

1. `normalize` sets `writeEpoch = store.epoch + 1`, writes each unequal value as a new `ServerEntry` with `timeUpdated: writeEpoch`, and sets `store.epoch = writeEpoch` at the first change. `garbageCollect` advances the epoch when it deletes anything.
2. Settlement verifies every retained and observed location. Verifying an entry: if `timeVerified === store.epoch`, return it. Otherwise walk `dependencies` in order; a server dependency changed when its `timeUpdated !== observed`; a resolver dependency is verified first (recursively) and changed when its `timeUpdated !== observed`. At the first changed dependency, recompute; if the result is `===` the old one, keep the old `timeUpdated` (backdating). Set `timeVerified = store.epoch`.

Diamond, `Rex` to `Max` (`Pet:1.name` gets `timeUpdated: 2`):

```text
verify BadgePage         timeVerified 1 != 2: check deps in order
  pet____id___"1"        server, timeUpdated 1 == observed: unchanged
  Badge                  verify Badge
    Monogram             verify Monogram
      Initial            verify Initial
        name             server, 2 != 1: changed
        compute Initial  "M" != "R": timeUpdated 2              (Initial call 1)
      2 != 1: changed
      compute Monogram   reads Initial: timeVerified 2, returned  (Monogram call 1)
    changed: stop checking, compute Badge
      reads Monogram     timeVerified 2, returned
      reads Signature    verify Signature
        Initial          timeVerified 2, returned; 2 != 1: changed
        compute Signature  reads Initial: returned               (Signature call 1)
                                                                 (Badge call 1)
  changed: compute BadgePage                                     (BadgePage call 1)
```

`Rex` to `Ray`: `Initial` computes `"R"`, `===` the old string, and keeps `timeUpdated: 1`; `Monogram`, `Signature`, and `Badge` compare `1 == observed` and do not compute. A write to `Pet:1.tagline`, which nothing in the diamond reads, still advances the epoch, so settlement walks every dependency of `BadgePage`, `Badge`, `Monogram`, `Signature`, and `Initial`, comparing epochs, and computes nothing.

Chain, `Pet:2.name`: verifying `FriendPage` computes `PetName` on `Pet:2`, then `FriendLabel`, `FriendCard`, `FriendPage`; verifying `PetSummary` then finds `PetName` verified in this epoch with a newer `timeUpdated` and computes `PetSummary` without calling `PetName`. The other order gives the same calls. The link move: `FriendLabel`'s `bestFriend` dependency changed, it recomputes reading `PetName` on `Pet:3`, and its new dependency list replaces the old one; `PetName` on `Pet:2` is never verified through `FriendLabel` again (R7).

- Ordering: a depth-first post-order over the current dependencies, by recursion; nothing stores the order. An entry is computed after the entries it reads were verified in this epoch, and at most once per epoch, because a computed entry has `timeVerified === store.epoch`.
- Cost per write that changed anything: `O(R + E)` epoch comparisons, every live edge once, plus the `A'` recomputations. A write that changed nothing costs `O(R)`.
- Memory: a tag and a `timeUpdated` on every one of the `S` server values, read by a resolver or not; two epochs and the forward dependency list per entry.
- Needs from `normalize`: an epoch stamp per changed value. The set `C` that `normalize` knows (R8) is reduced to stamps, and settlement rediscovers it by walking `E`.
- Equality cutoff: backdating, with `===` as the equality. A resolver that returns a new object gets a new `timeUpdated` on every computation, so every entry that reads it computes.
- Failure modes: `garbageCollect` deletes entries, which pico never does (pico.md, Source: `remove` keeps the node and stamps an epoch), so a dependency compares the observed epoch for equality instead of pico's newer-than (`execute_memoized_function.rs:192-206`), and a delete must advance the epoch. A read of an absent key records `observed: null`. Every server-value reader (`check`, `garbageCollect`, `read`) narrows a tagged entry, and a `ResolverEntry` under a server store key is a new broken-invariant throw site.

What epochs buy: correctness without reverse edges. An entry nobody demands is never visited, a dependency list is rebuilt from scratch on each computation so there is no inverse to keep consistent, and deleting a derived node unlinks nothing. In pico's setting this is the right trade: a compiler with a whole project's memos answers a hover by verifying the few it demands. In the runtime, R2 demands every observed entry after every write, and held 3a is lazy for every other entry through `Stale` marks, so pull verification's laziness buys nothing the push lacks; what remains is a walk over every edge under the observed roots on every write, and a stamp on every server value.

### Candidate 3: push along reverse edges

`normalize` already knows `C` (R8). Each entry stores, besides its forward dependencies, the set of entries that read it (`dependents`), and the store keeps the same for server locations. A write follows those reverse edges from `C`, so its cost is bounded by `A`, not by `E`. No epochs: a server value stays an untagged `StoreValue`.

Data structures common to the three orderings below:

```ts
// from libs/isograph-runtime/src/store.ts
export type ResolverEntry = {
  readonly location: EntryLocation;
  readonly artifact: ReaderArtifact<ResolverParam, unknown>;
  readonly variables: Variables;
  result: ReadResult<unknown>;
  /** What the last computation read, in read order. */
  dependencies: ReadonlyArray<Dependency>;
  /** Every entry whose `dependencies` hold a `Resolver` dependency on this one. */
  readonly dependents: Set<ResolverEntry>;
};

/** The entries that read each location of one record. */
export type RecordDependents = {
  /** Entries whose read found the record absent. */
  readonly absent: Set<ResolverEntry>;
  /** Entries that looked up each store key while the record existed. */
  readonly values: Map<StoreKey, Set<ResolverEntry>>;
};
```

`Store` holds `RecordDependents` by entity and record id, outside the records, so a dependency on an absent record has a place to live. The held design keeps the same edges in graph terms instead (change 2, The engine; The engine, the store, and the environment).

Recording and updating dependencies (R7): a computation collects its dependencies in read order, then one function replaces the entry's list and updates the inverse: remove the entry from every old dependency's set, add it to every new one's. MobX does the same with a diff (`bindDependencies`, `packages/mobx/src/core/derivation.ts`: it marks new dependencies, drops old ones no longer read, adds new ones); Reactively keeps the prefix of sources read in the same order and splices the rest (`updateIfNecessary` and `update`, `packages/core/src/core.ts` of `milomg/reactively`). The link move removes `FriendLabel` from `PetName` on `Pet:2`'s `dependents` and adds it to `PetName` on `Pet:3`'s.

What remains is order. Three ways.

#### 3a. Heights

Jane Street's Incremental: "stabilize traverses the DAG in topological order starting at variables that changed", taking "the lowest-height node" from a recompute heap, recomputing it, and enqueueing its parents when its value changed; a node's height is less than its parents' (`src/incremental_intf.ml` of `janestreet/incremental`). Its heights are dynamic: `bind` creates nodes whose height must exceed the bind's left-hand side, and heights are adjusted as the graph changes.

In isograph the height is static, and the compiler writes it into each reader artifact (change 4, Heights). `height(F)` is 0 when `F`'s reader AST has no `Resolver` node, else `1 + max(height(G))` over the client fields `G` its `Resolver` nodes select. The static reader graph has no cycle: every `Resolver` node of `F`'s reader AST selects a field that query generation inlines into `F` (`inlined_fields`, `crates/isograph_compiler/src/merged_selection.rs`), and a cycle of inlined fields is an `InlineCycle` error, which `reader_height` checks before it recurses. So `height(F) > height(G)` for every `G` that `F`'s reader AST selects.

The static height orders the runtime graph whatever the data. An entry of `F` on a record is computed by reading `F`'s reader AST from that record and calling `F`'s resolver with the data read. The resolver receives `data` and `parameters` and no store (open decision 8 covers a closure that reaches one), so every stored result the entry reads is read by a `Resolver` node of `F`'s reader AST: an entry of one of the fields `G`, on whichever record the links lead to. Links choose the record, never the field. So every `Resolver` dependency goes from an entry of height `h` to an entry of height below `h`, for every record and every link value, and the runtime graph has no cycle. This argument is this doc's, not Incremental's. It holds while the reader reads no client pointer (open decision 14).

A write: each entry that read a changed input goes into bucket `height`; buckets are drained lowest first; each entry recomputes, and when its result is unequal (`===`, R4) its dependents go into their buckets; when it is equal nothing is queued. An entry read during a recomputation has a lower height, so its bucket is already drained and it is current. Nothing is marked before it is known to need recomputing: the work of a write is the entries that recompute and the direct dependents of the ones whose result changed.

Diamond, with an observer at `BadgePage`, so every entry is needed. Heights `Initial` 0, `Monogram` 1, `Signature` 1, `Badge` 2, `BadgePage` 3. `Rex` to `Max`:

```text
write   name changed: Initial queued at 0
drain 0 Initial      recompute "M" != "R"               (Initial call 1)
                     queue Monogram, Signature at 1
drain 1 Monogram     recompute "M." != "R."             (Monogram call 1)
                     queue Badge at 2
        Signature    recompute "M " != "R "             (Signature call 1)
                     Badge already queued
drain 2 Badge        recompute                           (Badge call 1)
                     queue BadgePage at 3
drain 3 BadgePage    recompute                           (BadgePage call 1)
```

`Rex` to `Ray`:

```text
write   name changed: Initial queued at 0
drain 0 Initial      recompute "R" === "R": nothing queued   (Initial call 1)
```

The unchanged `Initial` stops propagation at once: `Monogram`, `Signature`, `Badge`, and `BadgePage` are never visited. A changed `Initial` queues `Monogram` and `Signature`, and `Badge` is queued once although both change.

New object, an observer at `ShapePage`, `Rex` to `Ray`: `NameShape` (0) recomputes to a new `{ length: 3 }`, which queues `ShapeLabel` (1); `ShapeLabel` recomputes to `"3 letters"`, `===` the old string, so `ShapePage` (2) is not queued.

Chain, observers at `FriendPage` and `PetSummary`, `Pet:2.name`: `PetName` on `Pet:2` (0); then `FriendLabel` (1) and `PetSummary` (1; it also selects `RecentCheckins`, 0, which is not queued); `FriendCard` (2); `FriendPage` (3). Each once. The link move: `Pet:1.bestFriend` queues `FriendLabel` (1), whose recomputation reads `PetName` on `Pet:3`, height 0; absent, it is computed by the read, and its dependencies move from `PetName` on `Pet:2` to `PetName` on `Pet:3`.

- Cost per write: `O(|A'| + dependents of A')` plus one step per bucket up to the largest height queued. Nothing that will not recompute is visited.
- Failure mode of plain 3a: it recomputes every entry with a changed input, whether or not anything needs it. An entry whose retention was disposed, or that a plain `read` stored, recomputes on every relevant write until `garbageCollect` deletes it.

Held 3a adds `Stale` marks for entries nobody needs (change 4). Each entry counts the observers at its location plus its needed dependents (`neededCount`); it is needed when the count is above 0. An entry whose input changed is queued when it is needed. Otherwise it and its dependents, transitively, are marked `Stale` and not queued; the marking stops at an entry already `Stale`. A read of a `Stale` entry recomputes it, and each `Stale` entry it reads is brought up to date by that read first. Every dependent of an entry nobody needs is itself not needed, so `Stale` marking never reaches a needed entry, and the queue never holds an entry nobody needs.

Diamond, `BadgePage` retained and not observed, `Rex` to `Max`: `Initial` is not needed, so it, `Monogram`, `Signature`, `Badge`, and `BadgePage` are marked `Stale` and no resolver runs in the write. A later `listen` at `BadgePage` reads it: `BadgePage` recomputes, its read of `Badge` recomputes `Badge`, whose read of `Monogram` recomputes `Monogram`, whose read of `Initial` recomputes `Initial`; then `Badge`'s read of `Signature` recomputes `Signature`, reading the `Clean` `Initial`. Each once. `Rex` to `Ray` the same way recomputes all five on that read: a `Stale` entry recomputes when read whether or not what it reads changed (open decision 15).

#### 3b. Mark, then bring up to date (Reactively, MobX)

Reactively's algorithm (`packages/core/src/core.ts` of `milomg/reactively`): each node is `CacheClean`, `CacheCheck` ("might be stale, check parent nodes to decide whether to recompute"), or `CacheDirty`. A write marks "direct children dirty and their deeper descendants check", stopping at nodes already marked. Reading a node runs `updateIfNecessary`: for `CacheCheck`, bring each source up to date in order, stopping at the first that made this node dirty; for `CacheDirty`, recompute; then clean. A recomputation that produced an unequal value marks its children dirty; an equal one leaves them at check, so they clean without recomputing. MobX's derivation states are the same scheme: `UP_TO_DATE_`, `POSSIBLY_STALE_` ("some deep dependency changed, but don't know if shallow dependency changed"), `STALE_`, and `shouldCompute` checks the computed dependencies of a possibly stale derivation in order (`packages/mobx/src/core/derivation.ts`).

Here the entry gains `state: 'Clean' | 'Check' | 'Dirty'`. A write, step by step:

1. While writing, `normalize` marks: for each changed location in `C`, every entry in its dependents set becomes `Dirty`; an entry that was `Clean` marks its dependents `Check`, recursively, stopping at an entry not `Clean`. Marking runs no resolver.
2. Settlement brings the marked roots up to date: the entries that are both marked by this write or an earlier one and the stored result at a retained or observed location. It does not visit an unmarked root. A `Clean` entry returns at once. A `Check` entry brings its `Resolver` dependencies up to date in recorded order and stops at the first after which it is `Dirty`. A `Dirty` entry recomputes; each stored result it reads is brought up to date first, by the read. A recomputed entry whose result is unequal sets its dependents `Dirty`. The entry ends `Clean`.

Diamond, `Rex` to `Max`:

```text
mark   name changed: Initial Dirty; Monogram, Signature, Badge, BadgePage Check
       marked roots: BadgePage (the only retained location; Initial, Monogram, Signature, Badge are marked and are not roots)
settle BadgePage         Check: dependencies in order
  Badge                  Check: dependencies in order
    Monogram             Check: dependencies in order
      Initial            Dirty: recompute "M" != "R"             (Initial call 1)
                         Monogram, Signature set Dirty; Initial Clean
    Monogram             Dirty: stop checking, recompute "M."     (Monogram call 1)
                         Badge set Dirty; Monogram Clean
  Badge                  Dirty: recompute
    reads Monogram       Clean
    reads Signature      Dirty: recompute "M "                   (Signature call 1)
                                                                  (Badge call 1)
                         BadgePage set Dirty
BadgePage                Dirty: recompute                        (BadgePage call 1)
```

`Rex` to `Ray`: `Initial` recomputes `"R"`, `===` the old string, so `Monogram` and `Signature` stay `Check`; `Monogram` finds no dependency made it `Dirty` and becomes `Clean`; so do `Signature`, `Badge`, and `BadgePage`. One call, five entries visited. A write to `Pet:1.tagline` marks nothing in the diamond, so `BadgePage` is not a marked root and settlement does not visit it.

With observers on `Badge` and `Initial` as well as the retained `BadgePage`, the marked roots are `Initial`, `Badge`, and `BadgePage`. Settlement brings each up to date in any order; the first brings up to date every marked entry it reaches by the recursion, and the later ones find those entries `Clean`. The calls are the same as in the trace above.

Why `Check` marks exist. A pure push from the `Dirty` entries (recompute each `Dirty` entry, and only when its result changed, mark its direct dependents `Dirty`) visits fewer entries: it never touches the dependents of an unchanged result, so `Rex` to `Ray` visits `Initial` alone. It needs an order, though. In the diamond, `Badge` reads both `Monogram` and `Signature`; pushed from `Monogram` first, it recomputes with the stale `Signature` and again after `Signature` recomputes, which R3 forbids. Holding `Badge` until every changed input is done is 3a's heights or 3c's pending-input counts. 3b instead marks every transitive dependent `Check` eagerly, while marking, and resolves the order by pulling: an entry brings its dependencies up to date before it decides. The price is that settlement clears the `Check` marks above an unchanged result (`Monogram`, `Signature`, `Badge`, `BadgePage` when `Initial` recomputes to `"R"`), so settlement costs `O(marked)` where the pure push costs `O(recomputed)`. Marking already visits those entries, so the order of growth of a write is the same.

New object, `Rex` to `Ray`:

```text
mark   name changed: NameShape Dirty; ShapeLabel, ShapePage Check
settle ShapePage         Check: dependencies in order
  ShapeLabel             Check: dependencies in order
    NameShape            Dirty: recompute { length: 3 }          (NameShape call 1)
                         a new object, !== the old one: ShapeLabel set Dirty
  ShapeLabel             Dirty: recompute "3 letters"            (ShapeLabel call 1)
                         === the old string: ShapePage stays Check
ShapePage                no dependency made it Dirty: Clean, no call
```

Propagation continues through `NameShape` although its content is unchanged, because `===` cannot see that, and stops at `ShapeLabel`, whose result is a primitive.

Chain, `Pet:2.name`: marking makes `PetName` on `Pet:2` `Dirty` and `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` `Check`. Settling `FriendPage` first recomputes `PetName` (a new object, so `FriendLabel` and `PetSummary` become `Dirty`), then `FriendLabel`, `FriendCard`, `FriendPage`; settling `PetSummary` recomputes it reading the `Clean` `PetName`. The other order gives the same calls. The link move: `Pet:1.bestFriend` marks `FriendLabel` `Dirty`; it recomputes reading `PetName` on `Pet:3` (computed now, since none is stored), and its dependencies move.

- Ordering: an entry recomputes only after every stored result it reads was brought up to date, because the only ways it reaches one are the `Check` walk and the read during recomputation, and both bring it up to date first. It recomputes at most once per write: after recomputing it is `Clean`, and only a dependency's recomputation can make it `Dirty` again, and every dependency was brought up to date before it.
- Laziness: an entry no settled location reaches keeps its mark and is not recomputed; a later `read` brings it up to date. Marking stops at marked entries, so repeated writes cost `O(1)` per already marked entry.
- Marked roots suffice. Every dependent of an entry that is not `Clean` is not `Clean` (an entry that leaves `Clean` marks its dependents, and a new dependent is added only by a computation, which brought the entry up to date first), so a root that reaches a marked entry is itself marked. Settling the marked roots therefore settles every root that reaches anything this write changed, and an unmarked root costs nothing.
- Cost per write: marking `O(|A| + edges within A)`; settlement `O(marked roots)` to start, plus `O(|A| + edges within A)` for the walk; `A'` recomputations. No term grows with the number of retained or observed locations `R`.
- Failure mode it shares with 3a and 3c: the inverse must equal the forward lists exactly. A missed removal keeps a dead entry in a set (memory, and a mark that does nothing); a missed addition loses a mark and leaves a stale result with no error. One function writing both sides is the guard.

#### 3c. Pending-input counts

Kahn's algorithm over `A`: a first pass over the reverse edges from `C` counts, for each entry of `A`, how many of its inputs are in `A`; a second pass recomputes an entry when its count reaches zero and at least one input changed, then decrements its dependents' counts. The order is topological without stored heights. The counting pass visits all of `A`, as 3b's marking does, and a recomputation that reads an entry of `A` its old dependencies did not count (the link move into a region the write also affected) must bring that entry up to date on demand, which is 3b's pull. It is 3b with an extra counter and no new property.

### Candidate 4: signals with version numbers (Preact)

Preact Signals (`packages/core/src/index.ts` of `preactjs/signals`): a global `globalVersion` incremented on every effective write; a per-signal `_version` incremented only when the value changes (`value !== this._value`); per edge a `Node` with `_source`, `_target`, and `_version`, the source version the target last observed. A write notifies targets, which set `OUTDATED | NOTIFIED` and propagate without recomputing; effects are queued and run at the end of the batch. A computed's `_refresh` returns at once when it is subscribed and not `OUTDATED`, or when its `_globalVersion` equals `globalVersion`; otherwise `needsToRecompute` walks its sources in order and recomputes when a source's `_version` differs from the edge's, after refreshing that source. A recomputation that returns an identical value does not increment `_version` (equality cutoff); as in every candidate here, a computed that returns a new object always propagates. A computed subscribes to its sources only while it is itself subscribed (`TRACKING`), so an unobserved computed holds no reverse edges and the JavaScript garbage collector can free it.

This is 2 and 3b combined. Subscribed nodes get a push mark (`OUTDATED`) on every transitive target, one flag where 3b distinguishes `Dirty` from `Check`, and per-edge versions then decide which marked node recomputes. Unsubscribed nodes get the epoch fast path (`globalVersion`, pico's `time_verified`) and version comparison alone, which is candidate 2 with a counter per node in place of epochs. In the runtime every stored entry is owned by the store and deleted by `garbageCollect`, not by the JavaScript collector, so no entry needs to be unsubscribed to be freed, and the version machinery for unsubscribed nodes has no user. With every entry subscribed, Preact is 3b with one flag plus a version per node and per edge, where 3b has a three-valued state per entry. Either detects "a dependency recomputed to an unequal value"; 3b's needs no field on the edge.

SolidJS's computations use a similar pair of stale and pending states; that is from memory, not checked against its source.

### Candidate 5: full re-run of every retained tree

The simplest eager baseline. No dependencies stored at all. After every write, settlement recomputes every retained and observed root from scratch, with a `Map` from location to result for the duration of that settlement, so an entry reached twice is computed once. Each new result is compared by `===` with the previous settlement's for observers.

- Ordering: correct by recursion; every read sees the finished write.
- Cost per write: every live resolver, `O(L)` resolver calls, whatever changed.
- Memory: one result per root, and the per-settlement map.
- Equality cutoff: none below the root. Every resolver runs on every write, and a resolver returning a new object (fixture resolvers return `data`) makes every observer fire on every write.
- Against the requirements: meets R1, R2, R3, R5, R6, R7. It has none of R4's optimization below the root.

### Comparison

- R1: 1 fails (render calls the resolver); 2, 3a, 3b, 3c, 4, 5 meet it.
- R2: 1 fails; the rest meet it.
- R3: 1 fails "nothing twice"; 2, 3a, 3b, 3c, 4, 5 meet it.
- R4, an optimization: 2, 3a, 3b, 3c, 4 stop propagation at an entry whose result is `===` its previous one (a primitive, or a stored or reused object), and propagate every new object. Held 3a stops it on the push and not on a read of a `Stale` entry (open decision 15). 1 compares only at the subscription; 5 only at the root. No candidate compares resolver outputs structurally, because that comparison is not well-defined on arbitrary JavaScript values.
- Structural equality on inputs: the one place it is well-defined is a resolver's inputs, which are store data shaped by the reader AST. 1 does it at the subscription with upstream's `mergeObjectsUsingReaderAst`, which also keeps the old object when equal and so gives stable identity. 2, 3a, 3b, 3c, 4 get the server half from `normalize`'s comparison by AST shape (change 1: an unchanged write changes no input, so nothing is queued, marked, or stamped; a leaf whose value is a JSON object or list always counts as changed) and compare resolver inputs by `===`. A snapshot whose identity must stay stable across a recomputation with equal data needs upstream's merge applied to the data read along the reader AST; it does not need structural equality on outputs.
- R5 and R7: every candidate meets them.
- R10: held 3a meets the lazy half from day one: an entry nobody needs is marked `Stale` and recomputed when read. Deferral needs no new state: a deferred write marks as if nothing were needed, so every entry its changes reach becomes `Stale`, and the observed entries the marking reaches are recorded for the deferred pass, which brings them up to date by the read path and notifies their observers (Follow-ups 9). 3b allows it: marking already runs in `normalize` apart from settlement, an entry that settlement skips keeps its mark, `settle` iterates the marked roots and a scheduler chooses which of them each pass settles, and the observed count is the observers at a location plus the observed `dependents`. 2 allows it: nothing is verified until demanded, and a deferred pass is a later verification of fewer roots; each pass still walks every live edge under its roots. 4 allows it: Preact keeps unsubscribed computeds lazy and queues effects to the end of a batch, so priority goes on that queue. Plain 3a and 3c fail: their propagation recomputes every affected entry as it goes, so skipping an unobserved entry leaves it stale with no record of it. 5 allows it per root at full cost per pass. 1 has nothing stored to defer; scheduling exists only in React's re-render.
- R6: 1 fails; the rest meet it.
- R8, what each does with `C`: 1 needs it at record granularity; 2 turns it into stamps and rediscovers it by walking `E`; 3a, 3b, 3c, 4 start from it; 5 ignores it.
- Per write with a small change on a large screen: 1 `O(overlapping subscription trees)`; 2 `O(R + E)`; held 3a the `A'` recomputations among needed entries plus the dependents of those whose result changed, and one `Stale` mark per entry nobody needs that the write reaches and that is not `Stale` already; 3b and 3c `O(A)` (3b starts settlement from the marked roots); 5 `O(L)` resolver calls.
- Memory beyond the results: 2 a tag and epoch on all `S` server values, two epochs and forward edges per entry; 3a, 3b, 3c, 4 forward and reverse edges, plus a dependents index for the server locations live entries read (only those, not all of `S`); held 3a a state and a needed count per entry, the bucket queue, and a height per artifact, written by the compiler; 3b a state per entry; 4 a version per edge.
- Machinery (R9): 2 has the fewest invariants (no inverse to keep). Held 3a adds the inverse, a three-valued state, the needed count, and the bucket queue; the runtime never computes or adjusts a height. 3b adds the inverse, a three-valued state, mark, and bring-up-to-date. 3c and 4 add 3b's pieces plus counters or versions.
- Unreached entries (a disposed retention before `garbageCollect`, a plain `read` outside any retention, a retained query nobody observes): 2, held 3a, 3b, 4 do not recompute them until read; held 3a marks each `Stale` once and later writes stop at it. Plain 3a and 3c recompute them.
- A read of an entry nobody needs, after a write that reached it: 3b brings its dependencies up to date in order and skips the recomputation when none changed; held 3a recomputes every `Stale` entry the read reaches (open decision 15).

For a sense of scale, from no measurement: a screen with 2,000 live entries averaging 5 dependencies is 10,000 epoch comparisons, each two or three `Map` lookups, for every write under 2, including a write of one field that one entry reads; under held 3a the same write recomputes that entry, and visits its dependents only if its result changed.

### Held: 3a, height-ordered push with compile-time heights and `Stale` marks

The eager requirement removes the one thing pico's epochs are for: pull verification exists so that undemanded memos are never visited, and R2 demands every observed entry after every write, so 2 visits every edge under the observed roots on every write while `normalize` already holds the exact set of changed locations and discards it into stamps. Push from that set bounds each write by what it affects.

Among the push orderings, held 3a does the least work. A write recomputes the needed entries that read a changed input; an entry whose result changed queues its direct dependents, and one whose result is `===` its previous one queues nothing. Nothing is marked in advance: there are no `Check` marks to set while writing and none to clear above an unchanged result, which 3b pays for with a walk over `A` (Candidate 3b, Why `Check` marks exist). In the diamond with `Rex` to `Ray`, held 3a visits `Initial`; 3b marks and then clears `Monogram`, `Signature`, `Badge`, and `BadgePage`. With no cutoff (every resolver returns a new object), `A'` is `A` and the two make the same recomputations; held 3a still skips 3b's marking walk. Its ordering state is one bucket queue, an array indexed by height, drained lowest first.

The classic objection to 3a is the cost of maintaining heights: Incremental keeps heights consistent as `bind` changes the graph, adjusting a node's height and its parents' when a new edge would point upward. That cost does not exist here. The compiler writes each reader artifact's height, and the runtime graph only gains edges from an entry to entries of fields its reader AST selects, which are lower by construction (Candidate 3a). A dependency that moves with a link (`PetName` on `Pet:2` to `PetName` on `Pet:3`) keeps its height. The runtime never computes, compares, or adjusts a height.

The earlier objection to 3a here was its lack of a lazy mode: it recomputed entries no root reaches until `garbageCollect`, and R10's laziness needed 3b's marks. `Stale` marks for entries nobody needs remove it from day one. A needed entry is never `Stale`, an entry nobody needs is never queued, and the needed count that separates them is the observed count R10 asks for. Laziness costs one `Stale` mark per entry nobody needs that a write reaches; marking stops at an entry already `Stale`, so a second write to the same input marks nothing new.

3b is the considered alternative (open decision 2 (b)). It keeps the cutoff on a read of an entry nobody needs, which held 3a gives up (open decision 15), and it needs no height. It marks every transitive dependent of a changed input on every write and walks the marked entries again to settle, where held 3a visits the entries that recompute and the dependents of those that changed. 3c and 4 add counters or versions to 3b without a new property; 5 meets the requirements and runs every live resolver on every write; 1 fails R1, R2, R3, and R6.

The cost held 3a takes on is the inverse edge set: memory per edge and an invariant (the dependents sets equal the inverse of the dependency lists) whose violation is a silently stale result. That is contained by making one function, `setDependencies`, the only writer of either side. The inverse also pays for itself elsewhere: it carries the needed counts, it is the reference count constructors and destructors need (Follow-ups 1, open decision 11), and it is the index open decision 6 (b) needs. Removing epochs also removes the tag and stamp from every server value, so records stay store.md's untagged maps and `check`, `garbageCollect`'s mark, and the reader's server reads do not change.

Held 3a's bound does not depend on equality cutoff, which stays a `===` optimization (R4): only needed entries that read a changed input recompute, and `normalize`'s comparison by AST shape (change 1) keeps an unchanged server value out of `C`. When every resolver returns a new object, held 3a recomputes exactly the needed part of `A`; 5 recomputes all of `L`, and 2 still compares every edge under the observed roots.

### What depends on this choice

Written for held 3a, and changed if another candidate is chosen: Steps; change 2 except The resolver store key and Results, equality, and a resolver that throws; Ordering; change 3's Computing the new root, Unobserved results are lazy, and Garbage collection; change 4 except `listen`, `snapshot`, and `Observed`; the engine's functions in changes 2 to 4; Values with lifecycles' Replacement and release; the mentions of `dependents` and marking in Follow-ups 1, 8, and 9; and open decisions 2, 3, 4, 6, 8, 11, 14, 15, and 16. Each of those sections in Steps and changes 2 to 4 opens with a line saying so.

Independent of it: Two levels, Step A, End-user experience, change 1, The resolver store key with `ReaderArtifact.selectableName`, Results, equality, and a resolver that throws, change 3's `normalizeData` signature, `Retention`, and Resolver markers, change 4's `listen`, `snapshot`, and `Observed`, The engine, the store, and the environment, Other node kinds, Later, the rest of Follow-ups, and open decisions 1, 5, 7, 9, 10, 12, 13, and 17.

## The engine, the store, and the environment

Settled (open decision 12). Changes 2 to 4 are written for it.

```text
environment  IsographEnvironment { store, graph, retainedQueries, networkFunction, networkRequestCache, subscriptions }
             normalizeData, garbageCollectEnvironment, readStoredResult, dispose, listen, snapshot
store        Store = Map<EntityName, Map<RecordId, StoreRecord>>
             normalize, check, garbageCollect, read, readButDoNotEvaluate
graph        RecomputationGraph<ResolverResult<unknown>> { nodes, inputDependents, owned, queue, observers, valuesEqual }
             track, markChanged, deleteInput, retainRoot, collect, settle, observe, valueOf, notify
```

- The store is store.md's nested map, unchanged. Its functions take the `Store` and import nothing from the graph or the environment. `normalize` returns the changes it made (change 1), `garbageCollect` returns the records it deleted (change 2), and the reader's walk reports each read to the `ReadHooks` its caller passes (change 2).
- The engine, `graph.ts`, knows two kinds of id, `InputId` and `NodeId`, both opaque and both minted by its caller (change 2). It imports nothing from the store, the reader, or the environment. Its state is one value, `RecomputationGraph`.
- The environment (network-and-subscriptions.md, The environment) holds the store and the graph as two fields, beside the retained queries, the network function, the request cache, and the subscriptions, as upstream's `IsographEnvironment` holds `store` beside its other state (`IsographEnvironment.ts:73-93`). Its functions are the only ones that read or write both: `normalizeData` passes `normalize`'s changes to `markChanged`, `garbageCollectEnvironment` passes each record `garbageCollect` deleted to `deleteInput`, and `readStoredResult` reads a resolver through `track` with hooks that report each read to the computation's `ComputeContext`.
- `inputs.ts` maps store locations to ids (`recordInput`, `valueInput`, `resolverNode`, `changeInput`). It imports the store's types and the graph's id types. The environment's functions and the hooks are its only callers.

Upstream draws the line one level higher: its `normalizeData` takes the environment and returns the records whose data changed (`EncounteredIds`, `cache.ts:77-85`), and `makeNetworkRequest` passes them to `callSubscriptions` (`makeNetworkRequest.ts:242-271`). Here `normalize` is the store's and returns the (record, store key) pairs it changed, and `normalizeData` is the environment's and passes them to the graph.

What this keeps:

- store.md's `Store` type and every v1 store function: `normalize`, `check`, `garbageCollect`, `read`, and `readButDoNotEvaluate` keep their parameters, and `check`, `read`, and `readButDoNotEvaluate` keep their behavior. `normalize` and `garbageCollect` gain return values that their v1 callers ignore.
- store.md's and reader-ast.md's tests, unchanged: they build `new Map()`, call the store functions, and compare the store with `toEqual(storeOf({ .. }))` and `toEqual(new Map())`. The tests of changes 1 to 4 compare `environment.store` the same way.
- The store-only functions stay store-only (network-and-subscriptions.md, Why the store-only functions stay store-only).
- Upstream's environment shape: the store is one field of the environment, and the state that is not server data lives in the environment's other fields.

What this costs:

- Every function that needs both the store and the graph takes the environment: `normalizeData`, `garbageCollectEnvironment`, `readStoredResult`, `dispose`, `listen`, and `snapshot`. A test of memoization builds an environment with a network function it never calls (`deferredNetwork().networkFunction`, `test/fixtures.ts`).
- `normalize` returns a list of changes instead of marking during the write: one `StoreChange` per changed value and per created record, held until it returns. A throw part-way through must still deliver the changes made before it (open decision 17).
- A `ReadHooks` parameter on every function of the reader's walk, and one hooks object per computation.
- A caller that writes `environment.store` with `normalize` directly, and not through `normalizeData`, leaves the nodes that read what it changed unmarked, and they return their old results until another write reaches them. The package index keeps exporting `normalize` for v1's callers. This is network-and-subscriptions.md's rule for `callSubscriptions` (Why the store-only functions stay store-only): a write through the environment is `normalizeData`.

### Why a store-agnostic engine

The alternative (open decision 12 (b)) writes the recomputation graph in the store's shape:

- Entries keyed by `EntityName`, record id, and resolver store key, each carrying its `StoreLink` and `StoreKey`.
- The inverse of server dependencies keyed by record, with one set for a record read while absent and one set per store key.
- Dependencies that name `StoreLink`s and store keys.
- A change type (`RecordCreated | RecordDeleted | ValueChanged`) whose dependents are found through that record grouping: a deleted record marks every set of the record and the dependents of every entry stored on it.
- Marking called with changes to the one base map, on the assumption (R8) that `normalize` is the only writer.

A layered store breaks the last two. With an optimistic layer over the base layer (upstream's `StoreLayer` chain, `optimisticProxy.ts:24-98`), a value's effective value is the topmost layer's: a base write under a shadowing layer changes no effective value and marks entries for nothing, and removing a layer changes effective values with no `normalize` call, so nothing marks. Upstream handles removal by comparing the old and new merged data and calling `callSubscriptions` with the records that differ (`revertOptimisticStoreLayerAndMaybeReplace`, `optimisticProxy.ts:393-493`; `compareData`, `:506-544`). That obligation exists whatever the engine looks like; the coupling question is whether the engine's types and functions name records, store keys, and layers. Under the held design they name none of them.

### What the environment must report

Two obligations, and nothing else couples the engine to the store. The store's functions meet neither alone: `normalize` and `garbageCollect` return what they changed and the walk reports through hooks, and the environment's functions forward both to the graph.

- Report every read. Each read a computation makes goes through the walk, whose hooks call `readInput` with `recordInput` and `valueInput` of the location read. For a layered store this is the effective location: one input per (record, store key), whichever layer answered.
- Report every effective change. After an operation that changes what a read would return, the environment calls `markChanged` with every input whose effective value changed, and `deleteInput` for each record whose effective presence went from present to absent (from absent to present is `markChanged` of its `recordInput`). It may report more: an input reported but unchanged costs a recomputation of each needed reader, which cuts off by `===` or recomputes to an equal result, and a `Stale` mark on each other reader. It must not report fewer: an unreported change is a stale result with no error. Settlement runs after the operation and before notification.

For the layered store of Later's optimistic writes:

- A write to the topmost layer that holds a key reports it when `isUnchanged` (change 1) rejects it against the previous effective value. A write to a layer that a higher layer shadows at that key reports nothing.
- Adding a layer reports each key it holds whose value differs from the previous effective value, and each record it creates.
- Removing a layer reports each key the removed layer, or a layer re-executed above it, wrote whose effective value differs after the removal, and `deleteInput` for each record that existed only in the removed layer. This is upstream's `compareData` (`optimisticProxy.ts:506-544`) at store key granularity instead of record granularity, comparing by change 1's AST shape: each layer keeps the `WrittenValue` of every key it wrote, so a list of scalars compares element by element. Without that, `===` reports every list of scalars, which is correct and costs recomputation.

### Lifetime still follows records

A node's `owner` is the `recordInput` of the record it was computed on. `deleteInput(graph, recordInput(record))` invalidates every reader of the record (every reader of any key on it read its presence), invalidates the dependents of every node the record owns, and disposes those nodes. A store-coupled engine does the same through its record grouping; here the engine does it without knowing what a record is. An invalidated dependent recomputes, by the queue when it is needed or by the next read when it is `Stale`, finds the record absent, and records only `recordInput`, with no node (change 2, the store's side). No computation reaches a disposed node: every one of its dependents was invalidated, a recomputation reads the record's presence instead, and a `Stale` node recomputes without walking its old dependencies.

components-in-userland.md says a closure field's wrapper is "stored on the record, and freed with it". "Stored on the record" does not hold literally: nodes live in `environment.graph`, beside the store. It holds as "keyed by the record": the wrapper is the value of the node `resolverNode(record, storeKey)` in `graph.nodes`. "Freed with the record" holds through `owner`: deleting the record calls `deleteInput` with its `recordInput`, which disposes the wrapper's node. The wrapper's computation reads only the record's presence (its `createWrapper` receives no data), so no write recomputes it while the record lives.

### Costs of the ids

- An id per read. Each server read in a computation builds two strings (`recordInput`, `valueInput`) and each resolver read one (`resolverNode`; `resolverStoreKey` is built in any design), and `setDependencies` does one `Map` lookup per id. A store-coupled engine does the same reads as three nested `Map` lookups with no string built. The order of growth is the same; the string building is the extra, and it is not measured. An interned object per (entity, id, store key) avoids the string but needs an intern table, whose entries need the same lifetime rule as `inputDependents` (deleted when no node reads them), so it is `inputDependents` again under a second name; the string is held.
- A presence edge per (node, record read): one more `inputDependents` set member than a store-coupled engine keeps for a node that read a present record.
- A map lookup instead of a field: a node does not sit in a per-record map, so deleting a record finds its nodes through `owned`, one more map kept by node creation and disposal.
- The casts in the id functions.

What the opaque ids buy: the engine has no import from the store, so a layered store, another record layout, or an input that is not server data (a client-only value, a clock) is an `InputId` and two calls. R8 becomes the environment's obligation above, stated once, instead of a property of `normalize`. The engine is testable with literal ids.

## Values with lifecycles

Status: design analysis, held options, not decided. Nothing in changes 1 to 4 produces a value with a lifecycle; this lands with the first feature that does (Follow-ups 1, 2, 4), and changes 1 to 4 must not preclude it.

A stored result can be a handle: a value with a constructor and a destructor, reference-counted, the `ItemCleanupPair<T>` shape of `@isograph/disposable-types` (`isograph-disposable-types/index.ts:2`). Examples: a client pointer's follow-up query (Follow-ups 2), a handle a retain returns (Follow-ups 4), a closure component whose wrapper holds a subscription. Held 3a handles them if the engine owns each node's value lifecycle: a node acquires its value when it computes, and releases it when the value is replaced or the node is disposed.

### The hook

A node's computation is created with a kind; the engine cannot tell an `ItemCleanupPair` from a plain array value at run time, so the kind comes from compiler-written data (the artifact's kind), not from the returned value.

```ts
// from libs/isograph-runtime/src/graph.ts
export type Computation<TValue> = PlainComputation<TValue> | DisposableComputation<TValue>;

/** A computation whose value holds nothing to release. */
export type PlainComputation<TValue> = {
  readonly kind: 'Plain';
  compute(context: ComputeContext<TValue>): TValue;
};

/** A computation whose value can hold a resource. */
export type DisposableComputation<TValue> = {
  readonly kind: 'Disposable';
  compute(context: ComputeContext<TValue>): Constructed<TValue>;
};

/** `Constructed`: the constructor ran; the engine calls the cleanup exactly once.
 * `NotConstructed`: nothing was acquired (the data read was `MissingData`, or the constructor threw). */
export type Constructed<TValue> =
  | { readonly kind: 'Constructed'; readonly pair: ItemCleanupPair<TValue> }
  | { readonly kind: 'NotConstructed'; readonly value: TValue };

/** What a node holds. */
export type Holding<TValue> =
  | { readonly kind: 'Value'; readonly value: TValue }
  | { readonly kind: 'Resource'; readonly pair: ItemCleanupPair<TValue> };
```

These types replace change 2's `Computation<TValue>`. `GraphNode.value` becomes `holding: Holding<TValue>`; the node's value is `holding.value` or `holding.pair[0]`. `RecomputationGraph` gains `readonly pendingReleases: CleanupFn[]`, the cleanups of replaced and disposed values not yet run, and:

```ts
// from libs/isograph-runtime/src/graph.ts
/** Runs and empties `pendingReleases` in order; then rethrows one error, or an `AggregateError` of several. */
export function flushReleases<TValue>(graph: RecomputationGraph<TValue>): void;
```

For the runtime, a resolver whose artifact is disposable reads its data as any resolver does; on `Ok` it calls the constructor with the data and returns `Constructed` with `[ok(handle), cleanup]`; on `MissingData` it returns `NotConstructed`.

Contract on a constructor: a constructor that can return a `===` value on two calls (a handle deduplicated by key) reference-counts it, so each pair's cleanup releases only its own retain. Follow-ups 5's registry is that count; each holding node holds one retain.

### Replacement and release

`recompute` of a `Disposable` node, after `setDependencies`:

1. The computation runs, so the new value is constructed before anything is released. A shared resource reached again (the same follow-up for the same target) gets a second retain, not a teardown and a rebuild.
2. When the new value is `valuesEqual` to the old (`===` for `Ok`), the node keeps its old holding, and the new pair's cleanup runs at once: it drops the second retain while the first still holds the resource, and no observer sees a change. Dependents are not marked: R4's cutoff works for handles deduplicated by key.
3. Otherwise the node holds the new holding, the old pair's cleanup is pushed to `pendingReleases`, and the dependents are invalidated, as change 4's `recompute` does.

Disposing a node (`collect`, `deleteInput`) pushes its pair's cleanup to `pendingReleases`. `normalizeData` runs `settle`, the new root's `retainRoot`, `notify`, then `flushReleases`; `garbageCollectEnvironment` runs `settle`, `notify`, and `flushReleases` at its end.

Ordering guarantees:

- A constructor runs inside its node's computation, so after every input it reads is current (R3), and at most once per node per pass.
- A new value is constructed before the old one is released.
- No value is released while a node holds it.
- A replaced or disposed value is released after every observer of that settlement was notified, so a `snapshot` in a callback never returns a released value, and each cleanup runs exactly once, in the order the values were replaced or disposed.
- Under React, release after notification is not late enough: a concurrent render can still be on screen showing the old value until the commit of the re-render. The engine's retain is one retain among others. The React package takes its own retain on the handle it renders: a temporary retain at render, converted to a permanent retain at commit, and released when the component unmounts or commits a newer value (network-and-subscriptions.md, the two-phase lifetime; `CacheItem.temporaryRetain`, `CacheItem.ts:195-256`, and `permanentRetainIfNotDisposed`, `:114-193`). The handle is destroyed when the engine's release and React's both happened, so a committed tree never holds a destroyed handle. The handle the constructor returns is therefore retainable by others: a `CacheItem`, or an entry of a registry that counts retains as network-and-subscriptions.md change 7's `RequestEntry` does.

Garbage collection: `collect` and `deleteInput` dispose nodes, which releases their values. Under held open decision 11 (a), a node no root reaches keeps its value until the next `garbageCollect`, and under held 3a it is `Stale` or `Clean` and not recomputed until read: a pointer node whose target moved keeps its old follow-up alive until collection. That is Follow-ups 7's batched destruction; 11 (b) would release at the release of the last root instead.

### Failures

A constructor that throws. Options:

- (a) The node's value becomes `ResolverThrew` (open decision 5 (a)), held as `NotConstructed`; the old value is released through `pendingReleases`, as any replacement; the dependents are invalidated. Held. Reason: `setDependencies` has already recorded the new computation's reads, so a node that kept its old value would hold a value that no recorded input explains, and it would stay stale until one of the new inputs changes.
- (b) Keep the old holding and value, and record the error elsewhere. Cost: the stale value above, and a second place an error lives.
- (c) Propagate out of `normalize`. Cost: open decision 5 (b)'s, every needed entry left unsettled.

A destructor that throws. Options:

- (a) `flushReleases` runs every remaining cleanup, then rethrows one error as is or several as an `AggregateError`, open decision 9's rule; `normalize` combines them with `notify`'s errors into one `AggregateError` when both threw. The node is unaffected: it dropped the value before the cleanup ran. Held.
- (b) Stop at the first throw. Cost: later resources leak.

A cleanup is never called twice by the engine; a handle's own second-call throw (`CacheItem.ts:125-130`) is not reachable from it.

### Relation to Follow-ups 1 to 7

- 1, constructors and destructors: the `Disposable` computation is the constructor, at the node's first computation; the cleanup is the destructor, run when the node's value is replaced or the node is disposed. "The last handle referencing that entry" is the handle's own count: the engine holds one retain per holding node, React holds its own.
- 2, client pointer and follow-up query: the pointer's node returns `[ok(handle), cleanup]` for the follow-up of its target. A recomputation that returns another id constructs the new follow-up and then releases the old one after notification; the parent retention's `dispose` releases it through `collect`.
- 3, a pointer whose target it read: no lifecycle; it removes the follow-up instead.
- 4, handles: the node's value is the handle; `snapshot` returns it, a lookup, readable during render.
- 5, deduplication: under R6, two retained queries that reach the same pointer on the same record already share one node, so the key Follow-ups 5 names (the record, the selectable, the variables) is the node's id and needs no registry. Sharing across different nodes (two pointers that return the same target) needs a key of the follow-up itself: the target link, the target fragment's selectable, and its variables. A recomputation to a new target constructs under the new key.
- 6, a destructor and an in-flight follow-up: the deferred release calls the follow-up's cleanup, which aborts its request through the `AbortSignal` the network function accepts.
- 7, deferral: `flushReleases` is the batch. It can be handed to the caller-supplied scheduler instead of run at the end of `normalize`, and a handle released and retained again before the batch runs is not destroyed, because the second retain reached it first. A constructor that defers its own setup returns the handle at once and schedules the rest; the engine sees only the pair.

## Steps

Depends on the held architecture (3a with compile-time heights and `Stale` marks) in changes 2 to 4.

Each change is one commit, in order; change 2 is two. Step A is optional and independent of 1 to 4.

- Step A. Re-read every subscription after each write, upstream's mechanism, with v1's `read`. No memoization.
- 1. Leaf selections in the normalization AST record their list depth, and `normalize` skips an unchanged value and returns the changes it made (prefactor). `Store` stays store.md's nested map.
- 2a. `graph.ts`, the engine's lazy half, over opaque `InputId`s and `NodeId`s, importing nothing from the store, the reader, or the environment: nodes, `setDependencies`, `markChanged` marking `Stale`, `bringUpToDate`, `track`, `deleteInput`. Its tests use literal ids.
- 2b. Stored resolver results. The environment gains `graph`. The reader's walk takes `ReadHooks`, and `readStoredResult` reads a resolver through the graph, storing each resolver's result with its dependencies and their inverse. `normalizeData` marks `Stale` every node that read what `normalize` changed and every node that reads those, transitively, stopping at a node already `Stale`; `garbageCollectEnvironment` does the same for each record `garbageCollect` deleted and disposes the nodes the record owned. A read of a `Stale` node recomputes it, each node it reads brought up to date by that read first. There are no observers, so nothing is needed, nothing is queued, and no height exists.
- 3. Level 1 retention. `normalizeData` takes the entrypoint, computes its root result and every result that result reaches by a read of the root, holds the root in the graph, and adds its `Retention` to `environment.retainedQueries`. `garbageCollectEnvironment` disposes the nodes no held root reaches. A later write marks a retained query's affected nodes `Stale`.
- 4. Level 2 and the height-ordered push. The compiler writes each reader artifact's `height` (Rust and TypeScript). Nodes gain `height`, `neededCount`, and the state `Queued`; the graph gains the bucket queue and the observers. `listen` brings its node up to date and adds to the needed counts; `snapshot` is a lookup. A write queues each needed node whose input changed and marks the others `Stale`; `settle` drains the queue lowest height first, queuing the direct dependents of each node whose result changed. Observers are notified once, after the pass. `garbageCollectEnvironment` settles and notifies after its deletions.

Values with lifecycles (open decision 13) adds no change here: it is a later change, with the first feature that produces a disposable value, and adds `Disposable` computations, `holding`, `pendingReleases`, and `flushReleases` to the engine.

## Step A: re-read every subscription after each write

A step close to upstream that lets a React demo exist before the memoized runtime. It is not the final design: it is read-plus-subscribe, with the read on every write.

```ts
// from libs/isograph-runtime/src/subscribe.ts
export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The data the callback last received the result of. */
  delivered: ReadResult<SelectionSetData>;
  /** A method signature, so a `Subscription<T>` callback is assignable (reader-ast.md, Why `resolver` is a method signature). */
  callback(result: ReadResult<unknown>): void;
};

/** Adds a subscription to `subscriptions`, with `delivered` read now. Returns the function that removes it. */
export function subscribe<TOutput>(
  store: Store,
  subscriptions: Set<Subscription>,
  artifact: ReaderArtifact<ResolverParam, TOutput>,
  root: StoreLink,
  variables: Variables,
  callback: (result: ReadResult<TOutput>) => void,
): () => void;

/** Re-reads every subscription's data and calls its callback with the resolver's result when the data changed. */
export function callSubscriptions(
  store: Store,
  subscriptions: ReadonlySet<Subscription>,
): void;
```

- The caller holds the `Set`, as v1's caller holds the retained list (store.md, `garbageCollect`); upstream holds it on the environment (`IsographEnvironment.ts:80`). The write site calls `callSubscriptions` after `normalize`, as upstream's write sites do (`makeNetworkRequest.ts:271`, `:542`, `writeData.ts:55`).
- `callSubscriptions` reads each subscription's data with v1's `readSelectionSet` (exported from `read.ts`), compares it with `delivered` by upstream's rule (`mergeObjectsUsingReaderAst`, `areEqualWithDeepComparison.ts:55-121`: store data along the reader AST, scalars by `===`, lists item by item, linked selections recursively; nested `Resolver` values by `===`, `:84-95`), and when unequal stores it and calls the callback with `ok(artifact.resolver({ data, parameters: variables }))`, or with the `MissingData`.
- No overlap filter: v1's `normalize` records no `EncounteredIds` (store.md, `normalize`), so every subscription is re-read, where upstream skips the ones whose records do not overlap (`subscribe.ts:74-79`).
- Order does not matter for correctness: every re-read runs after the write finished and reads the whole store, so nothing reads a stale input. Nothing is shared: a resolver two subscriptions reach runs once per subscription per write.

Costs: every write re-reads every subscription and re-runs every nested resolver under it; a nested resolver that returns a new object makes its parent's data unequal on every write (upstream has the same, `areEqualWithDeepComparison.ts:84-95`), so its callback fires on every write. With the callback being React's `setState`, a demo renders correctly and slowly. Changes 1 to 4 replace it; nothing in them builds on it.

## End-user experience

```ts
// from libs/isograph-runtime/test/level2.test.ts
const environment = createIsographEnvironment(new Map(), deferredNetwork().networkFunction);
const retention = normalizeData(environment, petSummaryEntrypoint, response, { id: '1' });
// Every resolver PetSummary reaches ran once, inside normalizeData.
const location = retention.location; // Query.__ROOT, 'PetSummary____id___"1"'
const onChange = vi.fn();
const stop = listen(environment, location, onChange);
// listen ran no resolver: nothing changed since normalizeData computed the results.
expect(snapshot(environment, location)).toEqual({ kind: 'Ok', value: /* .. */ });

normalizeData(environment, petSummaryEntrypoint, sameResponse, { id: '1' });
// No resolver ran, onChange was not called.

normalizeData(environment, petSummaryEntrypoint, responseWithNewNickname, { id: '1' });
// PetName on Pet:1 (height 0), then PetSummary (height 1), ran once each; RecentCheckins did not.
// onChange was called once, after both ran; snapshot returns the new result.

stop();
normalizeData(environment, petDetailEntrypoint, petDetailWithNewName, { id: '1', skip: null });
// Nobody observes PetSummary: PetName on Pet:1 and PetSummary are marked Stale and did not run.
// snapshot(environment, location) still returns the previous result.
const stopAgain = listen(environment, location, onChange);
// listen ran PetName on Pet:1 and PetSummary once each; snapshot returns the new result.

stopAgain();
dispose(environment, retention);
garbageCollectEnvironment(environment); // the other retentions keep their records
expect(environment.store).toEqual(storeOf({ /* .. the records the other retentions reach */ }));
```

The store is store.md's nested map: the test compares `environment.store` with `storeOf`, as store.md's tests compare a store.

A React package, which this doc does not plan, would render with:

```tsx
// from libs/isograph-react/src/useResolverResult.ts
const result = useSyncExternalStore(
  (onStoreChange) => listen(environment, location, onStoreChange),
  () => snapshot(environment, location),
);
```

## 1. `normalize` skips an unchanged value and returns the changes it made

Independent of the recomputation architecture's ordering. It depends on the architecture only in that it tags nothing: with epochs (Candidate 2), this change would instead tag every value.

The normalization AST's list depth (List depth on leaf selections) lands in this change, not in a prefactor before it. Its only reader is `isUnchanged`, and a prefactor would add a field nothing reads.

`Store` is store.md's, unchanged: `Map<EntityName, Map<RecordId, StoreRecord>>`, a record holds only server values, untagged, and the empty store is `new Map()`. The recomputation graph lives beside it on the environment (The engine, the store, and the environment).

### What `normalize` returns

Before (store.md, `normalize`):

```ts
// from libs/isograph-runtime/src/normalize.ts
export function normalize(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void;
```

After:

```ts
// from libs/isograph-runtime/src/normalize.ts
/** Writes `response` into `store` from `ast.root` down, by `ast` and `variables`, and returns
 * the changes it made, in write order. Throws `PartialWriteError` on a response that does not
 * match `ast` (store.md, Errors). */
export function normalize(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): ReadonlyArray<StoreChange> {
  const changes: StoreChange[] = [];
  const normalization: Normalization = {
    store,
    discriminatorResponseKey: ast.discriminatorResponseKey,
    variables,
    report: (change) => {
      changes.push(change);
    },
  };
  try {
    writeSelections(
      normalization,
      ast.selections,
      response,
      recordAt(normalization, ast.root),
      ast.root,
      [],
    );
  } catch (error: unknown) {
    throw new PartialWriteError(changes, error);
  }
  return changes;
}

/** One change `normalize` made to the store. */
export type StoreChange = RecordCreated | ValueChanged;

/** `normalize` inserted `record`, which the store did not hold. */
export type RecordCreated = {
  readonly kind: 'RecordCreated';
  readonly record: StoreLink;
};

/** `normalize` set `storeKey` on `record`, which did not hold it or held a value `isUnchanged` rejected. */
export type ValueChanged = {
  readonly kind: 'ValueChanged';
  readonly record: StoreLink;
  readonly storeKey: StoreKey;
};

/** A response that does not match the AST, thrown after the writes `changes` lists were made. */
export class PartialWriteError extends Error {
  constructor(
    /** What `normalize` changed before the throw, in write order. */
    readonly changes: ReadonlyArray<StoreChange>,
    cause: unknown,
  ) {
    super(cause instanceof Error ? cause.message : String(cause), { cause });
  }
}
```

```ts
// from libs/isograph-runtime/src/normalize.ts
/** What every step of one `normalize` call reads besides its position. */
type Normalization = {
  readonly store: Store;
  readonly discriminatorResponseKey: string;
  readonly variables: Variables;
  /** Receives each change as it is made. */
  readonly report: (change: StoreChange) => void;
};

/** The store's record at `link`, inserting an empty one and reporting `RecordCreated` when the store has none. */
function recordAt(normalization: Normalization, link: StoreLink): StoreRecord {
  return getOrInsert(
    getOrInsert(normalization.store, link.entityName, () => new Map()),
    link.id,
    () => {
      normalization.report({ kind: 'RecordCreated', record: link });
      return new Map();
    },
  );
}
```

Deltas from v1 (`libs/isograph-runtime/src/normalize.ts`): `Normalization` gains `report`; `normalize` holds the list next to the `report` that pushes to it, wraps the walk in `try`, and returns the list; `recordAt` takes the `Normalization` instead of the `Store`, and its insert reports; `compositeValue`'s call passes `normalization` where it passed `normalization.store`.

The list's reader is `normalizeData` (change 2), which maps each change to an `InputId` and calls `markChanged`. v1's callers (network-and-subscriptions.md's `makeNetworkRequest` until change 2, and store.md's tests) ignore it. A record written at two positions of one response reports `RecordCreated` once. A key written twice in one response with two different values reports `ValueChanged` twice; marking a node twice is the same as marking it once.

A `PartialWriteError`'s message is the message of the error it wraps, so store.md's `toThrow("Malformed response at ..")` assertions hold unchanged. Its reader is `normalizeData`, which marks the changes it carries before rethrowing (open decision 17).

### `normalize` skips an unchanged value

Before (store.md, `normalize`): every selected value is written with `record.set(key, value)`, and there is no change detection.

After, every write goes through `writeValue`. `normalize` already knows each value's shape from the AST: a `NormalizationLeafSelection` (`target: null`) writes a `LeafValue` with its `listDepth`, and a `NormalizationCompositeSelection` writes a `CompositeValue`. It passes that shape, and the comparison follows it, as upstream's does (`normalizeScalarField` and `normalizeLinkedField`, `cache.ts:232-341`). There is no generic deep equality.

```ts
// from libs/isograph-runtime/src/normalize.ts
/** A value `normalize` writes, with the shape its selection gives it. */
type WrittenValue =
  | {
      readonly kind: 'Leaf';
      readonly value: LeafValue;
      /** The selection's `listDepth`. */
      readonly listDepth: number;
    }
  | { readonly kind: 'Composite'; readonly value: CompositeValue };

/** Writes `written.value` under `key` of the record `owner` and reports `ValueChanged`, unless the
 * stored value is unchanged by `isUnchanged`. */
function writeValue(
  normalization: Normalization,
  record: StoreRecord,
  owner: StoreLink,
  key: StoreKey,
  written: WrittenValue,
): void;

/** `Leaf`: `leafUnchanged(stored, written.value, written.listDepth)`.
 * `Composite`: `compositeUnchanged(stored, written.value)`. */
function isUnchanged(stored: StoreValue, written: WrittenValue): boolean;

/** `listDepth` 0, or `written` not a list: `stored === written`. Otherwise `stored` is a list
 * of the same length whose items are `leafUnchanged` at `listDepth - 1` in order. */
function leafUnchanged(stored: unknown, written: LeafValue, listDepth: number): boolean {
  if (listDepth === 0 || !isArray(written)) {
    return stored === written;
  }
  return (
    isArray(stored) &&
    stored.length === written.length &&
    written.every((item, index) => leafUnchanged(stored[index], item, listDepth - 1))
  );
}

/** `null`: `stored` is `null`. A link: `stored` is a link with the same `entityName` and `id`.
 * A list: `stored` is a list of the same length whose items are `compositeUnchanged` in order. */
function compositeUnchanged(stored: StoreValue, written: CompositeValue): boolean;
```

- Absent key, or a value `isUnchanged` rejects: `record.set(key, written.value)`, then `normalization.report({ kind: 'ValueChanged', record: owner, storeKey: key })`.
- An unchanged value: nothing is written or reported; the stored value's identity stays.

`writeSelection` builds the `WrittenValue` from the selection: `selection.target === null` narrows `NormalizationSelection` to `NormalizationLeafSelection`, whose `listDepth` goes into the `Leaf` value.

`compositeUnchanged` reads `stored` as `StoreValue`, because `record.get` returns that type; it narrows by `isArray` and by `typeof stored === 'object'`, with no cast. `leafUnchanged` takes `stored: unknown`, because util's `isArray` narrows to `readonly unknown[]`, so a stored list's items are `unknown`; it compares them only with `===` and `isArray`, so it needs no cast either. Each comparison costs one `===` for a scalar leaf, one `===` per element for a leaf list, and one step per link for a composite, one comparison per written field.

A leaf at depth 0 whose JSON value is an object or a list (a `JSON` custom scalar such as `settings`) compares with `===` and so counts as changed on every rewrite: the AST records the selectable's list wrapping and not its scalar type, so `normalize` does not know whether the JSON value nests further. A leaf list item at depth 0 that is an object or a list (`[JSON]`) is the same case. A depth larger than the value's nesting is sound: `leafUnchanged` answers `true` only when every compared level is equal, and stops at a non-list `written` with `===`.

Its reader is the returned list: an unchanged write reports nothing, so change 2 marks nothing for it. The check exists because without it every response marks every written field changed, so every refetch of a retained query recomputes every resolver entry under it, and the whole retained tree recomputes on every response. Whether the comparison earns its cost against that recomputation is not measured.

A list of scalars (`tags: [String]`) is a new array on every response. Compared with `===`, it would count as changed on every rewrite, and every entry that reads it would recompute on every response. Lists of scalars are common in GraphQL schemas, so the AST records each leaf selection's list depth and `leafUnchanged` compares such a list element by element. A `JSON` custom scalar is also a new object on every response; it stays `===` and recomputes its readers on every response.

Deltas from upstream: upstream compares every scalar field with `!==` only (`cache.ts:248-249`), so a list of scalars counts as changed on every response; here a leaf list is compared by length and then `===` per element, recursing once per list level of the selectable's type. Upstream's `NormalizationScalarField` has no list depth. Upstream counts a `null` write as unchanged when the stored value is `null` (`cache.ts:243-246`, `:270-273`); `===` on a scalar leaf covers both that and the `!==` comparison. Upstream compares a single link by `__link` and `__typename` (`cache.ts:337-338`) and a list of links by length and then item by item (`dataIdsAreTheSame`, `cache.ts:372-393`). Here a link is compared by `entityName` and `id`, the fields of `StoreLink`, and lists nest at any depth (store.md's `CompositeValue`), so `compositeUnchanged` recurses where upstream's list check is one level. Upstream returns `RecordHasBeenUpdated` up the walk; here `writeValue` acts on the result at the write. The only structural comparison in changes 1 to 4 besides `compositeUnchanged` is `resultsEqual`'s comparison of `MissingData`, which the runtime builds from strings and links (R4).

### List depth on leaf selections

The depth is core data. The protocol converts a GraphQL type annotation into the core's `Wrapper<EntityName>` (`wrapper_of_graphql_type`, `crates/graphql_network_protocol/src/protocol_data.rs`), stored as `ServerSelectable.target`. The core reads the depth from that wrapper; the protocol does not change.

```rust
// from crates/isograph_compiler/src/wrapper.rs
/// How many list levels a value of a wrapper nests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListDepth(pub u32);

impl ListDepth {
    fn deeper(self) -> ListDepth {
        ListDepth(self.0.saturating_add(1))
    }
}

impl<T> Wrapper<T> {
    /// 0 for `Inner`, one more than the inner wrapper's for `List`, and the largest of the
    /// variants' for `Union`.
    pub fn list_depth(&self) -> ListDepth {
        match self {
            Wrapper::Inner(_) => ListDepth::default(),
            Wrapper::List(list) => list.list_depth().deeper(),
            Wrapper::Union(union) => union
                .0
                .iter()
                .map(|variant| match variant {
                    WrapperVariant::Inner(_) | WrapperVariant::Null => ListDepth::default(),
                    WrapperVariant::List(list) => list.list_depth().deeper(),
                })
                .max()
                .unwrap_or_default(),
        }
    }
}
```

GraphQL's `String` is `Union([Inner(String), Null])`, depth 0; `[String]` is `Union([List(Union([Inner(String), Null])), Null])`, depth 1; `[[String]!]!` is `List(List(Union([Inner(String), Null])))`, depth 2. A union whose variants differ in depth takes the largest, which is sound (`leafUnchanged`). `saturating_add` keeps the function free of an overflow panic; a depth of `u32::MAX` is not reachable, because the wrapper is built by recursion over source text, and a smaller depth only compares fewer levels.

The depth rides on the leaf variant at each stage:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
pub enum MergedSelectionTarget {
    Leaf(ListDepth),
    Composite(MergedCompositeTarget),
}

pub enum CompletedSelectionTarget {
    Leaf(ListDepth),
    Composite(CompletedCompositeTarget),
}
```

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
/// Whether a selection's target entity has selections, and for a leaf, how many list
/// levels its value nests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationTarget {
    Leaf(ListDepth),
    Composite(NormalizationCompositeTarget),
}
```

Where each stage sets it:

```rust
// from crates/isograph_compiler/src/merged_selection.rs
// server_contribution, before:
        (SelectableShape::Direct, EntityShape::Leaf) => match selection.selection_set {
            Some(_) => at_name(SelectionMapErrorKind::SelectionSetOnLeaf(key)).wrap_err(),
            None => {
                MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                    key: selection_key,
                    target: MergedSelectionTarget::Leaf,
                }))
                .wrap_ok()
            }
        },
// after:
        (SelectableShape::Direct, EntityShape::Leaf) => match selection.selection_set {
            Some(_) => at_name(SelectionMapErrorKind::SelectionSetOnLeaf(key)).wrap_err(),
            None => {
                MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                    key: selection_key,
                    target: MergedSelectionTarget::Leaf(server.target.list_depth()),
                }))
                .wrap_ok()
            }
        },
```

```rust
// from crates/isograph_compiler/src/merged_selection.rs
// MergedSelectionTarget::merge, before:
            (MergedSelectionTarget::Leaf, MergedSelectionTarget::Leaf) => {
                MergedSelectionTarget::Leaf.wrap_ok()
            }
// after:
            (MergedSelectionTarget::Leaf(first), MergedSelectionTarget::Leaf(second)) => {
                if first != second {
                    return MergeError::SelectionMismatch.wrap_err();
                }
                MergedSelectionTarget::Leaf(first).wrap_ok()
            }
```

Two selections under one key of one map select one selectable, so their depths are equal; the check mirrors the `Composite` arm's `first.entity != second.entity` rather than dropping `second` unread.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
// complete_target, before:
        MergedSelectionTarget::Leaf => CompletedSelectionTarget::Leaf.wrap_ok(),
// after:
        MergedSelectionTarget::Leaf(depth) => CompletedSelectionTarget::Leaf(depth.dereference()).wrap_ok(),
```

```rust
// from crates/isograph_compiler/src/normalization_ast.rs
// normalization_node, Identity arm, before:
                target: NormalizationTarget::Leaf,
// after:
                target: NormalizationTarget::Leaf(ListDepth::default()),

// normalization_selection, before:
            CompletedSelectionTarget::Leaf => NormalizationTarget::Leaf,
// after:
            CompletedSelectionTarget::Leaf(depth) => NormalizationTarget::Leaf(depth.dereference()),
```

`CompletedSelection::Identity` carries only a name. Its depth is 0 because its value is a record id, and `recordLink` throws on an identity value that is not a string, so no response `normalize` accepts holds a list there.

Every other match on a `Leaf` variant becomes `Leaf(_)`: `merged_selection.rs` (the variables walk, `complete_selection`, the query-text and test printers), `graphql_network_protocol/src/print_operation.rs` (`CompletedSelectionTarget::Leaf` in the operation printer, `NormalizationTarget::Leaf` in its tests, and `selection.target == NormalizationTarget::Leaf` becomes `matches!(selection.target, NormalizationTarget::Leaf(_))`). Test constructors of a leaf (`leaf` in `merged_selection.rs` and `print_operation.rs`, `using` in `query.rs`, the leaf nodes in `normalization_ast.rs` and `typescript_artifacts`) pass `ListDepth::default()`.

The printer writes `listDepth` after `target` on a leaf selection:

```rust
// from crates/typescript_artifacts/src/normalization_ast.rs
// node_value, Selection arm, before:
        NormalizationNode::Selection(selection) => ArtifactValue::Object(vec![
            kind("Selection"),
            entry(
                "responseKey",
                match &selection.response_key {
                    ResponseKey::SelectableName => ArtifactValue::Null,
                    ResponseKey::Alias(alias) => string(alias),
                },
            ),
            entry("selectableName", string(selection.selectable_name)),
            entry("arguments", arguments_value(&selection.arguments)),
            entry(
                "target",
                match &selection.target {
                    NormalizationTarget::Leaf => ArtifactValue::Null,
                    NormalizationTarget::Composite(target) => composite_target_value(target),
                },
            ),
        ]),
// after:
        NormalizationNode::Selection(selection) => ArtifactValue::Object(
            [
                kind("Selection"),
                entry(
                    "responseKey",
                    match &selection.response_key {
                        ResponseKey::SelectableName => ArtifactValue::Null,
                        ResponseKey::Alias(alias) => string(alias),
                    },
                ),
                entry("selectableName", string(selection.selectable_name)),
                entry("arguments", arguments_value(&selection.arguments)),
            ]
            .into_iter()
            .chain(match &selection.target {
                NormalizationTarget::Leaf(depth) => vec![
                    entry("target", ArtifactValue::Null),
                    entry("listDepth", ArtifactValue::Number(f64::from(depth.0))),
                ],
                NormalizationTarget::Composite(target) => {
                    entry("target", composite_target_value(target)).wrap_vec()
                }
            })
            .collect(),
        ),
```

`f64::from(u32)` is exact, so `cast_precision_loss` does not apply.

The TypeScript type splits by target, so a composite selection cannot carry a depth:

```ts
// from libs/isograph-runtime/src/artifacts.ts
// before:
/** A selection of a selectable. */
export type NormalizationSelection = {
  readonly kind: 'Selection';
  /** The alias the query text gives this selection, the key of its value in the response;
   * `null` when the query prints no alias and the key is `selectableName`. */
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  /** `null` when the target entity is a leaf. */
  readonly target: NormalizationCompositeTarget | null;
};
// after:
/** A selection of a selectable. */
export type NormalizationSelection =
  | NormalizationLeafSelection
  | NormalizationCompositeSelection;

/** A selection of a selectable whose target entity is a leaf. */
export type NormalizationLeafSelection = {
  readonly kind: 'Selection';
  /** The alias the query text gives this selection, the key of its value in the response;
   * `null` when the query prints no alias and the key is `selectableName`. */
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  readonly target: null;
  /** How many list levels the selectable's type nests: 0 for `String`, 1 for `[String]`,
   * 2 for `[[String]]`. */
  readonly listDepth: number;
};

/** A selection of a selectable whose target entity is composite. */
export type NormalizationCompositeSelection = {
  readonly kind: 'Selection';
  /** The alias the query text gives this selection, the key of its value in the response;
   * `null` when the query prints no alias and the key is `selectableName`. */
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  readonly target: NormalizationCompositeTarget;
};
```

`target` is the discriminant, so the existing `target == null` and `target != null` tests in `normalize`, `check`, and `garbageCollection` narrow without change, and `RecordPosition.selection` and `storeKey` read only fields both members have.

The artifact a user sees for `field Query.Leaves { tags settings }` holds `{ kind: "Selection", responseKey: null, selectableName: "tags", arguments: null, target: null, listDepth: 1 }` and `{ kind: "Selection", responseKey: null, selectableName: "settings", arguments: null, target: null, listDepth: 0 }`.

### Tests

store.md's tests are unchanged: `Store` keeps its type, `normalize` keeps its parameters, and the tests ignore its return value and compare the store with `toEqual(storeOf({ .. }))` and `toEqual(new Map())`. New cases in `test/normalize.test.ts`, each comparing the store with `storeOf` and asserting the list `normalize` returns:

- A first `PetDetail` response into `new Map()` returns `RecordCreated` for `Query.__ROOT`, then `ValueChanged` for each key written and `RecordCreated` for each record inserted, in write order; the store is `toEqual` the `storeOf` store.md's case asserts.
- The same response normalized again returns `[]`.
- A link `bestFriend` to `Pet:2` written twice: the first link object stays (`toBe`), and the second list holds no `ValueChanged` for `bestFriend`; then `Pet:3` stores the new link and returns `ValueChanged` for `bestFriend` on `Pet:1`.
- A list of links `checkins` with the same ids in the same order: the first list object stays (`toBe`); one fewer item, or two items swapped, stores the new list and returns `ValueChanged` for `checkins`.
- `petGrid` with equal links and `null` items at every depth: the first outer list object stays (`toBe`); one inner item changed stores a new outer list.
- `stats: null` then `stats: null`: the record holds `null`, and the second list holds no `ValueChanged` for `stats`.
- `tags: ["a", null]` then `tags: ["a", null]` keeps the first list (`toBe`), and the second list holds no `ValueChanged` for `tags`.
- `tags: ["a", null]` then `tags: ["b", null]` stores the new list; `["a"]` then `["a", null]` stores the new list; `null` then `["a"]` and `["a"]` then `null` store the new value. Each returns `ValueChanged` for `tags`.
- `tagGrid: [["a"], [], null]` then an equal response keeps the first outer list (`toBe`); `[["a"], ["b"], null]` after `[["a"], ["c"], null]` stores the new outer list.
- `settings: ["a"]`, a `JSON` custom scalar holding an array, written twice stores the new array (`not.toBe` the first) and returns `ValueChanged` for `settings`, and a `settings` object written twice does the same: a depth-0 leaf compares by `===`.
- A response whose second pet lacks `name`, after the first pet was written: `normalize` throws a `PartialWriteError` whose message is v1's "Malformed response at .." message and whose `changes` are the writes made before the throw; the store holds those writes.
- A response that lacks a root key: `PartialWriteError` whose `changes` hold only `RecordCreated` for `Query.__ROOT`.

Fixture additions to store.md's fixture project: `tagGrid: [[String]]` on `Query` in `fixtures/store/schema/schema.graphql`, and `fixtures/store/src/TagGrid.ts` with `field Query.TagGrid { tagGrid }` and `entrypoint Query.TagGrid`, whose resolver returns `data`.

New cases in the compiler:

- `crates/isograph_compiler/src/wrapper.rs`: `list_depth` of `Inner(String)` is 0, of `Union([Inner(String), Null])` is 0, of `Union([List(Union([Inner(String), Null])), Null])` is 1, of `List(List(Inner(String)))` is 2, of `Union([Inner(String), List(Inner(String)), Null])` is 1, and of `Union([])` is 0.
- `crates/graphql_network_protocol/src/tests/query.rs`, with the schema `scalar ID\nscalar String\nscalar JSON\ntype Query { greeting: String tags: [String] tagGrid: [[String]!]! settings: JSON pet: Pet } type Pet { id: ID! }` and `field Query.Leaves { greeting tags tagGrid settings pet { id } }`: the normalization AST holds `greeting` and `settings` with `Leaf(ListDepth(0))`, `tags` with `Leaf(ListDepth(1))`, `tagGrid` with `Leaf(ListDepth(2))`, and `pet`'s target's `id` (the identity) with `Leaf(ListDepth(0))`.
- `crates/typescript_artifacts`: a leaf selection prints `target: null, listDepth: 1` for depth 1, and a composite selection prints no `listDepth`.
- Normalizing the same `PetDetail` response twice leaves every link and list of links `toBe` the one after the first, and every string, number, boolean, and `null` leaf `toBe` it.

## 2. Stored resolver results

Depends on the held architecture throughout, except The resolver store key and Results, equality, and a resolver that throws.

Two commits: 2a, the engine (`graph.ts`, The engine); 2b, the store's side and the environment (The store's side, The environment).

A resolver's result is stored in `environment.graph` as a node, under an id built from the record it was read on and the client field's resolver store key. Reading a resolver whose inputs did not change returns the stored result without calling the resolver. `normalizeData` and `garbageCollectEnvironment` mark `Stale` the nodes that read what `normalize` and `garbageCollect` changed, and the nodes that read those; the next read of a `Stale` node recomputes it. This change has no observers, so no node is needed and every mark is `Stale`; change 4 queues the needed nodes instead.

A resolver entry, in the rest of this doc, is the graph node of one resolver store key on one record.
### The resolver store key

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** `selectableName`, then one `____<name>___<stableStringify(value)>` chunk per entry of
 * `variables`, names in code unit order: `argumentChunks`'s format. */
export function resolverStoreKey(
  selectableName: string,
  variables: Variables,
): StoreKey;
```

The variables are the field's own: for a `Resolver` node, `childVariables(variables, selection.arguments)` (reader-ast.md, The read function), which holds the arguments the selection passes and the compiler-written defaults, and omits an unpassed variable with no default; for a top-level `read`, the read's variables. `PetName` on `Pet:1` is `PetName`; `RecentCheckins` selected with no argument is `RecentCheckins____n___2`; `PetSummary` read with `{ id: "1" }` is `PetSummary____id___"1"` on `Query.__ROOT`. Two selections of one client field with equal argument values on one record are one key, whatever their aliases.

The key needs the client field's selectable name, which v1's reader artifact does not carry (reader-ast.md drops upstream's `fieldName` because nothing read it, `reader.ts:26-37`). It does now:

Before:

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<TParam, TOutput> = {
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
```

After:

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<TParam, TOutput> = {
  /** The client field's selectable name: the prefix of its resolver store key. */
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
```

`reader_artifact_file` (`crates/typescript_artifacts/src/reader_artifact.rs`, reader-ast.md) writes `selectableName: String(field.field.selectable_name)` as the object's first entry. `Pet/fullName/resolver_reader.ts` gains `selectableName: "fullName",`.

### The engine (2a)

```ts
// from libs/isograph-runtime/src/graph.ts
declare const inputIdBrand: unique symbol;
declare const nodeIdBrand: unique symbol;

/** A value the store owns that a computation can read. The engine compares ids by `===` and never parses one. */
export type InputId = string & { readonly [inputIdBrand]: true };

/** A node. One `NodeId` names one computation for the node's whole life. */
export type NodeId = string & { readonly [nodeIdBrand]: true };

/** The recomputation graph: nodes and the inverse of their dependencies. */
export type RecomputationGraph<TValue> = {
  readonly nodes: Map<NodeId, GraphNode<TValue>>;
  /** The nodes whose last computation read each input. Written only by `setDependencies`. */
  readonly inputDependents: Map<InputId, Set<GraphNode<TValue>>>;
  /** The nodes created with each owner. Written only by `createNode` and `disposeNode`. */
  readonly owned: Map<InputId, Set<GraphNode<TValue>>>;
  /** Whether two values of a node are equal, for cutoff: `resultsEqual` in the runtime. */
  readonly valuesEqual: (a: TValue, b: TValue) => boolean;
};

export type GraphNode<TValue> = {
  readonly id: NodeId;
  /** The input whose deletion disposes this node: the presence of the record it was computed on. */
  readonly owner: InputId;
  readonly computation: Computation<TValue>;
  value: TValue;
  state: NodeState;
  /** What the last computation read, in read order. Written only by `setDependencies`. */
  dependencies: ReadonlyArray<GraphDependency<TValue>>;
  /** Every node whose `dependencies` hold a `Node` dependency on this one. Written only by `setDependencies`. */
  readonly dependents: Set<GraphNode<TValue>>;
};

/**
 * `Clean`: `value` is what a computation against the store would return.
 * `Stale`: an input it read changed, or a node it reads is `Stale`. It recomputes when read.
 */
export type NodeState = 'Clean' | 'Stale';

export type GraphDependency<TValue> = InputDependency | NodeDependency<TValue>;
export type InputDependency = { readonly kind: 'Input'; readonly input: InputId };
export type NodeDependency<TValue> = { readonly kind: 'Node'; readonly node: GraphNode<TValue> };

/** A node's computation. It reports every input it reads, and reads other nodes, through `context`. */
export type Computation<TValue> = (context: ComputeContext<TValue>) => TValue;

export type ComputeContext<TValue> = {
  /** Records that the computation read `input`. */
  readInput(input: InputId): void;
  /** Brings the node `id` up to date, creating it from `create()` when absent, records the
   * dependency, and returns its value. */
  readNode(id: NodeId, create: () => NodeDefinition<TValue>): TValue;
};

/** What creating an absent node needs. */
export type NodeDefinition<TValue> = {
  readonly owner: InputId;
  readonly computation: Computation<TValue>;
};
```

The context is a parameter, built per computation by `compute`, as the walk's `ReadHooks` is; there is no ambient "current computation" (MobX's `globalState.trackingDerivation`, Reactively's `CurrentReaction`). The engine knows nothing about what a node computes: the caller supplies the computation, the owner, and the id. Resolver entries are the one node kind changes 2 to 4 create; Other node kinds lists the planned ones, and the engine's API admits them unchanged.

A node is mutated in place (`value`, `state`, `dependencies`), because other nodes' `dependents` sets and `NodeDependency`s hold it by identity. An input dependency is an `InputId`, because a server value has no object to point at and a record may not exist yet. A node dependency is the node, because `dependents` holds nodes and change 4's needed counts walk from a node to the nodes it reads. `graph.ts` imports `getOrInsert` from `util.ts` and nothing else.

```ts
// from libs/isograph-runtime/src/graph.ts
export function createGraph<TValue>(
  valuesEqual: (a: TValue, b: TValue) => boolean,
): RecomputationGraph<TValue> {
  return { nodes: new Map(), inputDependents: new Map(), owned: new Map(), valuesEqual };
}

/** The value of the node `id`, created from `create()` when absent and brought up to date.
 * Records no dependency: the caller is not a node. */
export function track<TValue>(
  graph: RecomputationGraph<TValue>,
  id: NodeId,
  create: () => NodeDefinition<TValue>,
): TValue {
  return upToDateNode(graph, id, create).value;
}

/** Marks `Stale` every `Clean` node that read one of `inputs`, with its `Clean` dependents,
 * transitively. Runs no computation. */
export function markChanged<TValue>(
  graph: RecomputationGraph<TValue>,
  inputs: ReadonlyArray<InputId>,
): void {
  inputs.forEach((input) => graph.inputDependents.get(input)?.forEach(markStale));
}

/** `markChanged(graph, [input])`, then marks the dependents of every node `input` owns, and
 * disposes those nodes. */
export function deleteInput<TValue>(
  graph: RecomputationGraph<TValue>,
  input: InputId,
): void {
  markChanged(graph, [input]);
  const owned = graph.owned.get(input);
  owned?.forEach((node) => node.dependents.forEach(markStale));
  owned?.forEach((node) => disposeNode(graph, node));
}
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** The node `id`, created from `create()` when absent, and brought up to date. */
function upToDateNode<TValue>(
  graph: RecomputationGraph<TValue>,
  id: NodeId,
  create: () => NodeDefinition<TValue>,
): GraphNode<TValue> {
  const node = graph.nodes.get(id) ?? createNode(graph, id, create());
  bringUpToDate(graph, node);
  return node;
}

/** Marks a `Clean` node `Stale`, then its dependents. A node already `Stale` is left as is. */
function markStale<TValue>(node: GraphNode<TValue>): void {
  if (node.state !== 'Clean') {
    return;
  }
  node.state = 'Stale';
  node.dependents.forEach(markStale);
}

/** Recomputes a `Stale` node, then marks it `Clean`. Does nothing to a `Clean` node. */
function bringUpToDate<TValue>(graph: RecomputationGraph<TValue>, node: GraphNode<TValue>): void {
  if (node.state === 'Clean') {
    return;
  }
  recompute(graph, node);
  node.state = 'Clean';
}

/** Computes a new node, stores it `Clean` in `nodes` and under its owner in `owned`, and
 * records its dependencies. */
function createNode<TValue>(
  graph: RecomputationGraph<TValue>,
  id: NodeId,
  definition: NodeDefinition<TValue>,
): GraphNode<TValue> {
  const computed = compute(graph, definition.computation);
  const node: GraphNode<TValue> = {
    id,
    owner: definition.owner,
    computation: definition.computation,
    value: computed.value,
    state: 'Clean',
    dependencies: [],
    dependents: new Set(),
  };
  setDependencies(graph, node, computed.dependencies);
  graph.nodes.set(id, node);
  getOrInsert(graph.owned, node.owner, () => new Set()).add(node);
  return node;
}

/** Recomputes `node`; stores the new value when `valuesEqual` rejects it against the old one. */
function recompute<TValue>(graph: RecomputationGraph<TValue>, node: GraphNode<TValue>): void {
  const computed = compute(graph, node.computation);
  setDependencies(graph, node, computed.dependencies);
  if (!graph.valuesEqual(node.value, computed.value)) {
    node.value = computed.value;
  }
}

type Computed<TValue> = {
  readonly value: TValue;
  readonly dependencies: ReadonlyArray<GraphDependency<TValue>>;
};

/** Runs `computation` with a context that collects what it reads, in read order. */
function compute<TValue>(
  graph: RecomputationGraph<TValue>,
  computation: Computation<TValue>,
): Computed<TValue> {
  const dependencies: GraphDependency<TValue>[] = [];
  const value = computation({
    readInput: (input) => {
      dependencies.push({ kind: 'Input', input });
    },
    readNode: (id, create) => {
      const node = upToDateNode(graph, id, create);
      dependencies.push({ kind: 'Node', node });
      return node.value;
    },
  });
  return { value, dependencies };
}

/**
 * Replaces `node.dependencies` with `next`, and removes `node` from the dependents of each old
 * dependency and adds it to those of each new one. Deletes an `inputDependents` set left empty.
 */
function setDependencies<TValue>(
  graph: RecomputationGraph<TValue>,
  node: GraphNode<TValue>,
  next: ReadonlyArray<GraphDependency<TValue>>,
): void;

/** `setDependencies(graph, node, [])`, then removes `node` from `nodes` and from its owner's
 * `owned` set, deleting the set when it is left empty. */
function disposeNode<TValue>(graph: RecomputationGraph<TValue>, node: GraphNode<TValue>): void;
```

A `Stale` node recomputes without checking whether anything it read changed. Its reads of other nodes go through `readNode`, which brings each one up to date first, so its computation sees current inputs (R3). It recomputes once: it ends `Clean`, and only marking makes it `Stale` again, and marking does not run during a read. `recompute` marks no dependent: by the invariant below, every dependent of a `Stale` node is `Stale` already, except the parent whose read reached it, which is computing and ends `Clean` having read the new value. Keeping the old value when `valuesEqual` holds keeps its identity, which change 4's cutoff and notification read.

A read of `Stale` nodes has no equality cutoff: after `Rex` to `Ray`, the next read of `BadgePage` recomputes `Monogram`, `Signature`, `Badge`, and `BadgePage` although `Initial`'s result is `===` the old one (open decision 15). Change 4 gives needed nodes the cutoff.

Invariant, outside a call that is computing a node: every dependent of a `Stale` node is `Stale`. Marking keeps it, because a node that becomes `Stale` marks its dependents. A new dependent is added only by a computation, which brought the node up to date first, and a recomputed node ends `Clean` only after every node it read was brought up to date. `markStale` stops at a node already `Stale` because the invariant says its dependents are `Stale` too, so a second write to an input costs one step per node that read it.

The marking is transitive because nothing computes during a write: whether a node's value changes is known only by computing it. Change 4 keeps this marking for nodes nobody needs and replaces it, for needed nodes, by queuing the direct dependents of a node whose value changed.

A node's dependents of both kinds together are the inverse of the dependency lists: `node.dependents` for `Node` dependencies, `inputDependents` for `Input` dependencies. `setDependencies` is the only writer of either side; a violation is a silently stale result. Removing every old edge and adding every new one is `O(old + new)`; Reactively keeps the matching prefix instead (`update`, `packages/core/src/core.ts`), which saves set operations and not order of growth. A dependency recorded twice (two selections of one field on one record) is one edge in each set.

### The store's side (2b)

```ts
// from libs/isograph-runtime/src/inputs.ts
/** The input every read on `record` reports, whether the record is present or absent. It
 * changes when the record is created or deleted. */
export function recordInput(record: StoreLink): InputId {
  return JSON.stringify(['Record', record.entityName, record.id]) as InputId;
}

/** The input a lookup of `storeKey` on a present `record` reports. */
export function valueInput(record: StoreLink, storeKey: StoreKey): InputId {
  return JSON.stringify(['Value', record.entityName, record.id, storeKey]) as InputId;
}

/** The node of the resolver store key `storeKey` on `record`. */
export function resolverNode(record: StoreLink, storeKey: StoreKey): NodeId {
  return JSON.stringify([record.entityName, record.id, storeKey]) as NodeId;
}

/** The input `change` changed. */
export function changeInput(change: StoreChange): InputId {
  switch (change.kind) {
    case 'RecordCreated':
      return recordInput(change.record);
    case 'ValueChanged':
      return valueInput(change.record, change.storeKey);
  }
}
```

The casts are the only places an `InputId` or `NodeId` is minted. Each id is the JSON of a list of strings, so no two locations share an id, whatever characters an entity name, record id, or store key holds.

The reader's walk reports through hooks its caller passes:

```ts
// from libs/isograph-runtime/src/read.ts
/** What the walk reports for each read, and how it reads a nested resolver. `TFailure` is what a
 * nested resolver's result can be besides `Ok` and `MissingData`. */
export type ReadHooks<TFailure> = {
  /** Called at every presence check of `record`: the read's root, and each value read. */
  readonly readRecord: (record: StoreLink) => void;
  /** Called at every lookup of `storeKey` on the present `record`. */
  readonly readValue: (record: StoreLink, storeKey: StoreKey) => void;
  /** The result of `artifact` on `root` with `variables`. */
  readonly readResolver: (
    artifact: ReaderArtifact<ResolverParam, unknown>,
    root: StoreLink,
    variables: Variables,
  ) => ReadResult<unknown> | TFailure;
};

/** `artifact`'s reader AST read at `root` with `variables` through `hooks`. Nested resolvers are
 * read by `hooks.readResolver`; `artifact`'s own resolver is not called. */
export function readData<TFailure>(
  store: Store,
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure;

/** v1's `MissingData` for the absent record `root`. */
export function noRecord(root: StoreLink): ReadMissingData {
  return { kind: 'MissingData', reason: `No record for ${recordName(root)}`, recordLink: root };
}
```

`readData` is v1's `readButDoNotEvaluate` body with `hooks.readRecord(root)` before its presence check. v1's walk functions (`readSelectionSet`, `readSelection`, `readStoreValue`, `readLinkedValue`, and `allOk`) gain a `hooks: ReadHooks<TFailure>` parameter after `store`, passed down unchanged, and return `ReadResult<..> | TFailure`; `allOk` stops at the first result whose `kind` is not `Ok`, so a `TFailure` passes up unchanged. `readStoreValue` calls `hooks.readRecord(root)` before its presence check, and `hooks.readValue(root, key)` when the record is present. `noRecord` is the object `readButDoNotEvaluate` and `readStoreValue` build today, factored out. The `Resolver` case:

```ts
// from libs/isograph-runtime/src/read.ts
// before:
    case 'Resolver': {
      const resolved = read(
        store,
        selection.readerArtifact(),
        root,
        childVariables(variables, selection.arguments),
      );
      return resolved.kind === 'MissingData'
        ? missingDataFor(selection.alias, root, resolved)
        : resolved;
    }
// after:
    case 'Resolver': {
      const resolved = hooks.readResolver(
        selection.readerArtifact(),
        root,
        childVariables(variables, selection.arguments),
      );
      return resolved.kind === 'MissingData'
        ? missingDataFor(selection.alias, root, resolved)
        : resolved;
    }
```

v1's exported functions keep their signatures and behavior through v1's hooks:

```ts
// from libs/isograph-runtime/src/read.ts
export function readButDoNotEvaluate<TEntityName extends EntityName>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<SelectionSetData> {
  return readData(store, evaluatingHooks(store), artifact, root, variables);
}

/** v1's reads: nothing reported, and each nested resolver called by `read`, its throw propagating. */
function evaluatingHooks(store: Store): ReadHooks<never> {
  return {
    readRecord: () => {},
    readValue: () => {},
    readResolver: (artifact, root, variables) => read(store, artifact, root, variables),
  };
}
```

`read` keeps its body, which calls `readButDoNotEvaluate` and then the resolver. `TFailure` is `never` for v1's hooks, so `read` and `readButDoNotEvaluate` keep their v1 return types, and reader-ast.md's tests hold unchanged.

The stored read:

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** The stored result of `artifact` on `root` with `variables`: computed when absent, recomputed
 * when `Stale`, and otherwise returned as stored. */
export function readStoredResult<TOutput>(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, TOutput>,
  root: StoreLink,
  variables: Variables,
): ResolverResult<TOutput> {
  if (storeRecord(environment.store, root) === undefined) {
    return noRecord(root);
  }
  // The node under `artifact.selectableName`'s resolver store key on `root` was computed by
  // `artifact`: one selectable of one entity has one reader artifact.
  return track(
    environment.graph,
    resolverNode(root, resolverStoreKey(artifact.selectableName, variables)),
    () => resolverDefinition(environment.store, artifact, root, variables),
  ) as ResolverResult<TOutput>;
}

/** The node definition of `artifact` on the present record `root` with `variables`. */
export function resolverDefinition(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  variables: Variables,
): NodeDefinition<ResolverResult<unknown>> {
  return {
    owner: recordInput(root),
    computation: (context) => {
      const data = readData(store, contextHooks(store, context), artifact, root, variables);
      return data.kind === 'Ok' ? callResolver(artifact, root, data.value, variables) : data;
    },
  };
}

/** Hooks that report each read to `context` and read each nested resolver as a node. */
function contextHooks(
  store: Store,
  context: ComputeContext<ResolverResult<unknown>>,
): ReadHooks<ReadResolverThrew> {
  return {
    readRecord: (record) => context.readInput(recordInput(record)),
    readValue: (record, storeKey) => context.readInput(valueInput(record, storeKey)),
    readResolver: (artifact, root, variables) => {
      context.readInput(recordInput(root));
      if (storeRecord(store, root) === undefined) {
        return noRecord(root);
      }
      return context.readNode(
        resolverNode(root, resolverStoreKey(artifact.selectableName, variables)),
        () => resolverDefinition(store, artifact, root, variables),
      );
    },
  };
}
```

The cast in `readStoredResult` is the one place a stored `ResolverResult<unknown>` is returned at the caller's type; the invariant is in its comment. The computation closes over `store`, the environment's `Map`, which is written in place and never replaced (network-and-subscriptions.md, The environment).

Every reader of any key on a record also reported the record's presence, so a deletion reports one input instead of the record's keys, including keys read while absent, which the store cannot enumerate. A computation that stops at the first `MissingData` records the inputs read up to it, which are exactly what its result depends on.

What reads a stored resolver result: `readStoredResult`; a parent's `Resolver` node while the parent computes; `normalizeData`'s hold of a new retention's root (change 3); settlement and `listen` (change 4); `snapshot` (change 4). `normalize`, `check`, `garbageCollect`, `read`, and `readButDoNotEvaluate` never read one.

### The environment (2b)

```ts
// from libs/isograph-runtime/src/environment.ts
export type IsographEnvironment = {
  readonly store: Store;
  readonly networkFunction: IsographNetworkFunction;
  readonly retainedQueries: Set<RetainedQuery>;
  readonly networkRequestCache: Map<NetworkRequestKey, ParentCache<PromiseWrapper<void, unknown>>>;
  readonly subscriptions: Set<Subscription>;
  /** Stored resolver results, what each last read, and the inverse of that. */
  readonly graph: RecomputationGraph<ResolverResult<unknown>>;
};
```

Delta from network-and-subscriptions.md's `IsographEnvironment`: `graph`, whose reader is every function below. `createIsographEnvironment` keeps its parameters and adds `graph: createGraph(resultsEqual)`.

```ts
// from libs/isograph-runtime/src/environment.ts
/** `normalize` of `response` into `environment.store`, then marks `Stale` every node that read
 * what it changed. On a `PartialWriteError`, marks the changes it carries and rethrows it. */
export function normalizeData(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void {
  markChanged(environment.graph, writeOrMark(environment, ast, response, variables).map(changeInput));
}

/** `normalize`'s changes; on a `PartialWriteError`, marks its changes before rethrowing. */
function writeOrMark(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): ReadonlyArray<StoreChange> {
  try {
    return normalize(environment.store, ast, response, variables);
  } catch (error: unknown) {
    if (error instanceof PartialWriteError) {
      markChanged(environment.graph, error.changes.map(changeInput));
    }
    throw error;
  }
}

/** `garbageCollect` of the store, keeping what the retained queries reach, then deletes the
 * nodes of each deleted record and marks their readers. */
export function garbageCollectEnvironment(environment: IsographEnvironment): void {
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteInput(environment.graph, recordInput(record)),
  );
}
```

`garbageCollect` keeps its parameters and returns the links of the records it deleted:

```ts
// from libs/isograph-runtime/src/garbageCollection.ts
/** Deletes from `store` every record no query in `retained` reaches, and returns their links. */
export function garbageCollect(
  store: Store,
  retained: ReadonlyArray<RetainedQuery>,
): ReadonlyArray<StoreLink>;
```

Its sweep builds the list from the records it deletes; store.md's tests ignore it, and new cases in `test/garbageCollection.test.ts` assert it (Tests).

`makeNetworkRequest` (network-and-subscriptions.md) calls `normalizeData(environment, entrypoint.normalizationAst, response, variables)` where it called `normalize(environment.store, ..)`, then `callSubscriptions(environment)` as before.

### Results, equality, and a resolver that throws

```ts
// from libs/isograph-runtime/src/read.ts
/** A stored resolver result: v1's `ReadResult`, or the resolver threw. */
export type ResolverResult<T> = ReadResult<T> | ReadResolverThrew;

/** The resolver of `selectableName` on `recordLink` threw `error`. */
export type ReadResolverThrew = {
  readonly kind: 'ResolverThrew';
  readonly selectableName: string;
  readonly recordLink: StoreLink;
  readonly error: unknown;
};
```

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** `artifact.resolver` called with `data`; a throw becomes `ResolverThrew`. */
function callResolver(
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ResolverResult<unknown>;

/** `Ok`: values `===`. `MissingData`: equal `reason`, `recordLink` entity and id, and
 * `nestedReason`, recursively. `ResolverThrew`: errors `===`. Different kinds: unequal. */
export function resultsEqual(a: ResolverResult<unknown>, b: ResolverResult<unknown>): boolean;
```

v1's `ReadResult` is unchanged, so `read` and `readButDoNotEvaluate`, which never produce `ResolverThrew`, do not admit it in their types. v1 lets a throw propagate out of `read` (reader-ast.md, Semantics), and still does. In a stored result it is a value, because changes 3 and 4 run resolvers inside `normalizeData` (the new root's, and every needed node's): a throw that propagated would abort `normalizeData`, and leave the other observed nodes unsettled. Open decision 5.

Equality is upstream's for resolver values, `===` (`areEqualWithDeepComparison.ts:84-95`), as pico's is the value's `==`. It is R4's optimization, not a guarantee. A resolver that returns a `===` primitive or the same object queues none of its dependents (change 4); one that returns a new object is unequal on every recomputation, structurally equal or not, and every node that reads it recomputes. `MissingData` is compared field by field because the runtime builds it from strings and links, where structural equality is well-defined. Open decision 7 settles `===`.

### Tests

`test/read.test.ts` (reader-ast.md, Tests, Runtime) is unchanged: `read` and `readButDoNotEvaluate` keep v1's behavior, call every resolver on every read, leave the store unchanged, and propagate a resolver's throw.

New, in `test/graph.test.ts` (2a), with literal ids and computations that read inputs from a map the test holds and report them through `context.readInput`:

- A node read twice with `track` with no `markChanged` between: its computation runs once, and both values are `===`.
- `markChanged` of an input the node read: the next `track` runs the computation once. Of an input it did not read: none. Of `[]`: none.
- A node whose computation reads another node: `markChanged` of the inner node's input recomputes both on the next `track` of the outer; the inner computation runs before the outer's finishes.
- A recomputed value that `valuesEqual` accepts keeps the old value object (`toBe`).
- Dependencies follow the last computation: a computation that read `a` and then, after `markChanged([a])`, reads `b` instead is not marked by a later `markChanged([a])`, and `inputDependents` no longer holds a set for `a`.
- `deleteInput` of a node's owner: `nodes` no longer holds the node, `owned` no longer holds the owner, and an outer node that read it is `Stale`.
- Degenerate: `deleteInput` of an input nothing owns or read changes nothing.

New, in `test/memo.test.ts` (2b), over an environment built with `createIsographEnvironment(new Map(), deferredNetwork().networkFunction)`. Call counts come from `vi.spyOn(readerArtifact, 'resolver')`, which replaces the method on the one artifact object every `Resolver` node reads. Fixture sources added to store.md's fixture project:

```text
field Pet.Initial { name }
field Pet.Monogram { Initial }
field Pet.Signature { Initial nickname }
field Pet.Badge { Monogram Signature }
field Query.BadgePage($id: ID!) { pet(id: $id) { Badge } }

field Pet.FriendLabel { bestFriend { PetName } }
field Pet.FriendCard { FriendLabel tagline }
field Query.FriendPage($id: ID!) { pet(id: $id) { FriendCard } }

field Pet.NameShape { name }
field Pet.ShapeLabel { NameShape }
field Query.ShapePage($id: ID!) { pet(id: $id) { ShapeLabel } }

field Query.TwoNames {
  a: pet(id: "1") { PetName }
  b: pet(id: "1") { PetName }
}
```

with `entrypoint Query.BadgePage`, `entrypoint Query.FriendPage`, `entrypoint Query.ShapePage`, and `entrypoint Query.TwoNames`. `Initial` returns `data.name.slice(0, 1)`, `Monogram` returns `` `${data.Initial}.` ``, `Signature` returns `` `${data.Initial} ${data.nickname ?? ""}` ``, `NameShape` returns `{ length: data.name.length }`, `ShapeLabel` returns `` `${data.NameShape.length} letters` ``; every other fixture resolver returns `data` (store.md's convention).

In this change nothing settles after a write; each case writes with `normalizeData`, then reads with `readStoredResult`.

- `environment.store` is unchanged by every `readStoredResult`: it is `toEqual` the same `storeOf` before and after. After `PetSummary` with `{ id: "1" }`, `environment.graph.nodes` holds `resolverNode` of `Query.__ROOT` and `PetSummary____id___"1"`, of `Pet:1` and `PetName`, and of `Pet:1` and `RecentCheckins____n___2`, and no other.
- A second read with no write between: every resolver's call count is unchanged, and the result is `===` the first read's. (v1's `read`: every count doubles.)
- Diamond, `Pet:1.name` `Rex` to `Max`: `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once; `Badge` receives `{ Monogram: "M.", Signature: "M " }` on its one call; `Initial` is called once although `Monogram` and `Signature` both select it.
- Diamond, `Rex` to `Ray`: on the next read, `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once, and `Initial` returns `"R"`: a read of `Stale` nodes has no cutoff (open decision 15; change 4's Tests cover the cutoff for observed nodes).
- New object, `Pet:1.name` `Rex` to `Ray`, after a read of `ShapePage` with `{ id: "1" }`: the next read calls `NameShape`, `ShapeLabel`, and `ShapePage` once each; `NameShape`'s stored result is `toEqual` and not `toBe` the previous one.
- Diamond, only `nickname` changes: `Signature`, `Badge`, `BadgePage` once each; `Initial` and `Monogram` zero.
- Chain across fragments, `Pet:1.bestFriend` is `Pet:2`; `FriendPage` with `{ id: "1" }` and `PetSummary` with `{ id: "2" }` read, then `Pet:2.name` changes, then both read again in either order: `PetName` on `Pet:2` is called once; `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` once each; `RecentCheckins` zero.
- Unrelated write: only `Pet:1.tagline` changes: `FriendCard` and `FriendPage` once; `FriendLabel` and `PetName` zero.
- Link moved: `Pet:1.bestFriend` changes to `Pet:3` (with `name` and `nickname`); the next read of `FriendPage` calls `PetName` on `Pet:3` once; then `Pet:2.name` changes, and the next read of `FriendPage` calls nothing.
- Same resolver selected twice: a first read of `TwoNames` calls `PetName` once, and `a.PetName` and `b.PetName` are `===`.
- An identical response normalized again: no resolver is called on the next read.
- A list of scalars written twice: after a read of `Leaves` with `{ name: "Bo" }`, normalizing `tags: ["a", null]` again with the other values unchanged leaves the `Leaves` call count unchanged on the next read; `tags: ["b", null]` then calls `Leaves` once.
- Nested depth: after a read of `TagGrid`, an equal `tagGrid: [["a"], [], null]` calls nothing on the next read; `[["a"], ["b"], null]` calls `TagGrid` once.
- A `JSON` custom scalar array: `settings: ["a"]` written twice calls `Leaves` once on the next read.
- Missing then present: a read of `FriendPage` with `Pet:2` lacking `nickname` is `MissingData`; after `nickname` is written, the next read is `Ok` and calls `PetName`.
- Absent record: `FriendPage` read while `Pet:1.bestFriend` links a `Pet:9` absent from the store is `MissingData` naming `Pet:9`; after `normalizeData` writes `Pet:9` with `name` and `nickname`, the next read is `Ok`.
- A deleted record: after `garbageCollectEnvironment` deletes `Pet:2`, the next read of a stored `FriendPage` result is `MissingData` with `recordLink` `Pet:2`, not the stored `Ok`, and `environment.graph.nodes` holds no node of `Pet:2`.
- A resolver that throws: `readStoredResult` returns `ResolverThrew` with the error, the field's selectable name, and the record; its parent's result is the same `ResolverThrew` object. `read` of the same artifact throws, as in v1.
- A partial write: a response that throws part-way after changing `Pet:1.name`: `normalizeData` rethrows the `PartialWriteError`, and the next read of `BadgePage` calls `Initial` once.
- `resolverStoreKey`: `RecentCheckins` with `{ n: 2 }` is `RecentCheckins____n___2`; `{}` is `RecentCheckins`; `{ b: 1, a: "x" }` is `X____a___"x"____b___1`.

New in `test/garbageCollection.test.ts`: `garbageCollect` with nothing retained returns every record's link, in the store's order; with every record reached, `[]`.

## Ordering

Depends on the held architecture.

A write can make several stored results stale at once. R3 holds on both paths, and observers (change 4) add nothing that could break it:

- Heights. A reader artifact's height is above the height of every client field its `Resolver` nodes select (change 4, Heights), and an entry reads only entries of those fields, on whatever records the links lead to (Candidate 3a). So every entry an entry reads has a lower height, for every record and link value.
- The read path (change 2, The engine). A `Stale` entry recomputes when read, and each entry its computation reads is brought up to date by that read first. It recomputes once: it ends `Clean`, and only marking, which does not run during a read, makes it `Stale` again. The recursion terminates because heights decrease along it.
- The push (change 4, Settlement). `settle` drains bucket 0, then bucket 1, and so on. An entry of height `h` recomputes after every bucket below `h` is empty. Each entry it reads has a lower height and is `Clean` (recomputed in this pass, or not invalidated), or `Stale` or absent, in which case the read computes it before returning. A recomputation during the drain of bucket `h` queues only dependents of the entry it recomputed: an entry of height `h` queues entries of greater height, and an entry the read path recomputes queues nothing, because its dependents are `Stale` (change 2's invariant) or the entry being computed. So no bucket at or below `h` gains an entry once its drain has started, and an entry recomputes at most once per pass.

Equality cutoff is `recompute`'s, by `resultsEqual`, on the push: an entry whose result is `===` its previous one queues nothing. A new object always queues its dependents. The read path has no cutoff (open decision 15). Cutoff is an optimization (R4); R3 holds whether or not any result cuts off. The traces are in Candidate 3a.

### The same resolver selected twice

Two selections of one client field on one record with equal argument values are one store key, so one entry: computed once, read by both parents, each parent recording a dependency on the same entry and appearing once in its `dependents`. This holds across fragments and across queries. Unequal argument values are two keys and two entries. Aliases do not enter the key.

### Reads during recalculation

- A level 2 callback never runs during recalculation: change 4 calls callbacks only after `settle` has emptied the queue, so every needed entry is `Clean`. A callback that calls `snapshot` receives the stored result, which is current for every needed entry; one that calls `readStoredResult` receives a current result for any entry, because the read recomputes a `Stale` one. A callback that calls `normalizeData` runs a nested write, settlement, and notification; when it returns, the outer notification loop compares each remaining observer's stored result with the one it last notified, so no observer is called twice for one result or receives an older result after a newer one.
- A `readStoredResult` outside a write is the read path. It meets no `Queued` entry, because `normalizeData` and `garbageCollectEnvironment` empty the queue before they return.
- A resolver that reads or writes the store (through a closure, since its parameter holds no store) during its own computation breaks the guarantee: its inner `readStoredResult` records no dependency on the outer entry, and an inner `normalizeData` marks and queues entries while an outer recomputation is reading them. Open decision 8.
- Cycles: none. The static reader graph has none (a cycle of client field selections is an `InlineCycle` query error, query-generation.md, Cycles, which `reader_height` also checks), and every runtime dependency goes to a lower height, so no entry is reached again while it computes, and marking terminates.

## 3. Level 1: `normalizeData` retains and computes its root

### `normalizeData`

Before (change 2):

```ts
// from libs/isograph-runtime/src/environment.ts
export function normalizeData(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void;
```

After:

```ts
// from libs/isograph-runtime/src/environment.ts
/** What `normalizeData` needs from an `entrypoint.ts` artifact. */
export type Entrypoint<TOutput> = {
  readonly normalizationAst: NormalizationAst;
  readonly readerArtifact: () => ReaderArtifact<ResolverParam, TOutput>;
};

/**
 * Writes `response` with `normalize` and marks what it changed, recomputes the nodes observers
 * need (change 4), computes this query's root result and every result it reaches, retains the
 * query, and calls observers (change 4). Throws on a response that does not match the AST
 * (store.md, Errors), after marking what it wrote.
 */
export function normalizeData<TOutput>(
  environment: IsographEnvironment,
  entrypoint: Entrypoint<TOutput>,
  response: JsonObject,
  variables: Variables,
): Retention<TOutput>;
```

`Entrypoint` is structural: the generated `entrypoint.ts` object (`IsographEntrypoint`, whose `readerArtifact` is a function returning the artifact, `libs/isograph-runtime/src/artifacts.ts`) is assignable, and `TOutput` is inferred from its reader artifact's declared type. No compiler change.

`normalize` stays the store's write, with change 1's return value; nothing is renamed. `makeNetworkRequest` passes the entrypoint and holds the returned `Retention` (Retention).

### Retention

```ts
// from libs/isograph-runtime/src/retention.ts
/** One `normalizeData` call's hold on its query's records and its entrypoint's resolver result. */
export type Retention<TOutput> = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly artifact: ReaderArtifact<ResolverParam, TOutput>;
  /** `ast.root` and `resolverStoreKey(artifact.selectableName, variables)`. */
  readonly location: EntryLocation;
  /** Removes the root hold `retainRoot` added. */
  readonly releaseRoot: CleanupFn;
};

/** Where a resolver entry lives: its node is `resolverNode(record, storeKey)`. */
export type EntryLocation = {
  readonly record: StoreLink;
  readonly storeKey: StoreKey;
};

/** Ends `retention`: its records and resolver results are no longer kept by it. */
export function dispose(environment: IsographEnvironment, retention: Retention<unknown>): void {
  environment.retainedQueries.delete(retention);
  retention.releaseRoot();
}
```

A `Retention` has `ast` and `variables`, so it is a `RetainedQuery` (`garbageCollection.ts`), and `normalizeData` adds it to `environment.retainedQueries`, the set the network path's retains already go into. `garbageCollectEnvironment` keeps what every member reaches, so a retention keeps its records through store.md's mark with no new environment field. This is upstream's `retainQuery` and `unretainQuery` (`garbageCollection.ts:40-63`) and its `RetainedQuery` (`:20-24`) with the entrypoint's reader artifact and the root hold added, without the release buffer (store.md, `garbageCollect`). Normalizing retains, so retaining and computing happen together. Upstream retains at the request (`makeNetworkRequest.ts:464-493`) and reads in render; here `makeNetworkRequest` keeps its retain at the request (network-and-subscriptions.md, `fetchEntrypoint`) and also holds the `Retention` its `normalizeData` returns, and its cleanup unretains the first and calls `dispose` on the second.

The graph gains root holds:

```ts
// from libs/isograph-runtime/src/graph.ts
export type GraphNode<TValue> = {
  // .. change 2's fields, and:
  /** The holds `retainRoot` added at this node. Read by `collect`. */
  rootCount: number;
};

/** The value of the node `id`, created from `create()` when absent and brought up to date, with
 * one more root hold. The cleanup removes the hold. */
export function retainRoot<TValue>(
  graph: RecomputationGraph<TValue>,
  id: NodeId,
  create: () => NodeDefinition<TValue>,
): ItemCleanupPair<TValue>;
```

`createNode` starts `rootCount` at 0. The cleanup closes over the node object, so it is safe after `deleteInput` disposed the node.

`normalizeData` does, in order:

1. `normalize(environment.store, entrypoint.normalizationAst, response, variables)`, and `markChanged` of its changes (change 2).
2. `settle(environment.graph)` (change 4).
3. Holds the new retention's root (Computing the new root).
4. Adds the retention to `environment.retainedQueries`, after the root hold returns, so a throw leaves nothing retained that the caller holds no handle to.
5. `notify(environment.graph)` (change 4).

A resolver entry is live while a held root or an observed location reaches it through `Node` dependencies. Several retentions of one entrypoint with equal variables have one `location` and one node, with one root hold each; a resolver shared by several queries is one node reached from several roots. The reference count of the user's design is implicit: a node stays live while any held root or observed location reaches it, and `dispose` of one retention leaves it live while another does. An explicit count would be computed from the root holds, the observers, and `dependents` (change 2); open decision 11.

### Computing the new root

Depends on the held architecture.

```ts
// from libs/isograph-runtime/src/environment.ts
// in normalizeData, after normalize and markChanged (and change 4's settle), with `artifact` the
// entrypoint's `readerArtifact()` and `ast` its `normalizationAst`:
const location: EntryLocation = {
  record: ast.root,
  storeKey: resolverStoreKey(artifact.selectableName, variables),
};
const [, releaseRoot] = retainRoot(
  environment.graph,
  resolverNode(location.record, location.storeKey),
  () => resolverDefinition(environment.store, artifact, ast.root, variables),
);
```

`ast.root` is present: `normalize` inserts it before writing anything. The entries the new retention reaches are computed here, children before parents: the root's computation reads each `Resolver` node through `readNode`, which computes the child, or brings it up to date, first. This is the bottom-up order the user's design creates them in on the way back up the response (Resolver markers, below). A node already stored and `Clean` is read as is; one that is `Stale` recomputes. A `normalizeData` of an already retained entrypoint and variables whose response changed nothing calls no resolver.

### Unobserved results are lazy

Depends on the held architecture.

After the `normalizeData` that retains a query, a later write recomputes its nodes only when an observer needs them (change 4). Until change 4 there are no observers, so a later write marks a retained query's affected nodes `Stale`, and the next read of them recomputes them: a `readStoredResult`, or the root hold of the next `normalizeData` of the same entrypoint and variables. Retention keeps records and nodes alive for `garbageCollectEnvironment`; it does not keep nodes current. Open decision 3.

### Garbage collection

Depends on the held architecture in its node part.

```ts
// from libs/isograph-runtime/src/environment.ts
// garbageCollectEnvironment, before (change 2):
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteInput(environment.graph, recordInput(record)),
  );
// after:
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteInput(environment.graph, recordInput(record)),
  );
  collect(environment.graph);
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** Disposes every node that no node with `rootCount > 0` reaches through `Node` dependencies. */
export function collect<TValue>(graph: RecomputationGraph<TValue>): void;
```

- Records: store.md's mark and sweep, from each member of `environment.retainedQueries`, with `garbageCollect`'s parameters unchanged. A node a retention reaches never loses a record: it reads only what its reader ASTs select along current links, and the retained query's normalization AST is the merge of those reader ASTs with the same variables (query-generation.md), so every record it reads is marked. A node reached only from an observed location can lose its records (Level 2). Each deleted record goes through `deleteInput`, which disposes its nodes (change 2).
- Nodes: nodes that held roots and observed locations reach through `Node` dependencies are kept; every other node is disposed by `disposeNode`, so no `dependents` set or `inputDependents` set keeps it. A disposed node has no kept dependent: a kept node that read it would make it reachable. Nodes a plain `readStoredResult` stored outside any retention, and those of a disposed retention, are collected this way, `Stale` or not.

### Resolver markers

The user's design has `normalize` walk the response to the leaves and, on the way back up, create the resolver subscriptions each record needs. That needs `normalize` to know which resolvers the query's readers select at each position. The normalization AST does not say: query generation inlines a client field's selections and keeps no trace of it (query-generation.md, Merging). Two ways:

- (a) Root only (held). `normalizeData` knows the entrypoint's reader artifact (`Entrypoint.readerArtifact`), and holding the root computes the root entry; computing it creates every entry the query reaches, children first, by the same recursion that orders recalculation. Nothing new in any artifact.
- (b) A marker per position. Each `NormalizationSelectionSet` gains the client fields the readers select on that record, `{ kind: "Resolver", readerArtifact, arguments }`, with `readerArtifact` imported and `arguments` in the operation's variables with defaults applied (what query generation substituted when it inlined the field), listed children first; `normalize` reports each one when it leaves the record and `normalizeData` creates its node. This is candidate 6 of `refactors/pending/runtime/normalization-ast-mount-markers.md` ("A marker is needed only if a subscription must be created for a record the response introduces").

Reason for (a): every marker in (b) is already reachable from the root through reader ASTs, so (b) adds no entry (a) does not create, and creates them in the same order. (b) also goes stale: a later response that moves `Pet:1.bestFriend` from `Pet:2` to `Pet:3` makes `FriendLabel` read `PetName` on `Pet:3`, where no marker of `FriendPage`'s response ran, so the live set has to follow dependencies anyway. And `normalization_ast.ts` would import reader artifacts, and through them the user's source modules, so loading a query's normalization AST would load its resolvers. Open decision 1.

### Tests

`test/level1.test.ts`, over the fixture of change 2, through `normalizeData` with the fixture's entrypoint artifacts, over an environment built with `createIsographEnvironment(new Map(), deferredNetwork().networkFunction)`:

- After `normalizeData` of `BadgePage`, every resolver it reaches was called once and no `readStoredResult` was made; `readStoredResult` of `BadgePage` then calls nothing and returns the stored result.
- Retained, not read: after `normalizeData` of `BadgePage` with `{ id: "1" }`, a `PetDetail` response that changes `Pet:1.name` calls none of `BadgePage`'s resolvers; the next `readStoredResult` of `BadgePage` calls `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` once each.
- A second `normalizeData` of `BadgePage` with `{ id: "1" }` whose response changes `name`: inside it, the root hold calls each of the five once; a third with the same response calls none.
- Two retentions of `BadgePage` with `{ id: "1" }`: `dispose` of one, then `garbageCollectEnvironment`, keeps `Pet:1` in `environment.store` and its nodes in `environment.graph.nodes`; `dispose` of both, then `garbageCollectEnvironment`, leaves `environment.store` `toEqual(new Map())` and `environment.graph.nodes` empty.
- `FriendPage` retained, then a `PetDetail` response for `Pet:1` whose `bestFriend` is `Pet:3` (`PetDetail` selects no `nickname` under `bestFriend`): the next `readStoredResult` of `FriendPage` recomputes `FriendLabel`, which reads `PetName` on `Pet:3`, and returns `MissingData` naming `Pet:3`; `garbageCollectEnvironment` keeps `Pet:3`, which `FriendPage`'s AST reaches through the new link.
- A throwing resolver in one retained query: `normalizeData` returns, that query's root holds `ResolverThrew`, and another retained query's stored results are unchanged.
- A node stored by a plain `readStoredResult` outside any retention is disposed by `garbageCollectEnvironment`; one under a held root is kept.
- `dispose` of `FriendPage`'s retention, then a change to `Pet:2.name` before `garbageCollectEnvironment`: no resolver is called.
- store.md's GC tests and network-and-subscriptions.md's tests of `garbageCollectEnvironment` are unchanged: the first call `garbageCollect` with a retained list and compare the store with `storeOf`; the second retain through `retainQuery` and compare `environment.store`.

## 4. Level 2 and the height-ordered push

Depends on the held architecture, except `listen`, `snapshot`, and `Observed`.

This change adds observers, and with them needed entries: an entry is needed when an observer is at its location or a needed entry reads it. A write recomputes the needed entries whose inputs changed inside `normalize`, lowest height first, and marks every other entry it reaches `Stale`, as change 2 does.

### Heights

The compiler writes each reader artifact's height. Its reader is the bucket queue (Settlement), so it lands in this change.

```rust
// from crates/isograph_compiler/src/reader_ast.rs
/// A reader artifact's height: 0 when its reader AST has no `Resolver` node, else one more
/// than the largest height of the client fields its `Resolver` nodes select.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReaderHeight(pub u32);

impl ReaderHeight {
    pub(crate) fn above(self) -> ReaderHeight {
        ReaderHeight(self.0.saturating_add(1))
    }
}

/// The client fields of `selection_set`'s `Resolver` nodes, at any depth, in selection order.
pub(crate) fn resolver_fields(
    selection_set: &ReaderSelectionSet,
) -> Box<dyn Iterator<Item = EntityNameAndSelectableName> + '_> {
    selection_set
        .0
        .iter()
        .flat_map(
            |selection| -> Box<dyn Iterator<Item = EntityNameAndSelectableName> + '_> {
                match selection {
                    ReaderSelection::Scalar(_) => std::iter::empty().boxed(),
                    ReaderSelection::Linked(linked) => resolver_fields(linked.selections.reference()),
                    ReaderSelection::Refinement(refinement) => {
                        resolver_fields(refinement.selections.reference())
                    }
                    ReaderSelection::Resolver(resolver) => std::iter::once(resolver.field).boxed(),
                }
            },
        )
        .boxed()
}
```

The iterator is boxed because it recurses into nested selection sets.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// The height of `field`'s reader artifact.
#[memo]
pub fn reader_height<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
) -> Result<ReaderHeight, GenerationError> {
    let key = SelectableKey {
        entity: field.parent_entity_name,
        selectable: field.selectable_name,
    };
    if let Some(cycle) = inline_cycle(db, client, key) {
        return GenerationError::InlineCycle(cycle).wrap_err();
    }
    let types = field_types(db, client, field)
        .as_ref()
        .map_err(GenerationError::clone)?;
    resolver_fields(types.reader_ast.reference())
        .map(|selected| {
            reader_height(db, client, selected)
                .clone()
                .map(ReaderHeight::above)
        })
        .try_fold(ReaderHeight::default(), |highest, height| {
            height.map(|height| highest.max(height))
        })
}
```

`inline_cycle` (`merged_selection.rs`) becomes `pub(crate)`. `GenerationError` gains:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
    #[error("These fields inline each other: {}.", .0.display())]
    InlineCycle(InlineCycle),
```

The guard comes first because pico panics on a memo cycle (pico.md). `inline_cycle` returns the first cycle reachable from `key` in the graph of `inlined_fields`, whether or not it contains `key`. When `field_types` succeeds, every `Resolver` node of its reader AST selects a field of `inlined_fields(entity, selectable)`: both walk the declaration's selection set, recurse into server selections with a nested selection set, list client field selections, and skip client pointers, and a failed lookup fails `field_types`. So the recursion runs on a subgraph of a graph `inline_cycle` found acyclic, and terminates. A field reachable from an entrypoint whose query compiled has no reachable cycle; for one that has, `reader_height` returns the cycle `field_selection_map` also reports, `project_artifacts` returns it, and no artifact is written (reader-ast.md relies on the same rule: a failed query writes no artifacts).

The height rides with the reader AST, because it exists exactly when a reader artifact is generated:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// What a field's `resolver_reader.ts` holds besides its resolver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolverReader {
    pub reader_ast: ReaderSelectionSet,
    pub height: ReaderHeight,
}

// FieldArtifacts, before:
    /// The reader AST, when the field is reachable from an entrypoint.
    pub resolver_reader: Option<ReaderSelectionSet>,
// after:
    /// The reader, when the field is reachable from an entrypoint.
    pub resolver_reader: Option<ResolverReader>,
```

```rust
// from crates/isograph_compiler/src/artifact_types.rs
// project_artifacts, before:
                resolver_reader: reachable.contains(field.reference()).then_some(types.reader_ast),
// after:
                resolver_reader: reachable
                    .contains(field.reference())
                    .then(|| {
                        reader_height(db, client, field)
                            .clone()
                            .map(|height| ResolverReader {
                                reader_ast: types.reader_ast,
                                height,
                            })
                    })
                    .transpose()?,
```

`isograph_compiler`'s `lib.rs` exports `ReaderHeight`, `ResolverReader`, and `reader_height`.

The printer writes `height` after change 2's `selectableName`:

```rust
// from crates/typescript_artifacts/src/reader_artifact.rs
// reader_artifact_file, before:
    reader: &ReaderSelectionSet,
    // ..
    let value = ArtifactValue::Object(vec![
        entry("readerAst", selection_set_value(reader)),
        entry(
            "resolver",
            // .. unchanged
        ),
    ]);
// after:
    reader: &ResolverReader,
    // ..
    let value = ArtifactValue::Object(vec![
        entry("selectableName", string(field.field.selectable_name)),
        entry("height", ArtifactValue::Number(f64::from(reader.height.0))),
        entry("readerAst", selection_set_value(reader.reader_ast.reference())),
        entry(
            "resolver",
            // .. unchanged
        ),
    ]);
```

`f64::from(u32)` is exact. The caller in `typescript_artifacts/src/lib.rs` passes `field.resolver_reader.as_ref()` unchanged, now a `&ResolverReader`. Test constructors that build `resolver_reader: ReaderSelectionSet(..).wrap_some()` (`reader_artifact.rs` `field_in` and `printed`, `lib.rs`) build `ResolverReader { reader_ast: ReaderSelectionSet(..), height: ReaderHeight::default() }` instead.

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<TParam, TOutput> = {
  /** The client field's selectable name: the prefix of its resolver store key. */
  readonly selectableName: string;
  /** 0 when `readerAst` has no `Resolver` node, else one more than the largest `height` of the
   * reader artifacts its `Resolver` nodes import. */
  readonly height: number;
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
```

Delta from the landed type: it has a third parameter, `TEntityName` (reader-ast.md), which this doc's snippets omit, as change 2's do. `Pet/fullName/resolver_reader.ts` gains `height: 0,` after `selectableName: "fullName",`. The runtime reads `artifact.height` and never computes one.

### Needed nodes

```ts
// from libs/isograph-runtime/src/graph.ts
export type GraphNode<TValue> = {
  // .. change 2 and 3's fields, and:
  /** Every node this one reads has a lower height. `NodeDefinition.height`, copied at creation. */
  readonly height: number;
  /** The callbacks `observe` holds at this node plus the dependents whose `neededCount` is
   * above 0. The node is needed when it is above 0. */
  neededCount: number;
};

/** What creating an absent node needs. */
export type NodeDefinition<TValue> = {
  readonly owner: InputId;
  /** For a resolver entry, the reader artifact's `height`. */
  readonly height: number;
  readonly computation: Computation<TValue>;
};

/**
 * `Clean`: `value` is what a computation against the store would return.
 * `Queued`: needed, and an input it read changed; it is in `graph.queue[height]`.
 * `Stale`: not needed, and an input it read changed or a node it reads is `Stale`. It recomputes when read.
 */
export type NodeState = 'Clean' | 'Queued' | 'Stale';
```

`resolverDefinition` (change 2) adds `height: artifact.height`.

```ts
// from libs/isograph-runtime/src/graph.ts
/** Adds one to `node.neededCount`; when it was 0, adds one to that of each node it reads. */
function addNeed<TValue>(node: GraphNode<TValue>): void {
  node.neededCount += 1;
  if (node.neededCount === 1) {
    nodeDependencies(node).forEach(addNeed);
  }
}

/** Subtracts one from `node.neededCount`; when it reaches 0, subtracts one from that of each node it reads. */
function removeNeed<TValue>(node: GraphNode<TValue>): void {
  node.neededCount -= 1;
  if (node.neededCount === 0) {
    nodeDependencies(node).forEach(removeNeed);
  }
}

/** The distinct nodes of `node`'s `Node` dependencies. */
function nodeDependencies<TValue>(node: GraphNode<TValue>): Set<GraphNode<TValue>>;
```

Writers of `neededCount`:

- `createNode` starts it, and `rootCount`, at the number of callbacks `graph.observers` holds at its id, before `setDependencies`, so an observed node created by a computation needs what it reads.
- `setDependencies`, when `node.neededCount > 0`, calls `addNeed` on each distinct `Node` dependency of `next` that the old list does not hold, and `removeNeed` on each distinct `Node` dependency of the old list that `next` does not hold. Distinct, because `dependents` is a set: a dependency recorded twice is one edge.
- `observe` and its stop function (`listen` and `snapshot`, below).
- Disposing a node calls `setDependencies(graph, node, [])`, which removes its need from what it read when it was needed.

Invariants, outside a call that is computing a node:

- `neededCount` is the callbacks at the node's id plus the number of its dependents whose `neededCount` is above 0.
- Every dependent of a `Stale` node is `Stale` (change 2). So every node a `Clean` node reads is `Clean`, and so is everything below it.
- No needed node is `Stale`, and every `Queued` node is needed.
- When `normalizeData` and `garbageCollectEnvironment` return, no node is `Queued`.

Why a kept count. A write decides "needed?" for every node it invalidates, and the answer must cost `O(1)`. Computing it on demand walks the node's dependents upward until an observed node, so each decision costs the node's ancestors, on every write. Marking the needed region from every observer down before each write costs the whole observed region per write, the `O(L)` the push exists to avoid. The count costs work only where neededness changes: `observe` and its stop function walk the region that becomes needed or stops being needed, and `setDependencies` of a needed node walks the dependencies it gained or lost. Counting is exact because the graph has no cycle (Ordering). The count is the observed count R10 and Follow-ups 9 ask for. Open decision 16.

### Propagation

```ts
// from libs/isograph-runtime/src/graph.ts
/** An input of `node` changed. A `Clean` needed node is queued at its height. A `Clean` node
 * nobody needs is marked `Stale` with its dependents. A `Queued` or `Stale` node already
 * recomputes before it is read. */
function invalidate<TValue>(graph: RecomputationGraph<TValue>, node: GraphNode<TValue>): void {
  if (node.state !== 'Clean') {
    return;
  }
  if (node.neededCount > 0) {
    node.state = 'Queued';
    bucket(graph, node.height).add(node);
  } else {
    markStale(node);
  }
}

/** `graph.queue[height]`, after appending an empty set for every missing index up to it. */
function bucket<TValue>(graph: RecomputationGraph<TValue>, height: number): Set<GraphNode<TValue>>;
```

```ts
// from libs/isograph-runtime/src/graph.ts
// markChanged, before (change 2):
  inputs.forEach((input) => graph.inputDependents.get(input)?.forEach(markStale));
// after:
  inputs.forEach((input) =>
    graph.inputDependents.get(input)?.forEach((node) => invalidate(graph, node)),
  );

// deleteInput, before (change 2):
  owned?.forEach((node) => node.dependents.forEach(markStale));
// after:
  owned?.forEach((node) => node.dependents.forEach((dependent) => invalidate(graph, dependent)));
```

`markStale` keeps change 2's body. It reaches only nodes nobody needs: a dependent of a node nobody needs is itself not needed, so it never meets a `Queued` node. `disposeNode` also removes a `Queued` node from its bucket.

```ts
// from libs/isograph-runtime/src/graph.ts
// recompute, before (change 2):
  if (!graph.valuesEqual(node.value, computed.value)) {
    node.value = computed.value;
  }
// after:
  if (!graph.valuesEqual(node.value, computed.value)) {
    node.value = computed.value;
    node.dependents.forEach((dependent) => invalidate(graph, dependent));
  }

// bringUpToDate, before (change 2):
  if (node.state === 'Clean') {
    return;
  }
  recompute(graph, node);
  node.state = 'Clean';
// after:
  if (node.state === 'Clean') {
    return;
  }
  if (node.state === 'Queued') {
    bucket(graph, node.height).delete(node);
  }
  recompute(graph, node);
  node.state = 'Clean';
```

The node keeps its state while its computation runs, so a node its read recomputes to a new value does not invalidate it again; it reads the new value and ends `Clean`. On the read path the new `forEach` changes nothing, because a recomputed `Stale` node's dependents are `Stale` already; on the push it queues the needed dependents and marks the others `Stale`.

### Settlement

```ts
// from libs/isograph-runtime/src/graph.ts
export type RecomputationGraph<TValue> = {
  // .. change 2's fields, and:
  /** Index `h` holds every `Queued` node of height `h`. No index is a hole. */
  readonly queue: Array<Set<GraphNode<TValue>>>;
  readonly observers: Map<NodeId, Observed<TValue>>;
};

/** Recomputes every `Queued` node, lowest height first, until the queue is empty. */
export function settle<TValue>(graph: RecomputationGraph<TValue>): void {
  for (const nodes of graph.queue) {
    for (const node of nodes) {
      bringUpToDate(graph, node);
    }
  }
}
```

`createGraph` starts `queue` and `observers` empty. A `for..of` over an array reads its length at each step, and one over a set visits members added during the walk and tolerates deleting the current one, so `settle` reaches buckets appended while it runs. By Ordering, a bucket gains no node once its drain has started, so each bucket is empty when the walk leaves it. Buckets stay in the array, empty; its length is one more than the largest height ever queued. The loop is a state machine over a queue that grows during the walk, which the iterator combinators do not express.

`normalizeData` runs `settle` after `normalize` and `markChanged` and before the root hold (change 3). The root hold queues nothing: every node it recomputes is `Stale` or new, and the dependents of a `Stale` node are `Stale`. So the queue is empty when `notify` runs. `garbageCollectEnvironment` runs `settle` and then `notify` after `collect`: a needed node that read a deleted record was queued by `deleteInput`'s invalidation and recomputes to `MissingData`, and a node disposed while `Queued` left its bucket when it was disposed.

```ts
// from libs/isograph-runtime/src/environment.ts
// garbageCollectEnvironment, after:
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteInput(environment.graph, recordInput(record)),
  );
  collect(environment.graph);
  settle(environment.graph);
  notify(environment.graph);
```

Work per write: the needed nodes that read a changed input recompute; the nodes nobody needs that read one are marked `Stale` with their dependents, once; each recomputed node whose value changed invalidates its direct dependents; and `settle` takes one step per bucket. Nothing is visited whose inputs are unchanged. The traces are in Candidate 3a.

A scheduler (Follow-ups 9) leaves the queue as is; deferral marks instead of queuing.

### `listen` and `snapshot`

```ts
// from libs/isograph-runtime/src/graph.ts
/** The observers of one node. */
export type Observed<TValue> = {
  readonly callbacks: Set<() => void>;
  /** The value the callbacks were last called for, or were registered at. */
  notified: TValue | undefined;
};

/** Adds `onChange` at `id`; when the node exists, brings it up to date, then adds one to its
 * `rootCount` and its `neededCount`. Returns the function that removes all three. */
export function observe<TValue>(
  graph: RecomputationGraph<TValue>,
  id: NodeId,
  onChange: () => void,
): CleanupFn;

/** The node's value as of its last computation, or `undefined` when no node is stored. Computes nothing. */
export function valueOf<TValue>(graph: RecomputationGraph<TValue>, id: NodeId): TValue | undefined;

/** Calls each observer whose node's value is not `===` its `notified`; open decision 9's error rule. */
export function notify<TValue>(graph: RecomputationGraph<TValue>): void;
```

```ts
// from libs/isograph-runtime/src/observe.ts
/** Calls `onChange` after each settlement that changed the result stored at `location`, and
 * makes that result needed. Returns the function that stops it. */
export function listen(
  environment: IsographEnvironment,
  location: EntryLocation,
  onChange: () => void,
): CleanupFn {
  return observe(environment.graph, resolverNode(location.record, location.storeKey), onChange);
}

/** The result stored at `location` as of its last computation, or `undefined` when none is
 * stored. Computes nothing. */
export function snapshot(
  environment: IsographEnvironment,
  location: EntryLocation,
): ResolverResult<unknown> | undefined {
  return valueOf(environment.graph, resolverNode(location.record, location.storeKey));
}
```

`observe`:

1. Adds `onChange` to the `Observed` at `id` in `graph.observers`, creating it.
2. When a node is stored at `id`: `bringUpToDate` on it, then adds one to `rootCount` and calls `addNeed`. Bringing it up to date first leaves it `Clean`, and so every node below it (Needed nodes, the invariants), so the region that becomes needed holds no `Stale` node. The nodes it recomputes were `Stale`, and their dependents are `Stale`, so nothing is queued and no other observer's value changes.
3. Sets `notified` to the node's value.

The returned function removes the callback, subtracts one from `rootCount` and calls `removeNeed` on the node stored at `id` when there is one, and removes the `Observed` when it has no callback left. When no node is stored at `id`, `observe` only registers, and `createNode` starts the node's `rootCount` and `neededCount` at the callbacks there.

A location that is `Stale` when a component first renders: `snapshot` returns the last computed result (R1: it computes nothing). `listen`, in React's subscribe, brings it up to date, and `useSyncExternalStore` compares the snapshot after subscribing and renders again with the current result. Open decision 3.

Notification, after settlement in `normalizeData` and `garbageCollectEnvironment`: for each `Observed` in `graph.observers` in insertion order, the node's value is compared by identity with `notified`; when different, `notified` is set and each callback is called once. Identity suffices because `recompute` keeps the value object when `valuesEqual` holds. An observed resolver that returns a new object on every recomputation notifies on every recomputation (R4); a stable identity for it would come from upstream's merge of the data it reads, not from comparing its output. Callbacks that throw do not stop the loop; after it, one error is rethrown as is and several as an `AggregateError` (open decision 9). The loop is `O(observers)`; a list of the observed nodes `recompute` changed would make it `O(changed)`, and is not needed while observers are the mounted components of one screen.

What each level owns:

- Level 1 holds values, dependencies, dependents, states, root holds, needed counts, and the queue. It runs user code (resolvers).
- Level 2 holds the observed ids, callbacks, and the last-notified identity. It runs no resolver, and is notified only after level 1 settled, once per changed node per settlement.

A parent component never re-renders with a stale child: every needed node is `Clean` before the first callback, and every node a mounted component observes is needed, so any `snapshot` during a re-render a callback caused returns the settled value, whatever the order callbacks run in. React additionally batches the `setState`s that `useSyncExternalStore` calls in one notification loop into one render.

What React needs, and gets without `@isograph/runtime` knowing React: `useSyncExternalStore(subscribe, getSnapshot)` where `subscribe(onStoreChange)` returns an unsubscribe function (`listen`), and `getSnapshot` returns the same value while nothing changed and is pure and cheap (`snapshot`, a lookup). How a component learns its location (the entry a parent's result names) is the React package's, with `@component` (Later).

An observer keeps its node current (it makes the node needed, so a write that reaches it recomputes it) and keeps the node through `collect` (its `rootCount`), but retains no record: data stays only while a retention keeps it. A location whose record `garbageCollectEnvironment` deleted has no stored node, and `snapshot` returns `undefined`.

### Tests

New cases in the compiler:

- `crates/graphql_network_protocol/src/tests/reader_ast.rs`, with change 2's fixture sources: `reader_height` of `Pet.Initial` is `ReaderHeight(0)`, of `Pet.Monogram` and `Pet.Signature` `ReaderHeight(1)`, of `Pet.Badge` `ReaderHeight(2)`, of `Query.BadgePage` `ReaderHeight(3)` (its `Resolver` node is under `pet`), of `Pet.FriendLabel` `ReaderHeight(1)` (under `bestFriend`), and of a field whose only client selection is a client pointer `ReaderHeight(0)`.
- The same file, with `field Pet.A { B }` and `field Pet.B { A }`: `reader_height` of `Pet.A` is `Err(GenerationError::InlineCycle(..))` naming `Pet.A` and `Pet.B`, and returns (no pico cycle panic).
- `crates/typescript_artifacts`: a reader artifact whose `ResolverReader` has `ReaderHeight(2)` prints `height: 2` after `selectableName`.

`test/graph.test.ts` gains, with literal ids and heights: two needed nodes of heights 0 and 1 that both read input `a`, the second also reading the first: `markChanged([a])` then `settle` runs the height-0 computation before the height-1 one, each once. A node nobody needs that read `a` is `Stale` after `markChanged([a])` and is not recomputed by `settle`. `observe` of an absent id registers only; the node created later starts with `rootCount` and `neededCount` 1.

`test/level2.test.ts`, over an environment built with `createIsographEnvironment(new Map(), deferredNetwork().networkFunction)`:

- `listen` then an unchanged `normalizeData`: no call. A changed one: one call, after which `snapshot` returns the new result; inside the callback, `snapshot` already returns it and every resolver's call count is final.
- Diamond, an observer at `BadgePage`, `Rex` to `Max` through a `PetDetail` response: inside that `normalizeData`, `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once, `Initial` before `Monogram` and `Signature`, both before `Badge`, `Badge` before `BadgePage`; `Badge` receives `{ Monogram: "M.", Signature: "M " }`; the observer is called once.
- Diamond, an observer at `BadgePage`, `Rex` to `Ray`: `Initial` is called once and no other resolver is called; the observer is not called.
- Diamond, an observer at `BadgePage`, only `nickname` changes: `Signature`, `Badge`, `BadgePage` once each; `Initial` and `Monogram` zero.
- New object, an observer at `ShapePage`, `Rex` to `Ray`: `NameShape` and `ShapeLabel` are called once each, `ShapePage` zero times, and the observer is not called. An observer at `NameShape` as well is called once, because its new `{ length: 3 }` is not `===` the old one.
- Two observers, at `FriendPage` and at `PetSummary` with `{ id: "2" }`: one write to `Pet:2.name` calls `PetName` on `Pet:2` once, `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` once each, `RecentCheckins` zero, and each observer once.
- One of two paths needed: an observer at `FriendPage`, `PetSummary` with `{ id: "2" }` retained and not observed; `Pet:2.name` changes: `PetName` on `Pet:2`, `FriendLabel`, `FriendCard`, and `FriendPage` are called once each inside the `normalizeData`, and `PetSummary` zero times; the next `readStoredResult` of `PetSummary` calls `PetSummary` once and `PetName` zero times.
- Retained, not observed: `BadgePage` retained, `Rex` to `Max` through a `PetDetail` response: none of `BadgePage`'s resolvers is called inside that `normalizeData`, and `snapshot` at `BadgePage` returns the previous result; `listen` at `BadgePage` then calls each of the five once and calls no callback, and `snapshot` returns the new result.
- The stop function: after the only observer at `BadgePage` stops, `Rex` to `Max` calls no resolver; `snapshot` still works.
- Link move under an observer at `FriendPage`: `Pet:1.bestFriend` moves to `Pet:3` (with `name` and `nickname`): inside that `normalizeData`, `FriendLabel` is called once and `PetName` on `Pet:3` once. Then `Pet:2.name` changes: no resolver is called. Then `Pet:3.name` changes: `PetName` on `Pet:3`, `FriendLabel`, `FriendCard`, and `FriendPage` are called once each.
- An observer at `FriendPage` whose retention was disposed: `garbageCollectEnvironment` deletes the records, calls the observer once inside it, `snapshot` returns `undefined`, and `environment.store` is `toEqual(new Map())`.
- A callback that calls `normalizeData` with another change: the nested call notifies; each observer receives each result once.
- A callback that throws: the other observers are still called, then `normalizeData` throws it; two that throw give an `AggregateError` of both.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: "`pico-runtime.md` stays parked" is replaced by this doc as the step after v1. `Store` is unchanged, and its functions keep their parameters and its tests hold. `normalize` writes through `writeValue`, skipping a value unchanged by its AST shape, returns `ReadonlyArray<StoreChange>`, and throws `PartialWriteError` (change 1); `garbageCollect` returns the links of the records it deleted (change 2).
- `refactors/pending/runtime/network-and-subscriptions.md`: `IsographEnvironment` gains `graph`, which `createIsographEnvironment` creates (change 2). `environment.ts` gains `normalizeData` (change 2), which takes the entrypoint and returns a `Retention` that `environment.retainedQueries` also holds (change 3), and settles and notifies (change 4); `garbageCollectEnvironment` deletes the nodes of deleted records (change 2), collects unreached nodes (change 3), and settles and notifies (change 4). `makeNetworkRequest` calls `normalizeData` where it called `normalize` (change 2), and from change 3 holds the returned `Retention` and disposes it in its cleanup. Its Why the store-only functions stay store-only holds: the functions that need the graph take the environment. Its change 7 is the only change that makes `Store` an object; this doc adds no field to `Store`.
- `refactors/pending/graphql-protocol/normalization-ast.md`: `MergedSelectionTarget::Leaf`, `CompletedSelectionTarget::Leaf`, and `NormalizationTarget::Leaf` carry a `ListDepth`; the TypeScript `NormalizationSelection` becomes `NormalizationLeafSelection | NormalizationCompositeSelection`, and a leaf selection prints `listDepth` after `target` (change 1).
- `refactors/pending/runtime/reader-ast.md`: `ReaderArtifact.selectableName` and its printing; the walk takes `ReadHooks<TFailure>` and `readButDoNotEvaluate` passes v1's hooks, so `read` and `readButDoNotEvaluate` keep their behavior and its tests hold unchanged; `ResolverResult` adds `ReadResolverThrew` to `ReadResult` for stored results, and `ReadResult` is unchanged (change 2). `ReaderArtifact.height`, printed after `selectableName`; `FieldArtifacts.resolver_reader` becomes `Option<ResolverReader>`, the reader AST with its `ReaderHeight`; the `reader_height` memo and `GenerationError::InlineCycle` (change 4). Its Client pointers, Later, note on reader artifact imports holds for imports and not for heights: an eager pointer that reads client fields on its target breaks static heights (open decision 14). Its note that this doc's `computeResolver` and `readSelection` replace its `read` is superseded: its `read` stays, and `readStoredResult` is the stored read beside it (change 2).
- `refactors/pending/runtime/normalization-ast-mount-markers.md`: candidate 6 is answered by Resolver markers (held: no marker); its open question on how this doc models a mount is answered by `Retention` and settlement.

## Other node kinds

The engine is general over node kinds: a caller supplies each node's id, owner, height, and computation, and the engine never inspects a value except through `valuesEqual`. Resolver entries are the only kind changes 1 to 4 create. Changes 1 to 4 keep the API in that shape, so that the kinds below are additions and not a redesign:

- `graph.ts` imports nothing from the store, the reader, or the environment, and names no resolver, record, or artifact.
- A node's computation, owner, and id come from its creator (`NodeDefinition`, `track`, `readNode`, `retainRoot`); no engine function builds one.
- The graph's value type is a parameter, `TValue`. A second kind widens the environment's graph to a union of the kinds' values, and `valuesEqual` to a function that compares two values of one kind and answers unequal for two kinds.
- Heights are the caller's: `NodeDefinition.height` is a `number` in change 4, because every kind changes 1 to 4 create has a height the compiler wrote, and the stored `check` below is height 0. A kind without a static height is Follow-ups 10.

### A stored `check` (planned, after change 4)

`check(store, ast, variables)` (store.md) as one node of the graph per (entrypoint, variables, root).

Inputs. `check` reads presence, not values: whether a record is present, whether a key of a present record is present, and the value of a key only when it is a link, because a link decides which record is checked next. A scalar's value changing does not reach the node; a record appearing or disappearing, a key appearing, or a link changing does.

```ts
// from libs/isograph-runtime/src/inputs.ts
/** The input a presence check of `storeKey` on a present `record` reports. It changes when the key
 * goes from absent to present; `normalize` never removes a key, and a deleted record reports `recordInput`. */
export function keyInput(record: StoreLink, storeKey: StoreKey): InputId {
  return JSON.stringify(['Key', record.entityName, record.id, storeKey]) as InputId;
}
```

The node reports `recordInput` for each record it looks up, `keyInput` for each key it checks, and `valueInput` for each composite key whose links it follows. `check`'s walk takes the reporting half of `ReadHooks` (`readRecord`, and a `readKey` beside `readValue`); v1's `check` passes hooks that report nothing. The store reports presence changes separately from value changes: `normalize`'s `ValueChanged` splits into `KeyAdded` (the record did not hold the key) and `ValueChanged` (it held a value `isUnchanged` rejected), and `changeInput` maps `KeyAdded` to both `keyInput` and `valueInput` of the key and `ValueChanged` to `valueInput` alone. `garbageCollect` reports deleted records, whose `recordInput` covers every key on them. Changes 1 to 4 keep one `ValueChanged`, because no node they create reads a `keyInput`.

- Value: `CheckResult`, `EnoughData` or `MissingData { record }`. `valuesEqual` compares `kind` and, for `MissingData`, the record's `entityName` and `id`, so a recomputation with the same answer keeps the old object, `===`.
- Height 0: it reads only inputs, never another node.
- Owner: `recordInput(ast.root)`. Id: the JSON of `['Check', queryText, stableCopy(variables), ast.root.entityName, ast.root.id]`.
- No intermediate nodes: the node is the whole walk, and nothing records that a branch of the query is fully present.

What a write costs: nothing when it changes only scalar values, which is the common case. When a key or record appears or disappears or a link changes among the inputs the last walk recorded, the node recomputes, which walks the query once, `O(records and keys the query reaches)`, as v1's `check` does on every call today.

It makes `shouldFetch: 'IfNecessary'` (network-and-subscriptions.md, `fetchEntrypoint`, which calls `check` on every render with that option) a lookup when no presence or link input changed since the last check: one `track` of a `Clean` node, instead of a walk of the whole AST. Observing it gives a suspended render an exact signal: the observer is called when the value goes from `MissingData` to `EnoughData`, and not on a write that leaves the query missing elsewhere. Upstream suspends on `onNextChangeToRecord(environment, response.recordLink)` (`read.ts:117-122`, `cache.ts:135-149`), which resumes on any change to the one record the read stopped at, so a write that fills that record and leaves another missing resumes a render that suspends again, and a write that fills another missing record first does not resume it.

Later, and only if presence changes become frequent for large retained queries (pagination appending items to a large retained list re-walks the whole query on every page): a node per (record, selection set position) that checks that record's keys and reads, through `readNode`, the child node of each link target it follows, with heights from the AST depth (a selection set's height is one more than its deepest target selection set's) and cutoff when a child's result is unchanged. A write then recomputes the changed record's node and its ancestors up to the first unchanged result. Cost: one intermediate node per (record, selection set position) a retained query reaches, each with its own dependency list, and one `Node` edge per followed link.

### Other kinds that fit

- Garbage collection reachability per retained query: a node per `RetainedQuery` whose value is the set of records its AST reaches (store.md's mark, from that query), with the inputs the stored `check` reports. `garbageCollectEnvironment` would take the union of the retained queries' node values instead of walking every retained AST on every collection, and a retained query whose inputs did not change costs a lookup. Its value is a new set on every recomputation, so it cuts off nothing (R4).
- Closure-component entries (components-in-userland.md): the wrapper is a node at the closure field's resolver store key, whose computation reads only the record's presence (The engine, the store, and the environment, Lifetime still follows records), and a second node per closure entry for its data read is that doc's open question 3. Both are resolver-shaped nodes with compile-time heights.

## Later

- `@component`: a component is a resolver result, stored and settled like any other; a mounted component observes its own entry.
- Client pointers, `@loadable`, refetch queries (reader-ast.md, Later): each adds entries or retentions, and the markers of normalization-ast-mount-markers.md.
- Optimistic writes: a layer of server entries that settlement reads through. What the layered store must report to the recomputation engine is The engine, the store, and the environment, What the environment must report.
- Eager client pointers: a pointer node that reads client fields on its target lets an entry of a field read an entry of the same field on another record, so static heights no longer order the graph. Open decision 14.

## Follow-ups

The user's design notes, recorded for later. None of them is part of Step A or changes 1 to 4, and none is approved; 10 is the user's planned addition. Upstream paths here follow the doc's convention; `isograph-react-disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`, and `isograph-disposable-types/` paths are under `isograph/libs/isograph-disposable-types/src`.

### 1. Constructors and destructors at normalization time

A normalization-time subscription is a constructor that `normalize` runs when a retained query first reaches a resolver (the first computation of its entry), paired with a destructor that runs when the last handle referencing that entry is released. Lifetime follows retain and release (`normalize` and `dispose`), not render.

Upstream ties the lifetime of such things to render and effects: `useDisposableState` creates the item during render and disposes it on unmount (`isograph-react-disposable-state/useDisposableState.ts:16`), and the pagination hooks hold one disposable per page through `useUpdatableDisposableState` (`loadable-hooks/useSkipLimitPagination.ts:96-99`, `loadable-hooks/useConnectionSpecPagination.ts`).

A destructor needs to know when the last reference goes away. Retention (change 3) keeps reference counts implicit (an entry is live while any retention or observed location reaches it). An explicit count per entry is the number of retentions and observers at its location plus the size of its `dependents` (change 2), so constructors and destructors need that count kept current on every change of those three: open decision 11 (b).

### 2. A client pointer is a function plus a follow-up query

A client pointer's resolver runs at normalization time, like any other resolver result in settlement, and returns an id. When the target record lacks the data the selection through the pointer needs (a `check` of the target's normalization AST at that record reports `MissingData`), `normalize` starts a follow-up query for that record (a refetch through `node(id:)`). The follow-up is a retained child of the parent retention: the parent's `dispose` releases it, and so does a later settlement in which the pointer returns a different id. This is candidate 4 of `refactors/pending/runtime/normalization-ast-mount-markers.md`, with the client pointer of its candidate 5 as the thing that starts it.

### 3. A pointer whose target is one of the records it read

A second kind of pointer has a target that is necessarily one of the records its resolver read. For example, it reads `friends { id }` and returns the id of the best friend. When a pointer opts in to this guarantee, query generation spreads the target fragment's selections at every place the pointer read an id it could return, so the parent query selects `friends { id name }` and no follow-up query is needed.

Costs:

- Overfetching: the query fetches `name` for every friend, not only the one the pointer returns.
- The guarantee must be enforced. Either the returned id's type admits only ids the resolver read (a type-level restriction), or `normalize` treats a returned id outside the read set as missing data and falls back to the follow-up query of note 2.

This differs from upstream's eager pointer. Upstream merges the pointer's own reader selections (`friends { id }`) into the parent query, as for any client field read eagerly (`merge_client_object_field` at `isograph_schema/src/create_merged_selection_set.rs:852-900`, called at `:642`), and fetches the selection through the pointer by a refetch through `node(id:)` (`:1135-1233`; `read.ts:859-970`). The two compose: the pointer's reader selections are merged, and the target fragment's selections are spread at the places the reader selections read ids.

### 4. Handles

The reference-counted handle a retain returns is like upstream's `PromiseWrapper` (`PromiseWrapper.ts:20-23`) plus a cleanup function, the pair upstream calls `ItemCleanupPair` (`isograph-disposable-types/index.ts:2`). It is readable during render: reading its state is a lookup and computes nothing, as `snapshot` (change 4).

`@isograph/runtime` stays React-free. The handle exposes its state (pending, ready, or error), the value when ready, and a way to wait for it to leave pending. The React package turns pending into suspending (it throws the wait's promise).

### 5. Deduplication

Two retained queries that reach the same pointer at the same record share one handle and one follow-up query. The key is the record, the selectable, and the variables: the resolver store key on that record (`resolverNode(record, resolverStoreKey(selectableName, variables))`, the key of `graph.nodes` and `graph.observers`). The handle's reference count is the number of retained parents that reach it.

### 6. A destructor and an in-flight follow-up

Upstream cannot cancel a request: `IsographNetworkFunction` is `(operation, variables) => Promise<any>` and takes no abort signal (`IsographEnvironment.ts:103-106`); dispose only stops the response from being normalized (`makeNetworkRequest.ts:235`). A network function that accepts an `AbortSignal` lets the destructor of a follow-up cancel its request.

### 7. Deferral

The user's direction: do not do all the work at construct and destruct time.

- Construction: `normalizeData` computes the result when the constructor runs, and defers setting up subscriptions to a scheduler. The runtime stays free of browser globals because the caller supplies the scheduler: `createIsographEnvironment` takes a function that receives a task and runs it later. The React package passes one that calls `requestIdleCallback` (or an equivalent where that global is absent); tests pass one that queues tasks and runs them when the test says.
- Destruction: garbage collecting on every release is wasteful. A release adds the handle to a buffer, and collection runs later as one batch, through the same scheduler or when the buffer exceeds its size. A handle that is retained again before the batch runs is not destroyed. Upstream's release buffer is the related mechanism: `unretainQuery` pushes the released query onto `gcBuffer` and reports overflow past `gcBufferSize` (`garbageCollection.ts:40-53`), whose default is 10 (`IsographEnvironment.ts:177`, `:205-206`), and overflow triggers collection (`makeNetworkRequest.ts:565-577`). v1 removed the buffer as not needed yet (`refactors/pending/runtime/store.md`, `garbageCollect`).

### 8. Pre-memoized selection sets

To explore. This may not work out, and the user expects it probably does not matter.

A reader's selection set read against a record (`{ firstName lastName }` on `Pet:4`) is itself a stored result, keyed by the record and the selection set. The selection set's part of the key is a stable key the compiler emits, such as a hash of the selection set or an interned id. Two resolvers whose reader ASTs contain the same selection set on the same record read it once. A resolver whose inputs are one stored selection-set result records one `EntryDependency` on that entry instead of one per field.

Savings:

- Repeated field reads: the second resolver that reads `{ firstName lastName }` on `Pet:4` reads the stored result instead of each field.
- Finer-grained dependencies collapse into one: a resolver entry holds one dependency on the selection-set entry instead of one per field, and one dependents edge per reader instead of one per field.

Costs:

- More stored entries: one per selection set per record read, alongside the resolver entries.
- A key per selection set: the compiler emits it into the reader artifact, and two selection sets must get equal keys exactly when they read the same fields with the same arguments.
- Equality of selection-set results: the stored result is a fresh object on each computation, so it needs upstream's `mergeObjectsUsingReaderAst` (`areEqualWithDeepComparison.ts:55-121`) applied to the read result, keeping the old object where the new data is equal, or every recomputation is unequal and invalidates every resolver that depends on it. The merge is well-defined here because a selection set's result is store data shaped by the reader AST; a nested resolver value in it is compared by `===`, as upstream does (`:84-95`).

Relation to changes 2 and 3: the stored selection-set result is an extra level between server values and resolver entries. A resolver entry depends on a selection-set entry, the selection-set entry depends on server values and on the resolver entries of the `Resolver` nodes it contains, and marking and bringing up to date walk that level as they walk a resolver entry. Under held 3a the level needs heights: the compiler would write a selection-set entry's height as one more than the largest height of the fields its `Resolver` nodes select, and a resolver entry's as one more than its selection set's.

### 9. Scheduled recalculation

Partly future work; R10 requires that the architecture allows it. The user wants control over when recalculation runs:

- Unobserved entries are lazy. An entry no observer needs (an unmounted component's result, a retained query nobody renders) is only marked on a write and is recomputed when next read. Changes 2 to 4 build this: it is the `Stale` state and the needed count.
- Deferred work. Some recalculation and re-rendering runs later at lower priority, for example the work caused by a write that came from user input. In the React layer this corresponds to `startTransition` and `useDeferredValue`. `@isograph/runtime` stays React-free and takes the caller-supplied scheduler of note 7; the scheduler decides when each pending pass runs.

What deferral changes in the requirements: R2 becomes "every needed result is current when the pass for its priority finishes; a result nobody needs is current when read". R5 becomes "observers are called once per pass", not once per write.

What deferral needs on top of changes 1 to 4:

- A priority on a write (`normalizeData` called inside a transition) or on an observer (a component rendered with a deferred value).
- A deferred write invalidates as if no entry were needed: `markStale` from each entry that read a changed input, so every entry the write reaches becomes `Stale`, and the observed entries the marking reaches are recorded under the write's priority. This lifts "no needed entry is `Stale`" until the deferred pass runs.
- An urgent write that reaches a needed `Stale` entry queues it, where `invalidate` would otherwise skip a non-`Clean` entry. Its recomputation reads the deferred changes below it too, so the urgent pass shows them early for that entry, as React does when a transition and an urgent update touch the same state.
- A deferred pass brings each recorded observed entry up to date by the read path and notifies its observers. An entry an urgent pass already brought up to date is `Clean`, and the deferred pass reads it as is. A second deferred write before the pass marks on top of the existing marks, so the pass recomputes each entry once for both writes. The read path has no cutoff (open decision 15), so a deferred pass recomputes every `Stale` entry it reads.

Fit with each candidate:

- Held 3a: laziness is built. Deferral is the deferred write's `Stale` marking and the recorded observed entries, plus `invalidate` queuing a needed `Stale` entry; no state is added.
- 3b: marking is already separate from bringing up to date. Laziness is "mark only, and skip `bringUpToDate`, for entries whose observed count is zero". Deferral is "the scheduler chooses which entries of the marked roots each pass brings up to date".
- 2: laziness is native, because nothing is verified until demanded, and deferral is the scheduler choosing when to verify which roots. Each deferred pass still walks every live edge under its roots.
- Plain 3a and 3c: neither has a lazy mode. Skipping an unobserved entry leaves it stale with no record that it is stale, so they need held 3a's `Stale` mark.
- 4: Preact already keeps unsubscribed computeds lazy and defers effects to the end of a batch; priority would be added to its effect queue.
- 5: both work per root, at full cost per pass and with no cutoff below the root.

React constraint: `useSyncExternalStore` performs a store update as blocking even inside a transition, per React's documentation. Deferred re-rendering therefore needs the React package to hold a deferred snapshot per observer and update it inside `startTransition`, or to read through `useDeferredValue`; this is the React package's design, not the runtime's.

### 10. Runtime-defined computations

A memoized computation the runtime or a user defines at run time, which reads other nodes: a derived value over several resolver results, a selector a hook creates. The compiler writes no height for it, so the bucket queue cannot place it.

- (a) A creation-time height: one more than the largest height of the nodes its first computation read. Cost: a later computation can read a node of equal or greater height (a dependency that moved upward), which needs open decision 14 (a)'s runtime adjustment, raising its height and its dependents' transitively and moving each queued one to its new bucket.
- (b) The pull path: a node without a compile-time height is never queued. A write that reaches it marks it, and a read or a settlement pass brings it up to date by 3b's rule, with a `Check` state: bring each `Node` dependency up to date in recorded order, and recompute only when one of them changed (Candidate 3b, open decision 15 (b)). Cost: a fourth state value and the dependency walk on reads of such nodes; an observed pull node is brought up to date after `settle` drains the queue, so it reads only settled height-ordered nodes.

Held: (b). Heights stay an optimization for the node kinds whose heights the compiler knows, and a node kind without one costs no height maintenance. It needs `NodeDefinition.height` to say which case a node is in, as an enum whose variants name the two (`{ kind: 'Static'; height: number } | { kind: 'Pulled' }`), and `invalidate` to mark a `Pulled` node instead of queuing it. Changes 1 to 4 keep `height: number`, because no node they create lacks one; widening it is this follow-up's change, and touches `NodeDefinition`, `GraphNode`, and `invalidate` only.

## Open decisions

### 1. Resolver markers in the normalization AST

- (a) Root only: `normalizeData` takes the entrypoint, and settlement creates the rest through reader ASTs (held, Resolver markers).
- (b) A marker per selection set, as the user's design describes. Cost: artifacts import reader artifacts and source modules, the marker set goes stale as links change, and it duplicates what reader ASTs already say.

### 2. Recomputation architecture

The candidates and their comparison are Recomputation architecture.

- (a) 3a with compile-time heights and `Stale` marks (held, the user's direction). A write queues each needed entry that read a changed input, drains the queue lowest height first, and queues an entry's direct dependents only when its result changed; an entry nobody needs is marked `Stale` with its dependents and recomputed when read. Cost: the inverse of every dependency list, kept by `setDependencies`; a state and a needed count per entry; a height per artifact, written by the compiler; no cutoff on a read of a `Stale` entry (open decision 15); heights that eager pointers break (open decision 14).
- (b) 3b, mark along reverse edges, then bring up to date (the considered alternative). Cost: the inverse, a state per entry, and every transitive dependent of a changed input marked `Check` on every write and walked again by settlement. Benefit: no height, and cutoff on reads of entries nobody needs.
- (c) 2, pico's pull with epochs. Cost: every live edge under the observed roots compared on every write, and a tag and epoch on every server value; benefit: no inverse to keep.
- (d) 5, full re-run per write. Cost: every live resolver on every write, and no equality cutoff below the roots.

Not approved for implementation.

### 3. Eager computation for retained queries nobody observes

- (a) Every retained root is brought up to date after every write. Cost: every retained query's affected resolvers run on every write, rendered or not (a prefetched route, a screen kept retained after navigation); under held 3a, `neededCount` would count retentions as well as observers.
- (b) A write recomputes only needed entries; a retained root nobody observes is `Stale` after a write that reaches it and is brought up to date when `listen` attaches, in the subscribe call, not in render (held, the user's choice). Cost: the first render of a newly observed `Stale` location reads the last computed result, and `useSyncExternalStore` renders again after `listen` brings it up to date.
- (c) Per retention, a choice between (a) and (b). Cost: an option on `normalizeData`.

### 4. Where resolver entries live

- (a) In `environment.graph`, beside the store on the environment; records keep store.md's untagged values (held, with open decision 12). Cost: `garbageCollectEnvironment` deletes records from the store and then their nodes from the graph, one `deleteInput` per record.
- (b) On the record, under the resolver store key, with every value tagged `Server` or `Resolver`. Cost: a tag object per server value, a narrowing in every server-value reader, and two broken-invariant throw sites (a resolver entry under a server store key and the reverse). Under candidate 2 the tag also carries the epoch, and this option is the one that candidate needs.

### 5. A resolver that throws

- (a) Stored as `ResolverThrew`, propagated to parents like `MissingData` (held). Reason: `normalizeData` runs resolvers (the new root's, and every needed entry's).
- (b) Propagate the exception, v1's behavior. Cost: one broken resolver aborts `normalizeData` for every query and leaves level 1 unsettled.

### 6. Garbage collection from live resolver dependencies

The user's idea, explicitly unsettled: records and fields linked to the resolver entries that read them, so collection knows what is live without walking ASTs.

- (a) Walk retained normalization ASTs, as v1 (held).
- (b) A record is live when a live entry depends on it. `graph.inputDependents` (change 2), with `recordInput` ids, is that index already; no new edges are needed. Cost: a record a retained query fetched but no live entry read is deleted (fields after a `MissingData`, records a reader stops before), so `check` of that retained query reports `MissingData` and a refetch follows; retention no longer means what store.md and upstream mean (`garbageCollection.ts:88-119`).

### 7. Result equality

Settled: (a).

- (a) `===`, upstream's for resolver values (`areEqualWithDeepComparison.ts:84-95`). A primitive or a reused object cuts off; a new object propagates (R4).
- (b) A comparator the field declares. Not taken: a directive, and v1 parses none.
- (c) Structural equality of resolver outputs. Not taken: outputs can be functions, class instances, React elements, or cyclic, so no comparison is both correct and cheap.

Upstream's deep comparison, `mergeObjectsUsingReaderAst`, applies where the compared values are store data shaped by a reader AST: subscription delivery in Step A (`selectionSetDataAreEqual`, network-and-subscriptions.md, Semantics) and the selection-set results of Follow-ups 8. In changes 1 to 4, server values are compared by their AST shape at write time (change 1) and notification compares stored result identity, so neither needs it.

### 8. A resolver that reads or writes the store

- (a) No guard: the resolver's parameter holds no store, and one reached through a closure is undefined behavior.
- (b) The graph records that a computation is running; `readStoredResult`, `normalizeData`, and `garbageCollectEnvironment` throw while it does. Cost: a phase field on `RecomputationGraph`.

Not held.

### 9. Callback errors

- (a) Call every callback, then rethrow one error or an `AggregateError` (held). Reason: one failing observer must not leave others stale.
- (b) Propagate the first throw. Cost: later observers miss the change until the next write.

### 10. Step A

- (a) Ship it before change 1, for a React demo.
- (b) Skip it. Cost: no React demo until change 4.

Not held: the user's choice.

### 11. When an entry no root reaches is deleted

- (a) By `garbageCollect`'s sweep (held, change 3). Under held 3a such an entry is marked `Stale` by the first write that reaches it, and later writes stop at it, so it costs no recomputation until then. Matches v1's caller-driven collection and Follow-ups 7's deferral.
- (b) By reference count: an entry's count is its `rootCount` plus `dependents.size`; `dispose`, the stop function of `listen`, and `setDependencies` decrement it, and at zero the node is disposed and `setDependencies(graph, node, [])` decrements its dependencies in turn. Cost: a count kept on three paths, and deletion in the middle of a write. It is what constructors and destructors need (Follow-ups 1).

### 12. Coupling of the recomputation engine to the store

The analysis is The engine, the store, and the environment.

Settled: (a).

- (a) A store-agnostic engine over opaque `InputId`s and `NodeId`s, held on the environment beside the store. `Store` stays store.md's nested map; `normalize` returns its changes and `garbageCollect` its deleted records; the walk reports reads through `ReadHooks`; `normalizeData`, `garbageCollectEnvironment`, and `readStoredResult` forward both to the graph. Nodes are owned by their record's presence input, so they are freed with the record. Keeps: v1's store code and tests, tests that compare the store directly, and upstream's environment shape. Cost: two id strings per server read and one per resolver read, a presence edge per (node, record read), the `owned` map, and every function that needs both the store and the graph takes the environment.
- (b) Entries and dependents keyed by record and store key, in fields of a `Store` that becomes an object. Not taken: the engine names records and store keys; a layered store must translate its changes into the base layer's shape, which has no case for removing a layer; and every v1 function and test reads `store.records` where it read `store`.

### 13. Values with lifecycles

The analysis is Values with lifecycles.

- (a) The engine owns each node's value lifecycle (held): `Disposable` computations return an `ItemCleanupPair`; a new value is constructed before the old is released; releases are deferred until after notification and flushed in order; a `===` value keeps the old holding and releases the new retain at once. A constructor that throws stores `ResolverThrew` and releases the old value (held); a destructor that throws does not stop the flush, and the errors are rethrown together (held).
- (b) Lifecycles outside the engine: a disposable value lives in a registry the resolver's code calls, and the engine stores only the handle. Cost: nothing releases a handle when the node's value is replaced or the node is disposed, so the registry needs its own reachability, which is the engine's again.

Not decided: the user's choice.

### 14. Static heights and eager client pointers

Changes 1 to 4 read no client pointer (reader-ast.md, `SelectionRead::NotRead`), so every `Resolver` dependency goes to a field the reader AST selects, at a lower height. An eager pointer (reader-ast.md, Client pointers, Later) reads client fields on its target: `field Pet.Card { bestFriend { Card } }`, with `bestFriend` a pointer, is not an `InlineCycle`, and the entry of `Card` on `Pet:1` reads the entry of `Card` on `Pet:2`, of the same height. The bucket queue cannot order the two.

- (a) Dynamic heights where pointers require them, Incremental's scheme: the artifact's height is the initial value, and a computation that reads a node whose height is not below its own raises its own height above it, and its dependents' transitively, moving each queued one to its new bucket. Cost: the adjust walk on such an edge, and an entry height that can differ from its artifact's. A data cycle (`Pet:1.bestFriend` is `Pet:2` and `Pet:2.bestFriend` is `Pet:1`) is then a cycle of entries, which the read must detect and report; upstream's eager reads have the same case.
- (b) The selection through a pointer is not a dependency of the reading entry: it is its own root, the follow-up retention of Follow-ups 2, observed by the component that renders it. Cost: a parent's resolver cannot read data through an eager pointer.
- (c) The compiler rejects a pointer read that closes a cycle of client field selections, and a pointer's height counts the fields read through it. Cost: `field Pet.Card { bestFriend { Card } }` does not compile.

Not decided; it arises with client pointers.

### 15. Cutoff on a read of a `Stale` entry

- (a) A `Stale` entry recomputes when read, its reads bringing the `Stale` entries below it up to date first (held, the user's specification). Cost: no cutoff on the read path. After `Rex` to `Ray` on an unobserved `BadgePage`, the next read recomputes all five entries where the push recomputes `Initial` alone; attaching `listen` to a retained screen recomputes every `Stale` entry under it.
- (b) Two lazy states. `Dirty`: an input it read changed, or an entry it reads recomputed to a new result. `Stale`: an entry it reads, directly or transitively, is `Dirty` or `Stale`. A read brings a `Stale` entry's `Resolver` dependencies up to date in recorded order and recomputes it only when that made it `Dirty`; `recompute` marks its direct dependents `Dirty` on the read path. This is 3b's `Check` restricted to entries nobody needs; the push keeps no such marks. Cost: one more state value, the dependency walk on reads, and a marking on the read path.

### 16. How the engine knows an entry is needed

- (a) A count per entry, kept current: the observers at its location plus its needed dependents (held, change 4, Needed entries). Cost: `listen`, its stop function, and `setDependencies` of a needed entry walk the region whose neededness changes.
- (b) On demand: walk an invalidated entry's dependents upward to an observed location. Cost: the ancestors of every invalidated entry, on every write.
- (c) Per write: mark the needed region from every observer down before invalidating. Cost: the whole observed region on every write.

### 17. A response that throws part-way

`normalize` writes in place and throws on the first position that does not match the AST (store.md, Errors), so the store holds the writes made before the throw.

- (a) `normalize` catches the throw and rethrows a `PartialWriteError` carrying the changes made before it and the original error as `cause`, with the original message; `normalizeData` marks those changes and rethrows (held, change 1 and change 2). Reason: a node that read a value written before the throw is marked, as it would be for a write that did not throw.
- (b) No wrapping: the changes are lost with the throw. Cost: every node that read a value written before the throw returns its old result until another write reaches the same input, with no error.
- (c) `normalize` validates the whole response against the AST before it writes. Cost: a second walk of every response, and `recordLink`'s checks run in both walks.
- (d) On any throw, `normalizeData` marks every node. Cost: every live node recomputes or is marked `Stale` after one malformed response.
