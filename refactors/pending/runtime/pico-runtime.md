# A pico-style memoized client runtime

Status: parked. Not part of v1. v1 generates artifacts and tests against them.

`@isograph/runtime` (`libs/isograph-runtime`) is upstream Isograph's runtime (`isograph/libs/isograph-react/src/core`) with the changes below and no React dependency. Everything this doc does not list keeps upstream's behavior: `getDataIdOfNetworkResponse` (`cache.ts:599-642`), `getNetworkResponseKey` (`cache.ts:538-591`), the normalization AST, `Arguments` and `Variables`, the network function's shape, `check` (`check.ts:43-215`), retained queries with the release buffer (`garbageCollection.ts:40-63`), and `generateChildVariableMap` (`read.ts:465-490`).

## End-user experience

```ts
// from demos/pet-demo/src/main.ts
import {
  createStore,
  fetchEntrypoint,
  subscribeEntrypoint,
} from '@isograph/runtime';
import homeRoute from './__isograph/Query/HomeRoute/entrypoint';

const store = createStore({
  networkFunction: (operation, variables) =>
    fetch('/graphql', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query: operation.text, variables }),
    }).then((response) => response.json()),
  log: null,
  gcBufferSize: 10,
});

const request = fetchEntrypoint(
  store,
  homeRoute,
  {},
  {
    shouldFetch: 'IfNecessary',
    optimisticResponse: null,
  },
);

const subscription = subscribeEntrypoint(store, homeRoute, {}, (result) => {
  switch (result.kind) {
    case 'Ok':
      render(result.value);
      return;
    case 'MissingData':
      renderLoading();
      return;
    case 'ResolverThrew':
      renderError(result.error);
      return;
  }
});

// subscription.initial is { kind: 'MissingData', ... } before the response lands.
// When it lands, the callback receives { kind: 'Ok', value }.

// later
subscription.dispose();
request.dispose();
```

With `field Pet.fullName { firstName lastName }` and `field Query.HomeRoute { pets { id fullName tagline } }`:

- A second response for the same query with identical data calls no callback and runs no resolver.
- A response that changes only `Pet:7.tagline` re-runs the selection set of `pets` on `Pet:7`, the selection set of `Query.HomeRoute`, and the `HomeRoute` resolver. `fullName` on `Pet:7` and every node for the other pets are reused without running. The callback receives a new array whose elements for the other pets are the same objects as before (`===`).
- A response that changes only `Pet:7.firstName` re-runs `fullName` on `Pet:7`. If `fullName` declares a comparator and the new string is equal, nothing above it re-runs and no callback fires.
- A change to a field nobody read (`Pet:7.age`) calls nothing and verifies nothing.

## 1. Field sources with epochs replace record-level `EncounteredIds`

Upstream marks a whole record as changed when any of its fields changed (`cache.ts:211-218`), and a read records every record it touches (`read.ts:155-159`).

```ts
// external isograph/libs/isograph-react/src/core/cache.ts
  if (recordHasBeenUpdated) {
    let encounteredRecordsIds = insertEmptySetIfMissing(
      mutableEncounteredIds,
      targetParentRecordLink.__typename,
    );

    encounteredRecordsIds.add(targetParentRecordLink.__link);
  }
```

Here every field of every record is a source with a `timeUpdated` epoch, and a reader depends on the sources it read.

```ts
// from libs/isograph-runtime/src/store.ts
export type Brand<T, TBrand extends string> = T & { readonly __brand: TBrand };

export type TypeName = Brand<string, 'TypeName'>;
export type DataId = Brand<string, 'DataId'>;
/** A field name plus its argument chunks, as `getStorageKey` writes it. */
export type StorageKey = Brand<string, 'StorageKey'>;
/** A store-wide counter. It advances once per committed transaction that changed a field. */
export type Epoch = Brand<number, 'Epoch'>;

export const ROOT_ID = '__ROOT' as DataId;

export type StoreLink = {
  readonly __typename: TypeName;
  readonly __link: DataId;
};

export type Store = {
  readonly records: Map<TypeName, Map<DataId, StoreRecord>>;
  /** The epoch of the last committed transaction that changed a field. */
  epoch: Epoch;
  phase: StorePhase;
  readonly derivedNodes: DerivedNodeIndex;
  /** Oldest first. */
  readonly optimisticLayers: OptimisticLayer[];
  readonly retainedQueries: Set<RetainedQuery>;
  readonly gcBuffer: RetainedQuery[];
  readonly gcBufferSize: number;
  /** The sequence the next subscription receives. */
  nextSubscriptionSequence: number;
  readonly networkFunction: NetworkFunction;
  readonly log: LogFunction | null;
};

/** A record exists once any field of it was written or read. */
export type StoreRecord = {
  readonly link: StoreLink;
  readonly fields: Map<StorageKey, FieldSource>;
};

/** One field of one record: the unit of change and of dependency. */
export type FieldSource = {
  readonly kind: 'FieldSource';
  readonly record: StoreRecord;
  readonly storageKey: StorageKey;
  /** The value without optimistic overrides. `Absent` until a write. */
  base: FieldValue;
  /** Oldest layer first. Reads see the last entry, else `base`. */
  overrides: OptimisticOverride[];
  /** The epoch in which the value reads see last changed. */
  timeUpdated: Epoch;
  /** Derived nodes whose current dependencies include this source. */
  readonly dependents: Set<DerivedNode>;
};

export type FieldValue = { readonly kind: 'Absent' } | PresentFieldValue;

export type PresentFieldValue =
  | { readonly kind: 'Scalar'; readonly value: ScalarValue }
  | { readonly kind: 'Linked'; readonly value: LinkedValue }
  | { readonly kind: 'Inline'; readonly value: InlineValue };

/** A JSON value from the response, including custom scalars that are objects. */
export type ScalarValue =
  | null
  | boolean
  | number
  | string
  | readonly ScalarValue[]
  | { readonly [key: string]: ScalarValue };

export type LinkedValue = null | StoreLink | readonly LinkedValue[];

/** An object of a type stored inline, with its selected fields. */
export type InlineValue = null | InlineObject | readonly InlineValue[];
/** A missing key is a field no query selected. */
export type InlineObject = ReadonlyMap<StorageKey, PresentFieldValue>;

export const ABSENT: FieldValue = { kind: 'Absent' };

export type StoreOptions = {
  readonly networkFunction: NetworkFunction;
  readonly log: LogFunction | null;
  /** Released retained queries kept before garbage collection runs. Upstream's default is 10. */
  readonly gcBufferSize: number;
};

/** Receives errors thrown by subscription callbacks and comparators. */
export type LogFunction = (error: unknown) => void;

/** An empty store at epoch 0 in `Idle`. */
export function createStore(options: StoreOptions): Store;

/** Upstream `getParentRecordKey` (`cache.ts:454-521`), renamed. */
export function getStorageKey(
  field:
    | ReaderScalarField
    | ReaderLinkedField
    | NormalizationScalarField
    | NormalizationLinkedField,
  variables: Variables,
): StorageKey;

/** Upstream `stableCopy` (`util.ts:44-59`), then `JSON.stringify`. */
export function stableStringify(variables: Variables): string;

/** The value reads see. */
export function effectiveValue(source: FieldSource): FieldValue {
  const newest = source.overrides.at(-1);
  return newest === undefined ? source.base : newest.value;
}
```

