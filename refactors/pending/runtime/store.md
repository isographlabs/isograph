# Runtime: the v1 store, normalizer, data checker, and garbage collector

Status: under discussion. Not approved for implementation.

This is the first code in `@isograph/runtime` (`libs/isograph-runtime`): a store, `normalize`, `check`, and `garbageCollect`. Each is a function of the store and the normalization AST artifact. The package has no knowledge of React and no React dependency. The store is the only state: nothing is hidden in an environment, a module-level variable, or a closure. `normalize` writes into the store it is passed and `garbageCollect` deletes from it, in place, as upstream does (`normalizeData` writes through `getMutableStoreRecordProxy`, `cache.ts:93`, `:423`; `garbageCollectBaseStoreLayer` deletes with `delete`, `garbageCollection.ts:105`, `:111`, `:116`). `check` only reads.

It plans against `refactors/pending/graphql-protocol/normalization-ast.md` (approved for implementation, including its record amendment: `root`, `NormalizationCompositeTarget` with `entity` and `identity`, `NormalizationIdentity` with `Fixed`) and `query-generation.md`. Prerequisite: normalization-ast.md changes 1 to 3 (the package, the AST, the TypeScript artifact), so `normalization_ast.ts` artifacts are real.

`refactors/pending/runtime/pico-runtime.md` stays parked; v1 has no memoization, no reader, no subscriptions, and no network layer. `refactors/pending/runtime/fetch-policies.md` lists directions beyond `check`.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src/core`.

## End-user experience

v1's only callers are the package's tests. A test normalizes a response, asserts on the store, checks, and collects:

```ts
// from libs/isograph-runtime/test/normalize.test.ts
import { expect, test } from "vitest";
import petDetail from "../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast";
import { check, garbageCollect, normalize, type Store } from "../src/index";
import { queryRoot, storeOf } from "./fixtures";

test("a pet with an identity-less stats object", () => {
  const store: Store = new Map();
  normalize(
    store,
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
    { id: "1", skip: null },
  );
  expect(store).toEqual(
    storeOf({
      Query: { __ROOT: { 'pet____id___"1"': { entityName: "Pet", id: "1" } } },
      Pet: { "1": { id: "1", name: "Rex", nickname: null, stats: { entityName: "PetStats", id: "Pet:1.stats" } /* ... */ } },
      PetStats: { "Pet:1.stats": { weight: 7, energy: null } },
    }),
  );
  expect(check(store, petDetail, { id: "1", skip: null })).toEqual({ kind: "EnoughData" });
  expect(check(store, petDetail, { id: "2", skip: null })).toEqual({ kind: "MissingData", record: queryRoot });
  garbageCollect(store, []);
  expect(store).toEqual(new Map());
});
```

What this shows:

- The response is read by each node's response key, `responseKey ?? selectableName` (`pet____id___v_id` for `pet(id: $id)`, `name` for `name`, which the query prints with no alias); the store is written by the store key, with variable values substituted (`pet____id___"1"`).
- The top level is written onto `petDetail.root`, `{ entityName: "Query", id: "__ROOT" }`; the caller passes no root.
- `Pet:1`'s entity is its target's `entity` (`Known`, `Pet`), and its id the value under its target's `identity` (`Select`, `id`); `PetStats`'s target has `identity: { kind: "Omit" }` and gets upstream's derived id `Pet:1.stats`.
- `null` is stored as `null` and is known data. `__typename` is not stored: the record's entity is its first-layer key.
- `check` with other variables reports the root record, which lacks `pet____id___"2"`.
- `normalize` and `garbageCollect` return nothing; the test inspects the same `store` object afterwards. Collecting with nothing retained empties it.

## The store

```ts
// from libs/isograph-runtime/src/store.ts
import type { EntityName } from "./artifacts";

/** Every record: `EntityName`, then record id, then the record. */
export type Store = Map<EntityName, Map<RecordId, StoreRecord>>;

/** One record's values by store key. An absent key is missing data; `null` is known data. */
export type StoreRecord = Map<StoreKey, StoreValue>;

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
  readonly entityName: EntityName;
  readonly id: RecordId;
};

export type JsonValue = null | boolean | number | string | ReadonlyArray<JsonValue> | JsonObject;
export type JsonObject = { readonly [key: string]: JsonValue };

/** Operation variables by name. Every variable of the operation is present (Variables). */
export type Variables = { readonly [name: string]: JsonValue };
```

The type is the two-layer map and nothing else. The empty store is `new Map()`. There is no root record until a function writes one, and no record is ever `null`.

`Map` at all three layers, mutable because `normalize` and `garbageCollect` write in place; `check` takes the same `Store` and does not write. Maps, not plain objects, because ids and store keys are arbitrary strings: a record id `"constructor"` or a selectable named `constructor` (a valid GraphQL name) would read `Object.prototype.constructor` from a plain object. For the same reason every read of a response object or of `variables` goes through `Object.hasOwn(object, key) ? object[key] : undefined` (ES2022).

A link carries no tag. A `LeafValue` can have the same shape as a `StoreLink` (a `JSON` scalar `{ entityName: "Pet", id: "1" }`); no function confuses them, because each reads a store key only under an AST selection whose `target` is `null` (leaf) or an object (composite). Upstream's links are `{ __link, __typename }` (`IsographEnvironment.ts:122-125`), and its walks also go through the AST.

A link is `{ entityName, id }`, not upstream's `{ __link, __typename }`. Reason: the names are the mental model's (`docs-website/docs/design-docs/mental-model.md`: an entity, and a record's id), and `__typename` is GraphQL's name, which the protocol-neutral runtime does not use. The generated link output types change to the same shape with the store implementation (change 6): a client pointer's resolver returns a link, and the type artifact `E/__link/output_type.ts` is the type of that link, so it must be the shape `read` follows.

Before (`demos/pet-demo/src/__isograph/Pet/__link/output_type.ts`, generated by `link_type_file` in `crates/typescript_artifacts/src/generate_types.rs`):

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
export type Pet____link__output_type = {
  readonly __link: string;
  readonly __typename: "Pet";
};
```

