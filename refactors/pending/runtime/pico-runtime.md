# Runtime: resolver results in the store

Status: under discussion. Not approved for implementation. It changes the landed code in `libs/isograph-runtime/src` and assumes `@isograph/pico-js` (`refactors/pending/runtime/pico-js.md`) has shipped.

Upstream paths are under `../isograph/libs/isograph-react/src/core` unless they name another directory.

A server value is a source node in the store. A resolver result is a derived node in the record container of the record it was computed on. Every write is inside one `startBatch` and runs no resolver. A read calls `pull`. Upstream stores no resolver value and calls every eager resolver it reaches on each read (`read.ts:600`); the user chose to store results so that a resolver reruns only when an input changed.

## The store

```ts
// from libs/isograph-runtime/src/store.ts
import { createSourceNode, startBatch, type DerivedNode, type SourceNode } from '@isograph/pico-js';

/** The parameter of a `startBatch` body. The engine does not export its class. */
export type Mutators = Parameters<Parameters<typeof startBatch>[0]>[0];

export type Store = Map<EntityName, Map<RecordId, RecordContainer>>;

export type RecordContainer = {
  presence: RecordPresence;
  readonly sources: Map<StoreKey, StoreSourceNode>;
  readonly derived: Map<StoreKey, ResolverNode>;
};

/** `Absent` means the store holds no record at the container's link. */
export type RecordPresence = 'Present' | 'Absent';

export type StoreSourceNode = SourceNode<StoreValue | Missing>;
export type ResolverNode = DerivedNode<ResolverResult<unknown>>;

/** A source node holds `MISSING` when its record has no value under its key. */
const MISSING: unique symbol = Symbol('Missing');
export type Missing = typeof MISSING;

export function isMissing(value: StoreValue | Missing): value is Missing {
  return value === MISSING;
}
```

Before (`libs/isograph-runtime/src/store.ts:4-8`):

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = Map<EntityName, Map<RecordId, StoreRecord>>;
export type StoreRecord = Map<StoreKey, StoreValue>;
```

The package index exports none of `MISSING`, `Missing`, and `isMissing`. `createIsographEnvironment` keeps its parameters (`libs/isograph-runtime/src/environment.ts:34-45`).

`libs/isograph-runtime/package.json` gains the engine:

```json
// from libs/isograph-runtime/package.json
  "dependencies": {
    "@isograph/disposable-types": "workspace:*",
    "@isograph/pico-js": "workspace:*"
  },
```

The runtime's three pre-commit hooks match `libs/pico-js` as well. `switch-exhaustiveness-check` checks every `switch` for exhaustiveness, and no `switch` has a `default` arm, as upstream does since #866.

### Accessors

No function outside `store.ts`, `normalize.ts`, and `storedResults.ts` touches `sources`, `derived`, or `presence`.

```ts
// from libs/isograph-runtime/src/store.ts
function recordContainer(store: Store, link: StoreLink): RecordContainer | undefined {
  return store.get(link.entityName)?.get(link.id);
}

/** `containerAt` inserts an `Absent` container when the store has none at `link`. */
export function containerAt(store: Store, link: StoreLink): RecordContainer {
  return getOrInsert(getOrInsert(store, link.entityName, () => new Map()), link.id, () => ({
    presence: 'Absent',
    sources: new Map(),
    derived: new Map(),
  }));
}

export function presentContainerAt(store: Store, link: StoreLink): RecordContainer {
  const container = containerAt(store, link);
  container.presence = 'Present';
  return container;
}

/** `sourceNodeAt` inserts a `MISSING` source node, and an `Absent` container, when `link` has none for `key`. */
export function sourceNodeAt(store: Store, link: StoreLink, key: StoreKey): StoreSourceNode {
  return getOrInsert(containerAt(store, link).sources, key, () => createSourceNode(MISSING));
}

export function isUnchanged(key: StoreKey): (stored: StoreValue | Missing, written: StoreValue) => boolean {
  const unchanged = key === VALUE_KEY ? leafUnchanged : compositeUnchanged;
  return (stored, written) => stored !== MISSING && unchanged(stored, written);
}

function leafUnchanged(stored: StoreValue, written: StoreValue): boolean {
  if (!isArray(written)) {
    return stored === written;
  }
  return (
    isArray(stored) &&
    stored.length === written.length &&
    written.every((item, index) => leafUnchanged(stored[index], item))
  );
}

function compositeUnchanged(stored: StoreValue, written: StoreValue): boolean {
  if (stored === null || written === null) {
    return stored === written;
  }
  if (isArray(stored) || isArray(written)) {
    return (
      isArray(stored) &&
      isArray(written) &&
      stored.length === written.length &&
      written.every((item, index) => compositeUnchanged(stored[index], item))
    );
  }
  return (
    typeof stored === 'object' &&
    typeof written === 'object' &&
    stored.entityName === written.entityName &&
    stored.id === written.id
  );
}

/** `storeValue` inserts nothing and records no dependency. */
export function storeValue(store: Store, link: StoreLink, key: StoreKey): StoreValue | Missing {
  const source = recordContainer(store, link)?.sources.get(key);
  return source === undefined ? MISSING : source.value;
}

export function recordPresence(store: Store, link: StoreLink): RecordPresence {
  return recordContainer(store, link)?.presence ?? 'Absent';
}

export function storeSnapshot(store: Store): PlainStore {
  return new Map(
    [...store].flatMap(
      ([entityName, containers]): ReadonlyArray<[EntityName, Map<RecordId, Map<StoreKey, StoreValue>>]> => {
        const records = new Map(
          [...containers]
            .filter(([, container]) => container.presence === 'Present')
            .map(([id, container]): [RecordId, Map<StoreKey, StoreValue>] => [id, presentValues(container)]),
        );
        return records.size === 0 ? [] : [[entityName, records]];
      },
    ),
  );
}

function presentValues(container: RecordContainer): Map<StoreKey, StoreValue> {
  return new Map(
    [...container.sources].flatMap(([key, source]): ReadonlyArray<[StoreKey, StoreValue]> => {
      const value = source.value;
      return value === MISSING ? [] : [[key, value]];
    }),
  );
}

export type PlainStore = Map<EntityName, Map<RecordId, Map<StoreKey, StoreValue>>>;

export function deleteRecord(mutators: Mutators, store: Store, link: StoreLink): void {
  const containers = store.get(link.entityName);
  const container = containers?.get(link.id);
  if (containers === undefined || container === undefined) {
    return;
  }
  container.sources.forEach((source) => {
    if (source.value !== MISSING) {
      mutators.writeSource(source, MISSING);
    }
  });
  container.derived.forEach((node) => mutators.disposeDerivedNode(node));
  containers.delete(link.id);
  if (containers.size === 0) {
    store.delete(link.entityName);
  }
}
```

- The engine's contract is a write only of a changed value, so `deleteRecord` writes `MISSING` to each present source and nothing to a `MISSING` one, and `writeValue` checks `isUnchanged` first.
- The store creates the resolver nodes, so `deleteRecord` disposes them. A caller that creates a derived node disposes it.
- `deleteRecord`'s `container.derived` line arrives in core change 7.
- Unresolved: `deleteRecord` removes source nodes that still have dependents (`refactors/pending/future-improvements.md`, "`deleteRecord` drops source nodes that still have dependents").

The sweep, after (`libs/isograph-runtime/src/garbageCollection.ts:32-41`):

```ts
// from libs/isograph-runtime/src/garbageCollection.ts
  startBatch((mutators) => {
    store.forEach((containers, entityName) => {
      const kept = reachable.get(entityName);
      containers.forEach((_container, id) => {
        if (kept?.has(id) !== true) {
          deleteRecord(mutators, store, { entityName, id });
        }
      });
    });
  });
