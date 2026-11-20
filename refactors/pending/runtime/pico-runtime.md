# Runtime: resolver results maintained by `normalize`

Status: under discussion. Not approved for implementation. This is the planned runtime step after v1 (`refactors/pending/runtime/store.md` and `refactors/pending/runtime/reader-ast.md`). It describes only what changes from v1, and from upstream where v1 matches upstream.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; `libs/isograph-react/src` is unchanged since `79e2a3c0`, which the other runtime docs cite). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`.

## Goal

Renders do no work. The `normalizeData` that retains a query, or the `retainQueryWithoutMakingNetworkRequest` that retains a query served from the store, computes every resolver result its entrypoint reaches. After every later write, the write marks every stored result that read what it changed, and every result above those, and runs no resolver. A pull clears a mark: it brings what the result read up to date first, and recomputes the result only when one of its inputs changed value. Before the write returns, the runtime pulls every observed result (open decision 28, held (a)) and every mounted result with an effect (Values with lifecycles). Every other marked result is pulled when next read. A render reads a precomputed value and nothing else.

Upstream reads and evaluates during render. `useReadAndSubscribe` reads the fragment's data in a `useState` initializer (`react/useReadAndSubscribe.ts:33-35`), `useResult` calls the resolver in the render body (`react/useResult.ts:45-64`), every nested eager resolver is called again on every read (`read.ts:600`), and the component subscribes in an effect after the render (`react/useRerenderOnChange.ts:21-37`, `subscribe.ts:15-40`). After a write, `callSubscriptions` re-reads every subscription whose recorded records overlap the written ones (`subscribe.ts:63-109`, called at `makeNetworkRequest.ts:271`). The deviation is the user's: no read-and-subscribe during render, because a render that reads and evaluates does resolver work on the render path, repeats it for every component that reads the same resolver, and needs a second read in the effect to catch writes between render and subscribe (`subscribe.ts:32-36`).

## Two levels

```text
level 2: observers        listen(location, onChange)  snapshot(location)
                                 ^ notified after level 1 settles
level 1: resolver graph   a DerivedNode per (record, resolver store key), in environment.graph, marked by normalizeData, pulled by reads
                                 ^ reads
store:                    server values on records, in environment.store, written by normalize
```

Level 1 is the resolver graph, held on the environment beside the store (The engine, the store, and the environment). Its derived nodes are stored resolver results, one per client field, record, and argument values. Its edges are each derived node's dependencies: the server values and the other resolver results its reader AST read. How level 1 brings its derived nodes up to date after a write, and which edges it stores to do so, is the Recomputation architecture below. `normalizeData`, the environment's write, owns level 1: it computes the derived nodes a newly retained query reaches, and after every write it marks the derived nodes the write reaches and pulls the observed ones. Level 1 owns ordering: a pull brings a derived node's inputs up to date before the node uses them. No level 2 code runs while level 1 is computing.

Level 2 is observers. An observer names one location of level 1 (a record and a resolver store key) and a no-argument callback. It holds no value. After level 1 settles, each observer whose location's stored result changed is called once. `snapshot` returns the stored result and computes nothing. This is the shape React's `useSyncExternalStore(subscribe, getSnapshot)` takes (Level 2), so a React package can render from level 2 while `@isograph/runtime` stays React-free.

With React, components are resolver results too (`@component`, Later): a component's derived node is a level 1 node like any other, and a mounted component is a level 2 observer of its own derived node.

## Recomputation architecture

How level 1 keeps its derived nodes current after a write. pico's algorithm (global epoch, `time_updated` and `time_verified`, verification of dependencies before a derived node, backdating) is one candidate, not a given. pico was built for the compiler, where an LSP request demands one memo and everything else stays unverified; the runtime's goal makes every observed derived node demanded after every write. This section states the requirements, lays out five candidates against three examples, and compares them. The option it ends on, 3b (mark, then pull), is held by the user's direction, and the user rejected 3a's recomputation inside the write; it is not approved for implementation. The candidates are written against the store's functions: a write is a `normalize` or a `garbageCollect`, and the state a candidate keeps is described without saying where it lives. Under the held design that state lives in `environment.graph`, and the writes are `normalizeData` and `garbageCollectEnvironment`, which call the store's functions (The engine, the store, and the environment).

### Requirements

From the goal and the decisions this doc, store.md, and reader-ast.md settle:

- R1. A render does no work. `snapshot` is a lookup; a render reads no store value and calls no resolver.
- R2. Eager where observed. The `normalize` that retains a query computes its entrypoint's result and every result that result reaches. After every later `normalize` or `garbageCollect`, every observed result is current when it returns (open decision 28, held (a); its (b) moves that pull to the reader). A result nobody observes is current when read: a write that reaches it marks it, and the next read pulls it (open decision 3, held (b)). A `snapshot` of a marked result returns its last computed value; `listen` brings it up to date (change 4).
- R3. Order. After a write, a derived node recomputes only after every stored result it reads is current. No derived node is computed from a stale input, and no derived node is computed twice for one write.
- R4. Equality cutoff is an optimization. It is not a requirement, and no candidate can guarantee it. A derived node whose recomputed result is `===` to its previous result does not mark its dependents. This stops propagation at a primitive result and at a result that is a stored or reused object. A result that is a new object is never `===` the previous one, so it always propagates, even when it is structurally equal. A general structural equality over arbitrary JavaScript values (functions, class instances, cycles) cannot be both correct and cheap, so the runtime does not attempt one. Server values keep a check at write time: `normalize` compares each value it writes with the stored one by the shape the AST gives it (change 1), as upstream does: a leaf by `===`, a link by `entityName` and `id`, and a list of links item by item and by length. A leaf whose value is a JSON object or list is a new object on every response, so it always counts as changed. Without the check, every response marks every written field changed and the whole retained tree recomputes on every response. With `Rex` to `Ray`, `Initial` recomputes to `"R"`, a string `===` the previous one, and nothing above it recomputes. With `Rex` to `Ray`, a resolver that returns `{ length: data.name.length }` recomputes to a new `{ length: 3 }`, and every derived node that reads it recomputes.

  Upstream never compares resolver outputs either. `callSubscriptionIfDataChanged` (`subscribe.ts:111-162`) re-reads the data the reader AST selects with `readButDoNotEvaluate` (`:117-136`) and compares that data, not the resolver's result, with `mergeObjectsUsingReaderAst(subscription.readerAst, oldItem, newItem)` (`:138-142`), calling the callback when the merge does not return the old item (`:152`). The merge (`areEqualWithDeepComparison.ts:4-121`) walks the reader AST, so it compares only store data: scalars by `===` (`:22-23`), lists item by item (`:33-53`), linked selections recursively (`:24-29`), links by entity and id (`:96-113`), nested resolver values by `===` (`:84-95`), and client pointers as always unequal (`:65-69`). Where a part is equal it writes the old value into the new object (`:77-82`, `:48`, `:110`) and returns the old object when every part is equal (`:120`), so equality also keeps identity stable. The well-defined place for structural equality is therefore a resolver's inputs, which are store data shaped by the reader AST. Under held 3b a derived node recomputes only when an input it read changed: a source is compared by `normalize` at write time by its AST shape (change 1), and a derived node by `===`. Reading data along the reader AST and keeping the old object where the new data is equal (upstream's merge) is the way to give a snapshot stable identity, which `useSyncExternalStore` requires of `getSnapshot` (network-and-subscriptions.md, The snapshot: `subscriptionSnapshot` returns the same object until a re-read is unequal; react.md's `useSubscriptionSnapshot` relies on it).
- R5. Observers are called once per write, after level 1 settles, and a `snapshot` inside a callback returns a settled result.
- R6. One derived node per client field, record, and argument values, shared by every parent and query that reads it (The same resolver selected twice).
- R7. Dependencies follow the data. A recomputation can read different inputs than the previous one (a link moved from `Pet:2` to `Pet:3`, a `MissingData` that became present). The next write is judged against the inputs of the last computation only.
- R8. `normalize` is the only writer of server values. It knows exactly which (record, store key) pairs it changed and which records it created, and returns them (change 1). `garbageCollect` is the only deleter, knows which records it deleted, and returns them (change 2).
- R9. Minimal machinery: the fewest stored fields and functions that meet R1 to R3 and R5 to R8, give R4's optimization, and allow R10.
- R10. Scheduled recalculation can be added later without redesign (Follow-ups 9). It has two halves. The first is laziness: a derived node nobody observes is only marked on a write and is pulled when next read. Changes 2 to 4 build it. The second is deferral: a caller-supplied scheduler decides when, and at which priority, observed derived nodes are brought up to date, with observers notified once per pass that settled them. Changes 1 to 4 do not build it, and the architecture must allow it without undoing anything in them. Deferral requires that marking a write is separate from recomputing, that a pass can leave a derived node marked for a later pass, and that a pass can choose which observed derived nodes it pulls.

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

Step A, landed, is this candidate with `readButDoNotEvaluate` and no overlap filter.

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
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
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

What epochs buy: correctness without reverse edges. A derived node nobody demands is never visited, a dependency list is rebuilt from scratch on each computation so there is no inverse to keep consistent, and deleting a derived node unlinks nothing. In pico's setting this is the right trade: a compiler with a whole project's memos answers a hover by verifying the few it demands. In the runtime, R2 demands every observed derived node after every write, and held 3b is lazy for every other derived node through its marks, so epoch verification's laziness buys nothing 3b's marks lack; what remains is a walk over every edge under the observed roots on every write, and a stamp on every server value.

### Candidate 3: push along reverse edges

`normalize` already knows `C` (R8). Each derived node stores, besides its forward dependencies, the set of derived nodes that read it (`dependents`), and the store keeps the same for server locations. A write follows those reverse edges from `C`, so its cost is bounded by `A`, not by `E`. No epochs: a server value stays an untagged `StoreValue`.

Data structures common to the three orderings below:

```ts
// from libs/isograph-runtime/src/store.ts
export type DerivedNode = {
  readonly location: DerivedNodeLocation;
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
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

In isograph each derived node's height would be computed at run time from the derived nodes it read. When a derived node's computation finishes, its height is 0 when it read no other stored resolver result, else `1 + max(height)` over the derived nodes it read. Every derived node it read was brought up to date by that read, so their heights are known. A derived node's height is above the height of every derived node it reads.

Heights depend on data, because what a derived node reads depends on data:

- A nullable link. `FriendLabel` on a pet whose `bestFriend` is `null` reads no `PetName` derived node, so its height is 0. `FriendLabel` on `Pet:1`, whose `bestFriend` is `Pet:2`, reads `PetName` on `Pet:2`, so its height is 1.
- A refinement. A `Refinement` node reads its selections only when the record's entity is the condition's entity, and is `null` otherwise (`ReaderRefinementSelection`, `libs/isograph-runtime/src/artifacts.ts`; `read.ts:150-153`). A derived node of a field whose `Resolver` node sits under `asPet` reads that resolver's derived node on a `Pet` record and reads no derived node under the refinement on a record of another entity, so two derived nodes of one field have different heights when their records' entities differ.
- An eager client pointer (Later). The derived node of `field Pet.Card { bestFriend { Card } }` on `Pet:1` reads the derived node of `Card` on `Pet:2`, so its height is one more than that derived node's.

A height set by one computation can be too low for a later one: after a write sets a `null` `bestFriend` to `Pet:2`, `FriendLabel`'s recomputation reads `PetName` on `Pet:2`, whose height 0 equals its own. Height adjustment restores the order, as Incremental's `adjust_heights` does (`src/adjust_heights_heap.ml` of `janestreet/incremental`). When a recomputation records a dependency whose height is at or above the derived node's own, the derived node's height rises to one more than that dependency's. Then each dependent whose height is now at or below the height of the derived node it reads rises to one more than it, transitively. A dependent queued in the bucket queue moves to its new bucket, as Incremental's `adjust_heights` calls `Recompute_heap.increase_height` on a node in the recompute heap. Heights only rise: a recomputation that reads lower derived nodes, or fewer, keeps its height. Outside a call that is computing a node, every derived node's height is above the height of every derived node it reads.

Under changes 1 to 4 the runtime graph has no cycle. Every `Resolver` node of `F`'s reader AST selects a field that query generation inlines into `F` (`inlined_fields`, `crates/isograph_compiler/src/merged_selection.rs`), and a cycle of inlined fields is an `InlineCycle` query error (query-generation.md, Cycles). A derived node of `F` on a record is computed by reading `F`'s reader AST from that record and calling `F`'s resolver with the data read. The resolver receives `data` and `parameters` and no store (open decision 8 covers a closure that reaches one), so every stored result the derived node reads is read by a `Resolver` node of `F`'s reader AST: a derived node of one of the fields it selects, on whichever record the links lead to. Links choose the record, never the field, so a cycle of derived nodes would be a cycle of client field selections, and there is none. So adjustment never reaches the dependency that started it. This argument is this doc's, not Incremental's. An eager client pointer makes a cycle of derived nodes possible (open decision 18).

A write: each derived node that read a changed source goes into bucket `height`; buckets are drained lowest first; each derived node recomputes, and when its result is unequal (`===`, R4) its dependents go into their buckets; when it is equal nothing is queued. A derived node read during a recomputation usually has a lower height, so its bucket is already drained and it is current. A derived node read at or above the reader's height (an upward read, one of the data-dependent cases above) can be `Clean` with an input still queued below it, so it returns its result from before the write. R3 then needs the reader's computation abandoned at that read, before the reader's resolver runs, and the reader raised above the node it read and queued again at its new height; without the abandon, the reader's resolver runs once with the stale result and again after the queued input recomputes. Nothing is marked before it is known to need recomputing: the work of a write is the derived nodes that recompute and the direct dependents of the ones whose result changed.

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

The unchanged `Initial` stops propagation at once: `Monogram`, `Signature`, `Badge`, and `BadgePage` are never visited. A changed `Initial` queues `Monogram` and `Signature`, and `Badge` is queued once although both change. The `===` cutoff stops propagation only when a resolver returns the same value; a resolver that returns a new object on each run never cuts off, so these traces, whose resolvers return strings, show the best case.

New object, an observer at `ShapePage`, `Rex` to `Ray`: `NameShape` (0) recomputes to a new `{ length: 3 }`, which queues `ShapeLabel` (1); `ShapeLabel` recomputes to `"3 letters"`, `===` the old string, so `ShapePage` (2) is not queued.

Chain, observers at `FriendPage` and `PetSummary`, `Pet:2.name`: `PetName` on `Pet:2` (0); then `FriendLabel` (1) and `PetSummary` (1; it also selects `RecentCheckins`, 0, which is not queued); `FriendCard` (2); `FriendPage` (3). Each once. The link move: `Pet:1.bestFriend` queues `FriendLabel` (1), whose recomputation reads `PetName` on `Pet:3`, height 0; absent, it is computed by the read, and its dependencies move from `PetName` on `Pet:2` to `PetName` on `Pet:3`.

A link set from `null`, an observer at `FriendPage` with `{ id: "4" }`, `Pet:4.bestFriend` `null`: `FriendLabel` on `Pet:4` has height 0, `FriendCard` 1, `FriendPage` 2. A write that sets `Pet:4.bestFriend` to `Pet:2` queues `FriendLabel` at 0. Its recomputation reads `PetName` on `Pet:2`, height 0, so `FriendLabel` rises to 1, `FriendCard` to 2, and `FriendPage` to 3. `FriendLabel`'s result changed, so `FriendCard` is queued at 2, its new height, and `FriendPage` after it at 3.

- Cost per write: `O(|A'| + dependents of A')` plus one step per bucket up to the largest height queued, plus the adjustment walk after each upward read. Nothing that will not recompute is visited, except the dependents adjustment raises.
- Failure mode of plain 3a: it recomputes every derived node with a changed input, whether or not anything needs it. A derived node whose retention was disposed, or that a plain `read` stored, recomputes on every relevant write until `garbageCollect` deletes it.

3a with `Stale` marks adds them for derived nodes nobody needs. It recomputes inside a write exactly the needed derived nodes: those an observer is at, and those a needed derived node reads, transitively. A retained derived node nobody observes is not needed, so a write marks it `Stale` and does not recompute it. Each derived node counts the observers at its location plus its needed dependents (`neededCount`); it is needed when the count is above 0. A derived node whose input changed is queued when it is needed. Otherwise it and its dependents, transitively, are marked `Stale` and not queued; the marking stops at a derived node already `Stale`. A read of a `Stale` derived node recomputes it, and each `Stale` derived node it reads is brought up to date by that read first. Every dependent of a derived node nobody needs is itself not needed, so `Stale` marking never reaches a needed derived node, and the queue never holds a derived node nobody needs.

Diamond, `BadgePage` retained and not observed, `Rex` to `Max`: `Initial` is not needed, so it, `Monogram`, `Signature`, `Badge`, and `BadgePage` are marked `Stale` and no resolver runs in the write. A later `listen` at `BadgePage` reads it: `BadgePage` recomputes, its read of `Badge` recomputes `Badge`, whose read of `Monogram` recomputes `Monogram`, whose read of `Initial` recomputes `Initial`; then `Badge`'s read of `Signature` recomputes `Signature`, reading the `Clean` `Initial`. Each once. `Rex` to `Ray` the same way recomputes all five on that read: a `Stale` derived node recomputes when read whether or not what it reads changed.

#### 3b. Mark, then bring up to date (Reactively, MobX)

Reactively's algorithm (`packages/core/src/core.ts` of `milomg/reactively`): each node is `CacheClean`, `CacheCheck` ("might be stale, check parent nodes to decide whether to recompute"), or `CacheDirty`. A write marks "direct children dirty and their deeper descendants check", stopping at nodes already marked. Reading a node runs `updateIfNecessary`: for `CacheCheck`, bring each source up to date in order, stopping at the first that made this node dirty; for `CacheDirty`, recompute; then clean. A recomputation that produced an unequal value marks its children dirty; an equal one leaves them at check, so they clean without recomputing. MobX's derivation states are the same scheme: `UP_TO_DATE_`, `POSSIBLY_STALE_` ("some deep dependency changed, but don't know if shallow dependency changed"), `STALE_`, and `shouldCompute` checks the computed dependencies of a possibly stale derivation in order (`packages/mobx/src/core/derivation.ts`).

Here the derived node gains `state: 'Clean' | 'Check' | 'Dirty'`. A write, step by step:

1. While writing, `normalize` marks: for each changed location in `C`, every derived node in its dependents set becomes `Dirty`; a derived node that was `Clean` marks its dependents `Check`, recursively, stopping at a derived node not `Clean`. Marking runs no resolver.
2. Settlement brings the marked roots up to date: the derived nodes that are both marked by this write or an earlier one and the stored result at an observed location. It does not visit an unmarked root. A `Clean` derived node returns at once. A `Check` derived node brings its `Resolver` dependencies up to date in recorded order and stops at the first after which it is `Dirty`. A `Dirty` derived node recomputes; each stored result it reads is brought up to date first, by the read. A recomputed derived node whose result is unequal sets its dependents `Dirty`. The derived node ends `Clean`.

Diamond, `Rex` to `Max`:

```text
mark   name changed: Initial Dirty; Monogram, Signature, Badge, BadgePage Check
       marked roots: BadgePage (the only observed location; Initial, Monogram, Signature, Badge are marked and are not roots)
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

With observers on `Badge` and `Initial` as well as on `BadgePage`, the marked roots are `Initial`, `Badge`, and `BadgePage`. Settlement brings each up to date in any order; the first brings up to date every marked derived node it reaches by the recursion, and the later ones find those derived nodes `Clean`. The calls are the same as in the trace above.

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
- Cost per write: marking `O(|A| + edges within A)`; settlement `O(marked roots)` to start, plus `O(|A| + edges within A)` for the walk; `A'` recomputations. Held 3b finds the marked roots by visiting every observed location, one step for each that is `Clean` (Held: 3b).
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
- R3: 1 fails "nothing twice"; 2, held 3b, 3c, 4, 5 meet it. 3a meets it only if an upward read during the drain abandons the reading computation and requeues it higher (Candidate 3a); without the abandon, the reader's resolver is called with a stale input and its derived node recomputes twice in one pass.
- R4, an optimization: 2, 3a, 3b, 3c, 4 stop propagation at a derived node whose result is `===` its previous one (a primitive, or a stored or reused object), and propagate every new object. 3a with `Stale` marks stops it on the push and not on a read of a `Stale` derived node. Held 3b stops it on every pull (open decision 15 (b); 15 (a) removes it). 1 compares only at the subscription; 5 only at the root. No candidate compares resolver outputs structurally, because that comparison is not well-defined on arbitrary JavaScript values.
- Structural equality on inputs: the one place it is well-defined is a resolver's inputs, which are store data shaped by the reader AST. 1 does it at the subscription with upstream's `mergeObjectsUsingReaderAst`, which also keeps the old object when equal and so gives stable identity. 2, 3a, 3b, 3c, 4 get the server half from `normalize`'s comparison by AST shape (change 1: an unchanged write changes no source, so nothing is queued, marked, or stamped; a leaf whose value is a JSON object or list always counts as changed) and compare resolver inputs by `===`. A snapshot whose identity must stay stable across a recomputation with equal data needs upstream's merge applied to the data read along the reader AST; it does not need structural equality on outputs.
- R5 and R7: every candidate meets them.
- R10: held 3b meets the lazy half from day one: marking runs in the write apart from the pull, and a derived node nobody observes keeps its mark until read. Deferral needs no new state: `settle` iterates the observed locations, and a scheduler chooses which of them each pass pulls; a derived node a pass skips keeps its mark (Follow-ups 9). 3a with `Stale` marks allows it: a deferred write marks as if nothing were needed, so every derived node its changes reach becomes `Stale`, and the observed derived nodes the marking reaches are recorded for the deferred pass. 2 allows it: nothing is verified until demanded, and a deferred pass is a later verification of fewer roots; each pass still walks every live edge under its roots. 4 allows it: Preact keeps unsubscribed computeds lazy and queues effects to the end of a batch, so priority goes on that queue. Plain 3a and 3c fail: their propagation recomputes every affected derived node as it goes, so skipping an unobserved derived node leaves it stale with no record of it. 5 allows it per root at full cost per pass. 1 has nothing stored to defer; scheduling exists only in React's re-render.
- R6: 1 fails; the rest meet it.
- R8, what each does with `C`: 1 needs it at record granularity; 2 turns it into stamps and rediscovers it by walking `E`; 3a, 3b, 3c, 4 start from it; 5 ignores it.
- Per write with a small change on a large screen: 1 `O(overlapping subscription trees)`; 2 `O(R + E)`; 3a with `Stale` marks the `A'` recomputations among needed derived nodes plus the dependents of those whose result changed, and one `Stale` mark per derived node nobody needs that the write reaches and that is not `Stale` already; held 3b `O(A)` marking, one step per observed location, and the pull of the marked derived nodes the observed ones read; 3c `O(A)`; 5 `O(L)` resolver calls.
- Memory beyond the results: 2 a tag and epoch on all `S` server values, two epochs and forward edges per derived node; 3a, 3b, 3c, 4 forward and reverse edges, plus a dependents index for the server locations live derived nodes read (only those, not all of `S`); 3a with `Stale` marks a state, a needed count, and a height per derived node, and the bucket queue; held 3b a mark per derived node; 4 a version per edge.
- Machinery (R9): 2 has the fewest invariants (no inverse to keep). 3a with `Stale` marks adds the inverse, a three-valued state, the needed count, the bucket queue, height adjustment after an upward read, and the abandon of an upward read. Held 3b adds the inverse, a three-valued mark, marking, and the pull. 3c and 4 add 3b's pieces plus counters or versions.
- Unreached derived nodes (a disposed retention before `garbageCollect`, a plain `read` outside any retention, a retained query nobody observes): 2, 3a with `Stale` marks, held 3b, and 4 do not recompute them until read; 3a and 3b mark each once and later writes stop at it. Plain 3a and 3c recompute them.
- A read of a derived node nobody observes, after a write that reached it: held 3b brings its dependencies up to date in order and skips the recomputation when none changed (open decision 15 (b)); 3a with `Stale` marks recomputes every `Stale` derived node the read reaches.

For a sense of scale, from no measurement: a screen with 2,000 live derived nodes averaging 5 dependencies is 10,000 epoch comparisons, each two or three `Map` lookups, for every write under 2, including a write of one field that one derived node reads; under 3a the same write recomputes that derived node, and visits its dependents only if its result changed; under held 3b it marks that derived node and every derived node above it, and the pull of each observed location above it walks those marks and recomputes what changed.

### Held: 3b, mark, then pull

A write marks and a pull recomputes. `markChanged` marks `Dirty` each derived node that read a changed source, and `Check` every derived node above those, stopping at a derived node already marked; it runs no resolver. A pull is `bringUpToDate` (change 2, The engine): a `Clean` derived node returns at once; a `Check` derived node brings its `Derived` dependencies up to date in recorded order and recomputes only when one of them recomputed to a value `valuesEqual` rejects (open decision 15 (b)); a `Dirty` derived node recomputes, and each derived node its computation reads is pulled first. Nothing computes while marking, so no resolver reads a half-written store.

Every read of a stored result is a pull: `track`, a computation's `readDerivedNode`, `retainRoot`, `observe`, and `settle`. Which derived nodes are pulled before a write returns:

- Observed derived nodes. `settle` pulls each derived node an observer is at (`graph.observers`), and `notify` then calls each observer whose value changed (change 4). This is open decision 28 (a); its (b) leaves observed derived nodes marked, calls their observers, and lets the reader pull.
- Mounted derived nodes with an effect (Values with lifecycles). `settle` pulls each, and `reconcileEffects` replaces the effect of each whose value changed. Changes 1 to 4 have none.
- The nested derived nodes of a subscription. `callSubscriptions`, after `normalizeData`, re-reads every subscription, and the re-read pulls each nested derived node through `storedResultHooks` (change 2, Subscriptions).

Every other marked derived node stays marked until a read pulls it: a retained query nobody observes (open decision 3, held (b)), a plain `readStoredResult`'s result, the derived nodes of a disposed retention.

Against 3a, 3b pays the marking walk. Every transitive dependent of a changed source is marked on every write, and the pull of an observed location above an unchanged result walks those marks again and clears them (Candidate 3b, Why `Check` marks exist). In the diamond with `Rex` to `Ray` under an observer at `BadgePage`, 3a visits `Initial`; 3b marks `Monogram`, `Signature`, `Badge`, and `BadgePage`, and `settle`'s pull of `BadgePage` clears them. With no cutoff (every resolver returns a new object), `A'` is `A` and the two make the same recomputations. In return, 3b computes nothing inside the marking, keeps the cutoff on the read of a derived node nobody observes, and has no height, no bucket queue, no needed count, no height adjustment, and no abandon of an upward read. The pull is the only place a resolver runs, so the read path and the post-write path are one function.

The cost held 3b takes on is the inverse edge set: memory per edge and an invariant (the dependents sets equal the inverse of the dependency lists) whose violation is a silently stale result. That is contained by making one function, `setDependencies`, the only writer of either side. The inverse also pays for itself elsewhere: it is the reference count constructors and destructors need (Follow-ups 1, open decision 11), and it is the index open decision 6 (b) needs. Removing epochs also removes the tag and stamp from every server value, so records stay store.md's untagged maps and `check`, `garbageCollect`'s mark, and the reader's server reads do not change.

Held 3b's bound does not depend on equality cutoff, which stays a `===` optimization (R4): marking visits `A` once per write and stops at derived nodes already marked; only pulled derived nodes recompute, and only when an input changed; and `normalize`'s comparison by AST shape (change 1) keeps an unchanged server value out of `C`. 5 recomputes all of `L`, and 2 compares every edge under the observed roots.

### What depends on this choice

Written for held 3b, and changed if another candidate is chosen: Steps; change 2 except The resolver store key and Results, equality, and a resolver that throws; Ordering; change 3's Computing the new root, Serving from the store, Unobserved results are lazy, and Garbage collection; change 4 except `listen`, `snapshot`, and `Observed`; the engine's functions in changes 2 to 4; Values with lifecycles' Reconciliation; the mentions of `dependents` and marking in Follow-ups 1, 8, 9, and 10; and open decisions 2, 3, 4, 6, 8, 11, 14, 15, 16, 18, 19, 22, 26, 27, and 28. Each of those sections in Steps and changes 2 to 4 opens with a line saying so.

Independent of it: Two levels, Step A, End-user experience, change 1, The resolver store key with `ReaderArtifact.selectableName`, Results, equality, and a resolver that throws, change 3's `normalizeData` signature, `Retention`, and Resolver markers, change 4's `listen`, `snapshot`, and `Observed`, The engine, the store, and the environment, Other derived node kinds, Later, the rest of Follow-ups, and open decisions 1, 5, 7, 9, 10, 12, 13, 17, 20, 21, 23, 24, 25, and 29.

## The engine, the store, and the environment

Settled (open decision 12). Changes 2 to 4 are written for it.

```text
environment  IsographEnvironment { store, graph, retainedQueries, networkFunction, networkRequestCache, subscriptions }
             normalizeData, garbageCollectEnvironment, readStoredResult, dispose, listen, snapshot
store        Store = Map<EntityName, Map<RecordId, StoreRecord>>
             normalize, check, garbageCollect, read, readButDoNotEvaluate
graph        RecomputationGraph<ResolverResult<unknown>> { derivedNodes, sourceDependents, owned, observers, valuesEqual }
             track, markChanged, deleteSource, retainRoot, collect, settle, observe, valueOf, notify
```

- The store is store.md's nested map, unchanged. Its functions take the `Store` and import nothing from the graph or the environment. `normalize` returns the changes it made (change 1), `garbageCollect` returns the records it deleted (change 2), and the reader's walk reports each read to the `ReadHooks` its caller passes (change 2).
- The engine, `graph.ts`, knows two kinds of id, `SourceId` and `DerivedNodeId`, both opaque and both minted by its caller (change 2). It imports nothing from the store, the reader, or the environment. Its state is one value, `RecomputationGraph`.
- The environment (network-and-subscriptions.md, The environment) holds the store and the graph as two fields, beside the retained queries, the network function, the request cache, and the subscriptions, as upstream's `IsographEnvironment` holds `store` beside its other state (`IsographEnvironment.ts:73-93`). Its functions are the only ones that read or write both: `normalizeData` passes `normalize`'s changes to `markChanged`, `garbageCollectEnvironment` passes each record `garbageCollect` deleted to `deleteSource`, and `readStoredResult` reads a resolver through `track` with hooks that report each read to the computation's `ComputeContext`. `readSubscription` reads a subscription's nested resolvers through `readStoredResult` (change 2, Subscriptions).
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
- Report every effective change. After an operation that changes what a read would return, the environment calls `markChanged` with every source whose effective value changed, and `deleteSource` for each record whose effective presence went from present to absent (from absent to present is `markChanged` of its `recordSource`). It may report more. It must not report fewer: an unreported change is a stale result with no error. A source reported but unchanged costs a `Dirty` mark on each reader and a recomputation of each reader when it is next pulled, which cuts off by `valuesEqual` or recomputes to an equal result. Settlement runs after the operation and before notification.

For the layered store of Later's optimistic writes:

- A write to the topmost layer that holds a key reports it when `isUnchanged` (change 1) rejects it against the previous effective value. A write to a layer that a higher layer shadows at that key reports nothing.
- Adding a layer reports each key it holds whose value differs from the previous effective value, and each record it creates.
- Removing a layer reports each key the removed layer, or a layer re-executed above it, wrote whose effective value differs after the removal, and `deleteSource` for each record that existed only in the removed layer. This is upstream's `compareData` (`optimisticProxy.ts:506-544`) at store key granularity instead of record granularity, comparing by change 1's AST shape: each layer keeps the `WrittenValue` of every key it wrote, so a list of scalars compares element by element. Without that, `===` reports every list of scalars, which is correct and costs recomputation.

### Lifetime still follows records

A derived node's `owner` is the `recordSource` of the record it was computed on. `deleteSource(graph, recordSource(record))` marks `Dirty` every reader of the record (every reader of any key on it read its presence) and the dependents of every derived node the record owns, and disposes those nodes. A store-coupled engine does the same through its record grouping; here the engine does it without knowing what a record is. A dependent marked `Dirty` recomputes when next pulled (by `settle` when it is observed, otherwise by its next read), finds the record absent, and records only `recordSource`, with no derived node (change 2, the store's side). No pull reaches a disposed derived node: every dependent of it read the owner's `recordSource` before reading it (`contextHooks`), so `markChanged` of that source marked every dependent `Dirty`; a `Dirty` node recomputes without walking its old dependencies, and a recomputation reads the record's presence instead.

components-in-userland.md says a closure field's wrapper is "stored on the record, and freed with it". "Stored on the record" does not hold literally: derived nodes live in `environment.graph`, beside the store. It holds as "keyed by the record": the wrapper is the value of the derived node `resolverDerivedNode(record, storeKey)` in `graph.derivedNodes`. "Freed with the record" holds through `owner`: deleting the record calls `deleteSource` with its `recordSource`, which disposes the wrapper's derived node. The wrapper's computation reads only the record's presence (its `createWrapper` receives no data), so no write recomputes it while the record lives.

### Costs of the ids

- An id per read. Each server read in a computation builds two strings (`recordSource`, `valueSource`) and each resolver read one (`resolverDerivedNode`; `resolverStoreKey` is built in any design), and `setDependencies` does one `Map` lookup per id. A store-coupled engine does the same reads as three nested `Map` lookups with no string built. The order of growth is the same; the string building is the extra, and it is not measured. An interned object per (entity, id, store key) avoids the string but needs an intern table, whose members need the same lifetime rule as `sourceDependents` (deleted when no node reads them), so it is `sourceDependents` again under a second name; the string is held.
- A presence edge per (derived node, record read): one more `sourceDependents` set member than a store-coupled engine keeps for a node that read a present record.
- A map lookup instead of a field: a derived node does not sit in a per-record map, so deleting a record finds its nodes through `owned`, one more map kept by node creation and disposal.
- The casts in the id functions.

What the opaque ids buy: the engine has no import from the store, so a layered store, another record layout, or a source that is not server data (a client-only value, a clock) is a `SourceId` and two calls. R8 becomes the environment's obligation above, stated once, instead of a property of `normalize`. The engine is testable with literal ids.

## Values with lifecycles

Status: design analysis, held options, not decided. Nothing in changes 1 to 4 has an effect; this lands with the first feature that does (Follow-ups 1, 2, 4), and changes 1 to 4 must not preclude it.

A derived node's computation is pure. It reads through its `ComputeContext`, returns a value, and acquires nothing. It runs whenever a pull reaches a marked or absent derived node: inside a write's `settle`, inside `listen`, inside a `readStoredResult` that a React render makes and React then discards, and, once forks exist, in a world that is never committed (Follow-ups 12). A computation that subscribed, fetched, or retained would acquire a resource in each of those pulls, and nothing would release it when the render or the world is discarded.

A resource that a derived node's value needs is therefore the node's effect, run apart from its computation:

- An effect is set up when its derived node is mounted, at React's commit: `observe`, which `listen` calls from `useSyncExternalStore`'s subscribe, sets it up with the node's value after bringing the node up to date.
- It is torn down when the node is unmounted (the last callback at its id is removed by the function `observe` returns), when the node is disposed, or when a write gives the node a value `valuesEqual` rejects; in the last case the effect is set up again with the new value first.
- It never runs inside a computation, and it runs only in the committed world (Follow-ups 12).

Examples: a client pointer's follow-up query (Follow-ups 2), a handle a retain returns (Follow-ups 4), a client root's subscription to an external value (Other derived node kinds), a closure component's subscription.

### The hook

A derived node's effect comes with its definition. The engine cannot tell from a returned value whether it holds a resource, so the kind comes from compiler-written data (the artifact's kind), not from the value.

```ts
// from libs/isograph-runtime/src/graph.ts
/** What creating an absent derived node needs. */
export type DerivedNodeDefinition<TValue> = {
  readonly owner: SourceId;
  readonly computation: Computation<TValue>;
  /** Set up while the node is mounted. */
  readonly effect: NodeEffect<TValue>;
};

/** `None`: the node's value needs no resource. `Effect`: `setUp` acquires what `value` needs and
 * returns its teardown. */
export type NodeEffect<TValue> =
  | { readonly kind: 'None' }
  | { readonly kind: 'Effect'; readonly setUp: (value: TValue) => CleanupFn };

/** The observers of one derived node. */
export type Observed<TValue> = {
  readonly callbacks: Set<() => void>;
  /** The value the callbacks were last called for, or were registered at. */
  notified: TValue | undefined;
  /** The effect of the derived node at this id, while it is mounted. */
  effect: ObservedEffect<TValue>;
};

/** `NotSetUp`: the node has no effect, no node is stored at the id, or `setUp` threw.
 * `SetUp`: `setUp` ran with `value`; `tearDown` releases it. */
export type ObservedEffect<TValue> =
  | { readonly kind: 'NotSetUp' }
  | { readonly kind: 'SetUp'; readonly value: TValue; readonly tearDown: CleanupFn };
```

These replace change 2's `DerivedNodeDefinition` and change 4's `Observed`. `DerivedNode` gains `readonly effect: NodeEffect<TValue>`, copied from its definition by `createDerivedNode`. `RecomputationGraph` gains `readonly pendingTeardowns: CleanupFn[]`, the teardowns of replaced and disposed effects not yet run, and:

```ts
// from libs/isograph-runtime/src/graph.ts
/** For each mounted derived node with an effect whose value is not `===` the value its effect was
 * set up with: sets up the effect with the new value, then pushes the old teardown to
 * `pendingTeardowns`. Each `setUp` runs through `reportCallbackError`. Never throws. */
export function reconcileEffects<TValue>(graph: RecomputationGraph<TValue>): void;

/** Runs and empties `pendingTeardowns` in order, each teardown through `reportCallbackError`. Never throws. */
export function flushTeardowns<TValue>(graph: RecomputationGraph<TValue>): void;
```

For the runtime, a resolver whose artifact has an effect reads its data and returns its value as any resolver does; the artifact's effect receives the stored `ResolverResult` and sets up only on `Ok`.

Contract on `setUp`: a `setUp` that can return a `===` resource on two calls (a handle deduplicated by key) reference-counts it, so each teardown releases only its own retain. Follow-ups 5's registry is that count; each mounted derived node holds one retain.

### Reconciliation

The runtime pulls the mounted derived nodes with an effect after every write, whether or not open decision 28 pulls observed derived nodes: an effect whose inputs changed must be replaced even when no reader reads the node. `settle` pulls every derived node whose `Observed` has a callback and whose `effect` is not `None`; under open decision 28 (a) that is a subset of the observed derived nodes it already pulls.

`normalizeData` runs `settle`, the new root's `retainRoot`, `notify`, `reconcileEffects`, then `flushTeardowns`; `garbageCollectEnvironment` runs `settle`, `notify`, `reconcileEffects`, and `flushTeardowns` at its end. `observe` sets up the effect of the node it mounts after bringing it up to date, and its stop function, when it removes the last callback, pushes the teardown to `pendingTeardowns` and flushes. `disposeDerivedNode` of a mounted node (`deleteSource`; `collect` keeps every node with `rootCount > 0`) pushes its teardown.

Ordering guarantees:

- No effect runs inside a computation, so a pull from a discarded render or an uncommitted world acquires nothing.
- A new effect is set up before the old one is torn down. A shared resource reached again (the same follow-up for the same target) gets a second retain, not a teardown and a rebuild.
- An effect whose node's new value is `===` the old one is left as is: R4's cutoff works for effects.
- A replaced or disposed effect is torn down after every observer of that settlement was notified, so a `snapshot` in a callback never returns a value whose resource was released, and each teardown runs exactly once, in the order the effects were replaced or disposed.
- Under React, teardown after notification is not late enough: a concurrent render can still be on screen showing the old value until the commit of the re-render. The engine's retain is one retain among others. The React package takes its own retain on the handle it renders: a temporary retain at render, converted to a permanent retain at commit, and released when the component unmounts or commits a newer value (network-and-subscriptions.md, the two-phase lifetime; `CacheItem.temporaryRetain`, `CacheItem.ts:195-256`, and `permanentRetainIfNotDisposed`, `:114-193`). The resource is destroyed when the engine's teardown and React's release both happened, so a committed tree never holds a destroyed resource.

A retained query nobody renders mounts nothing, so its derived nodes set up no effect: a retained, unrendered pointer starts no follow-up query. That is the difference from Follow-ups 1, whose constructor runs at normalization time (open decision 13).

### Failures

A `setUp` that throws. Options:

- (a) `reconcileEffects` and `observe` run `setUp` through `reportCallbackError`; the effect becomes `NotSetUp`, the old effect is torn down as for any replacement, and the next value the node takes sets up again. Held. Reason: open decision 9's rule, and a failed effect does not change the node's value, which the computation alone determines.
- (b) Keep the old effect. Cost: a resource set up for a value the node no longer has.
- (c) Propagate out of `normalizeData`. Cost: the remaining effects are not reconciled and the write's caller sees an effect's failure.

A teardown that throws. Options:

- (a) `flushTeardowns` runs each teardown through `reportCallbackError`, open decision 9's rule: a throw is rethrown in its own microtask, and the remaining teardowns run. `flushTeardowns`, `normalizeData`, and `garbageCollectEnvironment` do not throw on a teardown's error. Held.
- (b) Stop at the first throw. Cost: later resources leak.

A teardown is never called twice by the engine; a handle's own second-call throw (`CacheItem.ts:125-130`) is not reachable from it.

### Relation to Follow-ups 1 to 7

- 1, constructors and destructors: Follow-ups 1 runs the constructor at normalization time and ties lifetime to retain and release, not render. The effect split runs it at mount commit and tears it down at unmount, which is render lifetime. The two conflict; open decision 13 records it.
- 2, client pointer and follow-up query: the pointer's computation returns its target; the follow-up query of that target is its effect. A write that moves the target sets up the new follow-up and then tears down the old one after notification.
- 3, a pointer whose target it read: no effect; it removes the follow-up instead.
- 4, handles: the derived node's value is the handle the effect retains; `snapshot` returns it, a lookup, readable during render.
- 5, deduplication: under R6, two components that mount the same pointer on the same record share one derived node and one `Observed`, so one effect. Sharing across different derived nodes (two pointers that return the same target) needs a key of the follow-up itself: the target link, the target fragment's selectable, and its variables.
- 6, a destructor and an in-flight follow-up: the teardown aborts the follow-up's request through the `AbortSignal` the network function accepts.
- 7, deferral: `flushTeardowns` is the batch. It can be handed to the caller-supplied scheduler instead of run at the end of `normalizeData` and `garbageCollectEnvironment`, and a handle released and retained again before the batch runs is not destroyed, because the second retain reached it first.

## Steps

Depends on the held architecture (3b, mark, then pull) in changes 2 to 4.

Each change is one commit, in order; change 2 is two. Step A has landed and is independent of 1 to 4.

- Step A (landed). Re-read every subscription after each write, upstream's mechanism, with `readButDoNotEvaluate`. No memoization.
- 1. `normalize` skips an unchanged value and returns the changes it made (prefactor). `Store` stays store.md's nested map.
- 2a. `graph.ts`, the engine, over opaque `SourceId`s and `DerivedNodeId`s, importing nothing from the store, the reader, or the environment: derived nodes and their state record (`stateOf`), `setDependencies`, `markChanged` marking `Dirty` and `Check`, the pull (`bringUpToDate`, with the cutoff of open decision 15 (b)), `track`, `deleteSource`. Its tests use literal ids.
- 2b. Stored resolver results. The environment gains `graph`. The reader's walk takes `ReadHooks`, and `readStoredResult` reads a resolver through the graph, storing each resolver's result with its dependencies and their inverse. `normalizeData` marks `Dirty` every derived node that read what `normalize` changed and `Check` every node that reads those, transitively, stopping at a node already marked; `garbageCollectEnvironment` does the same for each record `garbageCollect` deleted and disposes the derived nodes the record owned. A read pulls a marked derived node: a `Check` node is verified through its dependencies and recomputes only when one of them changed value. There are no observers, so nothing is pulled after a write. `readSubscription` reads each subscription with `storedResultHooks`, so the subscription re-read that `callSubscriptions` runs after every write pulls each nested resolver through the graph: the subscription's own reader AST is walked on every write, as in Step A, and a nested resolver whose inputs did not change is not called and returns its stored result.
- 3. Level 1 retention. `normalizeData` takes the entrypoint, computes its root result and every result that result reaches by a read of the root, holds the root in the graph, and adds its `Retention` to `environment.retainedQueries`. `garbageCollectEnvironment` disposes the derived nodes that no held root and no current subscription's last read reaches; each `Subscription` records the derived nodes its last read read directly. A later write marks a retained query's affected derived nodes.
- 4. Level 2 and the post-write pull. The graph gains the observers. `listen` brings its derived node up to date and holds it as a root; `snapshot` is a lookup. After a write's marking, `settle` pulls every observed derived node, and observers are notified once, after the pull (open decision 28, held (a)). `garbageCollectEnvironment` settles and notifies after its deletions.

Values with lifecycles (open decision 13) adds no change here: it is a later change, with the first feature that produces a value with an effect, and adds `NodeEffect`, `Observed.effect`, `pendingTeardowns`, `reconcileEffects`, and `flushTeardowns` to the engine.

## Step A: re-read every subscription after each write

Landed (open decision 10, taken as (a)). Step A is network-and-subscriptions.md's Subscriptions (`refactors/pending/runtime/network-and-subscriptions.md:16`, `:1597`), in `libs/isograph-runtime/src/subscribe.ts`. It is close to upstream and is not the final design: it is read-plus-subscribe, with the read on every write.

```ts
// from libs/isograph-runtime/src/subscribe.ts
/** What a subscription delivers: the reader's data, `MissingData`, or what the read threw. */
export type SubscriptionResult =
  | ReadOk<SelectionSetData>
  | ReadMissingData
  | ReadErr;

/** A read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: 'Err';
  readonly error: unknown;
};

export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
  /** Called after each change to `delivered`, after `callback`. */
  readonly listeners: Set<() => void>;
};

