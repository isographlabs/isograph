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
level 1: resolver graph   a DerivedNode per (record, resolver store key), in environment.graph, recomputed after normalizeData
                                 ^ reads
store:                    server values on records, in environment.store, written by normalize
```

Level 1 is the resolver graph, held on the environment beside the store (The engine, the store, and the environment). Its derived nodes are stored resolver results, one per client field, record, and argument values. Its edges are each derived node's dependencies: the server values and the other resolver results its reader AST read. How level 1 brings its derived nodes up to date after a write, and which edges it stores to do so, is the Recomputation architecture below. `normalizeData`, the environment's write, owns level 1: it computes the derived nodes a newly retained query reaches, and after every write it recomputes the derived nodes observers need, in height order, and marks the others `Stale`. Level 1 owns ordering. No level 2 code runs while level 1 is computing.

Level 2 is observers. An observer names one location of level 1 (a record and a resolver store key) and a no-argument callback. It holds no value. After level 1 settles, each observer whose location's stored result changed is called once. `snapshot` returns the stored result and computes nothing. This is the shape React's `useSyncExternalStore(subscribe, getSnapshot)` takes (Level 2), so a React package can render from level 2 while `@isograph/runtime` stays React-free.

With React, components are resolver results too (`@component`, Later): a component's derived node is a level 1 node like any other, and a mounted component is a level 2 observer of its own derived node.

## Recomputation architecture

How level 1 keeps its derived nodes current after a write. pico's algorithm (global epoch, `time_updated` and `time_verified`, verification of dependencies before a derived node, backdating) is one candidate, not a given. pico was built for the compiler, where an LSP request demands one memo and everything else stays unverified; the runtime's goal makes every observed derived node demanded after every write. This section states the requirements, lays out five candidates against three examples, and compares them. The option it ends on, 3a with heights computed per derived node at run time and `Stale` marks for derived nodes nobody needs, is held by the user's direction; it is not approved for implementation. The candidates are written against the store's functions: a write is a `normalize` or a `garbageCollect`, and the state a candidate keeps is described without saying where it lives. Under the held design that state lives in `environment.graph`, and the writes are `normalizeData` and `garbageCollectEnvironment`, which call the store's functions (The engine, the store, and the environment).

### Requirements

From the goal and the decisions this doc, store.md, and reader-ast.md settle:

- R1. A render does no work. `snapshot` is a lookup; a render reads no store value and calls no resolver.
- R2. Eager where observed. The `normalize` that retains a query computes its entrypoint's result and every result that result reaches. After every later `normalize` or `garbageCollect`, every needed result is current when it returns: a derived node is needed when an observer is at its location or a needed derived node reads it. A result nobody needs is current when read: a write that reaches it marks it `Stale`, and the next read recomputes it (open decision 3, held (b)). A `snapshot` of a `Stale` result returns its last computed value; `listen` brings it up to date (change 4).
- R3. Order. After a write, a derived node recomputes only after every stored result it reads is current. No derived node is computed from a stale input, and no derived node is computed twice for one write.
- R4. Equality cutoff is an optimization. It is not a requirement, and no candidate can guarantee it. A derived node whose recomputed result is `===` to its previous result does not mark its dependents. This stops propagation at a primitive result and at a result that is a stored or reused object. A result that is a new object is never `===` the previous one, so it always propagates, even when it is structurally equal. A general structural equality over arbitrary JavaScript values (functions, class instances, cycles) cannot be both correct and cheap, so the runtime does not attempt one. Server values keep a check at write time: `normalize` compares each value it writes with the stored one by the shape the AST gives it (change 1), as upstream does: a leaf by `===`, a link by `entityName` and `id`, and a list of links item by item and by length. A leaf whose value is a JSON object or list is a new object on every response, so it always counts as changed. Without the check, every response marks every written field changed and the whole retained tree recomputes on every response. With `Rex` to `Ray`, `Initial` recomputes to `"R"`, a string `===` the previous one, and nothing above it recomputes. With `Rex` to `Ray`, a resolver that returns `{ length: data.name.length }` recomputes to a new `{ length: 3 }`, and every derived node that reads it recomputes.

  Upstream never compares resolver outputs either. `callSubscriptionIfDataChanged` (`subscribe.ts:111-162`) re-reads the data the reader AST selects with `readButDoNotEvaluate` (`:117-136`) and compares that data, not the resolver's result, with `mergeObjectsUsingReaderAst(subscription.readerAst, oldItem, newItem)` (`:138-142`), calling the callback when the merge does not return the old item (`:152`). The merge (`areEqualWithDeepComparison.ts:4-121`) walks the reader AST, so it compares only store data: scalars by `===` (`:22-23`), lists item by item (`:33-53`), linked selections recursively (`:24-29`), links by entity and id (`:96-113`), nested resolver values by `===` (`:84-95`), and client pointers as always unequal (`:65-69`). Where a part is equal it writes the old value into the new object (`:77-82`, `:48`, `:110`) and returns the old object when every part is equal (`:120`), so equality also keeps identity stable. The well-defined place for structural equality is therefore a resolver's inputs, which are store data shaped by the reader AST. Under held 3a a derived node recomputes only when an input it read changed: a source is compared by `normalize` at write time by its AST shape (change 1), and a derived node by `===`. Reading data along the reader AST and keeping the old object where the new data is equal (upstream's merge) is the way to give a snapshot stable identity, which `useSyncExternalStore` requires of `getSnapshot` (network-and-subscriptions.md, The snapshot: `subscriptionSnapshot` returns the same object until a re-read is unequal; react.md's `useSubscriptionSnapshot` relies on it).
- R5. Observers are called once per write, after level 1 settles, and a `snapshot` inside a callback returns a settled result.
- R6. One derived node per client field, record, and argument values, shared by every parent and query that reads it (The same resolver selected twice).
- R7. Dependencies follow the data. A recomputation can read different inputs than the previous one (a link moved from `Pet:2` to `Pet:3`, a `MissingData` that became present). The next write is judged against the inputs of the last computation only.
- R8. `normalize` is the only writer of server values. It knows exactly which (record, store key) pairs it changed and which records it created, and returns them (change 1). `garbageCollect` is the only deleter, knows which records it deleted, and returns them (change 2).
- R9. Minimal machinery: the fewest stored fields and functions that meet R1 to R3 and R5 to R8, give R4's optimization, and allow R10.
- R10. Scheduled recalculation can be added later without redesign (Follow-ups 9). It has two halves. The first is laziness: a derived node nobody needs is only marked on a write and is recomputed when next read. Changes 2 to 4 build it. The second is deferral: a caller-supplied scheduler decides when, and at which priority, observed derived nodes are brought up to date, with observers notified once per pass that settled them. Changes 1 to 4 do not build it, and the architecture must allow it without undoing anything in them. Deferral requires that marking a write is separate from recomputing, that a pass can leave a derived node marked for a later pass, and that the observers needing a derived node are counted on the derived node.

Cost terms used below, per write:

- `C`: the (record, store key) pairs the write changed and the records it created or deleted.
- `A`: the derived nodes that read something in `C`, directly or through other derived nodes (the affected set).
- `A'`: the derived nodes that recompute, a subset of `A`. A derived node is in `A'` when at least one input changed: a server value in `C`, a record created or deleted in `C`, or a stored result whose new result is not `===` its previous one. When every resolver in `A` returns a new object, `A'` is `A`.
- `L` and `E`: the live derived nodes (those a retained or observed location reaches) and the dependency edges among them and to server values.
- `R`: the retained and observed locations.
- `S`: the server values in the store.

### The examples

The fixture of change 2 (Tests).

Diamond. `Badge` (A) reads `Monogram` (B) and `Signature` (C). Both read `Initial` (D). `Initial` reads `Pet:1.name`; `Signature` also reads `Pet:1.nickname`. `BadgePage` on `Query.__ROOT` reads `pet____id___"1"` and `Badge` on `Pet:1`. Two writes: `name` from `Rex` to `Max`, where every derived node above `Initial` changes; and `Rex` to `Ray`, where `Initial` recomputes to `"R"`, a string `===` the previous one.

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
2. Settlement verifies every retained and observed location. Verifying a derived node: if `timeVerified === store.epoch`, return it. Otherwise walk `dependencies` in order; a server dependency changed when its `timeUpdated !== observed`; a resolver dependency is verified first (recursively) and changed when its `timeUpdated !== observed`. At the first changed dependency, recompute; if the result is `===` the old one, keep the old `timeUpdated` (backdating). Set `timeVerified = store.epoch`.

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

- Ordering: a depth-first post-order over the current dependencies, by recursion; nothing stores the order. A derived node is computed after the derived nodes it reads were verified in this epoch, and at most once per epoch, because a computed derived node has `timeVerified === store.epoch`.
- Cost per write that changed anything: `O(R + E)` epoch comparisons, every live edge once, plus the `A'` recomputations. A write that changed nothing costs `O(R)`.
- Memory: a tag and a `timeUpdated` on every one of the `S` server values, read by a resolver or not; two epochs and the forward dependency list per derived node.
- Needs from `normalize`: an epoch stamp per changed value. The set `C` that `normalize` knows (R8) is reduced to stamps, and settlement rediscovers it by walking `E`.
- Equality cutoff: backdating, with `===` as the equality. A resolver that returns a new object gets a new `timeUpdated` on every computation, so every derived node that reads it computes.
- Failure modes: `garbageCollect` deletes entries, which pico never does (pico.md, Source: `remove` keeps the source node and stamps an epoch), so a dependency compares the observed epoch for equality instead of pico's newer-than (`execute_memoized_function.rs:192-206`), and a delete must advance the epoch. A read of an absent key records `observed: null`. Every server-value reader (`check`, `garbageCollect`, `read`) narrows a tagged entry, and a `ResolverEntry` under a server store key is a new broken-invariant throw site.

What epochs buy: correctness without reverse edges. A derived node nobody demands is never visited, a dependency list is rebuilt from scratch on each computation so there is no inverse to keep consistent, and deleting a derived node unlinks nothing. In pico's setting this is the right trade: a compiler with a whole project's memos answers a hover by verifying the few it demands. In the runtime, R2 demands every observed derived node after every write, and held 3a is lazy for every other derived node through `Stale` marks, so pull verification's laziness buys nothing the push lacks; what remains is a walk over every edge under the observed roots on every write, and a stamp on every server value.

### Candidate 3: push along reverse edges

`normalize` already knows `C` (R8). Each derived node stores, besides its forward dependencies, the set of derived nodes that read it (`dependents`), and the store keeps the same for server locations. A write follows those reverse edges from `C`, so its cost is bounded by `A`, not by `E`. No epochs: a server value stays an untagged `StoreValue`.

Data structures common to the three orderings below:

```ts
// from libs/isograph-runtime/src/store.ts
export type DerivedNode = {
  readonly location: DerivedNodeLocation;
  readonly artifact: ReaderArtifact<ResolverParam, unknown>;
  readonly variables: Variables;
  result: ReadResult<unknown>;
  /** What the last computation read, in read order. */
  dependencies: ReadonlyArray<Dependency>;
  /** Every derived node whose `dependencies` hold a `Resolver` dependency on this one. */
  readonly dependents: Set<DerivedNode>;
};

/** The derived nodes that read each location of one record. */
export type RecordDependents = {
  /** Derived nodes whose read found the record absent. */
  readonly absent: Set<DerivedNode>;
  /** Derived nodes that looked up each store key while the record existed. */
  readonly values: Map<StoreKey, Set<DerivedNode>>;
};
```

`Store` holds `RecordDependents` by entity and record id, outside the records, so a dependency on an absent record has a place to live. The held design keeps the same edges in graph terms instead (change 2, The engine; The engine, the store, and the environment).

Recording and updating dependencies (R7): a computation collects its dependencies in read order, then one function replaces the derived node's list and updates the inverse: remove the derived node from every old dependency's set, add it to every new one's. MobX does the same with a diff (`bindDependencies`, `packages/mobx/src/core/derivation.ts`: it marks new dependencies, drops old ones no longer read, adds new ones); Reactively keeps the prefix of sources read in the same order and splices the rest (`updateIfNecessary` and `update`, `packages/core/src/core.ts` of `milomg/reactively`). The link move removes `FriendLabel` from `PetName` on `Pet:2`'s `dependents` and adds it to `PetName` on `Pet:3`'s.

What remains is order. Three ways.

#### 3a. Heights

Jane Street's Incremental: "stabilize traverses the DAG in topological order starting at variables that changed", taking "the lowest-height node" from a recompute heap, recomputing it, and enqueueing its parents when its value changed; a node's height is less than its parents' (`src/incremental_intf.ml` of `janestreet/incremental`). Its heights are dynamic: `bind` creates nodes whose height must exceed the bind's left-hand side, and heights are adjusted as the graph changes.

In isograph each derived node's height is computed at run time from the derived nodes it read (change 4, Heights). When a derived node's computation finishes, its height is 0 when it read no other stored resolver result, else `1 + max(height)` over the derived nodes it read. Every derived node it read was brought up to date by that read, so their heights are known. A derived node's height is above the height of every derived node it reads.

Heights depend on data, because what a derived node reads depends on data:

- A nullable link. `FriendLabel` on a pet whose `bestFriend` is `null` reads no `PetName` derived node, so its height is 0. `FriendLabel` on `Pet:1`, whose `bestFriend` is `Pet:2`, reads `PetName` on `Pet:2`, so its height is 1.
- A refinement. A `Refinement` node reads its selections only when the record's entity is the condition's entity, and is `null` otherwise (`ReaderRefinementSelection`, `libs/isograph-runtime/src/artifacts.ts`; `read.ts:150-153`). A derived node of a field whose `Resolver` node sits under `asPet` reads that resolver's derived node on a `Pet` record and reads no derived node under the refinement on a record of another entity, so two derived nodes of one field have different heights when their records' entities differ.
- An eager client pointer (Later). The derived node of `field Pet.Card { bestFriend { Card } }` on `Pet:1` reads the derived node of `Card` on `Pet:2`, so its height is one more than that derived node's.

A height set by one computation can be too low for a later one: after a write sets a `null` `bestFriend` to `Pet:2`, `FriendLabel`'s recomputation reads `PetName` on `Pet:2`, whose height 0 equals its own. Height adjustment restores the order, as Incremental's `adjust_heights` does (`src/adjust_heights_heap.ml` of `janestreet/incremental`). When a recomputation records a dependency whose height is at or above the derived node's own, the derived node's height rises to one more than that dependency's. Then each dependent whose height is now at or below the height of the derived node it reads rises to one more than it, transitively. A dependent queued in the bucket queue moves to its new bucket, as Incremental's `adjust_heights` calls `Recompute_heap.increase_height` on a node in the recompute heap. Heights only rise: a recomputation that reads lower derived nodes, or fewer, keeps its height. Outside a call that is computing a node, every derived node's height is above the height of every derived node it reads.

Under changes 1 to 4 the runtime graph has no cycle. Every `Resolver` node of `F`'s reader AST selects a field that query generation inlines into `F` (`inlined_fields`, `crates/isograph_compiler/src/merged_selection.rs`), and a cycle of inlined fields is an `InlineCycle` query error (query-generation.md, Cycles). A derived node of `F` on a record is computed by reading `F`'s reader AST from that record and calling `F`'s resolver with the data read. The resolver receives `data` and `parameters` and no store (open decision 8 covers a closure that reaches one), so every stored result the derived node reads is read by a `Resolver` node of `F`'s reader AST: a derived node of one of the fields it selects, on whichever record the links lead to. Links choose the record, never the field, so a cycle of derived nodes would be a cycle of client field selections, and there is none. So adjustment never reaches the dependency that started it. This argument is this doc's, not Incremental's. An eager client pointer makes a cycle of derived nodes possible (open decision 18).

A write: each derived node that read a changed source goes into bucket `height`; buckets are drained lowest first; each derived node recomputes, and when its result is unequal (`===`, R4) its dependents go into their buckets; when it is equal nothing is queued. A derived node read during a recomputation usually has a lower height, so its bucket is already drained and it is current. A derived node read at or above the reader's height (an upward read, one of the data-dependent cases above) abandons the reader's computation before the reader's resolver runs; the reader is raised above it and queued again at its new height (Ordering; open decision 19, held (c)). Nothing is marked before it is known to need recomputing: the work of a write is the derived nodes that recompute and the direct dependents of the ones whose result changed.

Diamond, with an observer at `BadgePage`, so every derived node is needed. Heights `Initial` 0, `Monogram` 1, `Signature` 1, `Badge` 2, `BadgePage` 3. `Rex` to `Max`:

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

A link set from `null`, an observer at `FriendPage` with `{ id: "4" }`, `Pet:4.bestFriend` `null`: `FriendLabel` on `Pet:4` has height 0, `FriendCard` 1, `FriendPage` 2. A write that sets `Pet:4.bestFriend` to `Pet:2` queues `FriendLabel` at 0. Its recomputation reads `PetName` on `Pet:2`, height 0, so `FriendLabel` rises to 1, `FriendCard` to 2, and `FriendPage` to 3. `FriendLabel`'s result changed, so `FriendCard` is queued at 2, its new height, and `FriendPage` after it at 3.

- Cost per write: `O(|A'| + dependents of A')` plus one step per bucket up to the largest height queued, plus the adjustment walk after each upward read. Nothing that will not recompute is visited, except the dependents adjustment raises.
- Failure mode of plain 3a: it recomputes every derived node with a changed input, whether or not anything needs it. A derived node whose retention was disposed, or that a plain `read` stored, recomputes on every relevant write until `garbageCollect` deletes it.

Held 3a adds `Stale` marks for derived nodes nobody needs (change 4). Each derived node counts the observers at its location plus its needed dependents (`neededCount`); it is needed when the count is above 0. A derived node whose input changed is queued when it is needed. Otherwise it and its dependents, transitively, are marked `Stale` and not queued; the marking stops at a derived node already `Stale`. A read of a `Stale` derived node recomputes it, and each `Stale` derived node it reads is brought up to date by that read first. Every dependent of a derived node nobody needs is itself not needed, so `Stale` marking never reaches a needed derived node, and the queue never holds a derived node nobody needs.

Diamond, `BadgePage` retained and not observed, `Rex` to `Max`: `Initial` is not needed, so it, `Monogram`, `Signature`, `Badge`, and `BadgePage` are marked `Stale` and no resolver runs in the write. A later `listen` at `BadgePage` reads it: `BadgePage` recomputes, its read of `Badge` recomputes `Badge`, whose read of `Monogram` recomputes `Monogram`, whose read of `Initial` recomputes `Initial`; then `Badge`'s read of `Signature` recomputes `Signature`, reading the `Clean` `Initial`. Each once. `Rex` to `Ray` the same way recomputes all five on that read: a `Stale` derived node recomputes when read whether or not what it reads changed (open decision 15).

