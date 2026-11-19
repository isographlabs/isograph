# Runtime: the React package

Status: under discussion. Not approved for implementation.

`@isograph/react` (`libs/isograph-react`) is the React package over `@isograph/runtime`. It holds upstream's hooks and components for fetching an entrypoint and reading its result: `IsographEnvironmentProvider`, `useIsographEnvironment`, `createIsographEnvironment`, `useLazyReference`, `useImperativeReference`, `useResult`, `FragmentReader`, and the network layer they call. `@isograph/runtime` stays React-free (normalization-ast.md, The `@isograph/runtime` package). The package name is upstream's (`isograph/libs/isograph-react/package.json`); it is React-specific, so the name fits here.

The user's direction: upstream's APIs are held as they are (names, signatures, behavior, `shouldFetch`, `suspendIfInFlight`). Every change below names upstream's code (`path:line`), the v1 decision that forces it, and the minimal adaptation. Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src`; `disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`.

Prerequisites, in their own docs:

- `refactors/pending/graphql-protocol/normalization-ast.md` changes 1 to 4.
- `refactors/pending/runtime/store.md` changes 1 to 7: `Store`, `StoreLink` (`{ entityName, id }`), `Variables`, `normalize`, `check`, `garbageCollect`, `RetainedQuery`, `stableStringify`, required parameters.
- `refactors/pending/runtime/reader-ast.md` changes 1 to 6: `ReaderArtifact`, `ResolverParam`, `SelectionSetData`, `read`, `ReadResult`, `ReadMissingData`, real `resolver_reader.ts` artifacts, `NamedImport`, `DeclaredConst`, `ExportDefaultBinding`.
- `refactors/pending/runtime/pico-runtime.md` Step A (its open decision 10 (a)), with this doc's amendments. This doc's change 2 is Step A.

`refactors/pending/runtime/components-in-userland.md` does not exist at this commit. The demo's components below are plain React components that read resolver results with `FragmentReader`; if that doc lands with another shape, the demo follows it.

## End-user experience

The pet demo renders `HomeRoute` and `PetDetailRoute` in a browser (`pnpm --filter pet-demo dev`) with a mock network function and no server:

```tsx
// from demos/pet-demo/src/App.tsx
import {
  createIsographEnvironment,
  IsographEnvironmentProvider,
  type IsographNetworkFunction,
} from '@isograph/react';
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
- Clicking "Home" fetches `HomeRoute` again. Unmounting `HomeRouteLoader` released its query, and garbage collection ran at once (no release buffer, store.md `garbageCollect`), so the pets it fetched were collected before `PetDetailRoute`'s response arrived.
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
- `createIsographEnvironment` (`react/createIsographEnvironment.ts:10-23`): adapted. It takes the store and the network function. `missingFieldHandler` (store.md, Later), `logFunction` (no log events), and `componentFunction` (no `@component`) are dropped.
- `IsographEnvironment` (`core/IsographEnvironment.ts:73-93`): adapted (The environment).
- `IsographNetworkFunction`, `IsographOperation` (`core/IsographEnvironment.ts:103-106`, `core/entrypoint.ts:59-62`): adapted (The network function).
- `useLazyReference` (`react/useLazyReference.ts:19-60`): adapted (Fetching with `useLazyReference`).
- `useImperativeReference`, `UseImperativeReferenceResult` (`react/useImperativeReference.ts:25-100`): adapted.
- `useResult` (`react/useResult.ts:16-67`): adapted. Only the eager case remains.
- `useReadAndSubscribe` (`react/useReadAndSubscribe.ts:25-43`): adapted; it re-reads when the fragment reference changes.
- `useRerenderOnChange` (`react/useRerenderOnChange.ts:10-38`): adapted, with the same fix.
- `FragmentReader` (`react/FragmentReader.tsx:10-29`): adapted type parameters.
- `FragmentReference`, `ExtractParameters`, `stableIdForFragmentReference`, `StableIdForFragmentReference` (`core/FragmentReference.ts:35-79`): adapted.
- `FragmentReferenceOfEntrypoint`, `ExtractReadFromStore`, `ExtractClientFieldValue` (`core/entrypoint.ts:95-100`, `:200-203`): adapted to two type parameters.
- `FetchOptions`, `ShouldFetch` (`core/check.ts:13-27`): adapted. `optimisticNetworkResponse` is dropped (no store layers, store.md).
- `NetworkRequestReaderOptions` (`core/read.ts:972-975`): kept.
- `makeNetworkRequest` (`core/makeNetworkRequest.ts:159-340`): adapted (Fetching).
- `retainQuery`, `unretainQuery`, `garbageCollectEnvironment` (`core/garbageCollection.ts:39-63`, `:65`): adapted. `unretainQuery` returns nothing (no release buffer).
- `PromiseWrapper`, `readPromise`, `getPromiseState`, `wrapResolvedValue`, `wrapPromise`, `PromiseState`, `Result`, `NotSet`, `NOT_SET` (`core/PromiseWrapper.ts:1-90`): kept, with `AnyError` replaced by `unknown`.

### Moved to `@isograph/runtime`

- `check`, `CheckResult` (`core/check.ts:34-70`): store.md's `check`.
- `normalizeData`, `NetworkResponseObject`, `NetworkResponseValue`, `NetworkResponseScalarValue` (`core/cache.ts`): store.md's `normalize`, `JsonObject`, `JsonValue`.
- `subscribe`, `callSubscriptions` (`core/subscribe.ts:15-109`): pico-runtime.md Step A, amended here (Re-rendering on writes).
- `readButDoNotEvaluate`, `ReadDataResult` (`core/read.ts:60-143`): `readButDoNotEvaluate` returning `ReadResult<SelectionSetData>` (this doc), and reader-ast.md's `ReadResult`.
- `RetainedQuery` (`core/garbageCollection.ts:20-24`): store.md's `{ ast, variables }`.
- `StoreLink`, `StoreRecord`, `TypeName`, `DataId`, `DataTypeValueLinked`, `DataTypeValueScalar`, `IsographStore` (`core/IsographEnvironment.ts:122-175`): store.md's `StoreLink`, `StoreRecord`, `EntityName`, `RecordId`, `StoreValue`, `Store`.
- `Variables`, `VariableValue` (`core/FragmentReference.ts:10-21`): store.md's `Variables`, `JsonValue`.
- `UnknownTReadFromStore` (`core/FragmentReference.ts:23-27`): reader-ast.md's `ResolverParam<TEntityName>`, which every type parameter below is constrained by.
- `EagerReaderArtifact`, `TopLevelReaderArtifact`, `ReaderAst`, `ReaderAstNode`, `ReaderLinkedField`, `ReaderScalarField`, `ReaderNonLoadableResolverField`, `ResolverFirstParameter` (`core/reader.ts`): reader-ast.md's reader types.
- `NormalizationAst`, `NormalizationAstNode`, `NormalizationAstNodes`, `NormalizationLinkedField`, `NormalizationScalarField`, `NormalizationInlineFragment`, `Argument`, `ArgumentName`, `ArgumentValue`, `Arguments` (`core/entrypoint.ts`, `core/util.ts`): normalization-ast.md's types.
- `IsographEntrypoint` (`core/entrypoint.ts:77-93`): adapted, in `@isograph/runtime`'s `artifacts.ts`, landed with normalization-ast.md (The entrypoint artifact type).

### Dropped

- `ComponentReaderArtifact`, `FragmentRenderer`, `IsExactlyIntrinsicAttributes`, `ExtractProps`, `ExtractSecondParam`, `CombineWithIntrinsicAttributes`, `ComponentOrFieldName`, `FieldCache`: v1 has no `@component` (reader-ast.md). `FragmentRenderer` renders a resolver result as a component; upstream gives each fragment reference one stable component through `getOrCreateCachedComponent` (`core/componentCache.ts`), and without it a resolver that returns a new function on each read would remount on each render.
- `LoadableFieldReader`, `LoadableFieldRenderer`, `LoadableField`, `ReaderLoadableField`, `useClientSideDefer`, `useImperativeLoadableField`, `UseImperativeLoadableFieldReturn`, `useSkipLimitPagination`, `useConnectionSpecPagination` and their types, `NormalizationAstLoader`, `IsographEntrypointLoader`, `RequiredFetchOptions`, `RequiredShouldFetch`: v1 has no `@loadable` and no lazily loaded artifacts.
- `useImperativeExposedMutationField`, `UseImperativeExposedMutationFieldReturn`: v1 has no directives (`@exposeField`).
- `RefetchReaderArtifact`, `RefetchQueryNormalizationArtifact`, `RefetchQueryNormalizationArtifactWrapper`, `ReaderWithRefetchQueries`, `ReaderImperativelyLoadedField`, `NetworkRequestInfo`: v1 has no refetch queries.
- `ReaderLinkField`, `StableId`: v1 has no `__link` selectable and no pointers (reader-ast.md). Upstream's `Link<T>` (`IsographEnvironment.ts:117-120`) is store.md's `Link<TEntityName>`, without the brand on the id.
- `StartUpdate`, `ExtractStartUpdate`: v1 has no `@updatable`.
- `ROOT_ID`, `createIsographStore`: the root record comes from the AST, and the empty store is `new Map()` (store.md).
- `MissingFieldHandler`: store.md, Later.
- `EncounteredIds`, `WithEncounteredRecords`: store.md's `normalize` records no encountered ids.
- `Subscriptions`, `Subscription` (environment union), `FragmentSubscription`, `AnyChangesToRecordSubscription`, `AnyRecordSubscription`: the environment holds Step A's `Subscription`; nothing subscribes to a record.
- `CacheMap`, `StringifiedArgs`: the network request cache is a `Map` (The environment); nothing reads stringified arguments.
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
- No release buffer: disposal unretains and collects at once (`garbageCollection.ts:39-53` and `makeNetworkRequest.ts:565-577` become two calls).
- No log events: every `logMessage` call is dropped (`useLazyReference.ts:42-48`, `makeNetworkRequest.ts:201-206`, `:218-222`, `:265-269`, `:300-304`), with the module-level counter `networkRequestId` (`makeNetworkRequest.ts:44`) that only the logs read.
- Store shape: the environment holds store.md's `Store` (a `Map`), and the root record is `entrypoint.normalizationAst.root` (`{ entityName, id }`), not `{ __link: ROOT_ID, __typename: entrypoint.concreteType }` (`getOrCreateCacheForArtifact.ts:77`).
- Links `{ entityName, id }`, typed `Link<TEntityName>`: `FragmentReference.root` and the stable id read `entityName` and `id` (`FragmentReference.ts:78`), and every hook over a reference carries the reference's `TEntityName`.
- `read` returns `MissingData` instead of throwing: the runtime's `readButDoNotEvaluate` returns a `ReadResult`, and the React package turns `MissingData` into the thrown promise upstream's `readButDoNotEvaluate` throws (`read.ts:92-122`).
- No subscriptions in store.md: the suspend-until-change promise and re-render on write use Step A's subscriptions, amended (Re-rendering on writes).
- Required variables (store.md, Variables): the hooks' `variables: ExtractParameters<TReadFromStore>` is unchanged; the generated parameters type requires every variable, so a caller passes nullable ones too.
- A protocol-neutral runtime with no `any`: the network function resolves to the operation's data object, and the React package reads no `errors` or `data` key (The network function).

## Packages

```text
libs/isograph-disposable-types/       @isograph/disposable-types (port)
libs/isograph-react-disposable-state/ @isograph/react-disposable-state (port)
libs/isograph-react/                  @isograph/react
  package.json
  tsconfig.json
  vitest.config.ts
  src/index.ts
  src/core/check.ts
  src/core/entrypoint.ts
  src/core/FragmentReference.ts
  src/core/garbageCollection.ts
  src/core/getOrCreateCacheForArtifact.ts
  src/core/IsographEnvironment.ts
  src/core/makeNetworkRequest.ts
  src/core/PromiseWrapper.ts
  src/core/read.ts
  src/react/createIsographEnvironment.ts
  src/react/FragmentReader.tsx
  src/react/IsographEnvironmentProvider.tsx
  src/react/maybeUnwrapNetworkRequest.ts
  src/react/useImperativeReference.ts
  src/react/useLazyReference.ts
  src/react/useReadAndSubscribe.ts
  src/react/useRerenderOnChange.ts
  src/react/useResult.ts
  test/setup.ts
  test/fixtures.tsx
  test/makeNetworkRequest.test.ts
  test/useLazyReference.test.tsx
  test/useImperativeReference.test.tsx
  test/useResult.test.tsx
  fixtures/pets/isograph.config.json
  fixtures/pets/schema/schema.graphql
  fixtures/pets/src/*.ts
  fixtures/pets/src/__isograph/**   (generated)
```

`src/core/` holds the React-free code and `src/react/` the React code, as upstream does. The network layer stays in this package with the environment (open decision 1).

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
export { type FetchOptions, type ShouldFetch } from './core/check';
export {
  type ExtractClientFieldValue,
  type ExtractReadFromStore,
  type FragmentReferenceOfEntrypoint,
} from './core/entrypoint';
export {
  type ExtractParameters,
  type FragmentReference,
  stableIdForFragmentReference,
  type StableIdForFragmentReference,
} from './core/FragmentReference';
export {
  garbageCollectEnvironment,
  retainQuery,
  unretainQuery,
} from './core/garbageCollection';
export {
  type IsographEnvironment,
  type IsographNetworkFunction,
  type IsographOperation,
} from './core/IsographEnvironment';
export { makeNetworkRequest } from './core/makeNetworkRequest';
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
} from './core/PromiseWrapper';
export { type NetworkRequestReaderOptions } from './core/read';
export { createIsographEnvironment } from './react/createIsographEnvironment';
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
export { useRerenderOnChange } from './react/useRerenderOnChange';
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

If `tsc` rejects a generated param type as a `ResolverParam` (a parameters type whose input object has a `readonly w?: T | null | void` field, open decision 10), the implementer stops and raises it. The pet demo and the fixtures have no such variable.

## The environment

store.md's runtime has no environment object: its functions take the store and their inputs. The environment lives in `@isograph/react`, as upstream's does (`core/IsographEnvironment.ts:73-93`, in the React package), because it holds the cache of in-flight requests that renders share, which is React-specific (it exists for render without commit).

```ts
// from libs/isograph-react/src/core/IsographEnvironment.ts
import type { ParentCache } from '@isograph/react-disposable-state';
import type {
  JsonObject,
  RetainedQuery,
  Store,
  Subscription,
  Variables,
} from '@isograph/runtime';
import type { PromiseWrapper } from './PromiseWrapper';

/** The store, the network function, and what is retained, read, and in flight. */
export type IsographEnvironment = {
  readonly store: Store;
  readonly networkFunction: IsographNetworkFunction;
  /** Every query a fragment reference holds; `garbageCollect` keeps what these reach. */
  readonly retainedQueries: Set<RetainedQuery>;
  /** Every mounted reader and every suspended read, re-read after each write. */
  readonly subscriptions: Set<Subscription>;
  /** One cache per query text and variables, holding that request while a render or a
   * mounted `useLazyReference` uses it. */
  readonly networkRequestCache: Map<
    string,
    ParentCache<PromiseWrapper<void, unknown>>
  >;
};

