# Runtime: the React package

Status: not part of the current work, and not approved for implementation. The current work is `@isograph/runtime` with no React (`refactors/pending/runtime/network-and-subscriptions.md`): `normalize`, `check`, `garbageCollect`, `read`, `readButDoNotEvaluate`, `readAndSubscribe`, `callSubscriptions`, `fetchEntrypoint`, and the environment, all plain functions. This doc is a later plan built on those functions. The environment, the network function, `PromiseWrapper`, the fetch options, retention, the network path, `readButDoNotEvaluate`, and the subscriptions moved out of this doc into network-and-subscriptions.md; this doc keeps the React hooks, components, the React request cache, the disposable-state packages, and the demo. The sections below that call the runtime are written against network-and-subscriptions.md's signatures; where they still need a decision, it is an open decision below.

`@isograph/react` (`libs/isograph-react`) is the React package over `@isograph/runtime`. It holds upstream's hooks and components for fetching an entrypoint and reading its result: `IsographEnvironmentProvider`, `useIsographEnvironment`, `useLazyReference`, `useImperativeReference`, `useResult`, and `FragmentReader`. `@isograph/runtime` stays React-free (normalization-ast.md, The `@isograph/runtime` package). The package name is upstream's (`isograph/libs/isograph-react/package.json`); it is React-specific, so the name fits here.

The user's direction: upstream's APIs are held as they are (names, signatures, behavior, `shouldFetch`), except where AGENTS.md's rules force a change (the two reader options are enums, Reading). Every change below names upstream's code (`path:line`), the v1 decision that forces it, and the minimal adaptation. Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; the `core/` and `react/` files cited are unchanged since `79e2a3c0`, and the disposable-state citations are of `9d72f6fc`). Paths without a directory are under `isograph/libs/isograph-react/src`; `disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`.

Prerequisites, in their own docs:

- `refactors/pending/graphql-protocol/normalization-ast.md` changes 1 to 4.
- `refactors/pending/runtime/store.md` changes 1 to 7: `Store`, `StoreLink` (`{ entityName, id }`), `Link`, `Variables`, `normalize`, `check`, `garbageCollect`, `RetainedQuery`, `stableStringify`, required parameters and input object fields.
- `refactors/pending/runtime/reader-ast.md` changes 1 to 6: `ReaderArtifact`, `ResolverParam`, `SelectionSetData`, `read`, `ReadResult`, `ReadMissingData`, real `resolver_reader.ts` artifacts, the typed `IsographEntrypoint.readerArtifact`, `NamedImport`, `DeclaredConst`, `ExportDefaultBinding`.
- `refactors/pending/runtime/network-and-subscriptions.md` changes 1 to 6: `IsographEnvironment`, `createIsographEnvironment`, `IsographNetworkFunction`, `ExtractParameters`, `PromiseWrapper` and its functions, `FetchOptions`, `ShouldFetch`, `fetchEntrypoint`, `getOrCreateCacheForArtifact`, `garbageCollectEnvironment`, `readButDoNotEvaluate`, `readAndSubscribe`, `subscriptionSnapshot`, `SubscriptionResult`; `@isograph/disposable-types` with `CacheItem` and `ParentCache`.

The demo's components below are plain React components that read resolver results with `FragmentReader`; `refactors/pending/runtime/components-in-userland.md` explores components returned by fields, and if it lands with another shape, the demo follows it.

## End-user experience

The pet demo renders `HomeRoute` and `PetDetailRoute` in a browser (`pnpm --filter pet-demo dev`) with a mock network function and no server:

```tsx
// from demos/pet-demo/src/App.tsx
import { IsographEnvironmentProvider } from '@isograph/react';
import {
  createIsographEnvironment,
  type IsographNetworkFunction,
} from '@isograph/runtime';
import { useState } from 'react';
import { HomeRouteLoader } from './HomeRouteLoader';
import { PetDetailRouteLoader } from './PetDetailRouteLoader';
import type { Route } from './routes';

export function App({
  networkFunction,
}: {
  readonly networkFunction: IsographNetworkFunction;
}) {
  const [environment] = useState(() =>
    createIsographEnvironment(new Map(), networkFunction),
  );
  const [route, setRoute] = useState<Route>({ kind: 'Home' });
  return (
    <IsographEnvironmentProvider environment={environment}>
      {route.kind === 'Home' ? (
        <HomeRouteLoader navigateTo={setRoute} />
      ) : (
        <PetDetailRouteLoader id={route.id} navigateTo={setRoute} />
      )}
    </IsographEnvironmentProvider>
  );
}
```

```tsx
// from demos/pet-demo/src/HomeRouteLoader.tsx
import { FragmentReader, useLazyReference } from '@isograph/react';
import { Suspense } from 'react';
import { iso } from './__isograph/iso';
import type { Route } from './routes';

export function HomeRouteLoader({
  navigateTo,
}: {
  readonly navigateTo: (route: Route) => void;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.HomeRoute`),
    {},
  );
  return (
    <Suspense fallback={<p>Loading pets...</p>}>
      <FragmentReader fragmentReference={fragmentReference}>
        {(pets) => (
          <ul>
            {pets.map((pet) => (
              <li key={pet.id}>
                <button
                  onClick={() => navigateTo({ kind: 'PetDetail', id: pet.id })}
                >
                  {pet.summary}
                </button>
              </li>
            ))}
          </ul>
        )}
      </FragmentReader>
    </Suspense>
  );
}
```

```tsx
// from demos/pet-demo/src/PetDetailRouteLoader.tsx
import { FragmentReader, useLazyReference } from '@isograph/react';
import { Suspense } from 'react';
import { iso } from './__isograph/iso';
import type { Route } from './routes';

