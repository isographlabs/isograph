# Runtime: resolver results maintained by `normalize`

Status: under discussion. Not approved for implementation. This is the planned runtime step after v1 (`refactors/pending/runtime/store.md` and `refactors/pending/runtime/reader-ast.md`). It describes only what changes from v1, and from upstream where v1 matches upstream.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`.

## Goal

Renders do no work. Every resolver result a retained query's entrypoint reaches is computed when `normalize` writes the response, and recomputed after every later `normalize` that changes one of its inputs, in dependency order. A render reads a precomputed value and nothing else.

Upstream reads and evaluates during render. `useReadAndSubscribe` reads the fragment's data in a `useState` initializer (`react/useReadAndSubscribe.ts:33-35`), `useResult` calls the resolver in the render body (`react/useResult.ts:45-64`), every nested eager resolver is called again on every read (`read.ts:600`), and the component subscribes in an effect after the render (`react/useRerenderOnChange.ts:21-37`, `subscribe.ts:15-40`). After a write, `callSubscriptions` re-reads every subscription whose recorded records overlap the written ones (`subscribe.ts:63-109`, called at `makeNetworkRequest.ts:271`). The deviation is the user's: no read-and-subscribe during render, because a render that reads and evaluates does resolver work on the render path, repeats it for every component that reads the same resolver, and needs a second read in the effect to catch writes between render and subscribe (`subscribe.ts:32-36`).

## Two levels

```text
level 2: observers        listen(location, onChange)  snapshot(location)
                                 ^ notified after level 1 settles
level 1: resolver graph   ResolverEntry on records, computed and recomputed by normalize
                                 ^ reads
store:                    ServerEntry on records, written by normalize
```

Level 1 is the resolver graph. Its nodes are stored resolver results (`ResolverEntry`), one per client field, record, and argument values, held on the record under the client field's store key. Its edges are each entry's dependencies: the server values and the other resolver results its reader AST read. `normalize` owns it: it computes the entries a newly retained query reaches, and after every write it brings every retained query's entries up to date. Level 1 owns ordering. No level 2 code runs while level 1 is computing.

Level 2 is observers. An observer names one location of level 1 (a record and a resolver store key) and a no-argument callback. It holds no value. After level 1 settles, each observer whose location's stored result changed is called once. `snapshot` returns the stored result and computes nothing. This is the shape React's `useSyncExternalStore(subscribe, getSnapshot)` takes (Level 2), so a React package can render from level 2 while `@isograph/runtime` stays React-free.

With React, components are resolver results too (`@component`, Later): a component's entry is a level 1 node like any other, and a mounted component is a level 2 observer of its own entry.

## Steps

Each change is one commit, in order. Step A is optional and independent of 1 to 4.

- Step A. Re-read every subscription after each write, upstream's mechanism, with v1's `read`. No memoization.
- 1. Store entries and epochs (prefactor): every value on a record is a tagged entry, the store carries an epoch, and `normalize` keeps an equal value's entry.
- 2. Stored resolver results: `read` stores each resolver's result on its record and reuses it while its inputs are unchanged, verifying dependencies before itself (pico's algorithm).
- 3. Level 1: `normalize` retains its query, computes the entries its entrypoint reaches, and settles every retained query after each write. `garbageCollect` reads the retained queries from the store.
- 4. Level 2: `listen` and `snapshot`, called after settlement.

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
- `callSubscriptions` reads each subscription's data with v1's `readSelectionSet` (exported from `read.ts`), compares it with `delivered` by upstream's rule (`mergeObjectsUsingReaderAst`, `areEqualWithDeepComparison.ts:55-121`: server values structurally, nested `Resolver` values by `===`, `:84-95`), and when unequal stores it and calls the callback with `ok(artifact.resolver({ data, parameters: variables }))`, or with the `MissingData`.
- No overlap filter: v1's `normalize` records no `EncounteredIds` (store.md, `normalize`), so every subscription is re-read, where upstream skips the ones whose records do not overlap (`subscribe.ts:74-79`).
- Order does not matter for correctness: every re-read runs after the write finished and reads the whole store, so nothing reads a stale input. Nothing is shared: a resolver two subscriptions reach runs once per subscription per write.

Costs: every write re-reads every subscription and re-runs every nested resolver under it; a nested resolver that returns a new object makes its parent's data unequal on every write (upstream has the same, `areEqualWithDeepComparison.ts:84-95`), so its callback fires on every write. With the callback being React's `setState`, a demo renders correctly and slowly. Changes 1 to 4 replace it; nothing in them builds on it.

## End-user experience

```ts
// from libs/isograph-runtime/test/level2.test.ts
const store = createStore();
const retention = normalize(store, petSummaryEntrypoint, response, { id: '1' });
// Every resolver PetSummary reaches ran once, inside normalize.
const location = retention.location; // Query.__ROOT, 'PetSummary____id___"1"'
const onChange = vi.fn();
const stop = listen(store, location, onChange);
expect(snapshot(store, location)).toEqual({ kind: 'Ok', value: /* .. */ });

normalize(store, petSummaryEntrypoint, sameResponse, { id: '1' });
// No resolver ran, onChange was not called.

normalize(store, petSummaryEntrypoint, responseWithNewNickname, { id: '1' });
// PetName on Pet:1 and PetSummary ran once each; RecentCheckins did not.
// onChange was called once, after both ran; snapshot returns the new result.

stop();
dispose(store, retention);
garbageCollect(store); // the store holds the second and third retentions' records
```

A React package, which this doc does not plan, would render with:

```tsx
// from libs/isograph-react/src/useResolverResult.ts
const result = useSyncExternalStore(
  (onStoreChange) => listen(store, location, onStoreChange),
  () => snapshot(store, location),
);
```

## 1. Store entries and epochs

### The store

Before (store.md, The store):

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = Map<EntityName, Map<RecordId, StoreRecord>>;
export type StoreRecord = Map<StoreKey, StoreValue>;
```

After:

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = {
  /** Every record: `EntityName`, then record id, then the record. */
  readonly records: Map<EntityName, Map<RecordId, StoreRecord>>;
  /** The epoch of the last `normalize` or `garbageCollect` that changed `records`. */
  epoch: Epoch;
};

/** A store-wide counter. Starts at 0. */
export type Epoch = number;

/** One record's entries by store key. A server value and a stored resolver result share the
 * record, because a server selectable and a client field of one entity have distinct names. */
export type StoreRecord = Map<StoreKey, StoreEntry>;

export type StoreEntry = ServerEntry | ResolverEntry;

/** A value `normalize` wrote from a response. */
export type ServerEntry = {
  readonly kind: 'Server';
  readonly value: StoreValue;
  /** The epoch of the `normalize` that last changed `value`. */
  readonly timeUpdated: Epoch;
};