/** Sends `operation` with `variables`. Resolves to the operation's data object, or rejects. */
export type IsographNetworkFunction = (
  operation: IsographOperation,
  variables: Variables,
) => Promise<JsonObject>;

export type IsographOperation = {
  readonly text: string;
};
```

```ts
// from libs/isograph-react/src/react/createIsographEnvironment.ts
import type { Store } from '@isograph/runtime';
import type {
  IsographEnvironment,
  IsographNetworkFunction,
} from '../core/IsographEnvironment';

export function createIsographEnvironment(
  store: Store,
  networkFunction: IsographNetworkFunction,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    retainedQueries: new Set(),
    subscriptions: new Set(),
    networkRequestCache: new Map(),
  };
}
```

Deltas from upstream's `IsographEnvironment` (`:73-93`) and `createIsographEnvironmentCore` (`:178-209`):

- `store` is store.md's `Store`, written in place, so the field is `readonly`; upstream reassigns `environment.store` to add layers (`makeNetworkRequest.ts:254`).
- `subscriptions` holds Step A's `Subscription`, not upstream's three-kind union.
- `fragmentCache: CacheMap<FragmentReference<any, any>>` is `networkRequestCache`, a `Map` of caches of network requests (Fetching with `useLazyReference`). The rename names what the cache holds now.
- Dropped: `componentFunction`, `componentCache`, `eagerReaderCache` (`@component`, `@updatable`), `missingFieldHandler`, `entrypointArtifactCache` (lazily loaded entrypoints), `gcBuffer`, `gcBufferSize` (no release buffer), `loggers`.

The user creates one environment per app, in a component's `useState` initializer (upstream's FAQ, `docs-website/docs/faq.md:11-15`, uses `useMemo`; the initializer runs once by contract).

```tsx
// external isograph/libs/isograph-react/src/react/IsographEnvironmentProvider.tsx
import * as React from 'react';
import type { ReactNode } from 'react';
import { createContext, useContext } from 'react';
import { type IsographEnvironment } from '../core/IsographEnvironment';

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

Copied verbatim from `react/IsographEnvironmentProvider.tsx:1-34`; the `IsographEnvironment` it imports is this doc's.

## The network function

Upstream: `IsographNetworkFunction = (operation, variables) => Promise<any>` (`IsographEnvironment.ts:103-106`). `makeNetworkRequest` reads `networkResponse.errors` (`:224-231`) and normalizes `networkResponse.data ?? {}` (`:246`, `:259`).