/**
 * Reads `readerArtifact` at `link` now and subscribes: after every write, re-reads and calls
 * `callback` when the result changed. Returns the subscription, whose `subscriptionSnapshot`
 * is the read, and the cleanup that unsubscribes.
 */
export function readAndSubscribe<TEntityName extends EntityName>(
  environment: IsographEnvironment,
  readerArtifact: ReaderArtifact<
    ResolverParam<TEntityName>,
    unknown,
    TEntityName
  >,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
  callback: (result: SubscriptionResult) => void,
): ItemCleanupPair<Subscription>;

/** The subscription's last result: the same object until a write changes the read. */
export function subscriptionSnapshot(
  subscription: Subscription,
): SubscriptionResult;

/** Calls `listener` after each change to `subscriptionSnapshot(subscription)`, until the
 * returned function is called. */
export function listenToSubscription(
  subscription: Subscription,
  listener: () => void,
): () => void;

/**
 * Re-reads every subscription in `environment.subscriptions`, in insertion order, and calls
 * each one's callback when its result changed. Never throws.
 */
export function callSubscriptions(environment: IsographEnvironment): void;

/** Calls `f`; a throw is rethrown in its own microtask, so it reaches the host's uncaught-error
 * handler and does not stop the caller. */
export function reportCallbackError(f: () => void): void;
```

- The subscriptions live on the environment (`environment.subscriptions`, `libs/isograph-runtime/src/environment.ts`), as upstream's do (`IsographEnvironment.ts:80`). `makeNetworkRequest` calls `callSubscriptions(environment)` after `normalize` (`libs/isograph-runtime/src/network.ts:143-149`), as upstream's write sites do (`makeNetworkRequest.ts:271`, `:542`, `writeData.ts:55`).
- Each read is `readButDoNotEvaluate` (`readSubscription`, `subscribe.ts:120-131`): the reader's data with every nested resolver called, and the subscription's own resolver not called. A throw from the read becomes `ReadErr`. The callback receives the `SubscriptionResult`, not the resolver's value; react.md's hook calls the resolver in render (react.md, Reading, `useResult`).
- `callSubscriptions` compares each re-read with `delivered` (`subscriptionResultsAreEqual`, `subscribe.ts:135-155`): `Ok` results by `selectionSetDataAreEqual` (`areEqualWithDeepComparison.ts`, upstream's `mergeObjectsUsingReaderAst` rule without the recycling: store data along the reader AST, scalars by `===`, lists item by item, linked selections recursively, nested `Resolver` values by `===`), two `MissingData` as equal, two `Err` by `===` of their errors. When unequal it sets `delivered`, then calls the callback and each listener through `reportCallbackError`.
- No overlap filter: store.md's `normalize` records no `EncounteredIds`, so every subscription is re-read, where upstream skips the ones whose records do not overlap (`subscribe.ts:74-79`).
- Order does not matter for correctness: every re-read runs after the write finished and reads the whole store, so nothing reads a stale input. Nothing is shared: a resolver two subscriptions reach runs once per subscription per write.

Costs: every write re-reads every subscription and re-runs every nested resolver under it; a nested resolver that returns a new object makes its parent's data unequal on every write (upstream has the same, `areEqualWithDeepComparison.ts:84-95`), so its callback fires on every write. With the callback being React's `setState`, a demo renders correctly and slowly. Change 2b keeps the mechanism and removes the nested cost (change 2, Subscriptions): `makeNetworkRequest` calls `callSubscriptions` after `normalizeData`, every subscription is still re-read on every write, and its reader AST is still walked, but `readSubscription` reads each nested resolver through the graph, so a nested resolver whose inputs did not change is not called and its value is `===` the one delivered before. Changes 3 and 4 keep that.

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
// PetName on Pet:1, then PetSummary, ran once each inside normalizeData; RecentCheckins did not.
// onChange was called once, after both ran; snapshot returns the new result.

stop();
normalizeData(environment, petDetailEntrypoint, petDetailWithNewName, { id: '1', skip: null });
// Nobody observes PetSummary: the write marked PetName on Pet:1 and PetSummary and ran neither.
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

A `PartialWriteError`'s message is the message of the error it wraps, so store.md's `toThrow("Malformed response at ..")` assertions hold unchanged. Its reader is `normalizeData`, which marks the changes it carries before rethrowing, and from change 4 settles and notifies first (open decision 17).

### `normalize` skips an unchanged value

Before (store.md, `normalize`): every selected value is written with `record.set(key, value)`, and there is no change detection.

After, every write goes through `writeValue`. `normalize` already knows each value's shape from the AST: a `NormalizationSelection` whose `target` is `null` writes a `LeafValue`, and one with a composite target writes a `CompositeValue`. It passes that shape, and the comparison follows it, as upstream's does (`normalizeScalarField` and `normalizeLinkedField`, `cache.ts:232-341`). There is no generic deep equality.

```ts
// from libs/isograph-runtime/src/normalize.ts
/** A value `normalize` writes, with the shape its selection gives it. */
type WrittenValue =
  | { readonly kind: 'Leaf'; readonly value: LeafValue }
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