```

Call sites:

- `check`: `storeRecord(store, link) === undefined` becomes `recordPresence(store, link) === 'Absent'` (`libs/isograph-runtime/src/check.ts:30`, `:54`), and `record.get(key)` becomes `storeValue(store, link, key)` with `isMissing(..)` for `=== undefined` (`:49-50`).
- The garbage collector's mark makes the same two replacements (`garbageCollection.ts:51`, `:70-73`). A link that reaches an `Absent` container keeps that container, and the mark does not recurse into it. The root of a retained query keeps its container whatever its presence.
- `readDataForOnComplete` (`libs/isograph-runtime/src/network.ts:196-218`) still calls `read`.
- Test fixtures: `storeOf` returns a `PlainStore`, and a test compares `storeSnapshot(store)` with it. `test/fixtures.ts` gains `storeFrom(plain: PlainStore): Store`, which calls `presentContainerAt` and `mutators.writeSource(sourceNodeAt(..), value)` for each value in one `startBatch`. A test that set one stored value calls `writeSource(sourceNodeAt(store, pet('1'), 'bestFriend'), 'x')`, a test that deleted a record calls `startBatch((mutators) => deleteRecord(mutators, store, link))`, and a test that deleted a key builds its store with `storeFrom` without that key.

### Absent records

An absent record reads as a record whose every key is missing. The walk reads the source node of the first key it tries, `sourceNodeAt` creates it with `MISSING` in an `Absent` container, and the read is `MissingData` naming that key. A derived field selected on an absent record creates its node in the `Absent` container. `presentContainerAt` sets `presence` and calls no engine function.

v1's two "No record for X" checks go (`libs/isograph-runtime/src/read.ts:68-74`, `:184-191`), so a reader AST that reads no key on an absent record is `Ok`. Upstream checks presence at every record it reads (`read.ts:161-167`); the deviation follows from presence not being a source node, so a `MissingData` from presence could never be marked when the record appears.

## Equality

A resolver node's computation returns `previous` when `resolverResultsEqual` accepts the new result.

```ts
// from libs/isograph-runtime/src/read.ts
export type ResolverResult<T> = ReadResolverOk<T> | ReadMissingData | ReadResolverThrew;

/** `WalkData` means `value` is the data object the walk assembled for this call. */
export type ReadResolverOk<T> =
  | { readonly kind: 'Ok'; readonly origin: 'WalkData'; readonly value: T & SelectionSetData }
  | { readonly kind: 'Ok'; readonly origin: 'ResolverValue'; readonly value: T };

export type OkOrigin = ReadResolverOk<unknown>['origin'];

export type ReadResolverThrew = {
  readonly kind: 'ResolverThrew';
  readonly selectableName: string;
  readonly recordLink: StoreLink;
  readonly error: unknown;
};
```

```ts
// from libs/isograph-runtime/src/storedResults.ts
export function resolverResultsEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  outputEqual: (previous: unknown, next: unknown) => boolean,
): (previous: ResolverResult<unknown>, next: ResolverResult<unknown>) => boolean {
  return (previous, next) => {
    switch (previous.kind) {
      case 'Ok':
        return next.kind === 'Ok' && okResultsEqual(artifact, outputEqual, previous, next);
      case 'MissingData':
        return next.kind === 'MissingData' && missingDataAreEqual(previous, next);
      case 'ResolverThrew':
        return next.kind === 'ResolverThrew' && previous.error === next.error;
    }
  };
}

function okResultsEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  outputEqual: (previous: unknown, next: unknown) => boolean,
  previous: ReadResolverOk<unknown>,
  next: ReadResolverOk<unknown>,
): boolean {
  switch (previous.origin) {
    case 'WalkData':
      return (
        next.origin === 'WalkData' &&
        selectionSetDataAreEqual(artifact.tables, artifact.parentEntityName, artifact.readerAst, previous.value, next.value)
      );
    case 'ResolverValue':
      return next.origin === 'ResolverValue' && outputEqual(previous.value, next.value);
  }
}

function missingDataAreEqual(previous: ReadMissingData, next: ReadMissingData): boolean {
  return (
    previous.reason === next.reason &&
    previous.recordLink.entityName === next.recordLink.entityName &&
    previous.recordLink.id === next.recordLink.id &&
    (previous.nestedReason === undefined || next.nestedReason === undefined
      ? previous.nestedReason === next.nestedReason
      : missingDataAreEqual(previous.nestedReason, next.nestedReason))
  );
}
```

```ts
// from libs/isograph-runtime/src/areEqualWithDeepComparison.ts
export function selectionSetDataAreEqual(
  tables: EntityTables,
  entity: EntityName,
  selections: ReaderSelectionSet,
  delivered: SelectionSetData,
  read: SelectionSetData,
): boolean {
  const table = tableOf(tables, entity);
  return selections.every((selection) =>
    selectionValuesAreEqual(tables, table, entity, selection, delivered[selection.alias], read[selection.alias]),
  );
}

function selectionValuesAreEqual(
  tables: EntityTables,
  table: EntityTable,
  entity: EntityName,
  selection: ReaderSelection,
  delivered: unknown,
  read: unknown,
): boolean {
  const field = fieldOf(table, entity, selection.fieldName);
  switch (field.kind) {
    case 'Source':
      return valuesAreEqual(delivered, read, (a, b) => targetValuesAreEqual(tables, field.target, selection.selections, a, b));
    case 'Derived': {
      const target = field.target;
      return target === null
        ? delivered === read
        : valuesAreEqual(delivered, read, (a, b) => targetValuesAreEqual(tables, target, selection.selections, a, b));
    }
  }
}

function targetValuesAreEqual(
  tables: EntityTables,
  entity: EntityName,
  selections: ReaderSelectionSet | null,
  delivered: unknown,
  read: unknown,
): boolean {
  return selections === null
    ? delivered === read
    : isSelectionSetData(delivered) &&
        isSelectionSetData(read) &&
        selectionSetDataAreEqual(tables, entity, selections, delivered, read);
}
```

Before (`areEqualWithDeepComparison.ts:9-44`): `selectionSetDataAreEqual(selections, delivered, read)` and a `switch` on `selection.kind`. `valuesAreEqual` and `isSelectionSetData` are unchanged. `read.ts` exports `fieldOf` and `tableOf` for this file. Resolvers and callers must not mutate a stored value or a stored result.

## The reader walk

```ts
// from libs/isograph-runtime/src/read.ts
/** `TFailure` is what a nested resolver's result can be besides `Ok` and `MissingData`. */
export type ReadHooks<TFailure> = {
  readonly readValue: (record: StoreLink, key: StoreKey) => StoreValue | Missing;
  readonly readResolver: (
    artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
    root: StoreLink,
    variables: Variables,
  ) => ReadResult<unknown> | TFailure;
};