export function PetDetailRouteLoader({
  id,
  navigateTo,
}: {
  readonly id: string;
  readonly navigateTo: (route: Route) => void;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailRoute`),
    { id },
  );
  return (
    <>
      <button onClick={() => navigateTo({ kind: 'Home' })}>Home</button>
      <Suspense fallback={<p>Loading pet...</p>}>
        <FragmentReader fragmentReference={fragmentReference}>
          {(pet) =>
            pet == null ? (
              <p>No pet with that id</p>
            ) : (
              <section>
                <h1>{pet.fullName}</h1>
                <p>Intelligence: {pet.intelligence}</p>
                <p>Best friend: {pet.bestFriend}</p>
                <ul>
                  {pet.checkins.map((location, index) => (
                    <li key={index}>{location}</li>
                  ))}
                </ul>
              </section>
            )
          }
        </FragmentReader>
      </Suspense>
    </>
  );
}
```

```ts
// from demos/pet-demo/src/routes.ts
export type Route =
  | { readonly kind: 'Home' }
  | { readonly kind: 'PetDetail'; readonly id: string };
```

What the user sees:

- The home page shows "Loading pets..." for one render, then one button per pet, "Makayla Balicki: I AM HUNGRY" and "Mr Biggles: Pet me now, human". The mock network function was called once, with `HomeRoute`'s query text and `{}`.
- Clicking a pet shows "Loading pet...", then the pet's full name, intelligence, best friend's first name ("Nobody" without one), and check-in locations, from the nested resolvers `fullName`, `PetStatsCard`, `PetBestFriendCard`, and `PetCheckinsCard`.
- Clicking "Home" fetches `HomeRoute` again when the React package collects at disposal (open decision 1): unmounting `HomeRouteLoader` released its query, and `garbageCollectEnvironment` collected the pets it fetched before `PetDetailRoute`'s response arrived. Releasing a request alone collects nothing (network-and-subscriptions.md).
- Rendering the same entrypoint with the same variables twice sends one request.

The demo's two resolvers return data the components read:

```ts
// from demos/pet-demo/src/HomeRoute.ts
import { iso } from './__isograph/iso';

export const HomeRoute = iso(`
  field Query.HomeRoute {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRoute({ data }) {
  return data.pets.map((pet) => ({ id: pet.id, summary: pet.PetSummaryCard }));
});

console.log(iso(`entrypoint Query.HomeRoute`));
```

```ts
// from demos/pet-demo/src/PetDetailRoute.ts
import { iso } from './__isograph/iso';

export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) {
    pet(id: $id) {
      id
      fullName
      PetStatsCard
      PetCheckinsCard(limit: 5)
      PetBestFriendCard
      bestFriend {
        fullName
      }
      owner {
        OwnerBadge
      }
    }
  }
`)(function PetDetailRoute({ data }) {
  const pet = data.pet;
  return pet == null
    ? null
    : {
        fullName: pet.fullName,
        intelligence: pet.PetStatsCard,
        bestFriend: pet.PetBestFriendCard,
        checkins: pet.PetCheckinsCard,
      };
});

console.log(iso(`entrypoint Query.PetDetailRoute`));
```

Before, `HomeRoute` returned `data.pets.map((pet) => pet.PetSummaryCard)` and `PetDetailRoute` returned `data.pet?.fullName ?? 'No pet with that id'`. The iso literals are unchanged, so no artifact changes: each `output_type.ts` is `ReturnType<typeof resolver>`.

## Upstream's exports and what v1 does with each

From `index.ts:1-183`. Keep means the name, signature, and behavior are upstream's. Adapt names the forcing decision; the sections below give the code. Drop names the reason. Types and functions that move to `@isograph/runtime` are not re-exported from `@isograph/react`: each name has one home, and users import runtime names from `@isograph/runtime`.

### Kept or adapted, in `@isograph/react`

- `IsographEnvironmentProvider`, `useIsographEnvironment`, `IsographEnvironmentProviderProps` (`react/IsographEnvironmentProvider.tsx:6-34`): kept.
- `useLazyReference` (`react/useLazyReference.ts:19-60`): adapted (Fetching with `useLazyReference`).
- `useImperativeReference`, `UseImperativeReferenceResult` (`react/useImperativeReference.ts:25-100`): adapted.
- `useResult` (`react/useResult.ts:16-67`): adapted. Only the eager case remains.
- `useReadAndSubscribe` (`react/useReadAndSubscribe.ts:25-43`): adapted; it reads through `useSyncExternalStore` over the runtime's subscription instead of React state, and reads again when the fragment reference changes.
- `FragmentReader` (`react/FragmentReader.tsx:10-29`): adapted type parameters.
- `FragmentReference`, `stableIdForFragmentReference`, `StableIdForFragmentReference` (`core/FragmentReference.ts:35-79`): adapted.
- `FragmentReferenceOfEntrypoint`, `ExtractReadFromStore`, `ExtractClientFieldValue` (`core/entrypoint.ts:95-100`, `:200-203`): adapted to two type parameters.
- `NetworkRequestReaderOptions` (`core/read.ts:972-975`): adapted. Its two booleans are the enums `whileInFlight` and `onNetworkError` (Reading).

### Moved to `@isograph/runtime`

- `check`, `CheckResult` (`core/check.ts:34-70`): store.md's `check`.
- `normalizeData`, `NetworkResponseObject`, `NetworkResponseValue`, `NetworkResponseScalarValue` (`core/cache.ts`): store.md's `normalize`, `JsonObject`, `JsonValue`.
- `subscribe`, `callSubscriptions` (`core/subscribe.ts:15-109`): network-and-subscriptions.md's `readAndSubscribe` and `callSubscriptions`.
- `readButDoNotEvaluate`, `ReadDataResult` (`core/read.ts:60-143`): network-and-subscriptions.md's `readButDoNotEvaluate`, returning reader-ast.md's `ReadResult<SelectionSetData>`.
- `IsographEnvironment`, `createIsographEnvironment`, `IsographNetworkFunction`, `IsographOperation` (`core/IsographEnvironment.ts:73-106`, `react/createIsographEnvironment.ts:10-23`, `core/entrypoint.ts:59-62`): network-and-subscriptions.md's (The environment, The network function).
- `makeNetworkRequest`, `FetchOptions`, `ShouldFetch` (`core/makeNetworkRequest.ts:46-340`, `core/check.ts:13-27`): network-and-subscriptions.md's `fetchEntrypoint`, `FetchOptions`, `ShouldFetch`.
- `retainQuery`, `unretainQuery`, `garbageCollectEnvironment` (`core/garbageCollection.ts:39-86`): network-and-subscriptions.md's.
- `PromiseWrapper`, `readPromise`, `getPromiseState`, `wrapResolvedValue`, `wrapPromise`, `PromiseState`, `Result`, `NotSet`, `NOT_SET` (`core/PromiseWrapper.ts:1-90`): network-and-subscriptions.md's.
- `ExtractParameters` (`core/FragmentReference.ts:29-33`): network-and-subscriptions.md's.
- `RetainedQuery` (`core/garbageCollection.ts:20-24`): store.md's `{ ast, variables }`.
- `StoreLink`, `StoreRecord`, `TypeName`, `DataId`, `DataTypeValueLinked`, `DataTypeValueScalar`, `IsographStore` (`core/IsographEnvironment.ts:122-175`): store.md's `StoreLink`, `StoreRecord`, `EntityName`, `RecordId`, `StoreValue`, `Store`.
- `Variables`, `VariableValue` (`core/FragmentReference.ts:10-21`): store.md's `Variables`, `JsonValue`.
- `UnknownTReadFromStore` (`core/FragmentReference.ts:23-27`): reader-ast.md's `ResolverParam<TEntityName>`, which every type parameter below is constrained by.
- `EagerReaderArtifact`, `TopLevelReaderArtifact`, `ReaderAst`, `ReaderAstNode`, `ReaderLinkedField`, `ReaderScalarField`, `ReaderNonLoadableResolverField`, `ResolverFirstParameter` (`core/reader.ts`): reader-ast.md's reader types.
- `NormalizationAst`, `NormalizationAstNode`, `NormalizationAstNodes`, `NormalizationLinkedField`, `NormalizationScalarField`, `NormalizationInlineFragment`, `Argument`, `ArgumentName`, `ArgumentValue`, `Arguments` (`core/entrypoint.ts`, `core/util.ts`): normalization-ast.md's types.
- `IsographEntrypoint` (`core/entrypoint.ts:77-93`): adapted, in `@isograph/runtime`'s `artifacts.ts`, landed with normalization-ast.md (The entrypoint artifact type).

### Dropped

- `useRerenderOnChange` (`react/useRerenderOnChange.ts:10-38`): `useSyncExternalStore`'s `subscribe` does its job, and the data it set in React state lives only in the runtime (Reading, `useReadAndSubscribe`).
- `ComponentReaderArtifact`, `FragmentRenderer`, `IsExactlyIntrinsicAttributes`, `ExtractProps`, `ExtractSecondParam`, `CombineWithIntrinsicAttributes`, `ComponentOrFieldName`, `FieldCache`: v1 has no `@component` (reader-ast.md). `FragmentRenderer` renders a resolver result as a component; upstream gives each fragment reference one stable component through `getOrCreateCachedComponent` (`core/componentCache.ts`), and without it a resolver that returns a new function on each read would remount on each render.
- `LoadableFieldReader`, `LoadableFieldRenderer`, `LoadableField`, `ReaderLoadableField`, `useClientSideDefer`, `useImperativeLoadableField`, `UseImperativeLoadableFieldReturn`, `useSkipLimitPagination`, `useConnectionSpecPagination` and their types, `NormalizationAstLoader`, `IsographEntrypointLoader`, `RequiredFetchOptions`, `RequiredShouldFetch`: v1 has no `@loadable` and no lazily loaded artifacts.
- `useImperativeExposedMutationField`, `UseImperativeExposedMutationFieldReturn`: v1 has no directives (`@exposeField`).
- `RefetchReaderArtifact`, `RefetchQueryNormalizationArtifact`, `RefetchQueryNormalizationArtifactWrapper`, `ReaderWithRefetchQueries`, `ReaderImperativelyLoadedField`, `NetworkRequestInfo`: v1 has no refetch queries.
- `ReaderLinkField`, `StableId`: v1 has no `__link` selectable and no pointers (reader-ast.md). Upstream's `Link<T>` (`IsographEnvironment.ts:117-120`) is store.md's `Link<TEntityName>`, without the brand on the id.
- `StartUpdate`, `ExtractStartUpdate`: v1 has no `@updatable`.
- `ROOT_ID`, `createIsographStore`: the root record comes from the AST, and the empty store is `new Map()` (store.md).
- `MissingFieldHandler`: store.md, Later.
- `EncounteredIds`, `WithEncounteredRecords`: store.md's `normalize` records no encountered ids.
- `Subscriptions`, `Subscription` (environment union), `FragmentSubscription`, `AnyChangesToRecordSubscription`, `AnyRecordSubscription`: the environment holds network-and-subscriptions.md's `Subscription`; nothing subscribes to a record.
- `CacheMap`, `StringifiedArgs`: the React request cache is a `Map` (Fetching with `useLazyReference`); nothing reads stringified arguments.
- `DidUnretainSomeQuery`: no release buffer.
- `IsographPersistedOperation`, `IsographPersistedOperationExtraInfo`: i2 has no persisted operations.
- `assertIsEntrypoint`: entrypoint artifacts are declared `IsographEntrypoint` (change 3); upstream's only caller is its export.
- `ExtractResolverResult`, `ExtractData`, `ExtractRawResponseType`: no reader in i2; `TRawResponseType` is gone with `optimisticNetworkResponse`.
- `useSubscribeToMultiple`: its only upstream callers are the pagination hooks.
- `writeData`: no caller in v1. Later.
- `RenderAfterCommit__DO_NOT_USE`: a Next.js debugging aid with no caller in i2.
- `LogMessage`, `LogFunction`, `WrappedLogFunction`, `logMessage`, `registerLogger`: no log events (store.md, Removed from upstream).
- `AnyError`: `any` is banned (AGENTS.md, TypeScript); errors are `unknown`.

## v1 decisions that force adaptations

- No `@component`, `@loadable`, or directives: `useResult` has one case (`useResult.ts:28-36` removed); `FragmentReference.readerArtifactKind` is dropped; `useReadAndSubscribe` re-reads when its fragment reference changes, because upstream relied on a component per fragment reference to remount (`useRerenderOnChange.ts:31-36`).
- No refetch queries and no lazily loaded readers: `FragmentReference.readerWithRefetchQueries` (a `PromiseWrapper`, `FragmentReference.ts:55-57`) is `readerArtifact`, the artifact itself; `getOrLoadReaderWithRefetchQueries` (`IsographEnvironment.ts:255-294`) is gone.
- No store layers: `optimisticNetworkResponse` and every optimistic branch of `makeNetworkRequest` (`:190-198`, `:236-251`, `:495-563`) are dropped.
- No release buffer: releasing a request unretains its query and does not collect (network-and-subscriptions.md); when the React package calls `garbageCollectEnvironment` is open decision 1.
- No log events: every `logMessage` call is dropped (`useLazyReference.ts:42-48`).
- Store shape: the environment holds store.md's `Store` (a `Map`), and the root record is `entrypoint.normalizationAst.root` (`{ entityName, id }`), not `{ __link: ROOT_ID, __typename: entrypoint.concreteType }` (`getOrCreateCacheForArtifact.ts:77`).
- Links `{ entityName, id }`, typed `Link<TEntityName>`: `FragmentReference.root` and the stable id read `entityName` and `id` (`FragmentReference.ts:78`), and every hook over a reference carries the reference's `TEntityName`.
- `read` returns `MissingData` instead of throwing: the runtime's `readButDoNotEvaluate` returns a `ReadResult`, and the React package turns `MissingData` into the thrown promise upstream's `readButDoNotEvaluate` throws (`read.ts:92-122`).
- The suspend-until-change promise and re-render on write use network-and-subscriptions.md's `readAndSubscribe`, whose callback receives `Ok`, `MissingData`, or `Err` (Reading).
- Required variables (store.md, Variables): the hooks' `variables: ExtractParameters<TReadFromStore>` is unchanged; the generated parameters type requires every variable, so a caller passes nullable ones too.
- A protocol-neutral runtime with no `any`: the network function resolves to the operation's data object (network-and-subscriptions.md, The network function), and the React package reads no `errors` or `data` key.

## Packages

```text
libs/isograph-disposable-types/       @isograph/disposable-types (port)
libs/isograph-react-disposable-state/ @isograph/react-disposable-state (port)
libs/isograph-react/                  @isograph/react
  package.json
  tsconfig.json
  vitest.config.ts
  src/index.ts
  src/core/entrypoint.ts
  src/core/FragmentReference.ts
  src/core/read.ts
  src/react/FragmentReader.tsx
  src/react/IsographEnvironmentProvider.tsx
  src/react/maybeUnwrapNetworkRequest.ts
  src/react/useImperativeReference.ts
  src/react/useLazyReference.ts
  src/react/useReadAndSubscribe.ts
  src/react/useResult.ts
  src/react/useSubscription.ts
  src/react/useSubscriptionSnapshot.ts
  test/setup.ts
  test/fixtures.tsx
  test/useLazyReference.test.tsx
  test/useImperativeReference.test.tsx
  test/useResult.test.tsx
  fixtures/pets/isograph.config.json
  fixtures/pets/schema/schema.graphql
  fixtures/pets/src/*.ts
  fixtures/pets/src/__isograph/**   (generated)
```

`src/core/` holds the React-free code and `src/react/` the React code, as upstream does. The network layer and the environment are `@isograph/runtime`'s (network-and-subscriptions.md); `src/core/` keeps the fragment reference, the request cache, and the reader options.

```json
// from libs/isograph-react/package.json
{
  "name": "@isograph/react",
  "description": "Use Isograph with React.",
  "private": true,
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/isographlabs/isograph.git",
    "directory": "libs/isograph-react"
  },
  "types": "./src/index.ts",
  "exports": {
    ".": {
      "types": "./src/index.ts",
      "import": "./src/index.ts"
    }
  },
  "scripts": {
    "typecheck": "tsc --noEmit",
    "test": "vitest run"
  },
  "dependencies": {
    "@isograph/disposable-types": "workspace:*",
    "@isograph/react-disposable-state": "workspace:*",
    "@isograph/runtime": "workspace:*"
  },
  "peerDependencies": {
    "react": "^18.0.0 || ^19.0.0"
  },
  "devDependencies": {
    "@babel/core": "^7.28.3",
    "@isograph/babel-plugin": "workspace:*",
    "@types/react": "18.3.1",
    "@types/react-test-renderer": "^18.3.0",
    "react": "18.3.1",
    "react-test-renderer": "^18.2.0",
    "typescript": "5.6.3",
    "vitest": "^2.1.2"
  },
  "sideEffects": false
}
```

Delta from upstream's `package.json`: `private`, source exports with `types` and `import` conditions instead of `dist` builds (the runtime's form, normalization-ast.md), no `@isograph/reference-counted-pointer` (its only callers are the pagination hooks), the test-only Babel and vitest dependencies, and `@types/react-test-renderer` for typechecking tests. `react` and `@types/react` are upstream's versions; the peer range is upstream's.

