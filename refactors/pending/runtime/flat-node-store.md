# Runtime exploration: a flat node store

Status: exploration, not planned. v0 (`refactors/pending/runtime/pico-runtime.md`, The store) uses the grouped representation: entity, then record id, then a record container that holds source nodes and resolver derived nodes by store key. This doc examines an alternative the user is not convinced against and wants examined: a flat map from a node key that encodes entity, record id, and store key to the node, and how far that flat map can go as the one representation for everything. Nothing here amends pico-runtime.md.

Upstream references are the checkout at `/Users/rbalicki/code/isograph`. Paths without a directory are under `isograph/libs/isograph-react/src/core`.

## Notation

Traces use pico-runtime.md's running example (The running example): records `root` and `other`, `root.2` holds the link to `other`, and two resolver derived nodes on `root`:

```text
FP reads: root.1, root.2, other.1, other.2, FC
FC reads: root.2, root.3, other.2, other.3
```

- `n(root.1)` is the source node of key `1` on `root`. `n(FP)` is the derived node of `FP` on `root`.
- `k(root, 1)` is the string key of `n(root.1)` under (b) and (c). `k(root)` is the record key under (c).
- `Lroot` is the root link object (`ast.root`, an object in the normalization AST artifact). `Lother` is the link object stored as `n(root.2)`'s value.
- `S.get(x)` is one `Map` or `WeakMap` lookup. A lookup whose key string was built for that lookup is written `S.get(build x)`.

## 1. Representations

### (a) Grouped (v0)

Copied from pico-runtime.md, The store, with no delta:

```ts
// from libs/isograph-runtime/src/store.ts
/** Every record container: `EntityName`, then record id. */
export type Store = Map<EntityName, Map<RecordId, RecordContainer>>;

/** One record's source nodes, and the derived nodes computed on it, which it owns. */
export type RecordContainer = {
  /** Read by `check`, the garbage collector's mark, and `storeSnapshot`; never by a computation. */
  presence: RecordPresence;
  /** Server values by store key. */
  readonly sources: Map<StoreKey, StoreSourceNode>;
  /** Resolver derived nodes by resolver store key. */
  readonly derived: Map<StoreKey, ResolverNode>;
};

/** `Present`: `normalize` wrote the record. `Absent`: a graph read created the container to hold a
 * source node or derived node of a record the store does not hold. */
export type RecordPresence = 'Present' | 'Absent';

export type StoreSourceNode = SourceNode<StoreValue | Missing, ResolverResult<unknown>>;
export type ResolverNode = DerivedNode<ResolverResult<unknown>>;

/** A source node's value when the record holds no value under its key: missing data. */
export const MISSING: unique symbol = Symbol('Missing');
export type Missing = typeof MISSING;
```

Upstream groups the same way: `StoreLayerData` is typename, then id, then a `StoreRecord` object of plain values (`IsographEnvironment.ts:150-158`, `:165-169`).

### (b) Flat map with an encoded key, no record nodes

```ts
// from libs/isograph-runtime/src/store.ts
declare const sourceNodeKeyBrand: unique symbol;
declare const resolverNodeKeyBrand: unique symbol;

/** The key of one server value of one record: entity, record id, server store key. */
export type SourceNodeKey = string & { readonly [sourceNodeKeyBrand]: true };

/** The key of one resolver derived node of one record: entity, record id, resolver store key. */
export type ResolverNodeKey = string & { readonly [resolverNodeKeyBrand]: true };

export type NodeKey = SourceNodeKey | ResolverNodeKey;

/** Every source node and resolver derived node of every record, by key. */
export type Store = Map<NodeKey, StoreNode>;

/** A node, tagged with the kind its key names. */
export type StoreNode = StoreSourceNode | ResolverNode;

export type StoreSourceNode = SourceNode<StoreValue | Missing, ResolverResult<unknown>> & {
  readonly kind: 'Source';
};
export type ResolverNode = DerivedNode<ResolverResult<unknown>> & { readonly kind: 'Resolver' };
```

```ts
// from libs/isograph-runtime/src/nodeKey.ts
/** `S`, then entity, record id, and store key, each of the first two prefixed by its length. */
export function sourceNodeKey(record: StoreLink, key: StoreKey): SourceNodeKey {
  return `S${record.entityName.length}:${record.entityName}${record.id.length}:${record.id}${key}` as SourceNodeKey;
}

/** `D`, then the same encoding with the resolver store key. */
export function resolverNodeKey(record: StoreLink, key: StoreKey): ResolverNodeKey {
  return `D${record.entityName.length}:${record.entityName}${record.id.length}:${record.id}${key}` as ResolverNodeKey;
}
```

