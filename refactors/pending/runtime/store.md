# Runtime: the v1 store, normalizer, data checker, and garbage collector

Status: under discussion. Not approved for implementation.

This is the first code in `@isograph/runtime` (`libs/isograph-runtime`): a store, `normalize`, `check`, and `garbageCollect`. Each is a pure function over the store and the normalization AST artifact. The package has no knowledge of React and no React dependency. The store is the only state: nothing is hidden in an environment, a module-level variable, or a closure, and every function returns its result instead of mutating its arguments.

It plans against `refactors/pending/graphql-protocol/normalization-ast.md` and `query-generation.md` as revised in the `b-prime-doc` worktree (normalization-ast.md "approved for implementation", commit `65a13a5e6` plus its uncommitted revision). Prerequisite: normalization-ast.md changes 1 to 3 (the package, the AST, the TypeScript artifact), so `normalization_ast.ts` artifacts are real.

`refactors/pending/runtime/pico-runtime.md` stays parked; v1 has no memoization, no reader, no subscriptions, and no network layer. `refactors/pending/runtime/fetch-policies.md` lists directions beyond `check`.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src/core`.

## End-user experience

v1's only callers are the package's tests. A test normalizes a response, asserts on the store, checks, and collects:

```ts
// from libs/isograph-runtime/test/normalize.test.ts
import { expect, test } from "vitest";
import petDetail from "../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast";
import { check, garbageCollect, normalize } from "../src/index";
import { emptyStore, queryRoot, storeOf } from "./fixtures";

test("a pet with an identity-less stats object", () => {
  const result = normalize(
    emptyStore,
    petDetail,
    {
      pet____id___v_id: {
        __typename: "Pet",
        id: "1",
        name: "Rex",
        nickname: null,
        stats: { __typename: "PetStats", weight: 7, energy: null },
        // ...
      },
    },
    { id: "1" },
    queryRoot,
  );
  expect(result.kind).toBe("Normalized");
  if (result.kind !== "Normalized") return;
  expect(result.store).toEqual(
    storeOf({
      Query: { __ROOT: { pet____id___1: { entity: "Pet", id: "1" } } },
      Pet: { "1": { id: "1", name: "Rex", nickname: null, stats: { entity: "PetStats", id: "Pet:1.stats" } /* ... */ } },
      PetStats: { "Pet:1.stats": { weight: 7, energy: null } },
    }),
  );
  expect(check(result.store, petDetail, { id: "1" }, queryRoot)).toEqual({ kind: "EnoughData" });
  expect(check(result.store, petDetail, { id: "2" }, queryRoot)).toEqual({ kind: "MissingData", record: queryRoot });
  expect(garbageCollect(result.store, [])).toEqual(new Map());
});
```

What this shows:

- The response is read by each node's `responseKey` (`pet____id___v_id`); the store is written by the store key, with variable values substituted (`pet____id___1`).
- `Pet:1` is keyed by its identity; `PetStats` has none and gets upstream's derived id `Pet:1.stats`.
- `null` is stored as `null` and is known data. `__typename` is not stored: the record's entity is its first-layer key.
- `check` with other variables reports the root record, which lacks `pet____id___2`.
- Collecting with nothing retained empties the store.

## The store

```ts
// from libs/isograph-runtime/src/store.ts
/** Every record: entity name, then record id, then the record. */
export type Store = ReadonlyMap<EntityName, ReadonlyMap<RecordId, StoreRecord>>;

/** One record's values by store key. An absent key is missing data; `null` is known data. */
export type StoreRecord = ReadonlyMap<StoreKey, StoreValue>;

export type EntityName = string;
export type RecordId = string;
/** A selectable name followed by one chunk per argument, variables substituted. */
export type StoreKey = string;

/** Which of the two a value is, the AST selection under that store key says. */
export type StoreValue = LeafValue | CompositeValue;

/** The response's value for a selection of a leaf entity, as received. */
export type LeafValue = JsonValue;

/** The value of a selection of a composite entity: `null`, a link, or a list of these at any depth. */
export type CompositeValue = null | StoreLink | ReadonlyArray<CompositeValue>;

/** Points at record `id` of entity `entity`. */
export type StoreLink = {
  readonly entity: EntityName;
  readonly id: RecordId;
};

export type JsonValue = null | boolean | number | string | ReadonlyArray<JsonValue> | JsonObject;
export type JsonObject = { readonly [key: string]: JsonValue };