`@isograph/runtime`'s `exports` gains `"import": "./src/index.ts"` (change 1): `@isograph/react` imports runtime values. normalization-ast.md anticipated this ("The runtime change in pico-runtime.md adds the `import` condition with its code").

```json
// from libs/isograph-react/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noUncheckedIndexedAccess": true,
    "noPropertyAccessFromIndexSignature": true,
    "exactOptionalPropertyTypes": true,
    "forceConsistentCasingInFileNames": true,
    "jsx": "react-jsx",
    "types": []
  },
  "include": [
    "./src/**/*.ts",
    "./src/**/*.tsx",
    "./test/**/*.ts",
    "./test/**/*.tsx"
  ]
}
```

Delta from the runtime's `tsconfig.json` (store.md): `jsx` and the `.tsx` globs.

```ts
// from libs/isograph-react/src/index.ts
export {
  type ExtractClientFieldValue,
  type ExtractReadFromStore,
  type FragmentReferenceOfEntrypoint,
} from './core/entrypoint';
export {
  type FragmentReference,
  stableIdForFragmentReference,
  type StableIdForFragmentReference,
} from './core/FragmentReference';
export {
  type NetworkRequestReaderOptions,
  type OnNetworkError,
  type WhileInFlight,
} from './core/read';
export { FragmentReader } from './react/FragmentReader';
export {
  IsographEnvironmentProvider,
  type IsographEnvironmentProviderProps,
  useIsographEnvironment,
} from './react/IsographEnvironmentProvider';
export {
  useImperativeReference,
  type UseImperativeReferenceResult,
} from './react/useImperativeReference';
export { useLazyReference } from './react/useLazyReference';
export { useReadAndSubscribe } from './react/useReadAndSubscribe';
export { useResult } from './react/useResult';
```

## The entrypoint artifact type

`IsographEntrypoint<TParam, TOutput, TEntityName>` landed in `@isograph/runtime` with normalization-ast.md (The artifact's TypeScript type), and the generated `entrypoint.ts` is declared with it: `const artifact: IsographEntrypoint<Query__HomeRoute__param, Query__HomeRoute__output_type, "Query"> = {..}; export default artifact;`. reader-ast.md change 3 types its reader artifact (reader-ast.md, The entrypoint's reader artifact):

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type IsographEntrypoint<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  readonly kind: 'Entrypoint';
  /** `<Entity>.<selectable>`. */
  readonly name: string;
  readonly queryText: string;
  readonly normalizationAst: NormalizationAst<TEntityName>;
  readonly readerArtifact: ReaderArtifact<TParam, TOutput, TEntityName>;
};
```

`useLazyReference` infers the variables type, the result type, and the root entity from its argument, as upstream's infers the first two from `IsographEntrypoint<P__f__param, P__f__output_type, NormalizationAst, P__f__raw_response_type>` (`isograph/demos/pet-demo/src/components/__isograph/Query/PetDetailRoute/entrypoint.ts:23-28`). Deltas from upstream (`core/entrypoint.ts:77-93`): three type parameters, `TEntityName` new and `TNormalizationAst` and `TRawResponseType` gone (`TNormalizationAst` existed for `NormalizationAstLoader`; `TRawResponseType` for `optimisticNetworkResponse`); `networkRequestInfo` is `queryText` and `normalizationAst`; `readerWithRefetchQueries` is `readerArtifact`; `concreteType` is gone because the root is `normalizationAst.root`; `name` is new and read by `stableIdForFragmentReference`. `kind` stays because the artifact prints it; nothing reads it. pico-runtime.md's structural `Entrypoint<TOutput>` is this type.

`TParam extends ResolverParam<TEntityName>` makes `ReaderArtifact<TParam, V, TEntityName>` assignable to `ReaderArtifact<ResolverParam<TEntityName>, V, TEntityName>` by the method bivariance reader-ast.md relies on (Why `resolver` is a method signature), so the package calls `read` and the resolver with no cast. `normalizationAst.root` is a `NormalizationRootRecord<TEntityName>`, structurally `Link<TEntityName>`, so a `FragmentReference`'s `root` is the entrypoint's root with its entity name's literal type.

Every generated parameters type is assignable to `Variables`, because every parameter and every input object field is required and none is `void` (store.md, Variables).

## The environment and the network path

The environment, the network function, `PromiseWrapper`, the fetch options, retention, the request cache (`networkRequestCache`, `getOrCreateCacheForArtifact`), `fetchEntrypoint`, and the two-phase subscriptions are `@isograph/runtime`'s (network-and-subscriptions.md); `CacheItem` and `ParentCache` are `@isograph/disposable-types`'s. The React package imports them, re-exports none of them, and adds no field to the environment. The app creates one environment with the runtime's `createIsographEnvironment`, once, at module level or in a component's `useState` initializer (upstream's demos use `useMemo`, `isograph/demos/pet-demo/pages/_app.tsx:68-70`; the initializer runs once by contract), and `IsographEnvironmentProvider` only passes it through context. The provider's state holds the environment, created once; it is never a cache of read results, which live only in the runtime (Reading, `useReadAndSubscribe`).

The React package calls the runtime's pieces only at render and commit: at render, `getOrCreateCacheForArtifact(..).getOrPopulateAndTemporaryRetain()` and a temporarily retained `readAndSubscribe`; at commit, `permanentRetainIfNotDisposed`, as `useCachedResponsivePrecommitValue` and `useLazyDisposableState` do (network-and-subscriptions.md, The request cache, The two-phase lifetime). The hook snippets below are upstream's shape, which subscribes in an effect and reads the request cache through a React-side `getOrCreateCacheForArtifact`; they are stale against the runtime's two-phase subscriptions and its `getOrCreateCacheForArtifact` until this doc is next worked, when the subscribe moves to render under a temporary retain and the cache lookup is the runtime's.

```tsx
// from libs/isograph-react/src/react/IsographEnvironmentProvider.tsx
import type { IsographEnvironment } from '@isograph/runtime';
import * as React from 'react';
import type { ReactNode } from 'react';
import { createContext, useContext } from 'react';

export const IsographEnvironmentContext =
  createContext<IsographEnvironment | null>(null);

export type IsographEnvironmentProviderProps = {
  readonly environment: IsographEnvironment;
  readonly children: ReactNode;
};

export function IsographEnvironmentProvider({
  environment,
  children,
}: IsographEnvironmentProviderProps): React.ReactElement {
  return (
    <IsographEnvironmentContext.Provider value={environment}>
      {children}
    </IsographEnvironmentContext.Provider>
  );
}

export function useIsographEnvironment(): IsographEnvironment {
  const context = useContext(IsographEnvironmentContext);
  if (context == null) {
    throw new Error(
      'Unexpected null environment context. Make sure to render ' +
        'this component within an IsographEnvironmentProvider component',
    );
  }
  return context;
}
```

Copied from `react/IsographEnvironmentProvider.tsx:1-34`. Delta: `IsographEnvironment` is imported from `@isograph/runtime` instead of `../core/IsographEnvironment`.

## `FragmentReference`

```ts
// from libs/isograph-react/src/core/FragmentReference.ts
import {
  type EntityName,
  type Link,
  type ReaderArtifact,
  type ExtractParameters,
  type PromiseWrapper,
  type ResolverParam,
  stableStringify,
} from '@isograph/runtime';