A read of a field that was never written creates the `StoreRecord` (if needed) and a `FieldSource` whose `base` is `Absent`, and records a dependency on it. A later write to that key advances its `timeUpdated`, and the reader re-runs. This is the semantics of `refactors/pending/pico-missing-key-dependency.md`. Garbage collection deletes an `Absent` source with no dependents.

A derived node is identified by `(kind, the reader AST object or artifact object, Root, Variables)`. Reader ASTs and artifacts are module constants, so object identity is the key.

```ts
// from libs/isograph-runtime/src/store.ts
/** Where a selection set or resolver reads its fields. */
export type Root =
  | { readonly kind: 'Record'; readonly link: StoreLink }
  | {
      readonly kind: 'Inline';
      readonly owner: StoreLink;
      readonly path: readonly InlinePathSegment[];
    };

export type InlinePathSegment =
  | { readonly kind: 'Field'; readonly storageKey: StorageKey }
  | { readonly kind: 'Index'; readonly index: number };

/** `${typename}:${id}`, then `.${storageKey}` or `.${index}` per path segment. */
export type RootKey = Brand<string, 'RootKey'>;
export function rootKey(root: Root): RootKey;
```

## 2. Inline storage for id-less types

Upstream stores every object without `id` as a separate record at a synthetic id built from the parent and field name (`cache.ts:628-641`).

```ts
// external isograph/libs/isograph-react/src/core/cache.ts
  let storeKey = `${parentRecordLink.__typename}:${parentRecordLink.__link}.${astNode.fieldName}`;
  if (index != null) {
    storeKey += `.${index}`;
  }
```

A linked field whose target type is listed in the config's `inline_types` is stored inline: the field's source holds the whole object (with only the selected subfields) as one `Inline` value. Any other linked field keeps upstream's normalization, including the synthetic id.

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "artifact_directory": "src",
  "inline_types": ["Date", "Money"]
}
```

The compiler writes the decision into every reader and normalization AST node as `storage: 'Normalized' | 'Inline'`. The runtime does not decide.

- `User:1.birthday` is one source holding `{ year, month, day }`. A reader of `birthday { year }` depends on that one source, so a change to `birthday.day` re-runs it.
- Writes merge: a response selecting `birthday { month }` adds `month` to the stored object and keeps `year`. Lists merge element by element at equal indices, and the new length wins. This keeps upstream's result, where a later write to a synthetic-id record overwrites only the selected fields.
- An inline value may contain links to normalized records. For a normalized object inside an inline value, the synthetic id's `parentKey` is that of the owning record followed by `.${storageKey}` or `.${index}` for each segment of the inline path.
- A type stored inline cannot be selected with `__link` (there is no `StoreLink`), which the compiler rejects. `field` selectables on an inline type are allowed; their root is `Root.Inline`.

## 3. Cached resolver results in derived nodes

Upstream does not cache resolver outputs: every read of a fragment calls every nested eager resolver again (`read.ts:600`), and `useResult` calls the top resolver on every render (`react/useResult.ts:64`).

```ts
// external isograph/libs/isograph-react/src/core/read.ts
      return {
        kind: 'Success',
        data: readerWithRefetchQueries.readerArtifact.resolver(firstParameter),
      };
```

Here every read of a selection set on a root, and every evaluation of a resolver on a root, is a derived node with recorded dependencies, a cached result, a `timeUpdated`, and a `timeVerified`.

```ts
// from libs/isograph-runtime/src/derivedNode.ts
export type DerivedNode = SelectionSetNode | ResolverNode;

/** The read of a selection set on one root. */
export type SelectionSetNode = DerivedNodeFields<SelectionSetData> & {
  readonly kind: 'SelectionSet';
  readonly selectionSet: ReaderSelectionSet;
  readonly root: Root;
  readonly variables: Variables;
};

/** A resolver applied to the read of its selection set on one root. */
export type ResolverNode = DerivedNodeFields<unknown> & {
  readonly kind: 'Resolver';
  readonly artifact: EagerReaderArtifact<unknown>;
  readonly root: Root;
  readonly variables: Variables;
};

type DerivedNodeFields<T> = {
  state: DerivedNodeState<T>;
  /** The last epoch in which a write reached this node through `dependents`. */
  markedDirtyAt: Epoch;
  readonly dependents: Set<DerivedNode>;
  readonly subscriptions: Set<Subscription>;
};

export type DerivedNodeState<T> =
  | {
      readonly kind: 'Executing';
      readonly previous: ComputedState<T> | null;
    }
  | ComputedState<T>;

export type ComputedState<T> = {
  readonly kind: 'Computed';
  readonly result: ReadResult<T>;
  /** The newest `timeUpdated` among the dependencies when the result last changed. */
  readonly timeUpdated: Epoch;
  /** The last epoch in which the result was confirmed current. */
  timeVerified: Epoch;
  /** In read order. */
  readonly dependencies: readonly Dependency[];
};

export type Dependency = FieldSource | DerivedNode;

export type SelectionSetData = { readonly [alias: string]: unknown };

export type ReadResult<T> =
  | { readonly kind: 'Ok'; readonly value: T }
  | MissingData
  | ResolverThrew;

export type MissingData = {
  readonly kind: 'MissingData';
  readonly root: Root;
  readonly storageKey: StorageKey;
};

export type ResolverThrew = {
  readonly kind: 'ResolverThrew';
  /** `EagerReaderArtifact.fieldName` of the resolver that threw. */
  readonly fieldName: string;
  readonly root: Root;
  readonly error: unknown;
};

export type DerivedNodeIndex = {
  readonly selectionSets: Map<
    ReaderSelectionSet,
    Map<DerivedNodeKey, SelectionSetNode>
  >;
  readonly resolvers: Map<
    EagerReaderArtifact<unknown>,
    Map<DerivedNodeKey, ResolverNode>
  >;
};