i2's network function resolves to the operation's data object, `JsonObject`, and rejects for anything else. Reasons: `any` is banned; the runtime is protocol-neutral and names no GraphQL key (normalization-ast.md, The discriminator), and `data` and `errors` are GraphQL's response envelope; store.md says a `null` `data` is the caller's to handle before `normalize`. The user's network function owns the envelope. A GraphQL one is upstream's demo function (`isograph/demos/pet-demo/pages/_app.tsx:12-46`) ending in `return json.data` after rejecting when `errors` is present or `data` is `null`. Open decision 6.

`operation` is `{ text }`: `IsographOperation`'s `kind` (`entrypoint.ts:59-62`) told it apart from `IsographPersistedOperation`, which i2 does not have. The package builds it as `{ text: entrypoint.queryText }`.

## Fetching

### `PromiseWrapper`

```ts
// external isograph/libs/isograph-react/src/core/PromiseWrapper.ts
export const NOT_SET = Symbol('NOT_SET');
export type NotSet = typeof NOT_SET;

export type Result<T, E> =
  | {
      readonly kind: 'Ok';
      readonly value: T;
    }
  | {
      readonly kind: 'Err';
      readonly error: E;
    };

/**
 * Invariant:
 * Before the promise is resolved, value becomes non-null.
 */
export type PromiseWrapper<T, E = unknown> = {
  readonly promise: Promise<Exclude<T, NotSet>>;
  result: Result<Exclude<T, NotSet>, E> | NotSet;
};

export interface PromiseWrapperOk<T, E = unknown> extends PromiseWrapper<T, E> {
  result: {
    readonly kind: 'Ok';
    readonly value: Exclude<T, NotSet>;
  };
}

export function wrapPromise<T>(
  promise: Promise<Exclude<T, NotSet>>,
): PromiseWrapper<T, unknown> {
  const wrapper: PromiseWrapper<T, unknown> = { promise, result: NOT_SET };
  promise
    .then((v) => {
      wrapper.result = { kind: 'Ok', value: v };
    })
    .catch((e: unknown) => {
      wrapper.result = { kind: 'Err', error: e };
    });
  return wrapper;
}

export function wrapResolvedValue<T>(
  value: Exclude<T, NotSet>,
): PromiseWrapperOk<T, never> {
  return {
    promise: Promise.resolve(value),
    result: {
      kind: 'Ok',
      value,
    },
  };
}

export function readPromise<T, E>(p: PromiseWrapper<T, E>): T {
  const { result } = p;
  if (result !== NOT_SET) {
    const resultKind = result;
    if (resultKind.kind === 'Ok') {
      return resultKind.value;
    } else {
      throw resultKind.error;
    }
  }

  throw p.promise;
}

export type PromiseState<T, E> =
  | {
      readonly kind: 'Pending';
      readonly promise: Promise<T>;
    }
  | Result<T, E>;

export function getPromiseState<T, E>(
  p: PromiseWrapper<T, E>,
): PromiseState<T, E> {
  if (p.result !== NOT_SET) {
    return p.result;
  }
  return {
    kind: 'Pending',
    promise: p.promise,
  };
}
```

Delta from `core/PromiseWrapper.ts:1-90`: `AnyError` (`= any`) is removed and `E` defaults to `unknown`; `wrapPromise`'s wrapper is `PromiseWrapper<T, unknown>` and its `catch` parameter is `unknown`; the comment `// TODO confirm suspense works if the promise is already resolved.` is dropped. `result` stays mutable: `wrapPromise` writes it.

### Fetch options

```ts
// from libs/isograph-react/src/core/check.ts
export type ShouldFetch = 'Yes' | 'No' | 'IfNecessary';

export const DEFAULT_SHOULD_FETCH_VALUE: ShouldFetch = 'IfNecessary';

export type FetchOptions<TReadOutData> = {
  readonly onComplete?: (data: TReadOutData) => void;
  readonly onError?: () => void;
  readonly shouldFetch?: ShouldFetch;
};
```

Delta from `core/check.ts:13-32`: `ShouldFetch` is spelled as its three members (upstream unions `RequiredShouldFetch`, which only `RequiredFetchOptions` uses); `FetchOptionsShared` is folded in; `optimisticNetworkResponse` and the `TRawResponseType` parameter are dropped (no store layers); the properties are `readonly`. Semantics are upstream's (`docs-website/docs/conditional-fetching.md`): `Yes` sends the request, `No` retains without sending, `IfNecessary` sends when `check` reports `MissingData`.

### Retention and collection

```ts
// from libs/isograph-react/src/core/garbageCollection.ts
import { garbageCollect, type RetainedQuery } from '@isograph/runtime';
import type { IsographEnvironment } from './IsographEnvironment';

export function retainQuery(
  environment: IsographEnvironment,
  queryToRetain: RetainedQuery,
): void {
  environment.retainedQueries.add(queryToRetain);
}

export function unretainQuery(
  environment: IsographEnvironment,
  retainedQuery: RetainedQuery,
): void {
  environment.retainedQueries.delete(retainedQuery);
}

export function garbageCollectEnvironment(
  environment: IsographEnvironment,
): void {
  garbageCollect(environment.store, Array.from(environment.retainedQueries));
}
```

Deltas from `core/garbageCollection.ts:39-120`: `unretainQuery` pushes nothing to a buffer and returns nothing (`DidUnretainSomeQuery` is gone); `garbageCollectEnvironment` calls store.md's `garbageCollect` with the retained queries, with no skip for optimistic layers (`:66-68`) or unloaded ASTs (`:70-76`); `RetainedQuery` is store.md's `{ ast, variables }`, whose root is `ast.root`.

### `maybeMakeNetworkRequest` and `makeNetworkRequest`

```ts
// from libs/isograph-react/src/core/makeNetworkRequest.ts
import type { ItemCleanupPair } from '@isograph/disposable-types';
import {
  callSubscriptions,
  check,
  type EntityName,
  type IsographEntrypoint,
  type NormalizationAst,
  normalize,
  read,
  type ResolverParam,
  type RetainedQuery,
  type Variables,
} from '@isograph/runtime';
import { DEFAULT_SHOULD_FETCH_VALUE, type FetchOptions } from './check';
import type { ExtractParameters } from './FragmentReference';
import {
  garbageCollectEnvironment,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import type { IsographEnvironment } from './IsographEnvironment';
import {
  type PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';

export function maybeMakeNetworkRequest<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions: FetchOptions<TClientFieldValue> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  switch (fetchOptions?.shouldFetch ?? DEFAULT_SHOULD_FETCH_VALUE) {
    case 'Yes':
      return makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
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
  }
}

export function retainQueryWithoutMakingNetworkRequest(
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
      if (status.kind !== 'Disposed') {
        status = unretainAndGarbageCollect(environment, status);
      }
    },
  ];
}

export function makeNetworkRequest<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions: FetchOptions<TClientFieldValue> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: retainArtifact(
      environment,
      entrypoint.normalizationAst,
      variables,
    ),
  };

  const promise = environment
    .networkFunction({ text: entrypoint.queryText }, variables)
    .then((response) => {
      if (status.kind === 'Disposed') {
        return;
      }
      normalize(
        environment.store,
        entrypoint.normalizationAst,
        response,
        variables,
      );
      callSubscriptions(environment.store, environment.subscriptions);

      const onComplete = fetchOptions?.onComplete;
      if (onComplete != null) {
        const data = readDataForOnComplete(environment, entrypoint, variables);
        try {
          onComplete(data);
        } catch {}
      }
    })
    .catch((error: unknown) => {
      try {
        fetchOptions?.onError?.();
      } catch {}
      throw error;
    });

  return [
    wrapPromise(promise),
    () => {
      if (status.kind !== 'Disposed') {
        status = unretainAndGarbageCollect(environment, status);
      }
    },
  ];
}

type NetworkRequestStatusUndisposed = {
  readonly kind: 'Undisposed';
  readonly retainedQuery: RetainedQuery;
};

type NetworkRequestStatusDisposed = {
  readonly kind: 'Disposed';
};

type NetworkRequestStatus =
  | NetworkRequestStatusUndisposed
  | NetworkRequestStatusDisposed;

/** The resolver's value for the response just normalized. */
function readDataForOnComplete<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
  variables: ExtractParameters<TReadFromStore>,
): TClientFieldValue {
  const result = read(
    environment.store,
    entrypoint.readerArtifact,
    entrypoint.normalizationAst.root,
    variables,
  );
  if (result.kind === 'MissingData') {
    throw new Error(
      `Reading ${entrypoint.name} right after normalizing its response found missing data: ` +
        `${result.reason}. This is indicative of a bug in Isograph.`,
    );
  }
  return result.value;
}

function retainArtifact(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  variables: Variables,
): RetainedQuery {
  const retainedQuery: RetainedQuery = { ast, variables };
  retainQuery(environment, retainedQuery);
  return retainedQuery;
}

function unretainAndGarbageCollect(
  environment: IsographEnvironment,
  status: NetworkRequestStatusUndisposed,
): NetworkRequestStatusDisposed {
  unretainQuery(environment, status.retainedQuery);
  garbageCollectEnvironment(environment);
  return { kind: 'Disposed' };
}
```