#### 3b. Mark, then bring up to date (Reactively, MobX)

Reactively's algorithm (`packages/core/src/core.ts` of `milomg/reactively`): each node is `CacheClean`, `CacheCheck` ("might be stale, check parent nodes to decide whether to recompute"), or `CacheDirty`. A write marks "direct children dirty and their deeper descendants check", stopping at nodes already marked. Reading a node runs `updateIfNecessary`: for `CacheCheck`, bring each source up to date in order, stopping at the first that made this node dirty; for `CacheDirty`, recompute; then clean. A recomputation that produced an unequal value marks its children dirty; an equal one leaves them at check, so they clean without recomputing. MobX's derivation states are the same scheme: `UP_TO_DATE_`, `POSSIBLY_STALE_` ("some deep dependency changed, but don't know if shallow dependency changed"), `STALE_`, and `shouldCompute` checks the computed dependencies of a possibly stale derivation in order (`packages/mobx/src/core/derivation.ts`).

Here the derived node gains `state: 'Clean' | 'Check' | 'Dirty'`. A write, step by step:

1. While writing, `normalize` marks: for each changed location in `C`, every derived node in its dependents set becomes `Dirty`; a derived node that was `Clean` marks its dependents `Check`, recursively, stopping at a derived node not `Clean`. Marking runs no resolver.
2. Settlement brings the marked roots up to date: the derived nodes that are both marked by this write or an earlier one and the stored result at a retained or observed location. It does not visit an unmarked root. A `Clean` derived node returns at once. A `Check` derived node brings its `Resolver` dependencies up to date in recorded order and stops at the first after which it is `Dirty`. A `Dirty` derived node recomputes; each stored result it reads is brought up to date first, by the read. A recomputed derived node whose result is unequal sets its dependents `Dirty`. The derived node ends `Clean`.

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

`Rex` to `Ray`: `Initial` recomputes `"R"`, `===` the old string, so `Monogram` and `Signature` stay `Check`; `Monogram` finds no dependency made it `Dirty` and becomes `Clean`; so do `Signature`, `Badge`, and `BadgePage`. One call, five derived nodes visited. A write to `Pet:1.tagline` marks nothing in the diamond, so `BadgePage` is not a marked root and settlement does not visit it.

With observers on `Badge` and `Initial` as well as the retained `BadgePage`, the marked roots are `Initial`, `Badge`, and `BadgePage`. Settlement brings each up to date in any order; the first brings up to date every marked derived node it reaches by the recursion, and the later ones find those derived nodes `Clean`. The calls are the same as in the trace above.

Why `Check` marks exist. A pure push from the `Dirty` derived nodes (recompute each `Dirty` derived node, and only when its result changed, mark its direct dependents `Dirty`) visits fewer derived nodes: it never touches the dependents of an unchanged result, so `Rex` to `Ray` visits `Initial` alone. It needs an order, though. In the diamond, `Badge` reads both `Monogram` and `Signature`; pushed from `Monogram` first, it recomputes with the stale `Signature` and again after `Signature` recomputes, which R3 forbids. Holding `Badge` until every changed input is done is 3a's heights or 3c's pending-input counts. 3b instead marks every transitive dependent `Check` eagerly, while marking, and resolves the order by pulling: a derived node brings its dependencies up to date before it decides. The price is that settlement clears the `Check` marks above an unchanged result (`Monogram`, `Signature`, `Badge`, `BadgePage` when `Initial` recomputes to `"R"`), so settlement costs `O(marked)` where the pure push costs `O(recomputed)`. Marking already visits those derived nodes, so the order of growth of a write is the same.

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

- Ordering: a derived node recomputes only after every stored result it reads was brought up to date, because the only ways it reaches one are the `Check` walk and the read during recomputation, and both bring it up to date first. It recomputes at most once per write: after recomputing it is `Clean`, and only a dependency's recomputation can make it `Dirty` again, and every dependency was brought up to date before it.
- Laziness: a derived node no settled location reaches keeps its mark and is not recomputed; a later `read` brings it up to date. Marking stops at marked derived nodes, so repeated writes cost `O(1)` per already marked derived node.
- Marked roots suffice. Every dependent of a derived node that is not `Clean` is not `Clean` (a derived node that leaves `Clean` marks its dependents, and a new dependent is added only by a computation, which brought the derived node up to date first), so a root that reaches a marked derived node is itself marked. Settling the marked roots therefore settles every root that reaches anything this write changed, and an unmarked root costs nothing.
- Cost per write: marking `O(|A| + edges within A)`; settlement `O(marked roots)` to start, plus `O(|A| + edges within A)` for the walk; `A'` recomputations. No term grows with the number of retained or observed locations `R`.
- Failure mode it shares with 3a and 3c: the inverse must equal the forward lists exactly. A missed removal keeps a dead derived node in a set (memory, and a mark that does nothing); a missed addition loses a mark and leaves a stale result with no error. One function writing both sides is the guard.

#### 3c. Pending-input counts

Kahn's algorithm over `A`: a first pass over the reverse edges from `C` counts, for each derived node of `A`, how many of its inputs are in `A`; a second pass recomputes a derived node when its count reaches zero and at least one input changed, then decrements its dependents' counts. The order is topological without stored heights. The counting pass visits all of `A`, as 3b's marking does, and a recomputation that reads a derived node of `A` its old dependencies did not count (the link move into a region the write also affected) must bring that derived node up to date on demand, which is 3b's pull. It is 3b with an extra counter and no new property.

### Candidate 4: signals with version numbers (Preact)

Preact Signals (`packages/core/src/index.ts` of `preactjs/signals`): a global `globalVersion` incremented on every effective write; a per-signal `_version` incremented only when the value changes (`value !== this._value`); per edge a `Node` with `_source`, `_target`, and `_version`, the source version the target last observed. A write notifies targets, which set `OUTDATED | NOTIFIED` and propagate without recomputing; effects are queued and run at the end of the batch. A computed's `_refresh` returns at once when it is subscribed and not `OUTDATED`, or when its `_globalVersion` equals `globalVersion`; otherwise `needsToRecompute` walks its sources in order and recomputes when a source's `_version` differs from the edge's, after refreshing that source. A recomputation that returns an identical value does not increment `_version` (equality cutoff); as in every candidate here, a computed that returns a new object always propagates. A computed subscribes to its sources only while it is itself subscribed (`TRACKING`), so an unobserved computed holds no reverse edges and the JavaScript garbage collector can free it.

This is 2 and 3b combined. Subscribed nodes get a push mark (`OUTDATED`) on every transitive target, one flag where 3b distinguishes `Dirty` from `Check`, and per-edge versions then decide which marked node recomputes. Unsubscribed nodes get the epoch fast path (`globalVersion`, pico's `time_verified`) and version comparison alone, which is candidate 2 with a counter per node in place of epochs. In the runtime every stored derived node is owned by the store and deleted by `garbageCollect`, not by the JavaScript collector, so no derived node needs to be unsubscribed to be freed, and the version machinery for unsubscribed nodes has no user. With every derived node subscribed, Preact is 3b with one flag plus a version per node and per edge, where 3b has a three-valued state per derived node. Either detects "a dependency recomputed to an unequal value"; 3b's needs no field on the edge.

SolidJS's computations use a similar pair of stale and pending states; that is from memory, not checked against its source.

### Candidate 5: full re-run of every retained tree

The simplest eager baseline. No dependencies stored at all. After every write, settlement recomputes every retained and observed root from scratch, with a `Map` from location to result for the duration of that settlement, so a derived node reached twice is computed once. Each new result is compared by `===` with the previous settlement's for observers.

- Ordering: correct by recursion; every read sees the finished write.
- Cost per write: every live resolver, `O(L)` resolver calls, whatever changed.
- Memory: one result per root, and the per-settlement map.
- Equality cutoff: none below the root. Every resolver runs on every write, and a resolver returning a new object (fixture resolvers return `data`) makes every observer fire on every write.
- Against the requirements: meets R1, R2, R3, R5, R6, R7. It has none of R4's optimization below the root.

### Comparison

- R1: 1 fails (render calls the resolver); 2, 3a, 3b, 3c, 4, 5 meet it.
- R2: 1 fails; the rest meet it.
- R3: 1 fails "nothing twice"; 2, 3a, 3b, 3c, 4, 5 meet it.
- R4, an optimization: 2, 3a, 3b, 3c, 4 stop propagation at a derived node whose result is `===` its previous one (a primitive, or a stored or reused object), and propagate every new object. Held 3a stops it on the push and not on a read of a `Stale` derived node (open decision 15). 1 compares only at the subscription; 5 only at the root. No candidate compares resolver outputs structurally, because that comparison is not well-defined on arbitrary JavaScript values.
- Structural equality on inputs: the one place it is well-defined is a resolver's inputs, which are store data shaped by the reader AST. 1 does it at the subscription with upstream's `mergeObjectsUsingReaderAst`, which also keeps the old object when equal and so gives stable identity. 2, 3a, 3b, 3c, 4 get the server half from `normalize`'s comparison by AST shape (change 1: an unchanged write changes no source, so nothing is queued, marked, or stamped; a leaf whose value is a JSON object or list always counts as changed) and compare resolver inputs by `===`. A snapshot whose identity must stay stable across a recomputation with equal data needs upstream's merge applied to the data read along the reader AST; it does not need structural equality on outputs.
- R5 and R7: every candidate meets them.
- R10: held 3a meets the lazy half from day one: a derived node nobody needs is marked `Stale` and recomputed when read. Deferral needs no new state: a deferred write marks as if nothing were needed, so every derived node its changes reach becomes `Stale`, and the observed derived nodes the marking reaches are recorded for the deferred pass, which brings them up to date by the read path and notifies their observers (Follow-ups 9). 3b allows it: marking already runs in `normalize` apart from settlement, a derived node that settlement skips keeps its mark, `settle` iterates the marked roots and a scheduler chooses which of them each pass settles, and the observed count is the observers at a location plus the observed `dependents`. 2 allows it: nothing is verified until demanded, and a deferred pass is a later verification of fewer roots; each pass still walks every live edge under its roots. 4 allows it: Preact keeps unsubscribed computeds lazy and queues effects to the end of a batch, so priority goes on that queue. Plain 3a and 3c fail: their propagation recomputes every affected derived node as it goes, so skipping an unobserved derived node leaves it stale with no record of it. 5 allows it per root at full cost per pass. 1 has nothing stored to defer; scheduling exists only in React's re-render.
- R6: 1 fails; the rest meet it.
- R8, what each does with `C`: 1 needs it at record granularity; 2 turns it into stamps and rediscovers it by walking `E`; 3a, 3b, 3c, 4 start from it; 5 ignores it.
- Per write with a small change on a large screen: 1 `O(overlapping subscription trees)`; 2 `O(R + E)`; held 3a the `A'` recomputations among needed derived nodes plus the dependents of those whose result changed, and one `Stale` mark per derived node nobody needs that the write reaches and that is not `Stale` already; 3b and 3c `O(A)` (3b starts settlement from the marked roots); 5 `O(L)` resolver calls.
- Memory beyond the results: 2 a tag and epoch on all `S` server values, two epochs and forward edges per derived node; 3a, 3b, 3c, 4 forward and reverse edges, plus a dependents index for the server locations live derived nodes read (only those, not all of `S`); held 3a a state, a needed count, and a height per derived node, and the bucket queue; 3b a state per derived node; 4 a version per edge.
- Machinery (R9): 2 has the fewest invariants (no inverse to keep). Held 3a adds the inverse, a three-valued state, the needed count, the bucket queue, and height adjustment after an upward read. 3b adds the inverse, a three-valued state, mark, and bring-up-to-date. 3c and 4 add 3b's pieces plus counters or versions.
- Unreached derived nodes (a disposed retention before `garbageCollect`, a plain `read` outside any retention, a retained query nobody observes): 2, held 3a, 3b, 4 do not recompute them until read; held 3a marks each `Stale` once and later writes stop at it. Plain 3a and 3c recompute them.
- A read of a derived node nobody needs, after a write that reached it: 3b brings its dependencies up to date in order and skips the recomputation when none changed; held 3a recomputes every `Stale` derived node the read reaches (open decision 15).

For a sense of scale, from no measurement: a screen with 2,000 live derived nodes averaging 5 dependencies is 10,000 epoch comparisons, each two or three `Map` lookups, for every write under 2, including a write of one field that one derived node reads; under held 3a the same write recomputes that derived node, and visits its dependents only if its result changed.

### Held: 3a, height-ordered push with heights per derived node and `Stale` marks

The eager requirement removes the one thing pico's epochs are for: pull verification exists so that undemanded memos are never visited, and R2 demands every observed derived node after every write, so 2 visits every edge under the observed roots on every write while `normalize` already holds the exact set of changed locations and discards it into stamps. Push from that set bounds each write by what it affects.

Among the push orderings, held 3a does the least work. A write recomputes the needed derived nodes that read a changed source; a derived node whose result changed queues its direct dependents, and one whose result is `===` its previous one queues nothing. Nothing is marked in advance: there are no `Check` marks to set while writing and none to clear above an unchanged result, which 3b pays for with a walk over `A` (Candidate 3b, Why `Check` marks exist). In the diamond with `Rex` to `Ray`, held 3a visits `Initial`; 3b marks and then clears `Monogram`, `Signature`, `Badge`, and `BadgePage`. With no cutoff (every resolver returns a new object), `A'` is `A` and the two make the same recomputations; held 3a still skips 3b's marking walk. Its ordering state is one bucket queue, an array indexed by height, drained lowest first.

The classic objection to 3a is the cost of maintaining heights: Incremental keeps heights consistent as `bind` changes the graph, adjusting a node's height and its parents' when a new edge would point upward. Held 3a pays that cost only where data makes an edge point upward. A derived node's height comes from what its computation read (Candidate 3a). Most recomputations read only derived nodes below the derived node: a dependency that moves with a link (`PetName` on `Pet:2` to `PetName` on `Pet:3`) keeps its height, and the recomputation compares one height per derived node it read. A recomputation that reads a derived node at or above its own height (a link set from `null`, a refinement on a record of another entity, an eager client pointer) runs the adjustment walk over the dependents whose heights rise (change 4, Heights).

The earlier objection to 3a here was its lack of a lazy mode: it recomputed derived nodes no root reaches until `garbageCollect`, and R10's laziness needed 3b's marks. `Stale` marks for derived nodes nobody needs remove it from day one. A needed derived node is never `Stale`, a derived node nobody needs is never queued, and the needed count that separates them is the observed count R10 asks for. Laziness costs one `Stale` mark per derived node nobody needs that a write reaches; marking stops at a derived node already `Stale`, so a second write to the same source marks nothing new.

3b is the considered alternative (open decision 2 (b)). It keeps the cutoff on a read of a derived node nobody needs, which held 3a gives up (open decision 15), and it needs no height. It marks every transitive dependent of a changed source on every write and walks the marked derived nodes again to settle, where held 3a visits the derived nodes that recompute and the dependents of those that changed. 3c and 4 add counters or versions to 3b without a new property; 5 meets the requirements and runs every live resolver on every write; 1 fails R1, R2, R3, and R6.

The cost held 3a takes on is the inverse edge set: memory per edge and an invariant (the dependents sets equal the inverse of the dependency lists) whose violation is a silently stale result. That is contained by making one function, `setDependencies`, the only writer of either side. The inverse also pays for itself elsewhere: it carries the needed counts, it is the reference count constructors and destructors need (Follow-ups 1, open decision 11), and it is the index open decision 6 (b) needs. Removing epochs also removes the tag and stamp from every server value, so records stay store.md's untagged maps and `check`, `garbageCollect`'s mark, and the reader's server reads do not change.

Held 3a's bound does not depend on equality cutoff, which stays a `===` optimization (R4): only needed derived nodes that read a changed source recompute, and `normalize`'s comparison by AST shape (change 1) keeps an unchanged server value out of `C`. When every resolver returns a new object, held 3a recomputes exactly the needed part of `A`; 5 recomputes all of `L`, and 2 still compares every edge under the observed roots.

### What depends on this choice

Written for held 3a, and changed if another candidate is chosen: Steps; change 2 except The resolver store key and Results, equality, and a resolver that throws; Ordering; change 3's Computing the new root, Unobserved results are lazy, and Garbage collection; change 4 except `listen`, `snapshot`, and `Observed`; the engine's functions in changes 2 to 4; Values with lifecycles' Replacement and release; the mentions of `dependents` and marking in Follow-ups 1, 8, and 9; and open decisions 2, 3, 4, 6, 8, 11, 14, 15, 16, 18, and 19. Each of those sections in Steps and changes 2 to 4 opens with a line saying so.

Independent of it: Two levels, Step A, End-user experience, change 1, The resolver store key with `ReaderArtifact.selectableName`, Results, equality, and a resolver that throws, change 3's `normalizeData` signature, `Retention`, and Resolver markers, change 4's `listen`, `snapshot`, and `Observed`, The engine, the store, and the environment, Other derived node kinds, Later, the rest of Follow-ups, and open decisions 1, 5, 7, 9, 10, 12, 13, and 17.

## The engine, the store, and the environment

Settled (open decision 12). Changes 2 to 4 are written for it.

```text
environment  IsographEnvironment { store, graph, retainedQueries, networkFunction, networkRequestCache, subscriptions }
             normalizeData, garbageCollectEnvironment, readStoredResult, dispose, listen, snapshot
store        Store = Map<EntityName, Map<RecordId, StoreRecord>>
             normalize, check, garbageCollect, read, readButDoNotEvaluate
graph        RecomputationGraph<ResolverResult<unknown>> { derivedNodes, sourceDependents, owned, queue, observers, valuesEqual }
             track, markChanged, deleteSource, retainRoot, collect, settle, observe, valueOf, notify
```

- The store is store.md's nested map, unchanged. Its functions take the `Store` and import nothing from the graph or the environment. `normalize` returns the changes it made (change 1), `garbageCollect` returns the records it deleted (change 2), and the reader's walk reports each read to the `ReadHooks` its caller passes (change 2).
- The engine, `graph.ts`, knows two kinds of id, `SourceId` and `DerivedNodeId`, both opaque and both minted by its caller (change 2). It imports nothing from the store, the reader, or the environment. Its state is one value, `RecomputationGraph`.
- The environment (network-and-subscriptions.md, The environment) holds the store and the graph as two fields, beside the retained queries, the network function, the request cache, and the subscriptions, as upstream's `IsographEnvironment` holds `store` beside its other state (`IsographEnvironment.ts:73-93`). Its functions are the only ones that read or write both: `normalizeData` passes `normalize`'s changes to `markChanged`, `garbageCollectEnvironment` passes each record `garbageCollect` deleted to `deleteSource`, and `readStoredResult` reads a resolver through `track` with hooks that report each read to the computation's `ComputeContext`.
- `sources.ts` maps store locations to ids (`recordSource`, `valueSource`, `resolverDerivedNode`, `changeSource`). It imports the store's types and the graph's id types. The environment's functions and the hooks are its only callers.

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
- A caller that writes `environment.store` with `normalize` directly, and not through `normalizeData`, leaves the derived nodes that read what it changed unmarked, and they return their old results until another write reaches them. From change 2b the package index exports `normalizeData` and not `normalize` (open decision 17), so such a caller is code inside the package. `normalize` stays a module export that `environment.ts` and the tests import, as store.md's helpers are (store.md, The package). This is network-and-subscriptions.md's rule for `callSubscriptions` (Why the store-only functions stay store-only): a write through the environment is `normalizeData`.

### Why a store-agnostic engine

The alternative (open decision 12 (b)) writes the recomputation graph in the store's shape:

- Derived nodes keyed by `EntityName`, record id, and resolver store key, each carrying its `StoreLink` and `StoreKey`.
- The inverse of server dependencies keyed by record, with one set for a record read while absent and one set per store key.
- Dependencies that name `StoreLink`s and store keys.
- A change type (`RecordCreated | RecordDeleted | ValueChanged`) whose dependents are found through that record grouping: a deleted record marks every set of the record and the dependents of every derived node stored on it.
- Marking called with changes to the one base map, on the assumption (R8) that `normalize` is the only writer.

A layered store breaks the last two. With an optimistic layer over the base layer (upstream's `StoreLayer` chain, `optimisticProxy.ts:24-98`), a value's effective value is the topmost layer's: a base write under a shadowing layer changes no effective value and marks derived nodes for nothing, and removing a layer changes effective values with no `normalize` call, so nothing marks. Upstream handles removal by comparing the old and new merged data and calling `callSubscriptions` with the records that differ (`revertOptimisticStoreLayerAndMaybeReplace`, `optimisticProxy.ts:393-493`; `compareData`, `:506-544`). That obligation exists whatever the engine looks like; the coupling question is whether the engine's types and functions name records, store keys, and layers. Under the held design they name none of them.

### What the environment must report

Two obligations, and nothing else couples the engine to the store. The store's functions meet neither alone: `normalize` and `garbageCollect` return what they changed and the walk reports through hooks, and the environment's functions forward both to the graph.

- Report every read. Each read a computation makes goes through the walk, whose hooks call `readSource` with `recordSource` and `valueSource` of the location read. For a layered store this is the effective location: one source per (record, store key), whichever layer answered.
- Report every effective change. After an operation that changes what a read would return, the environment calls `markChanged` with every source whose effective value changed, and `deleteSource` for each record whose effective presence went from present to absent (from absent to present is `markChanged` of its `recordSource`). It may report more: a source reported but unchanged costs a recomputation of each needed reader, which cuts off by `===` or recomputes to an equal result, and a `Stale` mark on each other reader. It must not report fewer: an unreported change is a stale result with no error. Settlement runs after the operation and before notification.

For the layered store of Later's optimistic writes:

- A write to the topmost layer that holds a key reports it when `isUnchanged` (change 1) rejects it against the previous effective value. A write to a layer that a higher layer shadows at that key reports nothing.
- Adding a layer reports each key it holds whose value differs from the previous effective value, and each record it creates.
- Removing a layer reports each key the removed layer, or a layer re-executed above it, wrote whose effective value differs after the removal, and `deleteSource` for each record that existed only in the removed layer. This is upstream's `compareData` (`optimisticProxy.ts:506-544`) at store key granularity instead of record granularity, comparing by change 1's AST shape: each layer keeps the `WrittenValue` of every key it wrote, so a list of scalars compares element by element. Without that, `===` reports every list of scalars, which is correct and costs recomputation.

### Lifetime still follows records

A derived node's `owner` is the `recordSource` of the record it was computed on. `deleteSource(graph, recordSource(record))` invalidates every reader of the record (every reader of any key on it read its presence), invalidates the dependents of every derived node the record owns, and disposes those nodes. A store-coupled engine does the same through its record grouping; here the engine does it without knowing what a record is. An invalidated dependent recomputes, by the queue when it is needed or by the next read when it is `Stale`, finds the record absent, and records only `recordSource`, with no derived node (change 2, the store's side). No computation reaches a disposed derived node: every one of its dependents was invalidated, a recomputation reads the record's presence instead, and a `Stale` node recomputes without walking its old dependencies.

components-in-userland.md says a closure field's wrapper is "stored on the record, and freed with it". "Stored on the record" does not hold literally: derived nodes live in `environment.graph`, beside the store. It holds as "keyed by the record": the wrapper is the value of the derived node `resolverDerivedNode(record, storeKey)` in `graph.derivedNodes`. "Freed with the record" holds through `owner`: deleting the record calls `deleteSource` with its `recordSource`, which disposes the wrapper's derived node. The wrapper's computation reads only the record's presence (its `createWrapper` receives no data), so no write recomputes it while the record lives.

### Costs of the ids

- An id per read. Each server read in a computation builds two strings (`recordSource`, `valueSource`) and each resolver read one (`resolverDerivedNode`; `resolverStoreKey` is built in any design), and `setDependencies` does one `Map` lookup per id. A store-coupled engine does the same reads as three nested `Map` lookups with no string built. The order of growth is the same; the string building is the extra, and it is not measured. An interned object per (entity, id, store key) avoids the string but needs an intern table, whose members need the same lifetime rule as `sourceDependents` (deleted when no node reads them), so it is `sourceDependents` again under a second name; the string is held.
- A presence edge per (derived node, record read): one more `sourceDependents` set member than a store-coupled engine keeps for a node that read a present record.
- A map lookup instead of a field: a derived node does not sit in a per-record map, so deleting a record finds its nodes through `owned`, one more map kept by node creation and disposal.
- The casts in the id functions.

What the opaque ids buy: the engine has no import from the store, so a layered store, another record layout, or a source that is not server data (a client-only value, a clock) is a `SourceId` and two calls. R8 becomes the environment's obligation above, stated once, instead of a property of `normalize`. The engine is testable with literal ids.

## Values with lifecycles

Status: design analysis, held options, not decided. Nothing in changes 1 to 4 produces a value with a lifecycle; this lands with the first feature that does (Follow-ups 1, 2, 4), and changes 1 to 4 must not preclude it.

A stored result can be a handle: a value with a constructor and a destructor, reference-counted, the `ItemCleanupPair<T>` shape of `@isograph/disposable-types` (`isograph-disposable-types/index.ts:2`). Examples: a client pointer's follow-up query (Follow-ups 2), a handle a retain returns (Follow-ups 4), a closure component whose wrapper holds a subscription. Held 3a handles them if the engine owns each derived node's value lifecycle: a node acquires its value when it computes, and releases it when the value is replaced or the node is disposed.

### The hook

A derived node's computation is created with a kind; the engine cannot tell an `ItemCleanupPair` from a plain array value at run time, so the kind comes from compiler-written data (the artifact's kind), not from the returned value.

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