After:

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
export type Pet____link__output_type = {
  readonly entityName: "Pet";
  readonly id: string;
};
```

An abstract entity's link type stays the union over its concrete entities, each member `{ readonly entityName: "<Concrete>"; readonly id: string }`; an entity with none stays `never`. `link_type_file` changes only the two `required(..)` entries of each object: `required("entityName", TypeScriptType::StringLiteral(concrete.to_string()))` then `required("id", TypeScriptType::Keyword(TypeScriptKeyword::String))`, in the field order of `StoreLink`.

Root records: an operation's root record is `ast.root`, `{ entityName: <root operation type>, id: "__ROOT" }`, which the compiler writes (normalization-ast.md, The record). A nested selection of any root operation type has `identity: { kind: "Fixed", id: "__ROOT" }` and is written onto the same record. The runtime defines no root id and no root entity name: both come from the AST (upstream hard-codes `ROOT_ID`, `IsographEnvironment.ts:163`).

Deltas from upstream (`IsographEnvironment.ts:122-169`, `:211-217`):

- Maps, not objects, written in place as upstream's objects are; no `null` record (upstream `StoreLayerData` allows `StoreRecord | null`, `:165-169`).
- No pre-created `Query.__ROOT` (`createIsographStore`, `:211-217`).
- No store layers, optimistic or otherwise (`getStoreRecordProxy` and `getMutableStoreRecordProxy`, `optimisticProxy.ts:24`, `:46`); the store is the base layer's data.
- `__typename` and `id` are not special fields of a record type (`StoreRecord`'s `__typename?`, `id?`, `:150-157`). `id` is stored because it is an AST selection; the discriminator is not stored.

## Store keys

```ts
// from libs/isograph-runtime/src/storeKey.ts
/** `selection.selectableName`, then `argumentChunks(selection.arguments, variables)`. */
export function storeKey(selection: NormalizationSelection, variables: Variables): StoreKey;

/** One `____<name>___<value>` chunk per argument, in the AST's order (sorted by name), with
 * `<value>` the argument's value as `stableStringify` writes it. */
export function argumentChunks(args: Arguments | null, variables: Variables): string;

/** `JSON.stringify(value)` with every object's keys in code unit order, at every depth. */
export function stableStringify(value: JsonValue): string;
```

The value of an argument, before `stableStringify`:

- `Literal`: its value (`5`, `true`, `null`).
- `String`: the string.
- `Variable`: `variables[name]`. A top-level argument whose variable is absent from `variables` is omitted from the key, chunk and all.
- `Object`: an object with one key per entry, each value by these rules; an entry whose variable is absent is `null`.

So `limit: 5` is `____limit___5`, `name: "Ada Lovelace"` is `____name___"Ada Lovelace"`, `name: null` is `____name___null`, `name: "null"` is `____name___"null"`, `id: 1` is `____id___1` and `id: "1"` is `____id___"1"`, and `filter: { nameContains: $q, minAge: 2 }` with `q: "Rex"` is `____filter___{"minAge":2,"nameContains":"Rex"}`.

`stableStringify` is `JSON.stringify(value, (_key, v) => isJsonObject(v) ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : v)`: the replacer returns each object with its keys sorted, and `JSON.stringify` recurses into what the replacer returns, so nested objects are sorted at every depth; arrays keep their order; a key whose value is `undefined` is omitted, as `JSON.stringify` omits it; `isJsonObject` is `typeof v === "object" && v !== null && !Array.isArray(v)`. The sort is code unit order, not `localeCompare`, so the key does not depend on the locale. The sort runs at runtime because a variable whose value is an object comes from the caller in any key order; literal object arguments are already sorted by the compiler (`MergedObject`), and one function covers both.

Every operation variable is present in `variables`: the generated parameters type of an entrypoint requires each one (Variables, below), and a missing one is undefined behavior, as any input outside its type is. The absent-variable rules above apply only to `read` of a client field whose parent did not pass a variable that has no default: query generation omits that top-level argument from the query and the normalization AST and writes `null` inside an object (query-generation.md, Source fields and variables), and the reader's key must match (`checkins`, not `checkins____limit___null`; reader-ast.md, Arguments and variable defaults).

`noPropertyAccessFromIndexSignature` and the `Object.hasOwn` read apply to `variables`.

Deltas from upstream (`getStoreKeyChunkForArgument` and `getStoreKeyChunkForArgumentValue`, `cache.ts:481-520`; `stableCopy`, `util.ts:44-59`):

- Each value is JSON, so `null` and `"null"`, and `1` and `"1"`, are distinct keys; upstream writes `${value}` for strings, numbers, and booleans, which makes each pair one chunk. A cost: `node(id: 1)` and `node(id: "1")`, one value after `ID` coercion, are two keys, which is a cache miss, not wrong data.
- The value is written by the standard serializer (`JSON.stringify`), with no hand-written escaping; upstream hand-rolls the chunk per kind and stringifies only objects and arrays.
- A top-level argument whose variable is unset is omitted; upstream writes `null` for it (`cache.ts:481-520`). An unset variable applies the schema default for the argument (GraphQL spec, CoerceArgumentValues), and `null` overrides it, so the two are different requests; query generation already omits the argument (query-generation.md, open decision 2, held), and the reader's key must match the normalizer's.
- Upstream's overloads for `LinkedParentRecordKey` and `ScalarParentRecordKey` (`cache.ts:449-469`) are one `StoreKey`, because a link and a leaf are told apart by the AST. `Enum` has no case (normalization-ast.md: `ArgumentValue` has no `Enum`).

### Variables

Every variable of an entrypoint is required in its generated `parameters_type.ts`: no `?` and no `void`. A nullable variable is `readonly limit: number | null`, and the caller passes it, `null` included. A variable with a declared default is required too: the caller passes the value it wants; the default matters only inside the compiler, for the arguments a parent field's selection leaves unpassed. Reason: an unprovided variable makes its argument behave as omitted, which applies the schema default, and is distinct from `null` (GraphQL spec, CoerceVariableValues and CoerceArgumentValues); requiring every variable removes the choice between `{}` and `{ limit: null }` from the API, and the runtime never sees an unset operation variable.

Before (`demos/pet-demo/src/__isograph/Pet/PetCheckinsCard/parameters_type.ts`, from `parameter_property` in `crates/typescript_artifacts/src/generate_types.rs`):

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
export type Pet__PetCheckinsCard__parameters = {
  readonly limit?: number | null | void;
};
```