Deltas from `core/makeNetworkRequest.ts`:

- `maybeMakeNetworkRequest` (`:46-121`): no `RefetchQueryNormalizationArtifact` in the artifact union and no `readerWithRefetchQueries` parameter (no refetch queries); no `NormalizationAstLoader` throw (`:85-93`); `check` takes store.md's arguments, and the root is the AST's.
- `retainQueryWithoutMakingNetworkRequest` (`:123-157`) takes the AST, the only part of the entrypoint it reads.
- `makeNetworkRequest` (`:159-340`): the network function receives `{ text: queryText }` and resolves to the data object (The network function), so the `errors` check (`:224-231`) and `?? {}` are gone; no optimistic layer (`:190-198`, `:236-251`, `:309-316`, `:326-333`); no logs; `normalize` then Step A's `callSubscriptions` (upstream `:253-271`). `onComplete` runs only when the response was normalized. Upstream runs it after disposal too (`:280-297`), when the store no longer holds what the response wrote and `readDataForOnComplete` would suspend inside a `.then`.
- The statuses (`:342-360`): `UndisposedIncomplete` and `UndisposedComplete` are one `Undisposed`. Upstream distinguishes them for the optimistic layer's revert; without it the two behave the same at disposal and at the response.
- `readDataForOnComplete` (`:362-462`) reads with store.md's root and reader-ast.md's `read`, which calls the resolver; there is no component case and no fake fragment reference. A `MissingData` here is a broken invariant (Errors).
- `fetchNormalizationAstAndRetainArtifact` (`:464-493`) is `retainArtifact`: the AST is never loaded lazily, so "fetch" left the name.
- `unretainAndGarbageCollect` (`:565-577`) collects on every disposal (open decision 3).

## `FragmentReference`

```ts
// from libs/isograph-react/src/core/FragmentReference.ts
import {
  type EntityName,
  type Link,
  type ReaderArtifact,
  type ResolverParam,
  stableStringify,
  type Variables,
} from '@isograph/runtime';
import type { PromiseWrapper } from './PromiseWrapper';

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

export type ExtractParameters<T> = T extends {
  parameters: infer P extends Variables;
}
  ? P
  : Variables;

export type StableIdForFragmentReference = string;

export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<ResolverParam<EntityName>, unknown, EntityName>,
): StableIdForFragmentReference {
  return stableStringify([
    fragmentReference.root.entityName,
    fragmentReference.root.id,
    fragmentReference.fieldName,
    fragmentReference.variables,
  ]);
}
```

Deltas from `core/FragmentReference.ts:35-79`: `kind` is dropped (nothing reads it); `readerWithRefetchQueries` is `readerArtifact` (no refetch queries, no lazily loaded readers); `readerArtifactKind` is dropped (one kind); `TEntityName` is new, and `root` is a `Link<TEntityName>` where upstream's is a `StoreLink` (`{ __link, __typename: string }`, `core/FragmentReference.ts:58`, `core/IsographEnvironment.ts:122-125`), so `readerArtifact` and `root` name the same entity and `read` of the two type-checks (reader-ast.md, The parent entity's name); `TReadFromStore` is constrained by `ResolverParam<TEntityName>` instead of `UnknownTReadFromStore`; `ExtractParameters` falls back to store.md's `Variables`. The stable id is the stable JSON of a list instead of `/`-joined text, so an id or a name containing `/` cannot make two ids equal; it is the key form pico-runtime.md uses for `Store.observed`.

```ts
// from libs/isograph-react/src/core/entrypoint.ts
import type {
  EntityName,
  IsographEntrypoint,
  ResolverParam,
} from '@isograph/runtime';
import type { FragmentReference } from './FragmentReference';

export type FragmentReferenceOfEntrypoint<
  TEntrypoint extends IsographEntrypoint<ResolverParam<EntityName>, unknown, EntityName>,
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
    infer X extends ResolverParam<EntityName>,
    unknown,
    EntityName
  >
    ? X
    : never;
export type ExtractClientFieldValue<Type> =
  Type extends IsographEntrypoint<ResolverParam<EntityName>, infer X, EntityName>
    ? X
    : never;
```

Delta from `core/entrypoint.ts:95-100`, `:200-203`: three type parameters, and `unknown`, `EntityName`, or `ResolverParam<EntityName>` where upstream writes `any`. `FragmentReferenceOfEntrypoint` infers the three parameters in one conditional type: `FragmentReference<ExtractReadFromStore<T>, ExtractClientFieldValue<T>, ..>` fails `tsc` 5.6.3, because a conditional type is not known to satisfy `ResolverParam<TEntityName>`.

## Fetching with `useLazyReference`

```ts
// from libs/isograph-react/src/react/useLazyReference.ts
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import type {
  EntityName,
  IsographEntrypoint,
  ResolverParam,
} from '@isograph/runtime';
import { useMemo } from 'react';
import type { FetchOptions } from '../core/check';
import type {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { getOrCreateCacheForArtifact } from '../core/getOrCreateCacheForArtifact';
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

```ts
// from libs/isograph-react/src/core/getOrCreateCacheForArtifact.ts
import type { Factory } from '@isograph/disposable-types';
import { ParentCache } from '@isograph/react-disposable-state';
import {
  type EntityName,
  type IsographEntrypoint,
  type ResolverParam,
  stableStringify,
} from '@isograph/runtime';
import type { FetchOptions } from './check';
import type { ExtractParameters } from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';
import { maybeMakeNetworkRequest } from './makeNetworkRequest';
import type { PromiseWrapper } from './PromiseWrapper';

export function getOrCreateCacheForArtifact<
  TReadFromStore extends ResolverParam<TEntityName>,
  TClientFieldValue,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue, TEntityName>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): ParentCache<PromiseWrapper<void, unknown>> {
  const cacheKey = entrypoint.queryText + stableStringify(variables);
  const factory = () =>
    maybeMakeNetworkRequest(
      environment,
      entrypoint,
      variables,
      fetchOptions ?? null,
    );
  return getOrCreateItemInSuspenseCache(environment, cacheKey, factory);
}

export function getOrCreateItemInSuspenseCache(
  environment: IsographEnvironment,
  index: string,
  factory: Factory<PromiseWrapper<void, unknown>>,
): ParentCache<PromiseWrapper<void, unknown>> {
  const existing = environment.networkRequestCache.get(index);
  if (existing !== undefined) {
    return existing;
  }
  const created = new ParentCache(factory);
  environment.networkRequestCache.set(index, created);
  return created;
}
```

How it fetches: the first render with a cache key calls the factory through `useLazyDisposableState`, which temporarily retains the item (5000 ms, `disposable-state/CacheItem.ts:7`) and permanently retains it at commit. The factory is `maybeMakeNetworkRequest`: `IfNecessary` calls store.md's `check` on the entrypoint's AST and variables, sends the request on `MissingData`, and retains the query either way. Normalizing writes the response, then `callSubscriptions` re-reads every reader (Re-rendering on writes). A second render or a second component with the same query text and variables gets the same cached request. Unmount disposes the permanent retain; when no retain is left the cache item disposes the request, which unretains the query and collects.

Deltas from `react/useLazyReference.ts:19-60` and `core/getOrCreateCacheForArtifact.ts:26-86`:

- The signature drops `TNormalizationAst` and `TRawResponseType` and the conditional `...[fetchOptions]` tuple, which existed for `NormalizationAstLoader` (`RequiredFetchOptions` when the AST is loaded lazily). The return type is `{ fragmentReference }` without the `unknown` branch for the same reason.
- No `kind !== 'Entrypoint'` log (`:42-48`): no log events, and the argument is a declared `IsographEntrypoint`.
- The cache key is the query text and store.md's `stableStringify` of the variables, upstream's `operation.text + JSON.stringify(stableCopy(variables))` (`:42-54`) with i2's stable JSON (store.md, Store keys).
- The cache holds the network request, not the `FragmentReference` (`:55-84`), and the hook builds the reference from its own typed `entrypoint` and `variables`. Upstream's `fragmentCache` is `CacheMap<FragmentReference<any, any>>` (`IsographEnvironment.ts:81-83`), whose `any` lets one map hold every entrypoint's references; with `any` banned, a map of references needs a cast to the caller's type parameters. The request is untyped (`PromiseWrapper<void, unknown>`), so the map needs none. `useMemo` keeps the reference's identity per request: one cache key gives one request item, and one query text and variables give one entrypoint and equal variables (open decision 9).
- `getOrCreateItemInSuspenseCache` (`core/cache.ts:34-47`) reads and writes a `Map`.

## `useImperativeReference`

```ts
// from libs/isograph-react/src/react/useImperativeReference.ts
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import type {
  EntityName,
  IsographEntrypoint,
  ResolverParam,
} from '@isograph/runtime';
import type { FetchOptions } from '../core/check';
import type {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
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
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
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
        () => {
          disposeNetworkRequest();
        },
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
  readButDoNotEvaluate,
  type ReadMissingData,
  type ResolverParam,
  type SelectionSetData,
  subscribe,
} from '@isograph/runtime';
import type { FragmentReference } from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';
import { NOT_SET } from './PromiseWrapper';