/** A fetched or retained entrypoint's reader, at its root, with its variables. */
export type FragmentReference<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
> = {
  readonly readerArtifact: ReaderArtifact<TReadFromStore, TClientFieldValue, TEntityName>;
  /** The entrypoint's name, `Query.HomeRoute`. */
  readonly fieldName: string;
  readonly root: Link<TEntityName>;
  readonly variables: ExtractParameters<TReadFromStore>;
  readonly networkRequest: PromiseWrapper<void, unknown>;
};

export type StableIdForFragmentReference = string;

export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<ResolverParam, unknown, EntityName>,
): StableIdForFragmentReference {
  return stableStringify([
    fragmentReference.root.entityName,
    fragmentReference.root.id,
    fragmentReference.fieldName,
    fragmentReference.variables,
  ]);
}
```

Deltas from `core/FragmentReference.ts:35-79`: `kind` is dropped (nothing reads it); `readerWithRefetchQueries` is `readerArtifact` (no refetch queries, no lazily loaded readers); `readerArtifactKind` is dropped (one kind); `TEntityName` is new, and `root` is a `Link<TEntityName>` where upstream's is a `StoreLink` (`{ __link, __typename: string }`, `core/FragmentReference.ts:58`, `core/IsographEnvironment.ts:122-125`), so `readerArtifact` and `root` name the same entity and `read` of the two type-checks (reader-ast.md, The parent entity's name); `TReadFromStore` is constrained by `ResolverParam<TEntityName>` instead of `UnknownTReadFromStore`; `ExtractParameters` is the runtime's. The stable id is the stable JSON of a list instead of `/`-joined text, so an id or a name containing `/` cannot make two ids equal; it is the key form pico-runtime.md uses for `Store.observed`.

```ts
// from libs/isograph-react/src/core/entrypoint.ts
import type {
  EntityName,
  IsographEntrypoint,
  ResolverParam,
} from '@isograph/runtime';
import type { FragmentReference } from './FragmentReference';

export type FragmentReferenceOfEntrypoint<
  TEntrypoint extends IsographEntrypoint<ResolverParam, unknown, EntityName>,
> =
  TEntrypoint extends IsographEntrypoint<
    infer TReadFromStore extends ResolverParam<infer TEntityName extends EntityName>,
    infer TClientFieldValue,
    infer TEntityName
  >
    ? FragmentReference<TReadFromStore, TClientFieldValue, TEntityName>
    : never;

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<
    infer X extends ResolverParam,
    unknown,
    EntityName
  >
    ? X
    : never;
export type ExtractClientFieldValue<Type> =
  Type extends IsographEntrypoint<ResolverParam, infer X, EntityName>
    ? X
    : never;
```

Delta from `core/entrypoint.ts:95-100`, `:200-203`: three type parameters, and `unknown`, `EntityName`, or `ResolverParam` where upstream writes `any`. `FragmentReferenceOfEntrypoint` infers the three parameters in one conditional type: `FragmentReference<ExtractReadFromStore<T>, ExtractClientFieldValue<T>, ..>` fails `tsc` 5.6.3, because a conditional type is not known to satisfy `ResolverParam<TEntityName>`.

## Fetching with `useLazyReference`

```ts
// from libs/isograph-react/src/react/useLazyReference.ts
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import type {
  EntityName,
  ExtractParameters,
  FetchOptions,
  IsographEntrypoint,
  ResolverParam,
} from '@isograph/runtime';
import { useMemo } from 'react';
import type { FragmentReference } from '../core/FragmentReference';
import { getOrCreateCacheForArtifact } from '@isograph/runtime';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export function useLazyReference<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue, TEntityName>;
} {
  const environment = useIsographEnvironment();
  const cache = getOrCreateCacheForArtifact(
    environment,
    entrypoint,
    variables,
    fetchOptions,
  );
  const networkRequest = useLazyDisposableState(cache).state;
  const fragmentReference = useMemo(
    () => ({
      readerArtifact: entrypoint.readerArtifact,
      fieldName: entrypoint.name,
      root: entrypoint.normalizationAst.root,
      variables,
      networkRequest,
    }),
    [networkRequest],
  );
  return { fragmentReference };
}
```

`getOrCreateCacheForArtifact` is the runtime's (network-and-subscriptions.md, The request cache): the environment's `networkRequestCache`, keyed by `queryText + stableStringify(variables)`, upstream's key, with `fetchEntrypoint` as each `ParentCache`'s factory. `useLazyReference` imports it from `@isograph/runtime`, and `src/core/getOrCreateCacheForArtifact.ts` is not in this package.

How it fetches: the first render with a cache key calls the factory through `useLazyDisposableState`, which temporarily retains the item (5000 ms, `disposable-state/CacheItem.ts:7`) and permanently retains it at commit. The factory is the runtime's `fetchEntrypoint`: `IfNecessary` calls store.md's `check` on the entrypoint's AST and variables, sends the request on `MissingData`, and retains the query either way. Normalizing writes the response, then `callSubscriptions` re-reads every subscription (network-and-subscriptions.md). A second render or a second component with the same query text and variables gets the same cached request. Unmount disposes the permanent retain; when no retain is left the cache item calls `fetchEntrypoint`'s cleanup, which unretains the query. Whether the package then calls `garbageCollectEnvironment` is open decision 1.

Deltas from `react/useLazyReference.ts:19-60` and `core/getOrCreateCacheForArtifact.ts:26-86`:

- The signature drops `TNormalizationAst` and `TRawResponseType` and the conditional `...[fetchOptions]` tuple, which existed for `NormalizationAstLoader` (`RequiredFetchOptions` when the AST is loaded lazily). The return type is `{ fragmentReference }` without the `unknown` branch for the same reason.
- No `kind !== 'Entrypoint'` log (`:42-48`): no log events, and the argument is a declared `IsographEntrypoint`.
- The cache and its key are the runtime's, upstream's `operation.text + JSON.stringify(stableCopy(variables))` (`:42-54`).
- The cache holds the network request, not the `FragmentReference` (`:55-84`), and the hook builds the reference from its own typed `entrypoint` and `variables`. Upstream's `fragmentCache` is `CacheMap<FragmentReference<any, any>>` (`IsographEnvironment.ts:81-83`), whose `any` lets one map hold every entrypoint's references; with `any` banned, a map of references needs a cast to the caller's type parameters. The request is untyped (`PromiseWrapper<void, unknown>`), so the map needs none. `useMemo` keeps the reference's identity per request: one cache key gives one request item, and one query text and variables give one entrypoint and equal variables.

## `useImperativeReference`

```ts
// from libs/isograph-react/src/react/useImperativeReference.ts
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  type EntityName,
  type ExtractParameters,
  type FetchOptions,
  fetchEntrypoint,
  type IsographEntrypoint,
  type ResolverParam,
} from '@isograph/runtime';
import type { FragmentReference } from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export type UseImperativeReferenceResult<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
> = {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue, TEntityName> | null;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    fetchOptions?: FetchOptions<TClientFieldValue>,
  ) => void;
};

export function useImperativeReference<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
): UseImperativeReferenceResult<TReadFromStore, TClientFieldValue, TEntityName> {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue, TEntityName>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state !== UNASSIGNED_STATE ? state : null,
    loadFragmentReference: (
      variables: ExtractParameters<TReadFromStore>,
      fetchOptions?: FetchOptions<TClientFieldValue>,
    ) => {
      const [networkRequest, disposeNetworkRequest] = fetchEntrypoint(
        environment,
        entrypoint,
        variables,
        fetchOptions ?? null,
      );
      setState([
        {
          readerArtifact: entrypoint.readerArtifact,
          fieldName: entrypoint.name,
          root: entrypoint.normalizationAst.root,
          variables,
          networkRequest,
        },
        disposeNetworkRequest,
      ]);
    },
  };
}
```

Delta from `react/useImperativeReference.ts:25-100`: two type parameters; `fetchOptions` is a plain optional parameter (the loader tuple is gone); no `getOrLoadReaderWithRefetchQueries` (`:72-76`); the reference's fields are i2's. Behavior is upstream's: `loadFragmentReference` before the component commits throws (`disposable-state/useUpdatableDisposableState.ts:73-79`), and each commit disposes every earlier reference (`:93-107`).

## Reading

### `useResult`

```ts
// from libs/isograph-react/src/react/useResult.ts
import type {
  EntityName,
  ReaderArtifact,
  ResolverParam,
} from '@isograph/runtime';
import type { FragmentReference } from '../core/FragmentReference';
import {
  getNetworkRequestOptionsWithDefaults,
  type NetworkRequestReaderOptions,
} from '../core/read';
import { maybeUnwrapNetworkRequest } from './maybeUnwrapNetworkRequest';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export function useResult<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue, TEntityName>,
  partialNetworkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): TClientFieldValue {
  const networkRequestOptions = getNetworkRequestOptionsWithDefaults(
    partialNetworkRequestOptions,
  );
  maybeUnwrapNetworkRequest(
    fragmentReference.networkRequest,
    networkRequestOptions,
  );
  const data = useReadAndSubscribe(fragmentReference, networkRequestOptions);
  const readerArtifact: ReaderArtifact<
    ResolverParam<TEntityName>,
    TClientFieldValue,
    TEntityName
  > = fragmentReference.readerArtifact;
  return readerArtifact.resolver({
    data,
    parameters: fragmentReference.variables,
    link: fragmentReference.root,
  });
}
```

Delta from `react/useResult.ts:16-67`: the `ComponentReaderArtifact` case (`:29-36`) is gone (no `@component`), and with it the `switch` and the `useIsographEnvironment` call it needed; no `readPromise` of the reader (`:42-44`, no lazily loaded readers); no `startUpdate` (`:53-61`, no `@updatable`); the resolver is called through a `ReaderArtifact<ResolverParam<TEntityName>, TClientFieldValue, TEntityName>` binding, which the method bivariance accepts with no `@ts-expect-error` (upstream `:63`), and receives the reference's `root` as its `link` (reader-ast.md, A resolver's own link).

```ts
// from libs/isograph-react/src/core/read.ts
import {
  type EntityName,
  type IsographEnvironment,
  NOT_SET,
  readAndSubscribe,
  type ResolverParam,
  type SelectionSetData,
  type SubscriptionResult,
  subscriptionSnapshot,
} from '@isograph/runtime';
import type { FragmentReference } from './FragmentReference';

