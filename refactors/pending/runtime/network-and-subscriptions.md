# Runtime: the network path and `readAndSubscribe`

Status: changes 1 to 6 are approved for implementation; implementation starts after store.md's store and reader-ast.md's `read` land on i2. Request entries in the store (its own section, change 7) is a follow-up design under discussion, not approved.

`@isograph/runtime` (`libs/isograph-runtime`) gains the network path and subscriptions, as plain functions with no React. After this doc the package exposes `normalize`, `check`, `garbageCollect` (store.md), `read` (reader-ast.md), `readButDoNotEvaluate`, `readAndSubscribe`, `subscriptionSnapshot`, `callSubscriptions`, `fetchEntrypoint`, `getOrCreateCacheForArtifact`, `transpose`, and the environment functions `createIsographEnvironment`, `retainQuery`, `unretainQuery`, and `garbageCollectEnvironment`. The package has no React dependency and no React knowledge. A React package (`refactors/pending/runtime/react.md`) is a later plan built on these functions; it is not part of this work.

The lifetime model is upstream's disposable-state pattern: at render, a caller gets or creates a disposable item and temporarily retains it; at commit, it permanently retains it; a render that never commits has its temporary retain released by a timeout. The runtime provides exactly what that pattern needs, with no React in it: a request cache of upstream's `ParentCache`s, whose items are fetched requests, and subscriptions that are disposable items too, so a subscription made at render delivers every write until commit and is released if no commit comes. The cache and item classes are upstream's, unchanged; the runtime adds the factories (`fetchEntrypoint`, `readAndSubscribe`) and the cache lookup (`getOrCreateCacheForArtifact`).

The functions follow upstream's `makeNetworkRequest.ts` (`maybeMakeNetworkRequest`, `makeNetworkRequest`), `getOrCreateCacheForArtifact.ts`, `subscribe.ts` (`subscribe`, `callSubscriptions`), and `areEqualWithDeepComparison.ts`, which upstream keeps in `@isograph/react`'s `core/` folder, and `@isograph/react-disposable-state`'s `CacheItem.ts` and `ParentCache.ts`. Every delta below names upstream's code (`path:line`) and the decision that forces it. Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; the `core/` and `react/` files cited are unchanged since `79e2a3c0`, which store.md and reader-ast.md cite); paths without a directory are under `isograph/libs/isograph-react/src/core`, and `disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`.

Prerequisites, in their own docs:

- `refactors/pending/runtime/store.md` changes 1 to 7: `Store`, `StoreLink`, `Link`, `Variables`, `JsonObject`, `normalize`, `check`, `CheckResult`, `garbageCollect`, `RetainedQuery`, `stableStringify`, required parameters and input object fields.
- `refactors/pending/runtime/reader-ast.md` changes 1 to 6: `ReaderArtifact`, `ResolverParam`, `SelectionSetData`, `ReaderSelectionSet`, `ReaderSelection`, `read`, `ReadResult`, `ReadOk`, `ReadMissingData`, the `TEntityName` type parameter of reader artifacts and links, the typed `IsographEntrypoint.readerArtifact`, real `resolver_reader.ts` artifacts, the fixture's reader additions and `vitest.config.ts`.

`refactors/pending/runtime/pico-runtime.md` Step A is this doc's `readAndSubscribe` and `callSubscriptions`, with the amendments in Subscriptions. `refactors/pending/runtime/fetch-policies.md` lists directions beyond `shouldFetch`; this doc implements none of them.

## End-user experience

The only callers are the package's tests. A test plays a render and a commit with the cache and item methods, through a network function it resolves by hand:

```ts
// from libs/isograph-runtime/test/network.test.ts
import { createTemporarilyRetainedCacheItem } from "@isograph/disposable-types";
import { expect, test, vi } from "vitest";
import petDetail from "../fixtures/store/src/__isograph/Query/PetDetail/entrypoint";
import {
  createIsographEnvironment,
  garbageCollectEnvironment,
  getOrCreateCacheForArtifact,
  getPromiseState,
  readAndSubscribe,
  subscriptionSnapshot,
} from "../src/index";
import { deferredNetwork, petDetailResponse, queryRoot } from "./fixtures";

test("render, commit, write, release, collect", async () => {
  const network = deferredNetwork();
  const environment = createIsographEnvironment(new Map(), network.networkFunction);
  const variables = { id: "1", skip: null };

  // Render: get or create the request, temporarily retained; subscribe, temporarily retained.
  const cache = getOrCreateCacheForArtifact(environment, petDetail, variables, null);
  const [requestItem, networkRequest, releaseRequestRender] = cache.getOrPopulateAndTemporaryRetain();
  expect(network.calls).toHaveLength(1);
  const onResult = vi.fn();
  const [subscriptionItem, releaseSubscriptionRender] = createTemporarilyRetainedCacheItem(
    () => readAndSubscribe(environment, petDetail.readerArtifact, queryRoot, variables, onResult),
    () => {},
  );
  const subscription = subscriptionItem.getValue();
  const firstSnapshot = subscriptionSnapshot(subscription);
  expect(firstSnapshot.kind).toBe("MissingData");

  // The response arrives between render and commit; the subscription delivers it.
  network.calls[0].resolve(petDetailResponse({ name: "Rex" }));
  await networkRequest.promise;
  expect(onResult).toHaveBeenCalledTimes(1);
  expect(onResult.mock.calls[0][0]).toMatchObject({ kind: "Ok", value: { pet: { name: "Rex" } } });
  expect(subscriptionSnapshot(subscription)).toBe(onResult.mock.calls[0][0]);

  // Commit: permanent retains.
  const [, releaseRequest] = requestItem.permanentRetainIfNotDisposed(releaseRequestRender)!;
  const [, unsubscribe] = subscriptionItem.permanentRetainIfNotDisposed(releaseSubscriptionRender)!;

  // Unmount: release; the store keeps the records until the caller collects.
  unsubscribe();
  releaseRequest();
  expect(environment.subscriptions.size).toBe(0);
  expect(environment.retainedQueries.size).toBe(0);
  expect(getPromiseState(networkRequest).kind).toBe("Ok");
  garbageCollectEnvironment(environment);
  expect(environment.store).toEqual(new Map());
});
```