/** `Leaf`: `leafUnchanged(stored, written.value)`.
 * `Composite`: `compositeUnchanged(stored, written.value)`. */
function isUnchanged(stored: StoreValue, written: WrittenValue): boolean;

/** `written` a list: `stored` is a list of the same length whose items are `leafUnchanged` in
 * order, at any depth. Otherwise `stored === written`. */
function leafUnchanged(stored: unknown, written: LeafValue): boolean {
  if (!isArray(written)) {
    return stored === written;
  }
  return (
    isArray(stored) &&
    stored.length === written.length &&
    written.every((item, index) => leafUnchanged(stored[index], item))
  );
}

/** `null`: `stored` is `null`. A link: `stored` is a link with the same `entityName` and `id`.
 * A list: `stored` is a list of the same length whose items are `compositeUnchanged` in order. */
function compositeUnchanged(stored: StoreValue, written: CompositeValue): boolean;
```

- Absent key, or a value `isUnchanged` rejects: `record.set(key, written.value)`, then `normalization.report({ kind: 'ValueChanged', record: owner, storeKey: key })`.
- An unchanged value: nothing is written or reported; the stored value's identity stays.

`writeSelection` builds the `WrittenValue` from the selection: `Leaf` when `selection.target === null`, and `Composite` otherwise. The normalization AST and its printing are unchanged.

`compositeUnchanged` reads `stored` as `StoreValue`, because `record.get` returns that type; it narrows by `isArray` and by `typeof stored === 'object'`, with no cast. `leafUnchanged` takes `stored: unknown`, because util's `isArray` narrows to `readonly unknown[]`, so a stored list's items are `unknown`; it compares them only with `===` and `isArray`, so it needs no cast either. `isArray(written)` narrows `LeafValue` to its list member, whose items are `LeafValue`, so the recursive call type-checks. Each comparison costs one `===` for a scalar leaf, one `===` per element for a leaf list, and one step per link for a composite, one comparison per written field.

The comparison follows the value, not the selectable's type. Two lists compare element by element, recursively, whatever the schema says the field's type is; anything else compares with `===`. It never reports a changed value as unchanged: `leafUnchanged` answers `true` only when every compared position is `===`, and a new object is never `===` an old one. A leaf whose value is an object (a `JSON` custom scalar such as `settings` holding an object) is a new object on every response, so it counts as changed on every rewrite; a list item that is an object (`[JSON]`) is the same case. That worst case is one `ValueChanged` too many, which invalidates the derived nodes that read the value and recomputes them to an equal result. A `JSON` value holding a list of scalars compares element by element, as a `[String]` does.

Its reader is the returned list: an unchanged write reports nothing, so change 2 marks nothing for it. The check exists because without it every response marks every written field changed, so every refetch of a retained query recomputes every derived node under it, and the whole retained tree recomputes on every response. Whether the comparison earns its cost against that recomputation is not measured.

A list of scalars (`tags: [String]`) is a new array on every response. Compared with `===`, it would count as changed on every rewrite, and every derived node that reads it would recompute on every response. Lists of scalars are common in GraphQL schemas, so `leafUnchanged` compares two lists element by element, at any depth.

Deltas from upstream: upstream compares every scalar field with `!==` only (`cache.ts:248-249`), so a list of scalars counts as changed on every response; here a leaf list is compared by length and then per element, recursing into each item that is a list. Upstream counts a `null` write as unchanged when the stored value is `null` (`cache.ts:243-246`, `:270-273`); `===` on a scalar leaf covers both that and the `!==` comparison. Upstream compares a single link by `__link` and `__typename` (`cache.ts:337-338`) and a list of links by length and then item by item (`dataIdsAreTheSame`, `cache.ts:372-393`). Here a link is compared by `entityName` and `id`, the fields of `StoreLink`, and lists nest at any depth (store.md's `CompositeValue`), so `compositeUnchanged` recurses where upstream's list check is one level. Upstream returns `RecordHasBeenUpdated` up the walk; here `writeValue` acts on the result at the write. The only structural comparison in changes 1 to 4 besides `compositeUnchanged` is `resultsEqual`'s comparison of `MissingData`, which the runtime builds from strings and links (R4).

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
- `settings: ["a"]`, a `JSON` custom scalar holding an array, written twice keeps the first array (`toBe`) and returns no `ValueChanged` for `settings`: two lists compare element by element whatever the field's type. A `settings` object `{ "a": 1 }` written twice stores the new object (`not.toBe` the first) and returns `ValueChanged` for `settings`: an object compares by `===`.
- Normalizing the same `PetDetail` response twice leaves every link and list of links `toBe` the one after the first, and every string, number, boolean, and `null` leaf `toBe` it.
- A response whose second pet lacks `name`, after the first pet was written: `normalize` throws a `PartialWriteError` whose message is v1's "Malformed response at .." message and whose `changes` are the writes made before the throw; the store holds those writes.
- A response that lacks a root key: `PartialWriteError` whose `changes` hold only `RecordCreated` for `Query.__ROOT`.

Fixture additions to store.md's fixture project: `tagGrid: [[String]]` on `Query` in `fixtures/store/schema/schema.graphql`, and `fixtures/store/src/TagGrid.ts` with `field Query.TagGrid { tagGrid }` and `entrypoint Query.TagGrid`, whose resolver returns `data`.

## 2. Stored resolver results

Depends on the held architecture throughout, except The resolver store key and Results, equality, and a resolver that throws.

Two commits: 2a, the engine (`graph.ts`, The engine); 2b, the store's side and the environment (The store's side, The environment).

A resolver's result is stored in `environment.graph` as a derived node, under an id built from the record it was read on and the client field's resolver store key. Reading a resolver whose inputs did not change returns the stored result without calling the resolver. `normalizeData` and `garbageCollectEnvironment` mark `Dirty` the derived nodes that read what `normalize` and `garbageCollect` changed, and `Check` the nodes that read those; the next read of a marked derived node pulls it. This change has no observers, so nothing is pulled after a write; change 4 pulls the observed derived nodes. The production reader of stored results in this change is the subscription re-read: `readSubscription` reads each nested resolver of a subscription's reader AST through the graph (Subscriptions).

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

The variables are the field's own: for a `Resolver` node, `childVariables(variables, selection.arguments)` (reader-ast.md, The read function), which holds the arguments the selection passes and the compiler-written defaults, and omits an unpassed variable with no default; for a top-level `read`, the read's variables. `PetName` on `Pet:1` is `PetName`; `RecentCheckins` selected with no argument is `RecentCheckins____n___2`; `PetSummary` read with `{ id: "1" }` is `PetSummary____id___"1"` on `Query.__ROOT`. Two selections of one client field with equal argument values on one record are one key, whatever their aliases. The format can make two keys equal: a selectable named `F____n___2` read with no variables and `F` read with `{ n: 2 }` are both `F____n___2`, the same property the server `storeKey` has (`libs/isograph-runtime/src/storeKey.ts:13-34`).

The key needs the client field's selectable name, which v1's reader artifact does not carry (reader-ast.md drops upstream's `fieldName` because nothing read it, `reader.ts:26-37`). It does now:

Before:

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  readonly readerAst: ReaderSelectionSet;
  /**
   * A method signature, so TypeScript compares its parameter bivariantly: an artifact
   * declared with one field's param type is also a
   * `ReaderArtifact<ResolverParam, TOutput, EntityName>`.
   */
  resolver(param: TParam): TOutput;
};
```

After:

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  /** The client field's selectable name: the prefix of its resolver store key. */
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  /**
   * A method signature, so TypeScript compares its parameter bivariantly: an artifact
   * declared with one field's param type is also a
   * `ReaderArtifact<ResolverParam, TOutput, EntityName>`.
   */
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
  /** The value, mark, and dependencies of the last computation. Read and written only through `stateOf`. */
  readonly state: DerivedNodeState<TValue>;
  /** Every derived node whose dependencies hold a `Derived` dependency on this one. Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

/** What the derived node's last computation established, and whether it is still current. */
export type DerivedNodeState<TValue> = {
  value: TValue;
  mark: DerivedNodeMark;
  /** What the last computation read, in read order. Written only by `setDependencies`. */
  dependencies: ReadonlyArray<GraphDependency<TValue>>;
};

/**
 * `Clean`: `value` is what a computation against the store would return.
 * `Check`: a derived node it reads, directly or transitively, is `Dirty`. A pull brings its `Derived`
 * dependencies up to date in recorded order and recomputes it only when that marks it `Dirty`.
 * `Dirty`: a source it read changed, or a derived node it read recomputed to a value `valuesEqual`
 * rejects. A pull recomputes it.
 */
export type DerivedNodeMark = 'Clean' | 'Check' | 'Dirty';

export type GraphDependency<TValue> = SourceDependency | DerivedNodeDependency<TValue>;
export type SourceDependency = { readonly kind: 'Source'; readonly source: SourceId };
export type DerivedNodeDependency<TValue> = { readonly kind: 'Derived'; readonly node: DerivedNode<TValue> };

/** A derived node's computation. It reports every source it reads, and reads other derived nodes, through `context`. */
export type Computation<TValue> = (context: ComputeContext<TValue>) => TValue;

export type ComputeContext<TValue> = {
  /** Records that the computation read `source`. */
  readSource(source: SourceId): void;
  /** Pulls the derived node `id`, creating it from `create()` when absent, records the dependency,
   * and returns its value. */
  readDerivedNode(id: DerivedNodeId, create: () => DerivedNodeDefinition<TValue>): TValue;
};

/** What creating an absent derived node needs. */
export type DerivedNodeDefinition<TValue> = {
  readonly owner: SourceId;
  readonly computation: Computation<TValue>;
};
```

The context is a parameter, built per computation by `compute`, as the walk's `ReadHooks` is; there is no ambient "current computation" (MobX's `globalState.trackingDerivation`, Reactively's `CurrentReaction`). The engine knows nothing about what a derived node computes: the caller supplies the computation, the owner, and the id. Resolver derived nodes are the one derived node kind changes 2 to 4 create; Other derived node kinds lists the kinds the follow-ups add and the parts of the API they need.

`Computation`, `ComputeContext`, and `DerivedNodeDefinition` are internal to the runtime. User code never receives a `ComputeContext`, an environment, or a store. A computation records a dependency only by calling `readSource` or `readDerivedNode`, so a computation that user code wrote as a closure could read a value without calling `readSource`; its derived node would not be marked when that value changes, and it would return its old result with no error. User code reads only through isograph syntax: a selection in a reader AST, which the reader walk reads, and inside a computation each `ReadHooks` hook the walk calls reports the source it reads (`contextHooks`). The walk is therefore the only reader of a source on behalf of user code, and no read inside a computation goes unrecorded. A subscription's re-read is not a computation and records no dependency (Subscriptions); it is re-read after every write. From change 3 it records the ids of the nested derived nodes it read, which `collect` keeps (change 3, Garbage collection).

A source other than a store field is exposed to user code the same way, as a selection in a reader AST, so the walk records its read as it records a server field read. The first such source is the state of a network request. Adding a source kind adds:

- isograph syntax for the selection;
- a reader AST node kind for it;
- a `ReadHooks` hook the walk calls at that node, which `contextHooks` implements by calling `context.readSource` with the kind's `SourceId` constructor in `sources.ts`, as `readValue` calls it with `valueSource` (for a request, `requestSource(key: RequestKey): SourceId`);
- in every writer of that source, the source among the sources it passes to `markChanged`, as `normalizeData` passes `changeSource` of each `StoreChange`, and `deleteSource` of the source in a writer that deletes what the source names, as `garbageCollectEnvironment` calls it with `recordSource` of each deleted record (for a request entry, network-and-subscriptions.md, Request entries in the store).

The syntax and the reader AST node's shape are not decided. Which request a request-state selection refers to is open decision 20.

Derived node kinds the runtime itself defines, such as the suspension decision (Other derived node kinds), are runtime code, and their computations call `ComputeContext` directly.

A derived node is mutated in place (its state record), because other nodes' `dependents` sets and `DerivedNodeDependency`s hold it by identity. A source dependency is a `SourceId`, because a server value has no object to point at and a record may not exist yet. A derived node dependency is the derived node, because `dependents` holds nodes and a `Check` node's pull walks from a node to the nodes it reads. `graph.ts` imports `getOrInsert` from `util.ts` and nothing else.

`stateOf` is the one accessor of a derived node's value, mark, and dependencies: every read and write of them in the engine goes through it, and nothing else touches `node.state`. They are what one computation of the node established, and Follow-ups 12 layers exactly that record per world. `id`, `owner`, `computation`, and `dependents` are not per computation and stay on the node.

```ts
// from libs/isograph-runtime/src/graph.ts
export function createGraph<TValue>(
  valuesEqual: (a: TValue, b: TValue) => boolean,
): RecomputationGraph<TValue> {
  return { derivedNodes: new Map(), sourceDependents: new Map(), owned: new Map(), valuesEqual };
}

/** The value of the derived node `id`, created from `create()` when absent and pulled.
 * Records no dependency: the caller is not a derived node. */
export function track<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  create: () => DerivedNodeDefinition<TValue>,
): TValue {
  return stateOf(upToDateDerivedNode(graph, id, create)).value;
}

/** Marks `Dirty` every derived node that read one of `sources`, and `Check` every `Clean` derived node
 * that reads one of those, transitively. Runs no computation. */
export function markChanged<TValue>(
  graph: RecomputationGraph<TValue>,
  sources: ReadonlyArray<SourceId>,
): void {
  sources.forEach((source) => graph.sourceDependents.get(source)?.forEach(markDirty));
}

/** `markChanged(graph, [source])`, then marks `Dirty` the dependents of every derived node `source`
 * owns, and disposes those derived nodes. */
export function deleteSource<TValue>(
  graph: RecomputationGraph<TValue>,
  source: SourceId,
): void {
  markChanged(graph, [source]);
  const owned = graph.owned.get(source);
  owned?.forEach((node) => node.dependents.forEach(markDirty));
  owned?.forEach((node) => disposeDerivedNode(graph, node));
}
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** The state record of `node`. */
function stateOf<TValue>(node: DerivedNode<TValue>): DerivedNodeState<TValue> {
  return node.state;
}

/** The derived node `id`, created from `create()` when absent, and pulled. */
function upToDateDerivedNode<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  create: () => DerivedNodeDefinition<TValue>,
): DerivedNode<TValue> {
  const node = graph.derivedNodes.get(id) ?? createDerivedNode(graph, id, create());
  bringUpToDate(graph, node);
  return node;
}

/** Marks `node` `Dirty`; when it was `Clean`, marks its dependents `Check` first. */
function markDirty<TValue>(node: DerivedNode<TValue>): void {
  const state = stateOf(node);
  if (state.mark === 'Clean') {
    node.dependents.forEach(markCheck);
  }
  state.mark = 'Dirty';
}

/** Marks a `Clean` derived node `Check`, then its dependents. A marked derived node is left as is. */
function markCheck<TValue>(node: DerivedNode<TValue>): void {
  const state = stateOf(node);
  if (state.mark !== 'Clean') {
    return;
  }
  state.mark = 'Check';
  node.dependents.forEach(markCheck);
}

/** The pull. `Check`: brings the `Derived` dependencies up to date until one marks `node` `Dirty`.
 * `Dirty` then: recomputes. The node ends `Clean`. Does nothing to a `Clean` derived node. A throw
 * out of a computation propagates before the mark is set, so the node keeps its mark, value, and
 * dependencies, and the next pull retries it. */
function bringUpToDate<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
  if (stateOf(node).mark === 'Check') {
    checkDependencies(graph, node);
  }
  if (stateOf(node).mark === 'Dirty') {
    recompute(graph, node);
  }
  stateOf(node).mark = 'Clean';
}

/** Pulls each `Derived` dependency of `node`, in recorded order, until one recomputes to a new value,
 * which marks `node` `Dirty`. */
function checkDependencies<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
  stateOf(node).dependencies.some((dependency) => {
    if (dependency.kind === 'Derived') {
      bringUpToDate(graph, dependency.node);
    }
    return stateOf(node).mark === 'Dirty';
  });
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
    state: { value: computed.value, mark: 'Clean', dependencies: [] },
    dependents: new Set(),
  };
  setDependencies(graph, node, computed.dependencies);
  graph.derivedNodes.set(id, node);
  getOrInsert(graph.owned, node.owner, () => new Set()).add(node);
  return node;
}

/** Recomputes `node`; when `valuesEqual` rejects the new value against the old one, stores it and
 * marks the dependents `Dirty`. */
function recompute<TValue>(graph: RecomputationGraph<TValue>, node: DerivedNode<TValue>): void {
  const computed = compute(graph, node.computation);
  setDependencies(graph, node, computed.dependencies);
  const state = stateOf(node);
  if (!graph.valuesEqual(state.value, computed.value)) {
    state.value = computed.value;
    node.dependents.forEach(markDirty);
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
      return stateOf(node).value;
    },
  });
  return { value, dependencies };
}

/**
 * Replaces `stateOf(node).dependencies` with `next`, and removes `node` from the dependents of each
 * old dependency and adds it to those of each new one. Deletes a `sourceDependents` set left empty.
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

A pull is `bringUpToDate`. A `Clean` derived node returns at once. A `Check` derived node pulls each `Derived` dependency in recorded order and stops at the first after which it is `Dirty`: a dependency that recomputed to a new value marked its dependents `Dirty` (`recompute`). A derived node still `Check` after the walk read nothing that changed, and ends `Clean` without recomputing; that is the cutoff on the pull (open decision 15 (b); 15 (a), recorded as the user's specification, recomputes every marked derived node). A `Dirty` derived node recomputes, and its computation's reads of other derived nodes go through `readDerivedNode`, which pulls each one first, so the computation sees current inputs (R3).

The walk in recorded order is sound when a recomputation would read other dependencies. A computation's later reads depend only on what it read earlier, so up to the first dependency whose value changed, the recorded reads are the reads a recomputation would make, and the walk stops at that dependency. A source dependency needs no walk: a changed source marked its readers `Dirty` directly.

A derived node recomputes at most once per write. It ends `Clean` after its pull, and only marking makes it non-`Clean`. `markChanged` runs in a write, before any pull. `recompute`'s marking of dependents runs during a pull and reaches only non-`Clean` derived nodes, by the invariant below, so it never re-marks a derived node pulled earlier in the same pass. The parent whose read reached the node is computing or checking; marking it `Dirty` makes a checking parent recompute, and a computing parent reads the new value and ends `Clean`. Keeping the old value when `valuesEqual` holds keeps its identity, which the cutoff and change 4's notification read.

A throw during a pull never leaves a derived node `Clean`. `compute` throws before `recompute` calls `setDependencies` and before it compares or stores a value, and `bringUpToDate` sets the mark only after `recompute` returns, so the node keeps its mark (`Dirty`), its previous value, and its previous dependencies and dependents. A checking ancestor whose `checkDependencies` was running keeps its mark too, and a `createDerivedNode` whose computation throws stores nothing. Every later pull of the node recomputes it again, and a later `markChanged` of a source it read leaves it `Dirty`. A resolver's own throw does not propagate: `callResolver` stores it as `ResolverThrew` (open decision 5 (a)), a value, and the node ends `Clean`. What does propagate is a throw of the walk outside the resolver, such as `linkAt` on a non-link (open decision 22).

Invariant, outside a call that is pulling: every dependent of a non-`Clean` derived node is non-`Clean`. Marking keeps it, because a derived node that leaves `Clean` marks its dependents `Check`. A new dependent is added only by a computation, which pulled the derived node first, and a pulled node ends `Clean` only after every node it read was pulled. So every derived node a `Clean` node reads is `Clean`, transitively. `markCheck` stops at a marked derived node because the invariant says its dependents are marked too, so a second write to a source costs one step per node that read it.

The marking is transitive because nothing computes during a write: whether a derived node's value changes is known only by computing it. `Check` is what lets a pull skip the recomputation above an unchanged result.

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
    artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
    root: StoreLink,
    variables: Variables,
  ) => ReadResult<unknown> | TFailure;
};

/** `artifact`'s reader AST read at `root` with `variables` through `hooks`. Nested resolvers are
 * read by `hooks.readResolver`; `artifact`'s own resolver is not called. */