After:

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
export type Pet__PetCheckinsCard__parameters = {
  readonly limit: number | null;
};
```

`parameter_property`'s nullable arm becomes `(Optionality::Required, wrapped_type(wrapper, &mut inner))`, the same as the other arm, so `Optionality::Optional` and the `void` keyword have no other reader in that function and the match collapses to the one arm. Delta from upstream: upstream's parameters types make a nullable variable optional (`readonly skip?: number | null | void`, `isograph/demos/pet-demo/src/components/__isograph/Pet/PetCheckinsCard/parameters_type.ts`). The type artifacts change lands with the store (change 7). artifact-set-and-types.md's `readonly w?: T | null | void` is amended (Docs this doc amends).

`parameters_type.ts` is one file per field, and a field's parameters type is also the type of its resolver's `parameters` when another field selects it (reader-ast.md, `childVariables`). There, a variable with no default that the parent does not pass is absent at runtime, so `readonly n: number | null` does not describe it. Open decision 11.

## Record ids

```ts
// from libs/isograph-runtime/src/recordId.ts
/** Where a composite response object sits: what `recordLink` reads besides the object. */
export type RecordPosition = {
  /** The record the object is linked from. */
  readonly parent: StoreLink;
  readonly selection: NormalizationSelection;
  /** One index per list level, from the outermost; `[]` outside a list. */
  readonly listIndexes: ReadonlyArray<number>;
  /** Response keys and list indexes from the top of the response, for error messages. */
  readonly path: ReadonlyArray<string | number>;
};

/** The record of the composite response object `response`, a value of `position.selection`,
 * whose target is `target`. Throws on a missing discriminator or a non-string identity value. */
export function recordLink(
  response: JsonObject,
  target: NormalizationCompositeTarget,
  discriminatorResponseKey: string,
  position: RecordPosition,
  variables: Variables,
): StoreLink;
```

The entity, from `target.entity`:

- `Known`: `target.entity.entityName`. The discriminator is not read and not compared.
- `Discriminator`: the value under `discriminatorResponseKey`, which must be a string (else it throws, Errors).

The identity: the `identity` of the `Conditional` among `target.selections` whose `condition.entityName` equals that entity, when one does and its identity is not `Omit`; otherwise `target.identity`. The conditional's comes first because it is the identity of the response object's concrete entity; a union target's is `Omit` and its `asBlogItem` conditional's is `Select("id")`. At most one conditional of a selection set matches: the conditions name distinct entities (the map keys conditionals by `Condition`). Only direct children are considered, because no condition nests under the current grammar (normalization-ast.md, Merging). The id, by the identity's `kind`:

- `Fixed`: `identity.id`. The record is a root entity's one record (Nested objects of a root entity).
- `Select`: the value under `identity.selectableName` (the identity selection has no arguments, so the query prints it with no alias), which must be a string (GraphQL serializes `ID` as a string, spec 3.5.5); any other value throws (Errors). An absent key throws as every absent selected key does.
- `Omit`: the derived id, upstream's (`getDataIdOfNetworkResponse`, `cache.ts:599-642`): `${parent.entityName}:${parent.id}.${selection.selectableName}`, then `.${index}` for each list level from the outermost, then `argumentChunks(selection.arguments, variables)`. Examples: `Pet:1.stats`, `Query:__ROOT.feed.1____limit___2`, `Query:__ROOT.statGrid.0.1`.

Each switch on a `kind` is exhaustive, with no `default` (Errors: artifacts).

Deltas from upstream (`getDataIdOfNetworkResponse`, `cache.ts:599-642`):

- The entity comes from `target.entity`, upstream's `astNode.concreteType ?? dataToNormalize.__typename` (`cache.ts:606-607`), as a tagged value instead of `string | null`.
- The identity's response key comes from the AST, per composite target and conditional; upstream reads `dataToNormalize.id` (`cache.ts:623-626`). The runtime names no GraphQL selectable.
- One `.${index}` per list level, because upstream's normalizer handles one list level only (`cache.ts:275-309`, and `getDataIdOfNetworkResponse` takes one `index`).
- The root rule is `Fixed`, decided by the compiler; upstream compares `__typename === 'Query'` and returns `ROOT_ID` (`cache.ts:614-618`).

### Nested objects of a root entity

A response object whose target or conditional identity is `Fixed` is that root entity's one record: its selections are written onto it, merged with what is there. This is safe because every selection is auto-aliased and selections auto-merge (query-generation.md, Normalization aliases, Merging): one response key in one selection set is one selection key, and one store key on one record is one selectable with one set of argument values. A store key written from the nested object and from the top level therefore holds the same fact about the same record, and the later write holds.

The rule covers every root operation type, not only the operation's own: a mutation payload's nested `Query` object is written onto `Query.__ROOT`, as upstream's rule does. The compiler decides it from the schema's root operation types, so `schema { query: RootQuery }` makes `RootQuery` the root entity, and the runtime names no GraphQL type.

## Conditions

A `Conditional`'s `condition` is `{ kind: "SelectionEquals", entityName }` (normalization-ast.md, The condition). Every walk compares `condition.entityName` with the entity of the record it is on, inline, with `===`: `normalize` with the entity of the record it is writing (the target's `Known` entity, or the response's discriminator value under `Discriminator`; at the top level, `ast.root.entityName`), and `check` and `garbageCollect` with the `entity` of the link they are at. The two agree for every record `normalize` wrote, because the link's entity is the entity `normalize` wrote it under. There is no `conditionHolds` function and no `src/condition.ts`. Upstream inlines the same comparison at each site (`__typename === type`, `cache.ts:356`, `check.ts:193`, `garbageCollection.ts:159`).

A switch on `condition.kind` would have one case; the comparison reads `condition.entityName` directly, because `kind` has one value in v1 and TypeScript narrows nothing by it. `VariableEquals` (normalization-ast.md, Later) adds the switch.

## `normalize`

```ts
// from libs/isograph-runtime/src/normalize.ts
/** Writes `response` into `store` from `ast.root` down, by `ast` and `variables`. Throws on a
 * response that does not match `ast` (Errors). */