The encoding is injective for any strings: the first character names the kind, a decimal length ends at the first `:`, and the store key is the remainder. Length prefixes are needed on the entity too, because a `Discriminator` entity name comes from the response (`recordEntityName`, `libs/isograph-runtime/src/recordId.ts:50-68`) and may hold any character. The alternative encoding is `JSON.stringify(['S', entityName, id, key])`, which is the form the previous revision of pico-runtime.md used for `valueSource` and `recordSource` (commit `fa58b6284`, The store's side (2b)). Open decision 2.

`S` and `D` keep a server key and a resolver store key of one name apart, as v0's two maps do.

The `kind` field is there because one `Map` holds two node types. A lookup by `SourceNodeKey` returns `StoreNode`, and the accessor narrows by `kind`. A `Resolver` node under an `S` key is a broken invariant the type system cannot express on a `Map`; the accessor throws on it. One map per kind (`Map<SourceNodeKey, StoreSourceNode>` and `Map<ResolverNodeKey, ResolverNode>`) has no throw site and no `kind` field. Open decision 3.

(b) has no place for `presence`. Its consequences are in Delete a record and Serialization.

### (c) Flat map plus a record node per record

```ts
// from libs/isograph-runtime/src/store.ts
declare const recordNodeKeyBrand: unique symbol;

/** The key of one record's record node: entity and record id. */
export type RecordNodeKey = string & { readonly [recordNodeKeyBrand]: true };

export type NodeKey = SourceNodeKey | ResolverNodeKey | RecordNodeKey;

/** Every source node, resolver derived node, and record node, by key. */
export type Store = Map<NodeKey, StoreNode>;

export type StoreNode = StoreSourceNode | ResolverNode | RecordNode;

/** One record: its presence, and every node under its entity and id. */
export type RecordNode = {
  readonly kind: 'Record';
  /** Read by `check`, the garbage collector's mark, and `storeSnapshot`; never by a computation. */
  presence: RecordPresence;
  readonly members: RecordMembers;
};
```

```ts
// from libs/isograph-runtime/src/nodeKey.ts
/** `R`, then entity and record id, each prefixed by its length. */
export function recordNodeKey(record: StoreLink): RecordNodeKey {
  return `R${record.entityName.length}:${record.entityName}${record.id.length}:${record.id}` as RecordNodeKey;
}
```

Three shapes for `members` (open decision 4):

```ts
// from libs/isograph-runtime/src/store.ts
// (c1) node references; each member node carries its own key so a delete can remove it from the map.
export type RecordMembers = {
  readonly sources: Set<StoreSourceNode>;
  readonly derived: Set<ResolverNode>;
};
export type StoreSourceNode = SourceNode<StoreValue | Missing, ResolverResult<unknown>> & {
  readonly kind: 'Source';
  readonly key: SourceNodeKey;
};
export type ResolverNode = DerivedNode<ResolverResult<unknown>> & {
  readonly kind: 'Resolver';
  readonly key: ResolverNodeKey;
};

// (c2) store keys; a delete rebuilds each node key from the record's link and the store key.
export type RecordMembers = {
  readonly sources: Set<StoreKey>;
  readonly derived: Set<StoreKey>;
};

// (c3) store key to node.
export type RecordMembers = {
  readonly sources: Map<StoreKey, StoreSourceNode>;
  readonly derived: Map<StoreKey, ResolverNode>;
};
```

(c3) is v0's `RecordContainer` with entity and id collapsed into one key, plus a second path to every node through its flat key. (c1) and (c2) are an index of the record's nodes, and the flat map is the only path to a node by key.

## 2. Operations

### Read a field

Marking and pulling are the same under all three. `markSourceChanged`, `markDirty`, `markCheck`, `verify`, and `recompute` walk `dependents` and `dependencies`, which are object references (pico-runtime.md, The graph), and build no key. What differs is the lookup the reader walk makes at each key it reads: `sourceNodeAt` in a computation's `contextHooks`, `storeValue` in v1's hooks, `check`, and the garbage collector's mark. A recomputation runs the walk again from its root, so it looks up every key again; the old `dependencies` list is not consulted by the walk.

```text
(a) root.1: S.get("Query") -> .get("__ROOT") -> .sources.get("1") = n(root.1)          3 lookups, no string built
(b) root.1: S.get(build k(root, 1)) = n(root.1)                                          1 lookup, 1 string built
(c) root.1: as (b)
```

v0's `sourceNodeAt` does all three lookups per key (`containerAt` then `sources`); it does not hold the container across the keys of one record. In V8 a string's hash is computed once and kept in the string. The entity name, the record id (held by the link object), and a store key with no arguments are strings already hashed by earlier lookups, so (a)'s three lookups hash nothing new. (b)'s key is a new string per lookup, so the lookup hashes all of it, `O(length)`. A store key with arguments is built per read under every option (`storeKey`, `libs/isograph-runtime/src/storeKey.ts:13-20`). Neither cost is measured.

Under (b) and (c) a first compute of `FP`:

```text
read FP: absent -> create: S.set(build k'(root, FP), n(FP)) -> recompute FP
  root.1  -> S.get(build k(root, 1)) = n(root.1) -> n(root.1).dependents = {FP}
  root.2  -> S.get(build k(root, 2)) = n(root.2) -> value Lother -> n(root.2).dependents = {FP}
  other.1 -> S.get(build k(other, 1)) = n(other.1) -> n(other.1).dependents = {FP}
  other.2 -> S.get(build k(other, 2)) = n(other.2) -> n(other.2).dependents = {FP}
  FC      -> S.get(build k'(root, FC)) absent -> create -> recompute FC ...
```

`k'` is `resolverNodeKey`. Under (a) each line is three lookups and no build.

### Key caching

Each strategy, what it caches, what invalidates it, what it costs, and how much of (b)'s per-lookup cost it removes.

#### A link-keyed cache of node keys

```ts
// from libs/isograph-runtime/src/store.ts
/** Source node keys already built, by the link object a read reached the record through. */
type NodeKeyCache = WeakMap<StoreLink, Map<StoreKey, SourceNodeKey>>;
```

A `WeakMap` key must be an object. The candidate is the `StoreLink` object a parent's source node holds. Its identity is stable while the value is unchanged, because `normalize` keeps an unchanged value's object (pico-runtime.md, Core changes 1: normalizing the same response twice keeps every stored link object, `toBe`).

```text
read FP, first compute, cache empty
  root.1  -> cache.get(Lroot) absent -> cache.set(Lroot, new Map) -> build k(root, 1) -> cache(Lroot).set("1", k) -> S.get(k) = n(root.1)
  root.2  -> cache(Lroot).get("2") absent -> build k(root, 2) -> set -> S.get(k) = n(root.2) -> value Lother
  other.1 -> cache.get(Lother) absent -> set -> build k(other, 1) -> set -> S.get(k) = n(other.1)
write root.1 -> n(root.1).dependents = {FP} -> FP Dirty
read FP, recompute
  root.1  -> cache.get(Lroot) -> .get("1") = k(root, 1), already hashed -> S.get(k) = n(root.1)     3 lookups, no string built
  other.1 -> cache.get(Lother) -> .get("1") = k(other, 1) -> S.get(k) = n(other.1)
```

A link retarget writes a new link object, which misses:

```text
write root.2 = Lnext -> FP, FC Dirty
read FP: root.2 value Lnext -> cache.get(Lnext) absent -> build k(next, 1) ...
  Lother: no source node holds it, no dependency holds it -> the WeakMap entry is collected with it
```

A record delete leaves stale keys, and a stale key is safe:

```text
delete other -> S.delete(k(other, 1)), S.delete(k(other, 2)), S.delete(k(other, 3)) -> FP, FC Dirty
read FP: root.2 still Lother -> cache(Lother).get("1") = k(other, 1) -> S.get misses -> insert n'(other.1) MISSING -> FP MissingData
```

Caveats:

- Several link objects point to one record. Each position a response links a record from gets its own link object (`recordLink`, `libs/isograph-runtime/src/recordId.ts:26-48`), so `other` linked from three positions has three cache entries, each with its own inner `Map`.
- The root has no parent link. `Lroot` is the AST's `root` object, stable per artifact, so it works as a `WeakMap` key, with one entry per artifact for one root record. A caller that passes a fresh `{ entityName, id }` per read (a test's `pet('1')`, a link a client pointer's resolver returns on each call) misses on every read and leaves a `Map` per call for the collector.
- The per-read cost becomes three lookups with no key built (`WeakMap`, inner `Map` by store key, flat map by a hashed key), which is (a)'s count. It removes the string build and the rehash. It does not make (b) cheaper than (a).

#### A link-keyed cache of nodes

```ts
// from libs/isograph-runtime/src/store.ts
type NodeCache = WeakMap<StoreLink, Map<StoreKey, StoreSourceNode>>;
```

Two lookups per read and no flat lookup, one fewer than (a). The cache is then a grouped index keyed by link identity: the grouping moves out of the store into a rebuildable index, and the flat map stays the source of truth.

A record delete makes it wrong:

```text
delete other -> S.delete(k(other, 1)) -> FP Dirty; n(other.1) is no longer in S, its value is still "x"
read FP: cache(Lother).get("1") = n(other.1) -> FP reads "x", n(other.1).dependents = {FP}
write other.1 "y" -> sourceNodeAt inserts n'(other.1) -> n'(other.1).dependents = {} -> FP not marked: stale with no error
```

The delete would have to clear every cache entry of the record, and a `WeakMap` cannot be enumerated, so it needs a list of the record's link objects: a per-record index again. The other repair is a store rule: a delete keeps a source node that has dependents in place with `MISSING`, and only a node with no dependents is removed. Then a node's identity is stable for its location while anything references it, and every node cache is sound. The previous revision of pico-runtime.md had that rule for value cells (commit `fa58b6284`, open decision 29 (a): "It cannot delete one with a reader"). v0 removes the container and its nodes (`deleteRecord`, pico-runtime.md, Accessors). Open decision 6.

The split, assessed. The link-keyed index cannot serve the operations that need record grouping: `deleteRecord` and the garbage collector's sweep start from a record, not a link, and cannot enumerate a `WeakMap`. So (b) plus a node cache still needs a record index or a scan for deletes, which makes three structures (flat map, record index, link cache) where (a) has one. The same cache keyed to a container (`WeakMap<StoreLink, RecordContainer>`) applies to (a) and saves (a) two lookups per read, with the same delete rule. Caching does not separate the options; it lowers both.

#### Store keys cached per reader AST selection

A selection with no arguments has the store key `selectableName` followed by an empty chunk string; a selection whose arguments are all literals has a constant store key. The compiler can print it on the selection (`readonly storeKey: StoreKey` beside `selectableName`), or the runtime can cache it in a `WeakMap<ReaderSelection, StoreKey>`. A selection with a variable argument has a key per variables object; a resolver node's variables object is closed over by its computation and stable, so a `WeakMap<Variables, WeakMap<ReaderSelection, StoreKey>>` caches it. Invalidation: none; an artifact never changes. Memory: one string per selection. It removes `storeKey`'s build under all three options equally and none of (b)'s entity and id encoding.

#### Interning node keys

An intern table `Map<string, NodeKey>` returns one string object per distinct key. Finding the interned string needs the string built and hashed, which is the cost being removed, so it removes nothing per lookup. The flat map already holds one copy of each key, so it saves no memory either. The previous revision of pico-runtime.md rejected an intern table for the same reason (commit `fa58b6284`, Costs of the ids).

#### Reusing the previous dependency on a recompute

A recompute usually reads the same (link, store key) pairs in the same order as the previous compute. If each `Source` dependency kept the link object and store key it was reached through, the walk's `i`-th read could compare its (link, key) with the `i`-th previous dependency by `===` and reuse that node with no lookup, falling back to a lookup on the first mismatch.

- Caches: one node per edge, on edges that already exist.
- Coupling: `Dependency` is store-agnostic (pico-runtime.md, The graph: `graph.ts` names no record or key). The location would ride as an opaque payload on `Source` dependencies, and `ComputeContext` would expose the previous dependency at a position, two changes to the engine's types.
- Retarget: the link object differs, so it misses and looks up. Correct.
- Delete: the dependent is `Dirty`, its previous dependency is the removed node, and the link object is unchanged, so it reuses the removed node, which is the node cache's failure. It is sound only under the delete rule above.
- Memory: two references per source edge.
- Removes: every lookup on an unchanged path of a recompute, under any option. First computes, new paths, `check`, the garbage collector's mark, and v1 reads pay full cost.

#### The reader walk's per-record frame

When the walk enters a record (`readSelectionSet` at a link), it could build the record's prefix (`S`, entity, id) once and append each store key. Each key is still a new string, hashed in full. It removes the length-to-decimal conversions and part of the concatenation, not the hash. Under (a) the analog is holding the container across the record's keys, which cuts (a) from three lookups to one per key after the first; v1's `normalize` did this (`writeSelections` received the `StoreRecord`, `libs/isograph-runtime/src/normalize.ts:52-79`), and v0's accessors dropped it.

#### A structured key

A key structured as entity, then id, then store key, looked up level by level, is a nested `Map`, which is (a).

#### Which make the lookup cost a non-issue

- The link-keyed node key cache brings (b) to (a)'s lookup count with no string built, and is safe under v0's delete. It costs a `Map` per link object read.
- The link-keyed node cache and the dependency reuse go below (a)'s count, and are safe only if a delete keeps nodes that have readers. Both apply to (a) as well.
- Per-selection store keys, the per-record frame, and interning do not: the first two remove part of the build, the last removes nothing.

### Write (normalize)

v0's `writeValue` calls `sourceNodeAt` per key and `presentContainerAt` per record object (pico-runtime.md, `normalize` writes source nodes). Change detection (`isUnchanged`) and `writeSource` are the same under all three.

```text
(a) Pet:1 object -> presentContainerAt: S.get("Pet") -> .get("1")                     per record: 2 lookups, set presence
    name "Rex"   -> S.get("Pet") -> .get("1") -> .sources.get("name") = n -> unchanged -> nothing
(b) Pet:1 object -> nothing per record
    name "Rex"   -> S.get(build k(Pet:1, name)) = n -> unchanged -> nothing
(c) Pet:1 object -> S.get(build k(Pet:1)) = record node -> presence Present
    name "Rex"   -> S.get(build k(Pet:1, name)) = n -> unchanged -> nothing
    tagline new  -> S.get(build k(Pet:1, tagline)) absent -> S.set(k, n MISSING) -> record node.members add -> writeSource
```

Under (c) the walk holds the record node across the keys of one object, so adding a member costs no second lookup. A new key costs one more insert under (c) than under (b).

### Create a record

```text
(a) first write of Pet:9 -> S.get("Pet") -> .get("9") absent -> insert container Present
(b) no step; the record is its first node: S.set(k(Pet:9, name), n)
(c) S.get(build k(Pet:9)) absent -> S.set(k(Pet:9), record node Present)
```

A graph read of an absent record creates it `Absent` under (a) and (c) (`sourceNodeAt`, pico-runtime.md, Absent records) and inserts only the `MISSING` node under (b).

### Delete a record

v0's `deleteRecord` marks the dependents of every source node of the record, disposes every derived node it owns, and removes the container (pico-runtime.md, Accessors and Traces).

```text
(a) delete other -> S.get(E).get("other") = container -> sources {1, 2, 3} -> markSourceChanged each -> FP, FC Dirty
                 -> derived {} -> remove container
(b) delete other -> scan every entry of S, decode each key's entity and id, select those of other
                 -> markSourceChanged each, dispose each Resolver node, S.delete each
(c) delete other -> S.get(build k(other)) = record node -> members
                 -> (c1) each node: markSourceChanged, S.delete(node.key)
                 -> (c2) each store key: S.get(build k(other, key)) -> markSourceChanged, S.delete
                 -> (c3) each node: markSourceChanged, S.delete(build k(other, key))
                 -> S.delete(k(other))
```

(b) costs `O(all nodes)` per deleted record, or one scan per batch of deletes. An index from record to its node keys removes the scan, and that index is (c). Ownership is the same: v0's `derived` map makes a record the owner of its resolver nodes with no `owned` map (pico-runtime.md, The store). (b) has no owner structure and finds owned nodes by the scan; (c)'s `members.derived` is the owner structure.

(b) and presence. (b) has no entry for a record that has no node, so it cannot hold v1's `Present` record with no values (EmptyStats's `stats {}` writes an empty `PetStats` record, `libs/isograph-runtime/test/normalize.test.ts:270`). `check` reads presence (`recordPresence`, `libs/isograph-runtime/src/check.ts:30`, `:54`), so under (b) `check` of `stats {}` finds no record, returns `MissingData`, and an `IfNecessary` fetch refetches that query on every render. (b) therefore also changes `check` to read a record with an empty selection set as satisfied, which is what v0's reader already does for a resolver on an absent record (pico-runtime.md, Absent records). Under (c) the record node holds presence, as v0's container does.

