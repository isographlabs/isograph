# Runtime: the network path and `readAndSubscribe`

Status: under discussion. Not approved for implementation.

`@isograph/runtime` (`libs/isograph-runtime`) gains the network path and subscriptions, as plain functions with no React. After this doc the package exposes `normalize`, `check`, `garbageCollect` (store.md), `read` (reader-ast.md), `readButDoNotEvaluate`, `readAndSubscribe`, `callSubscriptions`, `fetchEntrypoint`, and the environment functions `createIsographEnvironment`, `retainQuery`, `unretainQuery`, and `garbageCollectEnvironment`. The package has no React dependency and no React knowledge. A React package (`refactors/pending/runtime/react.md`) is a later plan built on these functions; it is not part of this work.

The functions follow upstream's `makeNetworkRequest.ts` (`maybeMakeNetworkRequest`, `makeNetworkRequest`), `subscribe.ts` (`subscribe`, `callSubscriptions`), and `areEqualWithDeepComparison.ts`, which upstream keeps in `@isograph/react`'s `core/` folder. Every delta below names upstream's code (`path:line`) and the decision that forces it. Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; the `core/` and `react/` files cited are unchanged since `79e2a3c0`, which store.md and reader-ast.md cite); paths without a directory are under `isograph/libs/isograph-react/src/core`.

Prerequisites, in their own docs:

- `refactors/pending/runtime/store.md` changes 1 to 7: `Store`, `StoreLink`, `Variables`, `JsonObject`, `normalize`, `check`, `CheckResult`, `garbageCollect`, `RetainedQuery`, `stableStringify`, required parameters.
- `refactors/pending/runtime/reader-ast.md` changes 1 to 6: `ReaderArtifact`, `ResolverParam`, `SelectionSetData`, `ReaderSelectionSet`, `ReaderSelection`, `read`, `ReadResult`, `ReadOk`, `ReadMissingData`, the `TEntityName` type parameter of reader artifacts and links, real `resolver_reader.ts` artifacts, `DeclaredConst`, `ExportDefaultBinding`, the fixture's reader additions and `vitest.config.ts`.

`refactors/pending/runtime/pico-runtime.md` Step A is this doc's `readAndSubscribe` and `callSubscriptions`, with the amendments in Subscriptions. `refactors/pending/runtime/fetch-policies.md` lists directions beyond `shouldFetch`; this doc implements none of them.

## End-user experience

The only callers are the package's tests. A test fetches an entrypoint through a network function it resolves by hand, subscribes to the entrypoint's reader, and observes the callbacks:

```ts
// from libs/isograph-runtime/test/network.test.ts
import { expect, test, vi } from "vitest";
import petDetail from "../fixtures/store/src/__isograph/Query/PetDetail/entrypoint";
import {
  createIsographEnvironment,
  fetchEntrypoint,
  getPromiseState,
  readAndSubscribe,
  readButDoNotEvaluate,
  garbageCollectEnvironment,
} from "../src/index";
import { deferredNetwork, petDetailResponse, queryRoot } from "./fixtures";

test("fetch, subscribe, dispose, collect", async () => {
  const network = deferredNetwork();
  const environment = createIsographEnvironment(new Map(), network.networkFunction);
  const variables = { id: "1", skip: null };

  const handle = fetchEntrypoint(environment, petDetail, variables, null);
  expect(network.calls).toHaveLength(1);
  expect(getPromiseState(handle.networkRequest).kind).toBe("Pending");

  const delivered = readButDoNotEvaluate(environment.store, petDetail.readerArtifact, queryRoot, variables);
  expect(delivered.kind).toBe("MissingData");
  const callback = vi.fn();
  const unsubscribe = readAndSubscribe(environment, petDetail.readerArtifact, queryRoot, variables, delivered, callback);
  expect(callback).not.toHaveBeenCalled();

  network.calls[0].resolve(petDetailResponse({ name: "Rex" }));
  await handle.networkRequest.promise;
  expect(getPromiseState(handle.networkRequest)).toEqual({ kind: "Ok", value: undefined });
  expect(callback).toHaveBeenCalledTimes(1);
  expect(callback.mock.calls[0][0]).toMatchObject({ kind: "Ok", value: { pet: { name: "Rex" } } });

  unsubscribe();
  handle.dispose();
  expect(environment.retainedQueries.size).toBe(0);
  expect(environment.store.size).toBeGreaterThan(0);
  garbageCollectEnvironment(environment);
  expect(environment.store).toEqual(new Map());
});
```

What this shows:

- `fetchEntrypoint` calls the network function at once and returns a handle whose `networkRequest` is upstream's `PromiseWrapper` and whose `dispose` releases the query.
- The caller reads first, then subscribes with the read it holds. A write between the two is caught by the comparison `readAndSubscribe` makes at once.
- The response is normalized into `environment.store`, then every subscription is re-read; the callback receives the new `ReadResult` of the reader's data.
- `dispose` unretains and does not collect. The records stay until the caller calls `garbageCollectEnvironment`.

## The package's additions

```text
libs/isograph-runtime/
  src/environment.ts
  src/PromiseWrapper.ts
  src/network.ts
  src/subscribe.ts
  src/areEqualWithDeepComparison.ts
  test/fixtures.ts            (gains deferredNetwork and response builders)
  test/environment.test.ts
  test/network.test.ts
  test/subscribe.test.ts
```