export function normalize(
  store: Store,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void;
```

`response` is the operation's `data` object, not the GraphQL envelope; v1 has no network layer. A GraphQL response with `errors` and partial data is not malformed: null propagation (spec 6.4.4) writes `null` at a key that is present, and `normalize` stores it as known `null`. A response whose `data` is `null` has no object to normalize; the caller handles it before calling `normalize`.

It returns nothing: no v1 caller reads what it wrote (no subscriptions, no `EncounteredIds`), and tests inspect `store`.

A selection's response key is `selection.responseKey ?? selection.selectableName`: the alias the query prints for a selection with arguments, and the selectable name for one without (normalization-ast.md, End-user experience). Below, `response[responseKey]` means the value under that key.

Behavior, walking `ast.selections` against `response` onto the record `ast.root` (`{ entityName, id }`; the top level selects no discriminator, query-generation.md):

- The record at a link is the store's record if present, else a new empty record inserted into `store`. Fields the AST does not select are kept. A store key written twice in one call (two paths to `Pet:1`) holds the later write, in AST and list order.
- `Selection` with `target: null` (leaf): writes `response[responseKey]` under `storeKey(selection, variables)`, as received: a string, number, boolean, `null`, a list of leaves at any depth, or a `JSON` object.
- `Selection` with a `target` (composite), by the response value:
  - `null`: writes `null`.
  - An object: its record is `recordLink(value, selection.target, ast.discriminatorResponseKey, position, variables)` (Record ids). The record is written by walking `selection.target.selections` against `value`, and the parent gets the link `{ entityName, id }`. `selections` may be `[]` (normalization-ast.md: `stats { X @loadable }`); the record is still created, empty.
  - A list: the value is the list with each item replaced, recursively: `null` stays `null`, an object becomes a link as above with its index path, a nested list recurses. `[]` writes `[]`.
  - A string, number, or boolean, here or as a list item: throws.
- `Conditional`: when `condition.entityName === record.entityName`, where `record` is the link of the record the walk is writing, walks `selections` against the same response object onto the same record; otherwise nothing.
- A selected response key absent from the response object, outside a conditional that does not hold: throws. A well-formed response holds every such key, and conditionals already skip the keys of a fragment that does not apply.
- The discriminator is never written to a record.

A throw part-way through leaves the writes made before it (open decision 9). The walk writes each value into `store` as it reaches it; there is no draft and no intermediate list.

Deltas from upstream (`cache.ts:77-437`):

- Writes the one store in place, as upstream writes a store layer through `getMutableStoreRecordProxy` (`cache.ts:93`, `:423`); there are no layers.
- No `EncounteredIds` and no change detection (`RecordHasBeenUpdated`, `cache.ts:151`, `:211-219`, `:231-250`, `:309`, `:337-338`, `dataIdsAreTheSame` `:372-393`): nothing reads them without subscriptions.
- Reads `responseKey ?? selectableName` instead of `getNetworkResponseKey` (`cache.ts:538-591`), which computes the same key, and `target.entity` (`Known`, or the value under `ast.discriminatorResponseKey`) where upstream reads `astNode.concreteType ?? response.__typename` (`cache.ts:295-296`, `:322-323`, `:412-413`).
- Starts at `ast.root`; upstream's caller passes the root record.
- Upstream throws only on a missing `__typename` (`cache.ts:297-302`, `:325-330`, `:415-420`) and writes anything else it meets, including `null` for an absent key (`cache.ts:243-246`, `:270-273`, `== null`). Here every mismatch between the response and the AST throws (Errors), an absent key included.
- Lists of lists and lists of leaves at any depth.

## `check`

```ts
// from libs/isograph-runtime/src/check.ts
/** Whether `store` holds every value `ast` selects from `ast.root` with `variables`. */
export function check(store: Store, ast: NormalizationAst, variables: Variables): CheckResult;

export type CheckResult =
  | { readonly kind: "EnoughData" }
  /** `record` lacks a selected value, or is linked to and absent. */
  | { readonly kind: "MissingData"; readonly record: StoreLink };
```

Upstream's semantics (`check.ts:43-215`): `null` is known data, only an absent key or an absent record is missing (`check.ts:88-89`). The walk returns the first missing position in AST order, then list order:

- `ast.root` absent from the store: `MissingData { record: ast.root }`.
- Leaf selection: absent key is `MissingData { record: current }`; any value, `null` included, is enough.
- Composite selection: absent key is `MissingData { record: current }`; `null` is enough; a link whose record is absent is `MissingData { record: link }`; a present record recurses with `selection.target.selections`, even when they are `[]`; a list recurses into every item and nested list, `null` items being enough. A value that is not `null`, a link, or a list of these throws (Errors).
- `Conditional`: when `condition.entityName === current.entityName`, recurses on the same record; otherwise enough.

v1 has no `shouldFetch` decision function. The later network layer decides upstream's `'Yes' | 'No' | 'IfNecessary'` (default `IfNecessary`, `check.ts:13-16`; `makeNetworkRequest.ts:67-118`) by calling `check`.

Deltas from upstream:

- An inline fragment never reports `MissingData` for a record without `__typename` (`check.ts:184-191`): the record's entity is always known.
- No `null` records (`check.ts:130-131`, `:163-164`). A non-link throws, as upstream's does (`check.ts:116-121`, `:149-154`), with this doc's message.
- Lists of lists.

## `garbageCollect`

```ts
// from libs/isograph-runtime/src/garbageCollection.ts
/** What the caller keeps: an AST and its variables. The walk starts at `ast.root`. */
export type RetainedQuery = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
};