(The `!` is the test's assertion that the item was not disposed between render and commit, which the test established; production callers handle `null` as `useCachedResponsivePrecommitValue` does.)

What this shows:

- A render gets or creates the request through the environment's cache and temporarily retains it; the cache item's factory is `fetchEntrypoint`, which sends the request. A second render with the same query text and variables while the item is in the cache gets the same request.
- A render subscribes at once: `readAndSubscribe` reads and subscribes in one call, so a write between the render and the commit (here, the response) reaches the callback. `subscriptionSnapshot` returns the last result, the same object until a write changes it, which is what a React layer needs to avoid `setState` before commit (The snapshot).
- A commit converts each temporary retain into a permanent one. A render that never commits would have both released when the temporary retain's timeout fires: the subscription unsubscribed, and the request unretained.
- Releasing the last retain disposes the item: the subscription is removed, and the query is unretained. Nothing collects until the caller calls `garbageCollectEnvironment`.

## Isograph's primitives

Each primitive below is upstream's and React-free. The runtime uses them unchanged and adds only the code in the later sections.

- `CleanupFn`, `ItemCleanupPair<T>`, `Factory<T>` (`libs/isograph-disposable-types/src/index.ts:1-3`) and `joinCleanups` (`:9-30`), in `@isograph/disposable-types`, whose `package.json` has no dependencies. `fetchEntrypoint` returns an `ItemCleanupPair<PromiseWrapper<void, unknown>>`, upstream's return type (`makeNetworkRequest.ts:66`, with `AnyError` as `unknown`), and `readAndSubscribe` returns an `ItemCleanupPair<Subscription>`, whose cleanup is the `CleanupFn` upstream's `subscribe` returns (`subscribe.ts:23`, `:39`). Both are therefore `Factory`s when partially applied, which a `CacheItem` takes.
- `CacheItem`, `createTemporarilyRetainedCacheItem`, `CacheItemOptions`, `CacheItemState` (`disposable-state/CacheItem.ts`) and `ParentCache` (`disposable-state/ParentCache.ts`). Neither file imports React: `ParentCache.ts:1-7` imports only `@isograph/disposable-types` types and `./CacheItem`, and `CacheItem.ts:1-5` only `@isograph/disposable-types` types; their tests (`CacheItem.test.ts:1-12`, `ParentCache.test.ts:1-4`) import only vitest, those types, and the two modules. Their package, `@isograph/react-disposable-state`, peer-depends on `react` (`package.json`, `peerDependencies`). Change 2 moves both files and both tests, unchanged, into `@isograph/disposable-types`, which `@isograph/runtime` depends on (decision 5).
- `PromiseWrapper<T, E>`, `wrapPromise`, `wrapResolvedValue`, `readPromise`, `getPromiseState`, `PromiseState`, `Result`, `NOT_SET`, `NotSet` (`PromiseWrapper.ts:1-90`). The file imports nothing; it moves into `@isograph/runtime` with the `any` deltas below (`PromiseWrapper`).

What the pattern uses from them, by upstream's code:

- `ParentCache.getOrPopulateAndTemporaryRetain()` (`ParentCache.ts:55-59`): the render's call. With no item, it creates one by calling the factory (`__populateAndTemporaryRetain`, `:61-91`, through `createTemporarilyRetainedCacheItem`, `CacheItem.ts:366-374`); with one, it temporarily retains it. It returns the item, its value, and the temporary retain's cleanup.
- `CacheItem.temporaryRetain()` (`CacheItem.ts:195-256`): a temporary retain expires on a `setTimeout` of `temporaryRetainTime`, default 5000 ms (`:7`, `:248-251`), so a render that never commits releases it.
- `CacheItem.permanentRetainIfNotDisposed(disposeOfTemporaryRetain)` (`CacheItem.ts:114-193`): the commit's call. When the item is not disposed, it adds a permanent retain, releases the temporary one, and returns the value with the permanent retain's cleanup; when the temporary retain already expired and disposed the item, it returns `null`. A cleanup called twice throws "A permanent retain should only be cleared once." (`:125-130`, `:162-167`).
- `ParentCache.getAndPermanentRetainIfPresent()` (`ParentCache.ts:49-53`) and `ParentCache.factory` (`:97-99`): the commit's fallback when its item was disposed, upstream's `useCachedResponsivePrecommitValue` effect (`disposable-state/useCachedResponsivePrecommitValue.ts:55-104`: `permanentRetainIfNotDisposed` at `:79`, then `getAndPermanentRetainIfPresent` at `:91-92`, then `factory()` at `:97`). `useLazyDisposableState` stores the committed pair and calls its cleanup on unmount (`disposable-state/useLazyDisposableState.ts:23-43`).
- An item leaves its `ParentCache` when its last temporary retain is released, even while permanent retains remain (`__maybeExitInParentCacheAndNotDisposedState`, `CacheItem.ts:334-352`), and is disposed, calling the factory's cleanup, when its last retain of either kind is released (`:338-343`, `:354-363`). So the cache shares an item between the renders that overlap its temporary retains; a render after every holder committed creates a new item.

## The environment

The environment is created once and is stable: the app creates it with `createIsographEnvironment`, once, and holds it at module level (the tests and any non-React caller) or, in the later React package, in context, which `IsographEnvironmentProvider` and `useIsographEnvironment` only pass through. Every runtime function takes that one value and reads or writes its fields in place; none creates an environment, replaces one, or replaces a field of one (every field is `readonly`, and the store and the collections are mutated in place). Upstream's apps hold theirs the same way, created once in `useMemo(() => createIsographEnvironment(..), [])` (`isograph/demos/pet-demo/pages/_app.tsx:68-70`, `docs-website/docs/quickstart.md:165-175`). Each test creates its own with `createIsographEnvironment(new Map(), network.networkFunction)`, so no state crosses tests.

```ts
// from libs/isograph-runtime/src/environment.ts
import type { ParentCache } from "@isograph/disposable-types";
import { garbageCollect, type RetainedQuery } from "./garbageCollection";
import type { NetworkRequestKey } from "./network";
import type { PromiseWrapper } from "./PromiseWrapper";
import type { JsonObject, Store, Variables } from "./store";
import type { Subscription } from "./subscribe";

/** The store, the network function, and what is retained, cached, and subscribed. */
export type IsographEnvironment = {
  readonly store: Store;
  readonly networkFunction: IsographNetworkFunction;
  /** Every query a request holds; `garbageCollectEnvironment` keeps what these reach. */
  readonly retainedQueries: Set<RetainedQuery>;
  /** One cache per query text and variables, holding the request while a caller retains it. */
  readonly networkRequestCache: Map<NetworkRequestKey, ParentCache<PromiseWrapper<void, unknown>>>;
  /** Re-read after every write the environment makes. */
  readonly subscriptions: Set<Subscription>;
};

/** Sends `operation` with `variables`. Resolves to the operation's data object, or rejects. */
export type IsographNetworkFunction = (
  operation: IsographOperation,
  variables: Variables,
) => Promise<JsonObject>;

export type IsographOperation = {
  readonly text: string;
};

/** The app's one environment, created once. */
export function createIsographEnvironment(
  store: Store,
  networkFunction: IsographNetworkFunction,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    retainedQueries: new Set(),
    networkRequestCache: new Map(),
    subscriptions: new Set(),
  };
}

export function retainQuery(environment: IsographEnvironment, queryToRetain: RetainedQuery): void {
  environment.retainedQueries.add(queryToRetain);
}

export function unretainQuery(environment: IsographEnvironment, retainedQuery: RetainedQuery): void {
  environment.retainedQueries.delete(retainedQuery);
}

/** `garbageCollect` of the store, keeping what the retained queries reach. */
export function garbageCollectEnvironment(environment: IsographEnvironment): void {
  garbageCollect(environment.store, Array.from(environment.retainedQueries));
}
```

Each field has a reader:

- `store`: every function below reads or writes it.
- `networkFunction`: `makeNetworkRequest`, to send a request.
- `retainedQueries`: `garbageCollectEnvironment`.
- `networkRequestCache`: `getOrCreateCacheForArtifact`.
- `subscriptions`: `callSubscriptions`.

`retainedQueries` holds `RetainedQuery` objects by identity, as upstream's `Set` does (`garbageCollection.ts:57-63`): two requests that retain equal queries hold two entries, and disposing one leaves the other.

`createIsographEnvironment` is upstream's (`react/createIsographEnvironment.ts:10-23`, which calls `createIsographEnvironmentCore`, `IsographEnvironment.ts:178-209`) in the runtime, taking the store and the network function; upstream's `componentFunction`, `missingFieldHandler`, and `logFunction` parameters are dropped with the fields below. Upstream's `store` parameter is `BaseStoreLayerData`, usually `createIsographStore()` (`:211-217`); here it is store.md's `Store`, usually `new Map()`.

Deltas from upstream's `IsographEnvironment` (`IsographEnvironment.ts:73-93`) and `createIsographEnvironmentCore` (`:178-209`):

- `store` is store.md's `Store`, written in place, so the field is `readonly`; upstream reassigns `environment.store` to add layers (`makeNetworkRequest.ts:254`).
- `fragmentCache: CacheMap<FragmentReference<any, any>>` (`:81-83`, a plain object of `ParentCache`s) is `networkRequestCache`, a `Map` of `ParentCache`s of requests. The cache holds the request, not a fragment reference: a fragment reference is the React package's, and a map of references of every entrypoint needs `any` (react.md's former open decision on what the request cache holds, option (a)). Its entries are never removed, as upstream's are not (`ParentCache.ts:32`, "TODO pass an onEmpty function"): one empty `ParentCache` per distinct query text and variables stays in the map for the environment's life.
- `subscriptions` holds this doc's `Subscription`, not upstream's three-kind union (`:43-67`): nothing subscribes to a record or to every write.
- Dropped: `componentFunction`, `componentCache`, `eagerReaderCache` (no `@component`, no `@updatable`), `missingFieldHandler` (store.md, Later), `entrypointArtifactCache` (no lazily loaded entrypoints), `gcBuffer`, `gcBufferSize` (no release buffer, store.md), `loggers` (no log events).
- `unretainQuery` returns nothing and pushes to no buffer (`garbageCollection.ts:39-53`); `garbageCollectEnvironment` has no skip for store layers or unloaded ASTs (`:65-76`).

### Why the store-only functions stay store-only

`normalize`, `check`, `garbageCollect`, `read`, and `readButDoNotEvaluate` keep taking the `Store` and their inputs, not the environment. Each reads or writes the store and nothing else: none sends a request, none reads the retained queries (`garbageCollect` takes the list it keeps), none calls a subscription. Their tests (store.md, reader-ast.md) build a `Map` and call them with no network function and no collections. Taking the environment would make every one of those tests build an environment whose other four fields the function never reads, and would hide from each signature which state it touches. The environment functions compose them: `garbageCollectEnvironment` is `garbageCollect` of the retained list, and `makeNetworkRequest` calls `normalize` and `callSubscriptions`.