export function readData<TFailure>(
  store: Store,
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
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
/** The stored result of `artifact` on `root` with `variables`: computed when absent, pulled when
 * marked, and otherwise returned as stored. */
export function readStoredResult<TEntityName extends EntityName, TOutput>(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
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
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
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

The cast in `readStoredResult` is the one place a stored `ResolverResult<unknown>` is returned at the caller's type; the invariant is in its comment. Its type parameters and `root` parameter are `read`'s (`libs/isograph-runtime/src/read.ts`), and its artifact passes to `resolverDefinition`'s `ReaderArtifact<ResolverParam, unknown, EntityName>` because `resolver` is a method signature. The computation closes over `store`, the environment's `Map`, which is written in place and never replaced (network-and-subscriptions.md, The environment).

Every reader of any key on a record also reported the record's presence, so a deletion reports one source instead of the record's keys, including keys read while absent, which the store cannot enumerate. A computation that stops at the first `MissingData` records the sources read up to it, which are exactly what its result depends on.

What reads a stored resolver result: `readStoredResult`; `storedResultHooks`, for each nested resolver of a subscription's reader AST (Subscriptions); a parent's `Resolver` node while the parent computes; `normalizeData`'s hold of a new retention's root (change 3); `settle` and `listen` (change 4); `snapshot` (change 4), which pulls nothing. `normalize`, `check`, `garbageCollect`, `read`, and `readButDoNotEvaluate` never read one.

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
/** `normalize` of `response` into `environment.store`, then marks every derived node that read
 * what it changed, and every derived node above those. On a `PartialWriteError`, marks the changes
 * it carries and rethrows it. */
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

### Subscriptions (2b)

Step A's re-read reads every nested resolver with `readButDoNotEvaluate`, which calls each one through `read` on every write. From 2b the re-read reads nested resolvers through the graph. The subscription is not a derived node: its read records no dependency, and `callSubscriptions` still re-reads every subscription after every write and walks its whole reader AST.

```ts
// from libs/isograph-runtime/src/subscribe.ts
// readSubscription, before (Step A):
function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionResult {
  try {
    return readButDoNotEvaluate(environment.store, artifact, root, variables);
  } catch (error: unknown) {
    return { kind: 'Err', error };
  }
}
// after:
function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionResult {
  try {
    return readData(environment.store, storedResultHooks(environment), artifact, root, variables);
  } catch (error: unknown) {
    return { kind: 'Err', error };
  }
}
```

```ts
// from libs/isograph-runtime/src/storedResults.ts
/** Hooks for a read that is not a derived node: nothing is recorded, and each nested resolver is
 * read by `readStoredResult`. */
export function storedResultHooks(environment: IsographEnvironment): ReadHooks<ReadResolverThrew> {
  return {
    readRecord: () => {},
    readValue: () => {},
    readResolver: (artifact, root, variables) => readStoredResult(environment, artifact, root, variables),
  };
}
```

`readStoredResult` receives `artifact` as `ReaderArtifact<ResolverParam, unknown, EntityName>` and `root` as `StoreLink`, so its `TEntityName` is `EntityName`, its `TOutput` is `unknown`, and it returns `ResolverResult<unknown>`, which is `ReadResult<unknown> | ReadResolverThrew`, the return type `ReadHooks<ReadResolverThrew>` gives `readResolver`. `readRecord` and `readValue` do nothing because no derived node is computing: a `readStoredResult` from outside a computation records no dependency (`track`), and the subscription's own reader is re-read on every write regardless.

A nested resolver's throw is a stored `ResolverThrew` from 2b, and `allOk` passes it up the walk unchanged, so the subscription delivers it:

```ts
// from libs/isograph-runtime/src/subscribe.ts
// before (Step A):
/** What a subscription delivers: the reader's data, `MissingData`, or what the read threw. */
export type SubscriptionResult =
  | ReadOk<SelectionSetData>
  | ReadMissingData
  | ReadErr;

/** A read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: 'Err';
  readonly error: unknown;
};
// after:
/** What a subscription delivers: the reader's data, `MissingData`, a nested resolver's throw, or
 * what the read threw. */
export type SubscriptionResult =
  | ReadOk<SelectionSetData>
  | ReadMissingData
  | ReadResolverThrew
  | ReadErr;

/** A read that threw: a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: 'Err';
  readonly error: unknown;
};
```

```ts
// from libs/isograph-runtime/src/subscribe.ts
// subscriptionResultsAreEqual, before (Step A):
    case 'Err':
      return result.kind === 'Err' && delivered.error === result.error;
  }
// after:
    case 'ResolverThrew':
      return result.kind === 'ResolverThrew' && delivered.error === result.error;
    case 'Err':
      return result.kind === 'Err' && delivered.error === result.error;
  }
```

`ResolverThrew` is a variant of its own, and not converted to `Err`, because it names the field and the record whose resolver threw, where `Err` is now only a broken store invariant. Two `ResolverThrew` compare as `resultsEqual` compares them, by `===` of their errors. `subscribe.ts` imports `readData` from `read.ts`, `ReadResolverThrew` as a type from `read.ts`, and `storedResultHooks` from `storedResults.ts`, and no longer imports `readButDoNotEvaluate`.

What this changes for a subscription:

- A nested resolver whose inputs did not change is not called, and its value in the delivered data is the stored result, `===` the one delivered before. `selectionSetDataAreEqual` compares a nested `Resolver` value by `===`, so a nested resolver that returns a new object makes its parent's data unequal only on a write that changed that resolver's inputs. Step A's other costs stay: every subscription is re-read and its reader AST walked on every write.
- A resolver that two subscriptions reach is called once per change to its inputs, not once per subscription per write.
- `readAndSubscribe`'s first read is `readSubscription` too, so subscribing computes and stores the nested results it reaches.
- A nested resolver that throws is delivered as `ResolverThrew` where Step A delivered `Err`. Its stored result is the same object until a write changes the resolver's inputs, so a re-read after an unrelated write is equal and calls no callback, where Step A called it on every write.
- A write must go through `normalizeData`. A direct `normalize` marks nothing, and the next re-read returns the stored nested results from before it (The engine, the store, and the environment, What this costs).
- In 2b the derived nodes a subscription's read stores are disposed only when `garbageCollectEnvironment` deletes their owner record. From change 3, `collect` disposes every derived node no root reaches, and the derived nodes each current subscription's last read read directly are roots: each `Subscription` records their ids in `readDerivedNodes`, filled by `storedResultHooks` (change 3, Garbage collection; open decision 27). A nested derived node a current subscription reads therefore keeps its stored value through `garbageCollectEnvironment`, and the next re-read calls no callback for it.

The other readers of the walk do not go through the graph. `read` and `readButDoNotEvaluate` keep v1's hooks (The store's side), so their exports and reader-ast.md's tests keep v1's behavior. `readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:195-218`) calls `read`, which calls the entrypoint's resolver and every nested resolver again after each response; whether it reads through the graph is open decision 26. components-in-userland.md's closure wrapper calls `readButDoNotEvaluate` when the wrapper is called; that doc is not part of the current work.

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
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ResolverResult<unknown>;

/** `Ok`: values `===`. `MissingData`: equal `reason`, `recordLink` entity and id, and
 * `nestedReason`, recursively. `ResolverThrew`: errors `===`. Different kinds: unequal. */
export function resultsEqual(a: ResolverResult<unknown>, b: ResolverResult<unknown>): boolean;
```

v1's `ReadResult` is unchanged, so `read` and `readButDoNotEvaluate`, which never produce `ResolverThrew`, do not admit it in their types. v1 lets a throw propagate out of `read` (reader-ast.md, Semantics), and still does. In a stored result it is a value, because changes 3 and 4 run resolvers inside `normalizeData` (the new root's, and every observed derived node's): a throw that propagated would abort `normalizeData`, and leave the other observed nodes unsettled. Open decision 5.

Equality is upstream's for resolver values, `===` (`areEqualWithDeepComparison.ts:84-95`), as pico's is the value's `==`. It is R4's optimization, not a guarantee. A resolver that returns a `===` primitive or the same object marks none of its dependents (change 2, The engine); one that returns a new object is unequal on every recomputation, structurally equal or not, and every derived node that reads it recomputes. `MissingData` is compared field by field because the runtime builds it from strings and links, where structural equality is well-defined. Open decision 7 settles `===`.

### Tests

`test/read.test.ts` (reader-ast.md, Tests, Runtime) is unchanged: `read` and `readButDoNotEvaluate` keep v1's behavior, call every resolver on every read, leave the store unchanged, and propagate a resolver's throw.

New, in `test/graph.test.ts` (2a), with literal ids and computations that read sources from a map the test holds and report them through `context.readSource`:

- A derived node read twice with `track` with no `markChanged` between: its computation runs once, and both values are `===`.
- `markChanged` of a source the derived node read: the node's mark is `Dirty` and its dependent's `Check`; the next `track` runs the computation once. Of a source it did not read: none. Of `[]`: none.
- A derived node whose computation reads another node: `markChanged` of the inner node's source recomputes both on the next `track` of the outer; the inner computation runs before the outer's finishes.
- A recomputed value that `valuesEqual` accepts keeps the old value object (`toBe`).
- Cutoff on the pull: an outer node reads an inner node that read `a`; after `markChanged([a])`, with the inner computation returning a value `valuesEqual` accepts, `track` of the outer runs the inner computation once and the outer computation zero times, and both end `Clean`.
- Dependencies follow the last computation: a computation that read `a` and then, after `markChanged([a])`, reads `b` instead is not marked by a later `markChanged([a])`, and `sourceDependents` no longer holds a set for `a`.
- `deleteSource` of a derived node's owner: `derivedNodes` no longer holds the node, `owned` no longer holds the owner, and an outer node that read it is `Dirty`.
- A computation that throws: an outer node reads an inner node that read `a`; after `markChanged([a])`, with the inner computation throwing, `track` of the outer throws that error; the inner node is `Dirty` and the outer `Check`, the inner value is `toBe` its value before the write, and `sourceDependents` still holds the inner node under `a`. A second `track` throws again and runs the inner computation once more. After the inner computation stops throwing, the next `track` runs it once, returns the new value, and both nodes end `Clean`.
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
- Diamond, `Rex` to `Ray`: on the next read, `Initial` is called once and returns `"R"`; `Monogram`, `Signature`, `Badge`, and `BadgePage` are called zero times, and the read's result is `toBe` the previous one (open decision 15 (b)).
- New object, `Pet:1.name` `Rex` to `Ray`, after a read of `ShapePage` with `{ id: "1" }`: the next read calls `NameShape` and `ShapeLabel` once each and `ShapePage` zero times; `NameShape`'s stored result is `toEqual` and not `toBe` the previous one.
- Diamond, only `nickname` changes: `Signature`, `Badge`, `BadgePage` once each; `Initial` and `Monogram` zero.
- Chain across fragments, `Pet:1.bestFriend` is `Pet:2`; `FriendPage` with `{ id: "1" }` and `PetSummary` with `{ id: "2" }` read, then `Pet:2.name` changes, then both read again in either order: `PetName` on `Pet:2` is called once; `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` once each; `RecentCheckins` zero.
- Unrelated write: only `Pet:1.tagline` changes: `FriendCard` and `FriendPage` once; `FriendLabel` and `PetName` zero.
- Link moved: `Pet:1.bestFriend` changes to `Pet:3` (with `name` and `nickname`); the next read of `FriendPage` calls `PetName` on `Pet:3` once; then `Pet:2.name` changes, and the next read of `FriendPage` calls nothing.
- Same resolver selected twice: a first read of `TwoNames` calls `PetName` once, and `a.PetName` and `b.PetName` are `===`.
- An identical response normalized again: no resolver is called on the next read.
- A list of scalars written twice: after a read of `Leaves` with `{ name: "Bo" }`, normalizing `tags: ["a", null]` again with the other values unchanged leaves the `Leaves` call count unchanged on the next read; `tags: ["b", null]` then calls `Leaves` once.
- Nested depth: after a read of `TagGrid`, an equal `tagGrid: [["a"], [], null]` calls nothing on the next read; `[["a"], ["b"], null]` calls `TagGrid` once.
- A `JSON` custom scalar: after a read of `Leaves`, `settings: ["a"]` written twice calls nothing on the next read; a `settings` object `{ "a": 1 }` written twice calls `Leaves` once.
- Missing then present: a read of `FriendPage` with `Pet:2` lacking `nickname` is `MissingData`; after `nickname` is written, the next read is `Ok` and calls `PetName`.
- Absent record: `FriendPage` read while `Pet:1.bestFriend` links a `Pet:9` absent from the store is `MissingData` naming `Pet:9`; after `normalizeData` writes `Pet:9` with `name` and `nickname`, the next read is `Ok`.
- A deleted record: after `garbageCollectEnvironment` deletes `Pet:2`, the next read of a stored `FriendPage` result is `MissingData` with `recordLink` `Pet:2`, not the stored `Ok`, and `environment.graph.derivedNodes` holds no derived node of `Pet:2`.
- A throw outside the resolver: after a read of `FriendPage` with `{ id: "1" }`, the test sets `Pet:1.bestFriend` to the string `"x"` with `environment.store.get('Pet')?.get('1')?.set('bestFriend', 'x')` and calls `markChanged(environment.graph, [valueSource(Pet:1, 'bestFriend')])`. `readStoredResult` of `FriendPage` throws `linkAt`'s "holds a non-link under the composite store key" error, and a second call throws it again and calls `FriendLabel`'s resolver zero times. After `normalizeData` of a `PetDetail` response whose `bestFriend` is `Pet:2`, the next `readStoredResult` of `FriendPage` is `Ok`.
- A resolver that throws: `readStoredResult` returns `ResolverThrew` with the error, the field's selectable name, and the record; its parent's result is the same `ResolverThrew` object. `read` of the same artifact throws, as in v1.
- A partial write: a response that throws part-way after changing `Pet:1.name`: `normalizeData` rethrows the `PartialWriteError`, and the next read of `BadgePage` calls `Initial` once.
- `resolverStoreKey`: `RecentCheckins` with `{ n: 2 }` is `RecentCheckins____n___2`; `{}` is `RecentCheckins`; `{ b: 1, a: "x" }` is `X____a___"x"____b___1`.

`test/subscribe.test.ts` (2b). Its helper `write` calls `normalizeData` where it called `normalize`, and the cases that call `normalize` on `environment.store` directly call `normalizeData`; the cases that set a value with `environment.store.get(..)?.get(..)?.set(..)` stay, because what they set is read by the subscription's own reader AST, which is walked on every re-read. Landed cases whose assertions change:

- "nested resolvers that return new objects make every re-read unequal" becomes "an unrelated write keeps nested resolver values": after `writePet9`, `onResult` is not called, and `subscriptionSnapshot(subscription)` is `toBe` the first snapshot.
- "a nested resolver that throws at subscribe gives an Err snapshot" and "a nested resolver that throws at a re-read delivers Err, and other subscriptions are still called" assert `kind: 'ResolverThrew'`, `selectableName: 'ThrowingName'`, and `recordLink` `Pet:1` with the same error; in the second, the `callSubscriptions` with no write between leaves `onThrowingResult` at one call, where Step A called it again.
- "a read that throws after a response leaves the request Ok" (`through fetchEntrypoint`) asserts `kind` `ResolverThrew`.

New:

- A nested resolver whose inputs did not change: `normalizeData` of `petDetailResponse()` with `PetDetail`'s AST and `{ id: "1", skip: null }`, then `readAndSubscribe` of `PetSummary`'s reader artifact at `Query.__ROOT` with `{ id: "1" }`, with `vi.spyOn` on the `resolver` of `RecentCheckins`'s reader artifact. `normalizeData` of `petDetailResponse({ nickname: "Rexy" })`, then `callSubscriptions`: `onResult` is called once; its `value.pet.PetName` is `toEqual` `{ name: "Rex", nickname: "Rexy" }`; `RecentCheckins`'s resolver was called once in all, at subscribe; and the delivered `value.pet.RecentCheckins` is `toBe` the first snapshot's.

New in `test/garbageCollection.test.ts`: `garbageCollect` with nothing retained returns every record's link, in the store's order; with every record reached, `[]`.

## Ordering

Depends on the held architecture.

A write can mark many stored results at once. R3 holds on every pull:

- The pull (change 2, The engine). A marked derived node is brought up to date only by a pull, and a pull brings every derived node a computation reads up to date before returning its value: a `Check` node's recorded dependencies by `checkDependencies`, and every read by `readDerivedNode`. So no resolver runs with a stale input. A derived node recomputes at most once per write: it ends `Clean`, and during pulls only `recompute`'s marking of dependents runs, which reaches only non-`Clean` derived nodes. The recursion terminates because the runtime graph has no cycle (Cycles, below).
- The post-write pull (change 4, Settlement). `settle` pulls the observed derived nodes in `graph.observers` order. The order changes neither which resolvers run nor how often: the first pull that reaches a marked derived node brings it up to date, and later pulls find it `Clean`.
- No height. A pull of any derived node at any time brings its inputs up to date first, so no read can reach a derived node whose own inputs are pending (change 4, No heights).

Equality cutoff is `recompute`'s and the pull's, by `resultsEqual`. A derived node whose recomputed result `valuesEqual` accepts keeps its old value and marks no dependent, and a `Check` derived node none of whose dependencies recomputed to a new value ends `Clean` without recomputing (open decision 15 (b)). A new object always marks its dependents `Dirty`. Cutoff is an optimization (R4); R3 holds whether or not any result cuts off. The traces are in Candidate 3b.

### The same resolver selected twice

Two selections of one client field on one record with equal argument values are one store key, so one derived node: computed once, read by both parents, each parent recording a dependency on the same derived node and appearing once in its `dependents`. This holds across fragments and across queries. Unequal argument values are two keys and two derived nodes. Aliases do not enter the key.

### Reads during recalculation

- A level 2 callback never runs during recalculation: change 4 calls callbacks only after `settle` pulled every observed derived node, so each is `Clean`, and so is every derived node it reads (change 2's invariant). A callback that calls `snapshot` receives the current result of every observed derived node; one that calls `readStoredResult` receives a current result for any derived node, because the read pulls it. A callback that calls `normalizeData` runs a nested write, settlement, and notification; when it returns, the outer notification loop compares each remaining observer's stored result with the one it last notified, so no observer is called twice for one result or receives an older result after a newer one.
- A `readStoredResult` outside a write is the same pull `settle` makes.
- A resolver that reads or writes the store (through a closure, since its parameter holds no store) during its own computation breaks the guarantee: its inner `readStoredResult` records no dependency on the outer derived node, and an inner `normalizeData` marks derived nodes while an outer recomputation is reading them. Open decision 8.
- Cycles: none under changes 1 to 4. The static reader graph has none (a cycle of client field selections is an `InlineCycle` query error, query-generation.md, Cycles), and every runtime dependency goes to a derived node of a field the reader AST selects (Candidate 3a), so no derived node is reached again while it is pulled. Marking terminates on any graph, because it stops at a derived node already marked. An eager client pointer makes a cycle of derived nodes possible (open decision 18).

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
  readonly readerArtifact: () => ReaderArtifact<ResolverParam, TOutput, EntityName>;
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

`normalize` stays the store's write, with change 1's return value; nothing is renamed. `makeNetworkRequest` passes the entrypoint and holds the returned `Retention` (Retention). `retainQueryWithoutMakingNetworkRequest` serves a query from the store without `normalizeData`, so it holds the root through `retainEntrypoint` (Serving from the store).

### Retention

```ts
// from libs/isograph-runtime/src/retention.ts
/** One `retainEntrypoint` call's hold on its query's records and its entrypoint's resolver result. */
export type Retention<TOutput> = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly artifact: ReaderArtifact<ResolverParam, TOutput, EntityName>;
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
3. `retainEntrypoint`: holds the new retention's root (Computing the new root), then adds the retention to `environment.retainedQueries`, after the root hold returns, so a throw leaves nothing retained that the caller holds no handle to.
4. `notify(environment.graph)` (change 4).

```ts
// from libs/isograph-runtime/src/environment.ts
// normalizeData, before (change 2):
export function normalizeData(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void {
  markChanged(environment.graph, writeOrMark(environment, ast, response, variables).map(changeSource));
}
// after:
export function normalizeData<TOutput>(
  environment: IsographEnvironment,
  entrypoint: Entrypoint<TOutput>,
  response: JsonObject,
  variables: Variables,
): Retention<TOutput> {
  markChanged(
    environment.graph,
    writeOrMark(environment, entrypoint.normalizationAst, response, variables).map(changeSource),
  );
  return retainEntrypoint(environment, entrypoint, variables);
}
```

A derived node is live while a held root or an observed location reaches it through `Derived` dependencies. Several retentions of one entrypoint with equal variables have one `location` and one derived node, with one root hold each; a resolver shared by several queries is one derived node reached from several roots. The reference count of the user's design is implicit: a derived node stays live while any held root or observed location reaches it, and `dispose` of one retention leaves it live while another does. An explicit count would be computed from the root holds, the observers, and `dependents` (change 2); open decision 11.

### Computing the new root

Depends on the held architecture.

```ts
// from libs/isograph-runtime/src/retention.ts
/** Holds the root derived node of `entrypoint` on `variables`, computing it and every result it
 * reaches when absent or marked, and adds the retention to `environment.retainedQueries`. Writes
 * nothing to the store. */
export function retainEntrypoint<TOutput>(
  environment: IsographEnvironment,
  entrypoint: Entrypoint<TOutput>,
  variables: Variables,
): Retention<TOutput> {
  const ast = entrypoint.normalizationAst;
  const artifact = entrypoint.readerArtifact();
  const location: DerivedNodeLocation = {
    record: ast.root,
    storeKey: resolverStoreKey(artifact.selectableName, variables),
  };
  const [, releaseRoot] = retainRoot(
    environment.graph,
    resolverDerivedNode(location.record, location.storeKey),
    () => resolverDefinition(environment.store, artifact, ast.root, variables),
  );
  const retention: Retention<TOutput> = { ast, variables, artifact, location, releaseRoot };
  environment.retainedQueries.add(retention);
  return retention;
}
```

When `normalizeData` calls it, `ast.root` is present: `normalize` inserts it before writing anything. When `retainQueryWithoutMakingNetworkRequest` calls it, `ast.root` can be absent (Serving from the store). The derived nodes the new retention reaches are computed here, children before parents: the root's computation reads each `Resolver` node through `readDerivedNode`, which computes the child, or brings it up to date, first. This is the bottom-up order the user's design creates them in on the way back up the response (Resolver markers, below). A derived node already stored and `Clean` is read as is; one that is marked is pulled. A `normalizeData` of an already retained entrypoint and variables whose response changed nothing calls no resolver.

### Serving from the store

`fetchEntrypoint` serves a query without a request in two cases (`libs/isograph-runtime/src/network.ts:71-105`): `shouldFetch` `No`, and `IfNecessary` when `check` reports `EnoughData`. Both call `retainQueryWithoutMakingNetworkRequest`, which retains the query's AST and makes no `normalizeData` call, so under change 3 as written above it holds no root and computes no result: a query served from the store has no root derived node, `snapshot` at its location returns `undefined`, and a `listen` there only registers, against the Goal. It holds and computes the root as `normalizeData` does, through `retainEntrypoint`:

```ts
// from libs/isograph-runtime/src/network.ts
// before (network-and-subscriptions.md, `fetchEntrypoint`):
function retainQueryWithoutMakingNetworkRequest(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  variables: Variables,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: retainArtifact(environment, ast, variables),
  };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind === 'Undisposed') {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: 'Disposed' };
      }
    },
  ];
}
// after:
function retainQueryWithoutMakingNetworkRequest<TOutput>(
  environment: IsographEnvironment,
  entrypoint: Entrypoint<TOutput>,
  variables: Variables,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  let status: RetentionStatus = {
    kind: 'Undisposed',
    retention: retainEntrypoint(environment, entrypoint, variables),
  };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind === 'Undisposed') {
        dispose(environment, status.retention);
        status = { kind: 'Disposed' };
      }
    },
  ];
}

type RetentionStatus = RetentionStatusUndisposed | NetworkRequestStatusDisposed;

type RetentionStatusUndisposed = {
  readonly kind: 'Undisposed';
  readonly retention: Retention<unknown>;
};
```

```ts
// from libs/isograph-runtime/src/network.ts
// fetchEntrypoint, before:
    case 'No':
      return retainQueryWithoutMakingNetworkRequest(
        environment,
        entrypoint.normalizationAst,
        variables,
      );
    case 'IfNecessary':
      return check(environment.store, entrypoint.normalizationAst, variables)
        .kind === 'EnoughData'
        ? retainQueryWithoutMakingNetworkRequest(
            environment,
            entrypoint.normalizationAst,
            variables,
          )
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
// after:
    case 'No':
      return retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables);
    case 'IfNecessary':
      return check(environment.store, entrypoint.normalizationAst, variables)
        .kind === 'EnoughData'
        ? retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables)
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
```

Deltas: the function takes the entrypoint instead of its AST; its status holds the `Retention` instead of a `RetainedQuery`, and its cleanup calls `dispose`, which removes the retention from `environment.retainedQueries` as `unretainQuery` removed the `RetainedQuery`, and releases the root hold. `retainArtifact` keeps its one other caller, `makeNetworkRequest`. `IsographEntrypoint<TParam, TOutput, TEntityName>` is assignable to `Entrypoint<TOutput>` (`normalizeData`). The retain is still taken when the call is made, so no write between the call and a later read can be missed: a write after `retainEntrypoint` marks or queues the root as it would after `normalizeData`. No `settle` or `notify` runs: the root hold pulls only marked or new derived nodes, and after the last write's `settle` every observed derived node is `Clean` and reads no marked one (change 2's invariant), so no observed value changes.

Under `IfNecessary` the root record is present, because `check` found every selection of the AST, starting at `ast.root`. Under `No` it can be absent. What the root hold does then is open decision 21 (held (a): the root derived node is created and computes `MissingData` for the absent record).

### Unobserved results are lazy

Depends on the held architecture.

After the `normalizeData` that retains a query, a later write marks its affected derived nodes, and only a pull brings them up to date; from change 4 `settle` pulls the observed ones. Until change 4 there are no observers, so the next read of them pulls them: a `readStoredResult`, a subscription's re-read of a nested resolver (change 2, Subscriptions), or the root hold of the next `normalizeData` of the same entrypoint and variables. Retention keeps records and derived nodes alive for `garbageCollectEnvironment`; it does not keep derived nodes current. Open decision 3.

### Garbage collection

Depends on the held architecture in its derived node part.