/** What a read does while its request is pending: suspend until it settles, or read the store. */
export type WhileInFlight = 'Suspend' | 'ReadStore';

/** What a read does when its request rejected: throw the error, or read the store. */
export type OnNetworkError = 'Throw' | 'ReadStore';

export type NetworkRequestReaderOptions = {
  readonly whileInFlight: WhileInFlight;
  readonly onNetworkError: OnNetworkError;
};

export function getNetworkRequestOptionsWithDefaults(
  networkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): NetworkRequestReaderOptions {
  return {
    whileInFlight: networkRequestOptions?.whileInFlight ?? 'ReadStore',
    onNetworkError: networkRequestOptions?.onNetworkError ?? 'Throw',
  };
}

/** The data in `result`, a snapshot of `fragmentReference`'s read. On `MissingData`, throws
 * the promise that resolves when that read changes, or the network error; on `Err`, throws
 * the read's error. */
export function unwrapOrSuspend<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  networkRequestOptions: NetworkRequestReaderOptions,
  result: SubscriptionResult,
): SelectionSetData {
  if (result.kind === 'Ok') {
    return result.value;
  }
  if (result.kind === 'Err') {
    throw result.error;
  }
  if (
    networkRequestOptions.whileInFlight === 'ReadStore' &&
    networkRequestOptions.onNetworkError === 'Throw'
  ) {
    const networkResult = fragmentReference.networkRequest.result;
    if (networkResult !== NOT_SET && networkResult.kind === 'Err') {
      throw new Error('NetworkError', { cause: networkResult.error });
    }
    throw new Promise<void>((resolve, reject) => {
      onNextChangeToFragmentData(environment, fragmentReference).then(resolve);
      fragmentReference.networkRequest.promise.catch(reject);
    });
  }
  throw onNextChangeToFragmentData(environment, fragmentReference);
}

/** Resolves at the first write after which `fragmentReference`'s read is no longer
 * `MissingData`, or at once when the read is already not `MissingData`. */
export function onNextChangeToFragmentData<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
): Promise<void> {
  return new Promise((resolve) => {
    const [subscription, unsubscribe] = readAndSubscribe(
      environment,
      fragmentReference.readerArtifact,
      fragmentReference.root,
      fragmentReference.variables,
      (result) => {
        if (result.kind !== 'MissingData') {
          unsubscribe();
          resolve();
        }
      },
    );
    if (subscriptionSnapshot(subscription).kind !== 'MissingData') {
      unsubscribe();
      resolve();
    }
  });
}
```

`NetworkRequestReaderOptions` and `getNetworkRequestOptionsWithDefaults` are `core/read.ts:972-984` with `readonly` properties and each boolean replaced by an enum whose variants name the two behaviors (AGENTS.md, Booleans): `suspendIfInFlight: boolean` is `whileInFlight: WhileInFlight`, default `'ReadStore'` (upstream's `false`), and `throwOnNetworkError: boolean` is `onNetworkError: OnNetworkError`, default `'Throw'` (upstream's `true`). This is a deviation from upstream's public API, made for AGENTS.md's rule; the pagination hooks, which upstream calls with `suspendIfInFlight: true`, pass `whileInFlight: 'Suspend'` when they return (Later). Both options are React-only: the runtime's `read` returns a typed result and never suspends or throws. `unwrapOrSuspend` is the throwing half of upstream's `readButDoNotEvaluate` (`read.ts:60-129`) on the React side: it receives the subscription's snapshot instead of reading, because the hook reads only through `useSyncExternalStore` (`useReadAndSubscribe`), and throws what upstream throws. The deltas: no `readPromise` of the reader (`:70-72`), no log (`:85-90`), no `encounteredRecords` in the result, an `Err` snapshot is thrown, and the promise is `onNextChangeToFragmentData` instead of `onNextChangeToRecord(environment, response.recordLink)` (`cache.ts:135-149`).

Upstream suspends until any write touches the record the read stopped at; i2's `normalize` records no touched records, so the suspended read subscribes with `readAndSubscribe` and resumes at the first write after which the read is no longer `MissingData` (`Ok`, or `Err` when a nested resolver throws, which the retried render then throws). A write that fills the record but leaves the read missing elsewhere resumes upstream's render, which suspends again; it does not resume i2's. `readAndSubscribe` reads when it subscribes, so a read that is no longer `MissingData` by then resolves the promise at once instead of waiting for a write that may never come; the callback refers to `unsubscribe` only when `callSubscriptions` calls it, after the binding. A suspended read whose component never retries keeps its subscription until the data arrives, as upstream's `onNextChangeToRecord` keeps its record subscription.

```ts
// from libs/isograph-react/src/react/maybeUnwrapNetworkRequest.ts
import { getPromiseState, type PromiseWrapper } from '@isograph/runtime';
import type { NetworkRequestReaderOptions } from '../core/read';

export function maybeUnwrapNetworkRequest(
  networkRequest: PromiseWrapper<void, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): void {
  const state = getPromiseState(networkRequest);
  if (state.kind === 'Err' && networkRequestOptions.onNetworkError === 'Throw') {
    throw state.error;
  } else if (
    state.kind === 'Pending' &&
    networkRequestOptions.whileInFlight === 'Suspend'
  ) {
    throw state.promise;
  }
}
```

Delta from `react/maybeUnwrapNetworkRequest.ts`: `PromiseWrapper<void, unknown>` for `<void, any>`, the return type, and the two enums.

What the two options do, upstream's semantics: `whileInFlight: 'Suspend'` suspends while the request is pending even when the store has the data; `onNetworkError: 'Throw'` (the default) throws the request's error at render, and makes a `MissingData` read reject when the request rejects. With `whileInFlight: 'ReadStore'` (the default), a read whose data the store already has renders it while the request is in flight.

### `useReadAndSubscribe`

This is a deviation from upstream. Upstream's `useReadAndSubscribe` keeps the read result in React state: it reads in a `useState` initializer, and `useRerenderOnChange` subscribes in an effect and calls `setState` from the callback (`react/useReadAndSubscribe.ts:33-35`, `react/useRerenderOnChange.ts:21-37`). i2 keeps the read result only in the runtime, in the subscription's `delivered`, and the hook reads it through `useSyncExternalStore(subscribe, getSnapshot)`. React state would hold a second copy of what the runtime already holds. That copy goes stale when a write lands between the render that read it and the commit that subscribes, which is why upstream re-reads on subscribe (`subscribe.ts:32-36`), and under concurrent rendering two components can render from copies taken after different writes. `useSyncExternalStore` exists to read an external store without that copy: React calls `getSnapshot` at render, calls it again after `subscribe` at commit, and re-renders when the two differ.

```ts
// from libs/isograph-react/src/react/useReadAndSubscribe.ts
import type {
  EntityName,
  ResolverParam,
  SelectionSetData,
} from '@isograph/runtime';
import type { FragmentReference } from '../core/FragmentReference';
import { type NetworkRequestReaderOptions, unwrapOrSuspend } from '../core/read';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { useSubscription } from './useSubscription';
import { useSubscriptionSnapshot } from './useSubscriptionSnapshot';

/**
 * The data `fragmentReference`'s reader AST reads, re-rendering when it changes. Suspends on
 * `MissingData`; throws a read's error.
 */
export function useReadAndSubscribe<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  networkRequestOptions: NetworkRequestReaderOptions,
): SelectionSetData {
  const environment = useIsographEnvironment();
  const subscription = useSubscription(
    environment,
    fragmentReference.readerArtifact,
    fragmentReference.root,
    fragmentReference.variables,
  );
  const result = useSubscriptionSnapshot(subscription);
  return unwrapOrSuspend(environment, fragmentReference, networkRequestOptions, result);
}
```

```ts
// from libs/isograph-react/src/react/useSubscriptionSnapshot.ts
import {
  listenToSubscription,
  type Subscription,
  type SubscriptionResult,
  subscriptionSnapshot,
} from '@isograph/runtime';
import { useCallback, useSyncExternalStore } from 'react';

/** `subscription`'s last result, re-rendering when it changes. */
export function useSubscriptionSnapshot(subscription: Subscription): SubscriptionResult {
  const subscribe = useCallback(
    (onStoreChange: () => void) => listenToSubscription(subscription, onStoreChange),
    [subscription],
  );
  return useSyncExternalStore(subscribe, () => subscriptionSnapshot(subscription));
}
```

```ts
// from libs/isograph-react/src/react/useSubscription.ts
import type {
  EntityName,
  IsographEnvironment,
  Link,
  ReaderArtifact,
  ResolverParam,
  Subscription,
  Variables,
} from '@isograph/runtime';

/**
 * The subscription to `readerArtifact`'s read at `link` with `variables`, created at render by
 * `readAndSubscribe` under a temporary retain and permanently retained at commit
 * (network-and-subscriptions.md, The two-phase lifetime). Another artifact, link, or
 * `stableStringify(variables)` gets another subscription, and the old one is released.
 */