A caller that writes `environment.store` with `normalize` directly calls `callSubscriptions(environment)` afterwards; `normalize` does not notify, because it does not receive the subscriptions. Every write this package makes through the environment (`makeNetworkRequest`'s) calls it.

## The network function

Upstream: `IsographNetworkFunction = (operation, variables) => Promise<any>` (`IsographEnvironment.ts:103-106`). `makeNetworkRequest` reads `networkResponse.errors` (`makeNetworkRequest.ts:224-231`) and normalizes `networkResponse.data ?? {}` (`:246`, `:259`).

i2's network function resolves to the operation's data object, `JsonObject`, and rejects for anything else. This was react.md's decision (its The network function section and its open decision on what the network function returns), moved here with the network path. Reasons: `any` is banned (AGENTS.md, TypeScript); the runtime is protocol-neutral and names no GraphQL key (normalization-ast.md, The discriminator), and `data` and `errors` are GraphQL's response envelope; store.md says a `null` `data` is the caller's to handle before `normalize`. The user's network function owns the envelope: a GraphQL one is upstream's demo function (`isograph/demos/pet-demo/pages/_app.tsx:12-46`) ending in `return json.data`, after rejecting when `errors` is present or `data` is `null`.

`operation` is `{ text }`. Upstream's `IsographOperation.kind` (`entrypoint.ts:59-62`) told it apart from `IsographPersistedOperation`, which i2 does not have. `makeNetworkRequest` builds it as `{ text: entrypoint.queryText }`.

## The entrypoint and its variables

`fetchEntrypoint` takes the landed `IsographEntrypoint<TParam, TOutput, TEntityName>` (normalization-ast.md, The artifact's TypeScript type), with its reader artifact typed by reader-ast.md change 3 (reader-ast.md, The entrypoint's reader artifact): `TParam extends ResolverParam<TEntityName>`, `readerArtifact: ReaderArtifact<TParam, TOutput, TEntityName>`, and `normalizationAst: NormalizationAst<TEntityName>`, whose `root` is structurally `Link<TEntityName>`. So `fetchEntrypoint` infers the variables type, the output type, and the root entity from its argument, and reads at `entrypoint.normalizationAst.root` with no conversion.

The variables type comes from the param type:

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The variables of a field whose resolver parameter is `T`. */
export type ExtractParameters<T> = T extends { readonly parameters: infer P extends Variables } ? P : Variables;
```

Upstream's `ExtractParameters` (`FragmentReference.ts:29-33`), with store.md's `Variables` as the fallback instead of `any`. A generated param type's `parameters` is the field's `P__f__parameters` or `Record<PropertyKey, never>` (artifact-set-and-types.md, Per field); every parameters type is assignable to `Variables`, because every parameter and every input object field is required and none is `void` (store.md, Variables).

## `PromiseWrapper` and `transpose`

`libs/isograph-runtime/src/PromiseWrapper.ts` is upstream's `PromiseWrapper.ts:1-90` with react.md's former deltas: `AnyError` (`= any`) is removed and `E` defaults to `unknown`; `wrapPromise`'s wrapper is `PromiseWrapper<T, unknown>` and its `catch` parameter is `unknown`; the comment `// TODO confirm suspense works if the promise is already resolved.` is dropped. `result` stays mutable, because `wrapPromise` writes it. Every other line is upstream's.

```ts
// external isograph/libs/isograph-react/src/core/PromiseWrapper.ts
export type PromiseWrapper<T, E = any> = {
  readonly promise: Promise<Exclude<T, NotSet>>;
  result: Result<Exclude<T, NotSet>, E> | NotSet;
};
```

`readPromise` throws the pending promise, which is React's Suspense protocol. It stays exported because it is upstream's export and reads nothing React-specific; no runtime function calls it. `getPromiseState` is what a runtime caller reads.

The same file gains `transpose`, which upstream does not have (no function in `isograph/libs` takes a `PromiseWrapper<ItemCleanupPair<..>>` or returns an `ItemCleanupPair<PromiseWrapper<..>>` built from one):

```ts
// from libs/isograph-runtime/src/PromiseWrapper.ts
import type { CleanupFn, ItemCleanupPair } from "@isograph/disposable-types";

/** Where a `transpose` cleanup stands. */
type TransposeStatus =
  /** The cleanup has not run, and the inner pair has not arrived. */
  | { readonly kind: "Waiting" }
  /** The inner pair arrived, and the cleanup has not run. */
  | { readonly kind: "Arrived"; readonly cleanup: CleanupFn }
  /** The cleanup ran before the inner pair arrived; the inner cleanup runs when it arrives. */
  | { readonly kind: "ClearedBeforeArrival" }
  /** The cleanup ran after the inner pair arrived, or after `wrapper` rejected. */
  | { readonly kind: "Cleared" };

/**
 * A disposable item that arrives later, as an item whose value arrives later. The returned
 * wrapper settles as `wrapper` does, with the inner item or the same error; the returned
 * cleanup runs the inner cleanup once the inner pair exists.
 */
export function transpose<T>(
  wrapper: PromiseWrapper<ItemCleanupPair<T>, unknown>,
): ItemCleanupPair<PromiseWrapper<T, unknown>> {
  let status: TransposeStatus =
    wrapper.result !== NOT_SET && wrapper.result.kind === "Ok"
      ? { kind: "Arrived", cleanup: wrapper.result.value[1] }
      : { kind: "Waiting" };
  const transposed = wrapPromise<T>(
    wrapper.promise.then(([item, cleanup]) => {
      switch (status.kind) {
        case "Waiting":
          status = { kind: "Arrived", cleanup };
          break;
        case "ClearedBeforeArrival":
          status = { kind: "Cleared" };
          cleanup();
          break;
        case "Arrived":
        case "Cleared":
          break;
      }
      return item;
    }),
  );
  if (wrapper.result !== NOT_SET) {
    transposed.result =
      wrapper.result.kind === "Ok"
        ? { kind: "Ok", value: wrapper.result.value[0] }
        : { kind: "Err", error: wrapper.result.error };
  }
  return [
    transposed,
    () => {
      switch (status.kind) {
        case "Waiting":
          status = { kind: "ClearedBeforeArrival" };
          return;
        case "Arrived": {
          const cleanup = status.cleanup;
          status = { kind: "Cleared" };
          cleanup();
          return;
        }
        case "ClearedBeforeArrival":
        case "Cleared":
          throw new Error("A transposed cleanup should only be called once.");
      }
    },
  ];
}
```

Semantics, stated as facts the tests assert:

- The returned wrapper resolves to the inner item when `wrapper.promise` resolves, and rejects with the same error when it rejects. Its `result` follows by `wrapPromise`.
- An already-settled `wrapper` (its `result` is not `NOT_SET`, as `wrapResolvedValue` makes it) gives a returned wrapper whose `result` is already settled, synchronously, and a cleanup that runs the inner cleanup at once. The `.then` arm still runs a microtask later and changes nothing, because the status is already `Arrived` or `Cleared`.
- The cleanup called before the inner pair arrives records `ClearedBeforeArrival`; when the pair arrives, the inner cleanup runs at once, and the returned wrapper still resolves to the item.
- The cleanup called after the pair arrived runs the inner cleanup at once.
- When `wrapper` rejects, there is no inner pair; a cleanup called before or after the rejection does nothing besides moving the status, and the inner cleanup never exists to run.
- A cleanup called a second time throws "A transposed cleanup should only be called once.", the convention of upstream's disposable primitives, whose retain cleanups throw on a second call ("A permanent retain should only be cleared once.", `disposable-state/CacheItem.ts:125-130`, `:162-167`; "A temporary retain should only be cleared once.", `:206-210`). `@isograph/disposable-types` itself states no convention (`index.ts:1-3`); upstream's `makeNetworkRequest` cleanup is the other precedent, a no-op on a second call (`makeNetworkRequest.ts:325-337`).

Nothing in this doc calls `transpose`. The user's direction overrides AGENTS.md's "We do not have anything we do not use" for this one function: it ships with its tests, for a later caller that holds a disposable item behind a promise (a lazily loaded entrypoint whose request is itself an `ItemCleanupPair`).

## Fetching

### Fetch options

```ts
// from libs/isograph-runtime/src/network.ts
export type ShouldFetch = "Yes" | "No" | "IfNecessary";

export const DEFAULT_SHOULD_FETCH_VALUE: ShouldFetch = "IfNecessary";

export type FetchOptions<TReadOutData> = {
  readonly onComplete?: (data: TReadOutData) => void;
  readonly onError?: () => void;
  readonly shouldFetch?: ShouldFetch;
};
```

Upstream's semantics (`docs-website/docs/conditional-fetching.md`; `makeNetworkRequest.ts:67-118`): `Yes` sends the request; `No` retains without sending; `IfNecessary` sends when `check` reports `MissingData` and retains without sending on `EnoughData`. Deltas from `check.ts:13-32`: `ShouldFetch` is written as its three members (upstream unions `RequiredShouldFetch`, which only `RequiredFetchOptions` uses); `FetchOptionsShared` is folded in; `optimisticNetworkResponse` and `TRawResponseType` are dropped (no store layers, store.md); the properties are `readonly`.

### The request cache

```ts
// from libs/isograph-runtime/src/network.ts
/** The entrypoint's query text, then `stableStringify(variables)`. */
export type NetworkRequestKey = string;

/**
 * The environment's cache for `entrypoint`'s query text and `variables`, created on first
 * use. Its factory is `fetchEntrypoint` with `fetchOptions`.
 */
export function getOrCreateCacheForArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ParentCache<PromiseWrapper<void, unknown>> {
  const key: NetworkRequestKey = entrypoint.queryText + stableStringify(variables);
  const existing = environment.networkRequestCache.get(key);
  if (existing !== undefined) {
    return existing;
  }
  const created = new ParentCache(() => fetchEntrypoint(environment, entrypoint, variables, fetchOptions));
  environment.networkRequestCache.set(key, created);
  return created;
}
```

The key is upstream's: `operation.text + JSON.stringify(stableCopy(variables))` (`getOrCreateCacheForArtifact.ts:42-54`). store.md's `stableStringify` is `JSON.stringify` with every object's keys sorted at every depth and arrays in order, which is the text `JSON.stringify(stableCopy(value))` writes for every `JsonValue` (`util.ts:44-59`: `stableCopy` copies each object with its keys sorted, at every depth).

The render and commit sequence over it, upstream's `useCachedResponsivePrecommitValue` (`disposable-state/useCachedResponsivePrecommitValue.ts:55-115`) and `useLazyDisposableState` (`disposable-state/useLazyDisposableState.ts:18-57`), with no React:

1. Render: `cache.getOrPopulateAndTemporaryRetain()` returns `[cacheItem, networkRequest, disposeTemporaryRetain]`. With no item in the cache, it calls the factory, `fetchEntrypoint`, which checks, sends, and retains the query.
2. Commit: `cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain)`; when that is `null` (the temporary retain expired and disposed the item), `cache.getAndPermanentRetainIfPresent()`; when that is `null`, `cache.factory()`. The result is the committed `ItemCleanupPair<PromiseWrapper<void, unknown>>`.
3. Release: the committed pair's cleanup. When the item has no retain left, the `CacheItem` calls `fetchEntrypoint`'s cleanup, which unretains the query.

A render that never commits: its temporary retain expires after 5000 ms, and the item, with no other retain, is removed from the cache and disposed (`CacheItem.ts:334-343`): the query is unretained, and a response that arrives later is dropped (`makeNetworkRequest`, below).

Sharing follows the cache: renders whose temporary retains overlap get one item and one request; once every holder committed, the item has left the cache (`CacheItem.ts:344-351`), and a later render creates a new item, whose `fetchEntrypoint` checks again. With `IfNecessary` after the first response, `check` is `EnoughData` and nothing is sent; with `IfNecessary` while the first request is still in flight after its holders committed, `check` is `MissingData` and a second request is sent, as upstream sends it (decision 4). The factory closes over the first caller's `fetchOptions`, so a second caller's `onComplete` for a shared item is not called, as upstream's is not (`getOrCreateCacheForArtifact.ts:55-84`).

Deltas from upstream's `getOrCreateCacheForArtifact` (`getOrCreateCacheForArtifact.ts:26-86`) and `getOrCreateItemInSuspenseCache` (`cache.ts:34-47`): the cache holds the request, not a `FragmentReference`, so the factory is `fetchEntrypoint` itself (no `getOrLoadReaderWithRefetchQueries`, no fragment reference built around the request); the two functions are one, reading and writing a `Map`; `fetchOptions` is `FetchOptions | null`, as `maybeMakeNetworkRequest` takes it, instead of optional; no `PersistedOperation` key (`:46-50`).

### `fetchEntrypoint`

```ts
// from libs/isograph-runtime/src/network.ts
import { ParentCache } from "@isograph/disposable-types";
import type { ItemCleanupPair } from "@isograph/disposable-types";
import type { EntityName, ExtractParameters, IsographEntrypoint, NormalizationAst, ResolverParam } from "./artifacts";
import { check } from "./check";
import { type IsographEnvironment, retainQuery, unretainQuery } from "./environment";
import type { RetainedQuery } from "./garbageCollection";
import { normalize } from "./normalize";
import { type PromiseWrapper, wrapPromise, wrapResolvedValue } from "./PromiseWrapper";
import { read } from "./read";
import type { Variables } from "./store";
import { stableStringify } from "./storeKey";
import { callSubscriptions, reportCallbackError } from "./subscribe";

/**
 * Sends `entrypoint`'s query with `variables`, or retains it without sending, by
 * `fetchOptions.shouldFetch`. The cleanup unretains the query.
 */
export function fetchEntrypoint<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  switch (fetchOptions?.shouldFetch ?? DEFAULT_SHOULD_FETCH_VALUE) {
    case "Yes":
      return makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
    case "No":
      return retainQueryWithoutMakingNetworkRequest(environment, entrypoint.normalizationAst, variables);
    case "IfNecessary":
      return check(environment.store, entrypoint.normalizationAst, variables).kind === "EnoughData"
        ? retainQueryWithoutMakingNetworkRequest(environment, entrypoint.normalizationAst, variables)
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
  }
}

function retainQueryWithoutMakingNetworkRequest(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  variables: Variables,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  let status: NetworkRequestStatus = { kind: "Undisposed", retainedQuery: retainArtifact(environment, ast, variables) };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind === "Undisposed") {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: "Disposed" };
      }
    },
  ];
}

function makeNetworkRequest<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  const promise = environment.networkFunction({ text: entrypoint.queryText }, variables).then(
    (response) => {
      if (status.kind === "Disposed") {
        return;
      }
      normalize(environment.store, entrypoint.normalizationAst, response, variables);
      callSubscriptions(environment);
      const onComplete = fetchOptions?.onComplete;
      if (onComplete !== undefined) {
        const data = readDataForOnComplete(environment, entrypoint, variables);
        reportCallbackError(() => onComplete(data));
      }
    },
    (error: unknown) => {
      const onError = fetchOptions?.onError;
      if (onError !== undefined) {
        reportCallbackError(onError);
      }
      throw error;
    },
  );
  let status: NetworkRequestStatus = {
    kind: "Undisposed",
    retainedQuery: retainArtifact(environment, entrypoint.normalizationAst, variables),
  };
  return [
    wrapPromise(promise),
    () => {
      if (status.kind === "Undisposed") {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: "Disposed" };
      }
    },
  ];
}

type NetworkRequestStatus = NetworkRequestStatusUndisposed | NetworkRequestStatusDisposed;

type NetworkRequestStatusUndisposed = {
  readonly kind: "Undisposed";
  readonly retainedQuery: RetainedQuery;
};

type NetworkRequestStatusDisposed = {
  readonly kind: "Disposed";
};

/** The resolver's value for the response just normalized. */
function readDataForOnComplete<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
): TOutput {
  const result = read(environment.store, entrypoint.readerArtifact, entrypoint.normalizationAst.root, variables);
  if (result.kind === "MissingData") {
    throw new Error(
      `Reading ${entrypoint.name} right after normalizing its response found missing data: ` +
        `${result.reason}. This is indicative of a bug in Isograph.`,
    );
  }
  return result.value;
}

function retainArtifact(environment: IsographEnvironment, ast: NormalizationAst, variables: Variables): RetainedQuery {
  const retainedQuery: RetainedQuery = { ast, variables };
  retainQuery(environment, retainedQuery);
  return retainedQuery;
}
```

`status` is declared after the network call so that a network function that throws synchronously, instead of returning a promise, propagates out of `fetchEntrypoint` before anything is retained. The `.then` arms run in a later microtask, after the `let` is initialized. `read` infers `TEntityName` from `entrypoint.readerArtifact`, and `entrypoint.normalizationAst.root` is a `NormalizationRootRecord<TEntityName>`, which `NoInfer<Link<TEntityName>>` accepts (reader-ast.md, The entrypoint's reader artifact).

Behavior, stated as facts the tests assert:

- `Yes` and `IfNecessary` with `MissingData` call `makeNetworkRequest`; `No` and `IfNecessary` with `EnoughData` call `retainQueryWithoutMakingNetworkRequest`. Every path retains one new `RetainedQuery` `{ ast, variables }` in `environment.retainedQueries`.
- The cleanup unretains that query. A second call does nothing, upstream's behavior (`makeNetworkRequest.ts:325-337`). It never calls `garbageCollect`: collection runs only when the caller calls `garbageCollectEnvironment` (or store.md's `garbageCollect`).
- The response is normalized only when the cleanup has not run; then `callSubscriptions(environment)` runs, then `onComplete` with the resolver's value. After the cleanup ran, the response is dropped, upstream's `Disposed` behavior (`makeNetworkRequest.ts:235`).
- `onComplete` is called only when the response was normalized. Upstream calls it after disposal too (`makeNetworkRequest.ts:280-297`), when nothing guarantees the store holds what the response wrote.
- `onError` is called when the request rejects, disposed or not, as upstream calls it (`:299-307`). The wrapper is then `Err` with the error.
- A callback (`onComplete`, `onError`) that throws does not change the wrapper's state; its error is reported by `reportCallbackError` (Errors in re-reads and callbacks).

Deltas from `makeNetworkRequest.ts`:

- `maybeMakeNetworkRequest` (`:46-121`) is `fetchEntrypoint`, the public name; upstream exports `makeNetworkRequest` (`index.ts`), which ignores `shouldFetch`, and keeps `maybeMakeNetworkRequest` for its hooks. `makeNetworkRequest` and `retainQueryWithoutMakingNetworkRequest` keep upstream's names and are module-private. No `RefetchQueryNormalizationArtifact` in the artifact union and no `readerWithRefetchQueries` parameter (no refetch queries); no `NormalizationAstLoader` throw (`:85-93`); `check` takes store.md's arguments, and the root is the AST's.
- `retainQueryWithoutMakingNetworkRequest` (`:123-157`) takes the AST, the only part of the entrypoint it reads.
- `makeNetworkRequest` (`:159-340`): the network function receives `{ text: queryText }` and resolves to the data object, so the `errors` check (`:224-231`) and `?? {}` are gone; no optimistic layer (`:190-198`, `:236-251`, `:309-316`, `:326-333`); no logs and no module-level `networkRequestId` counter (`:44`, `:181-182`), which only the logs read; the query is retained after the network call instead of before it (`:183-189`), so a synchronous throw retains nothing.
- The statuses (`:342-360`): `UndisposedIncomplete` and `UndisposedComplete` are one `Undisposed`. Upstream distinguishes them for the optimistic layer's revert; without it the two behave the same.
- `unretainAndGarbageCollect` (`:565-577`) is `unretainQuery` alone. The user's decision: collection runs only when the caller asks for it. Upstream collects when the release buffer overflows.
- `readDataForOnComplete` (`:362-462`) reads with reader-ast.md's `read`, which calls the resolver; there is no component case and no fake fragment reference. `fetchNormalizationAstAndRetainArtifact` (`:464-493`) is `retainArtifact`: the AST is never loaded lazily.
- Upstream swallows a throwing `onComplete` or `onError` (`try { .. } catch {}`, `:290-296`, `:305-307`); here it is reported.

## Reading without the root resolver

```ts
// from libs/isograph-runtime/src/read.ts
/** What `artifact`'s reader AST reads at `root` with `variables`, with every nested resolver
 * called and `artifact`'s own resolver not called. */
export function readButDoNotEvaluate<TEntityName extends EntityName>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<SelectionSetData> {
  if (store.get(root.entityName)?.get(root.id) === undefined) {
    return { kind: "MissingData", reason: `No record for ${recordName(root)}`, recordLink: root };
  }
  return readSelectionSet(store, artifact.readerAst, root, variables);
}
```

`read` becomes `readButDoNotEvaluate`, then, on `Ok`, `artifact.resolver({ data, parameters: variables, link: root })`; its behavior is unchanged. This is react.md's former `readButDoNotEvaluate`, moved here: the subscriptions compare the reader's data, not the resolver's value (Subscriptions). The name is upstream's (`read.ts:60`), which also evaluates nested resolvers and leaves the root's to its caller (`react/useResult.ts:63-64`). The type parameters and the root's type are reader-ast.md's `read`'s.

## Subscriptions

```ts
// from libs/isograph-runtime/src/subscribe.ts
import type { ItemCleanupPair } from "@isograph/disposable-types";
import { selectionSetDataAreEqual } from "./areEqualWithDeepComparison";
import type { EntityName, ReaderArtifact, ResolverParam, SelectionSetData } from "./artifacts";
import type { IsographEnvironment } from "./environment";
import { readButDoNotEvaluate, type ReadMissingData, type ReadOk } from "./read";
import type { Link, StoreLink, Variables } from "./store";

/** What a subscription delivers: the reader's data, `MissingData`, or what the read threw. */
export type SubscriptionResult = ReadOk<SelectionSetData> | ReadMissingData | ReadErr;

/** A read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: "Err";
  readonly error: unknown;
};

export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
};

/**
 * Reads `readerArtifact` at `link` now and subscribes: after every write, re-reads and calls
 * `callback` when the result changed. Returns the subscription, whose `subscriptionSnapshot`
 * is the read, and the cleanup that unsubscribes.
 */
export function readAndSubscribe<TEntityName extends EntityName>(
  environment: IsographEnvironment,
  readerArtifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
  callback: (result: SubscriptionResult) => void,
): ItemCleanupPair<Subscription> {
  const subscription: Subscription = {
    artifact: readerArtifact,
    root: link,
    variables,
    delivered: readSubscription(environment, readerArtifact, link, variables),
    callback,
  };
  environment.subscriptions.add(subscription);
  return [
    subscription,
    () => {
      environment.subscriptions.delete(subscription);
    },
  ];
}

/** The subscription's last result: the same object until a write changes the read. */
export function subscriptionSnapshot(subscription: Subscription): SubscriptionResult {
  return subscription.delivered;
}

/**
 * Re-reads every subscription in `environment.subscriptions`, in insertion order, and calls
 * each one's callback when its result changed. Never throws.
 */
export function callSubscriptions(environment: IsographEnvironment): void {
  environment.subscriptions.forEach((subscription) => {
    const result = readSubscription(environment, subscription.artifact, subscription.root, subscription.variables);
    if (!subscriptionResultsAreEqual(subscription.artifact, subscription.delivered, result)) {
      subscription.delivered = result;
      reportCallbackError(() => subscription.callback(result));
    }
  });
}

function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionResult {
  try {
    return readButDoNotEvaluate(environment.store, artifact, root, variables);
  } catch (error: unknown) {
    return { kind: "Err", error };
  }
}

/** `Ok` results by the reader AST's rule; two `MissingData` are equal; two `Err` when their
 * errors are `===`; different kinds never. */
function subscriptionResultsAreEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  delivered: SubscriptionResult,
  result: SubscriptionResult,
): boolean {
  switch (delivered.kind) {
    case "Ok":
      return result.kind === "Ok" && selectionSetDataAreEqual(artifact.readerAst, delivered.value, result.value);
    case "MissingData":
      return result.kind === "MissingData";
    case "Err":
      return result.kind === "Err" && delivered.error === result.error;
  }
}

/** Calls `f`; a throw is rethrown in its own microtask, so it reaches the host's uncaught-error
 * handler and does not stop the caller. */
export function reportCallbackError(f: () => void): void {
  try {
    f();
  } catch (error: unknown) {
    queueMicrotask(() => {
      throw error;
    });
  }
}
```

`reportCallbackError` is a module export that `network.ts` imports; the package index does not export it.

Typing: `readAndSubscribe` ties `readerArtifact` and `link` to one `TEntityName` with reader-ast.md's `read` signature (The parent entity's name): `TEntityName` is inferred from the artifact's declared third type argument, and `NoInfer` keeps the link out of the inference, so a `Link<"Pet">` passed with a `Query` reader fails `tsc`, and so does a `StoreLink`. The stored `Subscription` holds the pair at `EntityName`, because one `Set` holds subscriptions of every entity: `ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>` is assignable to `ReaderArtifact<ResolverParam, unknown, EntityName>` (`TEntityName` does not occur in the body, and `resolver` is compared bivariantly) and `Link<TEntityName>` to `StoreLink`, with no cast. `readSubscription` calls `readButDoNotEvaluate` with the pair at `EntityName`, which reader-ast.md's `Resolver` case does too. If `tsc` rejects either assignment, the implementer stops and raises it.