/** Operation variables by name. An absent name is an unset variable. */
export type Variables = { readonly [name: string]: JsonValue };
```

The type is the two-layer map and nothing else. The empty store is `new Map()`. There is no root record until a function writes one, and no record is ever `null`.

`ReadonlyMap` at all three layers, not plain objects, because ids and store keys are arbitrary strings: a record id `"constructor"` or a selectable named `constructor` (a valid GraphQL name) would read `Object.prototype.constructor` from a plain object. For the same reason every read of a response object or of `variables` goes through `Object.hasOwn(object, key) ? object[key] : undefined` (ES2022).

A link carries no tag. A `LeafValue` can have the same shape as a `StoreLink` (a `JSON` scalar `{ entity: "Pet", id: "1" }`); no function confuses them, because each reads a store key only under an AST selection whose `selections` is `null` (leaf) or an array (composite). Upstream's links are `{ __link, __typename }` (`IsographEnvironment.ts:122-125`), and its walks also go through the AST.

Root records: an operation's root record is `{ entity: <root operation type>, id: <fixed id> }`, passed by the caller. The fixed id is the caller's convention; v1's tests use `"__ROOT"` (upstream `ROOT_ID`, `IsographEnvironment.ts:163`). The package does not export it, because no v1 function reads it.

Deltas from upstream (`IsographEnvironment.ts:122-169`, `:211-217`):

- Maps, not objects; no `null` record (upstream `StoreLayerData` allows `StoreRecord | null`, `:165-169`).
- No pre-created `Query.__ROOT` (`createIsographStore`, `:211-217`).
- No store layers, optimistic or otherwise (`getStoreRecordProxy` and `getMutableStoreRecordProxy`, `optimisticProxy.ts:24`, `:46`); the store is the base layer's data.
- `__typename` and `id` are not special fields of a record type (`StoreRecord`'s `__typename?`, `id?`, `:150-157`). `id` is stored because it is an AST selection; the discriminator is not stored.

## Store keys

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** `selection.selectableName`, then `argumentChunks(selection.arguments, variables)`. */
export function storeKey(selection: NormalizationSelection, variables: Variables): StoreKey;

/** One `____<name>___<value>` chunk per argument, in the AST's order (sorted by name). */
export function argumentChunks(args: Arguments | null, variables: Variables): string;
```

The value of a chunk, upstream's `getStoreKeyChunkForArgument` and `getStoreKeyChunkForArgumentValue` (`cache.ts:481-521`):

- `Literal` (number, boolean, `null`): `${value}` (`5`, `true`, `null`).
- `String`: the string verbatim (`Ada Lovelace`, spaces kept; the response key replaces non-word characters, the store key does not).
- `Variable`: the variable's value when set, `"null"` when unset; a set value that is an object or array is `JSON.stringify(stableCopy(value))`, other values `${value}`.
- `Object`: the entries as an object, each value substituted by these rules, then `JSON.stringify(stableCopy(object))`.

`stableCopy` is upstream's (`util.ts:44-59`): object keys sorted, arrays element by element. It lives in `storeKey.ts`, typed `(value: JsonValue) => JsonValue`.

`noPropertyAccessFromIndexSignature` and the `Object.hasOwn` read apply to `variables`. The collisions this format has are open decision 6.

Delta from upstream: none in the format. Upstream's overloads for `LinkedParentRecordKey` and `ScalarParentRecordKey` (`cache.ts:449-469`) are one `StoreKey`, because a link and a leaf are told apart by the AST. `Enum` has no case (normalization-ast.md: `ArgumentValue` has no `Enum`).

## Record ids

```ts
// from libs/isograph-runtime/src/recordId.ts
/** The identity of `response`, or the id derived from where it sits under `parent`. */
export function recordId(
  response: JsonObject,
  parent: StoreLink,
  selection: NormalizationSelection,
  listIndexes: ReadonlyArray<number>,
  variables: Variables,
): RecordIdResult;

export type RecordIdResult =
  | { readonly kind: "Id"; readonly id: RecordId }
  | { readonly kind: "IdentityNotString" };
```

- A response object with an identity value: that value, which must be a string (GraphQL serializes `ID` as a string, spec 3.5.5); any other value is `IdentityNotString`. Which response key is the identity is open decision 2.
- Otherwise the derived id, upstream's (`getDataIdOfNetworkResponse`, `cache.ts:599-642`): `${parent.entity}:${parent.id}.${selection.selectableName}`, then `.${index}` for each list level from the outermost, then `argumentChunks(selection.arguments, variables)`. Examples: `Pet:1.stats`, `Query:__ROOT.feed.1____limit___2`, `Query:__ROOT.statGrid.0.1`.

Deltas from upstream: one `.${index}` per list level, because upstream's normalizer handles one list level only (`cache.ts:275-309`, and `getDataIdOfNetworkResponse` takes one `index`); upstream's `__typename === 'Query'` special case (`cache.ts:614-618`) is open decision 5.

## Conditions

```ts
// from libs/isograph-runtime/src/condition.ts
/** Whether `condition` holds for an object whose discriminator value is `discriminator`. */
export function conditionHolds(condition: NormalizationCondition, discriminator: JsonValue | undefined): boolean;
```

`SelectionEquals` holds when `discriminator === condition.value`. `undefined`, `null`, a number, or another string is false.

One evaluator serves all three traversals. `normalize` passes `response[condition.responseKey]` of the response object the conditional sits in. `check` and `garbageCollect` pass the current record's entity name, which is the first-layer key the record was reached under: the entity `normalize` read from that same response's discriminator. The two agree for every record `normalize` wrote.

Delta from normalization-ast.md's declaration `conditionHolds(condition, response, variables)` in `normalize.ts`: it takes the discriminator value rather than the response, so check and garbage collection can call it against a stored record, which holds no discriminator; it has no `variables` because no v1 condition reads one (`VariableEquals` is Later there). Open decision 7.

## `normalize`