/** Deletes from `store` every record no query in `retained` reaches. */
export function garbageCollect(store: Store, retained: ReadonlyArray<RetainedQuery>): void;
```

Mark, then sweep (upstream `garbageCollectBaseStoreLayer`, `garbageCollection.ts:88-119`, and `recordReachableIds`, `:125-226`):

- Each retained query marks `ast.root` when that record exists, and walks its AST from it.
- A leaf selection marks nothing. A composite selection follows each link in its value (every list level), and marks and walks each linked record that exists. An absent key, `null`, or an absent record stops that path. A value that is not `null`, a link, or a list of these throws (Errors).
- `Conditional`: when `condition.entityName === current.entityName`, walks the same record.
- The sweep deletes every unmarked record, then every entity map left empty. Marked records stay whole, including fields no retained AST selects. `retained = []` empties the store.
- Cyclic data terminates: the walk recurses on the AST, which is finite. A record reached by several paths is walked once per path.

The mark is an iterator, `function* reachableRecords(store, retained): Generator<StoreLink>`, consumed into a `Map<EntityName, Set<RecordId>>` before the sweep starts, so the sweep's deletes never run during the walk. The sweep deletes from `store` with `Map.prototype.delete`, as upstream's sweep does with `delete` (`garbageCollection.ts:101-118`).

It returns nothing. No v1 caller reads what was deleted or how many records were; tests inspect `store`.

The caller decides what is retained and when collection runs; there is no retain count, no release buffer, and no trigger.

Deltas from upstream:

- No retained set on an environment, no `gcBuffer` of recently released queries or its size, and no trigger on release (`garbageCollection.ts:39-63`, `:78-83`; `makeNetworkRequest.ts:565-577`; `IsographEnvironment.ts:177`, `:204-206`).
- No skip while optimistic layers exist (`garbageCollection.ts:66-68`) or while a lazily loaded AST is pending (`:70-76`): neither exists.
- Upstream marks a root id even when its record is absent (`garbageCollection.ts:133-136`); here an absent record has nothing to keep.
- Links carry their entity, so there is no `concreteType` store lookup (`garbageCollection.ts:188-204`).

## Errors

The runtime throws when something we generate is malformed, and when input breaks a contract that only a bug violates. A throw is `throw new Error(message)`; no error class, because nothing catches by type. There is no typed error result in v1: `check`'s `MissingData` is an answer, not an error.

- Artifacts (the normalization AST, later the reader AST): the compiler generated them, and the `NormalizationAst` annotation type-checks each one. A switch on `kind` is exhaustive over the union, with no `default` and no throw.
- Store states only our normalizer produces: one store key on one entity is one selectable, whose target is leaf or composite in every AST of one schema, so `normalize` never writes a non-link under a composite store key. A store that holds one was written with an AST that disagrees.
- The network response: the server answered the query text the compiler generated with this AST. A response that does not match the AST means the server broke that query's contract (schema skew between the compiler and the server, a broken server or proxy, or a wrong test fixture). A correct server never produces one.
- Caller-supplied variables: no check. Variables arrive typed by the generated parameters type, which requires each one, so a missing variable is undefined behavior, as any input outside its type is.

This replaces, for the runtime, AGENTS.md's rule that socket reads return a typed error: the user's decision is that a response the generated query's contract excludes is a broken invariant, like a malformed artifact.

Every throw site:

- `normalize`, a selected response key absent from a response object (outside a conditional that does not hold): `Malformed response at ${path}: expected the response key "${responseKey}".` Invariant: a response to the generated query holds every selected key outside a non-matching fragment (GraphQL spec 6.3.2, every field in the collected fields gets a response entry).
- `normalize`, a composite selection's value, or a list item at any depth, that is a string, number, or boolean: `Malformed response at ${path}: expected an object, a list, or null for "${responseKey}", got ${JSON.stringify(value)}.` Invariant: a composite field's value is an object, a list, or `null`.
- `recordLink`, a composite response object whose target's `entity` is `Discriminator`, with no string under `ast.discriminatorResponseKey`: `Malformed response at ${path}: expected a string under "${discriminatorResponseKey}".` Invariant: the compiler selects the discriminator in every selection set under a composite selection (query-generation.md, Completion, step 4). Under a `Known` entity the discriminator is not read, so its absence does not throw.
- `recordLink`, a `Select` identity value that is not a string: `Malformed response at ${path}: expected a string record id under "${selectableName}", got ${JSON.stringify(value)}.` Invariant: GraphQL serializes `ID` as a string (spec 3.5.5), and the protocol names only an `ID!` selectable as the identity (`IdField::Present`, query-generation.md).
- `check` and `garbageCollect`, a value under a composite store key that is not `null`, a link, or a list of these at any depth: `Store record ${entityName}:${id} holds a non-link under the composite store key "${storeKey}"; the store was written with an AST that disagrees with this one.` Invariant: Store states only our normalizer produces, above. A link is an object whose own keys are exactly `entityName` and `id`, both strings.

`${path}` is `JSON.stringify(path)`, the response keys and list indexes from the top of the response (`["pet____id___v_id","checkins____limit___2",1]`).

Upstream throws in the same three kinds of places: a missing `__typename` (`cache.ts:297-302`), a non-link in the store (`check.ts:116-121`), with messages ending "This is indicative of a bug in Isograph."; it does not check the rest.

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
  src/normalize.ts
  src/check.ts
  src/garbageCollection.ts
  test/fixtures.ts
  test/storeKey.test.ts
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
  Condition,
  EntityName,
  NormalizationAst,
  NormalizationCompositeTarget,
  NormalizationConditional,
  NormalizationEntity,
  NormalizationIdentity,
  NormalizationNode,
  NormalizationRootRecord,
  NormalizationSelection,
  NormalizationSelectionSet,
} from "./artifacts";
export type {
  CompositeValue,
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
export { normalize } from "./normalize";
```