### Garbage collection

The mark walks each retained query's normalization AST from its root, as upstream's does: `garbageCollectEnvironment` collects `retainedQueries` and `gcBuffer` (`garbageCollection.ts:65-86`), `recordReachableIds` starts at the root record (`:125-147`), and `recordReachableIdsFromRecord` follows `Linked` selections through `dataLayer[typename][id]` (`:149-226`). i2's mark is the same walk (`libs/isograph-runtime/src/garbageCollection.ts:46-95`). The mark's lookups are the reads above: three per key under (a), one built key per key under (b) and (c). The link-keyed caches apply.

The mark yields reachable records. The sweep:

```text
(a) for each entity map, for each container: reached? keep : deleteRecord             O(records) + deleted nodes
    upstream: for typeName in data, for dataId in data[typeName]: delete unless retained (garbageCollection.ts:98-118)
(b) for each entry of S: decode the key's record, reached? keep : delete                O(all nodes)
(c) for each RecordNode entry of S: reached? keep : deleteRecord                        O(all entries) to find record nodes
    or a second map of record nodes only, O(records)
```

(b) decodes each key or each node carries its record's link. With the length-prefixed encoding, decoding is two integer parses and a slice; with `JSON.stringify` it is a `JSON.parse` per key.

A field-granular collection is natural under (b): the mark yields the node keys it read, and the sweep deletes every source node not among them, so a reached record loses the keys no retained query selects. Upstream keeps every key of a reached record (`garbageCollection.ts:109-113` deletes by `dataId` only). Field granularity is expressible under (a) too. Open decision 7.