`src/read.ts` gains `readButDoNotEvaluate`, and `src/artifacts.ts` gains `ExtractParameters`.

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
  type NetworkRequestHandle,
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
} from "./subscribe";
```

`package.json`'s `exports` keeps only its `types` condition: no module outside the package imports its runtime values in this work, and tests import `../src/index`. The React plan adds the `import` condition with its first importer.

## The environment

```ts
// from libs/isograph-runtime/src/environment.ts
import type { RetainedQuery } from "./garbageCollection";
import type { JsonObject, Store, Variables } from "./store";
import type { InFlightRequest, NetworkRequestKey } from "./network";
import type { Subscription } from "./subscribe";

/** The store, the network function, and what is retained, in flight, and subscribed. */
export type IsographEnvironment = {
  readonly store: Store;
  readonly networkFunction: IsographNetworkFunction;
  /** Every query a handle holds; `garbageCollectEnvironment` keeps what these reach. */
  readonly retainedQueries: Set<RetainedQuery>;
  /** One entry per request sent and not yet settled, by query text and variables. */
  readonly inFlightRequests: Map<NetworkRequestKey, InFlightRequest>;
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

export function createIsographEnvironment(
  store: Store,
  networkFunction: IsographNetworkFunction,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    retainedQueries: new Set(),
    inFlightRequests: new Map(),
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
- `networkFunction`: `fetchEntrypoint`, to send a request.
- `retainedQueries`: `garbageCollectEnvironment`.
- `inFlightRequests`: `fetchEntrypoint`, to join an identical request in flight (Deduplication).
- `subscriptions`: `callSubscriptions`.

`store` is written in place (store.md), so the field is `readonly`; upstream reassigns `environment.store` to add layers (`makeNetworkRequest.ts:254`). `retainedQueries` holds `RetainedQuery` objects by identity, as upstream's `Set` does (`garbageCollection.ts:57-63`): two handles that retain equal queries hold two entries, and disposing one leaves the other.

Deltas from upstream's `IsographEnvironment` (`IsographEnvironment.ts:73-93`) and `createIsographEnvironmentCore` (`:178-209`):

- `fragmentCache: CacheMap<FragmentReference<any, any>>` (`:81-83`), React's cache of `ParentCache`s, is `inFlightRequests`, a `Map` of the requests in flight. It holds no React state, no retain counts, and no fragment references; an entry is removed when its request settles.
- `subscriptions` holds this doc's `Subscription`, not upstream's three-kind union (`:43-67`): nothing subscribes to a record or to every write.
- Dropped: `componentFunction`, `componentCache`, `eagerReaderCache` (no `@component`, no `@updatable`), `missingFieldHandler` (store.md, Later), `entrypointArtifactCache` (no lazily loaded entrypoints), `gcBuffer`, `gcBufferSize` (no release buffer, store.md), `loggers` (no log events).
- `unretainQuery` returns nothing and pushes to no buffer (`garbageCollection.ts:39-53`); `garbageCollectEnvironment` has no skip for store layers or unloaded ASTs (`:65-76`).

### Why the store-only functions stay store-only

`normalize`, `check`, `garbageCollect`, `read`, and `readButDoNotEvaluate` keep taking the `Store` and their inputs, not the environment. Each reads or writes the store and nothing else: none sends a request, none reads the retained queries (`garbageCollect` takes the list it keeps), none calls a subscription. Their tests (store.md, reader-ast.md) build a `Map` and call them with no network function and no sets. Taking the environment would make every one of those tests build an environment whose other four fields the function never reads, and would hide from each signature which state it touches. The environment functions compose them: `garbageCollectEnvironment` is `garbageCollect` of the retained list, and `fetchEntrypoint` calls `check`, `normalize`, and `callSubscriptions`.

A caller that writes `environment.store` with `normalize` directly calls `callSubscriptions(environment)` afterwards; `normalize` does not notify, because it does not receive the subscriptions. Every write this package makes through the environment (`fetchEntrypoint`'s) calls it.

## The network function

Upstream: `IsographNetworkFunction = (operation, variables) => Promise<any>` (`IsographEnvironment.ts:103-106`). `makeNetworkRequest` reads `networkResponse.errors` (`makeNetworkRequest.ts:224-231`) and normalizes `networkResponse.data ?? {}` (`:246`, `:259`).

i2's network function resolves to the operation's data object, `JsonObject`, and rejects for anything else. This was react.md's decision (its The network function section and its open decision on what the network function returns), moved here with the network path. Reasons: `any` is banned (AGENTS.md, TypeScript); the runtime is protocol-neutral and names no GraphQL key (normalization-ast.md, The discriminator), and `data` and `errors` are GraphQL's response envelope; store.md says a `null` `data` is the caller's to handle before `normalize`. The user's network function owns the envelope: a GraphQL one is upstream's demo function (`isograph/demos/pet-demo/pages/_app.tsx:12-46`) ending in `return json.data`, after rejecting when `errors` is present or `data` is `null`.

`operation` is `{ text }`. Upstream's `IsographOperation.kind` (`entrypoint.ts:59-62`) told it apart from `IsographPersistedOperation`, which i2 does not have. `fetchEntrypoint` builds it as `{ text: entrypoint.queryText }`.

## The entrypoint and its variables

`fetchEntrypoint` takes the landed `IsographEntrypoint<TParam, TOutput, TEntityName>` (normalization-ast.md, The artifact's TypeScript type), with its reader artifact typed by reader-ast.md change 3 (reader-ast.md, The entrypoint's reader artifact): `TParam extends ResolverParam<TEntityName>`, `readerArtifact: ReaderArtifact<TParam, TOutput, TEntityName>`, and `normalizationAst: NormalizationAst<TEntityName>`, whose `root` is structurally `Link<TEntityName>`. So `fetchEntrypoint` infers the variables type, the output type, and the root entity from its argument, and reads at `entrypoint.normalizationAst.root` with no conversion.

The variables type comes from the param type:

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The variables of a field whose resolver parameter is `T`. */
export type ExtractParameters<T> = T extends { readonly parameters: infer P extends Variables } ? P : Variables;
```

Upstream's `ExtractParameters` (`FragmentReference.ts:29-33`), with store.md's `Variables` as the fallback instead of `any`. A generated param type's `parameters` is the field's `P__f__parameters` or `Record<PropertyKey, never>` (artifact-set-and-types.md, Per field); every parameters type is assignable to `Variables`, because every parameter and every input object field is required and none is `void` (store.md, Variables).

## `PromiseWrapper`

Upstream's `PromiseWrapper.ts:1-90`, moved into the runtime with react.md's deltas: `AnyError` (`= any`) is removed and `E` defaults to `unknown`; `wrapPromise`'s wrapper is `PromiseWrapper<T, unknown>` and its `catch` parameter is `unknown`; the comment `// TODO confirm suspense works if the promise is already resolved.` is dropped. `result` stays mutable, because `wrapPromise` writes it. The code is react.md's `PromiseWrapper` snippet, verbatim, at `libs/isograph-runtime/src/PromiseWrapper.ts`.

`readPromise` throws the pending promise, which is React's Suspense protocol. It stays exported because it is upstream's export and reads nothing React-specific; no runtime function calls it. `getPromiseState` is what a runtime caller reads.

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

### The handle

```ts
// from libs/isograph-runtime/src/network.ts
/** One caller's hold on a fetched or retained entrypoint. */
export type NetworkRequestHandle = {
  /** Pending until the response is normalized and the subscriptions are called, then `Ok`;
   * `Err` with the network function's or `normalize`'s error. Resolved at once when nothing
   * was sent. */
  readonly networkRequest: PromiseWrapper<void, unknown>;
  /** Unretains this handle's query, once. Does not collect. */
  readonly dispose: () => void;
};
```

Upstream returns `ItemCleanupPair<PromiseWrapper<void, AnyError>>` (`makeNetworkRequest.ts:66`), the tuple `[item, cleanup]` from `@isograph/disposable-types`, which exists for `@isograph/react-disposable-state`'s `ParentCache` factories. The runtime has no disposable-state dependency, and a named field says which element is which; a React factory writes `[handle.networkRequest, handle.dispose]` (open decision 5).

### `fetchEntrypoint`

```ts
// from libs/isograph-runtime/src/network.ts
import { check } from "./check";
import type { EntityName, ExtractParameters, IsographEntrypoint, ResolverParam } from "./artifacts";
import { type IsographEnvironment, retainQuery, unretainQuery } from "./environment";
import type { RetainedQuery } from "./garbageCollection";
import { normalize } from "./normalize";
import { type PromiseWrapper, wrapPromise, wrapResolvedValue } from "./PromiseWrapper";
import { read } from "./read";
import type { NormalizationAst } from "./artifacts";
import { stableStringify } from "./storeKey";
import type { Variables } from "./store";
import { callSubscriptions, reportCallbackError } from "./subscribe";

/**
 * Sends `entrypoint`'s query with `variables`, or retains it without sending, by
 * `fetchOptions.shouldFetch`. Joins an identical request already in flight.
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
): NetworkRequestHandle {
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
): NetworkRequestHandle {
  let status: NetworkRequestStatus = { kind: "Undisposed", retainedQuery: retainArtifact(environment, ast, variables) };
  return {
    networkRequest: wrapResolvedValue(undefined),
    dispose: () => {
      if (status.kind === "Undisposed") {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: "Disposed" };
      }
    },
  };
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
): NetworkRequestHandle {
  const inFlight = joinOrSendRequest(environment, entrypoint, variables);
  const retainedQuery = retainArtifact(environment, entrypoint.normalizationAst, variables);
  inFlight.retainedQueries.add(retainedQuery);
  let status: NetworkRequestStatus = { kind: "Undisposed", retainedQuery };

  const promise = inFlight.promise.then(
    () => {
      if (status.kind === "Disposed") {
        return;
      }
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

  return {
    networkRequest: wrapPromise(promise),
    dispose: () => {
      if (status.kind === "Undisposed") {
        unretainQuery(environment, status.retainedQuery);
        inFlight.retainedQueries.delete(status.retainedQuery);
        status = { kind: "Disposed" };
      }
    },
  };
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

`read` infers `TEntityName` from `entrypoint.readerArtifact`, and `entrypoint.normalizationAst.root` is a `NormalizationRootRecord<TEntityName>`, which `NoInfer<Link<TEntityName>>` accepts (reader-ast.md, The entrypoint's reader artifact).

Behavior, stated as facts the tests assert:

- `Yes` and `IfNecessary` with `MissingData` call `makeNetworkRequest`; `No` and `IfNecessary` with `EnoughData` call `retainQueryWithoutMakingNetworkRequest`. Every path retains one new `RetainedQuery` `{ ast, variables }` in `environment.retainedQueries`.
- `dispose` unretains that query from `environment.retainedQueries` and, for a sent request, from the in-flight entry. A second `dispose` does nothing. `dispose` never calls `garbageCollect`: collection runs only when the caller calls `garbageCollectEnvironment` (or store.md's `garbageCollect`).
- The response is normalized only when at least one handle joined to the request is undisposed when it arrives (Deduplication). Then `callSubscriptions(environment)` runs, then each undisposed handle's `onComplete` with the resolver's value.
- `onComplete` is called only for a handle that is undisposed when its turn comes. Upstream calls it after disposal too (`makeNetworkRequest.ts:280-297`), when nothing guarantees the store holds what the response wrote; here a disposed handle's query is not retained, so its read is not covered by the retention invariant.
- `onError` is called when the request rejects, for every joined handle, disposed or not, as upstream calls it for every rejection (`:299-307`). The handle's `networkRequest` is then `Err` with the error.
- A callback (`onComplete`, `onError`) that throws does not change the handle's state; its error is reported by `reportCallbackError` (Errors in callbacks).
- A network function that throws synchronously instead of returning a promise propagates out of `fetchEntrypoint`. `joinOrSendRequest` runs before `retainArtifact`, so nothing was retained and no entry was added.

Deltas from `makeNetworkRequest.ts`:

- `maybeMakeNetworkRequest` (`:46-121`) is `fetchEntrypoint`, the public name; upstream exports `makeNetworkRequest` (`index.ts`), which ignores `shouldFetch`, and keeps `maybeMakeNetworkRequest` for its hooks. `makeNetworkRequest` and `retainQueryWithoutMakingNetworkRequest` keep upstream's names and are module-private. No `RefetchQueryNormalizationArtifact` in the artifact union and no `readerWithRefetchQueries` parameter (no refetch queries); no `NormalizationAstLoader` throw (`:85-93`); `check` takes store.md's arguments, and the root is the AST's.
- `retainQueryWithoutMakingNetworkRequest` (`:123-157`) takes the AST, the only part of the entrypoint it reads.
- `makeNetworkRequest` (`:159-340`): the network function receives `{ text: queryText }` and resolves to the data object, so the `errors` check (`:224-231`) and `?? {}` are gone; no optimistic layer (`:190-198`, `:236-251`, `:309-316`, `:326-333`); no logs and no module-level `networkRequestId` counter (`:44`, `:181-182`), which only the logs read; the request is shared through `inFlightRequests` (Deduplication).
- The statuses (`:342-360`): `UndisposedIncomplete` and `UndisposedComplete` are one `Undisposed`. Upstream distinguishes them for the optimistic layer's revert; without it the two behave the same.
- `unretainAndGarbageCollect` (`:565-577`) is `unretainQuery` alone. The user's decision: collection runs only when the caller asks for it. Upstream collects when the release buffer overflows.
- `readDataForOnComplete` (`:362-462`) reads with reader-ast.md's `read`, which calls the resolver; there is no component case and no fake fragment reference. `fetchNormalizationAstAndRetainArtifact` (`:464-493`) is `retainArtifact`: the AST is never loaded lazily.
- Upstream swallows a throwing `onComplete` or `onError` (`try { .. } catch {}`, `:290-296`, `:305-307`); here it is reported (Errors in callbacks).

### Deduplication

```ts
// from libs/isograph-runtime/src/network.ts
/** `stableStringify([queryText, variables])`. */
export type NetworkRequestKey = string;

/** A request sent and not yet settled. */
export type InFlightRequest = {
  /** Settles after the response is normalized and the subscriptions are called, or after
   * the response is dropped because no joined handle is undisposed. Rejects with the network
   * function's or `normalize`'s error. */
  readonly promise: Promise<void>;
  /** The retained query of every handle joined to this request and not disposed. */
  readonly retainedQueries: Set<RetainedQuery>;
};

/** The request in flight for `entrypoint`'s query text and `variables`, sent now if there is none. */
function joinOrSendRequest<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
): InFlightRequest {
  const key: NetworkRequestKey = stableStringify([entrypoint.queryText, variables]);
  const existing = environment.inFlightRequests.get(key);
  if (existing !== undefined) {
    return existing;
  }
  const retainedQueries = new Set<RetainedQuery>();
  const promise = environment.networkFunction({ text: entrypoint.queryText }, variables).then(
    (response) => {
      environment.inFlightRequests.delete(key);
      if (retainedQueries.size === 0) {
        return;
      }
      normalize(environment.store, entrypoint.normalizationAst, response, variables);
      callSubscriptions(environment);
    },
    (error: unknown) => {
      environment.inFlightRequests.delete(key);
      throw error;
    },
  );
  const created: InFlightRequest = { promise, retainedQueries };
  environment.inFlightRequests.set(key, created);
  return created;
}
```

- A `fetchEntrypoint` that would send (`Yes`, or `IfNecessary` with `MissingData`) while a request with the same key is in flight sends nothing and joins it: one network call for any number of such callers. Each joined caller gets its own handle, its own retained query, its own `onComplete` and `onError`, and its own `dispose`.
- The entry is removed when the request settles, before `normalize`, so a `fetchEntrypoint` from inside a subscription callback sends a new request instead of joining a settled one. A later `fetchEntrypoint` with the same key and `IfNecessary` finds the data with `check`; with `Yes` it sends again.
- When every joined handle was disposed before the response arrives, the response is dropped: nothing is normalized and no subscription is called, upstream's `Disposed` behavior (`makeNetworkRequest.ts:235`). A handle that joins after earlier ones were disposed but before the response makes the response normalized again.
- Only one entry per key exists while in flight, so the `delete` at settlement removes that entry and no other.
- Two entrypoints never share a key: the query text names the operation (query-generation.md), so equal text is one entrypoint, and one normalization AST normalizes the response.

The key is upstream's cache key (`getOrCreateCacheForArtifact.ts:42-54`: `operation.text + JSON.stringify(stableCopy(variables))`), written as the stable JSON of a two-item list with store.md's `stableStringify`. Delta: upstream concatenates two strings, so the boundary between query text and variables is not marked; the list form makes the key a function of the pair, and two keys are equal only when both parts are.

Delta from upstream's placement: upstream deduplicates in `@isograph/react`, through the `ParentCache` per key in `environment.fragmentCache`, which also holds the request after it settles for as long as a component retains it (`disposable-state/CacheItem.ts`). The runtime has no retain counts and no React state, so it deduplicates requests in flight only. A React package that wants a settled request shared between renders keeps its own cache over `fetchEntrypoint`.

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

`read` becomes `readButDoNotEvaluate`, then, on `Ok`, `artifact.resolver({ data, parameters: variables, link: root })`; its behavior is unchanged. This is react.md's `readButDoNotEvaluate`, moved here: the subscriptions compare the reader's data, not the resolver's value (Subscriptions). The name is upstream's (`read.ts:60`), which also evaluates nested resolvers and leaves the root's to its caller (`react/useResult.ts:63-64`). The type parameters and the root's type are reader-ast.md's `read`'s.

## Subscriptions

```ts
// from libs/isograph-runtime/src/subscribe.ts
import { selectionSetDataAreEqual } from "./areEqualWithDeepComparison";
import type { EntityName, ReaderArtifact, ResolverParam, SelectionSetData } from "./artifacts";
import type { IsographEnvironment } from "./environment";
import { readButDoNotEvaluate, type ReadMissingData, type ReadOk } from "./read";
import type { Link, StoreLink, Variables } from "./store";

/** What a subscription's callback receives: the reader's data, `MissingData`, or what the
 * re-read threw. */
export type SubscriptionResult = ReadOk<SelectionSetData> | ReadMissingData | ReadErr;

/** A re-read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: "Err";
  readonly error: unknown;
};

export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result the callback last received, or the caller read before subscribing. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
};

/**
 * Re-reads now and calls `callback` when the result differs from `delivered`, then adds the
 * subscription to `environment.subscriptions`. Returns the function that removes it.
 */
export function readAndSubscribe<TEntityName extends EntityName>(
  environment: IsographEnvironment,
  readerArtifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
  delivered: SubscriptionResult,
  callback: (result: SubscriptionResult) => void,
): () => void {
  const subscription: Subscription = { artifact: readerArtifact, root: link, variables, delivered, callback };
  callSubscriptionIfResultChanged(environment, subscription);
  environment.subscriptions.add(subscription);
  return () => {
    environment.subscriptions.delete(subscription);
  };
}

/**
 * Re-reads every subscription in `environment.subscriptions`, in insertion order, and calls
 * each one's callback when its result changed. Never throws.
 */
export function callSubscriptions(environment: IsographEnvironment): void {
  environment.subscriptions.forEach((subscription) => callSubscriptionIfResultChanged(environment, subscription));
}

function callSubscriptionIfResultChanged(environment: IsographEnvironment, subscription: Subscription): void {
  const result = rereadSubscription(environment, subscription);
  if (!subscriptionResultsAreEqual(subscription.artifact, subscription.delivered, result)) {
    subscription.delivered = result;
    reportCallbackError(() => subscription.callback(result));
  }
}

function rereadSubscription(environment: IsographEnvironment, subscription: Subscription): SubscriptionResult {
  try {
    return readButDoNotEvaluate(environment.store, subscription.artifact, subscription.root, subscription.variables);
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

Typing: `readAndSubscribe` ties `readerArtifact` and `link` to one `TEntityName` with reader-ast.md's `read` signature (The parent entity's name): `TEntityName` is inferred from the artifact's declared third type argument, and `NoInfer` keeps the link out of the inference, so a `Link<"Pet">` passed with a `Query` reader fails `tsc`, and so does a `StoreLink`. The stored `Subscription` holds the pair at `EntityName`, because one `Set` holds subscriptions of every entity: `ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>` is assignable to `ReaderArtifact<ResolverParam, unknown, EntityName>` (`TEntityName` does not occur in the body, and `resolver` is compared bivariantly) and `Link<TEntityName>` to `StoreLink`, with no cast. `callSubscriptions` calls `readButDoNotEvaluate` with the pair at `EntityName`, which reader-ast.md's `Resolver` case does too. If `tsc` rejects either assignment, the implementer stops and raises it.

### Semantics

pico-runtime.md's Step A with react.md's three amendments, each holding upstream's behavior:

- `readAndSubscribe` takes `delivered`, the result the caller read, and compares a fresh read with it at once, before adding the subscription (upstream `subscribe.ts:32-36`, `callSubscriptionIfDataChanged` before `add`). A write between the caller's read and the subscribe is delivered. Step A's `subscribe` read `delivered` itself, so such a write was never delivered. A caller with no earlier read passes the result of its own `readButDoNotEvaluate`.
- The callback receives the new `SubscriptionResult` of `readButDoNotEvaluate`, the reader's data, not the root resolver's value. Upstream's callback receives the data (`IsographEnvironment.ts:44-47`) and its caller calls the resolver (`react/useResult.ts:63-64`); a callback that also received the resolver's value would make a caller that renders it run the resolver twice per change. A caller that wants the value calls `readerArtifact.resolver(..)` with the data (open decision 1).
- A `MissingData` re-read is delivered as `MissingData` when `delivered` is not `MissingData`. A subscriber that showed data learns that it is gone (a garbage collection followed by a write, a hand edit), and a subscriber waiting with `delivered` set to `MissingData` is called at the first `Ok`. Upstream's re-read throws a promise on `MissingData` (`read.ts:92-122`), which `logAnyError` catches (`subscribe.ts:47-61`), so upstream's callback is never called with it; with a typed result there is nothing to catch, and dropping it would hide a state change from the caller.

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

`callSubscriptions` visits `environment.subscriptions` in insertion order with `Set.prototype.forEach`, as upstream's `forEach` does (`subscribe.ts:67`), and for each subscription re-reads, compares, and calls its callback before re-reading the next. A callback that removes a subscription not yet visited (its own or another's) removes it from this pass; one that adds a subscription adds it to this pass, where its first comparison is with the result `readAndSubscribe` just compared, so it is not called again unless a write happened in between. Every re-read runs after `normalize` finished and reads the whole store, so no re-read sees a partial write (pico-runtime.md, Step A). A callback that writes the store synchronously (store.md's `normalize` on `environment.store`) changes what later subscriptions in the same pass read; it does not re-run earlier ones, until its own `callSubscriptions`.

No overlap filter: store.md's `normalize` records no `EncounteredIds`, so every subscription is re-read after every write, where upstream skips the subscriptions whose recorded records do not overlap the written ones (`subscribe.ts:74-79`). A subscription whose reader reads only server values is still not called after an unrelated write, because its comparison is equal.

`garbageCollectEnvironment` does not call `callSubscriptions`, as upstream's collection calls none (`garbageCollection.ts:65-86`). A subscription whose records were collected learns of it at the next write, when its re-read is `MissingData`.

### Errors in re-reads and callbacks

Upstream wraps every subscription in `logAnyError` (`subscribe.ts:47-61`, `:67-108`): it catches what a re-read or a callback throws, logs it, and continues with the next subscription. i2 has no log function, so the two cases are handled as follows:

- A re-read that throws (a nested resolver threw; a store invariant broke, store.md's non-link message) becomes `{ kind: "Err", error }`, compared with `delivered` and delivered to that subscription's callback like any other result. The loop continues; nothing escapes `callSubscriptions`, `normalize`'s caller, or the request, whose handles stay `Ok`.
- A callback that throws: `reportCallbackError` rethrows its error in its own microtask (`queueMicrotask`), so it reaches the host's uncaught-error handler (`window.onerror`, Node's `uncaughtException`) without stopping the loop or rejecting the request. The same function reports a throwing `onComplete` or `onError`. Upstream logs and discards these (`logAnyError`; `try { .. } catch {}` at `makeNetworkRequest.ts:290-296`, `:305-307`); with no log, discarding would hide the error (open decision 3).

`delivered` is updated before the callback is called, so a callback that throws has still received the result.

Deltas from Step A (pico-runtime.md) and from react.md's amended Step A:

- `subscribe` is `readAndSubscribe`, takes the environment, the reader artifact, the link, the variables, `delivered`, and the callback, and returns the unsubscribe function.
- `callSubscriptions` takes the environment, since the subscriptions live there, and never throws. pico-runtime.md's open decision 9 (a), rethrow after the loop, held in react.md for the re-read error, is replaced by delivery as `Err` (upstream's isolation, without the log).
- The callback receives `SubscriptionResult`, `MissingData` and `Err` included; react.md's amendment called nothing on `MissingData`.

## Errors

Every throw this doc adds, and its invariant:

- `readDataForOnComplete` on `MissingData`: "Reading `<name>` right after normalizing its response found missing data: `<reason>`. This is indicative of a bug in Isograph." It rejects that handle's `networkRequest`. Invariant: the entrypoint's reader reads only what its query selects (query-generation.md), `normalize` wrote all of it or threw, and the handle's query is retained, so `garbageCollectEnvironment` keeps it. Only a caller's direct store-only `garbageCollect` with a list that leaves the query out, run from a subscription callback between the write and `onComplete`, breaks it, and that call breaks the retention contract.
- store.md's `normalize` throws on a malformed response; in `joinOrSendRequest` that rejects the request, so every joined handle is `Err` with the message, every `onError` is called, and no subscription is called. The writes made before the throw stay (store.md open decision 9).
- The network function's rejection: every joined handle is `Err` with that error, and every `onError` is called. The store is unchanged.
- A throwing callback: reported in its own microtask (Errors in re-reads and callbacks).

## Tests

vitest, in store.md's test infrastructure, over store.md's fixture project with reader-ast.md's additions (`libs/isograph-runtime/fixtures/store`), whose `entrypoint.ts` artifacts are `IsographEntrypoint`s with typed reader artifacts (reader-ast.md change 3). No snapshots; tests assert handle states, network calls, callback calls and arguments, store contents, and the sizes of the environment's sets. No test sleeps or polls (AGENTS.md, Wake on events): the network function returns a promise the test settles, and the test awaits the handle's promise.

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

`deferredNetwork` builds each promise with `Promise.withResolvers` (ES2024, Node 22) if the package's `lib` admits it, else `new Promise((resolve, reject) => ..)`; the fixture's code is plain test setup and the choice is the implementer's.

### `test/environment.test.ts`

- `createIsographEnvironment(store, f)`: `store` is the same object, and the three collections are empty.
- `retainQuery` twice with two equal `{ ast, variables }` objects: `retainedQueries.size` is 2; `unretainQuery` of one leaves 1.
- `garbageCollectEnvironment` with `PetDetail` normalized and retained: the store keeps `PetDetail`'s records and deletes a `Pet:9` that `PetLists` wrote; with nothing retained, the store equals `new Map()`.

### `test/network.test.ts`

shouldFetch:

- `IfNecessary` on an empty store: one call, with `{ text: petDetail.queryText }` and the variables; `retainedQueries.size` is 1; the handle is `Pending`. After `resolve`, the handle is `Ok`, `check` is `EnoughData`, and `inFlightRequests.size` is 0.
- `IfNecessary` after the same response was normalized: no call; the handle is `Ok` at once; `retainedQueries.size` is 1.
- `Yes` with the data present: one call. `No` on an empty store: no call, the handle is `Ok` at once, one retained query, and the store stays empty. `null` options behave as `IfNecessary`.

Deduplication:

- Two `fetchEntrypoint` calls with `PetDetail` and equal variables (two objects): one call; `inFlightRequests.size` is 1; two retained queries; after `resolve`, both handles are `Ok` and both `onComplete`s have one call with the resolver's value.
- The same with variables `{ id: "1", skip: null }` and `{ id: "2", skip: null }`: two calls.
- `Yes` while an `IfNecessary` request with the same key is in flight: no second call.
- After the first request settles, `Yes` with the same key: a second call.
- Degenerate: variables whose keys are in different orders (`{ skip: null, id: "1" }`) share the key.

Dispose:

- Dispose before the response: `retainedQueries.size` is 0; after `resolve`, the store equals `new Map()` (the response was dropped), the handle is `Ok`, and `onComplete` has no call.
- Two joined handles, the first disposed before the response: the response is normalized, the second `onComplete` has one call, the first none.
- Two joined handles, both disposed, then a third joins before the response: the response is normalized.
- Dispose after the response: `retainedQueries.size` is 0, and the store still holds the records (no collection); `garbageCollectEnvironment` then empties it. Disposing twice changes nothing.
- Two requests (`PetDetail` for `Pet:1`, `PetLists`), the second disposed, then `garbageCollectEnvironment`: `Pet:9` is deleted and `Pet:1` stays.

Errors:

- `reject(error)`: the handle is `Err` with `error`; `onError` has one call; the store is unchanged; `inFlightRequests.size` is 0. Two joined handles: both `Err`, both `onError`s called.
- A response missing `pet____id___v_id`: the handle is `Err` with store.md's malformed-response message; `onError` has one call; a subscription has no call.
- An `onComplete` that throws: the handle is `Ok`, and the error is reported (`vi.stubGlobal("queueMicrotask", spy)`: the spy received a function that throws that error).
- A network function that throws synchronously: `fetchEntrypoint` throws it; `retainedQueries.size` and `inFlightRequests.size` are 0.

### `test/subscribe.test.ts`

Each test builds an environment over `deferredNetwork` and writes through `fetchEntrypoint` (or `normalize` on `environment.store` followed by `callSubscriptions`).

- `readAndSubscribe` with `delivered` equal to the current read: no call. With a `delivered` read before a `normalize` that changed `Pet:1`'s `name`: one synchronous call, inside `readAndSubscribe`, with the new `Ok` result.
- Related and unrelated writes, `PetDetail` (only `Scalar` and `Linked` nodes): a `PetDetail` response that changes `Pet:1`'s `name`: one call with the new data, and `delivered` is that result. A `PetLists` response writing only `Pet:9`: no call. The same `PetDetail` response again: no call.
- `PetSummary` (nested resolvers that return `data` objects): a `PetLists` response writing only `Pet:9`: one call, with data equal in value to the last delivered. This pins v1's `===` rule on nested resolver values.
- Equality by node: `Leaves` re-normalized with equal `tags: ["a", null]` (a new list): no call; with `settings` an equal `JSON` object (a new object): one call; a refinement in `Feed` that stays `null`: no call.
- `MissingData` then data: a subscription with `delivered` a `MissingData` read of an empty store: no call on subscribe; the response arrives: one call with `Ok`.
- Data then `MissingData`: `Pet:1`'s `nickname` deleted by hand, then `callSubscriptions`: one call with `MissingData` whose `recordLink` is `Pet:1`; `callSubscriptions` again: no call; a write that restores it: one call with `Ok`.
- Unsubscribe: after the returned function runs, `subscriptions.size` is 0 and a changing write calls nothing. Calling it twice changes nothing.
- Re-read errors: a nested resolver that throws on its second read (a fixture field whose resolver throws when `data.name` is `"Throw"`): the subscription's callback is called once with `{ kind: "Err", error }`; a second subscription that reads healthy data is still called; the request's handle is `Ok`. The same throw again (a new `Error` object): one more call. `Pet:1.stats` set by hand to `"x"`: `Err` with store.md's non-link message.
- A callback that throws: the other subscriptions are still called, `callSubscriptions` returns, and the error is reported through `queueMicrotask` (stubbed as above).
- Ordering: three subscriptions all changed by one write are called in subscription order. A callback that unsubscribes the third: the third has no call. A callback that unsubscribes itself: the others are still called.
- Degenerate: `callSubscriptions` with no subscriptions does nothing; a reader whose AST is empty (`Constant` at `Pet:1`) is `Ok` with `{}` and is never called after a write.

The fixture gains one source file for the throwing resolver:

```text
field Pet.ThrowingName { name }        throws when data.name is "Throw", else returns data.name
field Query.ThrowingPet($id: ID!) {
  pet(id: $id) { ThrowingName }
}
```

with `entrypoint Query.ThrowingPet`.

## Changes

Each change is one commit, in order. Prerequisites: store.md changes 1 to 7 and reader-ast.md changes 1 to 6.

### 1. `readButDoNotEvaluate` (libs/isograph-runtime)

`readButDoNotEvaluate` in `src/read.ts`; `read` calls it; its export; `read.test.ts` additions: `readButDoNotEvaluate` of `PetSummary` returns the data `read` passes to the root resolver, with a spy on the root resolver having no call and spies on `PetName` and `RecentCheckins` one call each; on an absent root it is `MissingData` with `No record for Query:__ROOT`.

### 2. The environment (libs/isograph-runtime)

`src/environment.ts` with `IsographEnvironment`, `IsographNetworkFunction`, `IsographOperation`, `createIsographEnvironment`, `retainQuery`, `unretainQuery`, `garbageCollectEnvironment`; `InFlightRequest` and `NetworkRequestKey` as types in `src/network.ts`; `Subscription` as a type in `src/subscribe.ts`; `ExtractParameters` in `src/artifacts.ts`; their exports; `test/environment.test.ts`; `deferredNetwork` in `test/fixtures.ts`.

### 3. Subscriptions (libs/isograph-runtime)

`src/areEqualWithDeepComparison.ts`; `src/subscribe.ts` with `SubscriptionResult`, `ReadErr`, `readAndSubscribe`, `callSubscriptions`, `reportCallbackError`; their exports; the `ThrowingName` fixture source and its regenerated artifacts; `test/subscribe.test.ts`, writing with `normalize` on `environment.store` then `callSubscriptions`. Depends on 1 and 2.

### 4. The network path (libs/isograph-runtime)

`src/PromiseWrapper.ts`; `src/network.ts` with `ShouldFetch`, `DEFAULT_SHOULD_FETCH_VALUE`, `FetchOptions`, `NetworkRequestHandle`, `fetchEntrypoint`, and the private functions; their exports; `test/network.test.ts`; `test/subscribe.test.ts` cases that write through `fetchEntrypoint`. Depends on 3.

## Docs this doc amends

- `refactors/pending/runtime/react.md`: not part of this work. Its environment, network function, `ExtractParameters`, `PromiseWrapper`, fetch options, retention, `maybeMakeNetworkRequest` and `makeNetworkRequest`, `readButDoNotEvaluate`, and amended Step A are this doc's, in `@isograph/runtime`; the React package imports them. Disposal unretains and does not collect, so its open decision about when collection runs is the React package's choice of when to call `garbageCollectEnvironment`. Its request cache is a React cache over `fetchEntrypoint`'s handles.
- `refactors/pending/runtime/pico-runtime.md`, Step A: `subscribe` is `readAndSubscribe(environment, readerArtifact, link, variables, delivered, callback)`, `callSubscriptions(environment)` never throws, and the callback receives `SubscriptionResult` (Subscriptions). Its open decision 10 is taken as (a). Its open decision 9 (a) does not apply to Step A: a re-read error is delivered as `Err`.
- `refactors/pending/runtime/store.md`: its "No `IsographEnvironment`" and "No subscriptions" (Removed from upstream) hold for the store functions; the environment and subscriptions are this doc's, composed over them.
- `refactors/pending/runtime/reader-ast.md`: `read` is `readButDoNotEvaluate` then the resolver.

## Later

- A network function that takes an `AbortSignal`, so that disposing every joined handle cancels the request (pico-runtime.md, Follow-ups 6).
- An overlap filter on `callSubscriptions`, with `normalize` recording the records it wrote (upstream's `EncounteredIds`).
- Replacing `readAndSubscribe` with pico-runtime.md's level 2 (`listen`, `snapshot`).
- `writeData` (upstream `writeData.ts:23-79`), a second write site that calls `callSubscriptions`.

## Open decisions

### 1. What the subscription callback receives

- (a) `SubscriptionResult` of `readButDoNotEvaluate`: the reader's data (held). Upstream's shape (`IsographEnvironment.ts:44-47`); a caller that renders the resolver's value calls it once, in its own render. Cost: a non-React caller calls `readerArtifact.resolver(..)` itself.
- (b) The root resolver's value. Cost: the resolver runs inside `callSubscriptions` for every changed subscription, and again in a React render that calls it; the comparison still has to run on the data, so the subscription holds both.

### 2. When two `MissingData` results are equal

- (a) Always (held). A caller waiting for data acts only on `Ok`.
- (b) When `recordLink` and `reason` are equal. Cost: a caller is called each time the first missing value moves, which no caller in this work reads.

### 3. A callback that throws

- (a) Rethrow in its own microtask (held). The loop and the request continue, and the error reaches the host's uncaught-error handler. Cost: the error surfaces with no stack frame of the write that caused it, and a vitest run fails on it unless the test stubs `queueMicrotask`.
- (b) Swallow it, upstream's behavior without its log. Cost: the error is invisible.
- (c) Rethrow after the loop. Cost: the request whose response was normalized becomes `Err`, and every joined handle reports a failure the network did not have.

### 4. A joined handle's fetch options

- (a) Each handle's own `onComplete` and `onError` (held).
- (b) The first caller's, upstream's outcome: its factory closes over the first options (`getOrCreateCacheForArtifact.ts:55-84`), so a second component's `onComplete` with the same key is not called. Cost: a caller's callback is silently not called.

### 5. The handle's shape

- (a) `{ networkRequest, dispose }` (held). Each element is named, and the runtime has no `@isograph/disposable-types` dependency.
- (b) Upstream's `ItemCleanupPair<PromiseWrapper<void, unknown>>` tuple. Cost: a runtime dependency on `@isograph/disposable-types` for one type; the React plan's `ParentCache` factories take the tuple as is.

### 6. `Yes` while an identical request is in flight

- (a) Join it (held). Upstream's React cache gives the second caller the same request whatever its options (`getOrCreateCacheForArtifact.ts:42-84`).
- (b) Send a second request. Cost: two responses for one query in flight, normalized in arrival order; `Yes` then means "send", as `conditional-fetching.md` words it.