export function useSubscription<TEntityName extends EntityName>(
  environment: IsographEnvironment,
  readerArtifact: ReaderArtifact<ResolverParam<TEntityName>, unknown, TEntityName>,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
): Subscription;
```

`useSubscription` holds the `Subscription`, a disposable item, across renders, as `useLazyDisposableState` holds its item (`disposable-state/useLazyDisposableState.ts`). It holds the handle and never the data: every render reads the data from the handle through `subscriptionSnapshot`. Its body is the two-phase lifetime in `useLazyDisposableState`'s shape, written with the request-cache hooks when this doc is next worked (The environment and the network path). The `callback` it passes to `readAndSubscribe` does nothing: React receives the notification through `listenToSubscription`, because React hands its listener to `subscribe` at commit, after `readAndSubscribe` fixed the callback at render. `useCallback` keeps `subscribe` the same function while the subscription is the same, so React does not unsubscribe and subscribe again on every render; it memoizes a function of the handle, not a read result.

What `useSyncExternalStore` needs from the runtime, by name:

- `getSnapshot` is `() => subscriptionSnapshot(subscription)` (network-and-subscriptions.md, The snapshot). It must return the same object (`===`) until a write changes the read. React compares consecutive `getSnapshot` results with `Object.is`; a fresh object on every call is a change on every call, and React re-renders in a loop (React's warning: "The result of getSnapshot should be cached to avoid an infinite loop"). `callSubscriptions` replaces `delivered` only when a re-read is unequal to it by the reader AST's rule, which gives this. The read itself is never done in `getSnapshot`.
- `subscribe` is `(onStoreChange) => listenToSubscription(subscription, onStoreChange)`. `listenToSubscription` is not yet in network-and-subscriptions.md (Docs this doc amends).

```ts
// from libs/isograph-runtime/src/subscribe.ts
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

/** Calls `listener` after each change to `subscriptionSnapshot(subscription)`, until the
 * returned function is called. */
export function listenToSubscription(
  subscription: Subscription,
  listener: () => void,
): () => void;
```

`callSubscriptions` calls each listener through `reportCallbackError` after it replaces `delivered` and calls `callback`. Under pico-runtime.md's change 4, `listen` and `snapshot` take the places of `listenToSubscription` and `subscriptionSnapshot`, and the hook keeps this shape.

React constraint (pico-runtime.md, Follow-ups 9): `useSyncExternalStore` performs a store update as blocking even inside a transition. A write inside `startTransition` re-renders the components that read it synchronously, with no pending state. Deferring that re-render is `useDeferredValue` over the hook's result, or a deferred snapshot the React package updates inside `startTransition`; v1 does neither.

Deltas from `react/useReadAndSubscribe.ts:25-43` and `react/useRerenderOnChange.ts:10-38`:

- No `readerAst` parameter: the fragment reference holds the reader artifact.
- The data is `SelectionSetData`, not `ExtractData<TReadFromStore>`. The hook passes it only to the resolver; typing it would need a cast from the runtime's untyped read.
- No React state holds the data, and `useRerenderOnChange` is gone: the reasons above.
- When the fragment reference changes, `useSubscription` returns another subscription, `subscribe` changes with it, and React unsubscribes the old listener and subscribes the new one; `getSnapshot` reads the new subscription in the same render. Upstream reads only in the `useState` initializer (`useReadAndSubscribe.ts:33-35`) and subscribes once (`useRerenderOnChange.ts:31-37`, deps `[]`, "the fragment reference is stable for the life of the component"). That held because each upstream fragment reference rendered through its own cached component (`getOrCreateCachedComponent`), which remounts for a new reference. Without `@component`, `useResult` is called with a changing reference by the same component (a route whose `id` changes), and upstream's version would keep the old data and the old subscription.
- A `MissingData` or `Err` snapshot after mount suspends or throws, because the snapshot is what the subscription delivered last. Upstream keeps the rendered data, whose re-read throws and is caught by `logAnyError` (`core/subscribe.ts:47-61`). Open decision 2.
- `useSubscribeToMultiple` (`:45-86`) and `componentFunction` (`:88-130`) are dropped.

### `FragmentReader`

```tsx
// from libs/isograph-react/src/react/FragmentReader.tsx
import type { EntityName, ResolverParam } from '@isograph/runtime';
import type { FragmentReference } from '../core/FragmentReference';
import type { NetworkRequestReaderOptions } from '../core/read';
import { useResult } from './useResult';

export function FragmentReader<
  TReadFromStore extends ResolverParam<TEntityName>,
  TResult,
  TEntityName extends EntityName,
  TChildrenResult,