### Objects without ids

A composite object with no identity gets a derived id from its parent's link, the selection, list indexes, and argument chunks (`recordId`, `Omit`, `libs/isograph-runtime/src/recordId.ts:109-114`). Each level embeds its parent's id, so the id grows with depth.

```text
Query:__ROOT.viewer     -> Viewer, id "Query:__ROOT.viewer"
  .settings             -> Settings, id "Viewer:Query:__ROOT.viewer.settings"   (35 characters)
    .theme
(a) S.get("Settings") -> .get(id, held by the link object, already hashed) -> .sources.get("theme")
(b) S.get(build "S8:Settings35:Viewer:Query:__ROOT.viewer.settingstheme")             a 54-character string per read
```

Under (a) the long id is one string held by the link object and hashed once. Under (b) every read of every key on that record builds and hashes a string at least as long as the id, so (b)'s cost per read grows with nesting depth. The link-keyed node key cache removes it. Under (c) the record node key has the same length.

### Console and debug inspection

v0's store already prints poorly: `console.log(environment.store)` shows `Map` of `Map` of containers whose `sources` values are `SourceNode` objects with `dependents` sets of `DerivedNode` objects, each with a `slot` and its own `dependents`. The grouping by entity and record survives; the values are buried. (b) prints one `Map` of every node in insertion order, with keys like `S3:Pet1:1name`, and records are not grouped. Both need a debug view; (b)'s has to decode keys to group.