### The two-phase lifetime

`readAndSubscribe` returns an `ItemCleanupPair`, so `() => readAndSubscribe(environment, artifact, link, variables, callback)` is a `Factory<Subscription>` and a subscription is a disposable item like the request:

1. Render: `createTemporarilyRetainedCacheItem(factory, () => {})` (`CacheItem.ts:366-374`) calls the factory, so the subscription reads and subscribes at render, and returns `[cacheItem, disposeTemporaryRetain]`. `subscriptionSnapshot(cacheItem.getValue())` is the read the render uses. The no-op is the item's `removeFromParentCache`: a subscription is one caller's, so no `ParentCache` holds it.
2. Between render and commit, every write that changes the read calls `callback`, so no write is missed.
3. Commit: `cacheItem.permanentRetainIfNotDisposed(disposeTemporaryRetain)` returns the subscription and the permanent cleanup. When it returns `null`, the temporary retain expired and already unsubscribed; the commit calls the factory again and uses its fresh read.
4. A render that never commits: the temporary retain expires after 5000 ms, the item is disposed, and its cleanup unsubscribes (`CacheItem.ts:338-343`).
5. Release: the permanent cleanup unsubscribes.

A caller with no render and commit (a test, a non-React app) uses the pair directly: the subscription, whose snapshot is the read, and the unsubscribe.