```ts
// from libs/isograph-runtime/src/normalize.ts
/** `store` with `response` written from `root` down, by `ast` and `variables`. `store` is not modified. */
export function normalize(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
  root: StoreLink,
): NormalizeResult;

export type NormalizeResult =
  | { readonly kind: "Normalized"; readonly store: Store }
  | { readonly kind: "MalformedResponse"; readonly malformed: MalformedResponse };

/** Where the response disagrees with the AST, and how. */
export type MalformedResponse = {
  /** Response keys and list indexes from the top of the response. */
  readonly path: ReadonlyArray<string | number>;
  readonly reason: MalformedResponseReason;
};

export type MalformedResponseReason =
  /** A composite selection's value (or a list item at any depth) is a string, number, or boolean. */
  | "CompositeValueNotObject"
  /** A composite response object has no string under `ast.discriminatorResponseKey`. */
  | "DiscriminatorNotString"
  | "IdentityNotString";
```

`response` is the operation's `data` object, not the GraphQL envelope; v1 has no network layer and handles no `errors`.

Behavior, walking `ast.selections` against `response` onto the record `root`, whose entity is `root.entity` (the top level selects no discriminator, query-generation.md):

- The record at a link is the store's record if present, else a new empty record. Fields the AST does not select are kept. A store key written twice in one call (two paths to `Pet:1`) holds the later write, in AST and list order.
- `Selection` with `selections: null` (leaf): writes `response[responseKey]` under `storeKey(selection, variables)`, as received: a string, number, boolean, `null`, a list of leaves at any depth, or a `JSON` object.
- `Selection` with an array (composite), by the response value:
  - `null`: writes `null`.
  - An object: its entity is `value[ast.discriminatorResponseKey]` (else `DiscriminatorNotString`), its id `recordId(value, parentLink, selection, [], variables)`. The record is written by walking `selection.selections` against `value`, and the parent gets the link `{ entity, id }`. `selections` may be `[]` (normalization-ast.md: `stats { X @loadable }`); the record is still created, empty.
  - A list: the value is the list with each item replaced, recursively: `null` stays `null`, an object becomes a link as above with its index path, a nested list recurses. `[]` writes `[]`.
  - A string, number, or boolean, here or as a list item: `CompositeValueNotObject`.
- `Conditional`: when `conditionHolds(condition, response[condition.responseKey])`, walks `selections` against the same response object onto the same record; otherwise nothing.
- A response key absent from the response object: open decision 3.
- The discriminator is never written to a record.

On `MalformedResponse` nothing is returned but the error; the caller keeps `store`, so a malformed response writes nothing. A malformed response is a socket read, so it is a typed result, not a throw (AGENTS.md, Panic and UnexpectedError).

Implementation: a local copy-on-write draft (`Map<EntityName, Map<RecordId, Map<StoreKey, StoreValue>>>`): the first write to an entity in this call copies that entity's map from `store`, the first write to a record copies the record. Untouched entities and records are the input's objects. The internal walk returns `{ kind: "Ok" } | MalformedResponse` per selection set and `{ kind: "Ok"; value: CompositeValue } | MalformedResponse` per composite value, and stops at the first `MalformedResponse`. Each call costs a copy of every touched entity map, linear in that entity's record count; a persistent map would remove that and is not v1.

Deltas from upstream (`cache.ts:77-437`):

- Returns a new store; upstream mutates a store layer through `getMutableStoreRecordProxy` (`cache.ts:93`, `:423`).
- No `EncounteredIds` and no change detection (`RecordHasBeenUpdated`, `cache.ts:151`, `:211-219`, `:231-250`, `:309`, `:337-338`, `dataIdsAreTheSame` `:372-393`): nothing reads them without subscriptions.
- Reads `responseKey` instead of `getNetworkResponseKey` (`cache.ts:538-591`), and `ast.discriminatorResponseKey` instead of `astNode.concreteType ?? response.__typename` (`cache.ts:295-296`, `:322-323`, `:412-413`).
- A malformed response is `MalformedResponse`; upstream throws on a missing `__typename` (`cache.ts:297-302`) and writes anything else it meets.
- Lists of lists and lists of leaves at any depth.

## `check`

```ts
// from libs/isograph-runtime/src/check.ts
/** Whether `store` holds every value `ast` selects from `root` with `variables`. */
export function check(store: Store, ast: NormalizationAst, variables: Variables, root: StoreLink): CheckResult;

export type CheckResult =
  | { readonly kind: "EnoughData" }
  /** `record` lacks a selected value, or is linked to and absent. */
  | { readonly kind: "MissingData"; readonly record: StoreLink };
```

Upstream's semantics (`check.ts:43-215`): `null` is known data, only an absent key or an absent record is missing (`check.ts:88-89`). The walk returns the first missing position in AST order, then list order:

- `root` absent from the store: `MissingData { record: root }`.
- Leaf selection: absent key is `MissingData { record: current }`; any value, `null` included, is enough.
- Composite selection: absent key is `MissingData { record: current }`; `null` is enough; a link whose record is absent is `MissingData { record: link }`; a present record recurses with `selection.selections`, even when they are `[]`; a list recurses into every item and nested list, `null` items being enough. A value that is not a `CompositeValue`: open decision 4.
- `Conditional`: when `conditionHolds(condition, current.entity)`, recurses on the same record; otherwise enough.

v1 has no `shouldFetch` decision function. The later network layer decides upstream's `'Yes' | 'No' | 'IfNecessary'` (default `IfNecessary`, `check.ts:13-16`; `makeNetworkRequest.ts:67-118`) by calling `check`.

Deltas from upstream:

- An inline fragment never reports `MissingData` for a record without `__typename` (`check.ts:184-191`): the record's entity is always known.
- No `null` records (`check.ts:130-131`, `:163-164`) and no `throw` on a non-link (`check.ts:116-121`, `:149-154`); open decision 4.
- Lists of lists.

## `garbageCollect`

```ts
// from libs/isograph-runtime/src/garbageCollection.ts
/** What the caller keeps: an AST, its variables, and the record it was normalized from. */
export type RetainedQuery = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly root: StoreLink;
};

/** `store` with only the records some query in `retained` reaches. `store` is not modified. */
export function garbageCollect(store: Store, retained: ReadonlyArray<RetainedQuery>): Store;
```

Mark, then sweep (upstream `garbageCollectBaseStoreLayer`, `garbageCollection.ts:88-119`, and `recordReachableIds`, `:125-226`):

- Each retained query marks its root when the root record exists, and walks its AST from it.
- A leaf selection marks nothing. A composite selection follows each link in its value (every list level), and marks and walks each linked record that exists. An absent key, `null`, or an absent record stops that path. A value that is not a `CompositeValue`: open decision 4.
- `Conditional`: when `conditionHolds(condition, current.entity)`, walks the same record.
- The result holds every marked record whole, including fields no retained AST selects, and no entity whose records were all unmarked. `retained = []` returns `new Map()`.
- Cyclic data terminates: the walk recurses on the AST, which is finite. A record reached by several paths is walked once per path.

The mark is an iterator, `function* reachableRecords(store, retained): Generator<StoreLink>`, consumed into a `Map<EntityName, Set<RecordId>>`; the sweep builds the new store from it.

It returns the collected store, not the set of records to delete, because the store is the only state and the caller's next step is to use the collected store: a delete set would need an exported function to apply it (or the caller doing map surgery), and the tests would apply it before asserting. Returning `Store` gives `garbageCollect` the same `Store` to `Store` shape as `normalize`.

The caller decides what is retained and when collection runs; there is no retain count, no release buffer, and no trigger.

Deltas from upstream:

- No retained set on an environment, no `gcBuffer` of recently released queries or its size, and no trigger on release (`garbageCollection.ts:39-63`, `:78-83`; `makeNetworkRequest.ts:565-577`; `IsographEnvironment.ts:177`, `:204-206`).
- No skip while optimistic layers exist (`garbageCollection.ts:66-68`) or while a lazily loaded AST is pending (`:70-76`): neither exists.
- Upstream marks a root id even when its record is absent (`garbageCollection.ts:133-136`); here an absent record has nothing to keep.
- Links carry their entity, so there is no `concreteType` store lookup (`garbageCollection.ts:188-204`).

## Removed from upstream

- No log events: upstream's `logMessage` calls (`cache.ts:86-91`, `check.ts:65-68`) and the environment's `loggers` (`IsographEnvironment.ts:207`) are not carried over; tests observe return values and store contents.
- No `IsographEnvironment` (`IsographEnvironment.ts:73-93`, built by `createIsographEnvironmentCore`, `:178-209`): the functions take the store and their inputs.
- No subscriptions (`cache.ts:109-149`).

## The package

Files added to the package normalization-ast.md change 1 creates:

```text
libs/isograph-runtime/
  src/store.ts
  src/storeKey.ts
  src/recordId.ts
  src/condition.ts
  src/normalize.ts
  src/check.ts
  src/garbageCollection.ts
  test/fixtures.ts
  test/storeKey.test.ts
  test/condition.test.ts
  test/normalize.test.ts
  test/check.test.ts
  test/garbageCollection.test.ts
  fixtures/store/isograph.config.json
  fixtures/store/schema/schema.graphql
  fixtures/store/src/*.ts
  fixtures/store/src/__isograph/**   (generated)
```

```ts
// from libs/isograph-runtime/src/index.ts
export type {
  Argument,
  Arguments,
  ArgumentValue,
  NormalizationAst,
  NormalizationCondition,
  NormalizationConditional,
  NormalizationNode,
  NormalizationSelection,
  NormalizationSelectionSet,
} from "./artifacts";
export type {
  CompositeValue,
  EntityName,
  JsonObject,
  JsonValue,
  LeafValue,
  RecordId,
  Store,
  StoreKey,
  StoreLink,
  StoreRecord,
  StoreValue,
  Variables,
} from "./store";
export { check, type CheckResult } from "./check";
export { garbageCollect, type RetainedQuery } from "./garbageCollection";
export {
  normalize,
  type MalformedResponse,
  type MalformedResponseReason,
  type NormalizeResult,
} from "./normalize";
```

`storeKey`, `argumentChunks`, `recordId`, `stableCopy`, and `conditionHolds` are module exports that `normalize`, `check`, and `garbageCollect` import; the package index does not export them. Their tests import the modules directly.

```json
// from libs/isograph-runtime/package.json
{
  "scripts": {
    "typecheck": "tsc --noEmit",
    "test": "vitest run"
  },
  "devDependencies": {
    "typescript": "5.6.3",
    "vitest": "^2.1.2"
  }
}
```

Delta from normalization-ast.md's `package.json`: the `test` script and `vitest` at the babel plugin's version (`libs/isograph-babel-plugin/package.json`). Every other key is unchanged. `exports` keeps only its `types` condition: in v1 no module outside the package imports its runtime values, and tests import `../src/index`.