/** What a derived node holds. */
export type Holding<TValue> =
  | { readonly kind: 'Value'; readonly value: TValue }
  | { readonly kind: 'Resource'; readonly pair: ItemCleanupPair<TValue> };
```

These types replace change 2's `Computation<TValue>`. `DerivedNode.value` becomes `holding: Holding<TValue>`; the derived node's value is `holding.value` or `holding.pair[0]`. `RecomputationGraph` gains `readonly pendingReleases: CleanupFn[]`, the cleanups of replaced and disposed values not yet run, and:

```ts
// from libs/isograph-runtime/src/graph.ts
/** Runs and empties `pendingReleases` in order; then rethrows one error, or an `AggregateError` of several. */
export function flushReleases<TValue>(graph: RecomputationGraph<TValue>): void;
```

For the runtime, a resolver whose artifact is disposable reads its data as any resolver does; on `Ok` it calls the constructor with the data and returns `Constructed` with `[ok(handle), cleanup]`; on `MissingData` it returns `NotConstructed`.

Contract on a constructor: a constructor that can return a `===` value on two calls (a handle deduplicated by key) reference-counts it, so each pair's cleanup releases only its own retain. Follow-ups 5's registry is that count; each holding derived node holds one retain.

### Replacement and release

`recompute` of a `Disposable` derived node, after `setDependencies`:

1. The computation runs, so the new value is constructed before anything is released. A shared resource reached again (the same follow-up for the same target) gets a second retain, not a teardown and a rebuild.
2. When the new value is `valuesEqual` to the old (`===` for `Ok`), the derived node keeps its old holding, and the new pair's cleanup runs at once: it drops the second retain while the first still holds the resource, and no observer sees a change. Dependents are not marked: R4's cutoff works for handles deduplicated by key.
3. Otherwise the derived node holds the new holding, the old pair's cleanup is pushed to `pendingReleases`, and the dependents are invalidated, as change 4's `recompute` does.

Disposing a derived node (`collect`, `deleteSource`) pushes its pair's cleanup to `pendingReleases`. `normalizeData` runs `settle`, the new root's `retainRoot`, `notify`, then `flushReleases`; `garbageCollectEnvironment` runs `settle`, `notify`, and `flushReleases` at its end.

Ordering guarantees:

- A constructor runs inside its derived node's computation, so after every input it reads is current (R3), and at most once per node per pass.
- A new value is constructed before the old one is released.
- No value is released while a derived node holds it.
- A replaced or disposed value is released after every observer of that settlement was notified, so a `snapshot` in a callback never returns a released value, and each cleanup runs exactly once, in the order the values were replaced or disposed.
- Under React, release after notification is not late enough: a concurrent render can still be on screen showing the old value until the commit of the re-render. The engine's retain is one retain among others. The React package takes its own retain on the handle it renders: a temporary retain at render, converted to a permanent retain at commit, and released when the component unmounts or commits a newer value (network-and-subscriptions.md, the two-phase lifetime; `CacheItem.temporaryRetain`, `CacheItem.ts:195-256`, and `permanentRetainIfNotDisposed`, `:114-193`). The handle is destroyed when the engine's release and React's both happened, so a committed tree never holds a destroyed handle. The handle the constructor returns is therefore retainable by others: a `CacheItem`, or an entry of a registry that counts retains as network-and-subscriptions.md change 7's `RequestEntry` does.

Garbage collection: `collect` and `deleteSource` dispose derived nodes, which releases their values. Under held open decision 11 (a), a derived node no root reaches keeps its value until the next `garbageCollect`, and under held 3a it is `Stale` or `Clean` and not recomputed until read: the derived node of a pointer whose target moved keeps its old follow-up alive until collection. That is Follow-ups 7's batched destruction; 11 (b) would release at the release of the last root instead.

### Failures

A constructor that throws. Options:

- (a) The derived node's value becomes `ResolverThrew` (open decision 5 (a)), held as `NotConstructed`; the old value is released through `pendingReleases`, as any replacement; the dependents are invalidated. Held. Reason: `setDependencies` has already recorded the new computation's reads, so a derived node that kept its old value would hold a value that no recorded input explains, and it would stay stale until one of the new inputs changes.
- (b) Keep the old holding and value, and record the error elsewhere. Cost: the stale value above, and a second place an error lives.
- (c) Propagate out of `normalizeData`. Cost: open decision 5 (b)'s, every needed derived node left unsettled.

A destructor that throws. Options:

- (a) `flushReleases` runs every remaining cleanup, then rethrows one error as is or several as an `AggregateError`, open decision 9's rule; `normalizeData` and `garbageCollectEnvironment` combine them with `notify`'s errors into one `AggregateError` when both threw. The derived node is unaffected: it dropped the value before the cleanup ran. Held.
- (b) Stop at the first throw. Cost: later resources leak.

A cleanup is never called twice by the engine; a handle's own second-call throw (`CacheItem.ts:125-130`) is not reachable from it.

### Relation to Follow-ups 1 to 7

- 1, constructors and destructors: the `Disposable` computation is the constructor, at the derived node's first computation; the cleanup is the destructor, run when the derived node's value is replaced or the node is disposed. "The last handle referencing that derived node" is the handle's own count: the engine holds one retain per holding node, React holds its own.
- 2, client pointer and follow-up query: the pointer's derived node returns `[ok(handle), cleanup]` for the follow-up of its target. A recomputation that returns another id constructs the new follow-up and then releases the old one after notification; the parent retention's `dispose` releases it through `collect`.
- 3, a pointer whose target it read: no lifecycle; it removes the follow-up instead.
- 4, handles: the derived node's value is the handle; `snapshot` returns it, a lookup, readable during render.
- 5, deduplication: under R6, two retained queries that reach the same pointer on the same record already share one derived node, so the key Follow-ups 5 names (the record, the selectable, the variables) is the node's id and needs no registry. Sharing across different derived nodes (two pointers that return the same target) needs a key of the follow-up itself: the target link, the target fragment's selectable, and its variables. A recomputation to a new target constructs under the new key.
- 6, a destructor and an in-flight follow-up: the deferred release calls the follow-up's cleanup, which aborts its request through the `AbortSignal` the network function accepts.
- 7, deferral: `flushReleases` is the batch. It can be handed to the caller-supplied scheduler instead of run at the end of `normalizeData` and `garbageCollectEnvironment`, and a handle released and retained again before the batch runs is not destroyed, because the second retain reached it first. A constructor that defers its own setup returns the handle at once and schedules the rest; the engine sees only the pair.

## Steps

Depends on the held architecture (3a with heights per derived node and `Stale` marks) in changes 2 to 4.

Each change is one commit, in order; change 2 is two. Step A is optional and independent of 1 to 4.

- Step A. Re-read every subscription after each write, upstream's mechanism, with v1's `read`. No memoization.
- 1. Leaf selections in the normalization AST record their list depth, and `normalize` skips an unchanged value and returns the changes it made (prefactor). `Store` stays store.md's nested map.
- 2a. `graph.ts`, the engine's lazy half, over opaque `SourceId`s and `DerivedNodeId`s, importing nothing from the store, the reader, or the environment: derived nodes, `setDependencies`, `markChanged` marking `Stale`, `bringUpToDate`, `track`, `deleteSource`. Its tests use literal ids.
- 2b. Stored resolver results. The environment gains `graph`. The reader's walk takes `ReadHooks`, and `readStoredResult` reads a resolver through the graph, storing each resolver's result with its dependencies and their inverse. `normalizeData` marks `Stale` every derived node that read what `normalize` changed and every node that reads those, transitively, stopping at a node already `Stale`; `garbageCollectEnvironment` does the same for each record `garbageCollect` deleted and disposes the derived nodes the record owned. A read of a `Stale` derived node recomputes it, each node it reads brought up to date by that read first. There are no observers, so nothing is needed, nothing is queued, and no height exists.
- 3. Level 1 retention. `normalizeData` takes the entrypoint, computes its root result and every result that result reaches by a read of the root, holds the root in the graph, and adds its `Retention` to `environment.retainedQueries`. `garbageCollectEnvironment` disposes the derived nodes no held root reaches. A later write marks a retained query's affected derived nodes `Stale`.
- 4. Level 2 and the height-ordered push. Derived nodes gain `height`, set from what each computation read and raised by adjustment after an upward read, `neededCount`, and the state `Queued`; the graph gains the bucket queue and the observers. `listen` brings its derived node up to date and adds to the needed counts; `snapshot` is a lookup. A write queues each needed derived node whose input changed and marks the others `Stale`; `settle` drains the queue lowest height first, queuing the direct dependents of each derived node whose result changed. Observers are notified once, after the pass. `garbageCollectEnvironment` settles and notifies after its deletions.

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

The list's reader is `normalizeData` (change 2), which maps each change to a `SourceId` and calls `markChanged`. v1's callers (network-and-subscriptions.md's `makeNetworkRequest` until change 2, and store.md's tests) ignore it. A record written at two positions of one response reports `RecordCreated` once. A key written twice in one response with two different values reports `ValueChanged` twice; marking a derived node twice is the same as marking it once.

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

Its reader is the returned list: an unchanged write reports nothing, so change 2 marks nothing for it. The check exists because without it every response marks every written field changed, so every refetch of a retained query recomputes every derived node under it, and the whole retained tree recomputes on every response. Whether the comparison earns its cost against that recomputation is not measured.

A list of scalars (`tags: [String]`) is a new array on every response. Compared with `===`, it would count as changed on every rewrite, and every derived node that reads it would recompute on every response. Lists of scalars are common in GraphQL schemas, so the AST records each leaf selection's list depth and `leafUnchanged` compares such a list element by element. A `JSON` custom scalar is also a new object on every response; it stays `===` and recomputes its readers on every response.

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

A resolver's result is stored in `environment.graph` as a derived node, under an id built from the record it was read on and the client field's resolver store key. Reading a resolver whose inputs did not change returns the stored result without calling the resolver. `normalizeData` and `garbageCollectEnvironment` mark `Stale` the derived nodes that read what `normalize` and `garbageCollect` changed, and the nodes that read those; the next read of a `Stale` derived node recomputes it. This change has no observers, so no derived node is needed and every mark is `Stale`; change 4 queues the needed derived nodes instead.

A resolver derived node, in the rest of this doc, is the derived node of one resolver store key on one record.
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
declare const sourceIdBrand: unique symbol;
declare const derivedNodeIdBrand: unique symbol;

/** A value the store owns that a computation can read. The engine compares ids by `===` and never parses one. */
export type SourceId = string & { readonly [sourceIdBrand]: true };

/** A derived node. One `DerivedNodeId` names one computation for the derived node's whole life. */
export type DerivedNodeId = string & { readonly [derivedNodeIdBrand]: true };

/** The recomputation graph: derived nodes and the inverse of their dependencies. */
export type RecomputationGraph<TValue> = {
  readonly derivedNodes: Map<DerivedNodeId, DerivedNode<TValue>>;
  /** The derived nodes whose last computation read each source. Written only by `setDependencies`. */
  readonly sourceDependents: Map<SourceId, Set<DerivedNode<TValue>>>;
  /** The derived nodes created with each owner. Written only by `createDerivedNode` and `disposeDerivedNode`. */
  readonly owned: Map<SourceId, Set<DerivedNode<TValue>>>;
  /** Whether two values of a derived node are equal, for cutoff: `resultsEqual` in the runtime. */
  readonly valuesEqual: (a: TValue, b: TValue) => boolean;
};

export type DerivedNode<TValue> = {
  readonly id: DerivedNodeId;
  /** The source whose deletion disposes this derived node: the presence of the record it was computed on. */
  readonly owner: SourceId;
  readonly computation: Computation<TValue>;
  value: TValue;
  state: DerivedNodeState;
  /** What the last computation read, in read order. Written only by `setDependencies`. */
  dependencies: ReadonlyArray<GraphDependency<TValue>>;
  /** Every derived node whose `dependencies` hold a `Derived` dependency on this one. Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

/**
 * `Clean`: `value` is what a computation against the store would return.
 * `Stale`: a source it read changed, or a derived node it reads is `Stale`. It recomputes when read.
 */
export type DerivedNodeState = 'Clean' | 'Stale';

export type GraphDependency<TValue> = SourceDependency | DerivedNodeDependency<TValue>;
export type SourceDependency = { readonly kind: 'Source'; readonly source: SourceId };
export type DerivedNodeDependency<TValue> = { readonly kind: 'Derived'; readonly node: DerivedNode<TValue> };

/** A derived node's computation. It reports every source it reads, and reads other derived nodes, through `context`. */
export type Computation<TValue> = (context: ComputeContext<TValue>) => TValue;

export type ComputeContext<TValue> = {
  /** Records that the computation read `source`. */
  readSource(source: SourceId): void;
  /** Brings the derived node `id` up to date, creating it from `create()` when absent, records the
   * dependency, and returns its value. */
  readDerivedNode(id: DerivedNodeId, create: () => DerivedNodeDefinition<TValue>): TValue;
};

/** What creating an absent derived node needs. */
export type DerivedNodeDefinition<TValue> = {
  readonly owner: SourceId;
  readonly computation: Computation<TValue>;
};
```