>({
  fragmentReference,
  networkRequestOptions,
  children,
}: {
  readonly fragmentReference: FragmentReference<TReadFromStore, TResult, TEntityName>;
  readonly networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
  readonly children: (data: TResult) => TChildrenResult;
}): TChildrenResult {
  const result = useResult(fragmentReference, networkRequestOptions);
  return children(result);
}
```

Delta from `react/FragmentReader.tsx:10-29`: the reference's type is given by `TReadFromStore` directly, instead of `ExtractReadFromStore<TEntrypoint>` with `TEntrypoint extends IsographEntrypoint<any, TResult, any, TRawResponseType>`, which needs `any` and `TRawResponseType`.

## Re-rendering on writes

`readButDoNotEvaluate`, `readAndSubscribe`, `subscriptionSnapshot`, and `callSubscriptions` are network-and-subscriptions.md's (Reading without the root resolver, Subscriptions): pico-runtime.md's Step A with this doc's former amendments. A hook subscribes at render, so a write between the render and the commit replaces the subscription's `delivered`, and `useSyncExternalStore`'s check after `subscribe` re-renders with it. The callback receives the reader's data, and the hook calls the resolver in render; a `MissingData` or `Err` re-read is delivered as such, and becomes the snapshot.

## Disposal, StrictMode, and upstream's disposable-state behavior

StrictMode is supported, as upstream supports it. Upstream's `1ba744d7` ("useLazyDisposableState and friends survive effect re-runs under StrictMode and Fast Refresh") added `useEffectsRerunWithoutRenderRef` (`disposable-state/useEffectsRerunWithoutRender.ts:15-23`): `useCachedResponsivePrecommitValue`'s effect re-acquires the item from the parent cache, or the factory, when its effect re-runs without a render (`useCachedResponsivePrecommitValue.ts:74-98`), and `useLazyDisposableState`'s cleanup forgets its pair (`useLazyDisposableState.ts:31-43`). The same commit enabled `reactStrictMode` in upstream's demos (`demos/pet-demo/next.config.js:5`) and removed the FAQ and quickstart sections that told users to disable StrictMode, so upstream's `docs-website/docs/faq.md` and `quickstart.md` at `9d72f6fc` no longer say otherwise. The demo's `main.tsx` renders under `StrictMode`, and the hook tests render under it (Tests). In development a StrictMode mount sends the request, releases it on the simulated unmount, and re-acquires it on the remount: from the parent cache when another holder still temporarily retains it, else through the factory, which with `IfNecessary` sees `MissingData` while the first request is in flight and sends a second one, as upstream does (network-and-subscriptions.md, decision 4). The first response is dropped, because its item was disposed, and the second is normalized.

Traces of upstream bugs that the current checkout fixes are gone from this doc: the StrictMode "A permanent retain should only be cleared once." throw (fixed by `1ba744d7`), `useDisposableState`'s throw on its second `setState` (`useDisposableState.ts:31-41`, the throw removed by `1ba744d7`), and a temporary retain cleared twice by its callback (fixed by `a8e129cf`).

The following hold at `9d72f6fc`, traced from source, not reproduced:

- A render not committed within 5000 ms disposes its item (`CacheItem.ts:7`, `:248-251`). When the component that calls `useLazyReference` is itself suspended (a `Suspense` boundary above it, and its reader in the same component), the item is disposed before a slower response arrives, the response is dropped (network-and-subscriptions.md, `fetchEntrypoint`), and the read's subscription never fires. The demo places `Suspense` below the loader, as upstream's demo does (`isograph/demos/pet-demo/src/components/HomeRoute.tsx:32-47`).
- When the commit finds its item disposed and the parent cache empty, it creates one with `parentCache.factory()` outside the parent cache (`useCachedResponsivePrecommitValue.ts:89-98`), so another component with the same cache key creates another item. That item's `fetchEntrypoint` sends a second request while the first is in flight (`IfNecessary` sees `MissingData`); after the first settled, `IfNecessary` finds the data with `check` and sends nothing.
- The first caller's `fetchOptions` win for a cache key: the factory closes over them (`getOrCreateCacheForArtifact.ts:55-84`), so a second component's `onComplete` with the same query and variables is not called. The runtime's cache keeps this outcome (network-and-subscriptions.md, The request cache).
- Cache entries are never removed: one empty `ParentCache` per distinct query text and variables stays in the map for the environment's life (`ParentCache.ts:32`, "TODO pass an onEmpty function").
- `useReadAndSubscribe` keeps stale data when its fragment reference changes: fixed here, because v1 forces it, by a new subscription per reference (Reading).

Held for the rest: upstream's behavior. Each is listed so that a fix is a decision, not a surprise.

## The disposable-state packages

`@isograph/disposable-types` (`libs/isograph-disposable-types`) and `@isograph/react-disposable-state` (`libs/isograph-react-disposable-state`) are upstream's two packages, ported unchanged from the checkout at `9d72f6fc`: every source file, test, `index.ts`, `package.json` dependency, and `vitest.config.ts`, as upstream has them. `@isograph/disposable-types` is ported by network-and-subscriptions.md (change 1), which also moves `CacheItem.ts` and `ParentCache.ts` with their tests into it, unchanged but for their import of that package's own types (its change 2). `@isograph/react-disposable-state` is `useCachedResponsivePrecommitValue.ts`, `useDisposableArray.ts`, `useDisposableState.ts`, `useEffectsRerunWithoutRender.ts`, `useHasCommittedRef.ts`, `useLazyDisposableState.ts`, `useUpdatableDisposableClearableState.ts`, `useUpdatableDisposableState.ts`, and their tests, under `src/`, with upstream's `index.ts` (`export *` of each module and of `@isograph/disposable-types`, which now includes `CacheItem` and `ParentCache`, so the package's exports are upstream's). The only change from upstream is the import of `./CacheItem` and `./ParentCache`, which is `@isograph/disposable-types` in `useCachedResponsivePrecommitValue.ts`, `useLazyDisposableState.ts`, `useDisposableState.ts`, and their tests, and the two `export *` lines leaving `index.ts`.

The port keeps upstream's code as it is, including TypeScript's `private __x` fields, the tests' `(x as any).__state` reads, the `@ts-ignore`d `tsTests` functions, and the internal booleans (`cleared`, `hasCommittedRef`). These differ from AGENTS.md's TypeScript rules (ES private fields, no `any`) and its Booleans rule; the user's decision is that the two packages are upstream's code, ported as is, so that a later upstream fix ports by copying. Their `package.json` and `tsconfig.json` are upstream's, with `private` and the workspace's source `exports` (`types` and `import`) in place of upstream's `dist` builds, as `@isograph/react`'s. If the workspace's oxlint or typecheck hooks reject the ported code, the implementer stops and raises it.

## The network request lifecycle, end to end

1. `useLazyReference` renders: the runtime's `getOrCreateCacheForArtifact` returns the cache for `queryText + stableStringify(variables)`; `useLazyDisposableState` calls its factory on the first render.
2. The factory: the runtime's `fetchEntrypoint`, which checks, sends the request or retains without sending, and retains `{ ast, variables }` in `environment.retainedQueries`.
3. `FragmentReader` renders `useResult`: `maybeUnwrapNetworkRequest`, then `useReadAndSubscribe`: `useSubscription` reads and subscribes with `readAndSubscribe`, and `useSyncExternalStore` returns `subscriptionSnapshot`; `MissingData` throws `onNextChangeToFragmentData`'s promise.
4. The response arrives: `normalize`, then `callSubscriptions`: the suspended read's promise subscription sees `Ok`, removes itself, and resolves; React retries and the snapshot is `Ok`. The component commits; `useSubscription` retains the subscription permanently, and `useSyncExternalStore` calls `listenToSubscription`.
5. A later write that changes the data replaces the subscription's `delivered` and calls React's listener; React re-renders the component, which reads the new snapshot and calls the resolver on it.
6. Unmount: React's listener is removed and `useSubscription` releases the subscription; `useLazyReference`'s permanent retain is cleared, and the cache item calls `fetchEntrypoint`'s cleanup, which unretains the query. Collection follows open decision 1.

## Errors

Every throw this package adds or keeps, and its invariant:

- `useIsographEnvironment` outside a provider: upstream's message (`IsographEnvironmentProvider.tsx:25-34`). The invariant is the user's: hooks render under a provider.
- `unwrapOrSuspend` and `maybeUnwrapNetworkRequest` throw promises, the network error, and an `Err` snapshot's error. These are React's Suspense and error boundary protocol, upstream's behavior, not broken invariants.
- The ported disposable-state throws ("This indicates a bug in react-disposable-state.", "Calling setState before the component has committed is unsafe and disallowed."): upstream's, unchanged.
- The runtime's network path rejects a request's wrapper on a network error or a malformed response (network-and-subscriptions.md, Errors); a reader sees it through `onNetworkError`.

## The React demo

`demos/pet-demo` gains React and a browser entry. The `.ts` sources keep their `console.log(iso(\`entrypoint ..\`))` lines for `scripts/log-entrypoints.cjs`; the loaders reference the same entrypoints. Two literals naming one entrypoint are not a validation error (`crates/isograph_compiler/src/validation.rs` reports duplicate entities and selectables only) and produce one artifact (`project_artifacts` collects `ArtifactSource`s into a `BTreeSet`, `crates/isograph_compiler/src/artifact_types.rs:519-531`).

```json
// from demos/pet-demo/isograph.config.json
{
  "source_files": ["src/**/*.ts", "src/**/*.tsx"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src"
}
```

```json
// from demos/pet-demo/package.json
{
  "name": "pet-demo",
  "private": true,
  "scripts": {
    "dev": "vite",
    "log-entrypoints": "node scripts/log-entrypoints.cjs",
    "test": "vitest run",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "@isograph/react": "workspace:*",
    "@isograph/runtime": "workspace:*",
    "react": "18.3.1",
    "react-dom": "18.3.1"
  },
  "devDependencies": {
    "@babel/core": "^7.28.3",
    "@babel/plugin-transform-modules-commonjs": "^7.27.1",
    "@babel/preset-typescript": "^7.27.1",
    "@babel/register": "^7.28.3",
    "@isograph/babel-plugin": "workspace:*",
    "@types/react": "18.3.1",
    "@types/react-dom": "18.3.1",
    "@types/react-test-renderer": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.2",
    "react-test-renderer": "^18.2.0",
    "typescript": "5.6.3",
    "vite": "^5.4.8",
    "vitest": "^2.1.2"
  }
}
```

```json
// from demos/pet-demo/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "forceConsistentCasingInFileNames": true,
    "jsx": "react-jsx",
    "types": []
  },
  "include": ["./src/**/*.ts", "./src/**/*.tsx", "./test/**/*.tsx"]
}
```

Delta: `target` `ES2022` (`Object.hasOwn` in the mock, `Error` `cause`), `jsx`, and the `.tsx` and test globs.

```ts
// from demos/pet-demo/vite.config.ts
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const demoDirectory = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: [['@isograph/babel-plugin', { searchFrom: demoDirectory }]],
      },
    }),
  ],
  test: {
    setupFiles: ['./test/setup.ts'],
  },
});
```

Delta from `isograph/demos/vite-demo/vite.config.ts`: the Babel plugin is configured here instead of in `.babelrc.json`, so the config is one file; no `@iso` alias (i2's sources import `./__isograph/iso`); the `test` block, because vitest reads this file.

```html
<!-- from demos/pet-demo/index.html -->
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Pet demo</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

```tsx
// from demos/pet-demo/src/main.tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { mockNetworkFunction } from './mockNetwork';

const root = document.getElementById('root');
if (root == null) {
  throw new Error('index.html has no #root element.');
}
createRoot(root).render(
  <StrictMode>
    <App networkFunction={mockNetworkFunction} />
  </StrictMode>,
);
```

`main.tsx` is upstream's `vite-demo/src/main.tsx` with a null check in place of `!`, the network function passed in, and `StrictMode` (Disposal).

```ts
// from demos/pet-demo/src/mockNetwork.ts
import type { IsographNetworkFunction, JsonObject } from '@isograph/runtime';
import { iso } from './__isograph/iso';

const homeRoute = iso(`entrypoint Query.HomeRoute`);
const petDetailRoute = iso(`entrypoint Query.PetDetailRoute`);

type MockPet = {
  readonly id: string;
  readonly firstName: string;
  readonly lastName: string;
  readonly nickname: string | null;
  readonly age: number;
  readonly picture: string;
  readonly species: 'CAT' | 'DOG' | 'OTHER';
  readonly tagline: string;
  readonly bestFriendId: string | null;
  readonly intelligence: number;
  readonly checkins: ReadonlyArray<string>;
};

const PETS: ReadonlyArray<MockPet> = [
  {
    id: '0',
    firstName: 'Makayla',
    lastName: 'Balicki',
    nickname: 'Mak',
    age: 4,
    picture: 'https://example.com/pets/0.png',
    species: 'DOG',
    tagline: 'I AM HUNGRY',
    bestFriendId: '1',
    intelligence: 80,
    checkins: ['The park', 'The vet'],
  },
  {
    id: '1',
    firstName: 'Mr',
    lastName: 'Biggles',
    nickname: null,
    age: 7,
    picture: 'https://example.com/pets/1.png',
    species: 'CAT',
    tagline: 'Pet me now, human',
    bestFriendId: null,
    intelligence: 95,
    checkins: [],
  },
];

/** Answers `HomeRoute` and `PetDetailRoute` from `PETS`, with each query's response keys. */
export const mockNetworkFunction: IsographNetworkFunction = (
  operation,
  variables,
) => {
  if (operation.text === homeRoute.queryText) {
    return Promise.resolve({ pets: PETS.map(petSummary) });
  }
  const id = Object.hasOwn(variables, 'id') ? variables['id'] : undefined;
  if (operation.text === petDetailRoute.queryText && typeof id === 'string') {
    return Promise.resolve({ pet____id___v_id: petDetail(id) });
  }
  return Promise.reject(
    new Error(`The mock network has no response for ${operation.text}`),
  );
};

function petSummary(pet: MockPet): JsonObject {
  return {
    __typename: 'Pet',
    id: pet.id,
    age: pet.age,
    firstName: pet.firstName,
    lastName: pet.lastName,
    nickname: pet.nickname,
    picture: pet.picture,
    species: pet.species,
    tagline: pet.tagline,
  };
}

function petDetail(id: string): JsonObject | null {
  const pet = PETS.find((candidate) => candidate.id === id);
  if (pet === undefined) {
    return null;
  }
  const bestFriend = PETS.find(
    (candidate) => candidate.id === pet.bestFriendId,
  );
  return {
    __typename: 'Pet',
    id: pet.id,
    best_friend_relationship:
      bestFriend === undefined
        ? null
        : {
            __typename: 'BestFriendRelationship',
            best_friend: {
              __typename: 'Pet',
              id: bestFriend.id,
              firstName: bestFriend.firstName,
              lastName: bestFriend.lastName,
            },
            picture_together: null,
          },
    checkins____limit___l_5____skip___l_0: pet.checkins.map(
      (location, index) => ({
        __typename: 'Checkin',
        id: `${pet.id}-${index}`,
        location,
        time: '2026-11-19T12:00:00Z',
      }),
    ),
    firstName: pet.firstName,
    lastName: pet.lastName,
    nickname: pet.nickname,
    stats: {
      __typename: 'PetStats',
      cuteness: 90,
      energy: 70,
      hunger: 100,
      intelligence: pet.intelligence,
      sociability: 60,
      weight: 20,
    },
  };
}
```

The response keys are those of `Query/HomeRoute/query_text.graphql` and `Query/PetDetailRoute/query_text.graphql` at this commit. A compiler change that renames a response key makes `normalize` throw in the demo's test, which names the key. The mock resolves at once; the loading text shows for the render before the microtask that resolves it.

`test/setup.ts` sets `globalThis.IS_REACT_ACT_ENVIRONMENT = true` (with its `declare global` for the type), so `act` from `react-test-renderer` runs without warnings; `@isograph/react`'s `test/setup.ts` is the same file.

## Tests

No snapshots. Every test asserts facts: rendered text from `react-test-renderer`'s `toJSON()`, network calls, store contents, `retainedQueries` and `subscriptions` sizes. React tests use `react-test-renderer` and vitest, upstream's approach (`disposable-state/useLazyDisposableState.test.tsx:1-4`); upstream's `@isograph/react` has no hook tests. Network responses resolve when the test resolves them: `test/fixtures.tsx` exports a network function that records each call with its `resolve` and `reject`, so no test sleeps or polls (AGENTS.md, Wake on events).

### Fixture project (`libs/isograph-react/fixtures/pets`)

Generated by the real compiler and committed, as store.md's fixture. `vitest.config.ts` is reader-ast.md's runtime config with `fixtureDirectory` `./fixtures/pets/`, plus `esbuild: { jsx: 'automatic' }` and `test: { setupFiles: ['./test/setup.ts'] }`. Tests import entrypoint artifacts directly (`../fixtures/pets/src/__isograph/Query/PetNames/entrypoint`).

```graphql
# from libs/isograph-react/fixtures/pets/schema/schema.graphql
schema {
  query: Query
  mutation: Mutation
}

scalar ID
scalar String

type Query {
  pet(id: ID!): Pet
  pets: [Pet!]!
}

type Mutation {
  setName(id: ID!, name: String!): SetNamePayload!
}

type SetNamePayload {
  pet: Pet!
}

type Pet {
  id: ID!
  name: String!
}
```

```text
field Pet.Name { name }                                    returns data.name
field Query.PetNames { pets { id Name } }                  returns data.pets.map((pet) => pet.Name)
field Query.PetName($id: ID!) { pet(id: $id) { Name } }    returns data.pet?.Name ?? null
field Mutation.SetName($id: ID!, $name: String!) {
  setName(id: $id, name: $name) { pet { id Name } }
}                                                          returns data.setName.pet.Name
```

with `entrypoint Query.PetNames`, `Query.PetName`, `Mutation.SetName`, one source file per field.

### `@isograph/react` hooks (`test/*.test.tsx`)

`useLazyReference.test.tsx`, rendering `<IsographEnvironmentProvider><Loader /></IsographEnvironmentProvider>` where `Loader` calls `useLazyReference` and renders a `FragmentReader` inside `Suspense`:

- First render: the fallback; one network call; after the response, the names.
- A re-render with equal variables (a new object): no second call, and the same `fragmentReference` object.
- Two loaders with the same entrypoint and variables under one provider: one call.
- `IfNecessary` with the data already normalized: no call, and the first commit renders the names with no fallback.
- `shouldFetch: 'No'` on an empty store: no call, the fallback stays; then a second loader with `PetNames` and `'Yes'` mounts and its response arrives: both render the names.
- Unmount: `retainedQueries.size` and `subscriptions.size` are 0; the store equals `new Map()` after the collection open decision 1 chooses.
- Under `StrictMode`: the first render shows the fallback; two network calls (the remount's factory sends again while the first is in flight); after the second response, the names; `retainedQueries.size` is 1. Unmount: `retainedQueries.size` is 0.
- Unmount before the response, then the response: the store stays empty.
- A rejected request with an error boundary: the boundary renders the error; `onError` has one call.
- `onComplete` has one call with `["Rex", "Ash"]`.
- Variables change on a mounted `PetName` loader from `{ id: "1" }` to `{ id: "2" }`: a second call; the fallback, then pet 2's name; `subscriptions.size` is 1 (the first subscription was removed). This is the stale-reference fix.
- Degenerate: a response with `pets: []` renders an empty list.

`useImperativeReference.test.tsx`:

- Before `loadFragmentReference`, `fragmentReference` is `null` and the network has no call.
- `loadFragmentReference({ id: "1", name: "Ray" })` in `act` after commit: one call with `SetName`'s text and those variables; the component's `FragmentReader` renders `Ray`.
- A mounted `PetNames` reader re-renders with `Ray` after the mutation's response, through `callSubscriptions`.
- A second load: after its commit, `retainedQueries.size` counts only the second mutation and the mounted reader's query.

`useResult.test.tsx`:

- `whileInFlight: 'Suspend'` with the data present and `shouldFetch: 'Yes'`: the fallback until the response; with the default (`'ReadStore'`): the names at once.
- `onNetworkError: 'ReadStore'` with missing data and a rejected request: the fallback stays and no error boundary renders.
- A write that does not change the reader's data (a `PetName` response for a pet `PetNames` does not list): the `children` function is not called again.
- A write that changes it: one more call, with the new names.
- `useIsographEnvironment` with no provider throws upstream's message.

### Pet demo (`demos/pet-demo/test/App.test.tsx`)

Rendering `<App networkFunction={deferred} />`:

- One `HomeRoute` call with `{}`; after its response, the two button texts.
- Clicking `Mr Biggles: Pet me now, human`: one `PetDetailRoute` call with `{ id: "1" }`; after its response, `Mr Biggles`, `Intelligence: 95`, `Best friend: Nobody`, and no check-ins.
- Clicking `Home`: a second `HomeRoute` call when the package collects at disposal (open decision 1), then the two buttons.
- Clicking `Makayla Balicki: I AM HUNGRY`: `Best friend: Mr` and the check-ins `The park` and `The vet`.

## Changes

Each change is one commit, in order. Each package change adds its CI steps to `.github/workflows/js.yml` (`typecheck`, and `test` where the package has tests) and its pre-commit hooks (the runtime's two, with the package name and `files: '^libs/<directory>/'`), and runs `pnpm install` to update `pnpm-lock.yaml`.

### 1. `@isograph/react-disposable-state` (libs/isograph-react-disposable-state)

The port of upstream's package at `9d72f6fc` and its tests, with the import change above. Depends on network-and-subscriptions.md change 2.

### 2. `@isograph/react` core (libs/isograph-react)

The package files; `src/core/*`; `src/index.ts` exporting the core names; the fixture project and its generated artifacts; `vitest.config.ts`, `test/setup.ts`, `test/fixtures.tsx`. Depends on network-and-subscriptions.md change 6, reader-ast.md change 3, and 1.

### 3. `@isograph/react` hooks and components (libs/isograph-react)

`src/react/*`; the rest of `src/index.ts`; `test/useLazyReference.test.tsx`, `test/useImperativeReference.test.tsx`, `test/useResult.test.tsx`. Depends on 2.

### 4. The React pet demo (demos/pet-demo)

`isograph.config.json`, `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `src/App.tsx`, `src/routes.ts`, `src/HomeRouteLoader.tsx`, `src/PetDetailRouteLoader.tsx`, `src/mockNetwork.ts`, the two resolver bodies, `test/setup.ts`, `test/App.test.tsx`; the `test` CI step and hook for the pet demo. Depends on 3.

## Docs this doc amends

- `refactors/pending/runtime/store.md`: `stableStringify` is exported from the package index, and `exports` gains the `import` condition, with this doc's first import of runtime values.
- `refactors/pending/runtime/network-and-subscriptions.md`, Subscriptions: `Subscription` gains `listeners`, `listenToSubscription` is added and exported, and `callSubscriptions` calls the listeners after the callback (Reading, `useReadAndSubscribe`).

## Later

- `writeData`, upstream's write of data fetched elsewhere into a fragment reference (`core/writeData.ts:23-79`).
- `FragmentRenderer` and components: with `@component`, or with resolvers that return components in userland, and a per-reference component cache (`core/componentCache.ts`).
- The loadable hooks and pagination, with `@loadable` and refetch queries.
- A release buffer before collection (pico-runtime.md, Follow-ups 7).
- Replacing `readAndSubscribe`, `subscriptionSnapshot`, and `listenToSubscription` with pico-runtime.md's level 2 (`listen`, `snapshot`); the hooks keep their `useSyncExternalStore` shape.
- The pagination hooks, which pass `whileInFlight: 'Suspend'` where upstream passes `suspendIfInFlight: true`.

## Open decisions

### 1. When the React package collects

Releasing a runtime request unretains its query and does not collect (network-and-subscriptions.md); collection runs when a caller calls `garbageCollectEnvironment`.

- (a) After every disposal the React package makes. Cost: navigating back refetches what the previous screen released, as the demo shows, and every unmount walks every retained query.
- (b) Never automatically; the user calls `garbageCollectEnvironment`. Cost: the store grows until the user collects.
- (c) Upstream's buffer of 10 released queries (`garbageCollection.ts:39-53`, `IsographEnvironment.ts:177`), kept by the React package. Cost: reverses store.md's no-buffer decision for React users; a field and its size.

Not held.

### 2. What a mounted hook does with a `MissingData` or `Err` delivery

`readAndSubscribe` delivers `MissingData` and `Err` (network-and-subscriptions.md, Semantics), and `subscriptionSnapshot` returns them. Upstream's hook never sees either: its re-read throws and `logAnyError` swallows it (`core/subscribe.ts:47-61`). The hook keeps no copy of the data in React state (Reading, `useReadAndSubscribe`), so keeping the rendered data is the runtime's to do.

- (a) Keep the rendered data, upstream's outcome: the runtime adds a second snapshot function that returns the last `Ok` result, stable by the same rule, and `useSubscriptionSnapshot` passes it as `getSnapshot`. Cost: a component shows data the store no longer holds, a resolver error after mount is invisible, and a runtime function exists for one caller's policy.
- (b) Render the snapshot as it is (the snippets above): `MissingData` suspends until data arrives, and `Err` throws into the error boundary. Cost: a mounted component falls back to its `Suspense` fallback after a collection.
- (c) (b) for `Err` and (a) for `MissingData`. Cost: (a)'s runtime function, for `MissingData` only.

Not held. The snippets are written as (b), which is what `subscriptionSnapshot` gives; (a) and (c) change only the function `useSubscriptionSnapshot` passes as `getSnapshot`.

### 3. The demo's browser entry

- (a) Vite (`vite.config.ts`, `index.html`, `main.tsx`) and the vitest test (held). Upstream's `demos/vite-demo` is the shape.
- (b) The test only. Cost: no demo a person can open.