```json
// from libs/isograph-runtime/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2022"
  },
  "include": ["./src/**/*.ts", "./test/**/*.ts"]
}
```

Delta from normalization-ast.md's `tsconfig.json`: `target` `ES2022` (AGENTS.md, TypeScript; `Object.hasOwn` is ES2022), and `test` in `include`. Every other option is unchanged. The fixture artifacts are checked because the tests import them; they `import type { NormalizationAst } from "@isograph/runtime"`, which resolves to the package itself through its `exports` (TypeScript's package self-reference under `moduleResolution: "bundler"`). If `tsc` does not resolve the self-reference, the implementer stops and raises it.

`.github/workflows/js.yml` gains `pnpm --filter @isograph/runtime test` beside the runtime's `Typecheck` step (normalization-ast.md). `.pre-commit-config.yaml` gains `runtime-test` and `runtime-typecheck`, the babel plugin's two hooks with `@isograph/runtime` and `files: '^libs/isograph-runtime/'`. `.prettierignore` and `.oxlintrc.json` already ignore `__isograph`.

## Test infrastructure

Tests use vitest, as `libs/isograph-babel-plugin` does (`vitest run`). Every normalization AST a test uses is generated by the real compiler from the fixture project `libs/isograph-runtime/fixtures/store`, whose artifacts are committed under `fixtures/store/src/__isograph/`. How they are regenerated is open decision 8. Tests do not use the pet demo's artifacts: its schema has no list of lists and no `JSON` scalar, and an edit to the demo would break the runtime's tests.

Tests build responses, variables, and stores by hand; those are inputs. Tests assert on the store's contents after `normalize` and after `garbageCollect`, on `CheckResult`, and on `NormalizeResult`; the store's shape is observable behavior.

```ts
// from libs/isograph-runtime/test/fixtures.ts
export const queryRoot: StoreLink = { entity: "Query", id: "__ROOT" };
export const mutationRoot: StoreLink = { entity: "Mutation", id: "__ROOT" };
export const emptyStore: Store = new Map();

/** The store whose entities, ids, store keys, and values are `records`' keys and values, in order. */
export function storeOf(records: {
  readonly [entity: string]: { readonly [id: string]: { readonly [storeKey: string]: StoreValue } };
}): Store;

/** The result's store; fails the test with the malformed response otherwise. */
export function normalized(result: NormalizeResult): Store;
```

```json
// from libs/isograph-runtime/fixtures/store/isograph.config.json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src"
}
```

```graphql
# from libs/isograph-runtime/fixtures/store/schema/schema.graphql
schema {
  query: Query
  mutation: Mutation
}

scalar ID
scalar String
scalar Int
scalar Boolean
scalar JSON

type Query {
  node(id: ID!): Node
  pet(id: ID!): Pet
  pets: [Pet!]!
  maybePets: [Pet]
  petGrid: [[Pet]]
  statGrid: [[PetStats]]
  feed(limit: Int!): [FeedItem!]!
  search(filter: PetFilter!): [Pet!]!
  greeting(name: String, loud: Boolean, times: Int): String
  tags: [String]
  settings: JSON
}

type Mutation {
  setTagline(input: SetTaglineInput!): SetTaglinePayload!
}

interface Node {
  id: ID!
}

type Pet implements Node {
  id: ID!
  name: String!
  nickname: String
  tagline: String!
  stats: PetStats
  bestFriend: Pet
  checkins(limit: Int, skip: Int): [Checkin!]!
}

type PetStats {
  weight: Int
  energy: Int
}

type Checkin implements Node {
  id: ID!
  location: String!
}

union FeedItem = BlogItem | AdItem

type BlogItem implements Node {
  id: ID!
  title: String!
  author: Pet
}

type AdItem {
  advertiser: String!
}

input PetFilter {
  nameContains: String
  minAge: Int
}

input SetTaglineInput {
  id: ID!
  tagline: String!
}

type SetTaglinePayload {
  pet: Pet!
}
```

The source files, one per entrypoint, each `export const X = iso(\`field ...\`)(function X({ data }) { return data; });` followed by `iso(\`entrypoint ...\`);`, as `demos/pet-demo/src/*.ts` do. The selection sets:

```text
field Query.Leaves($name: String) {
  greeting
  literalGreeting: greeting(name: "Ada Lovelace", loud: true, times: 3)
  nullGreeting: greeting(name: null)
  variableGreeting: greeting(name: $name)
  tags
  settings
}

field Query.PetDetail($id: ID!, $skip: Int) {
  pet(id: $id) {
    id name nickname tagline
    stats { weight energy }
    bestFriend { id name bestFriend { id name } }
    checkins(limit: 2) { id location }
    pagedCheckins: checkins(limit: 2, skip: $skip) { id location }
  }
}

field PetStats.StatsCard { weight }
field Query.EmptyStats($id: ID!) {
  pet(id: $id) { stats { StatsCard @loadable } }
}

field Query.PetLists {
  pets { id name }
  maybePets { id name }
  petGrid { id name }
  statGrid { weight }
}

field Query.Feed($limit: Int!) {
  feed(limit: $limit) {
    asBlogItem { id title author { id name } }
    asAdItem { advertiser }
  }
}

field Query.Search($q: String) {
  search(filter: { nameContains: $q, minAge: 2 }) { id name }
}

field Pet.PetCard { name tagline }
entrypoint Pet.PetCard

field Mutation.SetTagline($id: ID!, $tagline: String!) {
  setTagline(input: { id: $id, tagline: $tagline }) { pet { id tagline } }
}
```