`storeKey`, `argumentChunks`, `stableStringify`, and `recordLink` are module exports that `normalize`, `check`, and `garbageCollect` import; the package index does not export them. Their tests import the modules directly.

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

Tests build responses, variables, and stores by hand; those are inputs. Each test creates its own `new Map()` store, because `normalize` and `garbageCollect` write the store they are passed. Tests assert on the same store object's contents after `normalize` and after `garbageCollect`, on `CheckResult`, and on the message of what throws; the store's shape is observable behavior.

```ts
// from libs/isograph-runtime/test/fixtures.ts
/** The root records the fixture's ASTs name, for asserting on results. */
export const queryRoot: StoreLink = { entityName: "Query", id: "__ROOT" };
export const mutationRoot: StoreLink = { entityName: "Mutation", id: "__ROOT" };

/** A new store whose entities, ids, store keys, and values are `records`' keys and values, in order. */
export function storeOf(records: {
  readonly [entityName: string]: { readonly [id: string]: { readonly [storeKey: string]: StoreValue } };
}): Store;
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
  query: Query!
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
  query: Query!
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

field Mutation.SetTagline($id: ID!, $tagline: String!) {
  setTagline(input: { id: $id, tagline: $tagline }) { pet { id tagline } query { greeting } }
}

field Query.NestedQuery {
  greeting
  query { tags }
}
```

with `entrypoint Query.Leaves`, `Query.NestedQuery`, `Query.PetDetail`, `Query.EmptyStats`, `Query.PetLists`, `Query.Feed`, `Query.Search`, and `Mutation.SetTagline`. If the compiler rejects one (for instance `@loadable` on an identity-less `PetStats` field), the implementer stops and raises it.

## Tests

### Store key (`test/storeKey.test.ts`)

Selections are taken from the fixture ASTs by response key.

- `greeting` with no arguments is `greeting`.
- `literalGreeting` is `greeting____loud___true____name___"Ada Lovelace"____times___3`: chunks in the AST's (sorted) order, the string JSON-quoted with its space, the boolean and number as JSON.
- `nullGreeting` is `greeting____name___null`.
- `variableGreeting` with `{ name: "Bo" }` is `greeting____name___"Bo"`; with `{ name: null }` it is `greeting____name___null`, equal to `nullGreeting`'s; with `{ name: "null" }` it is `greeting____name___"null"`, a different key.
- `search` with `{ q: "Rex" }` is `search____filter___{"minAge":2,"nameContains":"Rex"}`, keys sorted.
- `pet` with `{ id: "1" }` is `pet____id___"1"`; with `{ id: 1 }` (a test input of the wrong GraphQL type) it is `pet____id___1`, a different key; the response key is `pet____id___v_id`.
- `stableStringify` of `{ b: 1, a: { d: [3, { f: 1, e: 2 }], c: 2 } }` and of `{ a: { c: 2, d: [3, { e: 2, f: 1 }] }, b: 1 }` are both `{"a":{"c":2,"d":[3,{"e":2,"f":1}]},"b":1}`: key order is independent at every depth, and array order is kept (`[2, 1]` stays `[2,1]`).
- `stableStringify` of `{ a: undefined, b: 1 }`, a value outside `JsonValue` built by a cast, is `{"b":1}`, as `JSON.stringify` writes it.
- A top-level `Variable` argument whose variable is absent is omitted: `checkins(limit: $n)` with `{}` is `checkins`; inside an object it is `null`: `search(filter: { nameContains: $q, minAge: 2 })` with `{}` is `search____filter___{"minAge":2,"nameContains":null}`.

### `normalize` (`test/normalize.test.ts`)

Each case asserts the whole resulting store with `storeOf`.