```ts
// from libs/isograph-runtime/src/debug.ts
/** The store for a person reading it: entity, then record id, then the record's values and resolver
 * results as plain data. Callers: a developer at a console. */
export function storeDebugView(store: Store): DebugStore;

export type DebugStore = { readonly [entityName: string]: { readonly [recordId: string]: DebugRecord } };

export type DebugRecord = {
  /** (a) and (c): the container's or record node's presence. (b): `Present` when any node holds a value. */
  readonly presence: RecordPresence;
  /** Server values by store key; `MISSING` prints as the string `<missing>`. */
  readonly values: { readonly [storeKey: string]: StoreValue | '<missing>' };
  /** Resolver results by resolver store key, with the mark and the number of dependents. */
  readonly derived: { readonly [storeKey: string]: DebugDerived };
};

export type DebugDerived = {
  readonly mark: DerivedNodeMark;
  /** The stored value; absent for a `Dirty` node never computed. */
  readonly value?: ResolverResult<unknown>;
  readonly dependents: number;
};

export type DerivedNodeMark = 'Clean' | 'Check' | 'Dirty';
```

The objects are built with `Object.fromEntries`, which defines own properties, so a record id or store key `constructor` or `__proto__` prints as itself. Under (a) the view is three nested `forEach`; under (b) it is one pass that decodes each key and groups; under (c) it is one pass over record nodes and their members.

### Serialization and SSR snapshot

v0's `storeSnapshot(store): PlainStore` returns every `Present` record with every non-`MISSING` value as nested `Map`s (pico-runtime.md, Accessors). Neither form is JSON: `JSON.stringify` of a `Map` writes `{}`. A wire format is a separate choice from the in-memory representation.

- Nested objects, `{ [entity]: { [id]: { [storeKey]: value } } }`. This is upstream's `StoreLayerData` shape, which `createIsographEnvironmentCore` takes as its seed (`IsographEnvironment.ts:165-169`, `:178-193`). (a) writes it with three nested `Object.fromEntries`. (b) decodes every key and groups. (c) walks record nodes.
- Flat pairs, `[[nodeKey, value], ..]`. (b) writes it with one filter and map over the entries, and reads it back with one loop. The entity and id repeat once per key, so the payload is larger before compression than the nested form for a record with more than one key.