with `entrypoint Query.Leaves`, `Query.PetDetail`, `Query.EmptyStats`, `Query.PetLists`, `Query.Feed`, `Query.Search`, and `Mutation.SetTagline`. If the compiler rejects one (for instance `@loadable` on an identity-less `PetStats` field), the implementer stops and raises it.

## Tests

### Store key (`test/storeKey.test.ts`)

Selections are taken from the fixture ASTs by response key.

- `greeting` with no arguments is `greeting`.
- `literalGreeting` is `greeting____loud___true____name___Ada Lovelace____times___3`: chunks in the AST's (sorted) order, `String` verbatim with its space, `Literal` boolean and number.
- `nullGreeting` is `greeting____name___null`.
- `variableGreeting` with `{ name: "Bo" }` is `greeting____name___Bo`; with `{}` it is `greeting____name___null`, equal to `nullGreeting`'s (open decision 6).
- `search` with `{ q: "Rex" }` is `search____filter___{"minAge":2,"nameContains":"Rex"}`, keys sorted; with `{}` the entry is `"nameContains":"null"`.
- `pet` with `{ id: "1" }` is `pet____id___1`; the response key is `pet____id___v_id`.
- A variable whose value is an object (`{ q: { b: 1, a: 2 } }`, a test input of the wrong GraphQL type) stringifies with sorted keys.

### Conditions (`test/condition.test.ts`)

For `asBlogItem`'s condition: `"BlogItem"` holds; `"AdItem"`, `undefined`, `null`, `1`, and `"blogitem"` do not.

### `normalize` (`test/normalize.test.ts`)

Each case asserts the whole resulting store with `storeOf`.

- Leaves into the empty store: one record `Query.__ROOT` with the four `greeting` keys, `tags` and `settings`; `tags: []` is stored `[]`; `tags: ["a", null]` is stored as received; `settings: { entity: "Pet", id: "1" }` is stored as a leaf, and no `Pet` entity appears.
- A `null` leaf is stored `null`, and the key is present.
- `PetDetail`: `Query.__ROOT` holds `pet____id___1` linking `Pet:1`; `Pet:1` holds `id`, `name`, `nickname: null`, `tagline`, `stats`, `bestFriend`, `checkins____limit___2`, and `checkins____limit___2____skip___null` (variables `{ id: "1" }`); no record holds `__typename`.
- The same with `{ id: "1", skip: 4 }` writes `checkins____limit___2____skip___4`, a second key beside `checkins____limit___2`.
- Identity-less: `stats` links `PetStats` `Pet:1.stats`. `stats: null` writes `null` and creates no `PetStats` record.
- Cyclic data: `Pet:1.bestFriend` is `Pet:2`, whose `bestFriend` is `Pet:1`; `Pet:1` holds the fields of both depths, and both links.
- `PetLists`: `pets` is a list of links in response order; `pets: []` is `[]`; `maybePets: [null, { .. }]` is `[null, link]`; `petGrid: [[a], [], [null, b]]` is the same nesting of links; `petGrid: null` is `null`; `statGrid: [[x, y]]` creates `PetStats` `Query:__ROOT.statGrid.0.0` and `.0.1`.
- `Feed`, `limit: 2`, with a `BlogItem` and an `AdItem`: `BlogItem:b1` holds `id`, `title`, `author`, and no `advertiser`; the `AdItem` at index 1 is `Query:__ROOT.feed.1____limit___2` and holds `advertiser` only; a `__typename: "Other"` item creates an empty `Other` record (both conditions false).
- `Search` with `{ q: "Rex" }` links under the stable-JSON key.
- `EmptyStats`: `PetStats` `Pet:1.stats` exists with no fields, and `Pet:1.stats` links it.
- `PetCard` (`Node` fetch) with root `queryRoot` and `{ id: "4" }`: `Query.__ROOT.node____id___4` links `Pet:4`, which holds `id`, `name`, `tagline`.
- `SetTagline` with root `mutationRoot`: `Mutation.__ROOT` links `SetTaglinePayload` `Mutation:__ROOT.setTagline____input___{"id":"4","tagline":"Hi"}`, which links `Pet:4` with the new `tagline`.
- Into a non-empty store: `PetLists` then `PetDetail` for `Pet:1`: `Pet:1` holds the union of both; a field `PetDetail` does not select is kept; a field both write holds `PetDetail`'s value.
- The input store is unchanged: a deep copy taken before the call equals it after, for a `Normalized` and a `MalformedResponse` result.
- An empty response object `{}` for `Leaves`: per open decision 3.
- Malformed: `pet____id___v_id: "x"` is `CompositeValueNotObject` at `["pet____id___v_id"]`; `petGrid: [[3]]` at `["petGrid", 0, 0]`; a pet without `__typename` is `DiscriminatorNotString`; `id: 4` is `IdentityNotString`.

### `check` (`test/check.test.ts`)

Stores come from `normalize` of fixture responses, then are edited by hand where a case needs it.