/** `rootKey(root)`, a NUL, then `stableStringify(variables)`. */
export type DerivedNodeKey = Brand<string, 'DerivedNodeKey'>;
```

For `Query.HomeRoute`:

- `Query.HomeRoute` on `Query:__ROOT` is a `ResolverNode`. Its one dependency is the `SelectionSetNode` of its reader AST on the same root.
- That selection set node depends on the source `Query:__ROOT.pets`, and on one `SelectionSetNode` per pet for the nested `{ id fullName tagline }`.
- Each pet's selection set node depends on the sources `id` and `tagline` and on the `ResolverNode` of `fullName` on that pet.
- `fullName` on `Pet:7` depends on the `SelectionSetNode` of `{ firstName lastName }` on `Pet:7`, which depends on two sources.

A resolver receives `data`, the result of its selection set node, and does not read the store, so its dependencies are exactly that node.

A `MissingData` or `ResolverThrew` result is cached like `Ok`. A parent that reads a child whose result is not `Ok` returns the child's result object unchanged. A resolver that throws is caught; its node's result is `ResolverThrew` until a dependency changes.

### Recording dependencies

```ts
// from libs/isograph-runtime/src/derivedNode.ts
/** The dependencies of the node whose body is running. */
type DependencyRecorder = {
  readonly dependencies: Dependency[];
  maxTimeUpdated: Epoch;
};

function recordDependency(
  recorder: DependencyRecorder,
  dependency: Dependency,
  timeUpdated: Epoch,
): void {
  recorder.maxTimeUpdated = Math.max(
    recorder.maxTimeUpdated,
    timeUpdated,
  ) as Epoch;
  if (recorder.dependencies.at(-1) !== dependency) {
    recorder.dependencies.push(dependency);
  }
}
```

The recorder is a parameter of every read function, created by `recompute` for the node whose body runs. Cycle detection uses the `Executing` state.

### Verification

Verification is pico's algorithm (`docs-website/docs/design-docs/pico.md`) plus one short-circuit: `markedDirtyAt <= timeVerified` means no write reached the node since it was verified, so the dependency walk is skipped (see change 5).

```ts
// from libs/isograph-runtime/src/derivedNode.ts
/** Brings `node` up to date in the current epoch and returns its result. */
export function verify<T>(store: Store, node: NodeOf<T>): ReadResult<T> {
  const state = node.state;
  switch (state.kind) {
    case 'Executing':
      throw new CycleError(node);
    case 'Computed': {
      if (state.timeVerified === store.epoch) {
        return state.result;
      }
      if (
        node.markedDirtyAt <= state.timeVerified ||
        !anyDependencyChanged(store, state)
      ) {
        state.timeVerified = store.epoch;
        return state.result;
      }
      return recompute(store, node, state);
    }
  }
}

function anyDependencyChanged(
  store: Store,
  state: ComputedState<unknown>,
): boolean {
  const since = state.timeVerified;
  return state.dependencies.some((dependency) => {
    switch (dependency.kind) {
      case 'FieldSource':
        return dependency.timeUpdated > since;
      case 'SelectionSet':
      case 'Resolver':
        verify(store, dependency);
        return computedState(dependency).timeUpdated > since;
    }
  });
}

/** Runs the body of `node`, then stores the result, backdating when equal. */
function recompute<T>(
  store: Store,
  node: NodeOf<T>,
  previous: ComputedState<T> | null,
): ReadResult<T> {
  node.state = { kind: 'Executing', previous };
  const recorder: DependencyRecorder = {
    dependencies: [],
    maxTimeUpdated: 0 as Epoch,
  };
  const result = computeOrRestore(store, recorder, node, previous);
  replaceReverseEdges(
    node,
    previous?.dependencies ?? [],
    recorder.dependencies,
  );
  const equal =
    previous !== null && resultsEqual(store, node, previous.result, result);
  node.state = {
    kind: 'Computed',
    result: equal ? previous.result : result,
    timeUpdated: equal ? previous.timeUpdated : recorder.maxTimeUpdated,
    timeVerified: store.epoch,
    dependencies: recorder.dependencies,
  };
  return node.state.result;
}

/** The body by kind. On a throw (only `CycleError`), restores `previous`, or removes a new node from the index, and rethrows. */
function computeOrRestore<T>(
  store: Store,
  recorder: DependencyRecorder,
  node: NodeOf<T>,
  previous: ComputedState<T> | null,
): ReadResult<T>;

/** Removes `node` from each old dependency's `dependents` and adds it to each new one's. */
function replaceReverseEdges(
  node: DerivedNode,
  previous: readonly Dependency[],
  next: readonly Dependency[],
): void;

/** `node.state` after `verify` returned. */
function computedState<T>(node: NodeOf<T>): ComputedState<T>;

/** `SelectionSetNode` when `T` is `SelectionSetData`, else `ResolverNode`. */
type NodeOf<T> = DerivedNode & { state: DerivedNodeState<T> };
```

The walk stops at the first changed dependency, so after a link changes from `Pet:7` to `Pet:8`, the node re-runs before it verifies the old `Pet:7` child. Writes cannot happen while a body runs (see `StorePhase`), so the node's `timeVerified` replaces pico's per-dependency epoch.

## 4. Backdating and resolver comparators

Upstream compares nested resolver values with `!==` when it decides whether a fragment changed (`areEqualWithDeepComparison.ts:84-95`), so a nested resolver that returns a new object makes the fragment differ.

```ts
// external isograph/libs/isograph-react/src/core/areEqualWithDeepComparison.ts
      case 'Resolver': {
        const key = field.alias;
        // @ts-expect-error
        const oldValue = oldItemObject[key];
        // @ts-expect-error
        const newValue = newItemObject[key];

        if (oldValue !== newValue) {
          canRecycle = false;
        }
        break;
      }
```

Here equal results keep the previous result object and the previous `timeUpdated`, and a field may declare a comparator (`isEqual` on `EagerReaderArtifact`).

```ts
// from libs/isograph-runtime/src/equality.ts
function resultsEqual<T>(
  store: Store,
  node: NodeOf<T>,
  previous: ReadResult<T>,
  next: ReadResult<T>,
): boolean {
  if (previous.kind === 'Ok' && next.kind === 'Ok') {
    switch (node.kind) {
      case 'SelectionSet':
        return selectionSetDataEqual(
          node.selectionSet,
          previous.value,
          next.value,
        );
      case 'Resolver': {
        const isEqual = node.artifact.isEqual;
        return (
          isEqual !== null &&
          runUserCode(store, () => isEqual(previous.value, next.value))
        );
      }
    }
  }
  if (previous.kind === 'MissingData' && next.kind === 'MissingData') {
    return (
      rootKey(previous.root) === rootKey(next.root) &&
      previous.storageKey === next.storageKey
    );
  }
  return false;
}