The context is a parameter, built per computation by `compute`, as the walk's `ReadHooks` is; there is no ambient "current computation" (MobX's `globalState.trackingDerivation`, Reactively's `CurrentReaction`). The engine knows nothing about what a derived node computes: the caller supplies the computation, the owner, and the id. Resolver derived nodes are the one derived node kind changes 2 to 4 create; Other derived node kinds lists the kinds the follow-ups add and the parts of the API they need.

`Computation`, `ComputeContext`, and `DerivedNodeDefinition` are internal to the runtime. User code never receives a `ComputeContext`, an environment, or a store. A computation records a dependency only by calling `readSource` or `readDerivedNode`, so a computation that user code wrote as a closure could read a value without calling `readSource`; its derived node would not be marked when that value changes, and it would return its old result with no error. User code reads only through isograph syntax: a selection in a reader AST, which the reader walk reads, and each `ReadHooks` hook the walk calls reports the source it reads. The walk is therefore the only reader of a source on behalf of user code, and no read goes unrecorded.

A source other than a store field is exposed to user code the same way, as a selection in a reader AST, so the walk records its read as it records a server field read. The first such source is the state of a network request. Adding a source kind adds:

- isograph syntax for the selection;
- a reader AST node kind for it;
- a `ReadHooks` hook the walk calls at that node, which `contextHooks` implements by calling `context.readSource` with the kind's `SourceId` constructor in `sources.ts`, as `readValue` calls it with `valueSource` (for a request, `requestSource(key: RequestKey): SourceId`);
- in every writer of that source, the source among the sources it passes to `markChanged`, as `normalizeData` passes `changeSource` of each `StoreChange`, and `deleteSource` of the source in a writer that deletes what the source names, as `garbageCollectEnvironment` calls it with `recordSource` of each deleted record (for a request entry, network-and-subscriptions.md, Request entries in the store).

The syntax and the reader AST node's shape are not decided. Which request a request-state selection refers to is open decision 20.

Derived node kinds the runtime itself defines, such as the suspension decision (Other derived node kinds), are runtime code, and their computations call `ComputeContext` directly.

A derived node is mutated in place (`value`, `state`, `dependencies`), because other nodes' `dependents` sets and `DerivedNodeDependency`s hold it by identity. A source dependency is a `SourceId`, because a server value has no object to point at and a record may not exist yet. A derived node dependency is the derived node, because `dependents` holds nodes and change 4's needed counts walk from a node to the nodes it reads. `graph.ts` imports `getOrInsert` from `util.ts` and nothing else.

```ts
// from libs/isograph-runtime/src/graph.ts
export function createGraph<TValue>(
  valuesEqual: (a: TValue, b: TValue) => boolean,
): RecomputationGraph<TValue> {
  return { derivedNodes: new Map(), sourceDependents: new Map(), owned: new Map(), valuesEqual };
}

/** The value of the derived node `id`, created from `create()` when absent and brought up to date.
 * Records no dependency: the caller is not a derived node. */
export function track<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  create: () => DerivedNodeDefinition<TValue>,
): TValue {
  return upToDateDerivedNode(graph, id, create).value;
}

/** Marks `Stale` every `Clean` derived node that read one of `sources`, with its `Clean` dependents,
 * transitively. Runs no computation. */
export function markChanged<TValue>(
  graph: RecomputationGraph<TValue>,
  sources: ReadonlyArray<SourceId>,
): void {
  sources.forEach((source) => graph.sourceDependents.get(source)?.forEach(markStale));
}

/** `markChanged(graph, [source])`, then marks the dependents of every derived node `source` owns, and
 * disposes those derived nodes. */
export function deleteSource<TValue>(
  graph: RecomputationGraph<TValue>,
  source: SourceId,
): void {
  markChanged(graph, [source]);
  const owned = graph.owned.get(source);
  owned?.forEach((node) => node.dependents.forEach(markStale));
  owned?.forEach((node) => disposeDerivedNode(graph, node));
}
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** The derived node `id`, created from `create()` when absent, and brought up to date. */
function upToDateDerivedNode<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  create: () => DerivedNodeDefinition<TValue>,
): DerivedNode<TValue> {
  const node = graph.derivedNodes.get(id) ?? createDerivedNode(graph, id, create());
  bringUpToDate(graph, node);
  return node;
}

/** Marks a `Clean` derived node `Stale`, then its dependents. A derived node already `Stale` is left as is. */
function markStale<TValue>(node: DerivedNode<TValue>): void {
  if (node.state !== 'Clean') {
    return;
  }
  node.state = 'Stale';
  node.dependents.forEach(markStale);
}

/** Recomputes a `Stale` derived node, then marks it `Clean`. Does nothing to a `Clean` derived node. */
function bringUpToDate<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
  if (node.state === 'Clean') {
    return;
  }
  recompute(graph, node);
  node.state = 'Clean';
}

/** Computes a new derived node, stores it `Clean` in `derivedNodes` and under its owner in `owned`, and
 * records its dependencies. */
function createDerivedNode<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  definition: DerivedNodeDefinition<TValue>,
): DerivedNode<TValue> {
  const computed = compute(graph, definition.computation);
  const node: DerivedNode<TValue> = {
    id,
    owner: definition.owner,
    computation: definition.computation,
    value: computed.value,
    state: 'Clean',
    dependencies: [],
    dependents: new Set(),
  };
  setDependencies(graph, node, computed.dependencies);
  graph.derivedNodes.set(id, node);
  getOrInsert(graph.owned, node.owner, () => new Set()).add(node);
  return node;
}

/** Recomputes `node`; stores the new value when `valuesEqual` rejects it against the old one. */
function recompute<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
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
    readSource: (source) => {
      dependencies.push({ kind: 'Source', source });
    },
    readDerivedNode: (id, create) => {
      const node = upToDateDerivedNode(graph, id, create);
      dependencies.push({ kind: 'Derived', node });
      return node.value;
    },
  });
  return { value, dependencies };
}

/**
 * Replaces `node.dependencies` with `next`, and removes `node` from the dependents of each old
 * dependency and adds it to those of each new one. Deletes a `sourceDependents` set left empty.
 */
function setDependencies<TValue>(
  graph: RecomputationGraph<TValue>,
  node: DerivedNode<TValue>,
  next: ReadonlyArray<GraphDependency<TValue>>,
): void;

/** `setDependencies(graph, node, [])`, then removes `node` from `derivedNodes` and from its owner's
 * `owned` set, deleting the set when it is left empty. */
function disposeDerivedNode<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void;
```

A `Stale` derived node recomputes without checking whether anything it read changed. Its reads of other derived nodes go through `readDerivedNode`, which brings each one up to date first, so its computation sees current inputs (R3). It recomputes once: it ends `Clean`, and only marking makes it `Stale` again, and marking does not run during a read. `recompute` marks no dependent: by the invariant below, every dependent of a `Stale` derived node is `Stale` already, except the parent whose read reached it, which is computing and ends `Clean` having read the new value. Keeping the old value when `valuesEqual` holds keeps its identity, which change 4's cutoff and notification read.

A read of `Stale` derived nodes has no equality cutoff: after `Rex` to `Ray`, the next read of `BadgePage` recomputes `Monogram`, `Signature`, `Badge`, and `BadgePage` although `Initial`'s result is `===` the old one (open decision 15). Change 4 gives needed derived nodes the cutoff.

Invariant, outside a call that is computing a derived node: every dependent of a `Stale` node is `Stale`. Marking keeps it, because a derived node that becomes `Stale` marks its dependents. A new dependent is added only by a computation, which brought the derived node up to date first, and a recomputed node ends `Clean` only after every node it read was brought up to date. `markStale` stops at a derived node already `Stale` because the invariant says its dependents are `Stale` too, so a second write to a source costs one step per node that read it.

The marking is transitive because nothing computes during a write: whether a derived node's value changes is known only by computing it. Change 4 keeps this marking for derived nodes nobody needs and replaces it, for needed nodes, by queuing the direct dependents of a node whose value changed.

A derived node's dependents of both kinds together are the inverse of the dependency lists: `node.dependents` for `Derived` dependencies, `sourceDependents` for `Source` dependencies. `setDependencies` is the only writer of either side; a violation is a silently stale result. Removing every old edge and adding every new one is `O(old + new)`; Reactively keeps the matching prefix instead (`update`, `packages/core/src/core.ts`), which saves set operations and not order of growth. A dependency recorded twice (two selections of one field on one record) is one edge in each set.

### The store's side (2b)

```ts
// from libs/isograph-runtime/src/sources.ts
/** The source every read on `record` reports, whether the record is present or absent. It
 * changes when the record is created or deleted. */
export function recordSource(record: StoreLink): SourceId {
  return JSON.stringify(['Record', record.entityName, record.id]) as SourceId;
}

/** The source a lookup of `storeKey` on a present `record` reports. */
export function valueSource(record: StoreLink, storeKey: StoreKey): SourceId {
  return JSON.stringify(['Value', record.entityName, record.id, storeKey]) as SourceId;
}

/** The derived node of the resolver store key `storeKey` on `record`. */
export function resolverDerivedNode(record: StoreLink, storeKey: StoreKey): DerivedNodeId {
  return JSON.stringify([record.entityName, record.id, storeKey]) as DerivedNodeId;
}

/** The source `change` changed. */
export function changeSource(change: StoreChange): SourceId {
  switch (change.kind) {
    case 'RecordCreated':
      return recordSource(change.record);
    case 'ValueChanged':
      return valueSource(change.record, change.storeKey);
  }
}
```

The casts are the only places a `SourceId` or `DerivedNodeId` is minted. Each id is the JSON of a list of strings, so no two locations share an id, whatever characters an entity name, record id, or store key holds.

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
  // The derived node under `artifact.selectableName`'s resolver store key on `root` was computed by
  // `artifact`: one selectable of one entity has one reader artifact.
  return track(
    environment.graph,
    resolverDerivedNode(root, resolverStoreKey(artifact.selectableName, variables)),
    () => resolverDefinition(environment.store, artifact, root, variables),
  ) as ResolverResult<TOutput>;
}

/** The derived node definition of `artifact` on the present record `root` with `variables`. */
export function resolverDefinition(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  variables: Variables,
): DerivedNodeDefinition<ResolverResult<unknown>> {
  return {
    owner: recordSource(root),
    computation: (context) => {
      const data = readData(store, contextHooks(store, context), artifact, root, variables);
      return data.kind === 'Ok' ? callResolver(artifact, root, data.value, variables) : data;
    },
  };
}

/** Hooks that report each read to `context` and read each nested resolver as a derived node. */
function contextHooks(
  store: Store,
  context: ComputeContext<ResolverResult<unknown>>,
): ReadHooks<ReadResolverThrew> {
  return {
    readRecord: (record) => context.readSource(recordSource(record)),
    readValue: (record, storeKey) => context.readSource(valueSource(record, storeKey)),
    readResolver: (artifact, root, variables) => {
      context.readSource(recordSource(root));
      if (storeRecord(store, root) === undefined) {
        return noRecord(root);
      }
      return context.readDerivedNode(
        resolverDerivedNode(root, resolverStoreKey(artifact.selectableName, variables)),
        () => resolverDefinition(store, artifact, root, variables),
      );
    },
  };
}
```

The cast in `readStoredResult` is the one place a stored `ResolverResult<unknown>` is returned at the caller's type; the invariant is in its comment. The computation closes over `store`, the environment's `Map`, which is written in place and never replaced (network-and-subscriptions.md, The environment).

Every reader of any key on a record also reported the record's presence, so a deletion reports one source instead of the record's keys, including keys read while absent, which the store cannot enumerate. A computation that stops at the first `MissingData` records the sources read up to it, which are exactly what its result depends on.

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
/** `normalize` of `response` into `environment.store`, then marks `Stale` every derived node that read
 * what it changed. On a `PartialWriteError`, marks the changes it carries and rethrows it. */
export function normalizeData(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void {
  markChanged(environment.graph, writeOrMark(environment, ast, response, variables).map(changeSource));
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
      markChanged(environment.graph, error.changes.map(changeSource));
    }
    throw error;
  }
}

/** `garbageCollect` of the store, keeping what the retained queries reach, then deletes the
 * derived nodes of each deleted record and marks their readers. */
export function garbageCollectEnvironment(environment: IsographEnvironment): void {
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteSource(environment.graph, recordSource(record)),
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

v1's `ReadResult` is unchanged, so `read` and `readButDoNotEvaluate`, which never produce `ResolverThrew`, do not admit it in their types. v1 lets a throw propagate out of `read` (reader-ast.md, Semantics), and still does. In a stored result it is a value, because changes 3 and 4 run resolvers inside `normalizeData` (the new root's, and every needed derived node's): a throw that propagated would abort `normalizeData`, and leave the other observed nodes unsettled. Open decision 5.

Equality is upstream's for resolver values, `===` (`areEqualWithDeepComparison.ts:84-95`), as pico's is the value's `==`. It is R4's optimization, not a guarantee. A resolver that returns a `===` primitive or the same object queues none of its dependents (change 4); one that returns a new object is unequal on every recomputation, structurally equal or not, and every derived node that reads it recomputes. `MissingData` is compared field by field because the runtime builds it from strings and links, where structural equality is well-defined. Open decision 7 settles `===`.

### Tests

`test/read.test.ts` (reader-ast.md, Tests, Runtime) is unchanged: `read` and `readButDoNotEvaluate` keep v1's behavior, call every resolver on every read, leave the store unchanged, and propagate a resolver's throw.

New, in `test/graph.test.ts` (2a), with literal ids and computations that read sources from a map the test holds and report them through `context.readSource`:

- A derived node read twice with `track` with no `markChanged` between: its computation runs once, and both values are `===`.
- `markChanged` of a source the derived node read: the next `track` runs the computation once. Of a source it did not read: none. Of `[]`: none.
- A derived node whose computation reads another node: `markChanged` of the inner node's source recomputes both on the next `track` of the outer; the inner computation runs before the outer's finishes.
- A recomputed value that `valuesEqual` accepts keeps the old value object (`toBe`).
- Dependencies follow the last computation: a computation that read `a` and then, after `markChanged([a])`, reads `b` instead is not marked by a later `markChanged([a])`, and `sourceDependents` no longer holds a set for `a`.
- `deleteSource` of a derived node's owner: `derivedNodes` no longer holds the node, `owned` no longer holds the owner, and an outer node that read it is `Stale`.
- Degenerate: `deleteSource` of a source nothing owns or read changes nothing.

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

- `environment.store` is unchanged by every `readStoredResult`: it is `toEqual` the same `storeOf` before and after. After `PetSummary` with `{ id: "1" }`, `environment.graph.derivedNodes` holds `resolverDerivedNode` of `Query.__ROOT` and `PetSummary____id___"1"`, of `Pet:1` and `PetName`, and of `Pet:1` and `RecentCheckins____n___2`, and no other.
- A second read with no write between: every resolver's call count is unchanged, and the result is `===` the first read's. (v1's `read`: every count doubles.)
- Diamond, `Pet:1.name` `Rex` to `Max`: `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once; `Badge` receives `{ Monogram: "M.", Signature: "M " }` on its one call; `Initial` is called once although `Monogram` and `Signature` both select it.
- Diamond, `Rex` to `Ray`: on the next read, `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once, and `Initial` returns `"R"`: a read of `Stale` derived nodes has no cutoff (open decision 15; change 4's Tests cover the cutoff for observed derived nodes).
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
- A deleted record: after `garbageCollectEnvironment` deletes `Pet:2`, the next read of a stored `FriendPage` result is `MissingData` with `recordLink` `Pet:2`, not the stored `Ok`, and `environment.graph.derivedNodes` holds no derived node of `Pet:2`.
- A resolver that throws: `readStoredResult` returns `ResolverThrew` with the error, the field's selectable name, and the record; its parent's result is the same `ResolverThrew` object. `read` of the same artifact throws, as in v1.
- A partial write: a response that throws part-way after changing `Pet:1.name`: `normalizeData` rethrows the `PartialWriteError`, and the next read of `BadgePage` calls `Initial` once.
- `resolverStoreKey`: `RecentCheckins` with `{ n: 2 }` is `RecentCheckins____n___2`; `{}` is `RecentCheckins`; `{ b: 1, a: "x" }` is `X____a___"x"____b___1`.

New in `test/garbageCollection.test.ts`: `garbageCollect` with nothing retained returns every record's link, in the store's order; with every record reached, `[]`.

## Ordering

Depends on the held architecture.

A write can make several stored results stale at once. R3 holds on the read path, and on the push for every read of a derived node below the reader's height. An upward read during the push breaks it in change 4's code as written; held open decision 19 (c) restores it by abandoning the computation at that read (Once per pass, below):

- Heights (change 4, Heights). A derived node's height is above the height of every derived node it reads. Its computation sets the height from what it read, and adjustment raises it and its dependents, transitively, when a recomputation reads a derived node at or above it. Change 2 has no heights.
- The read path (change 2, The engine). A `Stale` derived node recomputes when read, and each derived node its computation reads is brought up to date by that read first. It recomputes once: it ends `Clean`, and only marking, which does not run during a read, makes it `Stale` again. The recursion terminates because the runtime graph has no cycle (Cycles, below).
- The push (change 4, Settlement). `settle` drains bucket 0, then bucket 1, and so on. A derived node of height `h` recomputes after every bucket below `h` is empty. Each derived node it reads is `Clean`, `Queued`, `Stale`, or absent, and the read brings a `Queued`, `Stale`, or absent one up to date before returning; under held open decision 19 (c), a read of a stored derived node at or above `h`, `Queued` included, abandons the computation instead. A derived node it reads below `h` is current: everything that derived node reads is lower still, and those buckets are empty. During the drain of bucket `h`, two things add to the queue, and both add above `h`. First, a recomputed derived node whose result changed queues its dependents. That derived node was `Queued` at a height at or above `h`, so its dependents are above `h`, or it was `Stale`, so its dependents are `Stale` (change 2's invariant) or the derived node computing, and it queues none. Second, adjustment moves a raised `Queued` derived node to a higher bucket. The derived node that started the raise recomputed during the drain of `h`, so it was `Queued` at a height at or above `h` or was `Stale`. Its `Queued` dependents are above its old height, so above `h`, and adjustment only raises them. A `Stale` derived node's dependents are `Stale` or computing, and neither is in a bucket. So no bucket at or below `h` gains a derived node once its drain has started.
- Once per pass. A derived node recomputes at most once per pass when every derived node it reads is below it. An upward read can break this. A derived node `X` of `field Pet.Y { bestFriend { FriendCard } }` on `Pet:5`, observed, with `Pet:5.bestFriend` `null`, has height 0, and `FriendCard` on `Pet:1` is needed by an observer at `FriendPage`, at height 2. One write sets `Pet:5.bestFriend` to `Pet:1` and changes `Pet:2.name`, which queues `X` and `PetName` on `Pet:2` at 0. When `X` drains first, it reads `FriendCard` on `Pet:1`, `Clean`, which returns its result from before the write, because `PetName` on `Pet:2` below it has not recomputed; `X` rises to 3. Then `PetName` on `Pet:2` recomputes and its change queues `FriendLabel` at 1, `FriendLabel` queues `FriendCard` at 2, and `FriendCard` queues `X` at 3, which recomputes a second time. Every result is current when `settle` returns, and observers are notified once, after it; `X`'s resolver was called twice, once with stale data, which R3 excludes. Under held open decision 19 (c), `X`'s computation is abandoned at its read of `FriendCard` on `Pet:1`, before `X`'s resolver runs; `X` rises to 3 and is queued there, and recomputes once, after `FriendCard`. A read during `settle` of a `Queued` derived node, by the drained derived node or by any derived node computed on demand under it, abandons the same way.

Equality cutoff is `recompute`'s, by `resultsEqual`, on the push: a derived node whose result is `===` its previous one queues nothing. A new object always queues its dependents. The read path has no cutoff (open decision 15). Cutoff is an optimization (R4); R3 holds whether or not any result cuts off. The traces are in Candidate 3a.

### The same resolver selected twice

Two selections of one client field on one record with equal argument values are one store key, so one derived node: computed once, read by both parents, each parent recording a dependency on the same derived node and appearing once in its `dependents`. This holds across fragments and across queries. Unequal argument values are two keys and two derived nodes. Aliases do not enter the key.

### Reads during recalculation

- A level 2 callback never runs during recalculation: change 4 calls callbacks only after `settle` has emptied the queue, so every needed derived node is `Clean`. A callback that calls `snapshot` receives the stored result, which is current for every needed derived node; one that calls `readStoredResult` receives a current result for any derived node, because the read recomputes a `Stale` one. A callback that calls `normalizeData` runs a nested write, settlement, and notification; when it returns, the outer notification loop compares each remaining observer's stored result with the one it last notified, so no observer is called twice for one result or receives an older result after a newer one.
- A `readStoredResult` outside a write is the read path. It meets no `Queued` derived node, because `normalizeData` and `garbageCollectEnvironment` empty the queue before they return.
- A resolver that reads or writes the store (through a closure, since its parameter holds no store) during its own computation breaks the guarantee: its inner `readStoredResult` records no dependency on the outer derived node, and an inner `normalizeData` marks and queues derived nodes while an outer recomputation is reading them. Open decision 8.
- Cycles: none under changes 1 to 4. The static reader graph has none (a cycle of client field selections is an `InlineCycle` query error, query-generation.md, Cycles), and every runtime dependency goes to a derived node of a field the reader AST selects (Candidate 3a), so no derived node is reached again while it computes, marking terminates, and adjustment never reaches the dependency that started it. An eager client pointer makes a cycle of derived nodes possible (open decision 18).

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
 * Writes `response` with `normalize` and marks what it changed, recomputes the derived nodes observers
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
  readonly location: DerivedNodeLocation;
  /** Removes the root hold `retainRoot` added. */
  readonly releaseRoot: CleanupFn;
};

/** Where a resolver derived node lives: its id is `resolverDerivedNode(record, storeKey)`. */
export type DerivedNodeLocation = {
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
export type DerivedNode<TValue> = {
  // .. change 2's fields, and:
  /** The holds `retainRoot` added at this derived node. Read by `collect`. */
  rootCount: number;
};

/** The value of the derived node `id`, created from `create()` when absent and brought up to date, with
 * one more root hold. The cleanup removes the hold. */
export function retainRoot<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  create: () => DerivedNodeDefinition<TValue>,
): ItemCleanupPair<TValue>;
```