- Empty store: `MissingData { record: queryRoot }`.
- For each fixture entrypoint, after `normalize` with the same variables: `EnoughData`.
- `PetDetail` with `{ id: "2" }` after `{ id: "1" }`: `MissingData { record: queryRoot }`.
- Leaf `null` (`nickname: null`): `EnoughData`. The `nickname` key deleted: `MissingData { record: Pet:1 }`.
- `Pet:2` deleted while `Pet:1.bestFriend` links it: `MissingData { record: Pet:2 }`.
- `stats: null`: `EnoughData`. `PetStats` `Pet:1.stats` deleted: `MissingData` for it.
- Lists: `maybePets: [null]` is `EnoughData`; `petGrid` with a link to a deleted record at `[1][0]` is `MissingData` for that record; `pets: []` is `EnoughData`.
- Conditionals: an `AdItem` record without `title` is `EnoughData`; `BlogItem:b1` without `title` is `MissingData { record: BlogItem:b1 }`.
- The first missing position in AST order is reported when two are missing.
- `EmptyStats`: the empty `PetStats` record is `EnoughData`; deleted, it is `MissingData` for it.
- `PetCard` after only `PetLists` wrote a complete `Pet:4`: `MissingData { record: queryRoot }` (open decision 1).
- A composite key holding `"x"` in a hand-built store: per open decision 4.
- The store is unchanged.

### `garbageCollect` (`test/garbageCollection.test.ts`)

- `retained = []`: `new Map()`. Empty store with any retained query: `new Map()`.
- One `PetDetail`: exactly `Query.__ROOT`, `Pet:1`, `Pet:2`, `PetStats` `Pet:1.stats`, and the checkins remain; a `Pet:9` written by `PetLists` is deleted, and so is the `PetLists`-only entity whose records were all unreached.
- `PetDetail` and `PetLists` both normalized, only `PetLists` retained: `Pet:1` stays whole, including fields only `PetDetail` selected; `Checkin` records go.
- Two roots: `PetDetail` from `queryRoot` and `SetTagline` from `mutationRoot` both retained: both roots and their records remain.
- Variables: `PetDetail` retained with `{ id: "2" }` when the store has only `pet____id___1`: everything but `Query.__ROOT` is deleted.
- Conditionals: `Feed` retained keeps `BlogItem:b1` and its `author`; an `AdItem` is kept (reached by the list), and the `asBlogItem` walk does not run on it.
- A link to an absent record and a retained root whose record is absent end the walk without error.
- Cyclic `bestFriend` data terminates.
- `PetCard` retained from `queryRoot` keeps `Pet:4` through `node____id___4`.
- A composite key holding `"x"`: per open decision 4.
- The input store is unchanged.

## Changes

Each change is one commit. Prerequisite: normalization-ast.md changes 1 to 3.

### 1. Test runner and fixture project

`vitest` and the `test` script; `target` and `include` in `tsconfig.json`; the CI step and the two pre-commit hooks; `fixtures/store` with its config, schema, sources, and generated artifacts; `test/fixtures.ts`.

### 2. Store, store keys, record ids, conditions

`src/store.ts`, `src/storeKey.ts`, `src/recordId.ts`, `src/condition.ts`; `test/storeKey.test.ts`, `test/condition.test.ts`. Depends on 1.

### 3. `normalize`

`src/normalize.ts`, its exports in `src/index.ts`, `test/normalize.test.ts`. Depends on 2.

### 4. `check`

`src/check.ts`, its export, `test/check.test.ts`. Depends on 3 (its tests build stores with `normalize`).

### 5. `garbageCollect`

`src/garbageCollection.ts`, its export, `test/garbageCollection.test.ts`. Depends on 3.

## Docs this doc amends

- `refactors/pending/graphql-protocol/normalization-ast.md`: `conditionHolds` moves to `src/condition.ts` with the signature above (open decision 7); `tsconfig.json` `target` is `ES2022`.
- `refactors/pending/runtime/pico-runtime.md`: the store is `Store` above, not upstream's layered store; `normalize`, `check`, and `garbageCollect` land here; its retained queries with a release buffer (intro, section 10, change 7) and its `log` option are dropped.

## Open decisions

### 1. The root of a `Node` fetch