/** `readData` does not call `artifact`'s own resolver. */
export function readData<TFailure>(
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure;
```

`readData` is v1's `readButDoNotEvaluate` body without the presence check. `readSelectionSet`, `readSelection`, `readLinkedValue`, and `allOk` take `hooks` where they took `store` and return `ReadResult<..> | TFailure`. `allOk` stops at the first result whose `kind` is not `Ok`.

```ts
// from libs/isograph-runtime/src/read.ts
// before:
function readStoreValue(store: Store, root: StoreLink, key: StoreKey): ReadResult<StoreValue> {
  const record = storeRecord(store, root);
  if (record === undefined) {
    return { kind: 'MissingData', reason: `No record for ${recordName(root)}`, recordLink: root };
  }
  const value = record.get(key);
  if (value === undefined) {
    return { kind: 'MissingData', reason: `No value for ${key} on ${recordName(root)}`, recordLink: root };
  }
  return ok(value);
}
// after:
function readStoreValue<TFailure>(hooks: ReadHooks<TFailure>, root: StoreLink, key: StoreKey): ReadResult<StoreValue> {
  const value = hooks.readValue(root, key);
  if (isMissing(value)) {
    return { kind: 'MissingData', reason: `No value for ${key} on ${recordName(root)}`, recordLink: root };
  }
  return ok(value);
}
```

```ts
// from libs/isograph-runtime/src/read.ts
export function readButDoNotEvaluate<TEntityName extends EntityName>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<SelectionSetData> {
  return readData(evaluatingHooks(store), artifact, root, variables);
}

/** `evaluatingHooks` records nothing, calls each nested resolver, and lets its throw propagate, as v1's reads do. */
function evaluatingHooks(store: Store): ReadHooks<never> {
  return {
    readValue: (record, key) => storeValue(store, record, key),
    readResolver: (artifact, root, variables) => read(store, artifact, root, variables),
  };
}
```

`read` keeps its body and its return type.

## Selections

A scalar field's value is its own record: `Pet:1.name` holds a link to the `String` record `Pet:1.name`, whose `__value` source node holds the string. The compiler side is `refactors/pending/language/linked-only-selections.md`.

### The reader AST

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderSelectionSet = ReadonlyArray<ReaderSelection>;

/** `selections` is `null` when the selection has no selection set. */
export type ReaderSelection = {
  readonly alias: string;
  readonly fieldName: string;
  readonly arguments: Arguments | null;
  readonly selections: ReadonlyArray<ReaderSelection> | null;
};
```

Before (`libs/isograph-runtime/src/artifacts.ts:144-193`): the union of `ReaderScalarSelection`, `ReaderLinkedSelection`, `ReaderRefinementSelection` (`condition`, `selections`), and `ReaderResolverSelection` (`readerArtifact`). `Scalar` becomes `selections: null`, `Refinement` becomes a selection of the generated refinement field (`asPet`), and `Resolver` becomes a selection whose entity table entry is `Derived`. `Condition` stays for the normalization AST.

`storeKey` becomes `storeKey(name: string, args: Arguments | null, variables: Variables): StoreKey` (`libs/isograph-runtime/src/storeKey.ts:13-21`). `normalize`, `check`, and the mark pass `selection.selectableName`, and the walk passes `selection.fieldName`.

The package index exports `ReaderSelection`, `ReaderSelectionSet`, `EntityTable`, `FieldEntry`, `EntityShape`, `RecordStorage`, `EMPTY_DATA`, `NormalizationTarget`, and `NormalizationLeafTarget`, and stops exporting `ReaderLinkedSelection`, `ReaderRefinementSelection`, `ReaderResolverSelection`, `ReaderScalarSelection`, and `NormalizationIdentity`.

### Entity tables

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type EntityTables = { readonly [entityName: string]: () => EntityTable };

export type EntityTable = {
  /** `fields` holds every protocol selectable and generated refinement field of the entity, and each client field on it whose reader artifact is generated. */
  readonly fields: { readonly [fieldName: string]: FieldEntry };
  readonly shape: EntityShape;
  readonly storage: RecordStorage;
};

export type FieldEntry =
  /** The source node of a server field holds a link to a `target` record, `null`, or a list of these. */
  | { readonly kind: 'Source'; readonly target: EntityName }
  /** `target` is `null` for a client field whose value is opaque. */
  | {
      readonly kind: 'Derived';
      readonly target: EntityName | null;
      readonly artifact: () => ReaderArtifact<ResolverParam, unknown, EntityName>;
    };

/** A bare selection of a record of a `Leaf` entity is its `__value`, and a bare selection of a record of a `Composite` entity is `EMPTY_DATA`. */
export type EntityShape = 'Leaf' | 'Composite';

export type RecordStorage =
  /** The response object's value under `selectableName` is the record id. */
  | { readonly kind: 'OwnId'; readonly selectableName: string }
  /** The record's id is `omittedRecordId` of its position. */
  | { readonly kind: 'FromParent' }
  /** Every record of the entity is the record `id`. */
  | { readonly kind: 'Fixed'; readonly id: string };
```

`RecordStorage` replaces the normalization AST's `NormalizationIdentity` (`Select`, `Omit`, `Fixed`, `artifacts.ts:54-62`). Scalars and objects with no identity have `FromParent` storage.

Generated modules, one per entity, at `<Entity>/entity_table.ts`, entries in byte order of the field names:

```ts
// generated by crates/typescript_artifacts/src/entity_table.rs
// libs/isograph-runtime/fixtures/store/src/__isograph/String/entity_table.ts
import type { EntityTable } from '@isograph/runtime';
import String__toUppercase__resolver_reader from './toUppercase/resolver_reader';

const entityTable = (): EntityTable => ({
  fields: {
    toUppercase: { kind: 'Derived', target: null, artifact: String__toUppercase__resolver_reader },
  },
  shape: 'Leaf',
  storage: { kind: 'FromParent' },
});
export default entityTable;
```

`Pet`'s table has the same form with `shape: 'Composite'` and `storage: { kind: 'OwnId', selectableName: 'id' }`. The `Node` table lists `NodeLink`, `__typename`, `asBlogItem`, `asCheckin`, `asPet`, and `id`, with `asPet: { kind: 'Derived', target: 'Pet', artifact: Node__asPet__resolver_reader }`. The `Query` table's storage is `{ kind: 'Fixed', id: '__ROOT' }`.

Imports:

- A reader artifact imports the table of its parent entity and of the target of every selection its own reader AST reads at. `Query/PetDetailPage/resolver_reader.ts` imports `Query`, `Pet`, and `String`.
- A normalization AST imports the table of every entity whose records `normalize` writes with it.
- A table imports the reader artifact of each of its derived fields.
- Every reference to an imported binding is inside a function body (`() => Pet__entity_table()` in the normalization AST, printed as `ImportCallThunk`, `crates/typescript_artifacts/src/module.rs:44-57`), because the modules import each other in a cycle.

### Refinements and `__self`

A refinement `asPet` on `Node` is a generated derived field on `Node` with target `Pet`, an empty reader AST, and the resolver `({ link }) => (link.entityName === 'Pet' ? { entityName: 'Pet', id: link.id } : null)`. Its node on `Pet:1` is keyed `Node.asPet/{}`.

`__self` is in no table. `fieldOf` returns `{ kind: 'Self' }` for it, and the walk reads its selections at the current record as the same entity. Core change 6 gives the code.

### The walk

```ts
// from libs/isograph-runtime/src/read.ts
/** Two bare reads of a composite entity compare `===` because both return this one frozen object. */
export const EMPTY_DATA: Readonly<Record<PropertyKey, never>> = Object.freeze({});

export function readData<TFailure>(
  hooks: ReadHooks<TFailure>,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure {
  return readSelectionSet(hooks, artifact.tables, artifact.readerAst, root, artifact.parentEntityName, variables);
}

function readSelectionSet<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  selections: ReaderSelectionSet,
  record: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<SelectionSetData> | TFailure;

function readSelection<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  table: EntityTable,
  selection: ReaderSelection,
  record: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<unknown> | TFailure {
  const field = fieldOf(table, entity, selection.fieldName);
  switch (field.kind) {
    case 'Source': {
      const key = storeKey(selection.fieldName, selection.arguments, variables);
      const value = readStoreValue(hooks, record, key);
      if (value.kind === 'MissingData') {
        return value;
      }
      return nested(
        selection.alias,
        record,
        eachLink(value.value, record, key, (link) => readAt(hooks, tables, selection.selections, link, field.target, variables)),
      );
    }
    case 'Derived': {
      const derived = hooks.readResolver(field.artifact(), record, childVariables(variables, selection.arguments));
      if (derived.kind !== 'Ok' || field.target === null) {
        return nested(selection.alias, record, derived);
      }
      const target = field.target;
      return nested(
        selection.alias,
        record,
        derived.value === null
          ? ok(null)
          : readAt(hooks, tables, selection.selections, derivedLink(derived.value, record, selection.fieldName), target, variables),
      );
    }
  }
}

function readAt<TFailure>(
  hooks: ReadHooks<TFailure>,
  tables: EntityTables,
  selections: ReaderSelectionSet | null,
  link: StoreLink,
  entity: EntityName,
  variables: Variables,
): ReadResult<unknown> | TFailure {
  if (selections !== null) {
    return readSelectionSet(hooks, tables, selections, link, entity, variables);
  }
  const shape = tableOf(tables, entity).shape;
  switch (shape) {
    case 'Leaf':
      return readStoreValue(hooks, link, VALUE_KEY);
    case 'Composite':
      return ok(EMPTY_DATA);
  }
}

/** `fieldOf` throws when the table has no entry, because the compiler lists every field of the entity. */
export function fieldOf(table: EntityTable, entity: EntityName, fieldName: string): FieldEntry;

/** `tableOf` throws when `tables` has no entry, because the compiler imports every entity the walk reads at. */
export function tableOf(tables: EntityTables, entity: EntityName): EntityTable;

/** `derivedLink` throws when the value is not a link, because the field's resolver is generated and returns a link or `null`. */
function derivedLink(value: unknown, owner: StoreLink, fieldName: string): StoreLink;

/** `nested` wraps a `MissingData` result with `missingDataFor(alias, root, ..)` and returns any other result unchanged. */
function nested<T, TFailure>(alias: string, root: StoreLink, result: ReadResult<T> | TFailure): ReadResult<T> | TFailure;

function eachLink<TFailure>(
  value: StoreValue,
  owner: StoreLink,
  key: StoreKey,
  f: (link: StoreLink) => ReadResult<unknown> | TFailure,
): ReadResult<unknown> | TFailure;
```

### Scalar records

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type NormalizationAst<TEntityName extends EntityName = EntityName> = {
  readonly discriminatorResponseKey: string;
  readonly root: NormalizationRootRecord<TEntityName>;
  readonly selections: NormalizationSelectionSet;
  readonly tables: EntityTables;
};

export type NormalizationSelection = {
  readonly kind: 'Selection';
  readonly responseKey: string | null;
  readonly selectableName: string;
  readonly arguments: Arguments | null;
  readonly target: NormalizationTarget;
};

export type NormalizationTarget = NormalizationLeafTarget | NormalizationCompositeTarget;

export type NormalizationLeafTarget = {
  readonly kind: 'Leaf';
  readonly entityName: EntityName;
  /** Arrays below `listDepth` are one value, because a custom scalar's value can be a JSON array. */
  readonly listDepth: number;
};

export type NormalizationCompositeTarget = {
  readonly kind: 'Composite';
  readonly entity: NormalizationEntity;
  readonly selections: NormalizationSelectionSet;
};

export type NormalizationConditional = {
  readonly kind: 'Conditional';
  readonly condition: Condition;
  readonly selections: NormalizationSelectionSet;
};
```

Before (`artifacts.ts:5-71`): `NormalizationAst` has no `tables`, a leaf's `target` is `null`, and `NormalizationCompositeTarget` and `NormalizationConditional` have `identity: NormalizationIdentity`. `recordIdentity`'s search of conditionals (`recordId.ts:73-84`) goes, because the concrete entity's table gives the storage.

```ts
// from libs/isograph-runtime/src/store.ts
/** GraphQL reserves names that start with `__`, so no server field is named `__value`, and `__value` differs from the `__self` field. */
export const VALUE_KEY: StoreKey = '__value';

// from libs/isograph-runtime/src/recordId.ts
/** The id joins the parent's link, the selectable name, one `.<index>` per list level, and the argument chunks. */
export function omittedRecordId(
  parent: StoreLink,
  selection: NormalizationSelection,
  listIndexes: ReadonlyArray<number>,
  variables: Variables,
): RecordId {
  return (
    `${parent.entityName}:${parent.id}.${selection.selectableName}` +
    listIndexes.map((index) => `.${index}`).join('') +
    argumentChunks(selection.arguments, variables)
  );
}

/** `recordId` throws on an `OwnId` value that is not an object holding a string under `storage.selectableName`. */
export function recordId(
  value: JsonValue,
  storage: RecordStorage,
  position: RecordPosition,
  variables: Variables,
): RecordId;

/** `storageOf` throws when the AST has no table for `entityName`, because the response then names an entity the compiled schema does not have. */
export function storageOf(normalization: Normalization, entityName: EntityName, path: ResponsePath): RecordStorage;

// from libs/isograph-runtime/src/normalize.ts
type Normalization = {
  readonly store: Store;
  readonly mutators: Mutators;
  readonly discriminatorResponseKey: string;
  readonly variables: Variables;
  readonly tables: EntityTables;
};
```

`omittedRecordId` is `recordId`'s `Omit` branch extracted verbatim (`recordId.ts:109-114`). `recordId`'s `FromParent` arm calls it, its `OwnId` arm is the `Select` branch (`:95-108`), and its `Fixed` arm is the `Fixed` branch (`:93-94`). `normalize` sets `mutators` and `tables: ast.tables` (`normalize.ts:37-41`).

```ts
// from libs/isograph-runtime/src/normalize.ts
// writeSelection, before (normalize.ts:96-109):
  record.set(
    storeKey(selection, normalization.variables),
    selection.target == null
      ? value
      : compositeValue(normalization, selection, selection.target, value, link, [], [...path, responseKey]),
  );
// after:
  const key = storeKey(selection.selectableName, selection.arguments, normalization.variables);
  const target = selection.target;
  const written = target.kind === 'Leaf'
    ? leafRecords(normalization, selection, target, value, link, [], target.listDepth, [...path, responseKey])
    : compositeValue(normalization, selection, target, value, link, [], [...path, responseKey]);
  writeValue(normalization, link, key, written);

function leafRecords(
  normalization: Normalization,
  selection: NormalizationSelection,
  target: NormalizationLeafTarget,
  value: JsonValue,
  parent: StoreLink,
  listIndexes: ReadonlyArray<number>,
  depth: number,
  path: ResponsePath,
): CompositeValue {
  if (value === null) {
    return null;
  }
  if (depth > 0) {
    if (!isArray(value)) {
      throw new Error(`Malformed response at ${JSON.stringify(path)}: expected a list or null, got ${JSON.stringify(value)}.`);
    }
    return value.map((item, index) =>
      leafRecords(normalization, selection, target, item, parent, [...listIndexes, index], depth - 1, [...path, index]),
    );
  }
  const position = { parent, selection, listIndexes, path };
  const link = {
    entityName: target.entityName,
    id: recordId(value, storageOf(normalization, target.entityName, path), position, normalization.variables),
  };
  presentContainerAt(normalization.store, link);
  writeValue(normalization, link, VALUE_KEY, value);
  return link;
}
```

`compositeValue` takes the id from `recordId(value, storageOf(normalization, entityName, path), position, variables)`. `check` and the mark follow a `Leaf` target one level: `check` yields the linked record when it is absent or its `__value` is missing (`check.ts:47-68`), and the mark keeps the linked record and does not recurse (`garbageCollection.ts:68-86`). `storeSnapshot` returns scalar records as stored.

## Stored resolver results

```ts
// from libs/isograph-runtime/src/storedResults.ts
import { createDerivedNode, type DerivedNode, pull, readDerived, readSource } from '@isograph/pico-js';

export function readStoredResult<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ResolverResult<TOutput> {
  return pull(resolverNodeAt(store, artifact, root, variables)) as ResolverResult<TOutput>;
}

export function resolverNodeAt(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): ResolverNode {
  return getOrInsert(
    containerAt(store, root).derived,
    resolverStoreKey(artifact, variables),
    () => createDerivedNode(resolverComputation(store, artifact, root, variables)),
  );
}

function resolverComputation(
  store: Store,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): (callingNode: DerivedNode<unknown>, previous: ResolverResult<unknown> | undefined) => ResolverResult<unknown> {
  const resultsEqual = resolverResultsEqual(artifact, identical);
  return (callingNode, previous) => {
    const data = readData(contextHooks(store, callingNode), artifact, root, variables);
    const result = data.kind === 'Ok' ? callResolver(artifact, root, data.value, variables) : data;
    return previous !== undefined && resultsEqual(previous, result) ? previous : result;
  };
}

function identical(previous: unknown, next: unknown): boolean {
  return previous === next;
}

function contextHooks(store: Store, callingNode: DerivedNode<unknown>): ReadHooks<ReadResolverThrew> {
  return {
    readValue: (record, key) => readSource(callingNode, sourceNodeAt(store, record, key)),
    readResolver: (artifact, root, variables) => readDerived(callingNode, resolverNodeAt(store, artifact, root, variables)),
  };
}

function callResolver(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  data: SelectionSetData,
  variables: Variables,
): ResolverResult<unknown> {
  try {
    const value = artifact.resolver({ data, parameters: variables, link: root });
    return value === data
      ? { kind: 'Ok', origin: 'WalkData', value: data }
      : { kind: 'Ok', origin: 'ResolverValue', value };
  } catch (error: unknown) {
    return { kind: 'ResolverThrew', selectableName: artifact.selectableName, recordLink: root, error };
  }
}
```

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** The key is `<parentEntityName>.<selectableName>/<variables>`, where `JSON.stringify(stableCopy(variables))` writes the variables. */
export function resolverStoreKey(
  field: Pick<ReaderArtifact<ResolverParam, unknown, EntityName>, 'parentEntityName' | 'selectableName'>,
  variables: Variables,
): StoreKey {
  return `${field.parentEntityName}.${field.selectableName}/${JSON.stringify(stableCopy(variables))}`;
}
```

The variables are the field's own: `childVariables(variables, selection.arguments)` for a `Derived` entry, and the read's variables at the top level. `PetName` on `Pet:1` is `Pet.PetName/{}`.

```ts
// from libs/isograph-runtime/src/artifacts.ts
// ReaderArtifact, before:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  readonly readerAst: ReaderSelectionSet;
  resolver(param: TParam): TOutput;
};
// after:
export type ReaderArtifact<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName> = {
  readonly parentEntityName: EntityName;
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  readonly tables: EntityTables;
  resolver(param: TParam): TOutput;
};
```

`reader_artifact_file` (`crates/typescript_artifacts/src/reader_artifact.rs`) writes `parentEntityName` and `selectableName` first and `tables` after `readerAst`. `Pet/PetName/resolver_reader.ts` gains `parentEntityName: 'Pet', selectableName: 'PetName',` and `tables: { Pet: Pet__entity_table, String: String__entity_table }`.

## `normalize` writes source nodes

```ts
// from libs/isograph-runtime/src/normalize.ts
import { startBatch } from '@isograph/pico-js';

export function normalize(store: Store, ast: NormalizationAst, response: JsonObject, variables: Variables): void {
  let thrown: { readonly error: unknown } | undefined;
  startBatch((mutators) => {
    const normalization: Normalization = {
      store,
      mutators,
      discriminatorResponseKey: ast.discriminatorResponseKey,
      variables,
      tables: ast.tables,
    };
    try {
      writeSelections(normalization, ast.selections, response, presentContainerAt(store, ast.root), ast.root, []);
    } catch (error: unknown) {
      thrown = { error };
    }
  });
  if (thrown !== undefined) {
    throw thrown.error;
  }
}

function writeValue(normalization: Normalization, link: StoreLink, key: StoreKey, value: StoreValue): void {
  const source = sourceNodeAt(normalization.store, link, key);
  if (!isUnchanged(key)(source.value, value)) {
    normalization.mutators.writeSource(source, value);
  }
}
```

A `startBatch` body must not throw, so `normalize` catches inside the body, lets the batch flush the writes before the throw, and rethrows after `startBatch` returns (Open decision 2). Callers read after `normalize` returns, so they read after the flush.

`writeSelections` and `compositeValue` pass the record's link where they passed its `StoreRecord`, and `recordAt` becomes `presentContainerAt` (`normalize.ts:163-169`). Upstream compares a scalar with `!==` only (`cache.ts:248-249`); `leafUnchanged` compares a leaf list by length and item, so a response that repeats a list marks nothing.

## End-user experience

The pet demo's fetch-and-read script reads with `readStoredResult` and prints the same output. Its `valueOrThrow` gains a `ResolverThrew` arm that throws `result.error`.

```ts
// from demos/pet-demo/src/fetchAndRead.ts
// readHomeRoute, before:
  const readHomeRoute = () =>
    valueOrThrow(homeRoute.name, read(environment.store, homeRoute.readerArtifact(), homeRoute.normalizationAst.root, {}));
// after:
  const readHomeRoute = () =>
    valueOrThrow(homeRoute.name, readStoredResult(environment.store, homeRoute.readerArtifact(), homeRoute.normalizationAst.root, {}));
```

After the `SetTagline` write, the `HomeRoute` read calls `PetSummaryCard` on `Pet:0` and `HomeRoute` once each. It does not call `PetSummaryCard` on `Pet:1` or `fullName` on either pet. A second read with no write between calls no resolver and returns the same object.

## Core changes

Each is one commit, in order, and each is independently shippable.

1. Prefactor: `normalize` skips an unchanged value on v1's plain store. Observable: normalizing the same response twice keeps every stored list and link object (`toBe`).

   ```ts
   // from libs/isograph-runtime/src/normalize.ts
   // core change 1, on v1's plain store
   // writeSelection, before (normalize.ts:96-109):
     record.set(
       storeKey(selection, normalization.variables),
       selection.target == null
         ? value
         : compositeValue(normalization, selection, selection.target, value, link, [], [...path, responseKey]),
     );
   // after:
     writeValue(
       record,
       storeKey(selection, normalization.variables),
       selection.target == null
         ? value
         : compositeValue(normalization, selection, selection.target, value, link, [], [...path, responseKey]),
       isUnchanged(selection),
     );

   function writeValue(
     record: StoreRecord,
     key: StoreKey,
     value: StoreValue,
     unchanged: (stored: StoreValue, written: StoreValue) => boolean,
   ): void {
     const stored = record.get(key);
     if (stored !== undefined && unchanged(stored, value)) {
       return;
     }
     record.set(key, value);
   }

   function isUnchanged(selection: NormalizationSelection): (stored: StoreValue, written: StoreValue) => boolean {
     return selection.target == null ? leafUnchanged : compositeUnchanged;
   }
   ```

   `leafUnchanged` and `compositeUnchanged` are the bodies in Accessors and live in `normalize.ts` until core change 4. Tests, in `test/normalize.test.ts`: "normalizing the same response twice keeps every stored list and link object"; "a changed link and a changed leaf are written, and the unchanged ones keep their stored objects"; "a leaf list equal by length and item keeps its stored list"; "a JSON object leaf is written again"; "an absent key is written".

2. Prefactor: the walk takes `ReadHooks`, `read` and `readButDoNotEvaluate` use `evaluatingHooks`, and the two `noRecord` checks go. Observable: an absent root's `MissingData` reads `No value for pet____id___"1" on Query:__ROOT`, and `Constant` on an absent record is `Ok`.

   ```ts
   // from libs/isograph-runtime/src/store.ts
   // core change 2, on v1's plain store
   /** A value is `MISSING` when no record is at the link or the record has no value under the key. */
   const MISSING: unique symbol = Symbol('Missing');
   export type Missing = typeof MISSING;

   export function isMissing(value: StoreValue | Missing): value is Missing {
     return value === MISSING;
   }

   export function storeValue(store: Store, link: StoreLink, key: StoreKey): StoreValue | Missing {
     const value = storeRecord(store, link)?.get(key);
     return value === undefined ? MISSING : value;
   }
   ```

   Tests, in `test/read.test.ts`: "a read of an absent root record is missing data on the root" (`:519`) and "an absent root is missing data on the root" (`:701`) assert `reason: 'No value for pet____id___"1" on Query:__ROOT'` (`:524`, `:708`). "a field with an empty reader AST is missing data on an absent record" (`:564`) becomes "a field with an empty reader AST is Ok on an absent record", asserting `Ok` with `data: {}` and one resolver call. New: "a read of a present record without the key and a read of an absent record give the same MissingData".

3. Prefactor: scalar records on v1's reader AST (linked-only-selections.md, change 1). This change adds the `Leaf` target, `VALUE_KEY`, `omittedRecordId`, `leafRecords` with ids from `omittedRecordId`, and the `Leaf` case of `check` and the mark. v1's `Scalar` case reads the link under its key, then `__value` at each linked record. Observable: after `petDetailResponse()`, the store holds `Pet:1.name` as `{ entityName: 'String', id: 'Pet:1.name' }` and `String:Pet:1.name` with `__value` `'Rex'`; every read returns what it returned before.

   ```ts
   // from libs/isograph-runtime/src/normalize.ts
   // core change 3, on v1's plain store
   // writeSelection, after core change 1's form:
     const target = selection.target;
     writeValue(
       record,
       storeKey(selection, normalization.variables),
       target.kind === 'Leaf'
         ? leafRecords(normalization, selection, target, value, link, [], target.listDepth, [...path, responseKey])
         : compositeValue(normalization, selection, target, value, link, [], [...path, responseKey]),
     );

   function writeValue(record: StoreRecord, key: StoreKey, value: StoreValue): void {
     const stored = record.get(key);
     if (stored !== undefined && isUnchanged(key)(stored, value)) {
       return;
     }
     record.set(key, value);
   }

   function isUnchanged(key: StoreKey): (stored: StoreValue, written: StoreValue) => boolean {
     return key === VALUE_KEY ? leafUnchanged : compositeUnchanged;
   }

   // leafRecords, Scalar records' body from `const position` to the end, before entity tables and containers:
     const link = {
       entityName: target.entityName,
       id: omittedRecordId(parent, selection, listIndexes, normalization.variables),
     };
     writeValue(recordAt(normalization.store, link), VALUE_KEY, value);
     return link;
   ```

   Tests. Every `storeOf(..)` that holds a leaf lists the link under the parent's key and the scalar record with its `__value`: `test/normalize.test.ts` (`:46`, `:79`, `:162`, `:200`, `:228`, `:259`, `:285`, `:302`, `:359`), `test/check.test.ts` (`:220`, `:226`), `test/garbageCollection.test.ts` (`:118`, `:149`, `:161`). The generated artifacts under `libs/isograph-runtime/fixtures/store/src/__isograph` and `demos/pet-demo/src/__isograph` are regenerated. New, in `test/normalize.test.ts`: "a leaf is written to the __value of its scalar record, and its key holds the link", "a rewrite that changes only a leaf writes only its __value", "a leaf list writes one scalar record per item, at its index", "a null leaf writes no scalar record", "a custom scalar's JSON array below its list depth is one __value". In `test/check.test.ts`: "a missing __value is missing its scalar record". In `test/garbageCollection.test.ts`: "a scalar record no retained query reaches is deleted".

4. Prefactor: record containers. This change adds the engine dependency, `RecordContainer` without `derived`, the accessors, `Mutators`, `storeSnapshot`, `deleteRecord` without its `derived` line, `storeFrom`, `normalize`'s `startBatch`, the sweep's `startBatch`, and `writeValue` over source nodes. `isUnchanged`, `leafUnchanged`, and `compositeUnchanged` move to `store.ts`. The exported type `StoreRecord` becomes `RecordContainer` (`libs/isograph-runtime/src/index.ts:38`). Observable: no change in behavior; `storeSnapshot(environment.store)` equals v1's nested map.

   Tests. `storeSnapshot(store)` replaces `store` in every comparison: `test/normalize.test.ts`, `test/garbageCollection.test.ts` (including `:42`, `:46`), `test/check.test.ts:236-242` (`before = storeSnapshot(store)` in place of `structuredClone(store)`), `test/network.test.ts` (`:114`, `:181`, `:209`, `:230`, `:295`, `:423`), and `test/environment.test.ts:95`. A deleted key (`test/check.test.ts:114`, `:179`, `:187`, `:193`) uses `storeFrom` without that key. A deleted record (`test/check.test.ts:122`, `:137`, `:164`, `:188`, `:213`; `test/garbageCollection.test.ts:135`; `test/read.test.ts:570`) uses `startBatch((mutators) => deleteRecord(mutators, store, link))`. A set value (`test/read.test.ts:328`, `test/subscribe.test.ts:492`) uses `writeSource(sourceNodeAt(..), 'x')`. New, in `test/store.test.ts`: "deleteRecord of an absent record changes nothing", "deleteRecord of the last record of an entity removes the entity from storeSnapshot", "storeSnapshot omits Absent containers and MISSING source nodes", "storeFrom then storeSnapshot is the identity on a PlainStore".

5. Prefactor: entity tables and the minimal reader AST (linked-only-selections.md, change 2). The compiler emits `ReaderSelection`, one `entity_table.ts` per imported entity, `ReaderArtifact.parentEntityName`, `selectableName`, and `tables`, `NormalizationAst.tables`, and a generated refinement field for every refinement of each entity whose table is generated. The runtime gains `EntityTable`, the walk, `recordId` over `RecordStorage`, `storageOf`, `storeKey(name, args, variables)`, and `selectionSetDataAreEqual` over tables. The four v1 selection kinds and `NormalizationIdentity` go. Observable: every read returns what it returned before, and `node(id:) { asPet { name } }` on a `Checkin` record is `null`.

   Tests. Every generated artifact is regenerated, and the landed tests keep their assertions. New, in `test/read.test.ts`: "a Node-typed node(id:) selection of asPet on a Checkin record is null", "asPet on a Pet record reads the selections at the record as Pet", "a Source entry missing from the entity table throws". In `test/normalize.test.ts`: "the record id comes from the entity table's storage", "a response naming an entity with no table throws". In `test/subscribe.test.ts`: "a link moved to a record with equal data is equal".

6. Prefactor: linked-only selections (linked-only-selections.md, change 3). Validation, query generation, and reader generation accept bare object selections, selection sets on scalars, client fields on scalar entities, and `__self`. The store fixture gains:

   ```text
   field Query.PetDetailPage($id: ID!) { pet(id: $id) { n1: name n2: name { toUppercase } bestFriend { name } } }
   field String.toUppercase { __self }
   ```

   Observable: `read` of `PetDetailPage` returns `{ pet: { n1: "Ricky", n2: { toUppercase: "RICKY" }, bestFriend: { name: "Mango" } } }`, and a bare `bestFriend` is `EMPTY_DATA` or `null`.

   ```ts
   // from libs/isograph-runtime/src/read.ts
   // core change 6
   // fieldOf, before (core change 5, The walk):
   export function fieldOf(table: EntityTable, entity: EntityName, fieldName: string): FieldEntry;
   // after:
   /** `fieldOf` throws when the table has no entry, because the compiler lists every field of the entity. */
   export function fieldOf(
     table: EntityTable,
     entity: EntityName,
     fieldName: string,
   ): FieldEntry | { readonly kind: 'Self' };

   // readSelection's switch gains, before `case 'Source'`:
       case 'Self':
         return nested(selection.alias, record, readAt(hooks, tables, selection.selections, record, entity, variables));

   // from libs/isograph-runtime/src/areEqualWithDeepComparison.ts
   // selectionValuesAreEqual's switch gains, before `case 'Source'`:
       case 'Self':
         return targetValuesAreEqual(tables, entity, selection.selections, delivered, read);
   ```

   Tests, in `test/read.test.ts`: "PetDetailPage reads n1, n2 through toUppercase, and bestFriend", "a bare bestFriend is EMPTY_DATA, or null when the link is null", "a bare __self on a scalar record is its value", "__self with a selection set reads at the same record".

7. Stored resolver results. This change adds `RecordContainer.derived`, Stored resolver results, `ResolverResult`, `resolverResultsEqual`, `resolverStoreKey`, and `deleteRecord`'s `derived` line. The package index adds `readStoredResult` and the types `ResolverResult`, `ReadResolverOk`, `OkOrigin`, `ReadResolverThrew`. The pet script reads with `readStoredResult`. Observable: End-user experience; after a write to `Pet:2.name`, a read of `PetDetailPage` does not call `toUppercase`.

   Tests, in a new `test/storedResults.test.ts`, counting resolver calls with `vi.fn`: "a second read with no write calls no resolver and returns the same object"; "after a write to Pet:2.name, PetDetailPage runs and toUppercase does not"; "a write that leaves toUppercase's result equal reruns a reader that reads n1, and not a reader without n1"; "a resolver that throws is stored as ResolverThrew and is not called again until an input changes"; "MissingData on an absent record becomes Ok after normalize writes the record"; "deleting a record disposes the nodes it owns and the next read recomputes their readers"; "a refinement node is computed once and not marked by a write to its record"; "two selections of one field with equal arguments on one record share one node"; "F____n___2 with no variables and F with { n: 2 } are two nodes"; "a resolver with an empty reader AST on an absent record is Ok and computed once".

8. The subscription re-read reads nested resolvers as stored results. `readSubscription` reads with `readData(storedResultHooks(store), ..)`, `SubscriptionResult` gains `ReadResolverThrew`, `makeNetworkRequest` calls `callSubscriptions` in a `finally` after `normalize`, and `garbageCollectEnvironment` calls `callSubscriptions` after the sweep. Observable: in the pet script, the re-read after the tagline write calls `PetSummaryCard` on `Pet:0` only; a re-read after an unrelated write calls no resolver and not the callback; a response that changes `Pet:0.tagline` and then throws delivers the new tagline; a collection that deletes a record a subscription read delivers `MissingData`.

   ```ts
   // from libs/isograph-runtime/src/network.ts
   // makeNetworkRequest's .then body, before (libs/isograph-runtime/src/network.ts:143-149):
         normalize(
           environment.store,
           entrypoint.normalizationAst,
           response,
           variables,
         );
         callSubscriptions(environment);
   // after:
         try {
           normalize(
             environment.store,
             entrypoint.normalizationAst,
             response,
             variables,
           );
         } finally {
           callSubscriptions(environment);
         }
   ```

   ```ts
   // from libs/isograph-runtime/src/environment.ts
   // before (libs/isograph-runtime/src/environment.ts:62-66):
   export function garbageCollectEnvironment(
     environment: IsographEnvironment,
   ): void {
     garbageCollect(environment.store, Array.from(environment.retainedQueries));
   }
   // after:
   export function garbageCollectEnvironment(
     environment: IsographEnvironment,
   ): void {
     garbageCollect(environment.store, Array.from(environment.retainedQueries));
     callSubscriptions(environment);
   }
   ```

   ```ts
   // from libs/isograph-runtime/src/storedResults.ts
   /** `storedResultHooks` records nothing and reads each nested resolver as its pulled stored result. */
   export function storedResultHooks(store: Store): ReadHooks<ReadResolverThrew> {
     return {
       readValue: (record, key) => storeValue(store, record, key),
       readResolver: (artifact, root, variables) => readStoredResult(store, artifact, root, variables),
     };
   }

   // from libs/isograph-runtime/src/subscribe.ts
   export type SubscriptionResult =
     | ReadOk<SelectionSetData>
     | ReadMissingData
     | ReadResolverThrew
     | ReadErr;

   /** A read that threw means a store invariant broke (store.md, Errors). */
   export type ReadErr = {
     readonly kind: 'Err';
     readonly error: unknown;
   };

   // readSubscription, before (libs/isograph-runtime/src/subscribe.ts:126-130):
     try {
       return readButDoNotEvaluate(environment.store, artifact, root, variables);
     } catch (error: unknown) {
       return { kind: 'Err', error };
     }
   // after:
     try {
       return readData(storedResultHooks(environment.store), artifact, root, variables);
     } catch (error: unknown) {
       return { kind: 'Err', error };
     }

   // subscriptionResultsAreEqual, after core change 5 and this change (before: subscribe.ts:135-155):
   function subscriptionResultsAreEqual(
     artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
     delivered: SubscriptionResult,
     result: SubscriptionResult,
   ): boolean {
     switch (delivered.kind) {
       case 'Ok':
         return (
           result.kind === 'Ok' &&
           selectionSetDataAreEqual(artifact.tables, artifact.parentEntityName, artifact.readerAst, delivered.value, result.value)
         );
       case 'MissingData':
         return result.kind === 'MissingData';
       case 'ResolverThrew':
         return result.kind === 'ResolverThrew' && delivered.error === result.error;
       case 'Err':
         return result.kind === 'Err' && delivered.error === result.error;
     }
   }
   ```

   `storedResultHooks` is not exported from the package index. Tests, in `test/subscribe.test.ts`:

   - "a nested resolver that throws at subscribe gives an Err snapshot" (`:444`) becomes "... gives a ResolverThrew snapshot" and asserts `kind: 'ResolverThrew'` (`:453-456`).
   - "a nested resolver that throws at a re-read delivers Err, and other subscriptions are still called" (`:459`) becomes "... delivers ResolverThrew ..." (`:475-478`), and the `callSubscriptions` with no write between (`:481-483`) leaves `onThrowingResult` at one call.
   - "a read that throws after a response leaves the request Ok" (`:631`) asserts `'ResolverThrew'` (`:657`).
   - "nested resolvers that return new objects make every re-read unequal" (`:175`) becomes "nested resolvers that return new objects are not called again after an unrelated write", and after `writePet9` `onResult` has no call (`:202-203`).
   - "a stored non-link under a composite store key delivers Err with the store message" (`:486`) still asserts `Err`.
   - New: "a re-read after an unrelated write calls no nested resolver and not the callback"; "a response that changes a value and then throws delivers the changed value"; "a collection that deletes a record a subscription read delivers MissingData".

## Later

- Retention and the fetch decision (`retainEntrypoint`, root holds, `IfNecessary` pulls the root node)
- `collect`: disposing derived nodes no root reaches
- Subscriptions and React observers as derived nodes whose `onDirty` schedules the re-read, in place of `callSubscriptions` re-reading every subscription after each write. A subscription disposes its node when it stops.
- A selector that many components read is created with `createEagerDerivedNode`. A React binding calls `setDerivedNodeEvaluation(node, LAZY)` when its screen is retained and `setDerivedNodeEvaluation(node, EAGER)` when it mounts (`refactors/pending/runtime/pico-js-where-it-wins.md`).
- Values with lifecycles
- Optimistic layers
- Request state, browser state, and client roots as source nodes
- Runtime-defined computations
- An `Embedded` storage kind
- Entity tables loaded on demand

## Open decisions

### 1. How the resolver store key encodes variables

- (a) `JSON.stringify(stableCopy(variables))` after `<parentEntityName>.<selectableName>/`.
- (b) `argumentChunks` of the variables after the selectable name.

Held: (a), because it is upstream's encoding (`FragmentReference.ts:75-79`) and it is injective, while (b) gives `F____n___2` with no variables and `F` with `{ n: 2 }` one node.

### 2. A response that throws part-way

- (a) Writes before the throw and their marks stand: `normalize` catches inside its `startBatch` body, the batch flushes, and `normalize` rethrows. `makeNetworkRequest` re-reads subscriptions in a `finally`.
- (b) `normalize` validates the whole response before it writes.
- (c) `normalize` writes into a layer that is discarded on a throw.

Held: (a), because upstream keeps a partial write too (`cache.ts:77-85`).

### 3. A resolver that throws

- (a) Stored as `ResolverThrew`, which a parent's walk passes up as it passes `MissingData`.
- (b) The throw propagates out of the read, as in v1.

Held: (a), because a stored throw is not recomputed until an input changes; upstream propagates (`read.ts:600`).

### 4. Whether `readDataForOnComplete` reads stored results

- (a) It keeps calling `read`.
- (b) `readData(storedResultHooks(store), ..)`, then the entrypoint's resolver, rethrowing a nested `ResolverThrew`.
- (c) `readStoredResult` of the entrypoint at its root.

Held: (a), because it is upstream's behavior (`makeNetworkRequest.ts:362-420`).

### 5. How a field supplies its output equality

- (a) A second argument to the field literal's function, `iso(`field Pet.Shape { name }`)(resolver, { equal: deepEqual })`, read as `artifact.resolver.equal ?? identical`.
- (b) A declaration directive naming a built-in comparator.
- (c) A map from selectable to comparator passed to `createIsographEnvironment`.
- (d) No API; a resolver returns a stable value itself.

Held: (a), because it needs no directive, no new iso syntax, and no compiler change beyond the generated `iso` overload.

### 6. An unreached `Absent` container whose source nodes have readers

- (a) The sweep deletes it and marks its readers, as for any record.
- (b) The sweep keeps it while any of its source nodes has a dependent or it owns a derived node.

Held: (a), because upstream deletes every unreached record (`garbageCollection.ts:88-119`).

### 7. `storeSnapshot` and scalar records

- (a) The snapshot returns scalar records as stored.
- (b) `RecordContainer` gains `origin: 'Response' | 'ScalarValue'`, and the snapshot inlines each scalar's `__value` into its parent's key.

Held: (a), because the runtime cannot tell a scalar record from another record without a field that only the snapshot reads.

### 8. The name of the self field

- (a) `__self`.
- (b) `__link`, upstream's name (`add_link_fields.rs:14-38` in `../isograph/crates/isograph_schema/src/validated_isograph_schema`).

Held: (a), because upstream's `__link` yields the record's link and takes no selection set, while this field yields the data read at the record or a scalar's value.

### 9. How the walk gives `__self` its value

- (a) `fieldOf` recognizes the name `__self` and returns `{ kind: 'Self' }`.
- (b) A generated derived field `__self` in every table, whose resolver returns `link`.
- (c) A source node `normalize` writes on every record, holding its own link.

Held: (a), because upstream reads `__link` with a reader AST node of its own kind (`read.ts:189-192`) and needs no node.

### 10. The target of an opaque client field's `Derived` entry

- (a) `target: EntityName | null`, `null` for an opaque client field.
- (b) `target: EntityName`, and the walk treats any link-shaped value as a link.

Held: (a), because (b) reads an opaque output shaped like a link as a record.

### 11. What a bare selection reads

- (a) The entity table's `shape` says `Leaf` or `Composite`.
- (b) The walk reads `__value` at every bare-selected record and treats `MISSING` as `EMPTY_DATA`.

Held: (a), because (b) reads an absent scalar record as `EMPTY_DATA` where it is missing data.