`collect` keeps two kinds of root: a derived node with `rootCount > 0` (a held retention root, and from change 4 an observed location), and a derived node that a current subscription's last read read directly. The second kind exists because a subscription holds no root hold: without it, a nested derived node that only a subscription reads is disposed by every `garbageCollectEnvironment`, the next re-read recomputes it, and when its resolver returns a new object that re-read is unequal and the subscription's callback is called with data that did not change. How `collect` obtains the second kind is open decision 27 (held (a)).

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
  collect(
    environment.graph,
    Array.from(environment.subscriptions).flatMap((subscription) =>
      Array.from(subscription.readDerivedNodes),
    ),
  );
```

```ts
// from libs/isograph-runtime/src/graph.ts
/** Disposes every derived node that no node with `rootCount > 0` and no node in `extraRoots` reaches
 * through `Derived` dependencies. An id in `extraRoots` with no stored derived node roots nothing. */
export function collect<TValue>(
  graph: RecomputationGraph<TValue>,
  extraRoots: ReadonlyArray<DerivedNodeId>,
): void;
```

`collect` takes ids, not `DerivedNode`s, because the engine knows nothing about subscriptions and because `deleteSource`, which runs first, can have disposed a node a subscription read: `graph.derivedNodes.get(id)` returns `undefined` for it, and it roots nothing. An id read twice, by two subscriptions or twice by one, is one root.

The subscription records the derived nodes its last read went through. Upstream's `FragmentSubscription` records the records its last read visited in the same way, replaced on each re-read (`encounteredDataAndRecords`, `isograph/libs/isograph-react/src/core/IsographEnvironment.ts:49`, written at `subscribe.ts:160`).

```ts
// from libs/isograph-runtime/src/subscribe.ts
// before (change 2):
export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
  /** Called after each change to `delivered`, after `callback`. */
  readonly listeners: Set<() => void>;
};
// after:
export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  /** The derived nodes of the nested resolvers the last read read. Read by `garbageCollectEnvironment`. */
  readDerivedNodes: ReadonlySet<DerivedNodeId>;
  readonly callback: (result: SubscriptionResult) => void;
  /** Called after each change to `delivered`, after `callback`. */
  readonly listeners: Set<() => void>;
};

/** A subscription read: what it delivers, and the derived nodes of the nested resolvers it read. */
type SubscriptionRead = {
  readonly result: SubscriptionResult;
  readonly readDerivedNodes: ReadonlySet<DerivedNodeId>;
};
```

```ts
// from libs/isograph-runtime/src/storedResults.ts
// storedResultHooks, before (change 2):
/** Hooks for a read that is not a derived node: nothing is recorded, and each nested resolver is
 * read by `readStoredResult`. */
export function storedResultHooks(environment: IsographEnvironment): ReadHooks<ReadResolverThrew> {
  return {
    readRecord: () => {},
    readValue: () => {},
    readResolver: (artifact, root, variables) => readStoredResult(environment, artifact, root, variables),
  };
}
// after:
/** Hooks for a read that is not a derived node: no dependency is recorded, each nested resolver is
 * read by `readStoredResult`, and its derived node's id is passed to `onReadDerivedNode` first. */
export function storedResultHooks(
  environment: IsographEnvironment,
  onReadDerivedNode: (id: DerivedNodeId) => void,
): ReadHooks<ReadResolverThrew> {
  return {
    readRecord: () => {},
    readValue: () => {},
    readResolver: (artifact, root, variables) => {
      onReadDerivedNode(resolverDerivedNode(root, resolverStoreKey(artifact.selectableName, variables)));
      return readStoredResult(environment, artifact, root, variables);
    },
  };
}
```

The id is reported before `readStoredResult` and whether or not the record is present. When `root` is absent, `readStoredResult` returns `noRecord` and stores nothing, so the id names no derived node and `collect` skips it; when a later write inserts the record, the next re-read creates the node and reports the same id again. The hook reports only the nested resolvers the subscription's own reader AST selects; a derived node those read is reached from them through `Derived` dependencies, so `collect` keeps it without the subscription naming it.

```ts
// from libs/isograph-runtime/src/subscribe.ts
// readSubscription, before (change 2):
function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionResult {
  try {
    return readData(environment.store, storedResultHooks(environment), artifact, root, variables);
  } catch (error: unknown) {
    return { kind: 'Err', error };
  }
}
// after:
function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionRead {
  const readDerivedNodes = new Set<DerivedNodeId>();
  const hooks = storedResultHooks(environment, (id) => {
    readDerivedNodes.add(id);
  });
  try {
    return { result: readData(environment.store, hooks, artifact, root, variables), readDerivedNodes };
  } catch (error: unknown) {
    return { result: { kind: 'Err', error }, readDerivedNodes };
  }
}
```

A read that throws keeps the ids it reported before the throw: those derived nodes are stored, and the next re-read walks the same path up to the broken invariant.

```ts
// from libs/isograph-runtime/src/subscribe.ts
// readAndSubscribe, before (change 2):
  const subscription: Subscription = {
    artifact: readerArtifact,
    root: link,
    variables,
    delivered: readSubscription(environment, readerArtifact, link, variables),
    callback,
    listeners: new Set(),
  };
// after:
  const read = readSubscription(environment, readerArtifact, link, variables);
  const subscription: Subscription = {
    artifact: readerArtifact,
    root: link,
    variables,
    delivered: read.result,
    readDerivedNodes: read.readDerivedNodes,
    callback,
    listeners: new Set(),
  };
```

```ts
// from libs/isograph-runtime/src/subscribe.ts
// callSubscriptions, before (change 2):
  environment.subscriptions.forEach((subscription) => {
    const result = readSubscription(
      environment,
      subscription.artifact,
      subscription.root,
      subscription.variables,
    );
    if (
      !subscriptionResultsAreEqual(
        subscription.artifact,
        subscription.delivered,
        result,
      )
    ) {
      subscription.delivered = result;
      reportCallbackError(() => subscription.callback(result));
      subscription.listeners.forEach((listener) =>
        reportCallbackError(listener),
      );
    }
  });
// after:
  environment.subscriptions.forEach((subscription) => {
    const read = readSubscription(
      environment,
      subscription.artifact,
      subscription.root,
      subscription.variables,
    );
    subscription.readDerivedNodes = read.readDerivedNodes;
    if (
      !subscriptionResultsAreEqual(
        subscription.artifact,
        subscription.delivered,
        read.result,
      )
    ) {
      subscription.delivered = read.result;
      reportCallbackError(() => subscription.callback(read.result));
      subscription.listeners.forEach((listener) =>
        reportCallbackError(listener),
      );
    }
  });
```

`readDerivedNodes` is replaced on every re-read, equal or not. Two `Ok` results that `selectionSetDataAreEqual` finds equal followed the same links with the same variables, so they read the same derived nodes. Two `MissingData` results are equal wherever the data is missing, so the new read can have read other nested resolvers, and the new set names what the next re-read will read. `subscribe.ts` imports `DerivedNodeId` as a type from `graph.ts`, and `storedResults.ts` already imports `resolverDerivedNode` and `resolverStoreKey`. The unsubscribe cleanup is unchanged: deleting the subscription from `environment.subscriptions` removes its ids from the next `collect`.

Cost: one `Set` per subscription read, with one insertion per `Resolver` node the subscription's reader AST walks, which the walk already visits; one array of every subscription's ids per `garbageCollectEnvironment`. A write pays only for the sets its re-reads fill; only `garbageCollectEnvironment` builds the array.

- Records: store.md's mark and sweep, from each member of `environment.retainedQueries`, with `garbageCollect`'s parameters unchanged. A derived node a retention reaches never loses a record: it reads only what its reader ASTs select along current links, and the retained query's normalization AST is the merge of those reader ASTs with the same variables (query-generation.md), so every record it reads is marked. A derived node reached only from an observed location or from a subscription can lose its records (Level 2). Each deleted record goes through `deleteSource`, which disposes its derived nodes (change 2).
- Derived nodes: nodes that held roots, observed locations, and current subscriptions' last reads reach through `Derived` dependencies are kept; every other derived node is disposed by `disposeDerivedNode`, so no `dependents` set or `sourceDependents` set keeps it. A disposed derived node has no kept dependent: a kept node that read it would make it reachable. Derived nodes a plain `readStoredResult` stored outside any retention and subscription, those of a disposed retention, and those only an unsubscribed subscription read, are collected this way, marked or not. A nested derived node a current subscription read survives `garbageCollectEnvironment` with its stored value, so the subscription's next re-read returns the same object and calls no callback.

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
- A nested derived node only a subscription reads: `normalizeData` of `ShapePage` with `{ id: "1" }`, `dispose` of its `Retention`, `retainQuery` of `ShapePage`'s normalization AST with `{ id: "1" }` (records kept, no root hold), and `readAndSubscribe` of `ShapeLabel`'s reader artifact at `Pet:1`. `garbageCollectEnvironment` disposes the derived nodes of `ShapePage` and `ShapeLabel` and keeps the one of `NameShape` on `Pet:1`; a following `callSubscriptions` calls neither `NameShape`'s resolver nor the subscription's callback. After the subscription's cleanup, `garbageCollectEnvironment` disposes the derived node of `NameShape`.
- A link move under a subscription: `normalizeData` of `FriendPage` and of `PetSummary` with `{ id: "2" }`, `dispose` of both `Retention`s, `retainQuery` of both normalization ASTs with their variables, and `readAndSubscribe` of `FriendLabel`'s reader artifact at `Pet:1`. A `PetDetail` response moves `Pet:1.bestFriend` to `Pet:3` (with `name` and `nickname`), `dispose` of the `Retention` its `normalizeData` returns, then `callSubscriptions`. `garbageCollectEnvironment` keeps the derived node of `PetName` on `Pet:3` and disposes the one on `Pet:2`, while `Pet:2` stays in `environment.store`.
- Served from the store: after `normalizeData` of `BadgePage` with `{ id: "1" }` and `dispose` of its retention, `fetchEntrypoint` of `BadgePage` with `shouldFetch` `IfNecessary` sends no request (the network function is not called), calls none of the five resolvers (their derived nodes are `Clean`), and `environment.retainedQueries` holds one more `Retention`, whose `location` has a stored derived node; the returned cleanup removes it, and `garbageCollectEnvironment` then leaves `environment.graph.derivedNodes` empty.
- `shouldFetch` `No` on `new Map()`: the call sends no request, and the derived node at the `Retention`'s `location` holds `MissingData` naming `Query.__ROOT`; a later `normalizeData` of `BadgePage`'s response calls each of the five resolvers once.
- `dispose` of `FriendPage`'s retention, then a change to `Pet:2.name` before `garbageCollectEnvironment`: no resolver is called.
- store.md's GC tests and network-and-subscriptions.md's tests of `garbageCollectEnvironment` are unchanged: the first call `garbageCollect` with a retained list and compare the store with `storeOf`; the second retain through `retainQuery` and compare `environment.store`.

## 4. Level 2 and the post-write pull

Depends on the held architecture, except `listen`, `snapshot`, and `Observed`.

This change adds observers. A write marks (change 2); `settle` then pulls every derived node an observer is at, and `notify` calls each observer whose value changed (open decision 28, held (a)). A derived node nobody observes stays marked until a read pulls it (open decision 3, held (b)).

### No heights

3b needs no height. A pull brings a derived node's inputs up to date before the node uses them, through `readDerivedNode` and `checkDependencies`, so the order of recomputations within a pass is the order of the reads, whatever the graph's shape. The order in which `settle` pulls the observed derived nodes changes nothing (Ordering). A read that would be upward under 3a (a link set from `null`, a refinement on a record of another entity, an eager client pointer) is one more pull of a derived node that may be marked, and needs no adjustment.

Nothing of heights remains in the engine: no field, no queue, no adjustment, and no abandon. Heights remain only in Candidate 3a. A scheduler that orders passes (Follow-ups 9) orders observers by priority, not by height.

### Observed derived nodes

```ts
// from libs/isograph-runtime/src/graph.ts
export type RecomputationGraph<TValue> = {
  // .. change 2's fields, and:
  readonly observers: Map<DerivedNodeId, Observed<TValue>>;
};

/** Pulls every stored derived node an observer is at, in `observers` order. */
export function settle<TValue>(graph: RecomputationGraph<TValue>): void {
  graph.observers.forEach((_observed, id) => {
    const node = graph.derivedNodes.get(id);
    if (node !== undefined) {
      bringUpToDate(graph, node);
    }
  });
}
```

`createGraph` starts `observers` empty. `settle` costs one lookup per observer and returns at once for a `Clean` one, plus the pulls of the marked derived nodes the observed ones read. `notify` iterates the observers too, so a write costs `O(observers)` before any recomputation. Recording the observed derived nodes that marking reaches (one `graph.observers.has(node.id)` per marked node) would make `settle` `O(marked observed)`, and is not needed while `notify` is `O(observers)`. A computation adds no observer (callbacks run in `notify`), so the `forEach` meets no entry added during it.

Observed derived nodes are the only derived nodes the runtime pulls after a write in changes 1 to 4. A subscription's nested derived nodes are pulled by its re-read in `callSubscriptions` (change 2, Subscriptions), and the mounted derived nodes with an effect by `settle` when Values with lifecycles lands.

### Settlement

`normalizeData` runs `settle` after `normalize` and `markChanged` and before the root hold (change 3). The root hold changes no observed value: every derived node it pulls is marked or new, and after `settle` every observed derived node is `Clean`, so by change 2's invariant none of them reads a marked one. `notify` runs last.

A response that throws part-way must settle too. From this change observers are called after a write, so `writeOrMark` as change 2 wrote it, which marks the `PartialWriteError`'s changes and rethrows, would leave the observed derived nodes the partial write reached marked and their observers uncalled, and `snapshot` would return the values from before the write until another write. Changes 2 and 3 have no observers, so their marking of a partial write is complete.

```ts
// from libs/isograph-runtime/src/environment.ts
// normalizeData, before (change 3):
  markChanged(
    environment.graph,
    writeOrMark(environment, entrypoint.normalizationAst, response, variables).map(changeSource),
  );
  return retainEntrypoint(environment, entrypoint, variables);
// after:
  markChanged(
    environment.graph,
    writeOrMark(environment, entrypoint.normalizationAst, response, variables).map(changeSource),
  );
  settle(environment.graph);
  const retention = retainEntrypoint(environment, entrypoint, variables);
  notify(environment.graph);
  return retention;

// writeOrMark, before (change 2):
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
// after:
/** `normalize`'s changes; on a `PartialWriteError`, marks its changes, settles, and notifies
 * before rethrowing. */
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
      settle(environment.graph);
      notify(environment.graph);
    }
    throw error;
  }
}
```

The partial write retains nothing: `retainEntrypoint` does not run, as in change 3. `notify` never throws (open decision 9), so the rethrown error is the `PartialWriteError`. `settle` throws only when a computation throws outside its resolver (open decision 22). The doc comment of `normalizeData` (change 3) gains: "On a `PartialWriteError`, settles and calls observers before rethrowing it." `garbageCollectEnvironment` runs `settle` and then `notify` after `collect`: an observed derived node that read a deleted record was marked `Dirty` by `deleteSource` and recomputes to `MissingData`, and `collect` disposes no observed derived node (its `rootCount` is above 0).

```ts
// from libs/isograph-runtime/src/environment.ts
// garbageCollectEnvironment, after:
  garbageCollect(environment.store, Array.from(environment.retainedQueries)).forEach((record) =>
    deleteSource(environment.graph, recordSource(record)),
  );
  collect(
    environment.graph,
    Array.from(environment.subscriptions).flatMap((subscription) =>
      Array.from(subscription.readDerivedNodes),
    ),
  );
  settle(environment.graph);
  notify(environment.graph);
```

Work per write: marking visits each derived node that read a changed source and each `Clean` derived node above those, once; `settle` takes one step per observer and pulls the marked derived nodes the observed ones read; a pulled derived node recomputes only when an input changed value. The traces are in Candidate 3b.

A scheduler (Follow-ups 9) chooses which observers a pass pulls; the others keep their marks.

### `listen` and `snapshot`

```ts
// from libs/isograph-runtime/src/graph.ts
/** The observers of one derived node. */
export type Observed<TValue> = {
  readonly callbacks: Set<() => void>;
  /** The value the callbacks were last called for, or were registered at. */
  notified: TValue | undefined;
};

/** Adds `onChange` at `id`; when the derived node exists, pulls it, then adds one to its `rootCount`.
 * Returns the function that removes both. */
export function observe<TValue>(
  graph: RecomputationGraph<TValue>,
  id: DerivedNodeId,
  onChange: () => void,
): CleanupFn;

/** The derived node's value as of its last computation, or `undefined` when no node is stored. Computes nothing. */
export function valueOf<TValue>(graph: RecomputationGraph<TValue>, id: DerivedNodeId): TValue | undefined;

/** Calls each observer whose derived node's value is not `===` its `notified`, each callback through
 * `reportCallbackError` (open decision 9). Never throws. */
export function notify<TValue>(graph: RecomputationGraph<TValue>): void;
```

```ts
// from libs/isograph-runtime/src/observe.ts
/** Calls `onChange` after each settlement that changed the result stored at `location`, and
 * keeps that result current after every write. Returns the function that stops it. */
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
2. When a derived node is stored at `id`: `bringUpToDate` on it, then adds one to `rootCount`. The pull leaves it `Clean`, and every derived node below it. The derived nodes it recomputes were marked, and by change 2's invariant no observed derived node reads one, so no other observer's value changes.
3. Sets `notified` to the derived node's value.

The returned function removes the callback, subtracts one from the `rootCount` of the derived node stored at `id` when there is one, and removes the `Observed` when it has no callback left. When no derived node is stored at `id`, `observe` only registers, and `createDerivedNode` starts the node's `rootCount` at the callbacks there.

A location that is marked when a component first renders: `snapshot` returns the last computed result (R1: it computes nothing). `listen`, in React's subscribe, brings it up to date, and `useSyncExternalStore` compares the snapshot after subscribing and renders again with the current result. Open decision 3.

Notification, after settlement in `normalizeData` and `garbageCollectEnvironment`: for each `Observed` in `graph.observers` in insertion order, the derived node's value is compared by identity with `notified`; when different, `notified` is set and each callback is called once. Identity suffices because `recompute` keeps the value object when `valuesEqual` holds, and a `Check` node verified without recomputing keeps it too. An observed resolver that returns a new object on every recomputation notifies on every recomputation (R4); a stable identity for it would come from upstream's merge of the data it reads, not from comparing its output. Each callback is called through `reportCallbackError` (`libs/isograph-runtime/src/subscribe.ts:157-167`), as `callSubscriptions` calls each subscription's callback and listeners: a throw is rethrown in its own microtask, the loop continues, and `notify` never throws (open decision 9). `graph.ts` imports nothing but `util.ts`, so change 4 moves `reportCallbackError` from `subscribe.ts` to `util.ts`, unchanged; `subscribe.ts` and `network.ts` import it from there. The loop is `O(observers)`; a list of the observed derived nodes `recompute` changed would make it `O(changed)`, and is not needed while observers are the mounted components of one screen.

What each level owns:

- Level 1 holds values, dependencies, dependents, marks, and root holds. It runs user code (resolvers).
- Level 2 holds the observed ids, callbacks, and the last-notified identity. It runs no resolver, and is notified only after level 1 settled, once per changed derived node per settlement.

A parent component never re-renders with a stale child: every observed derived node is `Clean` before the first callback, and so is every derived node it reads, so any `snapshot` during a re-render a callback caused returns the settled value, whatever the order callbacks run in. React additionally batches the `setState`s that `useSyncExternalStore` calls in one notification loop into one render.