Hydration inverts the same walk. Under every option, resolver nodes are not serialized: their values include functions and components. A `Present` record with no values survives the round trip under (a) and (c) and is lost under (b).

### Optimistic layers and A DAG of graphs

Upstream layers by record. Each layer's `data` is a sparse `StoreLayerData` in which a `null` record is a deletion (`IsographEnvironment.ts:165-169`); a read builds a record proxy that walks the layers per property (`getStoreRecordProxy`, `optimisticProxy.ts:24-44`; `getMutableStoreRecordProxy`, `:46-98`); a merge copies record by record (`mergeDataLayer`, `:185-200`); a revert compares old and new data record by record and reports a record when any key differs (`compareData`, `:506-544`).

Per-key overrides in a layer, under each representation:

```ts
// from libs/isograph-runtime/src/layer.ts
// (a), upstream's grouping with a record tombstone:
export type GroupedLayer = {
  readonly parent: GroupedLayer | null;
  readonly data: Map<EntityName, Map<RecordId, Map<StoreKey, StoreValue | Missing> | RecordDeleted>>;
};
// (b) and (c):
export type FlatLayer = {
  readonly parent: FlatLayer | null;
  readonly data: Map<SourceNodeKey, StoreValue | Missing>;
  /** Records this layer deleted; every key of each reads `MISSING` in this layer's worlds. */
  readonly deletedRecords: Set<RecordNodeKey>;
};
export const RECORD_DELETED: unique symbol = Symbol('RecordDeleted');
export type RecordDeleted = typeof RECORD_DELETED;
```

```text
read other.1 in world W = [base, L1, L2]
(a) L2.data.get(E)?.get(id) -> deleted? MISSING : .get("1") -> absent -> L1 ... -> base container      up to 3 lookups per layer
(b) k = build k(other, 1), rk = build k(other)
    L2.deletedRecords.has(rk) -> L2.data.get(k) -> absent -> L1 ... -> S.get(k)                       2 lookups per layer, 2 strings once
commit L2 into L1
(a) for each entity, for each record: deleted? set tombstone : merge keys                              upstream's mergeDataLayer
(b) L2.data.forEach((v, k) => L1.data.set(k, v)); for each rk in L2.deletedRecords: remove L1's keys of that record
```

Flat storage makes a layer one map and a key merge one loop. A record delete in a layer needs a record-level entry under every option, and committing a delete into the layer below needs that layer's keys of the record: (a) reads its record map, (c) its record node, (b) scans. So flat is simpler for value overrides and not simpler for deletion.

pico-runtime.md's direction for worlds puts per-world state on the node: `stateOf` is the one accessor of a derived node's state, so that A DAG of graphs can give a node one slot per layer by changing `stateOf` alone (The graph). If a source node's value is per layer the same way, a layer is the set of nodes it wrote, `Set<StoreSourceNode | ResolverNode>`, held by object reference, and a read in a world is a walk of one node's slots. Then the layer holds no keys, and flat versus grouped does not touch layering except for enumerating a deleted record's nodes. Whether worlds form a tree or a DAG decides how a read picks among two parents' slots, under every representation.

### Memory

Not measured. The estimates follow V8's `OrderedHashMap` layout with pointer compression (4-byte slots): a `Map` of up to 4 entries is about 80 bytes (the `JSMap` object, a header, 2 buckets, 4 entries of 3 slots), and each further entry about 14 bytes plus the slack of capacity doubling. A one-byte string is about 12 bytes of header plus its characters. A `Set` entry is 2 slots instead of 3.

Per record with `F` source nodes and `D` resolver nodes, counting store structure only (the nodes themselves are the same objects under every option):

- (a): a container object (about 20 bytes), two `Map`s (about 160 bytes), `F + D` entries, one entry in the entity's map. Store key strings are often shared with the AST (`selectableName` for a key with no arguments).
- (b): `F + D` entries in one `Map`, and `F + D` key strings of their own, each at least the entity, the id, and the store key long. A `kind` field per node.
- (c): (b), plus a record node, its entry, and one or two `Set`s or `Map`s of `F + D` members. (c1) adds a `key` field per node. (c) is never smaller than (b), and is roughly (a) plus (b)'s key strings.
- The link-keyed node key cache: per link object a read reached, a `WeakMap` entry and a `Map` of up to `F` entries. A record linked from three positions has three.

With 30-byte keys, (b) costs about `44 (F + D)` bytes against (a)'s about `180 + 14 (F + D)`, so (b) is smaller below about six nodes per record and larger above. Long derived ids move the crossover down.

One `Map` has a size limit. V8 throws `RangeError` when a `Map` grows past 2^24 entries (16,777,216). (b) and (c) put every node of the store in one `Map`, so one million records of 17 keys each exceed it. (a) spreads nodes over one `Map` per record.

## 3. The record node under (c): source or index

A record node's `members` changes when a node is first inserted at the record (a write's new key, and also a graph read's `MISSING` node on an absent record) and when the record is deleted. Its `presence` changes when `normalize` writes the record or the sweep deletes it.

pico-runtime.md removed record presence as a source: an absent record reads as a record whose every key is missing, a computation records the `MISSING` source node of the first key it tries, and a write of that key marks it (Absent records). v0 keeps `presence` on the container as plain data, read by `check`, the garbage collector's mark, and `storeSnapshot`, never by a computation.

What would read the record node as a dependency:

- The reader walk reads keys a reader AST names, never which keys exist. No reader AST selects a whole record.
- `check` reads presence and keys and is not a computation. A fetch decision made as a derived node (pico-runtime.md, Absent records, the fetch decision) reads the root derived node, which reads keys.
- The garbage collector's mark and sweep are not computations.
- A closure component's wrapper reads no data (components-in-userland.md) and is owned through the record's `derived`; under (c) through `members.derived`. Ownership is not a dependency.
- Upstream's `AnyChangesToRecord` subscription fires when any key of a record is written (`IsographEnvironment.ts:54-58`, `subscribe.ts:92-101`). As a dependency it would be an aggregate of the record's source nodes' changes, which `members` does not carry: a value write does not change `members`. i2's subscriptions have no such kind (`libs/isograph-runtime/src/subscribe.ts`).

Nothing reads it as a dependency, so under (c) the record node is an index: presence as plain data plus the list of the record's nodes. It has no `dependents` set, and (c)'s types above give it none (we do not have anything we do not use). (c) does not reintroduce presence as a source. It reintroduces exactly what v0's container is, the record grouping as plain data, reached through a key instead of two nested lookups. `members` also counts `MISSING` nodes that reads inserted, so it does not answer "which keys the server sent".

## 4. Non-record nodes in the same map

The graph has nodes that are not on records: a request's state per `RequestKey` (network-and-subscriptions.md, Request entries in the store), the URL (loaders.md, The router), a loader item per `LoaderInstanceKey` and effect-set nodes per `EffectRootKey` (loaders.md, An item that changes over time, The effect set). loaders.md is written against the previous revision's `SourceId`s and states what each becomes under source objects: a `Source` per key kept in an environment map, and one URL source the router holds (loaders.md, An item that changes over time).

Under (b) or (c) each gets a key kind:

```ts
// from libs/isograph-runtime/src/nodeKey.ts
export type NodeKey =
  | SourceNodeKey
  | ResolverNodeKey
  | RecordNodeKey
  | RequestNodeKey
  | LoaderItemNodeKey
  | EffectRootNodeKey
  | UrlNodeKey;

export function requestNodeKey(key: RequestKey): RequestNodeKey {
  return `Q${key}` as RequestNodeKey;
}
export function loaderItemNodeKey(key: LoaderInstanceKey): LoaderItemNodeKey {
  return `L${key}` as LoaderItemNodeKey;
}
export function effectRootNodeKey(key: EffectRootKey): EffectRootNodeKey {
  return `E${key}` as EffectRootNodeKey;
}
export const URL_NODE_KEY = 'U' as UrlNodeKey;
```

```ts
// from libs/isograph-runtime/src/store.ts
/** Every node of the environment, by key. Each accessor takes one key kind and returns that kind's node. */
export class Store {
  readonly #nodes: Map<NodeKey, StoreNode>;

  sourceAt(key: SourceNodeKey): StoreSourceNode | undefined;
  resolverAt(key: ResolverNodeKey): ResolverNode | undefined;
  recordAt(key: RecordNodeKey): RecordNode | undefined;
  requestAt(key: RequestNodeKey): RequestNode | undefined;
  loaderItemAt(key: LoaderItemNodeKey): LoaderItemNode | undefined;
  effectRootAt(key: EffectRootNodeKey): EffectRootNode | undefined;
  url(): UrlNode;
}
```

A `Map` cannot tie a key kind to a value kind, so every accessor narrows by the node's `kind` and has a throw site for a mismatch, one per kind. One map per kind has none. User code that passes `new Map()` as the store (`demos/pet-demo/src/fetchAndRead.ts:19`) calls a constructor instead.

What uniformity buys:

- One debug view and one enumeration of every node in the environment.
- One layering mechanism if overrides are per key: a world can hold its own URL or request state as an override of that key, which is what a pending world's inputs need (loaders.md, open decision 16).

What it does not buy:

- Uniform lifetime. Records live by reachability from retained queries, requests by retain counts, loader items by instances, effect roots by mounting. Each kind keeps its own release rule, so the sweep still branches per kind.
- Uniform data. A request entry holds a `PromiseWrapper` and retain counts, and loader instances hold cleanups (network-and-subscriptions.md, Request entries in the store: "Promises are not data"). The map's values stop being plain data, and serialization filters by kind.
- Simpler types. Seven key kinds, seven node kinds, and seven narrowing sites replace one environment field per kind, each already typed by its own `Map`. Upstream keeps each kind of state in its own environment field (`IsographEnvironment.ts:73-93`).

Under (a) the same nodes live in environment maps beside the store. (a) could host them under entity names GraphQL reserves (`__Request`), but the runtime is protocol-neutral and another protocol may not reserve the prefix.

If per-world state lives on nodes (Optimistic layers and A DAG of graphs), the layering benefit holds for nodes in environment maps too, and the remaining gain of one map is the single enumeration.

## 5. Assessment

Where (a) wins:

- Reads without a cache: no string built, hashes reused. Holding the container per record frame takes it to one lookup per key.
- Deletes and the sweep: `O(records)`, with ownership and presence in the container.
- Records with many keys, and records with long derived ids: less memory, no per-key strings.
- No `Map` size limit in practice.
- Upstream's shape for the store and the seed (`IsographEnvironment.ts:165-169`, `:178-193`), so the nested serialization is direct.

Where (b) wins:

- Records with few keys: less memory, no per-record `Map`s.
- The flat-pairs serialization and a key-merge of a layer are one loop.
- Field-granular collection, if wanted, is a plain set difference.
- No empty-map cleanup when a record's last node goes.