/** Same aliases; each value `===`, comparing arrays (plural fields) element by element at every list depth. */
function selectionSetDataEqual(
  selectionSet: ReaderSelectionSet,
  previous: SelectionSetData,
  next: SelectionSetData,
): boolean;
```

- A selection set's data is server values and child results, so one level of `===` is exact: a scalar is the stored reference when its source did not change, and a child result is the same object when the child backdated. It returns true when the node re-ran because of a dependency it reads but does not output, such as the `__typename` an inline fragment tests.
- Two `MissingData` results at the same root and storage key are equal, so a node still waiting for the same field does not notify.
- A resolver's output is never compared unless its artifact has `isEqual`. With a comparator, an equal output stops the parent selection set node, the parent resolver, and the subscriber.
- A comparator that throws is logged and treated as unequal.

Memoization replaces `mergeObjectsUsingReaderAst` (`areEqualWithDeepComparison.ts:55-121`), which upstream uses to recover old sub-objects after rebuilding the whole tree. A node that did not re-run returns its previous object, so a parent that re-runs builds a new object around unchanged children.

## 5. Dirty marks along reverse edges replace the subscription scan

Upstream loops over every subscription after a write (`subscribe.ts:63-109`); a fragment subscription whose last `EncounteredIds` overlaps the changed records (`subscribe.ts:164-189`) re-reads its whole fragment (`subscribe.ts:117`) and compares it with `mergeObjectsUsingReaderAst` (`subscribe.ts:138-161`).

```ts
// external isograph/libs/isograph-react/src/core/subscribe.ts
  environment.subscriptions.forEach((subscription) =>
    logAnyError(environment, { situation: 'calling subscriptions' }, () => {
      switch (subscription.kind) {
        case 'FragmentSubscription': {
          // TODO if there are multiple components subscribed to the same
          // fragment, we will call readButNotEvaluate multiple times. We
          // should fix that.
          if (
            hasOverlappingIds(
              recordsEncounteredWhenNormalizing,
              subscription.encounteredDataAndRecords.encounteredRecords,
            )
          ) {
            callSubscriptionIfDataChanged(environment, subscription);
          }
          return;
        }
```

Here a write pushes a dirty mark up `dependents` and collects the subscriptions it reaches, then each collected subscription pulls its node's result through `verify` (open decision 1).

Upstream's write sites call `callSubscriptions` directly after `normalizeData` (`makeNetworkRequest.ts:271`, `writeData.ts:55`, `optimisticProxy.ts:492`). Here writes happen inside a `transaction`, which notifies once at commit and guards reads during writes and during user code.

```ts
// from libs/isograph-runtime/src/transaction.ts
export type StorePhase =
  | IdlePhase
  | NotifyingPhase
  | {
      readonly kind: 'Transaction';
      readonly transaction: TransactionState;
      readonly resume: IdlePhase | NotifyingPhase;
    }
  | {
      /** A resolver or comparator is running. */
      readonly kind: 'RunningUserCode';
      readonly resume: IdlePhase | NotifyingPhase;
    };

export type IdlePhase = { readonly kind: 'Idle' };

/** Subscription callbacks are running. Insertion order is delivery order. */
export type NotifyingPhase = {
  readonly kind: 'Notifying';
  readonly queue: Set<Subscription>;
};

export type TransactionState = {
  /** `store.epoch + 1`. Every field this transaction changes gets it. */
  readonly epoch: Epoch;
  readonly changed: Set<FieldSource>;
};

/** Runs `body` as one transaction and notifies once at the end. Joins an open transaction. */
export function transaction<T>(
  store: Store,
  body: (transaction: TransactionState) => T,
): T {
  const phase = store.phase;
  switch (phase.kind) {
    case 'Transaction':
      return body(phase.transaction);
    case 'RunningUserCode':
      throw new StoreAccessDuringUserCodeError('transaction');
    case 'Idle':
    case 'Notifying': {
      const transaction: TransactionState = {
        epoch: (store.epoch + 1) as Epoch,
        changed: new Set(),
      };
      store.phase = { kind: 'Transaction', transaction, resume: phase };
      try {
        return body(transaction);
      } finally {
        commit(store, transaction, phase);
      }
    }
  }
}

function commit(
  store: Store,
  transaction: TransactionState,
  resume: IdlePhase | NotifyingPhase,
): void {
  if (transaction.changed.size > 0) {
    store.epoch = transaction.epoch;
  }
  const affected = markDirty(transaction.changed, transaction.epoch);
  switch (resume.kind) {
    case 'Notifying':
      affected.forEach((subscription) => resume.queue.add(subscription));
      store.phase = resume;
      return;
    case 'Idle': {
      const notifying: NotifyingPhase = {
        kind: 'Notifying',
        queue: new Set(affected),
      };
      store.phase = notifying;
      deliver(store, notifying);
      store.phase = resume;
      return;
    }
  }
}

/** Marks every node reachable from `changed` through `dependents`. Returns their subscriptions by `sequence`. */
function markDirty(
  changed: ReadonlySet<FieldSource>,
  epoch: Epoch,
): Subscription[] {
  const affected: Subscription[] = [];
  const pending: DerivedNode[] = [...changed].flatMap((source) => [
    ...source.dependents,
  ]);
  let node = pending.pop();
  while (node !== undefined) {
    if (node.markedDirtyAt !== epoch) {
      node.markedDirtyAt = epoch;
      affected.push(...node.subscriptions);
      pending.push(...node.dependents);
    }
    node = pending.pop();
  }
  return affected.sort((a, b) => a.sequence - b.sequence);
}

function deliver(store: Store, notifying: NotifyingPhase): void {
  let next = firstOf(notifying.queue);
  while (next !== undefined) {
    const subscription = next;
    notifying.queue.delete(subscription);
    if (subscription.node.subscriptions.has(subscription)) {
      const result = verify(store, subscription.node);
      if (result !== subscription.delivered) {
        subscription.delivered = result;
        callLogged(store, () => subscription.callback(result));
      }
    }
    next = firstOf(notifying.queue);
  }
}

/** The first element in insertion order. */
function firstOf<T>(set: ReadonlySet<T>): T | undefined;

/** Calls `body`; a throw goes to `store.log`. */
function callLogged(store: Store, body: () => void): void;
```

```ts
// from libs/isograph-runtime/src/errors.ts
/** A public read or `transaction` while a resolver or comparator runs. */
export class StoreAccessDuringUserCodeError extends Error {
  constructor(readonly operation: 'read' | 'transaction');
}

/** A public read while a transaction is open. */
export class StoreAccessDuringTransactionError extends Error {}

/** A derived node read itself while executing. */
export class CycleError extends Error {
  constructor(readonly node: DerivedNode);
}

/** A stored value's kind disagrees with the reader AST: the compiler and the store disagree. */
export class StoreShapeError extends Error {
  constructor(
    readonly root: Root,
    readonly storageKey: StorageKey,
    readonly found: FieldValue,
  );
}
```

Reads during a transaction throw `StoreAccessDuringTransactionError`, because a node verified mid-transaction would record a `timeVerified` equal to an epoch still receiving writes. Reads and transactions inside a resolver or comparator throw `StoreAccessDuringUserCodeError`. A subscription callback runs in `Notifying` and may read or open a transaction; that transaction commits at once and appends its affected subscriptions to the queue being delivered. A callback that throws is logged and does not stop delivery.

A subscription is called when the `ReadResult` object it last received is not the object `verify` returns. A backdated node returns the previous object, so a backdated subscription is not called.

## 6. Structural equality on writes

Upstream compares an incoming scalar with `!==` (`cache.ts:248-249`), so a list scalar or an object custom scalar counts as changed on every response.

```ts
// external isograph/libs/isograph-react/src/core/cache.ts
  targetStoreRecord[parentRecordKey] = networkResponseData;
  return existingValue !== networkResponseData;
```

Here a write compares structurally, and a source's epoch advances only when the value reads see changed.

```ts
// from libs/isograph-runtime/src/transaction.ts
export type WritePosition =
  /** Server data newer than every override except those of `keepOverridesOf`. */
  | {
      readonly kind: 'Base';
      readonly keepOverridesOf: ReadonlySet<OptimisticLayer>;
    }
  /** An optimistic response's data. */
  | { readonly kind: 'Layer'; readonly layer: OptimisticLayer };

export function writeField(
  transaction: TransactionState,
  source: FieldSource,
  value: PresentFieldValue,
  position: WritePosition,
): void {
  const before = effectiveValue(source);
  switch (position.kind) {
    case 'Base':
      source.base = fieldValueEqual(source.base, value) ? source.base : value;
      source.overrides = source.overrides.filter((override) =>
        position.keepOverridesOf.has(override.layer),
      );
      break;
    case 'Layer':
      source.overrides = [
        ...source.overrides.filter(
          (override) => override.layer !== position.layer,
        ),
        { layer: position.layer, value },
      ];
      position.layer.fields.add(source);
      break;
  }
  if (!fieldValueEqual(before, effectiveValue(source))) {
    source.timeUpdated = transaction.epoch;
    transaction.changed.add(source);
  }
}

/** Structural equality of stored values. Links compare `__typename` and `__link`. Object key order is ignored. */
export function fieldValueEqual(a: FieldValue, b: FieldValue): boolean;
```

When the incoming value equals the stored base, the stored reference is kept, so downstream `===` comparisons hold.

`normalize` returns nothing; upstream's `EncounteredIds` (`cache.ts:77-107`) is replaced by `writeField` recording each changed source in the transaction.

```ts
// from libs/isograph-runtime/src/normalize.ts
/** Writes `response` into the record `root` per `selections`. Called inside a transaction. */
export function normalize(
  store: Store,
  transaction: TransactionState,
  selections: NormalizationSelectionSet,
  response: NetworkResponseObject,
  variables: Variables,
  root: StoreLink,
  position: WritePosition,
): void;

/** Builds the `Inline` value for one inline linked field, merged with `existing` per `selections`. Nested normalized objects are written as records. */
function inlineValue(
  store: Store,
  transaction: TransactionState,
  existing: FieldValue,
  response: NetworkResponseValue,
  selections: NormalizationSelectionSet,
  variables: Variables,
  owner: StoreLink,
  path: readonly InlinePathSegment[],
  position: WritePosition,
): InlineValue;

/** Upstream `check` (`check.ts:43-215`) over `effectiveValue`, without recording dependencies. */
export function check(
  store: Store,
  selections: NormalizationSelectionSet,
  variables: Variables,
  root: StoreLink,
): CheckResult;

export type CheckResult =
  | { readonly kind: 'EnoughData' }
  | {
      readonly kind: 'MissingData';
      readonly root: Root;
      readonly storageKey: StorageKey;
    };
```

A `Linked` selection with `storage: 'Inline'` `writeField`s `{ kind: 'Inline', value: inlineValue(...) }`, where `existing` is `base` for a `Base` write and `effectiveValue` for a `Layer` write. Other selections normalize as upstream does, through `writeField`.

## 7. Optimistic overrides on each source replace the layer stack

Upstream reads every field through a `Proxy` that walks the store layers (`optimisticProxy.ts:46-98`).

```ts
// external isograph/libs/isograph-react/src/core/optimisticProxy.ts
  return new Proxy<StoreRecord>(
    {},
    {
      get(_, propertyName) {
        let currentStoreLayer: StoreLayer | null = childMostStoreLayer;
        while (currentStoreLayer != null) {
          const storeRecord =
            currentStoreLayer.data[link.__typename]?.[link.__link];
```

Here each source holds its overrides, and reads take `effectiveValue`.

```ts
// from libs/isograph-runtime/src/optimistic.ts
export type OptimisticLayer = {
  /** Every source this layer overrides. */
  readonly fields: Set<FieldSource>;
};

export type OptimisticOverride = {
  readonly layer: OptimisticLayer;
  readonly value: PresentFieldValue;
};

/** Appends a layer to `store.optimisticLayers`. */
export function createOptimisticLayer(store: Store): OptimisticLayer;

/** Removes the layer's overrides and the layer. Returns the layers newer than it, for a replacing write. */
export function revertOptimisticLayer(
  store: Store,
  transaction: TransactionState,
  layer: OptimisticLayer,
): ReadonlySet<OptimisticLayer>;
```

`revertOptimisticLayer` removes the layer's override from each of its sources and runs the same changed-check as `writeField`. A server response writes `Base` with `keepOverridesOf` empty, which matches upstream's `addNetworkResponseStoreLayer` on top of the stack (`optimisticProxy.ts:161-183`). The response that replaces an optimistic layer reverts the layer and writes `Base` keeping the overrides of the layers newer than it, which matches upstream's replacement in place (`optimisticProxy.ts:393-492`). Effective values are the same as upstream's for data-only layers.

## 8. Subscriptions on resolver nodes and `MissingData` results replace fragment subscriptions and Suspense throws

Upstream subscriptions are per fragment reference, plus `AnyRecords` and `AnyChangesToRecord` (`IsographEnvironment.ts:42-69`), and a read with missing data throws a promise built on `onNextChangeToRecord` (`read.ts:92-122`).

```ts
// external isograph/libs/isograph-react/src/core/read.ts
    throw onNextChangeToRecord(environment, response.recordLink);
```

`onNextChangeToRecord` (`cache.ts:135-149`) is built on `AnyChangesToRecord`, whose check is always true: `has` returns a boolean, and a boolean is never `null`, so the callback fires whenever any record of that type changed (`subscribe.ts:93-97`).

```ts
// external isograph/libs/isograph-react/src/core/subscribe.ts
        case 'AnyChangesToRecord': {
          if (
            recordsEncounteredWhenNormalizing
              .get(subscription.recordLink.__typename)
              ?.has(subscription.recordLink.__link) != null
          ) {
```

Here a subscription is attached to a `ResolverNode`, and missing data is a cached `MissingData` result that a later write updates through the normal dirty-mark path. `AnyRecords` and `AnyChangesToRecord` do not exist.

```ts
// from libs/isograph-runtime/src/subscribe.ts
export type Subscription = {
  readonly sequence: number;
  readonly node: ResolverNode;
  delivered: ReadResult<unknown>;
  readonly callback: (result: ReadResult<unknown>) => void;
};

export type SubscriptionHandle<T> = {
  readonly initial: ReadResult<T>;
  readonly dispose: () => void;
};

/** The current result of `artifact` on `link`. */
export function readField<T>(
  store: Store,
  artifact: EagerReaderArtifact<T>,
  link: StoreLink,
  variables: Variables,
): ReadResult<T>;

/** Reads, then calls `callback` after every committed transaction that changes the result. */
export function subscribeField<T>(
  store: Store,
  artifact: EagerReaderArtifact<T>,
  link: StoreLink,
  variables: Variables,
  callback: (result: ReadResult<T>) => void,
): SubscriptionHandle<T>;

/** `readField` of the entrypoint's reader artifact on `{ __typename: concreteType, __link: ROOT_ID }`. */
export function readEntrypoint<T, TVariables extends Variables>(
  store: Store,
  entrypoint: Entrypoint<T, TVariables, NetworkResponseObject>,
  variables: TVariables,
): ReadResult<T>;

export function subscribeEntrypoint<T, TVariables extends Variables>(
  store: Store,
  entrypoint: Entrypoint<T, TVariables, NetworkResponseObject>,
  variables: TVariables,
  callback: (result: ReadResult<T>) => void,
): SubscriptionHandle<T>;
```

`readField` throws in `Transaction` and `RunningUserCode`, gets or creates the `ResolverNode` for `(artifact, Root.Record(link), variables)`, and returns `verify` (creating a node runs `recompute` with `previous` null). `subscribeField` does the same, mints `sequence` from `store.nextSubscriptionSequence`, adds the subscription to the node with `delivered` set to the result, and returns that result as `initial`. `dispose` removes the subscription from the node.

Reads inside a node body:

```ts
// from libs/isograph-runtime/src/read.ts
function computeSelectionSet(
  store: Store,
  recorder: DependencyRecorder,
  node: SelectionSetNode,
): ReadResult<SelectionSetData> {
  const data: { [alias: string]: unknown } = {};
  for (const selection of node.selectionSet) {
    const value = readSelection(store, recorder, node, selection);
    if (value.kind !== 'Ok') {
      return value;
    }
    data[selectionAlias(selection)] = value.value;
  }
  return { kind: 'Ok', value: data };
}

function computeResolver(
  store: Store,
  recorder: DependencyRecorder,
  node: ResolverNode,
): ReadResult<unknown> {
  const data = readChild(
    store,
    recorder,
    selectionSetNode(
      store,
      node.artifact.readerAst,
      node.root,
      node.variables,
    ),
  );
  if (data.kind !== 'Ok') {
    return data;
  }
  return runUserCode(store, () => {
    try {
      return {
        kind: 'Ok',
        value: node.artifact.resolver({
          data: data.value,
          parameters: node.variables,
        }),
      };
    } catch (error) {
      return {
        kind: 'ResolverThrew',
        fieldName: node.artifact.fieldName,
        root: node.root,
        error,
      };
    }
  });
}

/**
 * Scalar: the field's `Scalar` value. Link: `node.root.link`.
 * Linked, normalized: `readChild` of `selectionSetNode(selection.selections, Root.Record(link))` per link, mapped over list depth.
 * Linked, inline: `readChild` of `selectionSetNode(selection.selections, Root.Inline(owner, path))` per object, mapped over list depth, with `Index` segments.
 * Resolver: `readChild` of `resolverNode(selection.readerArtifact(), node.root, generateChildVariableMap(node.variables, selection.arguments))`.
 * The first non-`Ok` child result is returned unchanged. An `Absent` field is `MissingData` at (root, storageKey).
 * A stored kind that disagrees with the selection (a `Scalar` selection reading a `Linked` value) throws `StoreShapeError`.
 */
function readSelection(
  store: Store,
  recorder: DependencyRecorder,
  node: SelectionSetNode,
  selection: ReaderSelection,
): ReadResult<unknown>;

/** Record root: the source at (root.link, storageKey); a missing one is created with `base` `Absent` and `timeUpdated` `store.epoch`. Inline root: the owner source, then the path, then `storageKey` in the object reached; a path that does not reach an object is `Absent`. Records the source. */
function readFieldValue(
  store: Store,
  recorder: DependencyRecorder,
  root: Root,
  storageKey: StorageKey,
): FieldValue;

/** `verify`, then records `child` with its `timeUpdated`. */
function readChild<T>(
  store: Store,
  recorder: DependencyRecorder,
  child: NodeOf<T>,
): ReadResult<T>;

/** Gets or creates; a created node is computed before it is returned. */
function selectionSetNode(
  store: Store,
  selectionSet: ReaderSelectionSet,
  root: Root,
  variables: Variables,
): SelectionSetNode;

function resolverNode(
  store: Store,
  artifact: EagerReaderArtifact<unknown>,
  root: Root,
  variables: Variables,
): ResolverNode;

/** `alias`, else `fieldName`. */
function selectionAlias(selection: ReaderSelection): string;

/** Sets `RunningUserCode` around `body` and restores the previous phase. */
function runUserCode<T>(store: Store, body: () => T): T;
```

## 9. `fetchEntrypoint` returns a typed outcome

Upstream `makeNetworkRequest` returns `ItemCleanupPair<PromiseWrapper<void, AnyError>>` (`makeNetworkRequest.ts:159-179`) and rejects when the response has `errors` (`makeNetworkRequest.ts:224-231`).

```ts
// external isograph/libs/isograph-react/src/core/makeNetworkRequest.ts
      if (networkResponse.errors != null) {
        try {
          fetchOptions?.onError?.();
        } catch {}
        throw new Error('Network response had errors', {
          cause: networkResponse,
        });
      }
```

Here the promise resolves a `FetchOutcome`, and only a rejection of `networkFunction` rejects it.

```ts
// from libs/isograph-runtime/src/network.ts
export type NetworkFunction = (
  operation: Operation,
  variables: Variables,
) => Promise<NetworkResponse>;

export type NetworkResponse = {
  readonly data?: NetworkResponseObject | null;
  readonly errors?: readonly unknown[];
};

export type FetchOptions<TRawResponse extends NetworkResponseObject> = {
  readonly shouldFetch: ShouldFetch;
  readonly optimisticResponse: TRawResponse | null;
};

/** Upstream `ShouldFetch` (`check.ts:13-14`). */
export type ShouldFetch = 'Yes' | 'No' | 'IfNecessary';

export type FetchHandle = {
  readonly response: Promise<FetchOutcome>;
  /** Reverts an in-flight optimistic layer and releases the retained query. */
  readonly dispose: () => void;
};

export type FetchOutcome =
  | { readonly kind: 'Written' }
  | { readonly kind: 'NotFetched' }
  | { readonly kind: 'DisposedBeforeResponse' }
  | { readonly kind: 'ResponseHadErrors'; readonly errors: readonly unknown[] };

type RequestStatus =
  | { readonly kind: 'InFlight'; readonly layer: OptimisticLayer | null }
  | { readonly kind: 'Complete' }
  | { readonly kind: 'Disposed' };

export function fetchEntrypoint<
  T,
  TVariables extends Variables,
  TRawResponse extends NetworkResponseObject,
>(
  store: Store,
  entrypoint: Entrypoint<T, TVariables, TRawResponse>,
  variables: TVariables,
  options: FetchOptions<TRawResponse>,
): FetchHandle;
```

- `No`, or `IfNecessary` when `check` finds every field present: `response` resolves `NotFetched`.
- A response after `dispose` is not written, as upstream (`makeNetworkRequest.ts:235`), and `response` resolves `DisposedBeforeResponse`.
- A response with `errors` reverts the layer in a transaction and resolves `ResponseHadErrors`.
- Otherwise one transaction reverts the layer (if any) and `normalize`s `data` at `{ kind: 'Base', keepOverridesOf }` (empty, or the set `revertOptimisticLayer` returned), then `response` resolves `Written`.
- The optimistic response is written in one transaction with `createOptimisticLayer` and `normalize` at `{ kind: 'Layer', layer }`.

## 10. Garbage collection of derived nodes, and records live nodes still read

Upstream deletes every record a retained query does not reach (`garbageCollection.ts:98-118`), so a reader of such a record suspends.

```ts
// external isograph/libs/isograph-react/src/core/garbageCollection.ts
    for (const dataId in dataById) {
      if (!retainedTypeIds.has(dataId)) {
        delete dataById[dataId];
      }
    }
```

```ts
// from libs/isograph-runtime/src/garbageCollection.ts
/** Upstream `RetainedQuery` (`garbageCollection.ts:20-24`) with a loaded normalization AST. */
export type RetainedQuery = {
  readonly normalizationAst: NormalizationSelectionSet;
  readonly variables: Variables;
  readonly root: StoreLink;
};

/** Deletes derived nodes no subscription reaches, and sources no retained query or live node reaches. */
export function garbageCollect(store: Store): void;
```

`garbageCollect` returns without work outside `Idle` or while any optimistic layer exists (upstream returns while the store has layers, `garbageCollection.ts:66-68`). Otherwise:

1. Live derived nodes: every node with a subscription, and every derived node reachable from one through `dependencies`.
2. Every other node is removed from `store.derivedNodes` and from the `dependents` of each of its dependencies.
3. Live records: upstream's mark (`recordReachableIds`, `garbageCollection.ts:125-226`) over effective values, including links inside inline values.
4. A live record keeps every source except an `Absent` source with no dependents. Any other record keeps only sources that have dependents. A record with no sources is deleted; a type with no records is deleted.

A node created by `readField` with no subscription survives until the next `garbageCollect` and is recomputed after it.

## 11. Artifacts

The runtime consumes the artifact types below.

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderSelectionSet = readonly ReaderSelection[];

export type ReaderSelection =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField
  | ReaderLinkField;

export type ReaderScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly arguments: Arguments | null;
};

export type ReaderLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly alias: string | null;
  readonly arguments: Arguments | null;
  readonly storage: LinkedFieldStorage;
  readonly selections: ReaderSelectionSet;
};

export type LinkedFieldStorage = 'Normalized' | 'Inline';

export type ReaderResolverField = {
  readonly kind: 'Resolver';
  readonly alias: string;
  readonly arguments: Arguments | null;
  readonly readerArtifact: () => EagerReaderArtifact<unknown>;
};

export type ReaderLinkField = {
  readonly kind: 'Link';
  readonly alias: string;
};

export type EagerReaderArtifact<T> = {
  readonly kind: 'EagerReaderArtifact';
  /** `Pet.fullName`. */
  readonly fieldName: string;
  readonly readerAst: ReaderSelectionSet;
  readonly resolver: (parameters: ResolverParameters) => T;
  readonly isEqual: ((previous: T, next: T) => boolean) | null;
};

export type ResolverParameters = {
  readonly data: SelectionSetData;
  readonly parameters: Variables;
};

export type NormalizationSelectionSet = readonly NormalizationSelection[];

export type NormalizationSelection =
  | NormalizationScalarField
  | NormalizationLinkedField
  | NormalizationInlineFragment;

export type NormalizationScalarField = {
  readonly kind: 'Scalar';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  readonly kind: 'Linked';
  readonly fieldName: string;
  readonly arguments: Arguments | null;
  readonly storage: LinkedFieldStorage;
  readonly concreteType: TypeName | null;
  readonly selections: NormalizationSelectionSet;
};

export type NormalizationInlineFragment = {
  readonly kind: 'InlineFragment';
  readonly type: TypeName;
  readonly selections: NormalizationSelectionSet;
};

export type Operation = { readonly kind: 'Operation'; readonly text: string };

export type Entrypoint<
  T,
  TVariables extends Variables,
  TRawResponse extends NetworkResponseObject,
> = {
  readonly kind: 'Entrypoint';
  readonly concreteType: TypeName;
  readonly operation: Operation;
  readonly normalizationAst: NormalizationSelectionSet;
  readonly readerArtifact: () => EagerReaderArtifact<T>;
  /** Type-only. Not present at runtime. */
  readonly '~variables'?: TVariables;
  readonly '~rawResponse'?: TRawResponse;
};
```

Deltas from upstream (`core/reader.ts`, `core/entrypoint.ts`):

- `ReaderScalarField` (`reader.ts:89-96`): no `isFallible`, no `isUpdatable`.
- `ReaderLinkedField` (`reader.ts:103-121`): gains `storage`; no `isFallible`, `condition`, `isUpdatable`, or `refetchQueryIndex`.
- `ReaderResolverField` is upstream `ReaderNonLoadableResolverField` (`reader.ts:127-134`) without `usedRefetchQueries`; `readerArtifact` returns `EagerReaderArtifact` only.
- `ReaderImperativelyLoadedField` and `LoadablySelectedField` do not exist.
- `EagerReaderArtifact` (`reader.ts:26-37`): gains `isEqual`; no `hasUpdatable`; `ResolverParameters` has no `startUpdate`.
- `NormalizationLinkedField` (`entrypoint.ts:146-153`) gains `storage`; no `isFallible` on any normalization node.
- `Entrypoint` replaces upstream `IsographEntrypoint` (`entrypoint.ts:77-93`): `operation` and `normalizationAst` move out of `networkRequestInfo`; no persisted operations, no `NormalizationAstLoader`, no `ReaderWithRefetchQueriesLoader`, no refetch queries.

### What the compiler emits

`refactors/pending/artifact-generation.md` generates `iso.ts` and an entrypoint file whose default export is the string `"Query.HomeRoute"`; it lists readers, query text, and normalization ASTs as later docs. This runtime needs those later docs to emit:

- `__isograph/<Parent>/<name>/resolver_reader.ts` for each `field` with a selection set: `export default` an `EagerReaderArtifact`, importing the resolver from its source file by its export name.
- `__isograph/<Parent>/<name>/entrypoint.ts`: `export default` an `Entrypoint` with the query text, the normalization AST, and `readerArtifact` importing the resolver reader. The `iso.ts` overload `typeof entrypoint_Query__HomeRoute` stays correct when the default export changes from a string to this object.
- `storage` on every linked reader and normalization node, from the config's `inline_types`.
- `isEqual` on each `EagerReaderArtifact`: `null`, or the comparator the field declares (open decision 3).

The compiler rejects `__link` selected on a type in `inline_types`.

## 12. Removed from upstream

React (components, hooks, Suspense), `startUpdate` and updatable data, client pointers, loadable and imperatively loaded fields, refetch queries, `missingFieldHandler`, persisted operations, lazy normalization ASTs and entrypoint loaders, `isFallible`, `writeData`, `AnyRecords` and `AnyChangesToRecord` subscriptions, and the logging event catalog (`log` receives callback and comparator errors only).

## Open decisions

1. Propagation. Options: pull only (verify every subscription after each commit), push only (recompute dirty nodes at commit), push-pull. Pull only makes every write cost the size of every subscribed dependency graph, the same order of work as upstream's overlap scan. Push only recomputes results nobody reads and needs a topological order to backdate. Push-pull costs a `Set` of reverse edges per source and per node and keeps pico's verification. I hold push-pull.

2. How a type is marked for inline storage. Options: (a) a config list `inline_types` in `isograph.config.json`; (b) a directive on the type in an SDL extension file in `schema`; (c) every type without an `id` field is inline (Apollo's default for objects without key fields), with an opt-out list; (d) every type without `id` is normalized at a synthetic id (upstream's default), with an opt-in list. (c) makes an id-less singleton such as `viewer` one source, so any change under it re-runs every reader of it. (b) requires the user to own an extension file and a directive name the server never sees. (a) and (d) together keep upstream's behavior by default and make inline storage an explicit choice per type. I hold (a) with (d) as the default.

3. How a field declares its comparator. Options: (a) a directive naming an export of the same module, `field Pet.fullName @equality(comparator: "fullNameEquality")`, and the reader artifact imports it; (b) a directive with built-in comparators only (`Identity`, `Shallow`, `Json`); (c) a second argument to the function `iso` returns, `iso(...)(fullName, { isEqual })`, which changes the exported value's type from exactly the resolver's (artifact-generation.md's `Identity`) and requires the transform to keep the options. (a) allows any function, and TypeScript checks the import and the comparator's type against the resolver's return type in the generated artifact. (b) covers fewer cases and adds runtime exports. I hold (a). The directive name is part of this decision.

4. Backdating selection set nodes. Options: compare with `selectionSetDataEqual`, or treat every re-run as changed. The comparison is one level of `===` per selected alias and stops a resolver from re-running when its selection set re-ran without a different output. I hold compare.

5. When callbacks run. Options: synchronously at the end of the transaction that changed the data, or on a microtask after it. Synchronous delivery is deterministic and one network response is already one transaction; a microtask would merge separate responses that land in one task, at the price of a window in which `readField` and the last delivered result disagree. I hold synchronous.

6. How dependencies are recorded. Options: an explicit `DependencyRecorder` parameter, or an ambient stack on the store like pico's `DependencyStack`. Resolvers do not read the store, so every read happens in the runtime's own functions, which can pass the recorder. An ambient stack would be needed only if user code read the store. I hold the explicit recorder.

7. Where a `ResolverThrew` stops. Options: propagate to every ancestor (the parent selection set's result is the child's `ResolverThrew`), or store the error as the field's value in the parent's data. Propagation matches upstream, where the exception unwinds the whole read. Storing the value lets the rest of a page read, and makes every resolver's `data` type include an error case. I hold propagate.

8. Cycles through resolvers over cyclic data (`field User.bestFriendName { bestFriend { bestFriendName } }` where two users are each other's best friend). Options: throw `CycleError` from the read, as pico panics (`crates/pico/src/dependency.rs:95-101`); or a `ReadResult` variant. A cached `Cycle` result would depend on a node that has no result yet. Upstream overflows the stack. I hold throw.

9. When garbage collection runs. Options: upstream's trigger (a released retained query overflows the buffer, `makeNetworkRequest.ts:565-577`) plus the exported `garbageCollect`; or also on every subscription dispose. Collecting on dispose bounds derived nodes sooner and makes an unmount followed by a remount of the same view recompute everything. I hold upstream's trigger plus `garbageCollect`.

10. A response that lands after `dispose`. Options: drop it (upstream, `makeNetworkRequest.ts:235`), or write it. Writing keeps data the server sent, and GC may delete it at once since nothing retains it. I hold drop.

11. Package location and name. Options: `libs/isograph-runtime` published as `@isograph/runtime`, or a directory under `crates`. i2 keeps JavaScript packages in `libs` (`libs/isograph-babel-plugin`), and `pnpm-workspace.yaml` lists `libs/*`. No upstream package is named `@isograph/runtime`. I hold `libs/isograph-runtime` and `@isograph/runtime`.

12. Reads of a field on a record, beyond entrypoints. Options: export `readField` and `subscribeField` taking an `EagerReaderArtifact` and a `StoreLink`, or export only the entrypoint forms. A consumer that renders one pet subscribes to that pet's field without subscribing to the whole page. I hold exporting both.

## Changes

Each change is one commit, in order.

1. `libs/isograph-runtime`: `package.json` (`@isograph/runtime`, no dependencies), `tsconfig.json`, `src/index.ts`.
2. `artifacts.ts`, `store.ts`, `equality.ts`: every type in changes 1, 2, and 11, `createStore`, `effectiveValue`, `rootKey`, `getStorageKey`, `fieldValueEqual`, `selectionSetDataEqual`.
3. `derivedNode.ts`, `read.ts`, `subscribe.ts`: derived nodes, verification, backdating, `readField`, `subscribeField`, `readEntrypoint`, `subscribeEntrypoint`.
4. `transaction.ts`: `StorePhase`, `transaction`, `writeField`, `markDirty`, delivery.
5. `normalize.ts`, `network.ts`: `normalize`, `inlineValue`, `check`, `fetchEntrypoint` without optimistic responses.
6. `optimistic.ts`: layers, `revertOptimisticLayer`, `optimisticResponse` in `fetchEntrypoint`.
7. `garbageCollection.ts`: retained queries, the release buffer, `garbageCollect`.
8. Compiler, after the reader and normalization artifact docs land: `inline_types` in the config, `storage` on emitted AST nodes, the `__link` check, and the comparator directive.