- Leaves into the empty store: one record `Query.__ROOT` with the four `greeting` keys, `tags` and `settings`; `tags: []` is stored `[]`; `tags: ["a", null]` is stored as received; `settings: { entityName: "Pet", id: "1" }` is stored as a leaf, and no `Pet` entity appears.
- A `null` leaf is stored `null`, and the key is present.
- `PetDetail`: `Query.__ROOT` holds `pet____id___"1"` linking `Pet:1`; `Pet:1` holds `id`, `name`, `nickname: null`, `tagline`, `stats`, `bestFriend`, `checkins____limit___2`, and `checkins____limit___2____skip___null` (variables `{ id: "1", skip: null }`); no record holds `__typename`.
- The same with `{ id: "1", skip: 4 }` writes `checkins____limit___2____skip___4`, a second key beside `checkins____limit___2`.
- Identity-less: `stats` links `PetStats` `Pet:1.stats`. `stats: null` writes `null` and creates no `PetStats` record.
- Cyclic data: `Pet:1.bestFriend` is `Pet:2`, whose `bestFriend` is `Pet:1`; `Pet:1` holds the fields of both depths, and both links.
- `PetLists`: `pets` is a list of links in response order; `pets: []` is `[]`; `maybePets: [null, { .. }]` is `[null, link]`; `petGrid: [[a], [], [null, b]]` is the same nesting of links; `petGrid: null` is `null`; `statGrid: [[x, y]]` creates `PetStats` `Query:__ROOT.statGrid.0.0` and `.0.1`.
- `Feed`, `limit: 2`, with a `BlogItem` and an `AdItem`: `BlogItem:b1` holds `id`, `title`, `author`, and no `advertiser`; the `AdItem` at index 1 is `Query:__ROOT.feed.1____limit___2` and holds `advertiser` only; a `__typename: "Other"` item creates an empty `Other` record (both conditions false).
- `Search` with `{ q: "Rex" }` links under the stable-JSON key.
- `EmptyStats`: `PetStats` `Pet:1.stats` exists with no fields, and `Pet:1.stats` links it.
- `SetTagline`: `Mutation.__ROOT` links `SetTaglinePayload` `Mutation:__ROOT.setTagline____input___{"id":"4","tagline":"Hi"}`, which links `Pet:4` with the new `tagline`.
- Into a non-empty store: `PetLists` then `PetDetail` for `Pet:1`: `Pet:1` holds the union of both; a field `PetDetail` does not select is kept; a field both write holds `PetDetail`'s value.
- Nested root entity: `NestedQuery` with `query: { __typename: "Query", tags: ["a"] }` writes `greeting` and `tags` onto `Query.__ROOT`, which links itself under `query` (`{ entityName: "Query", id: "__ROOT" }`), and creates no other `Query` record. The same with `query: { tags: ["a"] }` (no `__typename`) writes the same store: the `Known` entity reads no discriminator. `SetTagline` writes its nested `Query` object onto `Query.__ROOT` (`greeting`), and the `SetTaglinePayload` record links it under `query`.
- Malformed responses throw, with the message of Errors: an empty response object `{}` for `Leaves` names `greeting`; `pet____id___v_id: "x"` at `["pet____id___v_id"]`; `petGrid: [[3]]` at `["petGrid",0,0]`; a pet without `__typename`; `id: 4`.
- A throw part-way leaves the earlier writes: `pets: [{ ..valid Pet:1 }, "x"]` throws at `["pets",1]`, and `store` holds `Pet:1` (open decision 9).

### `check` (`test/check.test.ts`)

Stores come from `normalize` of fixture responses, then are edited by hand where a case needs it.

- Empty store: `MissingData { record: queryRoot }`.
- For each fixture entrypoint, after `normalize` with the same variables: `EnoughData`.
- `PetDetail` with `{ id: "2", skip: null }` after `{ id: "1", skip: null }`: `MissingData { record: queryRoot }`.
- Leaf `null` (`nickname: null`): `EnoughData`. The `nickname` key deleted: `MissingData { record: Pet:1 }`.
- `Pet:2` deleted while `Pet:1.bestFriend` links it: `MissingData { record: Pet:2 }`.
- `stats: null`: `EnoughData`. `PetStats` `Pet:1.stats` deleted: `MissingData` for it.
- Lists: `maybePets: [null]` is `EnoughData`; `petGrid` with a link to a deleted record at `[1][0]` is `MissingData` for that record; `pets: []` is `EnoughData`.
- Conditionals: an `AdItem` record without `title` is `EnoughData`; `BlogItem:b1` without `title` is `MissingData { record: BlogItem:b1 }`.
- The first missing position in AST order is reported when two are missing.
- `EmptyStats`: the empty `PetStats` record is `EnoughData`; deleted, it is `MissingData` for it.
- A composite key holding `"x"` in a hand-built store throws the non-link message naming the record and store key.
- The store is unchanged.

### `garbageCollect` (`test/garbageCollection.test.ts`)

- `retained = []`: the store equals `new Map()` afterwards. Empty store with any retained query: still `new Map()`.
- One `PetDetail`: exactly `Query.__ROOT`, `Pet:1`, `Pet:2`, `PetStats` `Pet:1.stats`, and the checkins remain; a `Pet:9` written by `PetLists` is deleted, and so is the `PetLists`-only entity whose records were all unreached.
- `PetDetail` and `PetLists` both normalized, only `PetLists` retained: `Pet:1` stays whole, including fields only `PetDetail` selected; `Checkin` records go.
- Two roots: `PetDetail` and `SetTagline` both retained: `Query.__ROOT`, `Mutation.__ROOT`, and their records remain.
- Variables: `PetDetail` retained with `{ id: "2", skip: null }` when the store has only `pet____id___"1"`: everything but `Query.__ROOT` is deleted.
- Conditionals: `Feed` retained keeps `BlogItem:b1` and its `author`; an `AdItem` is kept (reached by the list), and the `asBlogItem` walk does not run on it.
- A link to an absent record and a retained root whose record is absent end the walk without error.
- Cyclic `bestFriend` data terminates.
- A composite key holding `"x"` throws the non-link message.

## Changes