Where (b) loses on correctness, not cost: it cannot hold a `Present` record with no values, so `check` and the snapshot change (Delete a record). Deletes need a scan or an index, and the index is (c).

Where (c) wins: nowhere over both others. It is (b) with (a)'s grouping added back as an index, so it pays (b)'s key strings and (a)'s per-record structure. Its one shape that differs from (a), (c1) or (c2), lets the flat map be the only path to a node by key.

Key caching. The link-keyed node key cache removes (b)'s per-lookup cost down to (a)'s lookup count and is safe as v0 deletes records. Node caches and dependency reuse go further and need the delete rule that keeps nodes with readers in place; they speed (a) by the same amount. No cache makes (b) faster than (a) on reads; the best makes them equal.

What adopting (b) or (c) later changes in pico-runtime.md. v0 routes every store access through accessors: "No function outside `store.ts`, `normalize.ts`, and `storedResults.ts` touches `sources`, `derived`, or `presence`" (pico-runtime.md, Accessors). So the grouping is behind a seam, and a later switch rewrites:

- `store.ts`: `Store`, `RecordContainer`, `recordContainer`, `containerAt`, `presentContainerAt`, `sourceNodeAt`, `storeValue`, `recordPresence`, `storeSnapshot`, `deleteRecord`.
- `storedResults.ts`: `resolverNodeAt`, which inserts into `containerAt(store, root).derived`.
- `garbageCollection.ts`: the sweep, which iterates entity maps and containers.
- The package index: `StoreRecord`, renamed `RecordContainer` in Core changes 4, is an exported type; `Store` is exported and user code constructs it.
- Under (b): `check`'s presence reads and the `EmptyStats` behavior.

What does not change: `graph.ts`, `normalize`'s change detection, the reader walk and `ReadHooks` (they take links and store keys), `contextHooks`, the traces of marking and pulling, and the tests that compare `storeSnapshot(store)` with `storeOf(..)`. It is not a trap door, as long as the accessor rule holds; the switch is one module and the sweep.

## 6. Open decisions

### 1. The store's representation

- (a) Grouped, v0. Held: upstream groups records by typename and id (`IsographEnvironment.ts:150-158`, `:165-169`), sweeps by typename and id (`garbageCollection.ts:98-118`), and layers by record (`optimisticProxy.ts:24-98`, `:185-200`).
- (b) Flat, no record nodes. Cost: a scan or an index per delete; no `Present` record without values, which changes `check` and the snapshot; a key string per node; the `Map` size limit.
- (c) Flat with record nodes. Cost: (b)'s key strings and size limit, and (a)'s per-record structure.

### 2. The node key encoding, under (b) or (c)

- (a) Length-prefixed concatenation (`sourceNodeKey` above). Cost: a hand-written decoder for the sweep and the debug view.
- (b) `JSON.stringify([kind, entityName, id, storeKey])`, the previous revision's form. Cost: escaping on every build, `JSON.parse` to decode.

### 3. One map or one per node kind, under (b) or (c)

- (a) One `Map` with a `kind` on each node. Cost: a throw site per kind for a value that does not match its key's kind.
- (b) One `Map` per kind. Cost: the representation is no longer one map.

### 4. The record node's members, under (c)

- (c1) Node references, each node carrying its key.
- (c2) Store keys, rebuilt into node keys on delete.
- (c3) Store key to node: v0's container under a flat key.

### 5. Key caching

- (a) None.
- (b) Link-keyed node keys (`WeakMap<StoreLink, Map<StoreKey, SourceNodeKey>>`). Cost: a `Map` per link object read.
- (c) Link-keyed nodes. Cost: (b)'s memory, and decision 6 (b) is required.
- (d) Previous-dependency reuse on recompute. Cost: a location payload on source edges and a positional accessor on `ComputeContext`; decision 6 (b) is required.
- (e) Store keys printed per reader AST selection. Cost: one artifact field. Independent of the others and of the representation.

Upstream caches no key and no node; it reads `dataLayer[typename]?.[id]` per record (`garbageCollection.ts:130-131`, `optimisticProxy.ts:30`) and builds `getParentRecordKey` per read (`cache.ts:454-479`).

### 6. Whether a delete keeps a node that has readers

- (a) Remove every node of the record, v0's `deleteRecord`. Held: upstream deletes a record whole (`garbageCollection.ts:109-113`).
- (b) Keep a source node with dependents in place with `MISSING`, remove the rest; the sweep later removes a `MISSING` node with no dependents. Reason to deviate: it makes a node's identity stable for its location while anything references it, which decisions 5 (c) and 5 (d) need. Cost: a store that holds nodes for deleted records until their readers recompute.

### 7. Collection granularity

- (a) Record: a reached record keeps every key. Held: upstream (`garbageCollection.ts:98-118`).
- (b) Key: a reached record keeps only the keys a retained query selects. Cost: a read of a key no retained query selects, on a retained record, is missing after a collection.

### 8. Non-record nodes

- (a) Environment fields, one per kind. Held: upstream (`IsographEnvironment.ts:73-93`).
- (b) Key kinds in the one flat map. Cost: decision 3's narrowing per kind, and non-data values in the store.

### 9. The serialization format

- (a) Nested objects by entity and id. Held: upstream's seed is `StoreLayerData` (`IsographEnvironment.ts:165-169`, `:178-193`).
- (b) Flat pairs of node key and value. Cost: entity and id repeated per key; a format tied to the key encoding.