`createDerivedNode` starts `rootCount` at 0. The cleanup closes over the derived node object, so it is safe after `deleteSource` disposed the node.

`normalizeData` does, in order:

1. `normalize(environment.store, entrypoint.normalizationAst, response, variables)`, and `markChanged` of its changes (change 2).
2. `settle(environment.graph)` (change 4).
3. Holds the new retention's root (Computing the new root).
4. Adds the retention to `environment.retainedQueries`, after the root hold returns, so a throw leaves nothing retained that the caller holds no handle to.
5. `notify(environment.graph)` (change 4).

A derived node is live while a held root or an observed location reaches it through `Derived` dependencies. Several retentions of one entrypoint with equal variables have one `location` and one derived node, with one root hold each; a resolver shared by several queries is one derived node reached from several roots. The reference count of the user's design is implicit: a derived node stays live while any held root or observed location reaches it, and `dispose` of one retention leaves it live while another does. An explicit count would be computed from the root holds, the observers, and `dependents` (change 2); open decision 11.

### Computing the new root

Depends on the held architecture.

```ts
// from libs/isograph-runtime/src/environment.ts
// in normalizeData, after normalize and markChanged (and change 4's settle), with `artifact` the
// entrypoint's `readerArtifact()` and `ast` its `normalizationAst`:
const location: DerivedNodeLocation = {
  record: ast.root,
  storeKey: resolverStoreKey(artifact.selectableName, variables),
};
const [, releaseRoot] = retainRoot(
  environment.graph,
  resolverDerivedNode(location.record, location.storeKey),
  () => resolverDefinition(environment.store, artifact, ast.root, variables),
);
```

`ast.root` is present: `normalize` inserts it before writing anything. The derived nodes the new retention reaches are computed here, children before parents: the root's computation reads each `Resolver` node through `readDerivedNode`, which computes the child, or brings it up to date, first. This is the bottom-up order the user's design creates them in on the way back up the response (Resolver markers, below). A derived node already stored and `Clean` is read as is; one that is `Stale` recomputes. A `normalizeData` of an already retained entrypoint and variables whose response changed nothing calls no resolver.

### Unobserved results are lazy

Depends on the held architecture.

After the `normalizeData` that retains a query, a later write recomputes its derived nodes only when an observer needs them (change 4). Until change 4 there are no observers, so a later write marks a retained query's affected derived nodes `Stale`, and the next read of them recomputes them: a `readStoredResult`, or the root hold of the next `normalizeData` of the same entrypoint and variables. Retention keeps records and derived nodes alive for `garbageCollectEnvironment`; it does not keep derived nodes current. Open decision 3.

### Garbage collection

Depends on the held architecture in its derived node part.

```ts
// from libs/isograph-runtime/src/environment.ts
// garbageCollectEnvironment, before (change 2):
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteSource(environment.graph, recordSource(record)),
  );
// after:
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteSource(environment.graph, recordSource(record)),
  );
  collect(environment.graph);
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** Disposes every derived node that no node with `rootCount > 0` reaches through `Derived` dependencies. */
export function collect<TValue>(graph: RecomputationGraph<TValue>): void;
```

- Records: store.md's mark and sweep, from each member of `environment.retainedQueries`, with `garbageCollect`'s parameters unchanged. A derived node a retention reaches never loses a record: it reads only what its reader ASTs select along current links, and the retained query's normalization AST is the merge of those reader ASTs with the same variables (query-generation.md), so every record it reads is marked. A derived node reached only from an observed location can lose its records (Level 2). Each deleted record goes through `deleteSource`, which disposes its derived nodes (change 2).
- Derived nodes: nodes that held roots and observed locations reach through `Derived` dependencies are kept; every other derived node is disposed by `disposeDerivedNode`, so no `dependents` set or `sourceDependents` set keeps it. A disposed derived node has no kept dependent: a kept node that read it would make it reachable. Derived nodes a plain `readStoredResult` stored outside any retention, and those of a disposed retention, are collected this way, `Stale` or not.

### Resolver markers

The user's design has `normalize` walk the response to the leaves and, on the way back up, create the resolver subscriptions each record needs. That needs `normalize` to know which resolvers the query's readers select at each position. The normalization AST does not say: query generation inlines a client field's selections and keeps no trace of it (query-generation.md, Merging). Two ways:

- (a) Root only (held). `normalizeData` knows the entrypoint's reader artifact (`Entrypoint.readerArtifact`), and holding the root computes the root derived node; computing it creates every derived node the query reaches, children first, by the same recursion that orders recalculation. Nothing new in any artifact.
- (b) A marker per position. Each `NormalizationSelectionSet` gains the client fields the readers select on that record, `{ kind: "Resolver", readerArtifact, arguments }`, with `readerArtifact` imported and `arguments` in the operation's variables with defaults applied (what query generation substituted when it inlined the field), listed children first; `normalize` reports each one when it leaves the record and `normalizeData` creates its derived node. This is candidate 6 of `refactors/pending/runtime/normalization-ast-mount-markers.md` ("A marker is needed only if a subscription must be created for a record the response introduces").

Reason for (a): every marker in (b) is already reachable from the root through reader ASTs, so (b) adds no derived node (a) does not create, and creates them in the same order. (b) also goes stale: a later response that moves `Pet:1.bestFriend` from `Pet:2` to `Pet:3` makes `FriendLabel` read `PetName` on `Pet:3`, where no marker of `FriendPage`'s response ran, so the live set has to follow dependencies anyway. And `normalization_ast.ts` would import reader artifacts, and through them the user's source modules, so loading a query's normalization AST would load its resolvers. Open decision 1.

### Tests

`test/level1.test.ts`, over the fixture of change 2, through `normalizeData` with the fixture's entrypoint artifacts, over an environment built with `createIsographEnvironment(new Map(), deferredNetwork().networkFunction)`:

- After `normalizeData` of `BadgePage`, every resolver it reaches was called once and no `readStoredResult` was made; `readStoredResult` of `BadgePage` then calls nothing and returns the stored result.
- Retained, not read: after `normalizeData` of `BadgePage` with `{ id: "1" }`, a `PetDetail` response that changes `Pet:1.name` calls none of `BadgePage`'s resolvers; the next `readStoredResult` of `BadgePage` calls `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` once each.
- A second `normalizeData` of `BadgePage` with `{ id: "1" }` whose response changes `name`: inside it, the root hold calls each of the five once; a third with the same response calls none.
- Two retentions of `BadgePage` with `{ id: "1" }`: `dispose` of one, then `garbageCollectEnvironment`, keeps `Pet:1` in `environment.store` and its derived nodes in `environment.graph.derivedNodes`; `dispose` of both, then `garbageCollectEnvironment`, leaves `environment.store` `toEqual(new Map())` and `environment.graph.derivedNodes` empty.
- `FriendPage` retained, then a `PetDetail` response for `Pet:1` whose `bestFriend` is `Pet:3` (`PetDetail` selects no `nickname` under `bestFriend`): the next `readStoredResult` of `FriendPage` recomputes `FriendLabel`, which reads `PetName` on `Pet:3`, and returns `MissingData` naming `Pet:3`; `garbageCollectEnvironment` keeps `Pet:3`, which `FriendPage`'s AST reaches through the new link.
- A throwing resolver in one retained query: `normalizeData` returns, that query's root holds `ResolverThrew`, and another retained query's stored results are unchanged.
- A derived node stored by a plain `readStoredResult` outside any retention is disposed by `garbageCollectEnvironment`; one under a held root is kept.
- `dispose` of `FriendPage`'s retention, then a change to `Pet:2.name` before `garbageCollectEnvironment`: no resolver is called.
- store.md's GC tests and network-and-subscriptions.md's tests of `garbageCollectEnvironment` are unchanged: the first call `garbageCollect` with a retained list and compare the store with `storeOf`; the second retain through `retainQuery` and compare `environment.store`.

## 4. Level 2 and the height-ordered push

Depends on the held architecture, except `listen`, `snapshot`, and `Observed`.

This change adds observers, and with them needed derived nodes: a derived node is needed when an observer is at its location or a needed derived node reads it. A write recomputes the needed derived nodes whose inputs changed inside `normalizeData`, lowest height first, and marks every other derived node it reaches `Stale`, as change 2 does.

### Heights

A derived node's height comes from what its computation read. `createDerivedNode` sets it; `recompute` raises it when the computation read a derived node at or above it. Its reader is the bucket queue (Settlement), so it lands in this change.

```ts
// from libs/isograph-runtime/src/graph.ts
/** 0 when `dependencies` hold no `Derived` dependency, else one more than the largest `height` among them. */
function heightAbove<TValue>(dependencies: ReadonlyArray<GraphDependency<TValue>>): number {
  return dependencies.reduce(
    (height, dependency) =>
      dependency.kind === 'Derived' ? Math.max(height, dependency.node.height + 1) : height,
    0,
  );
}

/** Sets `node.height` to `height`, moving the node to `graph.queue[height]` when it is in its
 * bucket, then raises each dependent whose height is not above `height` to one more, transitively. */
function raiseHeight<TValue>(
  graph: RecomputationGraph<TValue>,
  node: DerivedNode<TValue>,
  height: number,
): void {
  if (graph.queue[node.height]?.delete(node) === true) {
    bucket(graph, height).add(node);
  }
  node.height = height;
  node.dependents.forEach((dependent) => {
    if (dependent.height <= height) {
      raiseHeight(graph, dependent, height + 1);
    }
  });
}
```

```ts
// from libs/isograph-runtime/src/graph.ts
// createDerivedNode, before (change 2):
    value: computed.value,
    state: 'Clean',
// after:
    value: computed.value,
    state: 'Clean',
    height: heightAbove(computed.dependencies),

// recompute, before (change 2):
  const computed = compute(graph, node.computation);
  setDependencies(graph, node, computed.dependencies);
// after:
  const computed = compute(graph, node.computation);
  setDependencies(graph, node, computed.dependencies);
  const height = heightAbove(computed.dependencies);
  if (height > node.height) {
    raiseHeight(graph, node, height);
  }
```

Every derived node a computation read was brought up to date by `readDerivedNode` before the computation used its value, so `heightAbove` reads final heights. A new derived node has no dependents yet, so `createDerivedNode` raises nothing. `recompute` raises before Propagation's `invalidate` of its dependents, so a dependent is queued at its raised height. A derived node that is computing is not in a bucket (`bringUpToDate` removed it), so `raiseHeight` moves only `Queued` nodes waiting in their buckets.

Raising during `settle` never adds to a drained bucket. The derived node that starts a raise recomputed during the drain of bucket `h`, so it was `Queued` at a height at or above `h` or was `Stale`. A `Queued` dependent of it has a height above its old height, so above `h`, and moves only upward; the dependents of a `Stale` derived node are `Stale` or the node computing, and neither is in a bucket (Ordering).

Cost: `heightAbove` is one comparison per dependency of every computation. `raiseHeight` runs only after a recomputation that read a derived node at or above its own height, and visits the raised nodes and their dependents. A dependent reachable along several paths of the raised region can be raised once per path that raises it further. Nothing lowers a height, so a derived node whose computation later reads lower nodes keeps the bucket it reached.

`raiseHeight` reaching the dependency that started the raise is a cycle of derived nodes. Changes 1 to 4 cannot reach it (Ordering, Cycles); without a check the recursion would not terminate on one. What the runtime does there is open decision 18.

### Needed derived nodes

```ts
// from libs/isograph-runtime/src/graph.ts
export type DerivedNode<TValue> = {
  // .. change 2 and 3's fields, and:
  /** Above the height of every derived node this one reads. Set by `createDerivedNode`, raised by `recompute` (Heights). */
  height: number;
  /** The callbacks `observe` holds at this derived node plus the dependents whose `neededCount` is
   * above 0. The derived node is needed when it is above 0. */
  neededCount: number;
};

/**
 * `Clean`: `value` is what a computation against the store would return.
 * `Queued`: needed, and an input it read changed; it is in `graph.queue[height]`.
 * `Stale`: not needed, and a source it read changed or a derived node it reads is `Stale`. It recomputes when read.
 */
export type DerivedNodeState = 'Clean' | 'Queued' | 'Stale';
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** Adds one to `node.neededCount`; when it was 0, adds one to that of each derived node it reads. */
function addNeed<TValue>(node: DerivedNode<TValue>): void {
  node.neededCount += 1;
  if (node.neededCount === 1) {
    derivedNodeDependencies(node).forEach(addNeed);
  }
}

/** Subtracts one from `node.neededCount`; when it reaches 0, subtracts one from that of each derived node it reads. */
function removeNeed<TValue>(node: DerivedNode<TValue>): void {
  node.neededCount -= 1;
  if (node.neededCount === 0) {
    derivedNodeDependencies(node).forEach(removeNeed);
  }
}

/** The distinct derived nodes of `node`'s `Derived` dependencies. */
function derivedNodeDependencies<TValue>(node: DerivedNode<TValue>): Set<DerivedNode<TValue>>;
```

Writers of `neededCount`:

- `createDerivedNode` starts it, and `rootCount`, at the number of callbacks `graph.observers` holds at its id, before `setDependencies`, so an observed derived node created by a computation needs what it reads.
- `setDependencies`, when `node.neededCount > 0`, calls `addNeed` on each distinct `Derived` dependency of `next` that the old list does not hold, and `removeNeed` on each distinct `Derived` dependency of the old list that `next` does not hold. Distinct, because `dependents` is a set: a dependency recorded twice is one edge.
- `observe` and its stop function (`listen` and `snapshot`, below).
- Disposing a derived node calls `setDependencies(graph, node, [])`, which removes its need from what it read when it was needed.

Invariants, outside a call that is computing a derived node:

- `neededCount` is the callbacks at the derived node's id plus the number of its dependents whose `neededCount` is above 0.
- Every dependent of a `Stale` derived node is `Stale` (change 2). So every derived node a `Clean` node reads is `Clean`, and so is everything below it.
- Every derived node's height is above the height of every node it reads (Heights).
- No needed derived node is `Stale`, and every `Queued` node is needed.
- When `normalizeData` and `garbageCollectEnvironment` return, no derived node is `Queued`.