/** An empty store at epoch 0. */
export function createStore(): Store;
```

`ResolverEntry` is change 2's; change 1 adds the union with that one member only and change 2 adds the second. `StoreValue`, `StoreLink`, `StoreKey`, `JsonValue`, and `Variables` are store.md's, unchanged. `epoch` is mutable for the reason store.md's maps are: the store is written in place.

The entries are tagged because the record now holds two kinds of value and every walk narrows by `kind`: v1's untagged values are told apart by the AST (store.md, The store), and that stays true for leaf against composite inside `value`, but a stored resolver result is not a `JsonValue`, so TypeScript cannot narrow `StoreValue | ResolverEntry` without a tag. `timeUpdated` is pico's `time_updated` of a source (`docs-website/docs/design-docs/pico.md`, Equality and backdating): the unit of change is one store key on one record, where upstream's unit is the record (`cache.ts:211-218`). Reason: a resolver that reads `Pet:1.name` must not re-run when only `Pet:1.tagline` changes.

### `normalize` keeps an equal value

Before (store.md, `normalize`): every selected value is written with `record.set(key, value)`, and there is no change detection.

After, at the start of each call `writeEpoch = store.epoch + 1`, and every write goes through:

```ts
// from libs/isograph-runtime/src/normalize.ts
/** Writes `value` under `key`, unless the stored value is equal. The first write that changes
 * the store sets `store.epoch` to `writeEpoch`. */
function writeValue(
  store: Store,
  record: StoreRecord,
  owner: StoreLink,
  key: StoreKey,
  value: StoreValue,
  writeEpoch: Epoch,
): void;

/** Structural equality of JSON values: object keys in any order, list items in order. */
export function jsonEqual(a: JsonValue, b: JsonValue): boolean;
```

- Absent key, or a `ServerEntry` whose value is not `jsonEqual`: sets `{ kind: 'Server', value, timeUpdated: writeEpoch }` and sets `store.epoch = writeEpoch`.
- A `ServerEntry` whose value is `jsonEqual`: nothing; the old entry, its `timeUpdated`, and its value's identity stay.
- A `ResolverEntry`: throws (Errors).
- Creating a record sets `store.epoch = writeEpoch` too; a stored resolver result can depend on a record's absence (change 2, Dependencies).

A link is a `JsonValue` (`{ entityName, id }`) and a composite value is `null`, a link, or lists of these, so one equality covers leaves and composites. Setting `store.epoch` at the first change rather than at the end keeps a throw part-way (store.md open decision 9 (b)) from leaving entries newer than `store.epoch`.

Deltas from upstream: upstream compares a scalar with `!==` (`cache.ts:248-249`), so a list leaf or a `JSON` object counts as changed on every response, and compares links by id (`dataIdsAreTheSame`, `cache.ts:372-393`). Here a list leaf with equal items is unchanged, because otherwise every response would re-run every resolver that reads one.

### Readers of entries

`check`, `garbageCollect`, and `read` read `entry.value` of a `ServerEntry` where v1 reads the value. A `ResolverEntry` under a store key a normalization or reader `Scalar`/`Linked` selection names throws (Errors). `garbageCollect` sets `store.epoch = store.epoch + 1` when it deletes anything, so that a stored resolver result verified before the deletion is verified again (change 2, Verification).

### Errors

Added to store.md's throw sites, both broken invariants of distinct selectable names (a server selectable and a client field of one entity never share a store key, `docs-website/docs/design-docs/mental-model.md`, Entity: `selectables: BTreeMap<SelectableName, Selectable>`):

- `normalize`, `check`, `garbageCollect`, and `read`'s `Scalar`/`Linked` nodes, a `ResolverEntry` under a server store key: `Store record ${entityName}:${id} holds a resolver result under the server store key "${storeKey}".`
- `read`'s resolver lookup, a `ServerEntry` under a resolver store key: `Store record ${entityName}:${id} holds a server value under the resolver store key "${storeKey}".`

### Tests

store.md's tests change mechanically:

```ts
// from libs/isograph-runtime/test/fixtures.ts
/** A new store at epoch 0 whose every value is a `ServerEntry` with `timeUpdated: 0`. */
export function storeOf(records: StoreValues): Store;

/** Every `ServerEntry`'s value, by entity, id, and store key; resolver entries are left out. */
export function serverValues(store: Store): StoreValues;

export type StoreValues = {
  readonly [entityName: string]: {
    readonly [id: string]: { readonly [storeKey: string]: StoreValue };
  };
};
```

`expect(store).toEqual(storeOf({ .. }))` becomes `expect(serverValues(store)).toEqual({ .. })` with the same literal; `new Map()` becomes `createStore()`; `expect(store).toEqual(new Map())` becomes `expect(serverValues(store)).toEqual({})`. New cases in `test/normalize.test.ts`:

- `store.epoch` is 0, then 1 after `PetDetail`; normalizing the same response again leaves it 1 and every entry `===` the one before.
- `tags: ["a"]` then `tags: ["a"]` keeps the first list object; then `tags: ["b"]` stores a new entry with `timeUpdated: 3`.
- `settings` objects with the same keys in another order are equal.
- A store whose `Pet:1.name` holds a hand-built `ResolverEntry` makes `PetDetail` throw the resolver-under-server-key message.
- `garbageCollect` that deletes a record advances `store.epoch`; one that deletes nothing does not.

## 2. Stored resolver results

A resolver's result is stored on the record it was read on, under the client field's resolver store key, in the same record as the server values. Reading a resolver whose inputs did not change returns the stored result without calling the resolver.

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

### The resolver entry

```ts
// from libs/isograph-runtime/src/store.ts
/** A resolver's stored result on the record it was read on. */
export type ResolverEntry = {
  readonly kind: 'Resolver';
  /** What recomputes it: the field's reader artifact and its variables. */
  readonly artifact: ReaderArtifact<ResolverParam, unknown>;
  readonly variables: Variables;
  readonly result: ReadResult<unknown>;
  /** The epoch in which `result` last changed. */
  readonly timeUpdated: Epoch;
  /** The last epoch in which `result` was confirmed current. */
  readonly timeVerified: Epoch;
  /** What the last computation read, in read order. */
  readonly dependencies: ReadonlyArray<Dependency>;
};