export type NetworkRequestReaderOptions = {
  readonly suspendIfInFlight: boolean;
  readonly throwOnNetworkError: boolean;
};

export function getNetworkRequestOptionsWithDefaults(
  networkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): NetworkRequestReaderOptions {
  return {
    suspendIfInFlight: networkRequestOptions?.suspendIfInFlight ?? false,
    throwOnNetworkError: networkRequestOptions?.throwOnNetworkError ?? true,
  };
}

/** The data `fragmentReference`'s reader AST reads. On `MissingData`, throws the promise
 * that resolves when that read changes, or the network error. */
export function readOrSuspend<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  networkRequestOptions: NetworkRequestReaderOptions,
): SelectionSetData {
  const result = readButDoNotEvaluate(
    environment.store,
    fragmentReference.readerArtifact,
    fragmentReference.root,
    fragmentReference.variables,
  );
  if (result.kind === 'Ok') {
    return result.value;
  }
  if (
    !networkRequestOptions.suspendIfInFlight &&
    networkRequestOptions.throwOnNetworkError
  ) {
    const networkResult = fragmentReference.networkRequest.result;
    if (networkResult !== NOT_SET && networkResult.kind === 'Err') {
      throw new Error('NetworkError', { cause: networkResult.error });
    }
    throw new Promise<void>((resolve, reject) => {
      onNextChangeToFragmentData(environment, fragmentReference, result).then(
        resolve,
      );
      fragmentReference.networkRequest.promise.catch(reject);
    });
  }
  throw onNextChangeToFragmentData(environment, fragmentReference, result);
}

/** Resolves at the first write after which `fragmentReference`'s read is no longer
 * `MissingData`. */
export function onNextChangeToFragmentData<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  missing: ReadMissingData,
): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = subscribe(
      environment.store,
      environment.subscriptions,
      fragmentReference.readerArtifact,
      fragmentReference.root,
      fragmentReference.variables,
      missing,
      () => {
        unsubscribe();
        resolve();
      },
    );
  });
}
```

`NetworkRequestReaderOptions` and `getNetworkRequestOptionsWithDefaults` are `core/read.ts:972-984` verbatim, with `readonly` properties. `readOrSuspend` is upstream's `readButDoNotEvaluate` (`read.ts:60-129`) on the React side: the runtime's `readButDoNotEvaluate` returns the `ReadResult`, and this function throws what upstream throws. The deltas: no `readPromise` of the reader (`:70-72`), no log (`:85-90`), no `encounteredRecords` in the result, and the promise is `onNextChangeToFragmentData` instead of `onNextChangeToRecord(environment, response.recordLink)` (`cache.ts:135-149`).

Upstream suspends until any write touches the record the read stopped at; i2's `normalize` records no touched records, so the suspended read subscribes with Step A and resumes at the first write after which the read is no longer `MissingData`. A write that fills the record but leaves the read missing elsewhere resumes upstream's render, which suspends again; it does not resume i2's. The promise is created in the same synchronous call as the read that returned `MissingData`, so `subscribe`'s immediate comparison sees `MissingData` again and does not call the callback before `unsubscribe` is bound. A suspended read whose component never retries keeps its subscription until the data arrives, as upstream's `onNextChangeToRecord` keeps its record subscription.

```ts
// external isograph/libs/isograph-react/src/react/maybeUnwrapNetworkRequest.ts
import { getPromiseState, type PromiseWrapper } from '../core/PromiseWrapper';
import type { NetworkRequestReaderOptions } from '../core/read';

export function maybeUnwrapNetworkRequest(
  networkRequest: PromiseWrapper<void, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): void {
  const state = getPromiseState(networkRequest);
  if (state.kind === 'Err' && networkRequestOptions.throwOnNetworkError) {
    throw state.error;
  } else if (
    state.kind === 'Pending' &&
    networkRequestOptions.suspendIfInFlight
  ) {
    throw state.promise;
  }
}
```

Delta: `PromiseWrapper<void, unknown>` for `<void, any>`, and the return type.

What the two options do, upstream's semantics: `suspendIfInFlight` (default `false`) suspends while the request is pending even when the store has the data; `throwOnNetworkError` (default `true`) throws the request's error at render, and makes a `MissingData` read reject when the request rejects. With `suspendIfInFlight: false`, a read whose data the store already has renders it while the request is in flight.

### `useReadAndSubscribe` and `useRerenderOnChange`

```ts
// from libs/isograph-react/src/react/useReadAndSubscribe.ts
import type {
  EntityName,
  ResolverParam,
  SelectionSetData,
} from '@isograph/runtime';
import { useState } from 'react';
import {
  type FragmentReference,
  stableIdForFragmentReference,
  type StableIdForFragmentReference,
} from '../core/FragmentReference';
import { type NetworkRequestReaderOptions, readOrSuspend } from '../core/read';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { useRerenderOnChange } from './useRerenderOnChange';

/** The data last read, and the fragment reference it was read for. */
type ReadOutData = {
  readonly stableId: StableIdForFragmentReference;
  readonly data: SelectionSetData;
};

/**
 * Read the data from a fragment reference and subscribe to updates.
 */
export function useReadAndSubscribe<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  networkRequestOptions: NetworkRequestReaderOptions,
): SelectionSetData {
  const environment = useIsographEnvironment();
  const stableId = stableIdForFragmentReference(fragmentReference);
  const [readOutData, setReadOutData] = useState<ReadOutData>(() => ({
    stableId,
    data: readOrSuspend(environment, fragmentReference, networkRequestOptions),
  }));
  let current = readOutData;
  if (readOutData.stableId !== stableId) {
    current = {
      stableId,
      data: readOrSuspend(environment, fragmentReference, networkRequestOptions),
    };
    setReadOutData(current);
  }
  useRerenderOnChange(current.data, fragmentReference, (data) => {
    setReadOutData({ stableId, data });
  });
  return current.data;
}
```

```ts
// from libs/isograph-react/src/react/useRerenderOnChange.ts
import {
  type EntityName,
  type ResolverParam,
  type SelectionSetData,
  subscribe,
} from '@isograph/runtime';
import { useEffect } from 'react';
import {
  type FragmentReference,
  stableIdForFragmentReference,
} from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export function useRerenderOnChange<
  TReadFromStore extends ResolverParam<TEntityName>,
  TEntityName extends EntityName,
>(
  data: SelectionSetData,
  fragmentReference: FragmentReference<TReadFromStore, unknown, TEntityName>,
  setData: (data: SelectionSetData) => void,
): void {
  const environment = useIsographEnvironment();
  const stableId = stableIdForFragmentReference(fragmentReference);
  useEffect(
    () =>
      subscribe(
        environment.store,
        environment.subscriptions,
        fragmentReference.readerArtifact,
        fragmentReference.root,
        fragmentReference.variables,
        { kind: 'Ok', value: data },
        setData,
      ),
    [environment, stableId],
  );
}
```

Deltas from `react/useReadAndSubscribe.ts:25-43` and `react/useRerenderOnChange.ts:10-38`:

- No `readerAst` parameter: the fragment reference holds the reader artifact.
- The data is `SelectionSetData`, not `ExtractData<TReadFromStore>`. The hook passes it only to the resolver; typing it would need a cast from the runtime's untyped read.
- The state records the stable id it was read for. When the fragment reference's stable id changes, the render reads again (React's "adjust state during render" pattern) and the subscription effect re-runs (`[environment, stableId]`). Upstream reads only in the `useState` initializer (`useReadAndSubscribe.ts:33-35`) and subscribes once (`useRerenderOnChange.ts:31-37`, deps `[]`, "the fragment reference is stable for the life of the component"). That held because each upstream fragment reference rendered through its own cached component (`getOrCreateCachedComponent`), which remounts for a new reference. Without `@component`, `useResult` is called with a changing reference by the same component (a route whose `id` changes), and upstream's version would keep the old data and the old subscription.
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

## Re-rendering on writes: runtime additions

### `readButDoNotEvaluate`

Before (reader-ast.md, The read function):

```ts
// from libs/isograph-runtime/src/read.ts
export function read<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<TOutput> {
  if (store.get(root.entityName)?.get(root.id) === undefined) {
    return {
      kind: 'MissingData',
      reason: `No record for ${recordName(root)}`,
      recordLink: root,
    };
  }
  const data = readSelectionSet(store, artifact.readerAst, root, variables);
  if (data.kind === 'MissingData') {
    return data;
  }
  return ok(artifact.resolver({ data: data.value, parameters: variables, link: root }));
}
```

After:

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
    return {
      kind: 'MissingData',
      reason: `No record for ${recordName(root)}`,
      recordLink: root,
    };
  }
  return readSelectionSet(store, artifact.readerAst, root, variables);
}

export function read<TEntityName extends EntityName, TOutput>(
  store: Store,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  root: NoInfer<Link<TEntityName>>,
  variables: Variables,
): ReadResult<TOutput> {
  const data = readButDoNotEvaluate(store, artifact, root, variables);
  if (data.kind === 'MissingData') {
    return data;
  }
  return ok(artifact.resolver({ data: data.value, parameters: variables, link: root }));
}
```