Each change is one commit. Prerequisite: normalization-ast.md changes 1 to 3.

### 1. Test runner and fixture project

`vitest` and the `test` script; `target` and `include` in `tsconfig.json`; the CI step and the two pre-commit hooks; `fixtures/store` with its config, schema, sources, and generated artifacts; `test/fixtures.ts`.

### 2. Store, store keys, record ids

`src/store.ts`, `src/storeKey.ts` with `stableStringify`, `src/recordId.ts`; `test/storeKey.test.ts`. Depends on 1.

### 3. `normalize`

`src/normalize.ts`, its exports in `src/index.ts`, `test/normalize.test.ts`. Depends on 2.

### 4. `check`

`src/check.ts`, its export, `test/check.test.ts`. Depends on 3 (its tests build stores with `normalize`).

### 5. `garbageCollect`

`src/garbageCollection.ts`, its export, `test/garbageCollection.test.ts`. Depends on 3.

### 6. Link output types (typescript_artifacts)

`link_type_file` writes `{ entityName, id }` (The store); the type artifact tests that assert `__link` and `__typename` assert `entityName` and `id`; the regenerated `demos/pet-demo/src/__isograph/*/__link/output_type.ts` and the fixture's. Independent of 2 to 5; lands with them.

### 7. Required parameters (typescript_artifacts)

`parameter_property` writes every parameter as required, a nullable one as `T | null` (Variables); the type artifact tests that assert `readonly limit?: number | null | void` assert `readonly limit: number | null`; the regenerated `parameters_type.ts` artifacts of the pet demo and the fixture. Independent of 2 to 6; lands with them.

## Docs this doc amends

- `refactors/pending/graphql-protocol/normalization-ast.md`: `conditionHolds` is removed and the condition loses `responseKey` (Conditions; normalization-ast.md's own text says so); `tsconfig.json` `target` is `ES2022`.
- `refactors/pending/artifact-set-and-types.md`, Per field: `P/f/parameters_type.ts` is `{ readonly v: T; readonly w: T | null; }`, every parameter required (Variables); its test "`readonly limit?: number | null | void`" is `readonly limit: number | null`.
- `refactors/pending/runtime/pico-runtime.md`: the store is `Store` above, not upstream's layered store; `normalize`, `check`, and `garbageCollect` land here; its retained queries with a release buffer (intro, section 10, change 7) and its `log` option are dropped.

## Later

- `Node`-fetched entrypoints (normalization-ast.md, Later; entrypoints.md, Later). With them, `check` of a `Node` fetch for `Pet:4` is `MissingData { record: queryRoot }` whenever `node____id___"4"` was never written, even when another query wrote all of `Pet:4`, unless the walk starts below `node`. Upstream answers this with `missingFieldHandler` (`read.ts:781-783`), which v1 does not have.

## Open decisions

### 8. How the fixture artifacts are generated

The compiler writes artifacts only from the daemon today (`crates/isograph_cli/src/compile.rs`, `generate_artifacts`); there is no one-shot compile verb.

- (a) Committed, regenerated by running the daemon on `libs/isograph-runtime/fixtures/store/isograph.config.json`, as the pet demo's are (`scripts/restart-daemon-if-running.sh <config>`). No new machinery. Cost: nothing detects stale artifacts after a compiler change.
- (b) (a), plus a cargo test in `crates/isograph_cli` that compiles the fixture project in-process and asserts every committed artifact equals its regenerated text. Cost: a whole-file equality is snapshot-shaped (AGENTS.md, Tests), even though its purpose is freshness.
- (c) An `isograph compile --config <path>` verb that compiles once, writes, and exits; the runtime's `test` script runs it first and the artifacts are gitignored. Cost: a new CLI verb no doc has designed, and the JS tests and their pre-commit hook need a built Rust binary.

### 9. What a throw part-way through `normalize` leaves in the store

`normalize` writes into `store` in place, so a throw after some writes has already changed the store.

- (a) Validate the response against the AST in a read-only pass, throwing before any write, then write. A malformed response leaves `store` untouched. Cost: two walks of every response.
- (b) Write while walking, throw at the first mismatch, and leave the writes made before it, upstream's behavior (`normalizeData` writes through `getMutableStoreRecordProxy`, `cache.ts:93`, `:423`, and throws mid-walk at `cache.ts:297-302`). Cost: the store holds part of a response the server sent against a broken contract.
- (c) Walk once, collecting writes into a list, and apply the list at the end; a throw leaves `store` untouched. Cost: an intermediate list holding every write of every response.

Held: (b). A malformed response happens only when the server broke the contract of the query the compiler generated (schema skew between the compiler and the server, a broken server or proxy, a wrong test fixture). A correct server never produces one, and after the throw the environment is broken anyway, so a second walk on every response, or a list of every write, buys nothing in practice.

### 11. The parameters type of a field another field selects

`parameters_type.ts` is one type per field, used as the entrypoint's variables type and as the resolver's `parameters` type when another field selects the field. With every parameter required, `field Pet.X($n: Int)` selected as `X` with no argument gets `parameters` `{}` at runtime (reader-ast.md, `childVariables`), which its type `{ readonly n: number | null }` does not describe.

- (a) Required for every field; a selection that leaves a variable with no default unpassed is a generation error (reader-ast.md's former open decision 2 (c)). Cost: rejects a selection GraphQL allows.
- (b) Two generated types: the entrypoint's variables type, every variable required, and the field's parameters type, a nullable variable without a default optional. Cost: a second type artifact per entrypoint and an `entrypoint.ts` type change.
- (c) Required for every field, and `childVariables` writes `null` for an unpassed variable with no default. Cost: the resolver's `parameters` then says `null` where the store key was computed for an omitted argument.

Not held: raised with this change.