/** Where a stored entry lives. */
export type EntryLocation = {
  readonly record: StoreLink;
  readonly storeKey: StoreKey;
};
```

A `ServerEntry` holds a value `normalize` wrote and the epoch it changed in. A `ResolverEntry` holds a result `read` computed, what it read, and what recomputes it: pico's derived node (`DerivedNode` with `inner_fn` and value, `DerivedNodeRevision` with `time_updated` and `time_verified`, and its dependency list; `crates/pico/src/execute_memoized_function.rs`). An entry is replaced, never mutated: `record.set(key, next)`.

What reads a stored resolver result: a top-level `read`; a parent's `Resolver` node while the parent computes; verification of a parent that depends on it; settlement (change 3); `snapshot` (change 4). `normalize`, `check`, and `garbageCollect`'s mark never read one; their walks name only server store keys.

### Dependencies

```ts
// from libs/isograph-runtime/src/store.ts
export type Dependency = RecordDependency | EntryDependency;

/** The read found `record` absent. Changed once the record exists. */
export type RecordDependency = {
  readonly kind: 'Record';
  readonly record: StoreLink;
};

/** The read looked up `storeKey` on `record`, which existed. */
export type EntryDependency = {
  readonly kind: 'Entry';
  readonly record: StoreLink;
  readonly storeKey: StoreKey;
  /** The entry's `timeUpdated` when read; `null` when the key was absent. */
  readonly observed: Epoch | null;
};
```

A dependency is a location, as pico's is a key (`NodeKind::Source(Key)`, `NodeKind::Derived(DerivedNodeId)`, `crates/pico/src/dependency.rs:11-15`), not a reference to an entry: an entry is replaced on every verification and deleted with its record.

A dependency changed when:

- `Record`: the record exists.
- `Entry`, record now absent: always.
- `Entry`, key now absent: `observed !== null`.
- `Entry` on a `ServerEntry`: `entry.timeUpdated !== observed`.
- `Entry` on a `ResolverEntry`: after verifying that entry, `entry.timeUpdated !== observed`.

Delta from pico: pico asks whether a dependency's `time_updated` is newer than the epoch it was recorded at (`execute_memoized_function.rs:192-206`). Here the observed epoch is compared for equality, because `garbageCollect` deletes entries and a deleted entry has no `time_updated` to compare; pico never deletes a source (pico.md, Source: `remove` keeps the node and stamps an epoch). Equality is exact: an entry written after the read has a later `timeUpdated`, an entry deleted and recreated was recreated in a later epoch (`garbageCollect` advances it), and an unchanged entry keeps its `timeUpdated`.

### Verification

```ts
// from libs/isograph-runtime/src/read.ts
/** The stored result of `artifact` on `root` with `variables`, and the dependency a parent records on it. */
type ResolverRead = {
  readonly result: ReadResult<unknown>;
  readonly dependency: Dependency;
};

function readResolver(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  variables: Variables,
): ResolverRead {
  const record = store.records.get(root.entityName)?.get(root.id);
  if (record === undefined) {
    return {
      result: {
        kind: 'MissingData',
        reason: `No record for ${recordName(root)}`,
        recordLink: root,
      },
      dependency: { kind: 'Record', record: root },
    };
  }
  const storeKey = resolverStoreKey(artifact.selectableName, variables);
  const entry = verifiedEntry(
    store,
    record,
    { record: root, storeKey },
    artifact,
    variables,
  );
  return {
    result: entry.result,
    dependency: {
      kind: 'Entry',
      record: root,
      storeKey,
      observed: entry.timeUpdated,
    },
  };
}

/** The entry at `location`, brought up to date in `store.epoch`. */
function verifiedEntry(
  store: Store,
  record: StoreRecord,
  location: EntryLocation,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  variables: Variables,
): ResolverEntry {
  const existing = record.get(location.storeKey);
  if (existing === undefined) {
    return compute(store, record, location, artifact, variables, null);
  }
  if (existing.kind === 'Server') {
    throw new Error(
      `Store record ${recordName(location.record)} holds a server value under the resolver store key "${location.storeKey}".`,
    );
  }
  if (existing.timeVerified === store.epoch) {
    return existing;
  }
  if (
    existing.dependencies.some((dependency) =>
      dependencyChanged(store, dependency),
    )
  ) {
    return compute(
      store,
      record,
      location,
      existing.artifact,
      existing.variables,
      existing,
    );
  }
  const verified: ResolverEntry = { ...existing, timeVerified: store.epoch };
  record.set(location.storeKey, verified);
  return verified;
}

function dependencyChanged(store: Store, dependency: Dependency): boolean {
  const record = store.records
    .get(dependency.record.entityName)
    ?.get(dependency.record.id);
  switch (dependency.kind) {
    case 'Record':
      return record !== undefined;
    case 'Entry': {
      if (record === undefined) {
        return true;
      }
      const entry = record.get(dependency.storeKey);
      if (entry === undefined) {
        return dependency.observed !== null;
      }
      switch (entry.kind) {
        case 'Server':
          return entry.timeUpdated !== dependency.observed;
        case 'Resolver':
          return (
            verifiedEntry(
              store,
              record,
              dependency,
              entry.artifact,
              entry.variables,
            ).timeUpdated !== dependency.observed
          );
      }
    }
  }
}

/** Reads `artifact`'s reader AST, calls its resolver, and stores the result, keeping
 * `previous`'s result and `timeUpdated` when the new result is equal. */
function compute(
  store: Store,
  record: StoreRecord,
  location: EntryLocation,
  artifact: ReaderArtifact<ResolverParam, unknown>,
  variables: Variables,
  previous: ResolverEntry | null,
): ResolverEntry {
  const dependencies: Dependency[] = [];
  const data = readSelectionSet(
    store,
    artifact.readerAst,
    location.record,
    variables,
    (dependency) => dependencies.push(dependency),
  );
  const result = data.kind === 'Ok' ? callResolver(artifact, location.record, data.value, variables) : data;
  const unchanged = previous !== null && resultsEqual(previous.result, result);
  const entry: ResolverEntry = {
    kind: 'Resolver',
    artifact,
    variables,
    result: unchanged ? previous.result : result,
    timeUpdated: unchanged ? previous.timeUpdated : store.epoch,
    timeVerified: store.epoch,
    dependencies,
  };
  record.set(location.storeKey, entry);
  return entry;
}
```

This is pico's `execute_memoized_function` (`crates/pico/src/execute_memoized_function.rs:59-105`): an entry verified in the current epoch is returned (`:77-83`); otherwise its dependencies are checked in recorded order, a resolver dependency is verified before it is compared (`derived_node_changed_since` calls `execute_memoized_function`, `:208-247`), the check stops at the first changed dependency (`.any`, `:190-200`), and a changed entry re-runs its body and is backdated when its value is equal (`update_derived_node`, `:137-181`). Deltas: the observed-epoch comparison above; a new entry's `timeUpdated` is `store.epoch` where pico takes the newest dependency's (`:131-134`), which is equivalent under equality comparison; dependencies are collected through the `track` parameter below where pico pushes onto an ambient `DependencyStack` (`dependency.rs:45-102`), because every read happens inside the runtime's own functions.

`read` keeps its v1 signature and becomes:

```ts
// from libs/isograph-runtime/src/read.ts
export function read<TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, TOutput>,
  root: StoreLink,
  variables: Variables,
): ReadResult<TOutput> {
  // The entry under `artifact.selectableName`'s resolver store key on `root` was computed by
  // `artifact`: one selectable of one entity has one reader artifact.
  return readResolver(store, artifact, root, variables).result as ReadResult<TOutput>;
}
```

The cast is the one place a stored `ReadResult<unknown>` is returned at the caller's type; the invariant is in its comment.

v1's walk gains a `track: (dependency: Dependency) => void` parameter on `readSelectionSet`, `readSelection`, `readStoreValue`, and `readLinkedValue`, passed down unchanged. `readStoreValue` calls `track({ kind: 'Record', record: root })` when the record is absent, and otherwise `track({ kind: 'Entry', record: root, storeKey: key, observed: entry?.timeUpdated ?? null })` before returning. The `Resolver` case:

Before (reader-ast.md):

```ts
// from libs/isograph-runtime/src/read.ts
    case "Resolver": {
      const resolved = read(store, selection.readerArtifact, root, childVariables(variables, selection.arguments));
      return resolved.kind === "MissingData" ? missingDataFor(selection.alias, root, resolved) : resolved;
    }