Why a kept count. A write decides "needed?" for every derived node it invalidates, and the answer must cost `O(1)`. Computing it on demand walks the derived node's dependents upward until an observed node, so each decision costs the node's ancestors, on every write. Marking the needed region from every observer down before each write costs the whole observed region per write, the `O(L)` the push exists to avoid. The count costs work only where neededness changes: `observe` and its stop function walk the region that becomes needed or stops being needed, and `setDependencies` of a needed derived node walks the dependencies it gained or lost. Counting is exact because the graph has no cycle (Ordering). The count is the observed count R10 and Follow-ups 9 ask for. Open decision 16.

### Propagation

```ts
// from libs/isograph-runtime/src/graph.ts
/** An input of `node` changed. A `Clean` needed derived node is queued at its height. A `Clean` derived node
 * nobody needs is marked `Stale` with its dependents. A `Queued` or `Stale` derived node already
 * recomputes before it is read. */
function invalidate<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
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
function bucket<TValue>(graph: RecomputationGraph<TValue>, height: number): Set<DerivedNode<TValue>>;
```

```ts
// from libs/isograph-runtime/src/graph.ts
// markChanged, before (change 2):
  sources.forEach((source) => graph.sourceDependents.get(source)?.forEach(markStale));
// after:
  sources.forEach((source) =>
    graph.sourceDependents.get(source)?.forEach((node) => invalidate(graph, node)),
  );

// deleteSource, before (change 2):
  owned?.forEach((node) => node.dependents.forEach(markStale));
// after:
  owned?.forEach((node) => node.dependents.forEach((dependent) => invalidate(graph, dependent)));
```

`markStale` keeps change 2's body. It reaches only derived nodes nobody needs: a dependent of a node nobody needs is itself not needed, so it never meets a `Queued` node. `disposeDerivedNode` also removes a `Queued` derived node from its bucket.

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

The derived node keeps its state while its computation runs, so a node its read recomputes to a new value does not invalidate it again; it reads the new value and ends `Clean`. On the read path the new `forEach` changes nothing, because a recomputed `Stale` derived node's dependents are `Stale` already; on the push it queues the needed dependents and marks the others `Stale`.

### Settlement

```ts
// from libs/isograph-runtime/src/graph.ts
export type RecomputationGraph<TValue> = {
  // .. change 2's fields, and:
  /** Index `h` holds every `Queued` derived node of height `h`. No index is a hole. */
  readonly queue: Array<Set<DerivedNode<TValue>>>;
  readonly observers: Map<DerivedNodeId, Observed<TValue>>;
};

/** Recomputes every `Queued` derived node, lowest height first, until the queue is empty. */
export function settle<TValue>(graph: RecomputationGraph<TValue>): void {
  for (const nodes of graph.queue) {
    for (const node of nodes) {
      bringUpToDate(graph, node);
    }
  }
}
```

`createGraph` starts `queue` and `observers` empty. A `for..of` over an array reads its length at each step, and one over a set visits members added during the walk and tolerates deleting the current one, so `settle` reaches buckets appended while it runs. By Ordering, a bucket gains no derived node once its drain has started, so each bucket is empty when the walk leaves it. Buckets stay in the array, empty; its length is one more than the largest height ever queued. The loop is a state machine over a queue that grows during the walk, which the iterator combinators do not express.