The name is upstream's (`core/read.ts:60`), which also evaluates nested resolvers and leaves the root's to its caller (`useResult.ts:63-64`). `read`'s behavior is unchanged. `index.ts` exports `readButDoNotEvaluate`, and `stableStringify` (store.md's module export), which `@isograph/react` uses for cache keys and stable ids.

### Step A, amended

This doc implements pico-runtime.md's Step A with three amendments, each to hold upstream's behavior.

Before (pico-runtime.md, Step A):

```ts
// from libs/isograph-runtime/src/subscribe.ts
export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam<EntityName>, unknown, EntityName>;
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
  artifact: ReaderArtifact<ResolverParam<EntityName>, TOutput, EntityName>,
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

After:

```ts
// from libs/isograph-runtime/src/subscribe.ts
export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam<EntityName>, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The read the callback last received, or the caller rendered with. */
  delivered: ReadResult<SelectionSetData>;
  readonly callback: (data: SelectionSetData) => void;
};

/**
 * Re-reads now and calls `callback` when the read differs from `delivered`, then adds the
 * subscription to `subscriptions`. Returns the function that removes it.
 */
export function subscribe(
  store: Store,
  subscriptions: Set<Subscription>,
  artifact: ReaderArtifact<ResolverParam<EntityName>, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
  delivered: ReadResult<SelectionSetData>,
  callback: (data: SelectionSetData) => void,
): () => void;

/**
 * Re-reads every subscription with `readButDoNotEvaluate` and, when the read is `Ok` and
 * differs from `delivered`, stores it and calls the callback with its data. A `MissingData`
 * read changes nothing. After every subscription was re-read, throws the one error a re-read
 * threw, or an `AggregateError` of several.
 */
export function callSubscriptions(
  store: Store,
  subscriptions: ReadonlySet<Subscription>,
): void;
```

The amendments:

- `subscribe` takes `delivered`, the read the caller rendered with, and compares it at once. Upstream subscribes in an effect after render and re-reads on subscribe because writes between the render and the effect are otherwise missed (`subscribe.ts:32-36`). Step A's `subscribe` read `delivered` itself, at subscribe time, so such a write was never delivered.
- The callback receives the data, not the resolver's result. Upstream's subscription callback receives the data (`IsographEnvironment.ts:45-47`) and the hook calls the resolver in render (`useResult.ts:63-64`); a callback that also called the resolver would run it twice per change.
- A `MissingData` re-read calls nothing and leaves `delivered` as it was. Upstream's re-read throws a promise on `MissingData` (`read.ts:92-122`), which `logAnyError` catches (`subscribe.ts:47-61`), so a mounted component keeps the data it showed. A suspended read subscribes with `delivered` set to its `MissingData`, so the first `Ok` re-read differs and wakes it.

`delivered` compares by Step A's rule (upstream's `mergeObjectsUsingReaderAst`, `areEqualWithDeepComparison.ts:55-121`): an `Ok` read differs from a `MissingData` `delivered`; two `Ok` reads compare server values structurally and nested `Resolver` values with `===`. `callSubscriptions` iterates the live set, as upstream's `forEach` does (`subscribe.ts:67`); a callback that removes its own subscription (`onNextChangeToFragmentData`) is safe in a `Set` iteration. A re-read that throws (a nested resolver threw) does not stop the loop; the error is rethrown after it, pico-runtime.md's open decision 9 (a) rule. Upstream logs it and continues (`subscribe.ts:47-61`); with no logs, that would discard it (open decision 7). In `makeNetworkRequest`, that rethrow rejects the request.

`index.ts` exports `subscribe`, `callSubscriptions`, and `Subscription`.

Step A's costs stand: every write re-reads every subscription and re-runs each nested resolver under it.

## Disposal, StrictMode, and upstream's disposable-state bugs

Upstream does not support StrictMode (`docs-website/docs/faq.md:3-9`; `docs-website/docs/quickstart.md:72-86`). Held: unsupported (open decision 4). The demo's `main.tsx` renders without `StrictMode`, as upstream's `demos/vite-demo/src/main.tsx` does.

The following are traced from source, not reproduced:

- StrictMode throws. After the first commit, `useCachedResponsivePrecommitValue`'s effect permanently retains the item and `useLazyDisposableState` stores the pair (`disposable-state/useLazyDisposableState.ts:25-31`). StrictMode's simulated unmount runs `useLazyDisposableState`'s cleanup (`:33-44`), which clears the permanent retain; the item has no retain left, so it is removed from the parent cache and disposed (`CacheItem.ts:333-340`): the request is unretained and, in i2, the store is collected at once. The simulated remount re-runs `useCachedResponsivePrecommitValue`'s effect (`useCachedResponsivePrecommitValue.ts:52-92`): `permanentRetainIfNotDisposed` returns `null` for the disposed item (`CacheItem.ts:188-191`), the parent cache is empty, so it calls `parentCache.factory()` (a second request) and passes the pair to `onCommit`, which calls the stored pair's cleanup again and throws "A permanent retain should only be cleared once." (`CacheItem.ts:126-130`). The second request's pair is never stored, so it is never disposed.
- A render not committed within 5000 ms disposes its item (`CacheItem.ts:7`, `:248-251`). When the component that calls `useLazyReference` is itself suspended (a `Suspense` boundary above it, and its reader in the same component), a response slower than that is dropped (`status` is `Disposed`), and the read's subscription never fires. The demo places `Suspense` below the loader, as upstream's demo does (`isograph/demos/pet-demo/src/components/HomeRoute.tsx:32-47`).
- When the commit finds its item disposed, it creates one with `parentCache.factory()` outside the parent cache (`useCachedResponsivePrecommitValue.ts:84`), so another component with the same cache key sends another request.
- The first caller's `fetchOptions` win for a cache key: the factory closes over them (`getOrCreateCacheForArtifact.ts:55-84`), so a second component's `onComplete` with the same query and variables is not called.
- Cache entries are never removed: one empty `ParentCache` per distinct query text and variables stays in the map for the environment's life (`ParentCache.ts:32`, "TODO pass an onEmpty function").
- `useDisposableState` throws on its second `setState`: `cleanupItemCleanupPairRefAfterSetState` (`useDisposableState.ts:32-47`) runs on every state change and throws when the ref it nulled on the first `setState` is null. Nothing in `@isograph/react` calls `useDisposableState`, so it is not ported.
- `useReadAndSubscribe` keeps stale data when its fragment reference changes: fixed here, because v1 forces it (Reading).

Held for the rest: upstream's behavior. Each is listed so that a fix is a decision, not a surprise.

## The disposable-state packages

`@isograph/disposable-types` (`libs/isograph-disposable-types`) and `@isograph/react-disposable-state` (`libs/isograph-react-disposable-state`) are upstream's packages, ported (open decision 5).

Ported files: `isograph-disposable-types/src/index.ts`; `CacheItem.ts`, `ParentCache.ts`, `useCachedResponsivePrecommitValue.ts`, `useLazyDisposableState.ts`, `useUpdatableDisposableState.ts`, `useHasCommittedRef.ts`, and their tests `CacheItem.test.ts`, `ParentCache.test.ts`, `useCachedResponsivePrecommitValue.test.tsx`, `useLazyDisposableState.test.tsx`, `useUpdatableDisposableState.test.tsx`, under `src/` and `test/` of the new packages. `index.ts` exports what upstream's does (`export *` of each ported module and of `@isograph/disposable-types`).

Deltas, applied to every ported file:

- TypeScript's `private __x` is an ES private field `#x` (AGENTS.md, TypeScript): `CacheItem`'s `__state`, `__options`, `__maybeExitInParentCacheAndNotDisposedState`, `__maybeExitNotInParentCacheAndNotDisposedState`; `ParentCache`'s `__cacheItem`, `__factory`, `__options`, `__populateAndTemporaryRetain`.
- `ItemCleanupPair<T>` is `readonly [T, CleanupFn]`.
- `useDisposableState.ts` is not ported (no caller; its `setState` bug above).
- The type-level `tsTests` functions (`useUpdatableDisposableState.ts:123-134`), kept compiling by `@ts-ignore`, are not ported.
- Tests read private state through `(x as any).__state` and `(x as any).__cacheItem` (`CacheItem.test.ts:14-16`, `ParentCache.test.ts:7`, `useCachedResponsivePrecommitValue.test.tsx:10`, `:14`), which neither `#` fields nor the ban on `any` allow. Each such assertion becomes the assertion of its public consequence: `InParentCacheAndNotDisposed` is `removeFromParentCache` not called and `disposeItem` not called (for a `ParentCache`, `isEmpty()` is `false`); `NotInParentCacheAndNotDisposed` is `removeFromParentCache` called once, `disposeItem` not called, and `getValue()` returning the value; `NotInParentCacheAndDisposed` is `disposeItem` called once and `getValue()` throwing. Assertions on `temporaryRetainCount` and `permanentRetainCount` are dropped; each test's later dispose or expiry steps assert their consequence. A test left with no assertion is deleted, and the commit message names it.