```

After:

```ts
// from libs/isograph-runtime/src/read.ts
    case 'Resolver': {
      const resolved = readResolver(
        store,
        selection.readerArtifact,
        root,
        childVariables(variables, selection.arguments),
      );
      track(resolved.dependency);
      return resolved.result.kind === 'MissingData'
        ? missingDataFor(selection.alias, root, resolved.result)
        : resolved.result;
    }
```

A computation that stops at the first `MissingData` records the dependencies read up to it, which are exactly what its result depends on.

### Results, equality, and a resolver that throws

```ts
// from libs/isograph-runtime/src/read.ts
export type ReadResult<T> = ReadOk<T> | ReadMissingData | ReadResolverThrew;

/** The resolver of `selectableName` on `recordLink` threw `error`. */
export type ReadResolverThrew = {
  readonly kind: 'ResolverThrew';
  readonly selectableName: string;
  readonly recordLink: StoreLink;
  readonly error: unknown;
};

/** `artifact.resolver` called with `data`; a throw becomes `ResolverThrew`. */
function callResolver(
  artifact: ReaderArtifact<ResolverParam, unknown>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ReadResult<unknown>;

/** `Ok`: values `===`. `MissingData`: equal `reason`, `recordLink` entity and id, and
 * `nestedReason`, recursively. `ResolverThrew`: errors `===`. Different kinds: unequal. */
function resultsEqual(a: ReadResult<unknown>, b: ReadResult<unknown>): boolean;
```

`allOk` stops at the first result whose `kind` is not `Ok`; a `ResolverThrew` passes up unchanged, a `MissingData` is wrapped as v1 wraps it. v1 lets a throw propagate out of `read` (reader-ast.md, Semantics). Here it is a stored result, because change 3 computes every retained query's resolvers inside `normalize`: a throw that propagated would abort `normalize` for every query in the store, and leave the other retained queries unsettled. Open decision 5.

Equality is upstream's for resolver values, `===` (`areEqualWithDeepComparison.ts:84-95`), as pico's is the value's `==`. A resolver that returns a string backdates when the string is equal; one that returns a new object never backdates, and every entry that reads it re-runs. Comparators are open decision 7.

### Tests

`test/read.test.ts` (reader-ast.md, Tests, Runtime) changes:

- "The store is unchanged by every read" becomes: `serverValues(store)` is unchanged by every read, and after `PetSummary` with `{ id: "1" }` the store holds resolver entries at `Query.__ROOT` `PetSummary____id___"1"`, `Pet:1` `PetName`, and `Pet:1` `RecentCheckins____n___2`.
- "A resolver that throws propagates out of `read`" becomes: `read` returns `ResolverThrew` with the error, the field's selectable name, and the record; its parent's result is the same `ResolverThrew` object.
- reader-ast.md asserts no call count across two reads (its one call-count case is a child whose data is `MissingData`, count 0). v1 calls every resolver on every `read` by construction (its `read` body calls `artifact.resolver`). The first case below fails against v1: that is the flip.

New, in `test/memo.test.ts`. Call counts come from `vi.spyOn(readerArtifact, 'resolver')`, which replaces the method on the one artifact object every `Resolver` node imports. Fixture sources added to store.md's fixture project:

```text
field Pet.Initial { name }
field Pet.Monogram { Initial }
field Pet.Signature { Initial nickname }
field Pet.Badge { Monogram Signature }
field Query.BadgePage($id: ID!) { pet(id: $id) { Badge } }

field Pet.FriendLabel { bestFriend { PetName } }
field Pet.FriendCard { FriendLabel tagline }
field Query.FriendPage($id: ID!) { pet(id: $id) { FriendCard } }

field Query.TwoNames {
  a: pet(id: "1") { PetName }
  b: pet(id: "1") { PetName }
}
```

with `entrypoint Query.BadgePage`, `entrypoint Query.FriendPage`, and `entrypoint Query.TwoNames`. `Initial` returns `data.name.slice(0, 1)`, `Monogram` returns `` `${data.Initial}.` ``, `Signature` returns `` `${data.Initial} ${data.nickname ?? ""}` ``; every other fixture resolver returns `data` (store.md's convention).

- A second read with no write between: every resolver's call count is unchanged, and the result is `===` the first read's. (v1: every count doubles.)
- Diamond, `Pet:1.name` `Rex` to `Max`: `Initial`, `Monogram`, `Signature`, `Badge`, and `BadgePage` are each called once; `Badge` receives `{ Monogram: "M.", Signature: "M " }` on its one call; `Initial` is called once although `Monogram` and `Signature` both select it.
- Diamond, `Rex` to `Ray`: `Initial` is called once and returns `"R"`; no other resolver is called, and `BadgePage`'s result is `===` the previous one.
- Diamond, only `nickname` changes: `Signature`, `Badge`, `BadgePage` once each; `Initial` and `Monogram` zero.
- Chain across fragments, `Pet:1.bestFriend` is `Pet:2`; `FriendPage` with `{ id: "1" }` and `PetSummary` with `{ id: "2" }` read, then `Pet:2.name` changes, then both read again in either order: `PetName` on `Pet:2` is called once; `FriendLabel`, `FriendCard`, `FriendPage`, and `PetSummary` once each; `RecentCheckins` zero.
- Unrelated write: only `Pet:1.tagline` changes: `FriendCard` and `FriendPage` once; `FriendLabel` and `PetName` zero.
- Same resolver selected twice: a first read of `TwoNames` calls `PetName` once, and `a.PetName` and `b.PetName` are `===`.
- An identical response normalized again: no resolver is called on the next read, and `store.epoch` is unchanged.
- Missing then present: a read of `FriendPage` with `Pet:2` lacking `nickname` is `MissingData`; after `nickname` is written, the next read is `Ok` and calls `PetName`.
- Absent record: `FriendPage` read while `Pet:1.bestFriend` links a `Pet:9` deleted by hand is `MissingData` naming `Pet:9`; after `normalize` writes `Pet:9` with `name` and `nickname`, the next read is `Ok`.
- A deleted record: after `garbageCollect` deletes `Pet:2`, the next read of a stored `FriendPage` result is `MissingData` with `recordLink` `Pet:2`, not the stored `Ok`.
- `resolverStoreKey`: `RecentCheckins` with `{ n: 2 }` is `RecentCheckins____n___2`; `{}` is `RecentCheckins`; `{ b: 1, a: "x" }` is `X____a___"x"____b___1`.

## Ordering

A write can make several stored results stale at once. Recalculation must run in dependency order: no entry recomputes from a stale input, and no entry recomputes twice for one write. The design guarantees both with pico's pull-based verification, and settlement (change 3) and observers (change 4) add nothing that could break it.

### Worked example: a diamond

`Badge` (A) reads `Monogram` (B) and `Signature` (C); both read `Initial` (D); `Initial` reads `Pet:1.name`. After a first read at epoch 1, the entries on `Pet:1`:

```text
Initial    deps [Entry(Pet:1, name, observed 1)]
Monogram   deps [Entry(Pet:1, Initial, observed 1)]
Signature  deps [Entry(Pet:1, Initial, observed 1), Entry(Pet:1, nickname, observed 1)]
Badge      deps [Entry(Pet:1, Monogram, observed 1), Entry(Pet:1, Signature, observed 1)]
BadgePage  deps [Entry(Query:__ROOT, pet____id___"1", observed 1), Entry(Pet:1, Badge, observed 1)]
```

`normalize` writes `name: "Max"`: `Pet:1.name` gets `timeUpdated: 2`, `store.epoch` is 2. Verifying `BadgePage`:

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

Every entry is computed after the entries it reads were verified in epoch 2, because the only way a computation reads a resolver result is `readResolver`, which verifies first. Every entry is computed at most once in epoch 2, because a computed entry has `timeVerified === store.epoch` and every later visit returns it. `Initial` is reached on two paths and computed once. With `Rex` to `Ray`, `Initial` computes `"R"`, is backdated (`timeUpdated` stays 1), and `Monogram`, `Signature`, and `Badge` see `observed 1` and do not compute.

### Worked example: a chain across fragments

`FriendPage` reads `FriendCard` on `Pet:1`, which reads `FriendLabel` on `Pet:1`, which reads `PetName` on `Pet:2` through `bestFriend`. A second query, `PetSummary` with `{ id: "2" }`, reads `PetName` on `Pet:2` directly. Both are stored at epoch 1. `normalize` writes `Pet:2.name` at epoch 2.

Verifying `FriendPage` first: `FriendCard` verifies `FriendLabel`, which checks `Pet:1.bestFriend` (unchanged), then verifies `PetName` on `Pet:2`, which computes (call 1, a new object), so `FriendLabel` computes, so `FriendCard` computes (reading `tagline`, unchanged), so `FriendPage` computes. Verifying `PetSummary` next: its `PetName` dependency is verified in epoch 2 and its `timeUpdated` is 2, so `PetSummary` computes and reads the stored `PetName` without calling it. Verifying in the other order gives the same calls: `PetSummary` computes `PetName`, and `FriendLabel`'s check later finds it verified. A write in one fragment's input does not schedule anything in another fragment; verification discovers it, through the shared entry.

### The options

- Pull-based verification, pico's (held). An entry verifies its dependencies, in recorded order, before itself (`execute_memoized_function.rs:183-247`), and is verified at most once per epoch (`node_verified_in_current_epoch`, `crates/pico/src/database.rs:246-251`). Order is a depth-first post-order over the current dependencies, computed by recursion; nothing stores it. Dependencies the new computation no longer reads (a link moved from `Pet:2` to `Pet:3`) are never verified. Cost: verifying a stored result whose inputs did not change walks its whole dependency graph once per epoch, comparing epochs.
- Push-based dirty marking, then pull. A write follows reverse edges (each entry's dependents) and marks every entry it reaches; verification then skips the dependency walk of an entry not marked since its last verification. The order is still the pull's: marks only say which entries might be stale and which observers to look at. Cost: a dependents set on every server and resolver entry that a stored result read, kept equal to the inverse of every dependency list on every computation, and entries for absent keys to hold the dependents of a read that found nothing (pico's missing-key dependency, `refactors/pending/pico-missing-key-dependency.md`). Benefit: a write's cost is proportional to what it reaches, not to the whole graph. Open decision 2.
- Topological order by height. Each entry has a height, one more than its highest dependency; a write enqueues the entries that read what changed, and recomputation takes the lowest height first, enqueuing dependents of each entry whose result changed. It recomputes each entry once and after its inputs. Cost: heights must be maintained as dependencies change with the data (a link change can raise an entry's height and every height above it), the queue is store state, and it recomputes every reachable entry eagerly whether or not anything will read it. Pico does not do this.

The failure mode all three avoid is push without order: `Initial` changes and notifies `Monogram` and `Signature`; `Monogram` recomputes and notifies `Badge`; `Badge` recomputes reading the old `Signature`; `Signature` recomputes and notifies `Badge` again. `Badge` runs twice, once from a stale input.

Held: pull, as pico. Push marks are an optimization of which entries to verify, and they can be added later without changing the order (open decision 2). Height ordering adds state and work pico shows is unnecessary.

### The same resolver selected twice

Two selections of one client field on one record with equal argument values are one store key, so one entry: computed once, read by both parents, each parent recording a dependency on the same location. This holds across fragments and across queries. Unequal argument values are two keys and two entries. Aliases do not enter the key.

### Reads during recalculation

- A level 2 callback never runs during recalculation: change 4 calls callbacks only after settlement has verified every retained and observed entry. A callback that calls `read` or `snapshot` receives a result verified in the current epoch. A callback that calls `normalize` runs a nested write, settlement, and notification; when it returns, the outer notification loop compares each remaining observer's stored result with the one it last notified, so no observer is called twice for one result or receives an older result after a newer one.
- A callback in a `read` caller during recalculation is the same case: `read` itself is a verification, so a `read` between two steps of settlement makes the entries it reaches current, and settlement later finds them verified.
- A resolver that reads or writes the store (through a closure, since its parameter holds no store) during its own computation breaks the guarantee: its inner `read` records no dependency on the outer entry, and an inner `normalize` advances the epoch mid-verification. Open decision 8.
- Cycles: none. A resolver entry reads only entries of client fields its reader AST selects, the reader import graph is acyclic (reader-ast.md, Reader artifact imports), and a client field that selects itself through data (`field Pet.X { bestFriend { X } }`) is an `InlineCycle` query error (query-generation.md, Cycles), so no entry is reached again while it computes.

## 3. Level 1: `normalize` retains and settles

### `normalize`

Before (store.md):

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
/** What `normalize` needs from an `entrypoint.ts` artifact. */
export type Entrypoint<TOutput> = {
  readonly normalizationAst: NormalizationAst;
  readonly readerArtifact: ReaderArtifact<ResolverParam, TOutput>;
};

/**
 * Writes `response`, retains the query, and settles: every retained query's resolver results,
 * this one's included, are current when it returns, and observers have been called.
 * Throws on a response that does not match the AST (store.md, Errors).
 */
export function normalize<TOutput>(
  store: Store,
  entrypoint: Entrypoint<TOutput>,
  response: JsonObject,
  variables: Variables,
): Retention<TOutput>;

/** v1's `normalize`, renamed and not exported from the package: writes only. */
export function writeResponse(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void;
```

`Entrypoint` is structural: the generated `entrypoint.ts` object (`kind`, `name`, `normalizationAst`, `readerArtifact`, artifact-set-and-types.md, Per entrypoint) is assignable, and `TOutput` is inferred from its reader artifact's declared type. No compiler change.

The package index exports `normalize` and no longer exports a write without settlement, so a write cannot leave level 1 stale.

### Retention

```ts
// from libs/isograph-runtime/src/retention.ts
/** One `normalize` call's hold on its query's records and its entrypoint's resolver result. */
export type Retention<TOutput> = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly artifact: ReaderArtifact<ResolverParam, TOutput>;
  /** `ast.root` and `resolverStoreKey(artifact.selectableName, variables)`. */
  readonly location: EntryLocation;
};

/** Ends `retention`: its records and resolver results are no longer kept by it. */
export function dispose(
  store: Store,
  retention: Retention<unknown>,
): void;
```

`Store` gains `readonly retained: Set<Retention<unknown>>`; `createStore` starts it empty. This is upstream's `retainQuery` and `unretainQuery` (`garbageCollection.ts:40-63`) and its `RetainedQuery` (`:20-24`) with the entrypoint's reader artifact added, without the release buffer (store.md, `garbageCollect`). Normalizing retains, so retaining and computing happen together; upstream retains at the request (`makeNetworkRequest.ts:464-493`) and reads in render.

`normalize` does, in order:

1. `writeResponse(store, entrypoint.normalizationAst, response, variables)`.
2. Settles (below) with the new retention included.
3. Adds the retention to `store.retained`, after settlement succeeds, so a throw leaves nothing retained that the caller holds no handle to.
4. Notifies observers (change 4).

A resolver entry is live while a retention's `location` or an observed location reaches it through `Entry` dependencies. Several retentions of one entrypoint with equal variables have one `location`, and verifying it twice in one epoch is free; a resolver shared by several queries is one entry reached from several roots. The reference count of the user's design is therefore implicit: an entry stays live while any retention or observed location reaches it, and `dispose` of one retention leaves it live while another does. Explicit counts are needed only with push marks (open decision 2).

### Settlement

```ts
// from libs/isograph-runtime/src/settle.ts
/** Verifies the resolver result at every retained location, then at every observed location. */
function settle(store: Store): void;
```

Each retained location is verified with `readResolver(store, retention.artifact, retention.location.record, retention.variables)`; each observed location with its stored entry's `artifact` and `variables` when one is stored. Verification is change 2's, so the order is pull's (Ordering), and an entry reached from several roots is verified once. After `settle` returns, every live entry has `timeVerified === store.epoch`, which is what change 4 reads.

The entries a new retention reaches are computed here, children before parents: the root's computation reads each `Resolver` node through `readResolver`, which computes the child first. This is the bottom-up order the user's design creates them in on the way back up the response (Resolver markers, below).

A `normalize` that changed nothing leaves `store.epoch` unchanged, and settlement returns at the first check of each root.

Cost: every `normalize` verifies every retained query's resolver graph, walking each graph's dependencies once when the epoch advanced. Upstream walks every subscription too (`subscribe.ts:67`) and re-reads the overlapping ones in full (`:117`).

### Eager computation

Settlement computes every resolver result a retained query reaches, whether or not any component renders it. That matters when retained queries outnumber rendered ones (a prefetched route, a screen kept retained after navigation) or when resolvers are expensive, and it costs nothing extra when every retained query is on screen. Open decision 3.

### Garbage collection

Before (store.md): `garbageCollect(store: Store, retained: ReadonlyArray<RetainedQuery>): void`.

After: `garbageCollect(store: Store): void`, reading `store.retained`.

- Records: store.md's mark and sweep, from each retention's `ast` and `variables`. An entry a retention reaches never loses a record: it reads only what its reader ASTs select along current links, and the retained query's normalization AST is the merge of those reader ASTs with the same variables (query-generation.md), so every record it reads is marked. An entry reached only from an observed location can lose its records (Level 2).
- Resolver entries: entries reachable from retained and observed locations through `Entry` dependencies on resolver entries are marked; the sweep deletes every other resolver entry on a surviving record. Entries a plain `read` stored outside any retention are collected this way.
- `store.epoch` advances when anything was deleted (change 1).

`RetainedQuery` is removed; `Retention` replaces it.

### Resolver markers

The user's design has `normalize` walk the response to the leaves and, on the way back up, create the resolver subscriptions each record needs. That needs `normalize` to know which resolvers the query's readers select at each position. The normalization AST does not say: query generation inlines a client field's selections and keeps no trace of it (query-generation.md, Merging). Two ways:

- (a) Root only (held). `normalize` knows the entrypoint's reader artifact (`Entrypoint.readerArtifact`), and settlement computes the root entry; computing it creates every entry the query reaches, children first, by the same recursion that orders recalculation. Nothing new in any artifact.
- (b) A marker per position. Each `NormalizationSelectionSet` gains the client fields the readers select on that record, `{ kind: "Resolver", readerArtifact, arguments }`, with `readerArtifact` imported and `arguments` in the operation's variables with defaults applied (what query generation substituted when it inlined the field), listed children first; `normalize` creates each entry when it leaves the record. This is candidate 6 of `refactors/pending/runtime/normalization-ast-mount-markers.md` ("A marker is needed only if a subscription must be created for a record the response introduces").

Reason for (a): every marker in (b) is already reachable from the root through reader ASTs, so (b) adds no entry (a) does not create, and creates them in the same order. (b) also goes stale: a later response that moves `Pet:1.bestFriend` from `Pet:2` to `Pet:3` makes `FriendLabel` read `PetName` on `Pet:3`, where no marker of `FriendPage`'s response ran, so the live set has to follow dependencies anyway. And `normalization_ast.ts` would import reader artifacts, and through them the user's source modules, so loading a query's normalization AST would load its resolvers. Open decision 1.

### Tests

`test/level1.test.ts`, over the fixture of change 2, through `normalize` with the fixture's entrypoint artifacts:

- After `normalize` of `BadgePage`, every resolver it reaches was called once and no `read` was made; `read` of `BadgePage` then calls nothing and returns the stored result.
- Two retentions of `BadgePage` with `{ id: "1" }`: a later change calls each affected resolver once; `dispose` of one keeps the entries current on the next `normalize`; `dispose` of both, then `garbageCollect`, deletes `Pet:1`'s resolver entries with `Pet:1`.
- The diamond and chain cases of change 2, with the second read replaced by a `normalize` of the changed response: the same call counts, all inside `normalize`.
- `FriendPage` retained, then a `PetDetail` response for `Pet:1` whose `bestFriend` is `Pet:3` (`PetDetail` selects no `nickname` under `bestFriend`): inside that `normalize`, `FriendLabel` re-reads on `Pet:3`, and `FriendPage`'s stored result is `MissingData` naming `Pet:3`; `garbageCollect` keeps `Pet:3`, which `FriendPage`'s AST reaches through the new link.
- A throwing resolver in one retained query: `normalize` returns, that query's root holds `ResolverThrew`, and another retained query's results are current.
- A resolver entry stored by a plain `read` outside any retention is deleted by `garbageCollect`; one under a retained root is kept.
- store.md's GC tests retain through `normalize` and release through `dispose` instead of passing `retained`; store.md's other tests call `writeResponse` from `../src/normalize`.

## 4. Level 2: observers

```ts
// from libs/isograph-runtime/src/observe.ts
/** Calls `onChange` after each settlement that changed the result stored at `location`.
 * Returns the function that stops it. */
export function listen(
  store: Store,
  location: EntryLocation,
  onChange: () => void,
): () => void;

/** The result stored at `location` as of the last settlement, or `undefined` when none is
 * stored. Computes nothing. */
export function snapshot(
  store: Store,
  location: EntryLocation,
): ReadResult<unknown> | undefined;
```

```ts
// from libs/isograph-runtime/src/store.ts
/** The observers of one location. */
export type Observed = {
  readonly location: EntryLocation;
  readonly callbacks: Set<() => void>;
  /** The stored result the callbacks were last called for, or were registered at. */
  notified: ReadResult<unknown> | undefined;
};
```

`Store` gains `readonly observed: Map<string, Observed>`, keyed by `stableStringify([entityName, id, storeKey])`. `listen` adds to it with `notified` set to the current stored result; the returned function removes the callback, and the `Observed` when it has none left.

Notification, after settlement in `normalize`: for each `Observed` in insertion order, the stored result is compared by identity with `notified`; when different, `notified` is set and each callback is called once. Identity suffices because a backdated entry keeps its result object. Callbacks that throw do not stop the loop; after it, one error is rethrown as is and several as an `AggregateError` (open decision 9).

What each level owns:

- Level 1 holds results, dependencies, and the epochs that order recalculation. It runs user code (resolvers).
- Level 2 holds locations, callbacks, and the last-notified identity. It runs no resolver, and is notified only after level 1 settled, once per changed location per settlement.

A parent component never re-renders with a stale child: every entry is verified before the first callback, so any `snapshot` during any re-render returns the settled value, whatever the order callbacks run in. React additionally batches the `setState`s that `useSyncExternalStore` calls in one notification loop into one render.

What React needs, and gets without `@isograph/runtime` knowing React: `useSyncExternalStore(subscribe, getSnapshot)` where `subscribe(onStoreChange)` returns an unsubscribe function (`listen`), and `getSnapshot` returns the same value while nothing changed and is pure and cheap (`snapshot`, a lookup). How a component learns its location (the entry a parent's result names) is the React package's, with `@component` (Later).

An observer keeps its entry verified (settlement verifies observed locations) but retains no record: data stays only while a retention keeps it. A location whose record `garbageCollect` deleted has no stored result, and `snapshot` returns `undefined`.

### Tests

`test/level2.test.ts`:

- `listen` then an unchanged `normalize`: no call. A changed one: one call, after which `snapshot` returns the new result; inside the callback, `snapshot` already returns it and every resolver's call count is final.
- Two observers of `PetName` on `Pet:2` through different locations' queries (`FriendPage`, `PetSummary`): one write, `PetName` called once, each observer called once.
- An observer of `Initial` and one of `BadgePage`, `Rex` to `Ray`: neither is called.
- A callback that calls `normalize` with another change: the nested call notifies; each observer receives each result once.
- A callback that throws: the other observers are still called, then `normalize` throws it; two that throw give an `AggregateError` of both.
- The stop function: no later call; `snapshot` still works.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: "`pico-runtime.md` stays parked" is replaced by this doc as the step after v1. `Store`, `StoreRecord`, `ServerEntry`, `Epoch`, `createStore`, `jsonEqual`, and the two throw sites (change 1); `normalize` becomes `writeResponse` and the exported `normalize` retains and settles, `garbageCollect` reads `store.retained`, and `RetainedQuery` becomes `Retention` (change 3); its test helpers `storeOf` and `serverValues`.
- `refactors/pending/runtime/reader-ast.md`: `ReaderArtifact.selectableName` and its printing; `ReadResult` gains `ReadResolverThrew`; `read` stores and reuses resolver results; "The store is unchanged by every read" and "A resolver that throws propagates out of `read`" change as change 2's Tests state. Its note that this doc's `computeResolver` and `readSelection` replace its `read` is superseded by change 2.
- `refactors/pending/runtime/normalization-ast-mount-markers.md`: candidate 6 is answered by Resolver markers (held: no marker); its open question on how this doc models a mount is answered by `Retention` and settlement.

## Later

- `@component`: a component is a resolver result, stored and settled like any other; a mounted component observes its own entry.
- Client pointers, `@loadable`, refetch queries (reader-ast.md, Later): each adds entries or retentions, and the markers of normalization-ast-mount-markers.md.
- Optimistic writes: a layer of server entries that settlement reads through.

## Follow-ups

The user's design notes, recorded for later. None of them is part of Step A or changes 1 to 4, and none is approved. Upstream paths here follow the doc's convention; `isograph-react-disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`, and `isograph-disposable-types/` paths are under `isograph/libs/isograph-disposable-types/src`.

### 1. Constructors and destructors at normalization time

A normalization-time subscription is a constructor that `normalize` runs when a retained query first reaches a resolver (the first computation of its entry in settlement), paired with a destructor that runs when the last handle referencing that entry is released. Lifetime follows retain and release (`normalize` and `dispose`), not render.

Upstream ties the lifetime of such things to render and effects: `useDisposableState` creates the item during render and disposes it on unmount (`isograph-react-disposable-state/useDisposableState.ts:16`), and the pagination hooks hold one disposable per page through `useUpdatableDisposableState` (`loadable-hooks/useSkipLimitPagination.ts:96-99`, `loadable-hooks/useConnectionSpecPagination.ts`).

A destructor needs to know when the last reference goes away. Retention (change 3) keeps reference counts implicit (an entry is live while any retention or observed location reaches it), so constructors and destructors need explicit counts per entry, as open decision 2 (b) has.

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

Two retained queries that reach the same pointer at the same record share one handle and one follow-up query. The key is the record, the selectable, and the variables: the resolver store key on that record (`stableStringify([entityName, id, resolverStoreKey(selectableName, variables)])`, the key `Store.observed` uses). The handle's reference count is the number of retained parents that reach it.

### 6. A destructor and an in-flight follow-up

Upstream cannot cancel a request: `IsographNetworkFunction` is `(operation, variables) => Promise<any>` and takes no abort signal (`IsographEnvironment.ts:103-106`); dispose only stops the response from being normalized (`makeNetworkRequest.ts:235`). A network function that accepts an `AbortSignal` lets the destructor of a follow-up cancel its request.

### 7. Deferral

The user's direction: do not do all the work at construct and destruct time.

- Construction: `normalize` computes the result when the constructor runs, and defers setting up subscriptions to a scheduler. The runtime stays free of browser globals because the caller supplies the scheduler: `createStore` takes a function that receives a task and runs it later. The React package passes one that calls `requestIdleCallback` (or an equivalent where that global is absent); tests pass one that queues tasks and runs them when the test says.
- Destruction: garbage collecting on every release is wasteful. A release adds the handle to a buffer, and collection runs later as one batch, through the same scheduler or when the buffer exceeds its size. A handle that is retained again before the batch runs is not destroyed. Upstream's release buffer is the related mechanism: `unretainQuery` pushes the released query onto `gcBuffer` and reports overflow past `gcBufferSize` (`garbageCollection.ts:40-53`), whose default is 10 (`IsographEnvironment.ts:177`, `:205-206`), and overflow triggers collection (`makeNetworkRequest.ts:565-577`). v1 removed the buffer as not needed yet (`refactors/pending/runtime/store.md`, `garbageCollect`).

## Open decisions

### 1. Resolver markers in the normalization AST

- (a) Root only: `normalize` takes the entrypoint, and settlement creates the rest through reader ASTs (held, Resolver markers).
- (b) A marker per selection set, as the user's design describes. Cost: artifacts import reader artifacts and source modules, the marker set goes stale as links change, and it duplicates what reader ASTs already say.

### 2. Which entries settlement visits

- (a) Every retained and observed root, pull only (held). Cost: per `normalize`, a walk of every live dependency graph comparing epochs.
- (b) Dirty marks along dependents, then pull from marked roots. Each live entry and each server entry a live entry read holds its dependents; reference counts (retentions plus live dependents) decide liveness and detach edges at zero. Cost: edge maintenance on every computation, entries for absent keys, and the counts; benefit: a write costs what it reaches. The order is (a)'s.
- (c) Height-ordered queue. Cost: maintained heights and a queue in the store; no laziness.

### 3. Eager computation for retained queries nobody renders

- (a) Every retained query is settled after every write (held; the user's goal: renders do no work).
- (b) Only roots that have an observer are settled; a retained root without one is verified when `listen` attaches, in the subscribe call, not in render. Cost: the first render of a newly observed location can get a stale `snapshot` before `listen` runs.
- (c) Per retention, a choice between (a) and (b). Cost: an option on `normalize`.

### 4. Entry representation

- (a) Tagged entries with a `timeUpdated` per server value (held, pico's sources).
- (b) Server values stay untagged and unchanged; `normalize` keeps an equal value's reference, a dependency records the value it read, and verification compares it with `===`; a resolver entry is told apart by `instanceof` of a class. Cost: dependencies hold references to old values, and `store.epoch` is still needed for the verified-this-epoch check, so `Store` changes anyway.

### 5. A resolver that throws

- (a) Stored as `ResolverThrew`, propagated to parents like `MissingData` (held). Reason: settlement runs every retained query's resolvers inside `normalize`.
- (b) Propagate the exception, v1's behavior. Cost: one broken resolver aborts `normalize` for every query and leaves level 1 unsettled.

### 6. Garbage collection from live resolver dependencies

The user's idea, explicitly unsettled: records and fields linked to the resolver entries that read them, so collection knows what is live without walking ASTs.

- (a) Walk retained normalization ASTs, as v1 (held).
- (b) A record is live when a live entry depends on it. Needs option 2 (b)'s dependents. Cost: a record a retained query fetched but no live entry read is deleted (fields after a `MissingData`, records a reader stops before), so `check` of that retained query reports `MissingData` and a refetch follows; retention no longer means what store.md and upstream mean (`garbageCollection.ts:88-119`).

### 7. Result equality

- (a) `===`, upstream's (held).
- (b) A comparator the field declares. Cost: a directive, and v1 parses none.
- (c) Structural equality of resolver outputs. Cost: outputs can be functions, class instances, or React elements.

### 8. A resolver that reads or writes the store

- (a) No guard: the resolver's parameter holds no store, and one reached through a closure is undefined behavior.
- (b) The store records that a resolver is running; `read`, `normalize`, and `garbageCollect` throw while it does. Cost: a phase field on `Store`.

Not held.

### 9. Callback errors

- (a) Call every callback, then rethrow one error or an `AggregateError` (held). Reason: one failing observer must not leave others stale.
- (b) Propagate the first throw. Cost: later observers miss the change until the next write.

### 10. Step A

- (a) Ship it before change 1, for a React demo.
- (b) Skip it. Cost: no React demo until change 4.

Not held: the user's choice.