The code above lets an upward read during `settle` bring the derived node it reads up to date (open decision 19 (a)). Held (c) changes `compute`, `createDerivedNode`, `recompute`, `bringUpToDate`, and `settle` as open decision 19 states, after its two open choices (how the drained height reaches `compute`, and the abandon's type) are made.

`normalizeData` runs `settle` after `normalize` and `markChanged` and before the root hold (change 3). The root hold queues nothing: every derived node it recomputes is `Stale` or new, and the dependents of a `Stale` node are `Stale`. So the queue is empty when `notify` runs. `garbageCollectEnvironment` runs `settle` and then `notify` after `collect`: a needed derived node that read a deleted record was queued by `deleteSource`'s invalidation and recomputes to `MissingData`, and a node disposed while `Queued` left its bucket when it was disposed.

```ts
// from libs/isograph-runtime/src/environment.ts
// garbageCollectEnvironment, after:
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteSource(environment.graph, recordSource(record)),
  );
  collect(environment.graph);
  settle(environment.graph);
  notify(environment.graph);
```

Work per write: the needed derived nodes that read a changed source recompute; the derived nodes nobody needs that read one are marked `Stale` with their dependents, once; each recomputed derived node whose value changed invalidates its direct dependents; and `settle` takes one step per bucket. Nothing is visited whose inputs are unchanged. The traces are in Candidate 3a.

A scheduler (Follow-ups 9) leaves the queue as is; deferral marks instead of queuing.

### `listen` and `snapshot`

```ts
// from libs/isograph-runtime/src/graph.ts
/** The observers of one derived node. */
export type Observed<TValue> = {
  readonly callbacks: Set<() => void>;
  /** The value the callbacks were last called for, or were registered at. */
  notified: TValue | undefined;
};

/** Adds `onChange` at `id`; when the derived node exists, brings it up to date, then adds one to its
 * `rootCount` and its `neededCount`. Returns the function that removes all three. */
export function observe<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  onChange: () => void,
): CleanupFn;

/** The derived node's value as of its last computation, or `undefined` when no node is stored. Computes nothing. */
export function valueOf<TValue>(graph: RecomputationGraph<TValue>, id: DerivedNodeId): TValue | undefined;

/** Calls each observer whose derived node's value is not `===` its `notified`; open decision 9's error rule. */
export function notify<TValue>(graph: RecomputationGraph<TValue>): void;
```

```ts
// from libs/isograph-runtime/src/observe.ts
/** Calls `onChange` after each settlement that changed the result stored at `location`, and
 * makes that result needed. Returns the function that stops it. */
export function listen(
  environment: IsographEnvironment,
  location: DerivedNodeLocation,
  onChange: () => void,
): CleanupFn {
  return observe(environment.graph, resolverDerivedNode(location.record, location.storeKey), onChange);
}

/** The result stored at `location` as of its last computation, or `undefined` when none is
 * stored. Computes nothing. */
export function snapshot(
  environment: IsographEnvironment,
  location: DerivedNodeLocation,
): ResolverResult<unknown> | undefined {
  return valueOf(environment.graph, resolverDerivedNode(location.record, location.storeKey));
}
```

`observe`:

1. Adds `onChange` to the `Observed` at `id` in `graph.observers`, creating it.
2. When a derived node is stored at `id`: `bringUpToDate` on it, then adds one to `rootCount` and calls `addNeed`. Bringing it up to date first leaves it `Clean`, and so every derived node below it (Needed derived nodes, the invariants), so the region that becomes needed holds no `Stale` node. The derived nodes it recomputes were `Stale`, and their dependents are `Stale`, so nothing is queued and no other observer's value changes.
3. Sets `notified` to the derived node's value.

The returned function removes the callback, subtracts one from `rootCount` and calls `removeNeed` on the derived node stored at `id` when there is one, and removes the `Observed` when it has no callback left. When no derived node is stored at `id`, `observe` only registers, and `createDerivedNode` starts the node's `rootCount` and `neededCount` at the callbacks there.

A location that is `Stale` when a component first renders: `snapshot` returns the last computed result (R1: it computes nothing). `listen`, in React's subscribe, brings it up to date, and `useSyncExternalStore` compares the snapshot after subscribing and renders again with the current result. Open decision 3.

Notification, after settlement in `normalizeData` and `garbageCollectEnvironment`: for each `Observed` in `graph.observers` in insertion order, the derived node's value is compared by identity with `notified`; when different, `notified` is set and each callback is called once. Identity suffices because `recompute` keeps the value object when `valuesEqual` holds. An observed resolver that returns a new object on every recomputation notifies on every recomputation (R4); a stable identity for it would come from upstream's merge of the data it reads, not from comparing its output. Callbacks that throw do not stop the loop; after it, one error is rethrown as is and several as an `AggregateError` (open decision 9). The loop is `O(observers)`; a list of the observed derived nodes `recompute` changed would make it `O(changed)`, and is not needed while observers are the mounted components of one screen.

What each level owns:

- Level 1 holds values, dependencies, dependents, states, root holds, needed counts, and the queue. It runs user code (resolvers).
- Level 2 holds the observed ids, callbacks, and the last-notified identity. It runs no resolver, and is notified only after level 1 settled, once per changed derived node per settlement.

A parent component never re-renders with a stale child: every needed derived node is `Clean` before the first callback, and every node a mounted component observes is needed, so any `snapshot` during a re-render a callback caused returns the settled value, whatever the order callbacks run in. React additionally batches the `setState`s that `useSyncExternalStore` calls in one notification loop into one render.

What React needs, and gets without `@isograph/runtime` knowing React: `useSyncExternalStore(subscribe, getSnapshot)` where `subscribe(onStoreChange)` returns an unsubscribe function (`listen`), and `getSnapshot` returns the same value while nothing changed and is pure and cheap (`snapshot`, a lookup). How a component learns its location (the derived node a parent's result names) is the React package's, with `@component` (Later).

An observer keeps its derived node current (it makes the node needed, so a write that reaches it recomputes it) and keeps the node through `collect` (its `rootCount`), but retains no record: data stays only while a retention keeps it. A location whose record `garbageCollectEnvironment` deleted has no stored derived node, and `snapshot` returns `undefined`.

### Tests

`test/graph.test.ts` gains, with literal ids: a derived node that reads no node has height 0, and one that reads nodes of heights 0 and 2 has height 3. Two needed derived nodes that both read source `a`, the second also reading the first (heights 0 and 1): `markChanged([a])` then `settle` runs the height-0 computation before the height-1 one, each once. A derived node nobody needs that read `a` is `Stale` after `markChanged([a])` and is not recomputed by `settle`. `observe` of an absent id registers only; the derived node created later starts with `rootCount` and `neededCount` 1. Adjustment: a needed derived node of height 0 whose recomputation after `markChanged` reads a node of height 1 has height 2, and its dependent of height 1 has height 3; a `Queued` dependent it raises is recomputed after the derived nodes below its new height. A derived node of height 2 whose recomputation reads no node keeps height 2.

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
- A link set from `null` under an observer at `FriendPage` with `{ id: "4" }`, `Pet:4.bestFriend` `null`: a write that sets it to `Pet:2` calls `FriendLabel` on `Pet:4`, `PetName` on `Pet:2`, `FriendCard`, and `FriendPage` once each. Then `Pet:2.name` changes: `PetName` on `Pet:2`, `FriendLabel` on `Pet:4`, `FriendCard`, and `FriendPage` are called once each, in that order.
- Link move under an observer at `FriendPage`: `Pet:1.bestFriend` moves to `Pet:3` (with `name` and `nickname`): inside that `normalizeData`, `FriendLabel` is called once and `PetName` on `Pet:3` once. Then `Pet:2.name` changes: no resolver is called. Then `Pet:3.name` changes: `PetName` on `Pet:3`, `FriendLabel`, `FriendCard`, and `FriendPage` are called once each.
- An observer at `FriendPage` whose retention was disposed: `garbageCollectEnvironment` deletes the records, calls the observer once inside it, `snapshot` returns `undefined`, and `environment.store` is `toEqual(new Map())`.
- A callback that calls `normalizeData` with another change: the nested call notifies; each observer receives each result once.
- A callback that throws: the other observers are still called, then `normalizeData` throws it; two that throw give an `AggregateError` of both.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: "`pico-runtime.md` stays parked" is replaced by this doc as the step after v1. `Store` is unchanged, and its functions keep their parameters and its tests hold. `normalize` writes through `writeValue`, skipping a value unchanged by its AST shape, returns `ReadonlyArray<StoreChange>`, and throws `PartialWriteError` (change 1); the package index stops exporting `normalize`, which `normalizeData` replaces there (change 2b, open decision 17); `garbageCollect` returns the links of the records it deleted (change 2).
- `refactors/pending/runtime/network-and-subscriptions.md`: `IsographEnvironment` gains `graph`, which `createIsographEnvironment` creates (change 2). `environment.ts` gains `normalizeData` (change 2), which takes the entrypoint and returns a `Retention` that `environment.retainedQueries` also holds (change 3), and settles and notifies (change 4); `garbageCollectEnvironment` deletes the derived nodes of deleted records (change 2), collects unreached nodes (change 3), and settles and notifies (change 4). `makeNetworkRequest` calls `normalizeData` where it called `normalize` (change 2), and from change 3 holds the returned `Retention` and disposes it in its cleanup. Its Why the store-only functions stay store-only holds: the functions that need the graph take the environment. Its change 7 keeps request entries in `environment.requests`, beside the store, and `Store` stays store.md's nested map; this doc adds no field to `Store` either.
- `refactors/pending/graphql-protocol/normalization-ast.md`: `MergedSelectionTarget::Leaf`, `CompletedSelectionTarget::Leaf`, and `NormalizationTarget::Leaf` carry a `ListDepth`; the TypeScript `NormalizationSelection` becomes `NormalizationLeafSelection | NormalizationCompositeSelection`, and a leaf selection prints `listDepth` after `target` (change 1).
- `refactors/pending/runtime/reader-ast.md`: `ReaderArtifact.selectableName` and its printing; the walk takes `ReadHooks<TFailure>` and `readButDoNotEvaluate` passes v1's hooks, so `read` and `readButDoNotEvaluate` keep their behavior and its tests hold unchanged; `ResolverResult` adds `ReadResolverThrew` to `ReadResult` for stored results, and `ReadResult` is unchanged (change 2). Its note that this doc's `computeResolver` and `readSelection` replace its `read` is superseded: its `read` stays, and `readStoredResult` is the stored read beside it (change 2).
- `refactors/pending/runtime/normalization-ast-mount-markers.md`: candidate 6 is answered by Resolver markers (held: no marker); its open question on how this doc models a mount is answered by `Retention` and settlement.

## Other derived node kinds

`check` stays the plain function from store.md; caching it as a derived node of the graph was considered and rejected by the user (no need).

The engine is general over derived node kinds only as far as the follow-ups that add nodes need: a caller supplies each node's id, owner, and computation, and the engine never inspects a value except through `valuesEqual`. Resolver derived nodes are the only kind changes 1 to 4 create. The parts of the API that serve the follow-ups:

- `graph.ts` imports nothing from the store, the reader, or the environment, and names no resolver, record, or artifact. This is open decision 12's held design, which changes 2 to 4 need on their own.
- A derived node's owner is a caller-supplied `SourceId` (`DerivedNodeDefinition.owner`). Closure-component derived nodes (components-in-userland.md) need it: the wrapper is a node at the closure field's resolver store key, owned by the record's `recordSource`, whose computation reads only the record's presence (The engine, the store, and the environment, Lifetime still follows records), and a second node per closure derived node for its data read is that doc's open question 3. Both are resolver-shaped derived nodes, so they need nothing else beyond change 2's resolver nodes.
- A derived node's computation and id come from its creator (`DerivedNodeDefinition`, `track`, `readDerivedNode`, `retainRoot`); no engine function builds one. Follow-up 10 needs it: a runtime-defined computation is not a resolver and has no resolver store key.
- The graph's value type is a parameter, `TValue`. Follow-up 10 needs it: a derived value or a selector's value is not a `ResolverResult`, so that follow-up widens the environment's graph to a union of the kinds' values, and `valuesEqual` to a function that compares two values of one kind and answers unequal for two kinds.
- Heights are the engine's: `createDerivedNode` and `recompute` set a derived node's height from what its computation read (change 4, Heights), so a caller supplies none. Follow-up 10 needs it: a runtime-defined computation has no artifact that could carry a height.
- Sources are opaque `SourceId`s, and the engine never inspects one. A source other than a store field, first the state of a network request, needs only a `SourceId` constructor beside `recordSource` and `valueSource`, a reader AST node and walk hook that report it, and its writers' `markChanged` calls (The engine (2a), after `ComputeContext`). User code reaches such a source only through a selection, never through a `ComputeContext`.
- Browser-owned state, such as a `localStorage` key or a cookie, is a possible later source kind. It is not planned.
  - The key is one source per (storage, key name). A coarser source, one per storage, marks every reader of that storage on any change to any key, and each of those readers recomputes.
  - The environment holds a mirror of each value a walk has read. The walk reads the mirror and calls `readSource`, and events update the mirror and call `markChanged`. This is the shape of the compiler mirroring disk files and editor buffers as events (AGENTS.md, Shared state and interior mutability, last paragraph). On a server, the environment fills the mirror from the request's `Cookie` header.
  - The selection names the key, so what the reader reads fixes the source, and open decision 20's problem does not arise.
  - A source is current only if every write to it reports a change. A write through an isograph function, one that writes the value and calls `markChanged`, reports it. The browser reports some writes and not others:
    - The `storage` event "is not fired on the window that made the change"; for `localStorage` it fires "in all other browsing contexts that are in the same origin as the initiating document", including other tabs (MDN, Window: storage event, `https://developer.mozilla.org/en-US/docs/Web/API/Window/storage_event`). A `localStorage.setItem` in the same document outside isograph reports nothing.
    - An assignment to `document.cookie` fires no event of its own, and a `Set-Cookie` response header changes a cookie with no event of its own. The Cookie Store API fires a `change` event at a `CookieStore` "when a change is made to any cookie" (MDN, CookieStore: change event, `https://developer.mozilla.org/en-US/docs/Web/API/CookieStore/change_event`). MDN marks it Baseline 2025, newly available, secure contexts only; MDN's browser-compat-data lists Chrome 87, Edge 87, Firefox 140, and Safari 18.4. The spec (`https://cookiestore.spec.whatwg.org/`, Processing model) defines the event over every script-visible change to the cookie store and names no source, so a `document.cookie` assignment or a `Set-Cookie` header that changes a cookie visible to script would reach that listener; this reading is inferred from the spec and not verified in a browser. An `HttpOnly` cookie is not visible to script and reaches no listener. Where the API is absent, a `document.cookie` assignment or a `Set-Cookie` header outside isograph reports nothing unless the code that made it reports it.
  - Polling is not an option (AGENTS.md, Wake on events). Such a value is therefore current when it was written through isograph or when the browser reported the write, and is stale otherwise until a write that isograph sees.
- Client roots reached through `with` blocks are a possible later direction for reaching such sources. They are not planned, and their syntax is not decided. A sketch:

  ```text
  field User.fullName
  with User { friendName }
  with Root1 { url }
  ```

  - Every selection set of a field is a `with E { ... }` block. A block is allowed when `E`'s record is determined by the derived node's key (field, record, variables) with no added component. That holds in two cases. `E` is the parent entity, and the block selects from the field's own record. Or `E` is a singleton, an entity with one record, which a read finds with no id and no variables. i2 already names a singleton: a root entity's identity is `Fixed` (`CompletedIdentity::Fixed(FixedRecordId)`, `crates/isograph_compiler/src/merged_selection.rs:1450-1458`; the normalization AST's `{ kind: "Fixed"; id }`, refactors/pending/graphql-protocol/normalization-ast.md, The identity), whose one record is at `FixedRecordId::root()`.
  - The parent block replaces the field's main selection set: `field User.fullName { first last }` is `field User.fullName` with `with User { first last }`. The two forms are one form.
  - A read from a singleton adds only `SourceDependency`s to the reading derived node, so the key stays (field, record, variables).
  - Roots are reached through blocks because a link from `User` to a root would be a selectable, such as `root`, on every entity. An entity's selectables are one map, `NamedEntity.selectables: BTreeMap<SelectableName, Selectable>` (docs-website/docs/design-docs/mental-model.md, Entity), which holds the upstream selectables and the project's selectables together (mental-model.md, line 7). Upstream reports a server selectable and a client selectable of one name on one entity as an error (`multiple_selectable_definitions_found_diagnostic`, `../isograph/crates/isograph_schema/src/memoized/selectable_access.rs:59-63`, defined at `client_selectable_access.rs:220-231`). A `root` selectable would therefore collide with any server field named `root`, and it would add a selectable to every entity. A `with` block adds nothing to `User`.
  - The resolver receives one part per block, keyed by entity: `{ User, Root1 }` for the sketch. A selection in the `User` block and a selection in the `Root1` block with the same name therefore do not collide. Two blocks that name one entity are an error. The exact shape of the resolver's parameter is not decided.
  - Client roots such as `Root1` are singletons that isograph or the project declares, and the network protocol's schema does not contain them. A client root's type (its selectables) is static, because the compiler generates the resolver's parameter type from it. A client root's values come from the environment: `createIsographEnvironment` takes a provider per declared client root, and the provider gives each selectable a read and a subscription that calls `markChanged`. Overriding a root's values is a different environment, such as one in a test or one on a server that fills the values from request headers. A fixed root name, including a default `Root`, fixes only the type.
  - A client root's selectables are environment sources: `url`, a `localStorage` key, a cookie, and possibly request state (open decision 20).
  - A missing provider must not read as `MissingData`, because `MissingData` means the data must be fetched, and nothing fetches a client root. Held: `createIsographEnvironment`'s type requires a provider for every declared client root, so a missing provider is a compile error. A value that a provider lacks is a nullable selectable whose read returns `null`.
  - A client root has no effect on query generation. Nothing in a client-root block is fetched, so the network protocol never receives it, and client roots are protocol-neutral. GraphQL stays out of the core.
  - A server singleton (`Query`, identity `Fixed`) passes the lookup rule, but fetching it is a separate feature with costs client roots do not have:
    - Query generation would place those selections at the operation root of every query that reaches the field.
    - A mutation or subscription operation cannot select `Query` fields unless the schema exposes them on its mutation or subscription root.
    - A `node(id:)` refetch of the field would have to carry those selections.
    - The value would depend on which query fetched it, which is the sharing problem of open decision 20: one derived node per (field, record, variables) has no single value for it.
  - Until that feature has its own design, `E` is the parent entity or a client root.
  - Upstream isograph has no client root and no client-only source: `../isograph/crates` and `../isograph/libs/*/src` contain no live field, no external-store read, and no read of `localStorage`, `document.cookie`, or `window.location`.
  - Relay's live resolvers (`@live`) are the closest precedent. A live resolver returns a `LiveState<T>` with `read(): T` and `subscribe(cb: () => void): () => void`; "the state provider should call the callback when the value of the live state changes", and Relay then calls `read()` again (Relay docs, Live Fields, `https://relay.dev/docs/guides/relay-resolvers/live-fields/`). That `subscribe` is the change notification a source needs. A client-root selectable backed by an external value needs a read and a subscription that calls `markChanged`, which is the condition the browser-owned-state bullet states: a source is current only if every write to it reports a change.
- A derived node kind the runtime defines for its own use is runtime code over `ComputeContext`. The first is the suspension decision of react.md: a derived node that reads a root resolver derived node with `readDerivedNode` and the entrypoint request's state with `readSource(requestSource(key))`, and yields `Ok`, `Suspend`, `Throw`, or `ReadStore` according to the `whileInFlight: WhileInFlight` and `onNetworkError: OnNetworkError` options (react.md, `NetworkRequestReaderOptions`). Its value is not a `ResolverResult`, so it is one of the kinds the widened value type of Follow-ups 10 covers.

## Later

- `@component`: a component is a resolver result, stored and settled like any other; a mounted component observes its own derived node.
- Client pointers, `@loadable`, refetch queries (reader-ast.md, Later): each adds derived nodes or retentions, and the markers of normalization-ast-mount-markers.md.
- Optimistic writes: a layer of server entries that settlement reads through. What the layered store must report to the recomputation engine is The engine, the store, and the environment, What the environment must report.
- Eager client pointers: a pointer node that reads client fields on its target lets a derived node of a field read a derived node of the same field on another record. Height adjustment orders it (change 4, Heights; open decision 14); a cycle of derived nodes through the pointer is open decision 18.

## Follow-ups

The user's design notes, recorded for later. None of them is part of Step A or changes 1 to 4, and none is approved; 10 is the user's planned addition, and 11 is held. Upstream paths here follow the doc's convention; `isograph-react-disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`, and `isograph-disposable-types/` paths are under `isograph/libs/isograph-disposable-types/src`.

### 1. Constructors and destructors at normalization time

A normalization-time subscription is a constructor that `normalize` runs when a retained query first reaches a resolver (the first computation of its derived node), paired with a destructor that runs when the last handle referencing that derived node is released. Lifetime follows retain and release (`normalize` and `dispose`), not render.

Upstream ties the lifetime of such things to render and effects: `useDisposableState` creates the item during render and disposes it on unmount (`isograph-react-disposable-state/useDisposableState.ts:16`), and the pagination hooks hold one disposable per page through `useUpdatableDisposableState` (`loadable-hooks/useSkipLimitPagination.ts:96-99`, `loadable-hooks/useConnectionSpecPagination.ts`).

A destructor needs to know when the last reference goes away. Retention (change 3) keeps reference counts implicit (a derived node is live while any retention or observed location reaches it). An explicit count per derived node is the number of retentions and observers at its location plus the size of its `dependents` (change 2), so constructors and destructors need that count kept current on every change of those three: open decision 11 (b).

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

Two retained queries that reach the same pointer at the same record share one handle and one follow-up query. The key is the record, the selectable, and the variables: the resolver store key on that record (`resolverDerivedNode(record, resolverStoreKey(selectableName, variables))`, the key of `graph.derivedNodes` and `graph.observers`). The handle's reference count is the number of retained parents that reach it.

### 6. A destructor and an in-flight follow-up

Upstream cannot cancel a request: `IsographNetworkFunction` is `(operation, variables) => Promise<any>` and takes no abort signal (`IsographEnvironment.ts:103-106`); dispose only stops the response from being normalized (`makeNetworkRequest.ts:235`). A network function that accepts an `AbortSignal` lets the destructor of a follow-up cancel its request.

### 7. Deferral

The user's direction: do not do all the work at construct and destruct time.

- Construction: `normalizeData` computes the result when the constructor runs, and defers setting up subscriptions to a scheduler. The runtime stays free of browser globals because the caller supplies the scheduler: `createIsographEnvironment` takes a function that receives a task and runs it later. The React package passes one that calls `requestIdleCallback` (or an equivalent where that global is absent); tests pass one that queues tasks and runs them when the test says.
- Destruction: garbage collecting on every release is wasteful. A release adds the handle to a buffer, and collection runs later as one batch, through the same scheduler or when the buffer exceeds its size. A handle that is retained again before the batch runs is not destroyed. Upstream's release buffer is the related mechanism: `unretainQuery` pushes the released query onto `gcBuffer` and reports overflow past `gcBufferSize` (`garbageCollection.ts:40-53`), whose default is 10 (`IsographEnvironment.ts:177`, `:205-206`), and overflow triggers collection (`makeNetworkRequest.ts:565-577`). v1 removed the buffer as not needed yet (`refactors/pending/runtime/store.md`, `garbageCollect`).

### 8. Pre-memoized selection sets

To explore. This may not work out, and the user expects it probably does not matter.

A reader's selection set read against a record (`{ firstName lastName }` on `Pet:4`) is itself a stored result, keyed by the record and the selection set. The selection set's part of the key is a stable key the compiler emits, such as a hash of the selection set or an interned id. Two resolvers whose reader ASTs contain the same selection set on the same record read it once. A resolver whose inputs are one stored selection-set result records one `DerivedNodeDependency` on that derived node instead of one per field.

Savings:

- Repeated field reads: the second resolver that reads `{ firstName lastName }` on `Pet:4` reads the stored result instead of each field.
- Finer-grained dependencies collapse into one: a resolver derived node holds one dependency on the selection-set derived node instead of one per field, and one dependents edge per reader instead of one per field.

Costs:

- More stored derived nodes: one per selection set per record read, alongside the resolver derived nodes.
- A key per selection set: the compiler emits it into the reader artifact, and two selection sets must get equal keys exactly when they read the same fields with the same arguments.
- Equality of selection-set results: the stored result is a fresh object on each computation, so it needs upstream's `mergeObjectsUsingReaderAst` (`areEqualWithDeepComparison.ts:55-121`) applied to the read result, keeping the old object where the new data is equal, or every recomputation is unequal and invalidates every resolver that depends on it. The merge is well-defined here because a selection set's result is store data shaped by the reader AST; a nested resolver value in it is compared by `===`, as upstream does (`:84-95`).

Relation to changes 2 and 3: the stored selection-set result is an extra level between server values and resolver derived nodes. A resolver derived node depends on a selection-set derived node, the selection-set derived node depends on server values and on the resolver derived nodes of the `Resolver` nodes it contains, and marking and bringing up to date walk that level as they walk a resolver derived node. Under held 3a the level needs no rule of its own for heights: a selection-set derived node's height comes from the resolver derived nodes it reads, and a resolver derived node's from its selection-set derived node (change 4, Heights).

### 9. Scheduled recalculation

Partly future work; R10 requires that the architecture allows it. The user wants control over when recalculation runs:

- Unobserved derived nodes are lazy. A derived node no observer needs (an unmounted component's result, a retained query nobody renders) is only marked on a write and is recomputed when next read. Changes 2 to 4 build this: it is the `Stale` state and the needed count.
- Deferred work. Some recalculation and re-rendering runs later at lower priority, for example the work caused by a write that came from user input. In the React layer this corresponds to `startTransition` and `useDeferredValue`. `@isograph/runtime` stays React-free and takes the caller-supplied scheduler of note 7; the scheduler decides when each pending pass runs.

What deferral changes in the requirements: R2 becomes "every needed result is current when the pass for its priority finishes; a result nobody needs is current when read". R5 becomes "observers are called once per pass", not once per write.

What deferral needs on top of changes 1 to 4:

- A priority on a write (`normalizeData` called inside a transition) or on an observer (a component rendered with a deferred value).
- A deferred write invalidates as if no derived node were needed: `markStale` from each derived node that read a changed source, so every derived node the write reaches becomes `Stale`, and the observed derived nodes the marking reaches are recorded under the write's priority. This lifts "no needed derived node is `Stale`" until the deferred pass runs.
- An urgent write that reaches a needed `Stale` derived node queues it, where `invalidate` would otherwise skip a non-`Clean` derived node. Its recomputation reads the deferred changes below it too, so the urgent pass shows them early for that derived node, as React does when a transition and an urgent update touch the same state.
- A deferred pass brings each recorded observed derived node up to date by the read path and notifies its observers. A derived node an urgent pass already brought up to date is `Clean`, and the deferred pass reads it as is. A second deferred write before the pass marks on top of the existing marks, so the pass recomputes each derived node once for both writes. The read path has no cutoff (open decision 15), so a deferred pass recomputes every `Stale` derived node it reads.

Fit with each candidate:

- Held 3a: laziness is built. Deferral is the deferred write's `Stale` marking and the recorded observed derived nodes, plus `invalidate` queuing a needed `Stale` derived node; no state is added.
- 3b: marking is already separate from bringing up to date. Laziness is "mark only, and skip `bringUpToDate`, for derived nodes whose observed count is zero". Deferral is "the scheduler chooses which derived nodes of the marked roots each pass brings up to date".
- 2: laziness is native, because nothing is verified until demanded, and deferral is the scheduler choosing when to verify which roots. Each deferred pass still walks every live edge under its roots.
- Plain 3a and 3c: neither has a lazy mode. Skipping an unobserved derived node leaves it stale with no record that it is stale, so they need held 3a's `Stale` mark.
- 4: Preact already keeps unsubscribed computeds lazy and defers effects to the end of a batch; priority would be added to its effect queue.
- 5: both work per root, at full cost per pass and with no cutoff below the root.

React constraint: `useSyncExternalStore` performs a store update as blocking even inside a transition, per React's documentation. Deferred re-rendering therefore needs the React package to hold a deferred snapshot per observer and update it inside `startTransition`, or to read through `useDeferredValue`; this is the React package's design, not the runtime's.

### 10. Runtime-defined computations

A memoized computation the runtime or a user defines at run time, which reads other derived nodes: a derived value over several resolver results, a selector a hook creates. Its height comes from what its computation reads, as every derived node's does (change 4, Heights), so the bucket queue places it. What it needs from the engine is a caller-supplied id and computation and a wider value type (Other derived node kinds).

### 11. The fetch decision from the root derived node

Held, not approved, and not part of changes 1 to 4; it needs changes 2 and 3. With `shouldFetch` `IfNecessary` (network-and-subscriptions.md, `fetchEntrypoint`), `fetchEntrypoint` decides from the root derived node of the entrypoint on its variables instead of from `check` over the normalization AST. `Ok` (or `ResolverThrew`) means the data is present; `MissingData` means fetch.

```ts
// from libs/isograph-runtime/src/network.ts
// fetchEntrypoint, before (network-and-subscriptions.md):
    case "IfNecessary":
      return check(environment.store, entrypoint.normalizationAst, variables).kind === "EnoughData"
        ? retainQueryWithoutMakingNetworkRequest(environment, entrypoint.normalizationAst, variables)
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
// after:
    case "IfNecessary":
      return readStoredResult(
        environment,
        entrypoint.readerArtifact(),
        entrypoint.normalizationAst.root,
        variables,
      ).kind === "MissingData"
        ? makeNetworkRequest(environment, entrypoint, variables, fetchOptions)
        : retainQueryWithoutMakingNetworkRequest(environment, entrypoint.normalizationAst, variables);
```

- A repeated call with no write between is a lookup: the root derived node is `Clean`, and `track` returns its stored result. After a write that reached it, the call recomputes it (`Stale`), or `settle` already did (needed). `check` walks the whole normalization AST on every call.
- A root derived node this call creates is outside any retention until `normalizeData`'s root hold or a `listen` reaches it; `collect` (change 3) disposes it otherwise.

Why a complete read implies that the normalization AST's data is present:

- The normalization AST is the merge of the reader ASTs the entrypoint reaches, with the same variables (query-generation.md, Merging), plus the selections `complete` adds: the discriminator and the identity (`crates/isograph_compiler/src/merged_selection.rs:1555-1560`). The discriminator has no normalization node (`normalization_node`, `crates/isograph_compiler/src/normalization_ast.rs:135-140`), so `normalize` does not store it and `check` does not look it up. An `Ok` root means every reader the entrypoint reaches read every selection it selects, along current links and refinements, which are the positions `check` visits.
- The identity is an ordinary `Selection` in the normalization AST: `CompletedSelection::Identity` becomes a `NormalizationNode::Selection` with a leaf target (`normalization_ast.rs:141-150`). `complete` adds it only for an entity whose identity is `Select`; an entity whose identity is `Fixed` or `Omit` has no `id` selection (`merged_selection.rs:1555-1558`). A reader need not select `id`, but `normalize` writes it in the pass that creates the record: `compositeValue` calls `recordAt`, which inserts the record, and then `writeSelections` over the target's selections, which hold the identity (`libs/isograph-runtime/src/normalize.ts:151-158`; `recordAt` at `:163-169`, `writeSelections` at `:52-79`; v1 line numbers, before change 1). So a present record of a `Select` entity holds its `id`.
- The exception is a `normalize` that threw part-way (open decision 17). Its writes stay, so a record can exist without its `id` while every selection a reader reads is present. The root is then `Ok` where `check` reports `MissingData`, and no fetch is made. The data the readers read is present.
- `@loadable` selections are not an exception. i2 accepts `@loadable` with no effect beyond validation: the query selects the field's selections and the parent's reader reads the field exactly as without the directive (graphql-protocol/node-entrypoints-and-loadable.md, the opening section and its example). No other normalization AST selection in i2 today is outside every reader: `complete` adds only the two selections above.
- `ResolverThrew` is not equivalent to `check`. The walk stops at a nested `ResolverThrew` (`allOk` stops at the first result that is not `Ok`), so the selections after the throwing resolver in each enclosing selection set are not read, and data missing there does not show in the root's result. Treating `ResolverThrew` as present skips a fetch that `check` would make, and that fetch could change the throwing resolver's inputs. Treating it as a reason to fetch sends a request on every call while the resolver throws. The user's statement holds it as present; this difference is not settled.

Components (later): a component field's data is read at render (components-in-userland.md, A field whose result is a closure), so the root derived node does not cover it unless each component field has a data derived node computed with the root (components-in-userland.md, Under changes 2 to 4, and its open question 3). Until then, an entrypoint that reaches a component field needs `check`.

`check` stays a plain function exported from the package index (`libs/isograph-runtime/src/index.ts:43`); `test/check.test.ts` calls it.

## Open decisions

### 1. Resolver markers in the normalization AST

- (a) Root only: `normalizeData` takes the entrypoint, and settlement creates the rest through reader ASTs (held, Resolver markers).
- (b) A marker per selection set, as the user's design describes. Cost: artifacts import reader artifacts and source modules, the marker set goes stale as links change, and it duplicates what reader ASTs already say.

### 2. Recomputation architecture

The candidates and their comparison are Recomputation architecture.

- (a) 3a with heights per derived node and `Stale` marks (held, the user's direction). A write queues each needed derived node that read a changed source, drains the queue lowest height first, and queues a derived node's direct dependents only when its result changed; a derived node nobody needs is marked `Stale` with its dependents and recomputed when read. Cost: the inverse of every dependency list, kept by `setDependencies`; a state, a needed count, and a height per derived node; height adjustment after a recomputation reads a derived node at or above its own height; no cutoff on a read of a `Stale` derived node (open decision 15); an abandoned partial walk and a re-run in a higher bucket after an upward read during `settle` (open decision 19, held (c)).
- (b) 3b, mark along reverse edges, then bring up to date (the considered alternative). Cost: the inverse, a state per derived node, and every transitive dependent of a changed source marked `Check` on every write and walked again by settlement. Benefit: no height, and cutoff on reads of derived nodes nobody needs.
- (c) 2, pico's pull with epochs. Cost: every live edge under the observed roots compared on every write, and a tag and epoch on every server value; benefit: no inverse to keep.
- (d) 5, full re-run per write. Cost: every live resolver on every write, and no equality cutoff below the roots.

Not approved for implementation.

### 3. Eager computation for retained queries nobody observes

- (a) Every retained root is brought up to date after every write. Cost: every retained query's affected resolvers run on every write, rendered or not (a prefetched route, a screen kept retained after navigation); under held 3a, `neededCount` would count retentions as well as observers.
- (b) A write recomputes only needed derived nodes; a retained root nobody observes is `Stale` after a write that reaches it and is brought up to date when `listen` attaches, in the subscribe call, not in render (held, the user's choice). Cost: the first render of a newly observed `Stale` location reads the last computed result, and `useSyncExternalStore` renders again after `listen` brings it up to date.
- (c) Per retention, a choice between (a) and (b). Cost: an option on `normalizeData`.

### 4. Where derived nodes live

- (a) In `environment.graph`, beside the store on the environment; records keep store.md's untagged values (held, with open decision 12). Cost: `garbageCollectEnvironment` deletes records from the store and then their derived nodes from the graph, one `deleteSource` per record.
- (b) On the record, under the resolver store key, with every value tagged `Server` or `Resolver`. Cost: a tag object per server value, a narrowing in every server-value reader, and two broken-invariant throw sites (a derived node under a server store key and the reverse). Under candidate 2 the tag also carries the epoch, and this option is the one that candidate needs.

### 5. A resolver that throws

- (a) Stored as `ResolverThrew`, propagated to parents like `MissingData` (held). Reason: `normalizeData` runs resolvers (the new root's, and every needed derived node's).
- (b) Propagate the exception, v1's behavior. Cost: one broken resolver aborts `normalizeData` for every query and leaves level 1 unsettled.

### 6. Garbage collection from live resolver dependencies

The user's idea, explicitly unsettled: records and fields linked to the derived nodes that read them, so collection knows what is live without walking ASTs.

- (a) Walk retained normalization ASTs, as v1 (held).
- (b) A record is live when a live derived node depends on it. `graph.sourceDependents` (change 2), with `recordSource` ids, is that index already; no new edges are needed. Cost: a record a retained query fetched but no live derived node read is deleted (fields after a `MissingData`, records a reader stops before), so `check` of that retained query reports `MissingData` and a refetch follows; retention no longer means what store.md and upstream mean (`garbageCollection.ts:88-119`).

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

### 11. When a derived node no root reaches is deleted

- (a) By `garbageCollect`'s sweep (held, change 3). Under held 3a such a derived node is marked `Stale` by the first write that reaches it, and later writes stop at it, so it costs no recomputation until then. Matches v1's caller-driven collection and Follow-ups 7's deferral.
- (b) By reference count: a derived node's count is its `rootCount` plus `dependents.size`; `dispose`, the stop function of `listen`, and `setDependencies` decrement it, and at zero the derived node is disposed and `setDependencies(graph, node, [])` decrements its dependencies in turn. Cost: a count kept on three paths, and deletion in the middle of a write. It is what constructors and destructors need (Follow-ups 1).

### 12. Coupling of the recomputation engine to the store

The analysis is The engine, the store, and the environment.

Settled: (a).

- (a) A store-agnostic engine over opaque `SourceId`s and `DerivedNodeId`s, held on the environment beside the store. `Store` stays store.md's nested map; `normalize` returns its changes and `garbageCollect` its deleted records; the walk reports reads through `ReadHooks`; `normalizeData`, `garbageCollectEnvironment`, and `readStoredResult` forward both to the graph. Derived nodes are owned by their record's presence source, so they are freed with the record. Keeps: v1's store code and tests, tests that compare the store directly, and upstream's environment shape. Cost: two id strings per server read and one per resolver read, a presence edge per (derived node, record read), the `owned` map, and every function that needs both the store and the graph takes the environment.
- (b) Derived nodes and dependents keyed by record and store key, in fields of a `Store` that becomes an object. Not taken: the engine names records and store keys; a layered store must translate its changes into the base layer's shape, which has no case for removing a layer; and every v1 function and test reads `store.records` where it read `store`.

### 13. Values with lifecycles

The analysis is Values with lifecycles.

- (a) The engine owns each derived node's value lifecycle (held): `Disposable` computations return an `ItemCleanupPair`; a new value is constructed before the old is released; releases are deferred until after notification and flushed in order; a `===` value keeps the old holding and releases the new retain at once. A constructor that throws stores `ResolverThrew` and releases the old value (held); a destructor that throws does not stop the flush, and the errors are rethrown together (held).
- (b) Lifecycles outside the engine: a disposable value lives in a registry the resolver's code calls, and the engine stores only the handle. Cost: nothing releases a handle when the derived node's value is replaced or the node is disposed, so the registry needs its own reachability, which is the engine's again.

Not decided: the user's choice.

### 14. Heights and eager client pointers

Settled: heights are per derived node and adjusted at run time (change 4, Heights). An eager pointer (reader-ast.md, Client pointers, Later) reads client fields on its target: `field Pet.Card { bestFriend { Card } }`, with `bestFriend` a pointer, is not an `InlineCycle`, and the derived node of `Card` on `Pet:1` reads the derived node of `Card` on `Pet:2`. That read is one more data-dependent dependency, and adjustment raises the derived node of `Card` on `Pet:1` above it. A cycle of derived nodes through the pointer is open decision 18.

### 15. Cutoff on a read of a `Stale` derived node

- (a) A `Stale` derived node recomputes when read, its reads bringing the `Stale` derived nodes below it up to date first (held, the user's specification). Cost: no cutoff on the read path. After `Rex` to `Ray` on an unobserved `BadgePage`, the next read recomputes all five derived nodes where the push recomputes `Initial` alone; attaching `listen` to a retained screen recomputes every `Stale` derived node under it.
- (b) Two lazy states. `Dirty`: a source it read changed, or a derived node it reads recomputed to a new result. `Stale`: a derived node it reads, directly or transitively, is `Dirty` or `Stale`. A read brings a `Stale` derived node's `Resolver` dependencies up to date in recorded order and recomputes it only when that made it `Dirty`; `recompute` marks its direct dependents `Dirty` on the read path. This is 3b's `Check` restricted to derived nodes nobody needs; the push keeps no such marks. Cost: one more state value, the dependency walk on reads, and a marking on the read path.

### 16. How the engine knows a derived node is needed

- (a) A count per derived node, kept current: the observers at its location plus its needed dependents (held, change 4, Needed derived nodes). Cost: `listen`, its stop function, and `setDependencies` of a needed derived node walk the region whose neededness changes.
- (b) On demand: walk an invalidated derived node's dependents upward to an observed location. Cost: the ancestors of every invalidated derived node, on every write.
- (c) Per write: mark the needed region from every observer down before invalidating. Cost: the whole observed region on every write.

### 17. A response that throws part-way

`normalize` writes in place and throws on the first position that does not match the AST (store.md, Errors), so the store holds the writes made before the throw.

- (a) `normalize` catches the throw and rethrows a `PartialWriteError` carrying the changes made before it and the original error as `cause`, with the original message; `normalizeData` marks those changes and rethrows (held, change 1 and change 2). Reason: a derived node that read a value written before the throw is marked, as it would be for a write that did not throw. Once the graph lands (change 2b), only `normalizeData` is exported from the package index, and `normalize` is internal: a module export that `environment.ts` and the tests import (held). Reason: a write through `normalize` alone marks nothing, so every derived node that read what it changed returns its old result with no error.
- (b) No wrapping: the changes are lost with the throw. Cost: every derived node that read a value written before the throw returns its old result until another write reaches the same source, with no error.
- (c) `normalize` validates the whole response against the AST before it writes. Cost: a second walk of every response, and `recordLink`'s checks run in both walks.
- (d) On any throw, `normalizeData` marks every derived node. Cost: every live derived node recomputes or is marked `Stale` after one malformed response.

### 18. A cycle of derived nodes

Changes 1 to 4 cannot form one (Ordering, Cycles). An eager client pointer can: with `field Pet.Card { bestFriend { Card } }`, `Pet:1.bestFriend` `Pet:2`, and `Pet:2.bestFriend` `Pet:1`, the derived node of `Card` on `Pet:1` reads the derived node of `Card` on `Pet:2`, which reads the derived node on `Pet:1`. Two places reach it. A `Stale` or absent derived node's computation reads a derived node that is computing, and `bringUpToDate` would recompute that derived node again inside its own computation. And `raiseHeight` reaches the dependency that started the raise. Neither has a check today; both recurse without end. Incremental fails at the second place with "adding edge made graph cyclic" (`ensure_height_requirement`, `src/adjust_heights_heap.ml` of `janestreet/incremental`).

Not decided; it arises with client pointers.

### 19. An upward read during `settle`

A derived node that recomputes during the drain of bucket `h` and reads a derived node at or above `h` can read a `Clean` derived node whose own dependency, at any depth, is still `Queued`, and receives that derived node's result from before the write. Every result is current when `settle` returns, because the `Queued` dependency's change reaches the reading derived node, raised by then, in a bucket not yet drained; but the reading derived node recomputes twice in one pass, once from a stale input (Ordering, Once per pass).

The same holds one level down, and Ordering's argument does not cover it. A derived node that is absent or `Stale` when a computation during `settle` reads it is computed on demand by that read (`createDerivedNode` or `recompute`, through `upToDateDerivedNode`). Its computation can read a `Queued` derived node, which change 4's `bringUpToDate` recomputes out of bucket order, or a `Clean` derived node at or above `h` that holds its result from before the write. So any rule for the drained derived node's own reads must also apply to the reads of every derived node computed on demand under it, recursively.

- (a) Accept it. Cost: R3's "no derived node is computed from a stale input, and no derived node is computed twice for one write" holds only for reads below the reader's height; the resolver is called with stale data, and that result is stored and queues its dependents before the second recomputation replaces it. Observers are notified once, after `settle`, so none receives the stale result.
- (b) Before a read returns a derived node at or above the reader's height, bring up to date every `Queued` derived node it reads, transitively, lowest height first. Cost: a walk of the derived nodes that derived node reads, transitively, on every upward read during `settle`, and `Queued` derived nodes recomputed out of bucket order. Not held: the recomputations run as a nested drain while the reading derived node's walk is half done and its new dependencies are half collected in `compute`'s local list and not yet recorded by `setDependencies`, so `invalidate` and `raiseHeight` run against a graph whose edges for that derived node are its old ones.
- (c) Abandon the computation at the upward read, before any resolver above the read runs; raise the derived node above the derived node it read and queue it again at its new height (held). Reason: the reader walk (`readData` with `contextHooks`) collects all data before `callResolver` runs, so a computation abandoned at its first read of a derived node at or above `h` has not called its resolver. When `settle` reaches the new bucket, every bucket below it is drained, so the derived node recomputes once in the pass, from current inputs, and every resolver runs at most once per pass with current inputs (R3). Cost: the partial walk before the abandoned read is discarded, and the derived nodes computed on demand under it that were abandoned walk again on the re-run. Not held (a): it breaks R3.

Held: (c). Not decided.

How (c) is implemented against change 2's read path (`upToDateDerivedNode`, `bringUpToDate`, `createDerivedNode`, `compute`) and change 4's `recompute` and `settle`:

- Detection is in `compute`, in the `readDerivedNode` closure of the context it builds, which is the one place a computation reads a derived node. During the drain of bucket `h`, before it calls `upToDateDerivedNode`, it looks up `graph.derivedNodes.get(id)`. When a derived node is stored there with `height >= h`, in any state, the closure does not bring it up to date and returns an abandon that carries that derived node's height. `h` is the height of the derived node `settle` is bringing up to date, not the height of the derived node whose computation made the read.
- A `Queued` derived node has height `>= h`, because every bucket below `h` is empty (Ordering), so every read of a `Queued` derived node during `settle` abandons. A `Clean` derived node below `h` is current: everything it reads is lower, `Clean` (change 2's invariant), and not `Queued`. A `Clean` derived node at or above `h` can have a `Queued` dependency at some depth; the rule abandons without walking to find out. An absent derived node has no height; it is created by `createDerivedNode`, whose `compute` checks each of its reads against the same `h`. A `Stale` derived node below `h` is recomputed by the read, and its reads are checked against the same `h`. So the rule covers derived nodes computed on demand, recursively, and comparing with `h` rather than with the nested reader's height does not abandon a read of a current derived node between the two heights.
- The abandon propagates as a value, by the convention the walk already has. `contextHooks`'s `readResolver` returns the abandon as part of its `TFailure`; `allOk` stops at the first result whose `kind` is not `Ok` and returns it unchanged, so the walk makes no read after the abandoned one; `missingDataFor` applies only to `MissingData`; `readData` returns the abandon; and `resolverDefinition`'s computation returns `data` unchanged when `data.kind !== 'Ok'`, so `callResolver` is not called. `compute` receives the abandon in place of a value and returns it to its caller, so the abandon passes out through every computation between the read and the drained derived node.
- `createDerivedNode` that receives an abandon from `compute` returns it without building the derived node: nothing is added to `derivedNodes` or `owned`, and `setDependencies` is not called, so no edge names it. The next read of its id calls `create()` again and computes it.
- `recompute` that receives an abandon leaves the derived node's `value` and `dependencies` unchanged, calls neither `setDependencies` nor `raiseHeight` from `heightAbove`, and invalidates no dependent. A `Stale` derived node computed on demand stays `Stale`; change 2's invariant holds, because its dependents are `Stale` or are the computations being abandoned.
- `bringUpToDate` of the drained derived node, when `recompute` reports an abandon carrying height `k`, does not mark it `Clean`. The derived node stays `Queued`; `raiseHeight(graph, node, k + 1)` raises it and its dependents, and `bucket(graph, k + 1).add(node)` queues it, since `bringUpToDate` removed it from its bucket. `k >= h`, so the bucket is above `h`, and no drained bucket gains a derived node (Ordering).
- The re-run. When `settle` drains bucket `k + 1`, the derived node of height `k` is current, so the re-run's read of it does not abandon. A derived node created on demand between them gets a height of at most `k + 1`; when it equals the drained height, `recompute`'s existing raise (change 4, Heights) puts the re-run derived node above it after a computation from current inputs. In Ordering's trace, `X` abandons at its read of `FriendCard` on `Pet:1` (height 2), rises to 3, and is queued at 3; `PetName` on `Pet:2`, `FriendLabel`, and `FriendCard` recompute in buckets 0 to 2; `FriendCard`'s invalidation of `X` finds it `Queued`; `X` recomputes once in bucket 3 and its resolver is called once.
- Termination. Each abandon raises the drained derived node strictly, because `k + 1 > h`. Under changes 1 to 4 every edge, recorded or read, goes from a derived node of a field to a derived node of a field its reader AST selects, and that graph of client field selections has no cycle (Ordering, Cycles). Every height set by `heightAbove`, by `raiseHeight`, or by an abandon is one more than the height of a derived node whose field is below its own in that graph (for an abandon under a derived node computed on demand, below by at least two). So no derived node's height exceeds the largest height at the start of the pass plus the longest chain of client field selections below its field, and the abandons of one pass are finite. A cycle of derived nodes through an eager pointer is open decision 18.
- Outside `settle` nothing is `Queued` (Reads during recalculation), so the read path (`track`, `retainRoot`, `observe`) makes no check and never abandons.

Two choices the doc does not establish:

- How `h` reaches the closure.
  - (i) A parameter: `settle` passes the drained height to `bringUpToDate`, which passes it through `recompute`, `compute`, the context's `readDerivedNode`, `upToDateDerivedNode`, and `createDerivedNode`; `track`, `retainRoot`, and `observe` pass a value that means no drain, such as `{ kind: 'Read' }` against `{ kind: 'Drain'; height: number }`. This follows change 2's rule that the context is a parameter built per computation, with no ambient current computation. Cost: one more parameter on six functions.
  - (ii) A field on `RecomputationGraph` holding the drained height or none, set by `settle` around each bucket. Cost: graph state that every read path must find unset; it is the same shape as open decision 8 (b)'s phase field.
- The abandon's type. The engine must tell it from a `TValue` without inspecting `TValue`.
  - (i) A tagged union: a computation returns `{ kind: 'Value'; value: TValue } | Abandoned`, with `Abandoned = { kind: 'Abandoned'; height: number }`, and `readDerivedNode` returns the same union; `contextHooks` unwraps `Value` and passes `Abandoned` as part of `ReadHooks<ReadResolverThrew | Abandoned>`. Cost: every computation wraps its value.
  - (ii) `Abandoned` is a class the engine exports, tested with `instanceof`, and a computation returns `TValue | Abandoned`. Cost: correctness rests on no `TValue` being an `Abandoned` instance, which no type enforces.
  - Under either, `track`, `retainRoot`, and `observe` never receive an abandon, and their types must not admit one (AGENTS.md, Panic and UnexpectedError). With a parameter (the first choice's (i)), the abandon type can ride on it as `never` on the read path, as `evaluatingHooks` is `ReadHooks<never>`; the computation type then becomes generic over it.

### 20. Which request a request-state selection refers to

A request-state selection (The engine (2a), after `ComputeContext`) reads the state of one request, and the walk reports `requestSource(key)` for it. The question is how the selection determines `key`.

A derived node is keyed by (field, record, variables) (`resolverDerivedNode(root, resolverStoreKey(artifact.selectableName, variables))`), not by request. The same resolver on the same record, reached from two entrypoints, is one derived node with one value. "The request that fetched me" has no single value for that derived node, and keying the selection that way would split the derived node per request, which breaks that sharing.

A key that the reader's own location determines works. A loadable field's request is such a key. The loadable query of `P.f @loadable` is the entrypoint query of `entrypoint P.f` with `P`'s wrapper, whose wrapper variable is the parent record's id (graphql-protocol/node-entrypoints-and-loadable.md, `@loadable` later), and the request cache keys a request on the query text and the operation variables, `entrypoint.queryText + JSON.stringify(stableCopy(variables))` (network-and-subscriptions.md, The request cache, `NetworkRequestKey`; the same key is `RequestKey` in its Request entries in the store). That key is a function of (record, field, arguments), all of which the reader holds at the selection.

The state of an entrypoint's own request stays with the caller of `fetchEntrypoint`, which holds the returned `PromiseWrapper`; in React it reaches a component through the suspension derived node (Other derived node kinds), not through a selection.

Not decided.