Each package's `package.json` and `tsconfig.json` follow `@isograph/react`'s: `private`, source `exports` with `types` and `import`, `typecheck`; `@isograph/react-disposable-state` has `test` (`vitest run`), the peer `react` range, and dev dependencies `react`, `@types/react`, `react-test-renderer`, `@types/react-test-renderer`, `vitest`, `typescript` at `@isograph/react`'s versions; `@isograph/disposable-types` has no tests, as upstream's has none.

## The network request lifecycle, end to end

1. `useLazyReference` renders: `getOrCreateCacheForArtifact` returns the cache for `queryText + stableStringify(variables)`; `useLazyDisposableState` calls its factory on the first render.
2. The factory: `check`, then `makeNetworkRequest` or `retainQueryWithoutMakingNetworkRequest`. Both retain `{ ast, variables }` in `environment.retainedQueries`.
3. `FragmentReader` renders `useResult`: `maybeUnwrapNetworkRequest`, then `useReadAndSubscribe` reads with `readButDoNotEvaluate`; `MissingData` throws `onNextChangeToFragmentData`'s promise.
4. The response arrives: `normalize`, then `callSubscriptions`: the suspended read's subscription sees `Ok`, removes itself, and resolves; React retries and the read is `Ok`. The component commits; `useRerenderOnChange` subscribes with the rendered data.
5. A later write that changes the data calls the subscription's callback, which sets state; the component re-renders and calls the resolver on the new data.
6. Unmount: `useRerenderOnChange` unsubscribes; `useLazyReference`'s permanent retain is cleared, the cache item disposes the request, which unretains the query and runs `garbageCollect` over the remaining retained queries.

## Errors

Every throw this package adds or keeps, and its invariant:

- `useIsographEnvironment` outside a provider: upstream's message (`IsographEnvironmentProvider.tsx:25-34`). The invariant is the user's: hooks render under a provider.
- `readDataForOnComplete` on `MissingData`: "Reading `<name>` right after normalizing its response found missing data: `<reason>`. This is indicative of a bug in Isograph." Invariant: the entrypoint's reader reads only what its query selects, by construction (query-generation.md), and `normalize` wrote all of it or threw; the read runs synchronously after the write, so nothing deleted it in between.
- `readOrSuspend` and `maybeUnwrapNetworkRequest` throw promises and the network error. These are React's Suspense and error boundary protocol, upstream's behavior, not broken invariants.
- The ported disposable-state throws ("This indicates a bug in react-disposable-state.", "Calling setState before the component has committed is unsafe and disallowed."): upstream's, unchanged.
- store.md's `normalize` throws on a malformed response; in `makeNetworkRequest` that rejects the request, calls `onError`, and reaches readers through `throwOnNetworkError`.

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
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { mockNetworkFunction } from './mockNetwork';

const root = document.getElementById('root');
if (root == null) {
  throw new Error('index.html has no #root element.');
}
createRoot(root).render(<App networkFunction={mockNetworkFunction} />);
```

`main.tsx` is upstream's `vite-demo/src/main.tsx` with a null check in place of `!` and the network function passed in. No `StrictMode` (Disposal).

```ts
// from demos/pet-demo/src/mockNetwork.ts
import type { IsographNetworkFunction } from '@isograph/react';
import type { JsonObject } from '@isograph/runtime';
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

### Runtime (`libs/isograph-runtime/test`)

`read.test.ts` additions:

- `readButDoNotEvaluate` of `PetSummary` returns the data `read` passes to the root resolver; a spy on the root resolver has no call, and spies on `PetName` and `RecentCheckins` have one each.
- `readButDoNotEvaluate` on an absent root is `MissingData` with `No record for Query:__ROOT`; with an empty reader AST (`Constant` at `Pet:9` absent) too.

`subscribe.test.ts`, over store.md's fixture with `PetSummary`:

- `subscribe` with `delivered` equal to the current read: no call. With a `delivered` read before a `normalize` that changed `Pet:1`'s `name`: one synchronous call with the new data.
- `callSubscriptions` after a write that changes selected data: one call, with the data; `delivered` is the new read. After a write to unselected data (`PetLists` writing `Pet:9`): no call.
- After `Pet:1`'s `nickname` key is deleted by hand: no call, and `delivered` is unchanged; after a write restores the same value: no call; after a write with a different value: one call.
- A subscription with `delivered` a `MissingData` (the store empty): no call on subscribe; `normalize` of the response then `callSubscriptions`: one call.
- The returned function removes the subscription: no later call.
- Two subscriptions whose nested resolver throws on the second read: both are re-read, a third healthy subscription is called, and `callSubscriptions` throws an `AggregateError` of the two errors; with one throwing, it throws that error.
- A callback that removes its own subscription during `callSubscriptions`: the other subscriptions are still called.

### Compiler (`crates/typescript_artifacts`)

- `entrypoint.ts`'s lines, in order: `import type { IsographEntrypoint } from "@isograph/runtime";`, `import type { Query__HomeRoute__output_type } from "./output_type";`, `import type { Query__HomeRoute__param } from "./param_type";`, the three default imports; a line starting with `const artifact: IsographEntrypoint<Query__HomeRoute__param, Query__HomeRoute__output_type, "Query"> = {`; the last line `export default artifact;`. The existing assertions (`kind: "Entrypoint"`, `name: "Query.HomeRoute"`, the three keys and imports) are unchanged.
- `pnpm --filter pet-demo typecheck` passes over the regenerated artifacts.

### `@isograph/react` core (`test/makeNetworkRequest.test.ts`, no React)

- `IfNecessary` on an empty store: one network call with `{ text: PetNames.queryText }` and `{}`; `retainedQueries.size` is 1; after the response resolves, `getPromiseState` is `Ok` and `check` is `EnoughData`.
- `IfNecessary` after the same response was normalized: no call; the request is `Ok` at once.
- `Yes` with the data present: one call. `No` on an empty store: no call, one retained query.
- Dispose before the response: `retainedQueries.size` is 0; after the response resolves, the store equals `new Map()`, and `onComplete` has no call.
- Dispose after the response: `retainedQueries.size` is 0 and the store equals `new Map()`. Disposing twice changes nothing.
- Two requests (`PetNames`, `PetName` with `{ id: "1" }`), the first disposed: `Pet:1` remains with `name`; `Pet:2`, reached only by `PetNames`, is deleted.
- `onComplete` receives the resolver's value (`["Rex", "Ash"]`); an `onComplete` that throws leaves the request `Ok`.
- A rejected network call: the request is `Err` with that error; `onError` has one call; the store is unchanged.
- A response missing `pets`: the request is `Err` with store.md's malformed-response message; `onError` has one call.
- A subscription on `PetNames` added before the response: its callback has one call with the data after the response.

### `@isograph/react` hooks (`test/*.test.tsx`)

`useLazyReference.test.tsx`, rendering `<IsographEnvironmentProvider><Loader /></IsographEnvironmentProvider>` where `Loader` calls `useLazyReference` and renders a `FragmentReader` inside `Suspense`:

- First render: the fallback; one network call; after the response, the names.
- A re-render with equal variables (a new object): no second call, and the same `fragmentReference` object.
- Two loaders with the same entrypoint and variables under one provider: one call.
- `IfNecessary` with the data already normalized: no call, and the first commit renders the names with no fallback.
- `shouldFetch: 'No'` on an empty store: no call, the fallback stays; then a second loader with `PetNames` and `'Yes'` mounts and its response arrives: both render the names.
- Unmount: `retainedQueries.size` and `subscriptions.size` are 0, and the store equals `new Map()`.
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

- `suspendIfInFlight: true` with the data present and `shouldFetch: 'Yes'`: the fallback until the response; with the default (`false`): the names at once.
- `throwOnNetworkError: false` with missing data and a rejected request: the fallback stays and no error boundary renders.
- A write that does not change the reader's data (a `PetName` response for a pet `PetNames` does not list): the `children` function is not called again.
- A write that changes it: one more call, with the new names.
- `useIsographEnvironment` with no provider throws upstream's message.

### Pet demo (`demos/pet-demo/test/App.test.tsx`)

Rendering `<App networkFunction={deferred} />`:

- One `HomeRoute` call with `{}`; after its response, the two button texts.
- Clicking `Mr Biggles: Pet me now, human`: one `PetDetailRoute` call with `{ id: "1" }`; after its response, `Mr Biggles`, `Intelligence: 95`, `Best friend: Nobody`, and no check-ins.
- Clicking `Home`: a second `HomeRoute` call (the first query was released and collected), then the two buttons.
- Clicking `Makayla Balicki: I AM HUNGRY`: `Best friend: Mr` and the check-ins `The park` and `The vet`.