### The snapshot

A React layer must not call `setState` before its component commits (`disposable-state/useUpdatableDisposableState.ts:73-79` throws "Calling setState before the component has committed is unsafe and disallowed."), yet the subscription made at render delivers writes before commit. `subscriptionSnapshot` lets that layer read instead of being told: it returns `subscription.delivered`, which `callSubscriptions` replaces only when a re-read is unequal to it (Semantics), so between changes it returns the same object (`===`), and after a change it returns the object the callback received. This is what both later React shapes need, with nothing React in the runtime:

- `useSyncExternalStore(subscribe, getSnapshot)`: `getSnapshot` is `() => subscriptionSnapshot(subscription)`, stable while nothing changed, as React requires; `subscribe` registers React's listener, which the subscription's `callback` calls (the React layer's callback forwards to the listener it holds once committed).
- Upstream's compare-at-commit (`useCachedResponsivePrecommitValue.ts:55-104`, re-render when the committed value differs from the rendered one): the commit compares `subscriptionSnapshot(subscription)` with the snapshot the render used, by `!==`, and re-renders when they differ; the callback before commit only needs to do nothing.

### Semantics

pico-runtime.md's Step A with react.md's three amendments, each holding upstream's behavior:

- The subscription reads at subscribe time, in the same synchronous call that adds it, so no write falls between the caller's read and the subscription. react.md's amendment (subscribe takes the data the caller read) is met by construction: the read the caller gets is the subscription's `delivered`. Upstream subscribes in an effect after render and re-reads on subscribe to catch writes in between (`subscribe.ts:32-36`); here the subscription exists from the render, under its temporary retain (The two-phase lifetime).
- The callback receives the new `SubscriptionResult` of `readButDoNotEvaluate`, the reader's data, not the root resolver's value. Upstream's callback receives the data (`IsographEnvironment.ts:44-47`) and its caller calls the resolver (`react/useResult.ts:63-64`); a callback that also received the resolver's value would make a caller that renders it run the resolver twice per change. A caller that wants the value calls `readerArtifact.resolver(..)` with the data (decision 1).
- A `MissingData` re-read is delivered as `MissingData` when `delivered` is not `MissingData`. A subscriber that showed data learns that it is gone (a garbage collection followed by a write, a hand edit), and a subscriber whose first read was `MissingData` is called at the first `Ok`. Upstream's re-read throws a promise on `MissingData` (`read.ts:92-122`), which `logAnyError` catches (`subscribe.ts:47-61`), so upstream's callback is never called with it; with a typed result there is nothing to catch, and dropping it would hide a state change from the caller.

Equality, `selectionSetDataAreEqual` (upstream's `mergeObjectsUsingReaderAst`, `areEqualWithDeepComparison.ts:4-121`), per reader node, at the node's `alias`:

- `Scalar`: `null` equals only `null`; two lists are equal when their lengths are equal and their items are equal at every depth; any other two values by `===`. A re-normalized string, number, or boolean is equal; a `JSON` scalar's object or a list of objects is a new object after every `normalize` that writes it, so it is unequal, as upstream's `===` makes it (`:22-23`).
- `Linked`: `null` and lists as `Scalar`; two records' data by `selectionSetDataAreEqual` of the node's `selections`.
- `Refinement`: as `Linked` (`null`, or the same record's data). Upstream reads a refinement as a `Linked` node with a `condition` (reader-ast.md, Refinements), which this rule follows.
- `Resolver`: `===`. In v1 every read calls every nested resolver again (reader-ast.md, no caching), so a nested resolver that returns a new object or list on every call (store.md's fixture resolvers return `data`) makes every re-read unequal, and its subscription's callback is called after every write, related or not. A nested resolver that returns a string, number, boolean, or `null` is equal when its inputs are. This is upstream's rule (`:84-95`), and pico-runtime.md's changes 1 to 4 remove the cost.

"Changed" therefore means: not equal by this rule. Upstream also recycles equal subtrees into the new object (`:42-52`, `:76-82`) so that a React render sees stable references; nothing in the runtime reads object identity of delivered data, so the comparison returns a `boolean` and does not write into the new read. The React plan adds recycling with its first reader.

```ts
// from libs/isograph-runtime/src/areEqualWithDeepComparison.ts
import type { ReaderSelection, ReaderSelectionSet, SelectionSetData } from "./artifacts";

/** Whether two reads of `selections` hold equal values, upstream's rule. */
export function selectionSetDataAreEqual(
  selections: ReaderSelectionSet,
  delivered: SelectionSetData,
  read: SelectionSetData,
): boolean {
  return selections.every((selection) =>
    selectionValuesAreEqual(selection, delivered[selection.alias], read[selection.alias]),
  );
}

function selectionValuesAreEqual(selection: ReaderSelection, delivered: unknown, read: unknown): boolean {
  switch (selection.kind) {
    case "Scalar":
      return valuesAreEqual(delivered, read, (a, b) => a === b);
    case "Linked":
    case "Refinement":
      return valuesAreEqual(
        delivered,
        read,
        (a, b) => isSelectionSetData(a) && isSelectionSetData(b) && selectionSetDataAreEqual(selection.selections, a, b),
      );
    case "Resolver":
      return delivered === read;
  }
}

/** `null` equals only `null`; lists by length and item, at every depth; other values by `itemsAreEqual`. */
function valuesAreEqual(
  delivered: unknown,
  read: unknown,
  itemsAreEqual: (delivered: unknown, read: unknown) => boolean,
): boolean {
  if (delivered === null || read === null) {
    return delivered === read;
  }
  if (isList(delivered) || isList(read)) {
    return (
      isList(delivered) &&
      isList(read) &&
      delivered.length === read.length &&
      delivered.every((item, index) => valuesAreEqual(item, read[index], itemsAreEqual))
    );
  }
  return itemsAreEqual(delivered, read);
}

function isList(value: unknown): value is ReadonlyArray<unknown> {
  return Array.isArray(value);
}

function isSelectionSetData(value: unknown): value is SelectionSetData {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
```

A `Linked` item that is not an object is a read result that reader-ast.md's `readLinkedValue` never produces; the comparison reports it unequal rather than throwing, because the callback then receives the read as it is.

### Ordering

`callSubscriptions` visits `environment.subscriptions` in insertion order with `Set.prototype.forEach`, as upstream's `forEach` does (`subscribe.ts:67`), and for each subscription re-reads, compares, and calls its callback before re-reading the next. A callback that removes a subscription not yet visited (its own or another's) removes it from this pass; one that adds a subscription adds it to this pass, where its first comparison is with the read `readAndSubscribe` just made, so it is not called unless a write happened in between. Every re-read runs after `normalize` finished and reads the whole store, so no re-read sees a partial write (pico-runtime.md, Step A). A callback that writes the store synchronously (store.md's `normalize` on `environment.store`) changes what later subscriptions in the same pass read; it does not re-run earlier ones, until its own `callSubscriptions`.

No overlap filter: store.md's `normalize` records no `EncounteredIds`, so every subscription is re-read after every write, where upstream skips the subscriptions whose recorded records do not overlap the written ones (`subscribe.ts:74-79`). A subscription whose reader reads only server values is still not called after an unrelated write, because its comparison is equal.

`garbageCollectEnvironment` does not call `callSubscriptions`, as upstream's collection calls none (`garbageCollection.ts:65-86`). A subscription whose records were collected learns of it at the next write, when its re-read is `MissingData`.

### Errors in re-reads and callbacks

Upstream wraps every subscription in `logAnyError` (`subscribe.ts:47-61`, `:67-108`): it catches what a re-read or a callback throws, logs it, and continues with the next subscription. i2 has no log function, so the two cases are handled as follows:

- A read that throws (a nested resolver threw; a store invariant broke, store.md's non-link message), at subscribe or at a re-read, becomes `{ kind: "Err", error }`, compared with `delivered` and delivered to that subscription's callback like any other result. The loop continues; nothing escapes `readAndSubscribe`, `callSubscriptions`, `normalize`'s caller, or the request, whose wrapper stays `Ok`.
- A callback that throws: `reportCallbackError` rethrows its error in its own microtask (`queueMicrotask`), so it reaches the host's uncaught-error handler (`window.onerror`, Node's `uncaughtException`) without stopping the loop or rejecting the request. The same function reports a throwing `onComplete` or `onError`. Upstream logs and discards these (`logAnyError`; `try { .. } catch {}` at `makeNetworkRequest.ts:290-296`, `:305-307`); with no log, discarding would hide the error (decision 3).

`delivered` is updated before the callback is called, so a callback that throws has still received the result.

Deltas from Step A (pico-runtime.md) and from react.md's former amended Step A:

- `subscribe` is `readAndSubscribe(environment, readerArtifact, link, variables, callback)`, which reads, subscribes, and returns `[subscription, unsubscribe]`, with `subscriptionSnapshot(subscription)` the read; there is no `delivered` parameter, because the read and the subscribe are one call.
- `callSubscriptions` takes the environment, since the subscriptions live there, and never throws. pico-runtime.md's open decision 9 (a), rethrow after the loop, held in react.md for the re-read error, is replaced by delivery as `Err` (upstream's isolation, without the log).
- The callback receives `SubscriptionResult`, `MissingData` and `Err` included; react.md's amendment called nothing on `MissingData`.

## Errors

Every throw this doc adds, and its invariant:

- `readDataForOnComplete` on `MissingData`: "Reading `<name>` right after normalizing its response found missing data: `<reason>`. This is indicative of a bug in Isograph." It rejects that request's wrapper. Invariant: the entrypoint's reader reads only what its query selects (query-generation.md), `normalize` wrote all of it or threw, and the request's query is retained, so `garbageCollectEnvironment` keeps it. Only a caller's direct store-only `garbageCollect` with a list that leaves the query out, run from a subscription callback between the write and `onComplete`, breaks it, and that call breaks the retention contract.
- `transpose`'s cleanup called twice: "A transposed cleanup should only be called once." Invariant: the caller releases a pair once, as every disposable primitive requires (`PromiseWrapper` and `transpose`).
- store.md's `normalize` throws on a malformed response; in `makeNetworkRequest` that rejects the request, so the wrapper is `Err` with the message, `onError` is called, and no subscription is called. The writes made before the throw stay (store.md, `normalize`).
- The network function's rejection: the wrapper is `Err` with that error, and `onError` is called. The store is unchanged.
- A throwing callback: reported in its own microtask (Errors in re-reads and callbacks).
- The moved `CacheItem` and `ParentCache` keep upstream's throws ("A permanent retain should only be cleared once.", "This indicates a bug in react-disposable-state.").

## Tests

vitest, in store.md's test infrastructure, over store.md's fixture project with reader-ast.md's additions (`libs/isograph-runtime/fixtures/store`), whose `entrypoint.ts` artifacts are `IsographEntrypoint`s with typed reader artifacts (reader-ast.md change 3). No snapshots; tests assert wrapper states, network calls, callback calls and arguments, store contents, and the sizes of the environment's collections. No test sleeps or polls (AGENTS.md, Wake on events): the network function returns a promise the test settles, the test awaits the wrapper's promise, and a temporary retain's expiry is driven with `vi.useFakeTimers()` and `vi.advanceTimersToNextTimer()`, as upstream's `CacheItem.test.ts:20`, `:93` do.

```ts
// from libs/isograph-runtime/test/fixtures.ts
/** One call of a `deferredNetwork` network function, which the test settles. */
export type DeferredNetworkCall = {
  readonly operation: IsographOperation;
  readonly variables: Variables;
  readonly resolve: (response: JsonObject) => void;
  readonly reject: (error: unknown) => void;
};

/** A network function that records each call and returns a promise only the test settles. */
export function deferredNetwork(): {
  readonly networkFunction: IsographNetworkFunction;
  readonly calls: ReadonlyArray<DeferredNetworkCall>;
};

/** `PetDetail`'s response for `Pet:1` with `Pet:2` as its best friend, with `overrides` on `Pet:1`. */
export function petDetailResponse(overrides: { readonly name?: string; readonly nickname?: string | null }): JsonObject;

/** `PetLists`'s response with one pet, `Pet:9`. */
export function petListsResponse(): JsonObject;
```

### `test/PromiseWrapper.test.ts` (`transpose`)

- Resolve then cleanup: `wrapPromise` of a deferred pair; `transpose`; the returned wrapper is `Pending`; resolve with `[item, innerCleanup]`; after its promise, the returned wrapper is `Ok` with `item`, and `innerCleanup` has no call; the returned cleanup: `innerCleanup` has one call.
- Cleanup then resolve: the returned cleanup before the resolve: `innerCleanup` has no call; resolve; after the returned wrapper's promise: `innerCleanup` has one call, and the returned wrapper is `Ok` with `item`.
- Reject: reject with `error`; the returned wrapper is `Err` with `error`, and its promise rejects with it; the returned cleanup, before and in another test after the rejection, does nothing and throws nothing.
- Double cleanup: a second call of the returned cleanup throws the message, before the resolve and after it; `innerCleanup` has at most one call.
- Synchronous: `transpose(wrapResolvedValue([item, innerCleanup]))`: the returned wrapper's `result` is `{ kind: "Ok", value: item }` synchronously; the returned cleanup calls `innerCleanup` at once; after a microtask, `innerCleanup` still has one call.

### `test/environment.test.ts`

- `createIsographEnvironment(store, f)`: `store` is the same object, and the three collections are empty.
- `retainQuery` twice with two equal `{ ast, variables }` objects: `retainedQueries.size` is 2; `unretainQuery` of one leaves 1.
- `garbageCollectEnvironment` with `PetDetail` normalized and retained: the store keeps `PetDetail`'s records and deletes a `Pet:9` that `PetLists` wrote; with nothing retained, the store equals `new Map()`.

### `test/network.test.ts`

`fetchEntrypoint`:

- `IfNecessary` on an empty store: one call, with `{ text: petDetail.queryText }` and the variables; `retainedQueries.size` is 1; the wrapper is `Pending`. After `resolve`, the wrapper is `Ok` and `check` is `EnoughData`.
- `IfNecessary` after the same response was normalized: no call; the wrapper is `Ok` at once; `retainedQueries.size` is 1.
- `Yes` with the data present: one call. `No` on an empty store: no call, the wrapper is `Ok` at once, one retained query, and the store stays empty. `null` options behave as `IfNecessary`.
- Cleanup before the response: `retainedQueries.size` is 0; after `resolve`, the store equals `new Map()` (the response was dropped), the wrapper is `Ok`, and `onComplete` has no call.
- Cleanup after the response: `retainedQueries.size` is 0, and the store still holds the records (no collection); `garbageCollectEnvironment` then empties it. A second cleanup changes nothing.
- `onComplete` receives the resolver's value; an `onComplete` that throws leaves the wrapper `Ok`, and the error is reported (`vi.stubGlobal("queueMicrotask", spy)`: the spy received a function that throws that error).
- `reject(error)`: the wrapper is `Err` with `error`; `onError` has one call; the store is unchanged.
- A response missing `pet____id___v_id`: the wrapper is `Err` with store.md's malformed-response message; `onError` has one call; a subscription has no call.
- A network function that throws synchronously: `fetchEntrypoint` throws it; `retainedQueries.size` is 0.

The request cache (`getOrCreateCacheForArtifact` with the moved `ParentCache`):

- The same entrypoint and equal variables (two objects, keys in different orders) return the same `ParentCache`; other variables return another.
- Two renders (`getOrPopulateAndTemporaryRetain` twice) before any commit: one network call, one retained query, the same `networkRequest` object.
- Render, then commit with `permanentRetainIfNotDisposed`: the pair's first element is the render's `networkRequest`; `cache.isEmpty()` is `true` (the item left the cache at its last temporary retain).
- Render with no commit, then `vi.advanceTimersToNextTimer()`: `retainedQueries.size` is 0; a response resolved afterwards is dropped (the store equals `new Map()`).
- Render, the timer fires, then commit: `permanentRetainIfNotDisposed` is `null`, `getAndPermanentRetainIfPresent()` is `null`, and `cache.factory()` sends a second network call.
- After a commit, a second render while the first request is in flight: a second network call (`check` is `MissingData`). After the first response, a render with `IfNecessary`: no call.
- Release of the committed pair: `retainedQueries.size` is 0; the store keeps the records.

### `test/subscribe.test.ts`

Each test builds an environment over `deferredNetwork` and writes through `fetchEntrypoint` (or `normalize` on `environment.store` followed by `callSubscriptions`).

- `readAndSubscribe` on an empty store: the snapshot is `MissingData`, `subscriptions.size` is 1, and the callback has no call. The response arrives: one call with `Ok`, and the snapshot is that object.
- Snapshot stability: after an unrelated write (`PetLists` writing `Pet:9`) on a `PetDetail` subscription, `subscriptionSnapshot` returns the same object as before (`toBe`); after a related write, a different object, equal to the callback's argument (`toBe`).
- Related and unrelated writes, `PetDetail` (only `Scalar` and `Linked` nodes): a `PetDetail` response that changes `Pet:1`'s `name`: one call with the new data, and `delivered` is that result. A `PetLists` response writing only `Pet:9`: no call. The same `PetDetail` response again: no call.
- `PetSummary` (nested resolvers that return `data` objects): a `PetLists` response writing only `Pet:9`: one call, with data equal in value to the last delivered. This pins v1's `===` rule on nested resolver values.
- Equality by node: `Leaves` re-normalized with equal `tags: ["a", null]` (a new list): no call; with `settings` an equal `JSON` object (a new object): one call; a refinement in `Feed` that stays `null`: no call.
- Data then `MissingData`: `Pet:1`'s `nickname` deleted by hand, then `callSubscriptions`: one call with `MissingData` whose `recordLink` is `Pet:1`; `callSubscriptions` again: no call; a write that restores it: one call with `Ok`.
- Unsubscribe: after the cleanup, `subscriptions.size` is 0 and a changing write calls nothing.
- Two-phase lifetime: `createTemporarilyRetainedCacheItem(() => readAndSubscribe(..), () => {})`; a write before the commit calls the callback; `permanentRetainIfNotDisposed` returns the pair; releasing it leaves `subscriptions.size` 0. Without a commit, `vi.advanceTimersToNextTimer()` leaves `subscriptions.size` 0 and a later write calls nothing.
- Read errors: a nested resolver that throws (a fixture field whose resolver throws when `data.name` is `"Throw"`): at subscribe, the snapshot is `Err`; at a re-read, the callback is called once with `{ kind: "Err", error }`; a second subscription that reads healthy data is still called; the request's wrapper is `Ok`. The same throw again (a new `Error` object): one more call. `Pet:1.stats` set by hand to `"x"`: `Err` with store.md's non-link message.
- A callback that throws: the other subscriptions are still called, `callSubscriptions` returns, and the error is reported through `queueMicrotask` (stubbed as above).
- Ordering: three subscriptions all changed by one write are called in subscription order. A callback that unsubscribes the third: the third has no call. A callback that unsubscribes itself: the others are still called.
- Degenerate: `callSubscriptions` with no subscriptions does nothing; a reader whose AST is empty (`Constant` at `Pet:1`) reads `Ok` with `{}` and is never called after a write.

The fixture gains one source file for the throwing resolver:

```text
field Pet.ThrowingName { name }        throws when data.name is "Throw", else returns data.name
field Query.ThrowingPet($id: ID!) {
  pet(id: $id) { ThrowingName }
}
```

with `entrypoint Query.ThrowingPet`.

## Request entries in the store

The user's direction: the request cache lives in the store, so that garbage collection handles it with the records. This section is the held design for that, its feasibility, and what does not fit. It is change 7, after changes 1 to 6, and is not approved; until it is, the environment's `networkRequestCache` and `retainedQueries` above stand.

### Upstream's two structures

Upstream keeps a request's lifetime in two places that different code maintains. The suspense cache, `fragmentCache: CacheMap<FragmentReference<any, any>>` (`IsographEnvironment.ts:81-83`), holds one `ParentCache` per query text and variables (`getOrCreateCacheForArtifact.ts:42-54`, created by `getOrCreateItemInSuspenseCache`, `cache.ts:34-47`), and its entries are never removed (`ParentCache.ts:32`). The retained set, `retainedQueries: Set<RetainedQuery>` (`IsographEnvironment.ts:89`) with the release buffer `gcBuffer` (`:90-91`), is what collection keeps (`garbageCollection.ts:65-86`). The two disagree by construction: a query in the buffer keeps its records while its `ParentCache` is empty, so a render refetches or re-checks data the store still holds; a `ParentCache` stays in the map after its query was collected.

### The held design

```ts
// from libs/isograph-runtime/src/store.ts
export type Store = {
  /** Every record: `EntityName`, then record id, then the record. */
  readonly records: Map<EntityName, Map<RecordId, StoreRecord>>;
  /** Every request not yet collected, by `RequestKey`. */
  readonly requests: Map<RequestKey, RequestEntry>;
};

/** The entrypoint's query text, then `stableStringify(variables)`: upstream's cache key. */
export type RequestKey = string;

/** A fetched or retained query: what collection keeps, and who holds it. */
export type RequestEntry = {
  readonly ast: NormalizationAst;
  readonly variables: Variables;
  readonly networkRequest: PromiseWrapper<void, unknown>;
  /** Renders not yet committed; each expires after 5000 ms. */
  temporaryRetainCount: number;
  /** Commits not yet released. */
  permanentRetainCount: number;
};
```

The root link is `ast.root` (normalization-ast.md), so the entry has no separate root field.

- The retained set is derived: an entry with `temporaryRetainCount + permanentRetainCount > 0` is retained. `retainedQueries`, `retainQuery`, and `unretainQuery` are removed, and so is `networkRequestCache`: one structure holds both what is cached and what is retained, so the two cannot disagree.
- `garbageCollect(store)` takes no retained list. Its mark walks the AST of every retained entry from `ast.root`, as store.md's walk does from each `RetainedQuery`; its sweep deletes every entry with no retain and every record no retained entry reaches, in the same call.
- A render calls `getOrCreateRequest(environment, entrypoint, variables, fetchOptions)`, which returns the entry under the key or creates one by `fetchEntrypoint`'s `shouldFetch` logic, then `temporaryRetainRequest(environment, entry): CleanupFn`, which increments `temporaryRetainCount` and schedules its expiry. A commit calls `permanentRetainRequest(entry, releaseTemporaryRetain): ItemCleanupPair<PromiseWrapper<void, unknown>>`, which increments `permanentRetainCount` and releases the temporary retain; its cleanup decrements. Each cleanup throws on a second call, as `CacheItem`'s do (`CacheItem.ts:125-130`, `:206-210`).
- A response is normalized when its entry is still in `requests`, and dropped when collection deleted it first. An entry whose retains reached 0 stays in `requests`, and a render with its key reuses it, until `garbageCollect` deletes it.
- Concurrent identical requests share one entry: an entry stays under its key while in flight and while retained, so a second render, before or after the first commits, finds it and sends nothing. Decision 4's second request does not happen.

### How `ParentCache` and `CacheItem` map onto it

The entry replaces `ParentCache` and `CacheItem` for requests; their retain counting is written again as data on the entry. Moving their storage into the store and keeping their role does not work, for two reasons:

- `CacheItem` keeps its counts and state in private fields (`CacheItem.ts:69-71`, `private __state`), so `garbageCollect` cannot read whether an item is retained without reaching into them.
- `ParentCache` forgets an item when its temporary retain count reaches 0, even while it is permanently retained (`CacheItem.ts:344-351`), so a store holding `ParentCache`s cannot find a committed request; collection would delete records a committed request still needs.

The entry's states map onto `CacheItem`'s three (`CacheItem.ts:46-67`): `InParentCacheAndNotDisposed` is an entry with `temporaryRetainCount > 0`; `NotInParentCacheAndNotDisposed` is an entry with only permanent retains, which here stays in `requests` and stays shareable (the deviation that gives the sharing above); `NotInParentCacheAndDisposed` is an entry with no retain, which here stays until collected instead of disposing at once. `CacheItem` and `ParentCache` stay for subscriptions (The two-phase lifetime) and for the React package.

Keeping `ParentCache` and holding the `ParentCache`s in the store, with `garbageCollect` deleting the empty ones, fixes only upstream's never-removed entries; the retained set stays a second structure (decision 6, option (b)).

### What does not fit

- Expiry needs a timer. A temporary retain expires after 5000 ms, a side effect that no store function may own: `normalize`, `check`, `garbageCollect`, and `read` stay pure functions of their arguments. `temporaryRetainRequest` takes the environment and schedules with `setTimeout`, as `CacheItem` does (`CacheItem.ts:248-251`); tests drive it with fake timers (decision 7).
- Promises are not data. An entry holds a `PromiseWrapper`, whose `promise` is a `Promise` and whose `result` `wrapPromise` writes later. The store stops being plain data: `structuredClone(store)` throws (`DataCloneError` on a `Promise`), so the store cannot be copied, persisted, or sent across a worker or server boundary; and vitest's `toEqual` treats any two `Promise`s as equal (they have no own enumerable properties), so a test that compares whole stores passes for different requests. store.md's tests compare `store.records` instead of `store`, and request entries are compared field by field with `toBe` on `networkRequest` (decision 8).
- `Store` stops being the two-layer `Map`. store.md's "The type is the two-layer map and nothing else", its empty store `new Map()`, and every `store.get(entityName)` in `normalize`, `check`, `garbageCollect`, `read`, and their tests become `store.records`; pico-runtime.md's store entries and epochs (its change 1) land in the same struct. This is an amendment of store.md, which is under discussion.
- Refetching. An entry shared after commit answers every later render with its key, `Yes` included, until it is collected; upstream's `Yes` after commit creates a new item and sends (decision 9).

### Change 7. Request entries in the store (not approved)

`Store` becomes `{ records, requests }` with `RequestKey` and `RequestEntry`; store.md's functions and tests read `store.records`; `garbageCollect(store)` marks from retained entries and sweeps entries and records; `getOrCreateRequest`, `temporaryRetainRequest`, `permanentRetainRequest` replace `getOrCreateCacheForArtifact`; `retainedQueries`, `retainQuery`, `unretainQuery`, `networkRequestCache`, and `garbageCollectEnvironment` are removed; the tests of The request cache are rewritten against entries. Depends on 6 and on the store.md amendment.

## Changes

Each change is one commit, in order. Prerequisites: store.md changes 1 to 7 and reader-ast.md changes 1 to 6. Each package change adds its CI steps to `.github/workflows/js.yml` and its pre-commit hooks (the runtime's two, with the package name and `files: '^libs/<directory>/'`), and runs `pnpm install` to update `pnpm-lock.yaml`.

### 1. `PromiseWrapper` and `transpose` (libs/isograph-disposable-types, libs/isograph-runtime)

`@isograph/disposable-types` ported unchanged from upstream at `9d72f6fc` (`src/index.ts`, `src/joinCleanups.test.ts`), with `private`, source `exports` (`types`, `import`), and the `typecheck` and `test` scripts; `@isograph/runtime` depends on it. `src/PromiseWrapper.ts` with `transpose`; their exports; `test/PromiseWrapper.test.ts`. Depends only on these two files and the port.

### 2. `CacheItem` and `ParentCache` in `@isograph/disposable-types`

`CacheItem.ts`, `ParentCache.ts`, `CacheItem.test.ts`, `ParentCache.test.ts` moved unchanged from upstream's `@isograph/react-disposable-state` at `9d72f6fc`, with two import lines changed: `from '@isograph/disposable-types'` is `from './index'` in the four files, because the files now live in that package. `src/index.ts` gains `export * from './CacheItem';` and `export * from './ParentCache';`. Depends on 1.

### 3. `readButDoNotEvaluate` (libs/isograph-runtime)

`readButDoNotEvaluate` in `src/read.ts`; `read` calls it; its export; `read.test.ts` additions: `readButDoNotEvaluate` of `PetSummary` returns the data `read` passes to the root resolver, with a spy on the root resolver having no call and spies on `PetName` and `RecentCheckins` one call each; on an absent root it is `MissingData` with `No record for Query:__ROOT`.

### 4. The environment (libs/isograph-runtime)

`src/environment.ts` with `IsographEnvironment`, `IsographNetworkFunction`, `IsographOperation`, `createIsographEnvironment`, `retainQuery`, `unretainQuery`, `garbageCollectEnvironment`; `NetworkRequestKey` as a type in `src/network.ts`; `Subscription` as a type in `src/subscribe.ts`; `ExtractParameters` in `src/artifacts.ts`; their exports; `test/environment.test.ts`; `deferredNetwork` in `test/fixtures.ts`. Depends on 2.

### 5. Subscriptions (libs/isograph-runtime)

`src/areEqualWithDeepComparison.ts`; `src/subscribe.ts` with `SubscriptionResult`, `ReadErr`, `readAndSubscribe`, `subscriptionSnapshot`, `callSubscriptions`, `reportCallbackError`; their exports; the `ThrowingName` fixture source and its regenerated artifacts; `test/subscribe.test.ts`, writing with `normalize` on `environment.store` then `callSubscriptions`. Depends on 3 and 4.

### 6. The network path (libs/isograph-runtime)

`src/network.ts` with `ShouldFetch`, `DEFAULT_SHOULD_FETCH_VALUE`, `FetchOptions`, `getOrCreateCacheForArtifact`, `fetchEntrypoint`, and the private functions; their exports; `test/network.test.ts`; `test/subscribe.test.ts` cases that write through `fetchEntrypoint`. Depends on 5.

```ts
// from libs/isograph-runtime/src/index.ts
// existing exports of store.md and reader-ast.md unchanged, plus:
export type { ExtractParameters } from "./artifacts";
export {
  createIsographEnvironment,
  garbageCollectEnvironment,
  type IsographEnvironment,
  type IsographNetworkFunction,
  type IsographOperation,
  retainQuery,
  unretainQuery,
} from "./environment";
export {
  DEFAULT_SHOULD_FETCH_VALUE,
  fetchEntrypoint,
  type FetchOptions,
  getOrCreateCacheForArtifact,
  type NetworkRequestKey,
  type ShouldFetch,
} from "./network";
export {
  getPromiseState,
  NOT_SET,
  type NotSet,
  type PromiseState,
  type PromiseWrapper,
  readPromise,
  type Result,
  transpose,
  wrapPromise,
  wrapResolvedValue,
} from "./PromiseWrapper";
export { readButDoNotEvaluate } from "./read";
export {
  callSubscriptions,
  readAndSubscribe,
  type ReadErr,
  type Subscription,
  type SubscriptionResult,
  subscriptionSnapshot,
} from "./subscribe";
```

The index re-exports nothing of `@isograph/disposable-types`; callers import `ParentCache`, `CacheItem`, and `ItemCleanupPair` from it. `package.json`'s `exports` keeps only its `types` condition: no module outside the package imports its runtime values in this work, and tests import `../src/index`. The React plan adds the `import` condition with its first importer.

## Docs this doc amends

- `refactors/pending/runtime/react.md`: not part of this work. Its environment, network function, `ExtractParameters`, `PromiseWrapper`, fetch options, retention, request cache (`getOrCreateCacheForArtifact`, `networkRequestCache`), `maybeMakeNetworkRequest` and `makeNetworkRequest`, `readButDoNotEvaluate`, and amended Step A are this doc's, in `@isograph/runtime`; `CacheItem` and `ParentCache` are `@isograph/disposable-types`'s, and its port of `@isograph/react-disposable-state` imports them from there. Disposal unretains and does not collect, so its open decision about when collection runs is the React package's choice of when to call `garbageCollectEnvironment`.
- `refactors/pending/runtime/pico-runtime.md`, Step A: `subscribe` is `readAndSubscribe(environment, readerArtifact, link, variables, callback)`, returning `[subscription, unsubscribe]`, with `subscriptionSnapshot`; `callSubscriptions(environment)` never throws, and the callback receives `SubscriptionResult` (Subscriptions). Its open decision 10 is taken as (a). Its open decision 9 (a) does not apply to Step A: a re-read error is delivered as `Err`.
- `refactors/pending/runtime/store.md`: its "No `IsographEnvironment`" and "No subscriptions" (Removed from upstream) hold for the store functions; the environment and subscriptions are this doc's, composed over them.
- `refactors/pending/runtime/reader-ast.md`: `read` is `readButDoNotEvaluate` then the resolver.

## Later

- A network function that takes an `AbortSignal`, so that disposing a request's item cancels it (pico-runtime.md, Follow-ups 6).
- An overlap filter on `callSubscriptions`, with `normalize` recording the records it wrote (upstream's `EncounteredIds`).
- Replacing `readAndSubscribe` with pico-runtime.md's level 2 (`listen`, `snapshot`).
- `writeData` (upstream `writeData.ts:23-79`), a second write site that calls `callSubscriptions`.
- A caller of `transpose`.

## Decisions

Decisions 1 to 5 were open during planning and are settled as their held options. Decisions 6 to 9 concern change 7 and are open.

### 1. What the subscription callback receives

- (a) `SubscriptionResult` of `readButDoNotEvaluate`: the reader's data. Settled. Upstream's shape (`IsographEnvironment.ts:44-47`); a caller that renders the resolver's value calls it once, in its own render. Cost: a non-React caller calls `readerArtifact.resolver(..)` itself.
- (b) The root resolver's value. Cost: the resolver runs inside `callSubscriptions` for every changed subscription, and again in a React render that calls it; the comparison still has to run on the data, so the subscription holds both.

### 2. When two `MissingData` results are equal

- (a) Always. Settled. A caller waiting for data acts only on `Ok`.
- (b) When `recordLink` and `reason` are equal. Cost: a caller is called each time the first missing value moves, which no caller in this work reads.

### 3. A callback that throws

- (a) Rethrow in its own microtask. Settled. The loop and the request continue, and the error reaches the host's uncaught-error handler. Cost: the error surfaces with no stack frame of the write that caused it, and a vitest run fails on it unless the test stubs `queueMicrotask`.
- (b) Swallow it, upstream's behavior without its log. Cost: the error is invisible.
- (c) Rethrow after the loop. Cost: the request whose response was normalized becomes `Err`, a failure the network did not have.

### 4. A second request for a query in flight after its holders committed

Once every holder of a request's item committed, the item has left its `ParentCache` (`CacheItem.ts:344-351`), and a new render with the same key creates a new item while the first request is still in flight; with `IfNecessary`, `check` is `MissingData` and a second request is sent.

- (a) Send it, upstream's behavior. Settled: the user's direction is upstream's lifetime model.
- (b) Keep a map of requests in flight beside the cache, and have `makeNetworkRequest` join one with the same key instead of sending. Cost: a second map and a join path that upstream does not have; each joined caller's options and cleanup need their own rule.

### 5. Where `CacheItem` and `ParentCache` live

- (a) In `@isograph/disposable-types`, moved unchanged. Settled. The package already holds the types they import and a runtime function (`joinCleanups`), it has no dependencies, and `@isograph/react-disposable-state` already re-exports all of it (`disposable-state/index.ts:1`, `export * from '@isograph/disposable-types'`), so that package's public exports are unchanged when its `CacheItem` and `ParentCache` modules move. Cost: four import lines change in the moved files (`'@isograph/disposable-types'` is `'./index'`), and the React package's port changes its imports of `./CacheItem` and `./ParentCache` (`useCachedResponsivePrecommitValue.ts`, `useLazyDisposableState.ts`, `useDisposableState.ts`, `index.ts`, and their tests) to `@isograph/disposable-types`.
- (b) A new React-free package holding the two files. Cost: a third package beside upstream's two, and the same import changes.
- (c) Leave them in `@isograph/react-disposable-state` and have the runtime depend on it. Cost: the runtime depends on a package whose peer dependency is `react`.

### 6. Where the request cache and the retained set live

Open; the user's direction holds (a) unless it fails.

- (a) Request entries in the store (Request entries in the store), one structure for cache and retention, collected with the records. Cost: retain counting written again outside `CacheItem`; the store holds promises; `Store` becomes a struct; sharing after commit (decision 9).
- (b) `networkRequestCache` holds its `ParentCache`s in the store, and `garbageCollect` deletes the empty ones; `retainedQueries` stays. Cost: two structures that can still disagree; it only fixes upstream's never-removed cache entries.
- (c) Changes 1 to 6 as approved: the environment holds both. Cost: upstream's two structures and its never-removed entries.

### 7. Who owns the temporary retain's timer

Open.

- (a) `temporaryRetainRequest` calls `setTimeout`, as `CacheItem` does. Cost: a runtime function with a time side effect; tests need fake timers.
- (b) `createIsographEnvironment` takes a scheduler, `(expire: () => void) => CleanupFn`, defaulting to `setTimeout` with 5000 ms. Cost: a parameter only tests and unusual hosts pass.

### 8. Promises in the store

Open.

- (a) The entry holds the `PromiseWrapper` (the held design). Cost: the store is not plain data (no `structuredClone`, no persistence), and whole-store `toEqual` is unsound for requests.
- (b) The entry holds the request's state as data (`Pending`, `Ok`, `Err` with the error), and the environment holds the promises in a `Map<RequestKey, Promise<void>>`. Cost: a second map again, keyed like the entries, which `garbageCollect` must also sweep.

### 9. What a render with an existing entry's key gets

Open.

- (a) The existing entry, whatever `shouldFetch` says, as upstream's cache does during a temporary retain (`getOrCreateCacheForArtifact.ts:55-84`), extended to the entry's whole life. Cost: `Yes` does not refetch while an entry with that key exists.
- (b) The existing entry while its request is in flight; after it settled, `Yes` (and `IfNecessary` with `MissingData`) sends a new request into the same entry, replacing `networkRequest`. Cost: holders of the old wrapper do not see the new request, and an entry's `networkRequest` changes under its holders.