What React needs, and gets without `@isograph/runtime` knowing React: `useSyncExternalStore(subscribe, getSnapshot)` where `subscribe(onStoreChange)` returns an unsubscribe function (`listen`), and `getSnapshot` returns the same value while nothing changed and is pure and cheap (`snapshot`, a lookup). How a component learns its location (the derived node a parent's result names) is the React package's, with `@component` (Later).

An observer keeps its derived node current (`settle` pulls it after every write) and keeps the node through `collect` (its `rootCount`), but retains no record: data stays only while a retention keeps it. A location whose record `garbageCollectEnvironment` deleted has no stored derived node, and `snapshot` returns `undefined`.

### Tests

`test/graph.test.ts` gains, with literal ids: two observed derived nodes that both read source `a`, the second also reading the first: `markChanged([a])` then `settle` runs the first's computation before the second's, each once. A derived node nobody observes that read `a` is `Dirty` after `markChanged([a])`, and `settle` does not run its computation. An observed derived node whose inner dependency recomputes to a value `valuesEqual` accepts: `settle` runs only the inner computation, and `notify` calls no callback. `observe` of an absent id registers only; the derived node created later starts with `rootCount` 1.

`test/level2.test.ts`, over an environment built with `createIsographEnvironment(new Map(), deferredNetwork().networkFunction)`:

- `listen` then an unchanged `normalizeData`: no call. A changed one: one call, after which `snapshot` returns the new result; inside the callback, `snapshot` already returns it and every resolver's call count is final.
- Diamond, an observer at `BadgePage`, `Rex` to `Max` through a `PetDetail` response: inside that `normalizeData`, `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once, `Initial` before `Monogram` and `Signature`, both before `Badge`, `Badge` before `BadgePage`; `Badge` receives `{ Monogram: "M.", Signature: "M " }`; the observer is called once.
- Diamond, an observer at `BadgePage`, `Rex` to `Ray`: `Initial` is called once and no other resolver is called; the observer is not called.
- Diamond, an observer at `BadgePage`, only `nickname` changes: `Signature`, `Badge`, `BadgePage` once each; `Initial` and `Monogram` zero.
- New object, an observer at `ShapePage`, `Rex` to `Ray`: `NameShape` and `ShapeLabel` are called once each, `ShapePage` zero times, and the observer is not called. An observer at `NameShape` as well is called once, because its new `{ length: 3 }` is not `===` the old one.
- Two observers, at `FriendPage` and at `PetSummary` with `{ id: "2" }`: one write to `Pet:2.name` calls `PetName` on `Pet:2` once, `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` once each, `RecentCheckins` zero, and each observer once.
- One of two paths observed: an observer at `FriendPage`, `PetSummary` with `{ id: "2" }` retained and not observed; `Pet:2.name` changes: `PetName` on `Pet:2`, `FriendLabel`, `FriendCard`, and `FriendPage` are called once each inside the `normalizeData`, and `PetSummary` zero times; the next `readStoredResult` of `PetSummary` calls `PetSummary` once and `PetName` zero times.
- Retained, not observed: `BadgePage` retained, `Rex` to `Max` through a `PetDetail` response: none of `BadgePage`'s resolvers is called inside that `normalizeData`, and `snapshot` at `BadgePage` returns the previous result; `listen` at `BadgePage` then calls each of the five once and calls no callback, and `snapshot` returns the new result.
- The stop function: after the only observer at `BadgePage` stops, `Rex` to `Max` calls no resolver; `snapshot` still works.
- A link set from `null` under an observer at `FriendPage` with `{ id: "4" }`, `Pet:4.bestFriend` `null`: a write that sets it to `Pet:2` calls `FriendLabel` on `Pet:4`, `PetName` on `Pet:2`, `FriendCard`, and `FriendPage` once each. Then `Pet:2.name` changes: `PetName` on `Pet:2`, `FriendLabel` on `Pet:4`, `FriendCard`, and `FriendPage` are called once each, in that order.
- Link move under an observer at `FriendPage`: `Pet:1.bestFriend` moves to `Pet:3` (with `name` and `nickname`): inside that `normalizeData`, `FriendLabel` is called once and `PetName` on `Pet:3` once. Then `Pet:2.name` changes: no resolver is called. Then `Pet:3.name` changes: `PetName` on `Pet:3`, `FriendLabel`, `FriendCard`, and `FriendPage` are called once each.
- An observer at `FriendPage` whose retention was disposed: `garbageCollectEnvironment` deletes the records, calls the observer once inside it, `snapshot` returns `undefined`, and `environment.store` is `toEqual(new Map())`.
- A callback that calls `normalizeData` with another change: the nested call notifies; each observer receives each result once.
- A partial write under an observer at `BadgePage`: a response that changes `Pet:1.name` from `Rex` to `Max` and then throws: inside that `normalizeData`, `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once, the observer is called once, and then `normalizeData` throws the `PartialWriteError`; `snapshot` at `BadgePage` returns the new result.
- A callback that throws, with `queueMicrotask` stubbed as `test/subscribe.test.ts`'s `stubQueueMicrotask` does: the other observers are still called, `normalizeData` returns its `Retention`, and running the queued task throws the callback's error; two that throw queue two tasks, one per error.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: "`pico-runtime.md` stays parked" is replaced by this doc as the step after v1. `Store` is unchanged, and its functions keep their parameters and its tests hold. `normalize` writes through `writeValue`, skipping a value unchanged by its AST shape, returns `ReadonlyArray<StoreChange>`, and throws `PartialWriteError` (change 1); the package index stops exporting `normalize`, which `normalizeData` replaces there (change 2b, open decision 17); `garbageCollect` returns the links of the records it deleted (change 2).
- `refactors/pending/runtime/network-and-subscriptions.md`: `IsographEnvironment` gains `graph`, which `createIsographEnvironment` creates (change 2). `environment.ts` gains `normalizeData` (change 2), which takes the entrypoint and returns a `Retention` that `environment.retainedQueries` also holds (change 3), and settles and notifies (change 4); `garbageCollectEnvironment` deletes the derived nodes of deleted records (change 2), collects unreached nodes (change 3), and settles and notifies (change 4). `makeNetworkRequest` calls `normalizeData` where it called `normalize` (change 2), and from change 3 holds the returned `Retention` and disposes it in its cleanup. Its Why the store-only functions stay store-only holds: the functions that need the graph take the environment. `retainQueryWithoutMakingNetworkRequest` takes the entrypoint, holds a `Retention` from `retainEntrypoint`, and disposes it in its cleanup (change 3, Serving from the store). Its change 7 keeps request entries in `environment.requests`, beside the store, and `Store` stays store.md's nested map; this doc adds no field to `Store` either. Its `readSubscription` reads with `readData` and `storedResultHooks` instead of `readButDoNotEvaluate`, `SubscriptionResult` gains `ReadResolverThrew`, `ReadErr` is only a broken store invariant, and `subscriptionResultsAreEqual` compares two `ResolverThrew` by `===` of their errors (change 2, Subscriptions).
- `refactors/pending/runtime/react.md`: `unwrapOrSuspend` throws `result.error` on `ResolverThrew` as it does on `Err`; without that arm a `ResolverThrew` reaches the `MissingData` path and suspends (change 2, Subscriptions).
- `refactors/pending/runtime/components-in-userland.md`: `useClosureData`'s switch gains `case 'ResolverThrew': throw result.error;` (change 2, Subscriptions).
- `refactors/pending/runtime/reader-ast.md`: `ReaderArtifact.selectableName` and its printing; the walk takes `ReadHooks<TFailure>` and `readButDoNotEvaluate` passes v1's hooks, so `read` and `readButDoNotEvaluate` keep their behavior and its tests hold unchanged; `ResolverResult` adds `ReadResolverThrew` to `ReadResult` for stored results, and `ReadResult` is unchanged (change 2). Its `read` stays, and `readStoredResult` is the stored read beside it (change 2).
- `refactors/pending/runtime/normalization-ast-mount-markers.md`: candidate 6 is answered by Resolver markers (held: no marker); its open question on how this doc models a mount is answered by `Retention` and settlement.

## Other derived node kinds

`check` stays the plain function from store.md; caching it as a derived node of the graph was considered and rejected by the user (no need).

The engine is general over derived node kinds only as far as the follow-ups that add nodes need: a caller supplies each node's id, owner, and computation, and the engine never inspects a value except through `valuesEqual`. Resolver derived nodes are the only kind changes 1 to 4 create. The parts of the API that serve the follow-ups:

- `graph.ts` imports nothing from the store, the reader, or the environment, and names no resolver, record, or artifact. This is open decision 12's held design, which changes 2 to 4 need on their own.
- A derived node's owner is a caller-supplied `SourceId` (`DerivedNodeDefinition.owner`). Closure-component derived nodes (components-in-userland.md) need it: the wrapper is a node at the closure field's resolver store key, owned by the record's `recordSource`, whose computation reads only the record's presence (The engine, the store, and the environment, Lifetime still follows records), and a second node per closure derived node for its data read is that doc's open question 3. Both are resolver-shaped derived nodes, so they need nothing else beyond change 2's resolver nodes.
- A derived node's computation and id come from its creator (`DerivedNodeDefinition`, `track`, `readDerivedNode`, `retainRoot`); no engine function builds one. Follow-up 10 needs it: a runtime-defined computation is not a resolver and has no resolver store key.
- The graph's value type is a parameter, `TValue`. Follow-up 10 needs it: a derived value or a selector's value is not a `ResolverResult`, so that follow-up widens the environment's graph to a union of the kinds' values, and `valuesEqual` to a function that compares two values of one kind and answers unequal for two kinds.
- Ordering is the engine's: a pull brings a derived node's inputs up to date before the node uses them, so a caller supplies no order. Follow-up 10 needs it: a runtime-defined computation has no artifact that could carry one.
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
- Optimistic writes: a layer of server entries that settlement reads through, and later a tree of them (Follow-ups 12). What the layered store must report to the recomputation engine is The engine, the store, and the environment, What the environment must report.
- Eager client pointers: a pointer node that reads client fields on its target lets a derived node of a field read a derived node of the same field on another record. The pull orders it (open decision 14); a cycle of derived nodes through the pointer is open decision 18.

## Follow-ups

The user's design notes, recorded for later. None of them is part of Step A or changes 1 to 4, and none is approved; 10 is the user's planned addition, 11 is held, and 12 is a stated goal that constrains changes 2 to 4. Upstream paths here follow the doc's convention; `isograph-react-disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`, and `isograph-disposable-types/` paths are under `isograph/libs/isograph-disposable-types/src`.

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

Relation to changes 2 and 3: the stored selection-set result is an extra level between server values and resolver derived nodes. A resolver derived node depends on a selection-set derived node, the selection-set derived node depends on server values and on the resolver derived nodes of the `Resolver` nodes it contains, and marking and bringing up to date walk that level as they walk a resolver derived node. Under held 3b the level needs no rule of its own: marking and the pull walk a selection-set derived node as they walk a resolver derived node.

### 9. Scheduled recalculation

Partly future work; R10 requires that the architecture allows it. The user wants control over when recalculation runs:

- Unobserved derived nodes are lazy. A derived node nobody observes (an unmounted component's result, a retained query nobody renders) is only marked on a write and is pulled when next read. Changes 2 to 4 build this: it is the marks, and `settle`'s pull of the observed derived nodes only.
- Deferred work. Some recalculation and re-rendering runs later at lower priority, for example the work caused by a write that came from user input. In the React layer this corresponds to `startTransition` and `useDeferredValue`. `@isograph/runtime` stays React-free and takes the caller-supplied scheduler of note 7; the scheduler decides when each pending pass runs.

What deferral changes in the requirements: R2 becomes "every observed result is current when the pass for its priority finishes; a result nobody observes is current when read". R5 becomes "observers are called once per pass", not once per write.

What deferral needs on top of changes 1 to 4:

- A priority on a write (`normalizeData` called inside a transition) or on an observer (a component rendered with a deferred value).
- A deferred write marks as every write does. `settle` of a pass pulls only the observed derived nodes of that pass's priority, and the others keep their marks until their pass. This lifts "every observed derived node is `Clean` when the write returns" until the deferred pass runs.
- An urgent pull of a derived node that a deferred write also marked reads the deferred changes below it too, so the urgent pass shows them early for that derived node, as React does when a transition and an urgent update touch the same state.
- A deferred pass pulls each observed derived node of its priority and notifies its observers. A derived node an urgent pass already pulled is `Clean`, and the deferred pass reads it as is. A second write before the pass marks on top of the existing marks, so the pass recomputes each derived node once for both writes.

Fit with each candidate:

- Held 3b: marking is already separate from pulling, and a derived node a pass skips keeps its mark. Deferral is `settle` choosing which observers each pass pulls; no state is added.
- 3a with `Stale` marks: a deferred write marks every derived node it reaches `Stale`, and the observed derived nodes the marking reaches are recorded under the write's priority; an urgent write that reaches a needed `Stale` derived node queues it.
- 2: laziness is native, because nothing is verified until demanded, and deferral is the scheduler choosing when to verify which roots. Each deferred pass still walks every live edge under its roots.
- Plain 3a and 3c: neither has a lazy mode. Skipping an unobserved derived node leaves it stale with no record that it is stale.
- 4: Preact already keeps unsubscribed computeds lazy and defers effects to the end of a batch; priority would be added to its effect queue.
- 5: both work per root, at full cost per pass and with no cutoff below the root.

React constraint: `useSyncExternalStore` performs a store update as blocking even inside a transition, per React's documentation. Deferred re-rendering therefore needs the React package to hold a deferred snapshot per observer and update it inside `startTransition`, or to read through `useDeferredValue`; this is the React package's design, not the runtime's.

### 10. Runtime-defined computations

A memoized computation the runtime or a user defines at run time, which reads other derived nodes: a derived value over several resolver results, a selector a hook creates. The pull orders it as it orders every derived node, from what its computation reads. What it needs from the engine is a caller-supplied id and computation and a wider value type (Other derived node kinds).

### 11. The fetch decision from the root derived node

Held, not approved, and not part of changes 1 to 4; it needs changes 2 and 3. With `shouldFetch` `IfNecessary` (network-and-subscriptions.md, `fetchEntrypoint`), `fetchEntrypoint` decides from the root derived node of the entrypoint on its variables instead of from `check` over the normalization AST. `Ok` (or `ResolverThrew`) means the data is present; `MissingData` means fetch.

```ts
// from libs/isograph-runtime/src/network.ts
// fetchEntrypoint, before (change 3, Serving from the store):
    case "IfNecessary":
      return check(environment.store, entrypoint.normalizationAst, variables).kind === "EnoughData"
        ? retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables)
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
        : retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables);
```

- A repeated call with no write between is a lookup: the root derived node is `Clean`, and `track` returns its stored result. After a write that reached it, the call pulls it, or `settle` already did (observed). `check` walks the whole normalization AST on every call.
- A root derived node this call creates is outside any retention until `retainEntrypoint`'s root hold (in `normalizeData`, or in `retainQueryWithoutMakingNetworkRequest` on the branch that serves from the store) or a `listen` reaches it; `collect` (change 3) disposes it otherwise.

Why a complete read implies that the normalization AST's data is present:

- The normalization AST is the merge of the reader ASTs the entrypoint reaches, with the same variables (query-generation.md, Merging), plus the selections `complete` adds: the discriminator and the identity (`crates/isograph_compiler/src/merged_selection.rs:1555-1560`). The discriminator has no normalization node (`normalization_node`, `crates/isograph_compiler/src/normalization_ast.rs:135-140`), so `normalize` does not store it and `check` does not look it up. An `Ok` root means every reader the entrypoint reaches read every selection it selects, along current links and refinements, which are the positions `check` visits.
- The identity is an ordinary `Selection` in the normalization AST: `CompletedSelection::Identity` becomes a `NormalizationNode::Selection` with a leaf target (`normalization_ast.rs:141-150`). `complete` adds it only for an entity whose identity is `Select`; an entity whose identity is `Fixed` or `Omit` has no `id` selection (`merged_selection.rs:1555-1558`). A reader need not select `id`, but `normalize` writes it in the pass that creates the record: `compositeValue` calls `recordAt`, which inserts the record, and then `writeSelections` over the target's selections, which hold the identity (`libs/isograph-runtime/src/normalize.ts:151-158`; `recordAt` at `:163-169`, `writeSelections` at `:52-79`; v1 line numbers, before change 1). So a present record of a `Select` entity holds its `id`.
- The exception is a `normalize` that threw part-way (open decision 17). Its writes stay, so a record can exist without its `id` while every selection a reader reads is present. The root is then `Ok` where `check` reports `MissingData`, and no fetch is made. The data the readers read is present.
- `@loadable` selections are not an exception. i2 accepts `@loadable` with no effect beyond validation: the query selects the field's selections and the parent's reader reads the field exactly as without the directive (graphql-protocol/node-entrypoints-and-loadable.md, the opening section and its example). No other normalization AST selection in i2 today is outside every reader: `complete` adds only the two selections above.
- `ResolverThrew` is not equivalent to `check`. The walk stops at a nested `ResolverThrew` (`allOk` stops at the first result that is not `Ok`), so the selections after the throwing resolver in each enclosing selection set are not read, and data missing there does not show in the root's result. Treating `ResolverThrew` as present skips a fetch that `check` would make, and that fetch could change the throwing resolver's inputs. Treating it as a reason to fetch sends a request on every call while the resolver throws. The user's statement holds it as present; this difference is not settled.

Components (later): a component field's data is read at render (components-in-userland.md, A field whose result is a closure), so the root derived node does not cover it unless each component field has a data derived node computed with the root (components-in-userland.md, Under changes 2 to 4, and its open question 3). Until then, an entrypoint that reaches a component field needs `check`.

`check` stays a plain function exported from the package index (`libs/isograph-runtime/src/index.ts:43`); `test/check.test.ts` calls it.

### 12. Forks: a tree of optimistic worlds

A stated goal, not built now. Changes 2 to 4 must keep the constraint below so that forks can be added without redesign.

Upstream keeps optimistic state as a stack of layers:

- Storage. `environment.store` is the child-most layer (`isograph/libs/isograph-react/src/core/IsographEnvironment.ts:74`). Each layer has a `parentStoreLayer` and one `childStoreLayer`, and holds `data: StoreLayerData`, a sparse map from typename and id to a partial record of plain values, in which a `null` record is a deletion (`IsographEnvironment.ts:165-169`). The base layer's child is an optimistic layer or none; the other kinds are network response, `startUpdate`, optimistic network response, and optimistic updater layers (`optimisticProxy.ts:100-159`).
- Reads. A read of a field walks from the child-most layer toward the base and returns the first value a layer holds; a layer that holds `null` for the record ends the walk as absent (`getStoreRecordProxy`, `optimisticProxy.ts:24-44`; `getMutableStoreRecordProxy`, `:46-98`, the field walk at `:53-69`). A write through the proxy goes to the child-most layer (`:89-95`).
- An optimistic response. `makeOptimisticUpdate` pushes an optimistic network response layer, normalizes the optimistic response into it, and calls `callSubscriptions` with the records it wrote (`makeNetworkRequest.ts:495-544`; `addOptimisticNetworkResponseStoreLayer`, `optimisticProxy.ts:268-289`).
- A network response under an optimistic layer. `addNetworkResponseStoreLayer` writes into the base or a network response layer when that is the child-most layer, and otherwise pushes a new network response layer above the optimistic one, so an unrelated response shadows the optimistic values for the fields it writes (`optimisticProxy.ts:161-183`; called at `makeNetworkRequest.ts:254` and `writeData.ts:39`). A `startUpdate` above an optimistic layer becomes a `startUpdate` layer that keeps its updater for re-execution, and one on the base layer writes into it (`addStartUpdateStoreLayer`, `optimisticProxy.ts:203-240`).
- Commit and rollback. Upstream never merges an optimistic layer into its parent; it always reverts it. When the real response arrives, `revertOptimisticStoreLayerAndMaybeReplace` empties the optimistic layer, normalizes the response into a new network response layer in its place (`optimisticProxy.ts:406-431`), re-runs the updater of every `startUpdate` and optimistic updater layer above it and keeps the network response layers (`reexecuteUpdatesAndMergeData`, `:332-364`), and splices the optimistic layer out: into the base, by merging every layer down to the next optimistic one (`mergeLayersWithDataIntoBaseLayer`, `:303-316`, used at `:451-459`), or by merging two adjacent network response layers (`:460-477`), or by relinking (`:478-486`). On a network error and on dispose the same function runs with no response, which is rollback (`makeNetworkRequest.ts:305-316`, `:322-334`). Merging the layers that cannot be reverted into the base is upstream's commit.
- Notification. After a revert, `compareData` compares the merged data of the reverted layers with the merged data that replaced them, by record, each field by `!==` ("TODO: compare links, compare arrays", `optimisticProxy.ts:506-544`, the comparison at `:533-534`), and `callSubscriptions` re-reads every subscription whose records overlap the changed ones (`:488-492`; `subscribe.ts:63-109`).
- Garbage collection runs only when the base layer is the only layer (`garbageCollection.ts:65-68`).

A fork generalizes upstream's stack to a tree of layers. A stack is a tree with one path. A world is a path from the base layer to a layer, and a read in world `W` takes, for each location, the value of the deepest layer on `W`'s path that holds it, which is upstream's read when the tree has one path. Two forks of one parent are two worlds that share the parent's layers and nothing below their fork point, such as two optimistic mutations a user can confirm or cancel independently, or a transition's world beside the committed one.

The constraint on changes 2 to 4. A derived node's state in one world (its value, its mark, and the dependencies of the computation that produced that value) lives together in one record, `DerivedNodeState`, and the engine reads and writes it only through one accessor, `stateOf` (change 2, The engine). Later, that record is layered per world: `states: Map<Layer, DerivedNodeState>` on the derived node, and `stateOf` in world `W` takes the state of the deepest layer on `W`'s path. What is not per world stays on the node: its id, owner, computation, and dependents. Effects run only in the committed world (Values with lifecycles): a pull in a fork runs computations and sets up nothing.

Future work that follows from the layering:

- The accessor. A read in `W` must not copy an inherited state, or the copy goes stale when the ancestor's state changes; a write in `W` must not change a state `W` inherits. The layered `stateOf` copies the inherited record into `W`'s layer before a write.
- Dependents are the union across worlds: a derived node is in a source's or node's `dependents` while any layer's state lists that dependency.
- Heights: none under held 3b. Under 3a they would be the maximum across worlds.
- 3b's marks are per layer. A write to layer `L` marks the states of `L` and of every layer below `L` in the tree, since every world through `L` reads it; a node whose state in `L` is inherited gets its own marked state in `L`.
- Sources follow the same rule. Under open decision 29 (a) a source cell holds one value per layer that wrote it, and a read in `W` takes the deepest on `W`'s path.
- Garbage collection roots become (derived node, world) pairs; a root hold and an observer belong to a world.
- Committing a layer merges its overrides, the source values and derived node states it holds, into its parent, and its children re-parent to the parent; a removed layer's overrides are dropped, and the environment marks each location whose effective value changed in each world below it (The engine, the store, and the environment, What the environment must report).

## Open decisions

### 1. Resolver markers in the normalization AST

- (a) Root only: `normalizeData` takes the entrypoint, and settlement creates the rest through reader ASTs (held, Resolver markers).
- (b) A marker per selection set, as the user's design describes. Cost: artifacts import reader artifacts and source modules, the marker set goes stale as links change, and it duplicates what reader ASTs already say.

### 2. Recomputation architecture

The candidates and their comparison are Recomputation architecture.

- (a) 3b, mark along reverse edges, then pull (held, the user's direction). A write marks `Dirty` each derived node that read a changed source and `Check` every derived node above those, and runs no resolver; a pull verifies a `Check` derived node through its dependencies and recomputes a derived node only when an input changed value; `settle` pulls the observed derived nodes after each write (open decision 28). Cost: the inverse of every dependency list, kept by `setDependencies`; a mark per derived node; every transitive dependent of a changed source marked on every write and walked again by the pull of an observed location above it.
- (b) 3a with heights per derived node and `Stale` marks. Rejected by the user: it recomputes the needed derived nodes inside the write. A write queues each needed derived node (observed, or read by a needed one) that read a changed source, drains the queue lowest height first, and queues a derived node's direct dependents only when its result changed; a derived node nobody needs, a retained one included, is marked `Stale` with its dependents and recomputed when read. Cost: the inverse; a state, a needed count, and a height per derived node; height adjustment after a recomputation reads a derived node at or above its own height; no cutoff on a read of a `Stale` derived node; an abandoned partial walk and a re-run in a higher bucket after an upward read during the drain.
- (c) 2, pico's pull with epochs. Cost: every live edge under the observed roots compared on every write, and a tag and epoch on every server value; benefit: no inverse to keep.
- (d) 5, full re-run per write. Cost: every live resolver on every write, and no equality cutoff below the roots.

Not approved for implementation.

### 3. Eager computation for retained queries nobody observes

- (a) Every retained root is brought up to date after every write. Cost: every retained query's affected resolvers run on every write, rendered or not (a prefetched route, a screen kept retained after navigation); under held 3b, `settle` would pull every held root as well as every observed derived node.
- (b) A write pulls only observed derived nodes; a retained root nobody observes stays marked after a write that reaches it and is pulled when `listen` attaches, in the subscribe call, not in render (held, the user's choice). Cost: the first render of a newly observed marked location reads the last computed result, and `useSyncExternalStore` renders again after `listen` brings it up to date.
- (c) Per retention, a choice between (a) and (b). Cost: an option on `normalizeData`.
- (d) (b), and `snapshot` pulls a marked derived node before it returns. A write between the retention and the mount (or between two mounts) marks the location, and under (b) the first render reads the result from before that write, then `listen` pulls it and `useSyncExternalStore` renders a second time. Under (d) the first render reads the current result and `listen` finds the node `Clean`, so there is one render. Cost: a render computes when the location is marked, which the Goal and R1 exclude (`snapshot` is a lookup), and which is the deviation from upstream the Goal gives its reason for: resolver work on the render path, once per marked location per render. A `Clean` location is still a lookup, and the work moves from `listen` to the render rather than adding to it. `valueOf` gains a `bringUpToDate`, so `snapshot` is no longer pure while the node is marked; React requires `getSnapshot` to return the same value while nothing changed, which holds because the pull ends `Clean` and later calls return the stored value. Under held 3b a marked derived node whose inputs come back equal is verified without recomputing and returns the same object. Open decision 28 (b) depends on this option. Listed as an option; (b) is the user's choice.

### 4. Where derived nodes live

- (a) In `environment.graph`, beside the store on the environment; records keep store.md's untagged values (held, with open decision 12). Cost: `garbageCollectEnvironment` deletes records from the store and then their derived nodes from the graph, one `deleteSource` per record.
- (b) On the record, under the resolver store key, with every value tagged `Server` or `Resolver`. Cost: a tag object per server value, a narrowing in every server-value reader, and two broken-invariant throw sites (a derived node under a server store key and the reverse). Under candidate 2 the tag also carries the epoch, and this option is the one that candidate needs.

### 5. A resolver that throws

- (a) Stored as `ResolverThrew`, propagated to parents like `MissingData` (held). Reason: `normalizeData` runs resolvers (the new root's, and every observed derived node's).
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
- (c) Writes only: `normalizeData` and `garbageCollectEnvironment` throw while a computation runs, and `readStoredResult` does not. Cost: the phase field of (b), and an inner `readStoredResult` still records no dependency on the outer derived node.
- (d) A write counter. `RecomputationGraph` gains `writes: number`, which `markChanged` and `deleteSource` increment; `compute` reads it when the computation starts, and when it differs at the end, `bringUpToDate` leaves the node `Dirty` after `setDependencies` instead of setting it `Clean`, so the next pull recomputes it. Cost: a counter on the graph, and `markChanged` takes the graph (open decision 29 (a) drops that parameter). It repairs the stale `Clean` node and nothing else: the inner write's `settle` still pulls observed derived nodes in the middle of the outer computation, which can pull the outer node itself while it computes.
- (e) After `setDependencies`, `bringUpToDate` leaves the node `Dirty` when any recorded `Derived` dependency is not `Clean`. Cost: one check per dependency per pull. It does not cover a source the outer computation read directly before the inner write changed it: the outer node is not yet in that source's dependents, and no derived node records the change. Not sufficient alone.

The review's scenario holds under held 3b. A derived node `P` is `Dirty` and recomputing; its computation has pulled a `Clean` child `C` and recorded it in `compute`'s local list. `P`'s resolver, through a closure, calls `normalizeData` with a change to a source `C` read. `markChanged` marks `C` `Dirty` and `C`'s dependents `Check`. `P` is not among them when `P`'s previous computation did not read `C`: `setDependencies` adds `P` to `C.dependents` only after `compute` returns. `P` finishes, records `C`, and `bringUpToDate` sets `P` `Clean`, so a `Clean` derived node reads a `Dirty` one, against change 2's invariant. A later write to `C`'s sources calls `markDirty(C)`, which marks no dependent of a node already marked, so `P` is never marked and returns its value until a source `P` read directly changes. When `P`'s previous computation did read `C`, `P` is in `C.dependents` and already `Dirty`, and the same ending holds. The inner `normalizeData`'s `settle` also pulls the observed derived nodes during `P`'s computation; one that reads `P` pulls `P`, whose computation runs again inside itself. (c) and (b) prevent the scenario; (d) repairs the stale mark only.

pico guards writes, not reads, as (c) does: `Database::set`, `remove`, `remove_singleton`, and `run_garbage_collection` call `assert_empty_dependency_stack`, which panics with "Cannot modify database while a memoized function is being invoked." when a memo is running (`crates/pico/src/database.rs:151-170`, the assertion at `:193-198`). A memo that calls another memo is a read, recorded on pico's dependency stack, which is ambient state. Here a computation's reads are recorded only through its `ComputeContext`, a parameter (change 2, The engine), so an inner `readStoredResult` from a closure is unrecorded, where pico's would be recorded; that is why (b) guards reads as well.

Callbacks are outside this decision. Level 2 callbacks run in `notify`, after `settle` pulled every observed derived node, and a callback that calls `normalizeData` runs a nested write with its own settlement and notification (Reads during recalculation). A Step A callback runs in `callSubscriptions`, after `normalize` returned. The effects of Values with lifecycles are set up in `reconcileEffects` and `observe` and torn down in `flushTeardowns`, after `notify`. None runs while a computation runs, so none is reentrant into level 1. A computation's call of `readStoredResult`, `normalizeData`, or `garbageCollectEnvironment` is the only reentrant path.

Not held.

### 9. Callback errors

Settled: (c), the landed rule. `callSubscriptions` calls each callback and listener through `reportCallbackError`, which never throws (`libs/isograph-runtime/src/subscribe.ts:96-118`, `:157-167`), and network-and-subscriptions.md settles it (Decisions, 3. A callback that throws, (a); its Deltas from Step A replace this decision's former (a)). Change 4's `notify` and the later `reconcileEffects` and `flushTeardowns` follow it.

- (a) Call every callback, then rethrow one error or an `AggregateError`. Cost: the `normalizeData` whose response was written throws for an observer's failure, so the request that called it becomes `Err`, a failure the network did not have (network-and-subscriptions.md, Decisions, 3 (c)).
- (b) Propagate the first throw. Cost: later observers miss the change until the next write.
- (c) Call each callback through `reportCallbackError`: a throw is rethrown in its own microtask, so it reaches the host's uncaught-error handler, and the loop continues. Cost: the error surfaces with no stack frame of the write that caused it, and a test that expects it stubs `queueMicrotask`.

### 10. Step A

Taken: (a). Step A landed as network-and-subscriptions.md's `readAndSubscribe`, `subscriptionSnapshot`, `listenToSubscription`, and `callSubscriptions` (`libs/isograph-runtime/src/subscribe.ts`), and network-and-subscriptions.md records this decision as taken (`refactors/pending/runtime/network-and-subscriptions.md:1597`).

- (a) Ship it before change 1, for a React demo.
- (b) Skip it. Cost: no React demo until change 4.

### 11. When a derived node no root reaches is deleted

- (a) By `garbageCollect`'s sweep (held, change 3). Under held 3b such a derived node is marked by the first write that reaches it, and later writes stop at it, so it costs no recomputation until then. Matches v1's caller-driven collection and Follow-ups 7's deferral.
- (b) By reference count: a derived node's count is its `rootCount` plus `dependents.size`; `dispose`, the stop function of `listen`, and `setDependencies` decrement it, and at zero the derived node is disposed and `setDependencies(graph, node, [])` decrements its dependencies in turn. Cost: a count kept on three paths, and deletion in the middle of a write. It is what constructors and destructors need (Follow-ups 1).

### 12. Coupling of the recomputation engine to the store

The analysis is The engine, the store, and the environment.

Settled: (a).

- (a) A store-agnostic engine over opaque `SourceId`s and `DerivedNodeId`s, held on the environment beside the store. `Store` stays store.md's nested map; `normalize` returns its changes and `garbageCollect` its deleted records; the walk reports reads through `ReadHooks`; `normalizeData`, `garbageCollectEnvironment`, and `readStoredResult` forward both to the graph. Derived nodes are owned by their record's presence source, so they are freed with the record. Keeps: v1's store code and tests, tests that compare the store directly, and upstream's environment shape. Cost: two id strings per server read and one per resolver read, a presence edge per (derived node, record read), the `owned` map, and every function that needs both the store and the graph takes the environment.
- (b) Derived nodes and dependents keyed by record and store key, in fields of a `Store` that becomes an object. Not taken: the engine names records and store keys; a layered store must translate its changes into the base layer's shape, which has no case for removing a layer; and every v1 function and test reads `store.records` where it read `store`.

### 13. Values with lifecycles

The analysis is Values with lifecycles.

- (a) Computation and effect are split (held, the user's direction). A computation is pure and runs on every pull, in any world and in any render. A resource the value needs is the node's effect: set up at mount commit (`observe`), torn down on unmount, on disposal, or when a write gives the node a new value, after the new effect is set up; teardowns are deferred until after notification and flushed in order; a `===` value leaves the effect as is. After a write the runtime pulls the mounted derived nodes with an effect and reconciles their effects (`reconcileEffects`). A `setUp` that throws is reported and leaves the effect `NotSetUp` (held); a teardown that throws is reported and does not stop the flush (held, open decision 9). Cost: a resource exists only while its node is mounted, so a retained query nobody renders acquires nothing. That conflicts with Follow-ups 1, the user's note, whose constructor runs at normalization time and whose lifetime follows retain and release rather than render.
- (b) The engine constructs inside the computation: a `Disposable` computation returns an `ItemCleanupPair`, the engine holds it, and releases it when the value is replaced or the node is disposed. Not held: a computation runs in every pull, including one a discarded concurrent render makes and, with forks, one in a world that is never committed, so it would acquire a resource whose release no commit or unmount triggers.
- (c) Lifecycles outside the engine: a disposable value lives in a registry the resolver's code calls, and the engine stores only the handle. Cost: nothing releases a handle when the derived node's value is replaced or the node is disposed, so the registry needs its own reachability, which is the engine's again.

Not decided: the user's choice.

### 14. Heights and eager client pointers

Settled: no heights under held 3b (change 4, No heights). An eager pointer (reader-ast.md, Client pointers, Later) reads client fields on its target: `field Pet.Card { bestFriend { Card } }`, with `bestFriend` a pointer, is not an `InlineCycle`, and the derived node of `Card` on `Pet:1` reads the derived node of `Card` on `Pet:2`. That read is one more `Derived` dependency, and the pull brings the derived node on `Pet:2` up to date before the one on `Pet:1` uses it. A cycle of derived nodes through the pointer is open decision 18.

### 15. Cutoff on a pull of a marked derived node

Open. Held 3b, as the user stated it for change 4, puts the cutoff on the pull: a marked derived node that is read brings its dependencies up to date first, and when none of them changed value it is verified without recomputing. That is (b). (a) is recorded as the user's specification and conflicts with it. This doc does not settle the conflict; changes 2 and 4 are written for (b).

- (a) A marked derived node recomputes when read, its reads bringing the marked derived nodes below it up to date first (the user's specification). Under held 3b every derived node, observed or not, is brought up to date by a pull, so (a) removes the cutoff everywhere, not only on reads of derived nodes nobody observes: after `Rex` to `Ray` under an observer at `BadgePage`, `settle` recomputes all five derived nodes where (b) recomputes `Initial` alone, and the observer is called because `BadgePage`'s resolver returns a new object. The mark needs one non-`Clean` value, `Check` and `checkDependencies` go, and `recompute` marks no dependent.
- (b) Two marks, `Dirty` and `Check`. A pull of a `Check` derived node brings its `Derived` dependencies up to date in recorded order and recomputes it only when one of them recomputed to a value `valuesEqual` rejects; `recompute` marks its direct dependents `Dirty` (change 2, The engine). Cost: one more mark value, the dependency walk on pulls, and a marking during the pull.

### 16. How the engine knows a derived node is needed

Closed: under held 3b the engine does not decide, for a marked derived node, whether something needs it. Marking marks every reader, observed or not, and `settle` starts from the observed ids in `graph.observers` (and, with Values with lifecycles, the mounted derived nodes with an effect), one step per observer, as `notify` does. A count per derived node (the observers at its location plus its needed dependents), an upward walk per invalidated derived node, and a per-write marking of the needed region were 3a's ways to decide it in `O(1)` per invalidated node; 3b has no decision to make.

### 17. A response that throws part-way

`normalize` writes in place and throws on the first position that does not match the AST (store.md, Errors), so the store holds the writes made before the throw.

- (a) `normalize` catches the throw and rethrows a `PartialWriteError` carrying the changes made before it and the original error as `cause`, with the original message; `normalizeData` marks those changes, and from change 4 settles and notifies, then rethrows (held, change 1, change 2, and change 4's Settlement). Reason: a derived node that read a value written before the throw is marked, as it would be for a write that did not throw. Once the graph lands (change 2b), only `normalizeData` is exported from the package index, and `normalize` is internal: a module export that `environment.ts` and the tests import (held). Reason: a write through `normalize` alone marks nothing, so every derived node that read what it changed returns its old result with no error.
- (b) No wrapping: the changes are lost with the throw. Cost: every derived node that read a value written before the throw returns its old result until another write reaches the same source, with no error.
- (c) `normalize` validates the whole response against the AST before it writes. Cost: a second walk of every response, and `recordLink`'s checks run in both walks.
- (d) On any throw, `normalizeData` marks every derived node. Cost: every live derived node is marked after one malformed response, and every observed one recomputes.

### 18. A cycle of derived nodes

Changes 1 to 4 cannot form one (Ordering, Cycles). An eager client pointer can: with `field Pet.Card { bestFriend { Card } }`, `Pet:1.bestFriend` `Pet:2`, and `Pet:2.bestFriend` `Pet:1`, the derived node of `Card` on `Pet:1` reads the derived node of `Card` on `Pet:2`, which reads the derived node on `Pet:1`. The pull reaches it: a marked or absent derived node's computation reads a derived node that is computing, and `bringUpToDate` would recompute that derived node again inside its own computation; `checkDependencies` of a `Check` derived node on the cycle recurses the same way. Neither has a check today; both recurse without end. Marking terminates on a cycle, because it stops at a derived node already marked. Incremental fails on a cycle with "adding edge made graph cyclic" (`ensure_height_requirement`, `src/adjust_heights_heap.ml` of `janestreet/incremental`).

Not decided; it arises with client pointers.

### 19. An upward read during `settle`

Closed: the question does not arise under held 3b. Under 3a as written, with this decision's (a), an upward read during the drain stored a result computed from `FriendCard` on `Pet:1`'s value from before the write as `Clean` until `FriendCard`'s own change requeued the reader. It was 3a's: a derived node recomputing in its height's bucket could read a `Clean` derived node at or above that height whose own input was still queued, and recompute from that derived node's result from before the write. Under 3b there are no buckets, and every read is a pull, which brings the derived node read up to date, its inputs first, before returning its value (Ordering).

### 20. Which request a request-state selection refers to

A request-state selection (The engine (2a), after `ComputeContext`) reads the state of one request, and the walk reports `requestSource(key)` for it. The question is how the selection determines `key`.

A derived node is keyed by (field, record, variables) (`resolverDerivedNode(root, resolverStoreKey(artifact.selectableName, variables))`), not by request. The same resolver on the same record, reached from two entrypoints, is one derived node with one value. "The request that fetched me" has no single value for that derived node, and keying the selection that way would split the derived node per request, which breaks that sharing.

A key that the reader's own location determines works. A loadable field's request is such a key. The loadable query of `P.f @loadable` is the entrypoint query of `entrypoint P.f` with `P`'s wrapper, whose wrapper variable is the parent record's id (graphql-protocol/node-entrypoints-and-loadable.md, `@loadable` later), and the request cache keys a request on the query text and the operation variables, `entrypoint.queryText + JSON.stringify(stableCopy(variables))` (network-and-subscriptions.md, The request cache, `NetworkRequestKey`; the same key is `RequestKey` in its Request entries in the store). That key is a function of (record, field, arguments), all of which the reader holds at the selection.

The state of an entrypoint's own request stays with the caller of `fetchEntrypoint`, which holds the returned `PromiseWrapper`; in React it reaches a component through the suspension derived node (Other derived node kinds), not through a selection.

Not decided.

### 21. The root hold of a query served from the store whose root record is absent

`fetchEntrypoint` with `shouldFetch` `No` calls `retainQueryWithoutMakingNetworkRequest` whatever the store holds (`libs/isograph-runtime/src/network.ts:89-94`), so `retainEntrypoint` can run while `ast.root` is absent (change 3, Serving from the store). `readStoredResult` and `contextHooks` create no derived node on an absent record; they record `recordSource` and return `noRecord` (change 2, The store's side). `retainRoot` has no such branch.

- (a) Create the root derived node anyway. Its computation is `resolverDefinition`'s: `readData` reports `readRecord(ast.root)` and returns `noRecord`, so the node holds `MissingData` naming the root and depends on `recordSource(ast.root)`. The first `normalize` that writes the root reports `RecordCreated` for it (change 1, `recordAt`), which marks the node `Dirty`. Held, not decided. Reason: the retention then has a node at its `location` from the call on, so a `listen` there observes it and a later write's `settle` pulls it as for any retention. Cost: a derived node whose owner record is absent, the one exception to "no derived node on an absent record"; `deleteSource` does not reach it until the record exists and is deleted, and `collect` disposes it once no root holds it.
- (b) Hold no root while `ast.root` is absent: `retainEntrypoint` adds the `Retention` to `environment.retainedQueries` and its `releaseRoot` does nothing. Cost: the location has no node; a later `normalizeData` of another entrypoint that writes the root creates none, so the first read of the location computes on demand, and a `listen` there only registers.

### 22. A throw inside a computation outside its resolver

`callResolver` catches only the resolver's throw (change 2, Results, equality, and a resolver that throws). The walk that `resolverDefinition`'s computation runs before it throws on a broken store invariant: `linkAt` on a non-link under a composite store key (`libs/isograph-runtime/src/store.ts:74-87`, the throw at `:81`, reached from `readLinkedValue`, `read.ts:226`), and `storeKey`'s `variableValue` on a missing variable (`storeKey.ts:56-63`, the throw at `:59`, reached from `readSelection`, `read.ts:131` and `:133`). Step A's read catches both as `ReadErr` (`readSubscription`, `subscribe.ts:120-131`); the stored read catches neither.

Under 3a this lost the node: `bringUpToDate` removed it from its bucket before recomputing, so a throw such as `linkAt`'s at `store.ts:81` on a non-link under `Pet:1.bestFriend` left it `Queued` in no bucket; `invalidate` skipped it because it was not `Clean`, `settle` never reached it, and `snapshot` returned its value from before the corruption forever. Held 3b removes that: there is no bucket, and the rule change 2's engine keeps is that any throw that propagates out of a pull leaves the node marked with its previous dependencies, never `Clean`, and the next pull retries it (change 2, The engine; its Tests cover it).

Under held 3b, a throw out of `compute` propagates out of `recompute` before `setDependencies` records the new reads, and out of `bringUpToDate` before it sets the mark `Clean`. The derived node keeps its mark, its previous value, and its previous dependencies, and the next pull of it (a read, or the next write's `settle`) recomputes it again. A checking ancestor whose `checkDependencies` was running keeps its mark too. A throw out of `createDerivedNode`'s `compute` stores nothing. Either propagates out of `readStoredResult`, `retainRoot`, or `observe`. A throw during `settle` stops its loop: the observed derived nodes after it stay marked, and `normalizeData` skips `retainEntrypoint` and `notify` and throws.

- (a) `compute` catches any throw from the computation and stores it as the derived node's value, as the subscription path stores `ReadErr`. Held, not decided. Reason: `bringUpToDate` returns normally, so the node ends `Clean`, `settle` continues, and `notify` runs; the dependencies the computation recorded before the throw are kept, so a write that repairs one of them recomputes the node. Cost: an engine bug in a computation is also stored as a value instead of surfacing as a throw. It departs from the rule the body keeps: the node ends `Clean` holding `Err`, and it is retried only when a source or derived node it recorded before the throw changes, not on the next pull.

  ```ts
  // from libs/isograph-runtime/src/graph.ts
  export type RecomputationGraph<TValue> = {
    // .. change 2's fields, and:
    /** The value a derived node holds when its computation threw `error`: `{ kind: 'Err', error }`
     * in the runtime. */
    readonly thrownValue: (error: unknown) => TValue;
  };

  export function createGraph<TValue>(
    valuesEqual: (a: TValue, b: TValue) => boolean,
    thrownValue: (error: unknown) => TValue,
  ): RecomputationGraph<TValue>;

  // compute, before (change 2):
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
  // after:
    const value = runComputation(graph, computation, {
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

  /** `computation(context)`; a throw becomes `graph.thrownValue(error)`. */
  function runComputation<TValue>(
    graph: RecomputationGraph<TValue>,
    computation: Computation<TValue>,
    context: ComputeContext<TValue>,
  ): TValue {
    try {
      return computation(context);
    } catch (error: unknown) {
      return graph.thrownValue(error);
    }
  }
  ```

  ```ts
  // from libs/isograph-runtime/src/read.ts
  // before (change 2):
  export type ResolverResult<T> = ReadResult<T> | ReadResolverThrew;
  // after:
  /** A stored resolver result: v1's `ReadResult`, the resolver threw, or the read threw. */
  export type ResolverResult<T> = ReadResult<T> | ReadResolverThrew | ReadErr;
  ```

  `ReadErr` is the landed type of `subscribe.ts`, imported as a type. `createIsographEnvironment` passes `createGraph(resultsEqual, (error) => ({ kind: 'Err', error }))`; `resultsEqual` compares two `Err` by `===` of their errors, as `subscriptionResultsAreEqual` does; `contextHooks` and `storedResultHooks` return `ReadHooks<ReadResolverThrew | ReadErr>`, so a nested `Err` passes up the walk unchanged (`allOk`), as a nested `ResolverThrew` does; `SubscriptionResult` already admits both (change 2, Subscriptions). A nested derived node's throw is caught by its own `compute`, so the outer `compute` catches only throws of the outer computation's own walk. `ResolverThrew` stays: it names the field and record whose resolver threw, where `Err` is a broken invariant of the store. An abandon under open decision 19 (c) is a returned value, not a throw, so (a) does not catch it.
- (b) `bringUpToDate` restores the node and `settle` rethrows after pulling the rest. `bringUpToDate` catches the throw, sets the node `Clean` with its old value and dependencies, and returns the error to `settle`; `settle` pulls the rest of the observed derived nodes, and `normalizeData` runs `notify` and then rethrows the first error, or an `AggregateError` of several. Cost: the node holds a value no current input explains; `normalizeData` throws for a broken store invariant after a response was written, so the request that called it becomes `Err`; and the read path needs the same restore in `createDerivedNode` and `recompute`.
- (c) Propagate, and leave the node marked with its previous value and dependencies; the next pull retries it. This is the body as written (change 2, The engine). Cost: `settle` stops at the throw, so the observed derived nodes after it are not pulled and no observer is called for the write; the throw reaches the caller of `normalizeData`, so the request that called it becomes `Err`.

### 23. Two client fields of one name on an abstract entity and on a concrete entity

A resolver derived node's id is (record, `resolverStoreKey(artifact.selectableName, variables)`) (change 2, The resolver store key), and the record is the concrete one: `normalize` stores a record under the entity its response object names (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`). A client field can be declared on an abstract entity and read on a concrete record: `field Node.NodeLink` (`libs/isograph-runtime/fixtures/store/src/NodeLink.ts`) is selected under `node(id: $id)` (`fixtures/store/src/Links.ts`), whose target is `interface Node`, and is read on `Pet:1` when the response's `__typename` is `Pet`.

The compiler accepts `field Node.Label` beside `field Pet.Label` when `Pet` implements `Node`. Its one duplicate check is per (entity, selectable name): `selectable_definitions` counts the source declarations and the protocol's definition of `entity_name.selectable_name` (`crates/isograph_compiler/src/schema.rs:278-294`), and `selectable_named` reports "defined more than once" only for that pair (`:322-344`). A selection resolves on its parent's static entity, so `node(id: "1") { Label }` reads `Node.Label` and `pet(id: "1") { Label }` reads `Pet.Label` (`selectable_named(db, client, parent, name)`, `crates/isograph_compiler/src/merged_selection.rs:1105`, `:1321`). No check in `crates/isograph_compiler/src` relates a concrete entity's selectables to those of an abstract entity it implements. Both reads on `Pet:1` compute the id `resolverDerivedNode(Pet:1, "Label")`: the second read returns the first field's stored result, and `readStoredResult`'s invariant comment ("one selectable of one entity has one reader artifact") does not hold, because the record's entity is not the declaring entity.

The key format collides a second way. `resolverStoreKey` writes the selectable name, then `____<name>___<value>` per argument (`argumentChunks`, `libs/isograph-runtime/src/storeKey.ts:22-34`). A GraphQL name may contain `_`, so a client field named `F____n___2` read with no variables and `F` read with `{ n: 2 }` both have the key `F____n___2` on one record, and the second read returns the first field's stored result (change 2, The resolver store key, states the same property for server keys).

The body is wrong until this is decided: every `resolverStoreKey` and `resolverDerivedNode` in changes 2 to 4 is written with the selectable name and `argumentChunks` alone, so both collisions return one field's stored result for another field.

Upstream has both collisions. Its server store key is the field name followed by `____<name>___<value>` per argument (`getParentRecordKey`, `isograph/libs/isograph-react/src/core/cache.ts:454-479`; `FIRST_SPLIT_KEY` and `SECOND_SPLIT_KEY`, `:594-595`). It stores no resolver result, and its one per-(record, client field) key, `stableIdForFragmentReference`, is `` `${root.__typename}/${root.__link}/${fieldName}/${JSON.stringify(stableCopy(variables))}` `` (`FragmentReference.ts:75-79`, used by the component cache at `componentCache.ts:19-21` and the `startUpdate` cache at `startUpdate.ts:48-50`): it names the record's concrete typename and the field name, not the declaring entity, so `Node.Label` and `Pet.Label` on `Pet:1` share one key there too.

- (a) The resolver store key names the declaring entity. Held, not decided. The reader artifact carries it, and the key is `<declaring entity>.<selectable name>` and then the argument chunks: `Node.Label` and `Pet.Label` on `Pet:1` are two derived nodes. A GraphQL name cannot hold `.`, so the prefix of one field's key is never another field's. The body of this doc stays written with `selectableName` alone until this is decided; every key in it (`PetName`, `RecentCheckins____n___2`, `PetSummary____id___"1"`) gains its entity prefix under (a).

  ```ts
  // from libs/isograph-runtime/src/artifacts.ts
  // ReaderArtifact, change 2's after, and:
    /** The entity the client field is declared on: the parent entity, abstract or concrete. */
    readonly parentEntityName: EntityName;
  ```

  ```ts
  // from libs/isograph-runtime/src/storeKey.ts
  // before (change 2):
  export function resolverStoreKey(
    selectableName: string,
    variables: Variables,
  ): StoreKey;
  // after:
  /** `parentEntityName`, `.`, `selectableName`, then one `____<name>___<stableStringify(value)>`
   * chunk per entry of `variables`, names in code unit order. */
  export function resolverStoreKey(
    artifact: Pick<ReaderArtifact<ResolverParam, unknown, EntityName>, 'parentEntityName' | 'selectableName'>,
    variables: Variables,
  ): StoreKey;
  ```

  `reader_artifact_file` writes `parentEntityName: String(field.field.parent_entity_name)` before `selectableName`. Callers pass the artifact where they passed `artifact.selectableName` (`readStoredResult`, `contextHooks`, `retainEntrypoint`). Cost: a longer key per resolver read, and one more field per reader artifact.
- (b) The compiler rejects `field Pet.Label` when an abstract entity that `Pet` implements declares a client field `Label`, and the reverse. Cost: a validation that reads the protocol's implements relation (`NetworkProtocol::concrete_types`, used at `crates/isograph_compiler/src/artifact_types.rs:901-923`) for every client field declaration, and a rule upstream does not state.
- (c) Accept the collision. Cost: two different resolvers share one stored result on a record, with no error.
- (d) A key format that cannot collide, alone or with (a)'s declaring entity. Either a separator outside the GraphQL name alphabet (`[_A-Za-z][_0-9A-Za-z]*`): `` `${parentEntityName}.${selectableName}(${name}:${value},..)` ``, which a name cannot contain, with each value a JSON text, which is self-delimiting; or the JSON of a list, as `sources.ts` builds its ids (`JSON.stringify(['Record', record.entityName, record.id])`, change 2, The store's side): `JSON.stringify([parentEntityName, selectableName, entries])` with `entries` the variables as `[name, stableCopy(value)]` pairs in code unit order. Both are injective. Cost: the derived node key no longer shares the server key's format, and a JSON list is longer. The server `storeKey` keeps upstream's format; its collision is store.md's and not this decision's.

### 24. Level 2's lifetime against react.md's render-time subscription

react.md and the landed runtime create a component's subscription at render: `useSubscription` calls `readAndSubscribe` under a temporary retain at render and retains it permanently at commit (`refactors/pending/runtime/react.md:359`, `:864-877`; network-and-subscriptions.md, The two-phase lifetime, `:709-719`). Between render and commit the subscription exists, so every write that changes its read updates `delivered` and calls its callback (that section's step 2), and a render that never commits releases it when the temporary retain expires (step 4). React hands its listener over only at commit, so `listenToSubscription` is the commit-time half.

Level 2 has no render-time half. `observe`, the body of `listen`, runs in React's `subscribe`, at commit (change 4, `listen` and `snapshot`), and it is what puts the derived node among those `settle` pulls and adds to its `rootCount`. `snapshot` at render is a lookup that holds nothing. Between render and commit the location's derived node is pulled after a write only if another observer is at it, so a write in that window leaves it marked, and `listen` at commit pulls it and `useSyncExternalStore` renders again (open decision 3's cost). The derived node is kept through `collect` in that window only if a held root reaches it, and a derived node at a location no held root reaches is not. react.md settles part of the mapping: under change 4 `listen` and `snapshot` take the places of `listenToSubscription` and `subscriptionSnapshot`, and the hook keeps its shape (`react.md:905`, `:1292`). It settles nothing for `readAndSubscribe`'s render-time part.

Open: what level 2 offers at render in place of `readAndSubscribe` under a temporary retain. Candidates: no render-time step, the `listen`-at-commit behavior above; a render-time hold, an `ItemCleanupPair` that puts the location among those `settle` pulls and adds to its `rootCount` from render, under react.md's temporary retain, with `listen` adding only the callback at commit; or a render-time `rootCount` hold alone, which keeps the derived node through `collect` without `settle` pulling it. Not held: no doc settles it.

### 25. Who calls `garbageCollectEnvironment`

`garbageCollectEnvironment` runs only when a caller calls it: releasing a request unretains its query and does not collect (`refactors/pending/runtime/network-and-subscriptions.md:84`, `:531`), and the landed function is called by no other runtime function (`libs/isograph-runtime/src/environment.ts`). Under changes 2 to 4 it is also the only function that disposes derived nodes (`deleteSource` and `collect`, change 3, Garbage collection), so a derived node a plain `readStoredResult` stored outside any retention and subscription, one that only an unsubscribed subscription read, the derived nodes of a disposed retention, and those of a pointer whose target moved live until the next call, marked or `Clean` (open decision 11 (a)). When the React package calls it is react.md's open decision 1 (`refactors/pending/runtime/react.md:1297-1305`: after every disposal, never automatically, or upstream's buffer of 10), not held. This doc adds no option of its own; its open decision 11 (b) would dispose derived nodes without a call, by reference count.

### 26. Whether `readDataForOnComplete` reads through the graph

`readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:195-218`) gives `onComplete` the entrypoint's value after `makeNetworkRequest` normalized the response. It calls `read` with v1's hooks, so it calls the entrypoint's resolver and every nested resolver once more per response, a nested resolver's throw propagates out of it and makes the request `Err`, and `MissingData` throws "This is indicative of a bug in Isograph.". Upstream's `readDataForOnComplete` does the same: `readButDoNotEvaluate`, then the resolver (`isograph/libs/isograph-react/src/core/makeNetworkRequest.ts:362-420`). Under change 2 the subscriptions' re-reads read nested resolvers through the graph (change 2, Subscriptions), so the value `onComplete` receives holds nested values that are not `===` the ones the subscriptions deliver for the same response.

- (a) Unchanged: `read`. Held, not decided, because it is upstream's behavior. Cost: every resolver the entrypoint reaches is called once more per response with `onComplete`, and the nested values `onComplete` receives are separate objects from the stored results.
- (b) `readData(environment.store, storedResultHooks(environment, () => {}), ..)`, then the entrypoint's resolver; from change 3 `storedResultHooks` takes `onReadDerivedNode` (change 3, Garbage collection), and `readDataForOnComplete` keeps nothing it reports. Nested resolvers are read through the graph and keep their stored identity; the entrypoint's resolver is called once per response. Cost: a nested `ResolverThrew` is a value in the walk's result, so `readDataForOnComplete` rethrows its `error` to keep the request's `Err`, or `onComplete`'s type admits it.
- (c) `readStoredResult(environment, entrypoint.readerArtifact(), entrypoint.normalizationAst.root, variables)`. The entrypoint's result is stored as the root derived node. From change 3 it is the derived node `normalizeData`'s `retainEntrypoint` already computed and holds, at the returned `Retention`'s `location`, so the read is a lookup. Cost: `ResolverThrew` and `MissingData` are values, so `readDataForOnComplete` rethrows the first's `error` and throws on the second, as it throws today; in change 2 the root derived node it stores is disposed only when `garbageCollectEnvironment` deletes `Query.__ROOT`.

### 27. How `collect` keeps the derived nodes subscriptions read

A subscription is not a derived node and holds no root hold, so without a rule `collect` (change 3, Garbage collection) disposes every nested derived node that only a subscription reads, the next re-read recomputes it, and a resolver that returns a new object makes that re-read unequal and calls the callback with data that did not change. The user's direction is that `collect` treats the derived nodes current subscriptions read as roots. Upstream stores no resolver results, so it has no such rule; its subscriptions record what their last read visited (`encounteredDataAndRecords`, `isograph/libs/isograph-react/src/core/IsographEnvironment.ts:49`, replaced at `subscribe.ts:160`) and use it to skip re-reads, not to keep data.

- (a) Each `Subscription` records `readDerivedNodes: ReadonlySet<DerivedNodeId>`, the ids of the nested derived nodes its last read read directly, which `storedResultHooks` reports through `onReadDerivedNode`. `readAndSubscribe` sets it and `callSubscriptions` replaces it on every re-read. `garbageCollectEnvironment` passes every current subscription's ids to `collect` as `extraRoots`. Held, not decided. Reason: it is upstream's shape (a set filled by the read and replaced on the next one); unsubscribing needs no release, because `collect` reads `environment.subscriptions` when it runs; a throw part-way through a read leaves nothing to balance. Cost: a `Set` per subscription read, one insertion per `Resolver` node the subscription's reader AST walks, an array per `garbageCollectEnvironment`, and a second parameter on `collect`. Under open decision 11 (b) it does not suffice: a reference count that reaches zero disposes the node in the middle of a write, before any `collect`, so a subscription-only node would need (b)'s holds.
- (b) Each subscription holds one root hold (`rootCount`) on each nested derived node its last read read directly. A re-read takes holds on the new set and releases the holds of the old one; the unsubscribe cleanup releases them. `collect` is unchanged. Cost: a new engine function that adds a root hold without `retainRoot`'s bring-up-to-date (the walk already brought the node up to date), a hold and a release per nested derived node per subscription per write, a cleanup list per subscription, and holds left unbalanced when a read throws part-way unless the re-read releases in a `finally`. Benefit: `rootCount` stays the one root mechanism, and it carries over to open decision 11 (b) and Follow-ups 1.
- (c) `garbageCollectEnvironment` walks each current subscription's reader AST from its root link with hooks that report each nested resolver's id, and passes the ids to `collect`. Nothing is recorded on the subscription. Cost: every subscription's reader AST is walked once more per `garbageCollectEnvironment`; the walk's `readResolver` hook must return each nested resolver's value, and `readStoredResult` pulls a marked one, so collection calls resolvers; and the ids can differ from what the subscription's `delivered` holds when a write has not yet been followed by `callSubscriptions`.

### 28. Observers: pull, then call back, or call back, then the reader pulls

Under held 3b a write only marks. Something must pull an observed derived node before its observer sees the new value, and the observer must learn that it may have changed.

- (a) Pull, then call back. Held, not decided. `settle` pulls every observed derived node before the write returns, and `notify` calls each observer whose value is not `===` its `notified` (change 4). Subscriptions do the same through `callSubscriptions`, which re-reads each one and calls its callback with the new `SubscriptionResult` only when `subscriptionResultsAreEqual` rejects it against `delivered`. Reason: it keeps R1 and the Goal ("Renders do no work": `snapshot` is a lookup) and open decision 3 (b), the user's choice, which puts the pull in `listen` and not in render; and it keeps the landed subscription API, whose `callback: (result: SubscriptionResult) => void` (`libs/isograph-runtime/src/subscribe.ts`, `readAndSubscribe`) needs the result before the subscription decides whether to call back. Cost: every observed derived node a write marks is pulled inside the write, including one whose component will not re-render soon (an offscreen tree, a deferred one), and the write's caller pays for it.
- (b) Call back, then the reader pulls. After marking, `notify` calls each observer whose derived node is not `Clean`, which means its value may have changed; the reader pulls when it reads. React's `getSnapshot` pulls the derived node; a marked node whose dependencies come back equal is verified without recomputing and returns the same object, so React does not re-render. The runtime itself pulls only the mounted derived nodes with an effect (Values with lifecycles). Costs:
  - `snapshot` computes when its derived node is marked. That is open decision 3 (d), and it conflicts with R1 and the Goal, which record the user's deviation from upstream ("no read-and-subscribe during render"), and with 3 (b), the user's choice. A pull during a render React discards leaves its computed values stored; they are correct for later reads, because computations are pure (Values with lifecycles).
  - An observer is called for every write that marks its derived node, including one whose pull cuts off and changes nothing. React then calls `getSnapshot`, receives the same object, and skips the render. A non-React observer that does work in its callback does that work for nothing unless it pulls and compares first.
  - Subscriptions. `callSubscriptions` decides whether to call back by re-reading and comparing with `delivered`, so it cannot call back before reading. Either subscriptions keep (a) while level 2 takes (b), or the subscription API changes: the callback becomes `() => void`, `subscriptionSnapshot` re-reads a subscription that was called back and has not read since, keeping `delivered` when `subscriptionResultsAreEqual` holds, and every subscriber that needs the result calls `subscriptionSnapshot`. `readAndSubscribe`, `listenToSubscription`, and react.md's `useSubscription` change with it.
  - Open decision 27 (a) holds under both. `readDerivedNodes` is what the last read went through; under (b) the last read is the last `subscriptionSnapshot`, not the last write. Between a write and that read it names the previous read's derived nodes, and `collect` keeps them.
  - The notify loop finds the marked observed derived nodes by iterating `graph.observers`, as (a)'s does.

The body is written for (a).

### 29. Sources as cell objects

A source is today an opaque `SourceId` string that `sources.ts` builds per read, and the engine finds a source's readers in `graph.sourceDependents: Map<SourceId, Set<DerivedNode>>` (change 2, The engine). The alternative stores each source as an object that carries its own readers, and the engine uses that object's identity.

- (a) Each source is a cell object that carries its dependents. Held, not decided. A record field is stored as a `ValueCell`, `{ contents, dependents }`, instead of a raw value; a record's presence is a `RecordCell`, which also holds the derived nodes the record owns. The engine reads `cell.dependents` where it read `sourceDependents.get(id)`, and no id is built on a read. A deleted record drops its value cells, and with them their dependents sets. The body of this doc stays written for (b) until this is decided.
- (b) `SourceId` strings and `sourceDependents`, as the body is written. Cost: two strings built per server read and a `Map` lookup per id in `setDependencies` (The engine, the store, and the environment, Costs of the ids), a `sourceDependents` entry per location read, deleted only when its last reader recomputes, and the `owned` map.

Explicitly rejected, and not an option: storing derived nodes inside store records. Derived nodes stay in `graph.derivedNodes` under their `DerivedNodeId` (open decision 4 (a)); under (a) only sources move into the store.

The engine's types under (a):

```ts
// from libs/isograph-runtime/src/graph.ts
/** A value a computation can read. The engine holds the source's readers on it and never reads its value. */
export type Source<TValue> = {
  /** The derived nodes whose last computation read this source. Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

/** A source whose deletion disposes the derived nodes created with it as owner: a record's presence. */
export type OwnerSource<TValue> = Source<TValue> & {
  /** The derived nodes created with this owner. Written only by `createDerivedNode` and `disposeDerivedNode`. */
  readonly owned: Set<DerivedNode<TValue>>;
};

/** The recomputation graph: derived nodes by id. */
export type RecomputationGraph<TValue> = {
  readonly derivedNodes: Map<DerivedNodeId, DerivedNode<TValue>>;
  /** Whether two values of a derived node are equal, for cutoff: `resultsEqual` in the runtime. */
  readonly valuesEqual: (a: TValue, b: TValue) => boolean;
};

export type DerivedNode<TValue> = {
  readonly id: DerivedNodeId;
  /** The source whose deletion disposes this derived node: the presence of the record it was computed on. */
  readonly owner: OwnerSource<TValue>;
  readonly computation: Computation<TValue>;
  /** The value, mark, and dependencies of the last computation. Read and written only through `stateOf`. */
  readonly state: DerivedNodeState<TValue>;
  /** Every derived node whose dependencies hold a `Derived` dependency on this one. Written only by `setDependencies`. */
  readonly dependents: Set<DerivedNode<TValue>>;
};

export type GraphDependency<TValue> = SourceDependency<TValue> | DerivedNodeDependency<TValue>;
export type SourceDependency<TValue> = { readonly kind: 'Source'; readonly source: Source<TValue> };
export type DerivedNodeDependency<TValue> = { readonly kind: 'Derived'; readonly node: DerivedNode<TValue> };

export type ComputeContext<TValue> = {
  /** Records that the computation read `source`. */
  readSource(source: Source<TValue>): void;
  /** Pulls the derived node `id`, creating it from `create()` when absent, records the dependency,
   * and returns its value. */
  readDerivedNode(id: DerivedNodeId, create: () => DerivedNodeDefinition<TValue>): TValue;
};

/** What creating an absent derived node needs. */
export type DerivedNodeDefinition<TValue> = {
  readonly owner: OwnerSource<TValue>;
  readonly computation: Computation<TValue>;
};

/** Marks `Dirty` every derived node that read one of `sources`, and `Check` every `Clean` derived node
 * that reads one of those, transitively. Runs no computation. */
export function markChanged<TValue>(sources: ReadonlyArray<Source<TValue>>): void {
  sources.forEach((source) => source.dependents.forEach(markDirty));
}

/** `markChanged([owner])`, then marks `Dirty` the dependents of every derived node `owner` owns, and
 * disposes those derived nodes. */
export function deleteSource<TValue>(graph: RecomputationGraph<TValue>, owner: OwnerSource<TValue>): void {
  markChanged([owner]);
  owner.owned.forEach((node) => node.dependents.forEach(markDirty));
  owner.owned.forEach((node) => disposeDerivedNode(graph, node));
}
```

The store's types under (a):

```ts
// from libs/isograph-runtime/src/store.ts
/** Every record cell: `EntityName`, then record id. A cell is kept while it has a reader, whether the
 * record is present or not. */
export type Store = Map<EntityName, Map<RecordId, RecordCell>>;

/** A record's presence, the derived nodes that read it, and the derived nodes it owns. */
export type RecordCell = OwnerSource<ResolverResult<unknown>> & {
  /** The record's value cells, or `ABSENT` when a reader read the record while the store did not hold it. */
  values: StoreRecord | typeof ABSENT;
};

/** One record's value cells by store key. */
export type StoreRecord = Map<StoreKey, ValueCell>;

/** One value of a record and the derived nodes that read it. */
export type ValueCell = Source<ResolverResult<unknown>> & {
  /** The value, or `MISSING` when a reader looked the key up and the record held no value (missing data). */
  contents: StoreValue | typeof MISSING;
};

/** A record cell that only readers created: the store does not hold the record. */
export const ABSENT: unique symbol = Symbol('Absent');

/** A value cell that only readers created: the record holds no value under the key. */
export const MISSING: unique symbol = Symbol('Missing');
```

`ABSENT` and `MISSING` are symbols, so no JSON value equals them and a `===` test narrows `values` and `contents` with no cast. A cell for a record or key that is not in the store exists because a computation that found it absent must be marked when it appears; that is what today's `recordSource` and `valueSource` ids of an absent record or key are, kept in `sourceDependents`.

What each use of `SourceId` in this doc becomes under (a):

- `SourceId` and its brand (The engine (2a)): removed; `Source<TValue>` and `OwnerSource<TValue>`. `DerivedNodeId` stays: derived nodes are still found by id (`graph.derivedNodes`, `graph.observers`, `retainRoot`, `Subscription.readDerivedNodes` of open decision 27, which are `DerivedNodeId`s and unchanged).
- `RecomputationGraph.sourceDependents`: removed; `Source.dependents`. `setDependencies` adds and removes `node` in `dependency.source.dependents`, and deletes no set: an empty set stays on its cell, and `garbageCollect` drops cells.
- `RecomputationGraph.owned`: removed; `OwnerSource.owned`. `createDerivedNode` adds to `definition.owner.owned`, and `disposeDerivedNode` removes from `node.owner.owned`.
- `DerivedNode.owner` and `DerivedNodeDefinition.owner`: `OwnerSource<TValue>`, the record cell. `resolverDefinition`'s `owner: recordSource(root)` becomes `owner: recordCell(store, root)`.
- `SourceDependency.source` and `ComputeContext.readSource`: take a `Source<TValue>`.
- `markChanged(graph, sources)`: `markChanged(sources)`; it touches only the sources and the derived nodes they reach, so it takes no graph.
- `deleteSource(graph, source)`: takes the `OwnerSource`; it still takes the graph, to remove disposed derived nodes from `graph.derivedNodes`.
- `sources.ts`: `recordSource(record)` becomes `recordCell(store, record): RecordCell`, which returns the store's cell or inserts an `ABSENT` one; `valueSource(record, storeKey)` becomes `valueCell(record: RecordCell & { values: StoreRecord }, storeKey): ValueCell`, which returns the record's cell or inserts a `MISSING` one; `changeSource` is removed (below); `resolverDerivedNode` stays. The casts that minted `SourceId`s go.
- `contextHooks` (change 2, The store's side): `readRecord` calls `context.readSource(recordCell(store, record))`, `readValue` calls `context.readSource(valueCell(..))`, and `readResolver` reads the root's record cell. `ReadHooks` keeps its parameters (links and store keys); the hook does a second lookup of the cell the walk already looked up, where today it builds a string.
- `normalizeData`'s `markChanged(graph, changes.map(changeSource))` and `writeOrMark`: see `normalize` below.
- `garbageCollectEnvironment`'s `deleteSource(graph, recordSource(record))`: `garbageCollect` returns the record cells it emptied, and `garbageCollectEnvironment` calls `deleteSource(graph, cell)` for each.
- `requestSource(key)` (The engine (2a), after `ComputeContext`; open decision 20): the request entry in `environment.requests` (network-and-subscriptions.md, Request entries in the store, `RequestEntry`) holds the cell; removing an entry calls `markChanged([cell])`.
- Other derived node kinds: a browser-owned value's mirror entry holds its cell, and each client-root selectable's provider entry holds its cell; the provider's subscription calls `markChanged([cell])`. A closure component's wrapper is owned by the record cell.
- Open decision 6 (b): the index of live records is each record cell's `dependents`.
- Open decision 12 (a): the engine still imports nothing from the store; the store's types import `Source`, `OwnerSource`, and `DerivedNode` from `graph.ts` and `ResolverResult` from `read.ts`.
- Open decision 21 (a): the root derived node on an absent root record is owned by that record's `ABSENT` cell, which its computation's read inserted; the first `normalize` that writes the root fills the cell's `values` and marks it.
- Open decision 27's ids are `DerivedNodeId`s and do not change.
- The engine's tests (change 2, Tests) build literal sources, `{ dependents: new Set() }`, where they used literal ids.

What `normalize` does under (a). The user's statement is that `normalize` writes `cell.contents` and calls `markChanged` of the cell. Held. `writeValue` gets or inserts the value cell, compares `contents` with `isUnchanged` (change 1), and on a change writes `contents` and calls `markChanged([cell])`; `recordAt` on a cell that is `ABSENT` sets `values` and calls `markChanged([cell])`. Consequences:

- `normalize` imports `markChanged` from `graph.ts`, so a store function depends on the engine. It needs no graph argument, because `markChanged` takes none.
- A write is marked as it happens, so a response that throws part-way is marked up to the throw with no help. `PartialWriteError.changes` and the `ReadonlyArray<StoreChange>` `normalize` returns (change 1) lose their one production reader, `normalizeData`'s `markChanged`; change 1's tests would be their only reader, and a value only tests read does not exist (AGENTS.md, Coding standards). `normalizeData` still catches a throw to run `settle` and `notify` before rethrowing (change 4). Open decision 17 (a) changes with it.
- The alternative is that `normalize` returns the changed cells and `normalizeData` calls `markChanged`, as the body's `changeSource` does with ids. That keeps change 1's return value, decision 17 (a), and a store that calls nothing in the engine.

Costs of (a):

- Memory. Every stored field is a `ValueCell` and its `Set`, two objects, where store.md and upstream store a plain value: upstream's `StoreRecord` is an object of `DataTypeValueScalar | DataTypeValueLinked` (`isograph/libs/isograph-react/src/core/IsographEnvironment.ts:150-158`), and store.md's is `Map<StoreKey, StoreValue>` (`libs/isograph-runtime/src/store.ts`). Creating the `Set` on the first reader would save the second object for a field no computation reads, at the cost of a nullable field.
- Every field is wrapped. A store where some fields are cells and some are raw values cannot tell a raw `JSON` scalar that holds `{ contents, dependents }` from a cell, so the wrapping is total.
- Every store reader unwraps. `check`, `read`, `readButDoNotEvaluate`, `garbageCollect`'s mark, and `normalize`'s comparison read `contents` and `values` and treat `MISSING` and `ABSENT` as absent. `garbageCollect`'s sweep sets a deleted record's `values` to `ABSENT` and keeps the cell while it has dependents or owned nodes; it deletes an `ABSENT` record cell and a `MISSING` value cell whose `dependents` and `owned` are empty. It cannot delete one with a reader: that reader found it absent, and a later write that creates it would make a new cell and mark nothing.
- Store snapshots and serialization unwrap. A test compares `unwrapStore(environment.store)` with `storeOf(..)`, where `unwrapStore` drops `ABSENT` records and `MISSING` values and replaces each cell by its contents; store.md's and reader-ast.md's tests change to compare that, and `toEqual` on the cells themselves would compare `dependents` sets. A store written out for hydration is unwrapped the same way, and one read in is wrapped.
- store.md's `Store` type changes, so The engine, the store, and the environment's "What this keeps" (store.md's `Store` type and its tests unchanged) no longer holds.

What (a) buys: no string building and no `sourceDependents` lookup per read; a deleted record drops its cells' dependents with it instead of leaving `sourceDependents` entries until each reader recomputes; and the `owned` index is a field of the record cell.

Upstream's optimistic layers, against (a). Upstream stores each layer's writes in that layer: a layer holds `data: StoreLayerData`, a sparse map from typename and id to a partial record of plain values (`IsographEnvironment.ts:165-169`; the layer types, `optimisticProxy.ts:100-159`); a read walks the layers from the child-most to the base and takes the first layer that holds the field (`getMutableStoreRecordProxy`, `optimisticProxy.ts:46-69`); a write goes to the child-most layer (`:89-95`). Under (a) a layered store keeps per-cell overrides: one `ValueCell` per (record, store key), holding the base value and each layer's value for it, and each layer keeping the set of cells it wrote. That deviates from upstream's per-layer record storage. Reason: a cell's `dependents` are the readers of the effective location, whichever layer answered (The engine, the store, and the environment, What the environment must report). With a cell per layer, a reader that read the base cell would not be in the dependents of the cell an optimistic layer creates for the same key, so that write would mark nothing. One cell per location makes every layer's write to it a `markChanged` of the one cell, and removing a layer a `markChanged` of each cell in its set whose effective value changed. A layered store that kept upstream's per-layer maps of plain values, and one cell per location in the base layer that every layer's write marks, would match upstream's storage; it needs a base cell for every location any layer writes, read or not. Follow-ups 12 states what forks need of this.

Not decided: the user's choice.