## Changes

Each change is one commit, in order. Each package change adds its CI steps to `.github/workflows/js.yml` (`typecheck`, and `test` where the package has tests) and its pre-commit hooks (the runtime's two, with the package name and `files: '^libs/<directory>/'`), and runs `pnpm install` to update `pnpm-lock.yaml`.

### 1. `readButDoNotEvaluate` and runtime exports (libs/isograph-runtime)

`readButDoNotEvaluate`; `read` calls it; `index.ts` exports it and `stableStringify`; `package.json` `exports` gains `import`; `read.test.ts` additions. Depends on reader-ast.md change 5.

### 2. Subscriptions: pico-runtime.md Step A, amended (libs/isograph-runtime)

`src/subscribe.ts` with `Subscription`, `subscribe`, `callSubscriptions`, its exports, and `test/subscribe.test.ts`. Depends on 1.

### 3. The entrypoint artifact type (libs/isograph-runtime, typescript_artifacts)

No work of its own: `IsographEntrypoint<TParam, TOutput, TEntityName>`, its printing in `entrypoint.ts`, and the regenerated artifacts landed with normalization-ast.md, and reader-ast.md change 3 types its `readerArtifact` (The entrypoint artifact type). The compiler test below is that landed one. Changes 6 and 7 depend on reader-ast.md change 3 in its place.

### 4. `@isograph/disposable-types` (libs/isograph-disposable-types)

The port, its `package.json` and `tsconfig.json`, the `typecheck` CI step and hook.

### 5. `@isograph/react-disposable-state` (libs/isograph-react-disposable-state)

The port and its tests, with the deltas above. Depends on 4.

### 6. `@isograph/react` core (libs/isograph-react)

The package files; `src/core/*`; `src/index.ts` exporting the core names; the fixture project and its generated artifacts; `vitest.config.ts`, `test/setup.ts`, `test/fixtures.tsx`; `test/makeNetworkRequest.test.ts`. Depends on 2, 3, and 5.

### 7. `@isograph/react` hooks and components (libs/isograph-react)

`src/react/*`; the rest of `src/index.ts`; `test/useLazyReference.test.tsx`, `test/useImperativeReference.test.tsx`, `test/useResult.test.tsx`. Depends on 6.

### 8. The React pet demo (demos/pet-demo)

`isograph.config.json`, `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `src/App.tsx`, `src/routes.ts`, `src/HomeRouteLoader.tsx`, `src/PetDetailRouteLoader.tsx`, `src/mockNetwork.ts`, the two resolver bodies, `test/setup.ts`, `test/App.test.tsx`; the `test` CI step and hook for the pet demo. Depends on 7.

## Docs this doc amends

- `refactors/pending/runtime/pico-runtime.md`: Step A is implemented by this doc's change 2 with the three amendments (Step A, amended); `Entrypoint<TOutput>` (section 3) is `IsographEntrypoint<TReadFromStore, TOutput, TEntityName>`, a declared type of the artifact rather than a structural one; open decision 10 is taken as (a) by this doc's dependency on it.
- `refactors/pending/runtime/reader-ast.md`: `read` is `readButDoNotEvaluate` then the resolver; `index.ts` exports `readButDoNotEvaluate`.
- `refactors/pending/runtime/store.md`: `stableStringify` is exported from the package index; `exports` gains the `import` condition. Its "No `IsographEnvironment`" holds for the runtime; the environment is `@isograph/react`'s.
- `refactors/pending/artifact-set-and-types.md`, Per entrypoint: `entrypoint.ts` is declared `IsographEntrypoint<P__f__param, P__f__output_type, "<root entity>">` and imports those two types and the type from `@isograph/runtime`.

## Later

- `writeData`, upstream's write of data fetched elsewhere into a fragment reference (`core/writeData.ts:23-79`).
- `FragmentRenderer` and components: with `@component`, or with resolvers that return components in userland, and a per-reference component cache (`core/componentCache.ts`).
- The loadable hooks and pagination, with `@loadable` and refetch queries.
- A network function that takes an `AbortSignal`, so disposal cancels the request (pico-runtime.md, Follow-ups 6).
- A release buffer before collection (pico-runtime.md, Follow-ups 7).
- Replacing Step A with pico-runtime.md's level 2 (`listen`, `snapshot`) and `useSyncExternalStore`.

## Open decisions

### 1. Where the network layer lives

- (a) In `@isograph/react`'s `src/core/`, with the environment (held). Upstream keeps it in `@isograph/react`'s `core/` (`core/makeNetworkRequest.ts`); store.md's runtime has no environment, and the environment holds the React-specific request cache. Cost: another binding (a Solid package) would move it to the runtime first.
- (b) In `@isograph/runtime`, as functions of the store, the network function, and the two sets, which the React environment passes. Cost: seven-parameter functions, or a runtime environment type that the React package extends with its cache.

### 2. How a read learns of writes

- (a) pico-runtime.md Step A, amended (held): upstream's mechanism, re-reading every subscription on every write. Cost: every write re-reads every mounted reader and re-runs its nested resolvers.
- (b) A per-environment set of listeners called after every write; the hook re-reads and compares itself. Cost: the comparison rule moves into the React package, and the runtime cannot test it without React.
- (c) Wait for pico-runtime.md's level 2 and `useSyncExternalStore`. Cost: no React package until changes 1 to 4 of pico-runtime.md land.

### 3. When garbage collection runs

- (a) At every disposal (held; v1 has no release buffer). Cost: navigating back refetches what the previous screen released, as the demo shows.
- (b) Never automatically; the user calls `garbageCollectEnvironment`. Cost: the store grows until the user collects.
- (c) Restore upstream's buffer of 10 released queries (`garbageCollection.ts:39-53`, `IsographEnvironment.ts:177`). Cost: reverses store.md's decision; an environment field and its size.

### 4. StrictMode

- (a) Unsupported, as upstream (held). Cost: users must disable it; React's default templates enable it.
- (b) Support it: `useLazyDisposableState`'s cleanup nulls its pair, and the effect that re-runs on the simulated remount re-acquires the item from the parent cache or the factory. Cost: two requests per mount in development, as upstream's FAQ predicts (`faq.md:9`), and a delta from upstream's disposable-state semantics.

### 5. Packaging of the disposable state

- (a) Two packages, as upstream (held).
- (b) Inside `@isograph/react` under `src/disposable-state/`. Cost: a reader of upstream's code finds it elsewhere; one package fewer to maintain.

### 6. What the network function returns

- (a) The operation's data object; the user's function rejects on errors or `null` data (held). Reason: no `any`, and the runtime names no GraphQL envelope key.
- (b) Upstream's GraphQL envelope, `{ data, errors }`, read by `makeNetworkRequest`. Cost: GraphQL names in the network layer, and a typed envelope.

### 7. An error thrown while re-reading a subscription

- (a) Re-read every subscription, then rethrow (held; pico-runtime.md open decision 9 (a)). Cost: in `makeNetworkRequest` the request rejects after its response was normalized, so readers of that request with `throwOnNetworkError` show the error at their next render.
- (b) Swallow it, upstream's behavior without its log. Cost: the error is invisible.
- (c) Deliver it to the subscription whose re-read threw, so that component's render throws it into its error boundary. Cost: the callback and the hook's state carry an error case.

### 8. The demo's browser entry

- (a) Vite (`vite.config.ts`, `index.html`, `main.tsx`) and the vitest test (held). Upstream's `demos/vite-demo` is the shape.
- (b) The test only. Cost: no demo a person can open.

### 9. What the request cache holds

- (a) The network request; the hook builds the `FragmentReference` (held). No cast.
- (b) The `FragmentReference`, as upstream (`getOrCreateCacheForArtifact.ts:69-82`). Cost: the map holds references of every entrypoint, so reading one back needs a cast to the caller's type parameters.

### 10. Input object fields in a parameters type

store.md keeps input object fields as `readonly w?: T | null | void`. A parameters type with such a field may not be assignable to `Variables` (`void` is not a `JsonValue`), which `ExtractParameters` and `IsographEntrypoint`'s `ResolverParam` constraint require. Options: (a) input object fields become `readonly w?: T | null` and the optional key is omitted when unset; (b) `Variables` admits `undefined` in input objects and `stableStringify` drops it, as `JSON.stringify` already does. Neither the pet demo nor the fixtures has such a variable. Not held.

### 11. The two booleans of `NetworkRequestReaderOptions`

`suspendIfInFlight` and `throwOnNetworkError` are `boolean` (AGENTS.md, Booleans). They are upstream's public options, which the user's direction holds by name and type; the ported disposable-state code keeps upstream's internal booleans (`cleared`, `hasCommittedRef`). The alternative is an enum per option (`InFlight: 'Suspend' | 'RenderStoreData'`, `NetworkError: 'Throw' | 'Suspend'`), which changes upstream's API. Held: upstream's booleans.