The user's settlement says a `Node` fetch is rooted at `(entity, id)`. The approved normalization AST puts the `node(id: $id)` selection in the AST (normalization-ast.md, End-user experience: `selections: [{ responseKey: "node____id___v_id", selectableName: "node", .. }]`), so its root is the operation's root record, as upstream (`makeNetworkRequest.ts:233`, an entrypoint's `concreteType` is `"Query"`). The three functions take `root` either way; the options differ in the AST and in what the caller passes.

- (A) Root `(Query, "__ROOT")` for every query, `Node` fetches included. No AST change. Cost: `check` of `PetCard` for `Pet:4` is `MissingData { record: queryRoot }` whenever `node____id___4` was never written, even when another query wrote all of `Pet:4` (the `check` test above). Upstream answers this with `missingFieldHandler` (`read.ts:781-783`), which v1 does not have.
- (B) Root `(Pet, "4")`. The AST of a `Node` fetch starts below `node`: normalization-ast.md gains the fetch selection's response key (for instance `fetch: { responseKey: "node____id___v_id" } | null` beside `selections`), the caller passes `response[fetch.responseKey]` and `{ entity: "Pet", id: variables.id }` to `normalize`, `check`, and `garbageCollect`. Cost: an amendment to an approved doc, and a caller that knows the record's entity and that `id` is the variable holding its identity. `check` then answers from `Pet:4` however it was written.

### 2. Which response key is a record's identity

normalization-ast.md has the identity as an ordinary `Selection` node and lists "the identity in the AST" as Later, and says the runtime hardcodes no GraphQL name. `recordId` needs a key.

- (a) Read `id`, upstream (`cache.ts:623-626`). No AST change. Cost: a GraphQL name in the runtime, against normalization-ast.md; under `IdentityFieldSelection::NodeOnly` or `KeyDirective` a user selection of `id` keys a record that has another identity (query-generation.md, Config).
- (b) `identityResponseKey: string` on `NormalizationAst`, once per artifact like `discriminatorResponseKey`, from the GraphQL protocol's identity name. Cost: brings that Later item into v1 and amends normalization-ast.md; correct for `Auto`, still wrong for a per-entity identity.
- (c) The identity per composite selection and per conditional (`identityResponseKey: string | null`), from `IdentitySelection`. Correct for every identity rule. Cost: a field on every composite node, and the normalizer reads the conditionals that apply before it can key the record (a union's identity sits inside `... on BlogItem`).

### 3. A response key absent from a response object

A well-formed GraphQL response holds every selected key that is not inside a non-matching fragment, and conditionals already skip those.

- (a) Write `null`, upstream (`cache.ts:243-246`, `:270-273`, `== null`). Cost: the store records known `null` for data the server did not send, so `check` reports `EnoughData` for it.
- (b) `MalformedResponse` with a reason `MissingKey`. Cost: a response from a server that omits keys is rejected whole.
- (c) Leave the store key as it was. Cost: a later `check` reports the key missing and refetches it, every time.

### 4. A stored value under a composite key that is not a `CompositeValue`

`normalize` never writes one: one store key on one entity is one selectable, whose target is leaf or composite in every AST of one schema. A hand-built store, or a store kept across a schema change, can hold one.

- (a) `check` reports `MissingData { record: current }`, `garbageCollect` does not follow it. Total, no throw.
- (b) Throw, upstream (`check.ts:116-121`, `IsographEnvironment.ts:219-232`). A panic in AGENTS.md's sense; it needs the user's approval.
- (c) Split `StoreRecord` into a leaf map and a composite map, so the type carries the distinction and no runtime check exists. Cost: a third layer the user's settlement does not have, and a store key that one AST writes as a leaf and another as composite would live in both maps.

### 5. A nested object of the root entity

Upstream stores any response object whose type is `Query` at `ROOT_ID` (`cache.ts:614-618`), so a mutation payload's `query { pets }` updates the root record.

- (a) Drop it: such an object gets its identity or a derived id like any other. No GraphQL name in the runtime. Cost: data under a nested `Query` is not seen by reads from the root.
- (b) A response object whose entity equals `root.entity` is stored at `root.id`. No GraphQL name. Cost: correct only while `root` is an operation root (open decision 1 (A)); a nested `Mutation` never occurs, and under 1 (B) a `Pet` inside a `Pet` root would be stored at the root's id, which is wrong.

### 6. Store key encoding

Upstream's format (held above) makes distinct arguments share a key: `name: "null"`, `name: null`, and an unset `$name` are all `____name___null`, and a string value `1` and the number `1` are one chunk. An unset variable in GraphQL means "not provided", which applies the argument's default, not `null`.

- (a) Upstream verbatim. Cost: the collisions above; a store key written for `skip: null` is read for unset `$skip` where the server applied `skip = 0`.
- (b) Each chunk is `JSON.stringify(stableCopy(value))` (`____name___"null"` against `____name___null`), and an unset variable's argument is omitted from the key. Cost: a delta from upstream; `node(id: 1)` and `node(id: "1")` (one value after `ID` coercion) become two keys, which is a cache miss, not wrong data.

### 7. `conditionHolds`'s signature and the condition's `responseKey`

Held above: `conditionHolds(condition, discriminator)`; `normalize` reads `condition.responseKey`, check and collection pass the record's entity.

- (a) As held. Amends normalization-ast.md's signature only.
- (b) Also drop `responseKey` from `NormalizationCondition`, since it always equals `discriminatorResponseKey`, and `normalize` passes the entity it already read. Cost: amends the approved AST type.
- (c) Keep normalization-ast.md's `conditionHolds(condition, response, variables)` for `normalize`, and a second comparison against the entity for check and collection. Cost: two evaluators, against the user's one.

### 8. How the fixture artifacts are generated

The compiler writes artifacts only from the daemon today (`crates/isograph_cli/src/compile.rs`, `generate_artifacts`); there is no one-shot compile verb.

- (a) Committed, regenerated by running the daemon on `libs/isograph-runtime/fixtures/store/isograph.config.json`, as the pet demo's are (`scripts/restart-daemon-if-running.sh <config>`). No new machinery. Cost: nothing detects stale artifacts after a compiler change.
- (b) (a), plus a cargo test in `crates/isograph_cli` that compiles the fixture project in-process and asserts every committed artifact equals its regenerated text. Cost: a whole-file equality is snapshot-shaped (AGENTS.md, Tests), even though its purpose is freshness.
- (c) An `isograph compile --config <path>` verb that compiles once, writes, and exits; the runtime's `test` script runs it first and the artifacts are gitignored. Cost: a new CLI verb no doc has designed, and the JS tests and their pre-commit hook need a built Rust binary.
