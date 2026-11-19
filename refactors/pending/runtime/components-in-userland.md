# Components returned by fields

Status: exploration. This is not a plan for v1 and not approved for implementation. It lays out the design space, a concrete shape for the option that keeps component identity, and open questions.

The question: what happens when a client field's resolver returns a React component. React compares an element's `type` with `Object.is`; when the type at one position of the tree changes, React unmounts the old subtree (its state is lost and its effects are cleaned up) and mounts the new one. A field that returns a component must therefore return the same function on every read for as long as the component is meant to keep its state. Something has to hold that function. This doc asks where, and whether userland can do it in i2, where v1 has no directives (`refactors/pending/iso-directives-unexpected.md`) and `@isograph/runtime` has no React knowledge (`refactors/pending/runtime/store.md`, `refactors/pending/runtime/reader-ast.md`).

The answer turns on one distinction (Two kinds of resolver result): whether the field's result is a closure that reads the field's data when the closure is executed, or a value computed from data read when the resolver runs. Every option below produces one of the two, or is storage for one.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`. "Change 1" to "change 4" and "Step A" are those of `refactors/pending/runtime/pico-runtime.md`. `readButDoNotEvaluate`, `readAndSubscribe`, `SubscriptionResult`, and `IsographEnvironment` are `refactors/pending/runtime/network-and-subscriptions.md`'s (Step A, amended). `subscriptionSnapshot` is network-and-subscriptions.md's too, and `listenToSubscription` is the addition to it that react.md names. `useIsographEnvironment`, `onNextChangeToFragmentData`, `useSubscription`, `useSubscriptionSnapshot`, and `useResult` are `refactors/pending/runtime/react.md`'s.

## What upstream does

A client field declared with `@component` gets a `ComponentReaderArtifact` (`reader.ts:39-51`), whose resolver takes the field's param and the component's props:

```ts
// external isograph/libs/isograph-react/src/core/reader.ts
export type ComponentReaderArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TComponentProps extends Record<string, unknown> = Record<PropertyKey, never>,
> = {
  readonly kind: 'ComponentReaderArtifact';
  readonly fieldName: ComponentOrFieldName;
  readonly readerAst: ReaderAst<TReadFromStore>;
  readonly resolver: (
    data: ResolverFirstParameter<TReadFromStore>,
    runtimeProps: TComponentProps,
  ) => React.ReactNode;
  readonly hasUpdatable: boolean;
};
```

The compiler writes it in `eager_reader_artifact.rs:131-153` (`isograph/crates/artifact_content/src`), the branch taken when the field's directive set is not `None` (`:100-104`), types the field's output as `React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>` (`generate_artifacts.rs:623-626`), and gives the iso overload `IdentityWithParamComponent` (`iso_overload_file.rs:97-102`, `:156-168`). Query generation merges a component field's selections into the parent query as for any client field (`merge_client_scalar_field`, `isograph/crates/isograph_schema/src/create_merged_selection_set.rs:932`, which has no case on `@component`).

When a parent's read reaches a `Resolver` node whose artifact is a component, it does not read the component's data. It returns the cached component:

```ts
// external isograph/libs/isograph-react/src/core/read.ts
    case 'ComponentReaderArtifact': {
      return {
        kind: 'Success',
        data: getOrCreateCachedComponent(
          environment,
          fragment,
          networkRequestOptions,
        ),
      };
    }
```

(`read.ts:603-612`; the `EagerReaderArtifact` case above it, `:568-602`, reads the child's data and calls its resolver.) `useResult` (`react/useResult.ts:29-35`) and `readDataForOnComplete` (`makeNetworkRequest.ts:421-440`) call the same function.

The cache:

```ts
// external isograph/libs/isograph-react/src/core/componentCache.ts
export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<any, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): React.FC<any> {
  // We create startUpdate outside of component to make it stable
  const startUpdate = createStartUpdate(
    environment,
    fragmentReference,
    networkRequestOptions,
  );

  return (environment.componentCache[
    stableIdForFragmentReference(fragmentReference)
  ] ??= environment.componentFunction(
    environment,
    fragmentReference,
    networkRequestOptions,
    startUpdate,
  ));
}
```

```ts
// external isograph/libs/isograph-react/src/core/FragmentReference.ts
export function stableIdForFragmentReference(
  fragmentReference: FragmentReference<any, any>,
): StableIdForFragmentReference {
  return `${fragmentReference.root.__typename}/${fragmentReference.root.__link}/${fragmentReference.fieldName}/${JSON.stringify(stableCopy(fragmentReference.variables))}`;
}
```

- The key (`FragmentReference.ts:75-79`) is the record's entity name and id, the field name, and the variables. It does not include the network request or the read options.
- `componentCache` is a plain object, `FieldCache<React.FC<any>>` (`IsographEnvironment.ts:38-40`, `:78`), created empty in `createIsographEnvironmentCore` (`:199`), and held for the environment's lifetime.
- The function stored is `componentFunction` (`react/useReadAndSubscribe.ts:88-130`, injected at `react/createIsographEnvironment.ts:19` so that `core` does not import React at runtime). Its `Component` reads its own data: `useReadAndSubscribe` reads in a `useState` initializer and subscribes in an effect (`react/useReadAndSubscribe.ts:25-43`, `react/useRerenderOnChange.ts:21-37`), and a read that finds `MissingData` throws a promise, so the component suspends at its own Suspense boundary (`read.ts:92-122`). It then calls the resolver with `{ data, parameters, startUpdate }` and the props it was rendered with (`:115-125`), and sets `displayName` to `` `${fieldName} (type: .., id: ..) @component` `` (`:127-128`).

Whether it leaks: yes. The only references to `componentCache` in `libs/isograph-react/src` are its type (`IsographEnvironment.ts:78`), its creation (`:199`), and the `??=` (`componentCache.ts:19-26`). Nothing deletes an entry. `garbageCollectBaseStoreLayer` deletes store records and empty entity maps only (`garbageCollection.ts:88-119`). Every (entity, id, field, variables) ever read keeps one function, and that function closes over the environment, the first `FragmentReference` (with its `networkRequest`), the first call's `networkRequestOptions`, and a `startUpdate`, after its record is collected. `eagerReaderCache` has the same shape and the same lifetime (`startUpdate.ts:48-54`).

Two consequences of the `??=`:

- The first caller's `networkRequest` and options win. A later read of the same field on the same record under another request receives a component that waits on the first request (`maybeUnwrapNetworkRequest(fragmentReference.networkRequest, ..)`, `react/useReadAndSubscribe.ts:95-98`).
- Identity survives garbage collection. A record collected and fetched again gets the old function back, so a mounted component is never remounted by a collection.

`getOrCreateCachedComponent` also builds a `startUpdate` on every call, a cache hit included, and drops it (`componentCache.ts:13-17`).

## The user's idea

The component's stable location is determined by the record id, the field, and the variables. That is a resolver store key on a record (pico-runtime.md, The resolver store key). Store the component there, on the record, as change 2 stores a resolver result, and it is deleted with the record: change 3's `garbageCollect` deletes the records no retention reaches, and with them every resolver entry they hold, and sweeps resolver entries on surviving records that no retained or observed location reaches.

The rest of this doc asks how much of that userland can do without `@component` and without runtime code that knows about components. The closure field below needs runtime code that knows that a resolver can create a wrapper; it needs none that knows React.

## What a resolver has in i2

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ResolverParam<TEntityName extends EntityName = EntityName> = {
  readonly data: SelectionSetData;
  readonly parameters: Variables;
  /** The record the resolver's reader AST was read at. */
  readonly link: Link<TEntityName>;
};
```

(reader-ast.md, The artifact's TypeScript types; A resolver's own link.) A resolver receives its selections, its variables, and its own record's link. It does not receive the store or the record. The link works for every record: a `PetStats` record's link carries its derived id `Pet:1.stats` (store.md, Record ids), which no selectable holds, and on a field of an abstract entity the link names the concrete entity, which the store does not otherwise keep (store.md, The store). `link` is typed `Pet____link__output_type`, `Link<"Pet">` (store.md change 6). A nested record's link comes from a one-line client field on that entity that returns `link`, selected in the nested selection set (reader-ast.md, A resolver's own link).

## Two kinds of resolver result

(a) A closure that reads its data when the closure is executed. The field's result is a function. When React calls it (a component, at render) or code calls it (a plain function), the function reads the field's selections from the store at that moment and computes from what it read.

- The field's result is computed from the record's link, the field's reader artifact, and the field's variables. None of them changes for one record, field, and variables, so the result is the same function for that triple for as long as it is stored. Stability holds by construction, with no memo that compares data.
- The data dependency moves from the field to the closure's execution. The parent that selects the field does not read the field's selections and does not depend on them. Whoever executes the closure reads them, and a component that must re-render on a change subscribes to them itself.
- Upstream's `@component` is this kind. The parent's read returns the cached component and does not read the component's data (`read.ts:603-612`); the component reads at render through `useReadAndSubscribe` (`react/useReadAndSubscribe.ts:88-130`).

(b) A value computed from data read eagerly, when the resolver runs. The runtime reads the field's selections, then calls the resolver with them. That happens at read time under v1 (reader-ast.md, The read function), during render under react.md's `useResult`, or at normalization time under changes 2 and 3, where settlement computes every resolver result a retained query reaches.

- The field's result changes whenever its data changes: the resolver is called again with the new data, and under change 2 an entry whose dependency changed is recomputed.
- A component built this way is a new function after each change of its data, and React remounts it. A component that keeps the first function forever shows the first data forever. Why closing over data breaks stability, below, works this through.
- Its parent depends on the component's data, so a data change also reaches the parent. Under change 2, with `===` result equality, every component-returning ancestor recomputes and remounts too (Option 4).

The options this doc considered before are placed under this split in The earlier options under the split. The design for (a) comes first.

## A field whose result is a closure

The user's function receives the field's data and the caller's arguments. The runtime does not call it at read. It creates, once per record, field, and variables, a wrapper that reads the field's data through the field's reader artifact and the record's link when the wrapper is executed, and calls the user's function with that data. The wrapper is the field's value, stored on the record, and freed with it.

### End-user experience

```tsx
// from demos/pet-demo/src/PetCard.tsx
import { component } from '@isograph/react';

export const PetCard = iso(`
  field Pet.PetCard {
    PetName
    tagline
  }
`)(component(function PetCard({ data }, props: { readonly selected: boolean }) {
  return (
    <div className={props.selected ? 'selected' : undefined}>
      {data.PetName.name}: {data.tagline}
    </div>
  );
}));
```

```tsx
// from demos/pet-demo/src/HomeRoute.tsx
import { component } from '@isograph/react';

export const HomeRoute = iso(`
  field Query.HomeRoute {
    pets {
      id
      PetCard
    }
  }
`)(component(function HomeRoute({ data }, props: { readonly selectedId: string | null }) {
  return data.pets.map((pet) => (
    <pet.PetCard key={pet.id} selected={pet.id === props.selectedId} />
  ));
}));
```

One iso literal per component, data in the first parameter, props in the second, as upstream's `@component` (A `@component` directive). `pet.PetCard` is typed `React.FC<{ readonly selected: boolean }>`. When `Pet:1.tagline` changes, `PetCard` on `Pet:1` re-renders and `HomeRoute` does not: `HomeRoute`'s data holds `PetCard`'s stored wrapper, which is the same function before and after the write.

A plain function is the same mechanism without React:

```ts
// from demos/pet-demo/src/shareText.ts
import { closure } from '@isograph/runtime';

export const shareText = iso(`
  field Pet.shareText {
    PetName
    tagline
  }
`)(closure(function shareText({ data }, prefix: string) {
  return `${prefix} ${data.PetName.name}: ${data.tagline}`;
}));
```

`data.shareText` is typed `(prefix: string) => ReadResult<string>`. Calling it reads `PetName` and `tagline` at the time of the call, so an event handler that calls it after a write sees the written values.

### The resolver's two forms

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
> = {
  /** The client field's selectable name: the prefix of its resolver store key. */
  readonly selectableName: string;
  readonly readerAst: ReaderSelectionSet;
  readonly resolver: EagerResolver<TParam, TOutput> | ClosureResolver<TParam, TOutput>;
};

/**
 * A resolver `read` calls with the data it read. Written as React's `bivarianceHack`
 * (`@types/react`'s `EventHandler`) so that TypeScript compares `TParam` bivariantly, as it
 * compared the method signature this replaces (reader-ast.md, Why `resolver` is a method
 * signature).
 */
export type EagerResolver<TParam, TOutput> = {
  bivarianceHack(param: TParam): TOutput;
}['bivarianceHack'];

/**
 * A resolver whose value is a wrapper `read` creates once per record, field, and variables,
 * without reading the field's selections.
 */
export type ClosureResolver<TParam extends ResolverParam, TWrapper> = {
  readonly kind: 'Closure';
  /** Called once per record, field, and variables. A method signature, so it is bivariant
   * in `TParam`. */
  createWrapper(reference: ClosureReference<TParam>, store: Store): TWrapper;
};

/** What a wrapper reads with when it is executed. */
export type ClosureReference<TParam extends ResolverParam> = {
  /** The closure field's own reader artifact. Its reader AST is the field's selections. */
  readonly artifact: ReaderArtifact<TParam, unknown, EntityName>;
  readonly link: TParam['link'];
  /** The field's variables: `childVariables` of the selecting node's arguments. */
  readonly parameters: TParam['parameters'];
};
```

Deltas from reader-ast.md's `ReaderArtifact`: `selectableName` is change 2's field, brought forward because the wrapper's store key needs it; `resolver` is a property whose type is a union of the two forms, instead of a method signature. `EagerResolver` is the user's function as v1 has it. `ClosureResolver` is an object made by a helper (`closure` in `@isograph/runtime`, `component` in `@isograph/react`), which the generated `resolver_reader.ts` imports through `resolver: resolver` exactly as it imports a function today. The compiler does not know which form a field has.

### Reading

`read` (reader-ast.md, The read function) returns the stored wrapper for a closure resolver, and reads the selections only for an eager one. Before:

```ts
// from libs/isograph-runtime/src/read.ts
  const data = readSelectionSet(store, artifact.readerAst, root, variables);
  if (data.kind === "MissingData") {
    return data;
  }
  return ok(artifact.resolver({ data: data.value, parameters: variables, link: root }));
```

After:

```ts
// from libs/isograph-runtime/src/read.ts
  const resolver = artifact.resolver;
  if (typeof resolver !== "function") {
    return ok(closureWrapper(store, record, artifact, resolver, root, variables));
  }
  const data = readSelectionSet(store, artifact.readerAst, root, variables);
  if (data.kind === "MissingData") {
    return data;
  }
  return ok(resolver({ data: data.value, parameters: variables, link: root }));
```

`record` is the record `read` already looks up for its absent-record check, bound to a `const` instead of compared and dropped.

```ts
// from libs/isograph-runtime/src/closure.ts
/**
 * The wrapper stored on `record` under the field's resolver store key, created by
 * `resolver.createWrapper` the first time the field is read there with `variables`.
 */
export function closureWrapper<TEntityName extends EntityName, TWrapper>(
  store: Store,
  record: StoreRecord,
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TWrapper, TEntityName>,
  resolver: ClosureResolver<ResolverParam<TEntityName>, TWrapper>,
  root: Link<TEntityName>,
  variables: Variables,
): TWrapper {
  const key = resolverStoreKey(artifact.selectableName, variables);
  const stored = record.get(key);
  if (stored === undefined) {
    const wrapper = resolver.createWrapper({ artifact, link: root, parameters: variables }, store);
    record.set(key, { kind: "Closure", wrapper, timeUpdated: store.epoch });
    return wrapper;
  }
  if (stored.kind !== "Closure") {
    throw new Error(`${artifact.selectableName} on ${recordName(root)} holds a ${stored.kind} entry`);
  }
  // `key` on `record` is written only here, by `artifact`: one selectable of one entity has
  // one reader artifact.
  return stored.wrapper as TWrapper;
}
```

The throw is store.md's kind of throw (Errors): a client field's store key is never a server store key (change 1, The store), and a field whose resolver is a closure never stores a `ResolverEntry`, so only a broken invariant reaches it.

The `Resolver` case of `readSelection` is unchanged: it calls `read`, which now returns the wrapper. A parent that selects a closure field therefore never reads that field's selections, and a `MissingData` inside them does not end the parent's read.

`readButDoNotEvaluate` (network-and-subscriptions.md, Reading without the root resolver) is unchanged. For a closure field's artifact it reads the field's selections and calls nothing, which is the read a wrapper makes when it is executed.

### What is stored, and how it is freed

```ts
// from libs/isograph-runtime/src/store.ts
export type StoreEntry = ServerEntry | ResolverEntry | ClosureEntry;

/** A closure field's wrapper, created once by its resolver's `createWrapper`. */
export type ClosureEntry = {
  readonly kind: 'Closure';
  readonly wrapper: unknown;
  /** The epoch it was created in. */
  readonly timeUpdated: Epoch;
};
```

The entry lives on the record the field was read on, under `resolverStoreKey(artifact.selectableName, variables)` (change 2, The resolver store key): `PetCard` on `Pet:1` is `PetCard`; a closure field with `$limit: Int = 3` selected with no argument is `F____limit___3`. Two selections of the field with equal argument values on one record share one wrapper, whatever their aliases.

This needs change 1's tagged entries and nothing else of changes 1 to 4. v1's `StoreRecord` holds untagged `StoreValue`s (store.md, The store), which are JSON, so a function cannot be stored in it without widening every walk's value type. Before change 2, `ResolverEntry` is not yet in the union, and `timeUpdated` has no reader; change 2's dependency check reads it.

Freeing:

- store.md's `garbageCollect` deletes a record no retained query reaches, with every entry on it, the wrapper's included. Nothing else holds the wrapper except a parent's data that held it and the React tree that renders it, and both are dropped when the parent re-reads and the component unmounts.
- Under change 3, the sweep of resolver entries on surviving records also sweeps closure entries that no retained or observed location reaches. A parent that selects the closure field reaches it through its `Entry` dependency.
- A record collected and later normalized again gets a new wrapper, so a component that was mounted across the collection remounts (open question 4). Upstream's leaked cache keeps the old one (What upstream does).

Reading while writing: `closureWrapper` writes the store during a read. It advances no epoch, and no dependency observed the key before the write: the only reader of the key is a read of the closure field, which creates the entry before it records a dependency on it. Under change 2 that dependency is `Entry(record, key)` with `observed` the creation epoch, which changes only when the entry is deleted and created again; `garbageCollect` advances the epoch when it deletes, so the recreated entry's `timeUpdated` differs. pico-runtime.md open decision 8 concerns a resolver that reaches the store; this write is the runtime's, and does not break verification for the reason above.

### The wrappers

The plain function, in `@isograph/runtime`:

```ts
// from libs/isograph-runtime/src/closure.ts
/** A field whose value is a function that reads the field's data each time it is called. */
export function closure<
  TParam extends ResolverParam,
  TArgs extends ReadonlyArray<unknown>,
  TResult,
>(
  body: (param: TParam, ...args: TArgs) => TResult,
): ClosureResolver<TParam, (...args: TArgs) => ReadResult<TResult>> {
  const bound: { call(param: ResolverParam, ...args: TArgs): TResult } = { call: body };
  return {
    kind: 'Closure',
    createWrapper(reference, store) {
      return (...args) => {
        const data = readButDoNotEvaluate(store, reference.artifact, reference.link, reference.parameters);
        if (data.kind === 'MissingData') {
          return data;
        }
        return ok(bound.call({ data: data.value, parameters: reference.parameters, link: reference.link }, ...args));
      };
    },
  };
}
```

`ok` is `read.ts`'s, exported for this module. `bound` binds `body` through a method signature, so `TParam` is compared bivariantly and the runtime's untyped `SelectionSetData` reaches the generated param type with no cast, as react.md's `useResult` binds the reader artifact. The wrapper reads from the store it was created with. It does not subscribe: a function reads when it is called.

The component, in `@isograph/react` (react.md):

```tsx
// from libs/isograph-react/src/react/component.ts
import type { ClosureResolver, ResolverParam } from '@isograph/runtime';
import type React from 'react';
import { useClosureData } from './useClosureData';

/** A field whose value is a component that reads the field's data when it renders. */
export function component<TParam extends ResolverParam, TProps extends object>(
  body: (param: TParam, props: TProps) => React.ReactNode,
): ClosureResolver<TParam, React.FC<TProps>> {
  const bound: { call(param: ResolverParam, props: TProps): React.ReactNode } = { call: body };
  return {
    kind: 'Closure',
    createWrapper(reference) {
      function Component(props: TProps): React.ReactNode {
        const data = useClosureData(reference);
        return bound.call({ data, parameters: reference.parameters, link: reference.link }, props);
      }
      Component.displayName = `${reference.artifact.selectableName} (${reference.link.entityName}:${reference.link.id})`;
      return Component;
    },
  };
}
```

```ts
// from libs/isograph-react/src/react/useClosureData.ts
import type { ClosureReference, ResolverParam, SelectionSetData } from '@isograph/runtime';
import { onNextChangeToFragmentData } from '../core/read';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { useSubscription } from './useSubscription';
import { useSubscriptionSnapshot } from './useSubscriptionSnapshot';

/**
 * The data `reference`'s reader AST reads now, re-rendering when it changes. Suspends on
 * `MissingData` until a write fills it; throws a read's error.
 */
export function useClosureData<TParam extends ResolverParam>(
  reference: ClosureReference<TParam>,
): SelectionSetData {
  const environment = useIsographEnvironment();
  const subscription = useSubscription(environment, reference.artifact, reference.link, reference.parameters);
  const result = useSubscriptionSnapshot(subscription);
  switch (result.kind) {
    case 'Ok':
      return result.value;
    case 'MissingData':
      throw onNextChangeToFragmentData(environment, reference.artifact, reference.link, reference.parameters);
    case 'Err':
      throw result.error;
  }
}
```

- The data lives only in the runtime, in the subscription's `delivered`; no `useState` or `useRef` holds a read result. The hook reads through `useSyncExternalStore(subscribe, getSnapshot)` (react.md, `useReadAndSubscribe`), with `subscribe` `listenToSubscription` and `getSnapshot` `subscriptionSnapshot`. A copy in React state would duplicate the store, would go stale when a write lands between the render that read it and the commit that subscribes, and is the copy `useSyncExternalStore` exists to avoid. This deviates from upstream's `componentFunction`, which reads through `useReadAndSubscribe`'s `useState` (`react/useReadAndSubscribe.ts:25-43`).
- `subscriptionSnapshot` must return the same object until a write changes the read, or React re-renders in a loop; `callSubscriptions` replaces `delivered` only on an unequal re-read, which gives this (network-and-subscriptions.md, The snapshot).
- `useSubscription` holds the `Subscription` handle across renders, never its data. A wrapper has exactly one `ClosureReference` for its life, so `useSubscription` keeps one subscription for the mount, which is upstream's assumption (`react/useRerenderOnChange.ts:31-37`) made true by construction.
- A `MissingData` or `Err` snapshot after mount suspends or throws, as react.md's `useReadAndSubscribe` does (react.md, open decision 2).
- `useSyncExternalStore` performs a store update as blocking even inside a transition (pico-runtime.md, Follow-ups 9), so a write inside `startTransition` re-renders a closure component synchronously.
- `onNextChangeToFragmentData` takes the artifact, the link, and the variables instead of a `FragmentReference`, because a closure has no network request. The component waits for the data and does not see a request's error; the error reaches the entrypoint's reader through `useResult`'s `maybeUnwrapNetworkRequest`. Upstream's component waits on the first request that created it (What upstream does, the first caller wins), which this removes.
- The component reads through the environment in context, not through the `store` `createWrapper` receives. A component rendered under another provider than the one whose store created it reads the other store. Open question 5.
- `displayName` is upstream's (`react/useReadAndSubscribe.ts:127-128`) without the `@component` suffix.

### Typing

The param type is unchanged: `Pet__PetCard__param` has `data`, `parameters`, and `link: Pet____link__output_type` (reader-ast.md, The param type's link). The output type and the iso overload change, in the generators, so that both resolver forms type-check without the compiler knowing the form.

Before (`crates/typescript_artifacts/src/generate_types.rs`, `output_type.ts`):

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
import type { PetCard as resolver } from "./../../../PetCard";
export type Pet__PetCard__output_type = ReturnType<typeof resolver>;
```

After:

```ts
// generated by crates/typescript_artifacts/src/generate_types.rs
import type { ResolverOutput } from "@isograph/runtime";
import type { PetCard as resolver } from "./../../../PetCard";
export type Pet__PetCard__output_type = ResolverOutput<typeof resolver>;
```

```ts
// from libs/isograph-runtime/src/artifacts.ts
/** The value `read` returns for a field with `TResolver`: a closure resolver's wrapper, or an
 * eager resolver's return type. */
export type ResolverOutput<TResolver> =
  TResolver extends ClosureResolver<ResolverParam, infer TWrapper>
    ? TWrapper
    : TResolver extends EagerResolver<never, infer TOutput>
      ? TOutput
      : never;
```

Before (`crates/typescript_artifacts/src/iso_overloads.rs`, `iso.ts`):

```ts
// generated by crates/typescript_artifacts/src/iso_overloads.rs
type IdentityWithParam<TParam extends object, TReturnConstraint = unknown> = <TClientFieldReturn extends TReturnConstraint>(clientField: (param: TParam) => TClientFieldReturn) => (param: TParam) => TClientFieldReturn;
```

After:

```ts
// generated by crates/typescript_artifacts/src/iso_overloads.rs
import type { ClosureResolver, ResolverParam } from "@isograph/runtime";
type IdentityWithParam<TParam extends ResolverParam, TReturnConstraint = unknown> = {
  <TClientFieldReturn extends TReturnConstraint>(clientField: (param: TParam) => TClientFieldReturn): (param: TParam) => TClientFieldReturn;
  <TWrapper extends TReturnConstraint>(clientField: ClosureResolver<TParam, TWrapper>): ClosureResolver<TParam, TWrapper>;
};
```

- `TEntityName`: `ClosureReference<TParam>.link` is `TParam['link']`, the generated `E____link__output_type`, so a `PetCard` wrapper's link is `Link<"Pet">`, and on an abstract entity the union of its concrete entities' names (reader-ast.md, The parent entity's name). `reference.artifact` is held at `EntityName`; `useSubscription` and `onNextChangeToFragmentData` are called with the pair at `EntityName`, which network-and-subscriptions.md's `Subscription` already does.
- Props: `component`'s `TProps` is inferred from the annotation on `body`'s second parameter, and the field's output type is `React.FC<TProps>`. `key` is admitted by TypeScript's JSX check through `JSX.IntrinsicAttributes`, so upstream's `CombineWithIntrinsicAttributes` (`generate_artifacts.rs:623-626`) is not needed.
- `TParam` for `body`'s first parameter comes from the contextual type: `component(..)` is the argument of the second `IdentityWithParam` signature, whose parameter type is `ClosureResolver<Pet__PetCard__param, TWrapper>`, and TypeScript infers `component`'s `TParam` from that return type. This is not checked with `tsc`. If inference through the overload fails, `data` is `SelectionSetData`'s values typed `unknown`, and the implementer stops and raises it.
- `TArgs` of `closure` is inferred from `body`'s parameters after the first, `TResult` from its return.

### Fetching the closure's data

The query includes the closure's selections with no compiler change. To the compiler, `PetCard` is an ordinary client field: its iso literal declares `PetName` and `tagline`, `HomeRoute`'s selection set reaches it, and query generation merges the selections of every client field the entrypoint reaches into the query (query-generation.md, Merging). Its reader AST and its `Resolver` node in `HomeRoute`'s reader AST are generated as for any field. Only the runtime treats the field differently, and only at read, from the resolver's value. Upstream does the same for `@component` (`merge_client_scalar_field`, `create_merged_selection_set.rs:932`, has no case on it).

This is the split option 2 builds out of `PetCardFetched` and `PetCardData` (Option 2): the query includes the selections, and the parent's read does not depend on them. Here the field's own selections play `PetCardData`'s part, and `read` skipping them plays `PetCardFetched`'s. `check` (store.md) of the merged normalization AST reports the closure's data as missing when it is, so `fetchEntrypoint` with `IfNecessary` sends the request (network-and-subscriptions.md, Fetching).

### Under changes 2 to 4

- Change 2: a parent that selects a closure field records one dependency on the closure entry. Its observed `timeUpdated` changes only when the entry is deleted and created again, so the parent's entry never recomputes for the closure's data.
- Change 3: settlement does not compute the closure's data, because no parent depends on it. A component's render then reads its data, which pico-runtime.md's goal ("Renders do no work") excludes. The alternative is a second entry per closure entry, holding the data read as a `ResolverEntry`, which settlement computes when a retained root reaches the closure entry and which the component observes. Open question 3.
- Change 4: Step A's `readAndSubscribe`, `subscriptionSnapshot`, and `listenToSubscription` are replaced by `listen` and `snapshot`, which take an `EntryLocation`, and `useClosureData` keeps its `useSyncExternalStore` shape with them as `subscribe` and `getSnapshot`. `useClosureData` then needs that second entry's location, so change 4 decides open question 3.

### Without runtime support

A helper with no runtime change cannot stop `read` from reading the field's selections before it calls the resolver, so the field stays an eager result whose resolver ignores `data` and returns a stored wrapper. What that costs, each of which the runtime version removes:

- The parent's read depends on the closure's selections. `MissingData` in them ends the parent's read, and the resolver runs again after every change to them. Under Step A the parent's data is still equal (`===` on the same wrapper), and under change 2 the recomputed entry backdates, so identity survives; the work does not.
- The wrapper needs a key whose lifetime is the record's. The resolver receives the link, not the record, so the helper's map is option 1's string-keyed map, with its leak and its sharing across stores, or option 3's `RecordHandle` or `memo`, which are runtime API.
- The wrapper needs the field's reader artifact to read at execution, which the resolver does not receive. The user's module imports its own `resolver_reader.ts`, the module cycle of option 2.

## The earlier options under the split

The fixture used below is store.md's `Pet` entity (`id`, `name`, `nickname`, `tagline`) and the field `Pet.PetName`. The examples select `id` and build the link by hand (`{ entityName: 'Pet', id: data.id }`). With `link` on `ResolverParam`, each of them reads `link` instead and drops the `id` selection; nothing else in them changes.

- Option 1 is eager. Its result is stable only because its one input, `id`, never changes, and it cannot show data that does.
- Option 2 is a closure built by hand. The field is eager over inputs that never change (`id` and a constant-returning field), and its result is a component that reads its data at render through a hook. The closure field above is option 2 with the runtime doing the three literals' work.
- Option 2 with a module-level component is a closure whose identity is the module binding. The field returns no component.
- Option 3 is storage. For an eager result it makes the result stable by ignoring later data; for a closure it is where the wrapper lives. The runtime's `ClosureEntry` is that storage.
- Option 4 is eager.

### Option 1: a resolver that selects only `id` and returns a component

```ts
// from demos/pet-demo/src/PetCard.tsx
export const PetCard = iso(`
  field Pet.PetCard {
    id
  }
`)(function PetCard({ data }) {
  return function PetCardComponent(props: { readonly selected: boolean }) {
    return <div className={props.selected ? 'selected' : undefined}>Pet {data.id}</div>;
  };
});
```

The component closes over `data`, and `data` holds only `id`, which never changes for one record. Whether the function is the same across reads depends on what calls the resolver:

- v1 `read` calls every resolver on every read (reader-ast.md, The read function: `ok(artifact.resolver(..))`). Each read returns a new function, and each render that reads again remounts the card.
- Step A re-reads every subscription after every write with v1's `read`. The nested resolver returns a new function, its parent's data compares unequal (`===` on nested resolver values), and every subscribed parent receives a new card on every write (pico-runtime.md, Step A, Costs). Every write remounts every card.
- Change 2 stores the result in a `ResolverEntry` on `Pet:1` under `PetCard`. Its one dependency is `Entry(Pet:1, id)`. `normalize` keeps an equal value's entry (change 1), so `id`'s `timeUpdated` never moves, verification finds the dependency unchanged, and the stored function is returned on every read. Option 4 covers this.

Without change 2, stability needs a memo the user writes. The only key a resolver has is the link, so the memo is a module-level map keyed by a string:

```ts
// from demos/pet-demo/src/PetCard.tsx
const cards = new Map<string, (props: { readonly selected: boolean }) => React.ReactNode>();

export const PetCard = iso(`
  field Pet.PetCard {
    id
  }
`)(function PetCard({ data }) {
  const key = `Pet:${data.id}`;
  const existing = cards.get(key);
  if (existing !== undefined) {
    return existing;
  }
  const created = function PetCardComponent(props: { readonly selected: boolean }) {
    return <div className={props.selected ? 'selected' : undefined}>Pet {data.id}</div>;
  };
  cards.set(key, created);
  return created;
});
```

This is upstream's cache moved into user code, with upstream's leak, and worse: the map is shared by every store in the process (two test stores, two server-rendered requests), where upstream's is per environment.

### Why closing over data breaks stability

A component that shows `name` has to get `name` from somewhere. If the resolver selects it and the component closes over it:

```ts
// from demos/pet-demo/src/PetCard.tsx
export const PetCard = iso(`
  field Pet.PetCard {
    name
  }
`)(function PetCard({ data }) {
  return function PetCardComponent() {
    return <div>{data.name}</div>;
  };
});
```

there are two choices, and both are wrong:

- Return a new function when `name` changes (change 2 does this, since the resolver re-runs when its input changes). The new identity remounts the card on every change of `name`. State inside the card (a `useState`, an input's focus and text, a running effect) is lost.
- Return the first function forever (the memo above). The card shows the first `name` forever.

A function's identity and the data it closes over change together. This is the eager kind's limit: a component that must keep its identity across data changes cannot be computed from its data, and has to read the data when it is executed, from somewhere that changes without changing the function. That is the closure kind: option 2 by hand, or the closure field.

### Option 2: a component that reads its own data through a hook, given its link

The resolver's result depends only on the link. The component reads its data at render through a hook in the React package:

```ts
// from libs/isograph-react/src/useFragment.ts
/**
 * The stored result of `artifact` on `link` with `variables`, re-rendering when it changes.
 * Suspends on `MissingData` and throws on `ResolverThrew`.
 */
export function useFragment<TEntityName extends EntityName, TOutput>(
  artifact: ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
): TOutput {
  const store = useStore();
  const location: EntryLocation = {
    record: link,
    storeKey: resolverStoreKey(artifact.selectableName, variables),
  };
  const result = useSyncExternalStore(
    (onStoreChange) => listen(store, location, onStoreChange),
    () => snapshot(store, location),
  );
  // The entry at `location` was computed by `artifact`: one selectable of one entity has one
  // reader artifact (change 2, `read`).
  return unwrapForRender(result as ReadResult<TOutput> | undefined);
}

/** `Ok`'s value; throws a promise for `MissingData` or `undefined`, the error for `ResolverThrew`. */
function unwrapForRender<T>(result: ReadResult<T> | undefined): T;
```

`listen`, `snapshot`, `EntryLocation`, `resolverStoreKey`, and `ReaderArtifact.selectableName` are changes 2 and 4. `useStore` reads the store from a React context provider. The hook is the general fragment-reading hook pico-runtime.md sketches (End-user experience, `useResolverResult`), addressed by a link instead of a retention. It knows nothing about components.

The user code:

```ts
// from demos/pet-demo/src/PetCard.tsx
import PetCardDataReader from './__isograph/Pet/PetCardData/resolver_reader';
import type { Pet____link__output_type } from './__isograph/Pet/__link/output_type';

export const PetCardData = iso(`
  field Pet.PetCardData {
    PetName
    tagline
  }
`)(function PetCardData({ data }) {
  return data;
});

export const PetCard = iso(`
  field Pet.PetCard {
    id
  }
`)(function PetCard({ data }) {
  const link: Pet____link__output_type = { entityName: 'Pet', id: data.id };
  return function PetCardComponent(props: { readonly selected: boolean }) {
    const card = useFragment(PetCardDataReader, link, {});
    return (
      <div className={props.selected ? 'selected' : undefined}>
        {card.PetName.name}: {card.tagline}
      </div>
    );
  };
});
```

The parent renders `<data.PetCard selected={isSelected} />`. Under change 2, `PetCard`'s stored result depends only on `Pet:1.id`, so it is the same function until `Pet:1` is deleted. `PetCardComponent` re-renders when `PetCardData`'s stored result changes, through its own observer, and the parent does not re-render for it.

As written, this does not fetch the card's data. Query generation inlines the selections of client fields the entrypoint reaches (query-generation.md, Merging), and nothing reaches `PetCardData`: the hook call is runtime code the compiler does not read. The query fetches `Pet.id` and no `name` or `tagline`, and the hook suspends with no request that would fill it.

#### Fetching the component's data without the field depending on it

`PetCard` has to select something that brings `PetCardData`'s selections into the query. If it selects `PetCardData` directly, its resolver entry depends on `PetCardData`'s entry, re-runs when the card's data changes, and returns a new function: the closure problem again, though the function does not use the value. The selection has to reach `PetCardData` through a field whose own result does not change. In userland that is a field whose resolver returns a constant:

```ts
// from demos/pet-demo/src/PetCard.tsx
export const PetCardFetched = iso(`
  field Pet.PetCardFetched {
    PetCardData
  }
`)(function PetCardFetched() {
  return null;
});

export const PetCard = iso(`
  field Pet.PetCard {
    id
    PetCardFetched
  }
`)(function PetCard({ data }) {
  const link: Pet____link__output_type = { entityName: 'Pet', id: data.id };
  return function PetCardComponent(props: { readonly selected: boolean }) {
    const card = useFragment(PetCardDataReader, link, {});
    return (
      <div className={props.selected ? 'selected' : undefined}>
        {card.PetName.name}: {card.tagline}
      </div>
    );
  };
});
```

Under change 2, after `Pet:1.tagline` changes:

```text
verify PetCard              deps [Entry(Pet:1, id), Entry(Pet:1, PetCardFetched)]
  id                        server, unchanged
  PetCardFetched            verify: deps [Entry(Pet:1, PetCardData)]
    PetCardData             verify: tagline changed, compute, new object, timeUpdated moves
    changed: compute PetCardFetched, returns null === null: backdated
  PetCardFetched unchanged: PetCard verified, same function
notify                      the observer of (Pet:1, PetCardData) is called; PetCardComponent re-renders
```

The query fetches `name`, `nickname`, and `tagline` because `PetCardFetched` reaches `PetCardData` and `PetName`. Settlement computes `PetCardData` eagerly because the retained root reaches it through `PetCard` and `PetCardFetched` (change 3, Settlement), so the hook's first `snapshot` finds a stored result. This reproduces upstream's split (the query includes the component's selections, the parent's read does not depend on them, `read.ts:603-612`) out of three fields and the backdating of change 2.

What is left that the runtime does not give userland, each of which the closure field removes:

- `MissingData` still propagates. When `PetCardData` lacks `tagline`, `PetCardFetched`'s resolver is not called and its result is `MissingData`, and so is `PetCard`'s, and so is the parent's (reader-ast.md, Semantics: the first `MissingData` ends the read). The parent cannot render the card at all, where upstream renders it and the card suspends at its own boundary. The parent's result also changes from `MissingData` to `Ok` when the data arrives, which is a new function and a mount, the first one.
- Arguments. When `PetCardData` declares a variable (`$limit: Int = 3`), the store key the hook computes must use the values `PetCardFetched`'s selection passed. The user writes them twice, once in the iso literal and once in the hook call, and nothing checks that they agree. A mismatch is a store key no query wrote and a hook that suspends forever.
- A module cycle. `PetCard.tsx` imports `PetCardData/resolver_reader.ts`, which imports the `PetCardData` resolver from `PetCard.tsx`. ES modules allow it because the reader artifact is read only inside the component body, at render, after both modules finished evaluating. Moving `PetCardData` into its own file removes the cycle.

#### Option 2 without a function per record

The function the resolver creates closes over nothing but the link. The link can be a prop instead, and the component a module-level constant:

```ts
// from demos/pet-demo/src/PetCard.tsx
export function PetCardView(props: {
  readonly link: Pet____link__output_type;
  readonly selected: boolean;
}) {
  const card = useFragment(PetCardDataReader, props.link, {});
  return (
    <div className={props.selected ? 'selected' : undefined}>
      {card.PetName.name}: {card.tagline}
    </div>
  );
}
```

Its identity is the module binding, so nothing needs to be stored, there is nothing to free, and it works under v1's `read` and Step A as well as under change 2. The field no longer returns a component: the parent selects `id` and `PetCardFetched`, builds the link, imports `PetCardView`, and renders `<PetCardView link={link} selected={isSelected} />`. This is Relay's fragment reference with the masking done by hand. The parent's resolver depends on `PetCardFetched`, which backdates, so the parent's own result is stable for the same reason `PetCard`'s was.

### Option 3: a userland memo per record

Under v1 and Step A, where no resolver result is stored, a memo is the only way an eager resolver returns the same function twice. The key must be an object whose lifetime is the record's, or the memo leaks (Option 1's string map). Links are not that object: a link value is stored under the store key of the field that points at the record, many links point at one record, and the resolver receives none of them. The record itself is that object: a `StoreRecord` map is created once by `normalize` and dropped when `garbageCollect` deletes it (store.md, `garbageCollect`). userland cannot reach it, so the runtime would expose an opaque handle to it.

#### In a `WeakMap`, through a record handle

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ResolverParam = {
  readonly data: SelectionSetData;
  readonly parameters: Variables;
  /** The record the resolver was read on. Identity is the record's: one handle per record
   * per store, dropped when `garbageCollect` deletes the record. */
  readonly record: RecordHandle;
};

declare const recordHandle: unique symbol;
/** An opaque object with the record's lifetime. */
export type RecordHandle = { readonly [recordHandle]: true };
```

```ts
// from demos/pet-demo/src/memoOnRecord.ts
const memos = new WeakMap<RecordHandle, Map<string, unknown>>();

/** `create()` the first time `key` is asked for on `record`, the same value after. */
export function memoOnRecord<T>(record: RecordHandle, key: string, create: () => T): T {
  let byKey = memos.get(record);
  if (byKey === undefined) {
    byKey = new Map();
    memos.set(record, byKey);
  }
  if (!byKey.has(key)) {
    byKey.set(key, create());
  }
  // `key` is written only by `create()` of this call site's `T`.
  return byKey.get(key) as T;
}
```

The runtime passes the record's `StoreRecord` map, typed as `RecordHandle`, in `callResolver`; the brand stops user code from mutating it through the type. The `WeakMap` entry is collectable once the record is deleted. Two stores have distinct records, so nothing is shared across stores. The user chooses `key`, so two fields that pick the same key collide, and the cast is an invariant the user keeps by discipline.

#### In the store, through a memo function

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ResolverParam = {
  readonly data: SelectionSetData;
  readonly parameters: Variables;
  /** `create()` once per record, field, and variables; the same value on every later read. */
  memo<T>(create: () => T): T;
};
```

```ts
// from libs/isograph-runtime/src/store.ts
export type StoreEntry = ServerEntry | ResolverEntry | MemoEntry;

/** A value a resolver's `memo` created, under `memoStoreKey(resolverStoreKey)`. */
export type MemoEntry = {
  readonly kind: 'Memo';
  readonly value: unknown;
};
```

The runtime keys the memo by the reading field's resolver store key, so two fields never collide and the user writes no key. It is deleted with its record. Costs: a third entry kind that every walk narrows away; a store key namespace for memos that no selectable name can produce; a write to the store during a read, which pico-runtime.md open decision 8 lists as breaking verification unless it records nothing and advances no epoch; and a value nothing invalidates, so a `memo` whose `create` closes over `data` returns stale data forever, with no type to stop it.

`ClosureEntry` is the store version keyed by the closure field's own resolver store key, with the stale-data problem removed: the wrapper's `createWrapper` receives no `data`, only the reference it reads with. The memo has no use for components once closure fields exist. Under change 2 without closure fields it has one: a resolver that must depend on changing data and still return the same function, where `memo` makes the recomputed result `===` the stored one, so it backdates. Option 2's constant-returning field reaches the same result with no new API.

### Option 4: change 2 alone

Change 2 makes every resolver result stable until one of its recorded inputs changes, stores it on the record it was read on, and change 3 deletes it with the record or when no retained or observed location reaches it. That is the user's idea for any eager resolver, and it needs nothing component-specific.

Whether that suffices depends on what the component's resolver depends on:

- A resolver whose dependencies never change (`id` only, or `id` and a field that backdates) returns the same function for the record's lifetime. This is options 1 and 2 under change 2: stable with no memo, and freed with the record.
- A resolver that closes over data re-runs when the data changes and returns a new function. `===` equality (pico-runtime.md, Results, equality) makes every entry that reads it re-run as well. When the parent is itself a component-returning resolver that reads `data.PetCard`, the parent gets a new function too and remounts, and so on up to the first resolver that does not return a component. A change of one pet's `tagline` remounts every component on the path from that card to the root.
- A recomputation the user did not cause gives a new function: `garbageCollect` deletes `Pet:1` and a later `normalize` writes it again; or no retained or observed location reaches the entry, change 3 sweeps it, and a later read recomputes it. A mounted component's entry is reached through its parent's retention, so neither happens while the parent query is retained. Upstream's leaked cache keeps identity across both (What upstream does).

Change 2 therefore solves storage and freeing for eager results. It does not solve where a component's data comes from; a closure does.

## Comparison

The closure field (a `ClosureResolver` made by `component` or `closure`, `ClosureEntry`, and `read`'s case):

- Kind: closure.
- Ergonomics: one iso literal, data in the first parameter, props or arguments after it, one helper call around the function.
- Type safety: param type as any resolver; props from the annotation; output `React.FC<TProps>` or `(...args) => ReadResult<TResult>` through `ResolverOutput`. `TParam`'s inference through the overload is not checked with `tsc`.
- Garbage collection: the wrapper is a `ClosureEntry` on the record, deleted with the record or by change 3's sweep.
- React: stable for the record's lifetime; the component re-renders alone on its own data; the parent does not re-render; data missing under the closure suspends the component at its own boundary and does not block the parent.
- Support needed: change 1's tagged entries, `selectableName` on the reader artifact, `ClosureResolver` and `read`'s case in `@isograph/runtime`, `component` and `useClosureData` in `@isograph/react`, and two generator changes (`ResolverOutput` in `output_type.ts`, a second `IdentityWithParam` signature). Works with Step A (network-and-subscriptions.md); under change 4 it needs open question 3 decided.

Option 1, string-keyed memo, v1 or later:

- Kind: eager.
- Ergonomics: one field, plus a hand-written cache per field. The component cannot show changing data.
- Type safety: the props type is the user's annotation; the output type is `ReturnType<typeof resolver>`, so `<data.PetCard selected />` is checked. The cache's key is a string the user builds.
- Garbage collection: never freed; shared across stores.
- React: stable identity. Data changes are not shown.
- Support needed: none.

Option 2 under change 2 (`id`, a constant-returning field, a data field, and `useFragment`):

- Kind: closure built by hand over an eager field whose inputs never change.
- Ergonomics: three iso literals per component, a hand-built link, and arguments repeated in the hook call.
- Type safety: `useFragment` checks the artifact's output type, and, typed as reader-ast.md's `read` is (`ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>` and `NoInfer<Link<TEntityName>>`), that the link's entity is the artifact's parent entity. It does not check that the variables are the ones the selection passed (`Variables` is any JSON map).
- Garbage collection: the component lives in `PetCard`'s resolver entry on `Pet:1` and is deleted with the record or by change 3's sweep.
- React: stable across data changes; the component re-renders alone on its own data; the parent does not re-render. The component suspends at its own boundary only when its data goes missing after the parent rendered; data missing at the parent's read blocks the parent.
- Support needed: changes 2 to 4, and a React package with `useStore`, `useFragment`, and a store provider. No compiler change. Records without an identity selectable and fields on abstract entities get their link from `ResolverParam.link` (reader-ast.md, A resolver's own link).

Option 2 with a module-level component (`PetCardView`):

- Kind: closure, not returned by a field.
- Ergonomics: two iso literals, a hand-built link, an import in the parent. The field does not return the component.
- Type safety: as option 2; the link is a typed prop.
- Garbage collection: nothing stored.
- React: stable by module identity, under v1 too. Re-renders as option 2.
- Support needed: the React package's hook. Under v1, the hook has no `listen` and needs Step A or a re-read on every write.

Option 3, a memo per record:

- Kind: storage for an eager result.
- Ergonomics: one call per resolver. Easy to misuse by closing over `data`.
- Type safety: the `WeakMap` version needs a user-maintained cast; the store version types `memo<T>` per call.
- Garbage collection: freed with the record.
- React: stable; data must still come from option 2's hook.
- Support needed: a public runtime API (`RecordHandle` or `memo`) and, for the store version, a third entry kind. Redundant once closure fields exist.

Option 4, change 2 alone with data in the closure:

- Kind: eager.
- Ergonomics: one field, data in `data`, as any resolver.
- Type safety: as any resolver.
- Garbage collection: freed with the record.
- React: remounts on every change of the data and up the chain of component-returning parents.
- Support needed: change 2.

## A `@component` directive

Upstream's shape, with the directive in the iso literal instead of a helper call:

```ts
// from demos/pet-demo/src/PetCard.tsx
export const PetCard = iso(`
  field Pet.PetCard @component {
    PetName
    tagline
  }
`)(function PetCard({ data }, props: { readonly selected: boolean }) {
  return (
    <div className={props.selected ? 'selected' : undefined}>
      {data.PetName.name}: {data.tagline}
    </div>
  );
});
```

The closure field already supplies what the directive supplied in upstream and what the hand-built option 2 lacks: the split between fetching and depending, the link, the variables, `MissingData` that does not block the parent, `displayName`, and storage freed with the record. What a directive would still add over `component(..)`:

- The compiler knows the field's form. It can print a distinct reader node or artifact kind, and the runtime dispatches on compiler-written data instead of on the shape of the resolver value. With the helper, the generated artifact holds whatever the user's module exports, and the runtime distinguishes the forms by `typeof`.
- Typing without inference through an overload. The compiler can print a component overload for that field alone (upstream's `IdentityWithParamComponent`, `iso_overload_file.rs:97-102`, `:156-168`) and a component output type (`generate_artifacts.rs:623-626`), so `ResolverOutput` and the second `IdentityWithParam` signature are not needed, and `data` is typed by the overload's parameter directly.
- Ergonomics: no helper import and no wrapping call. This is the difference a user sees.

What it costs:

- A supported directive: a variant in the parser's supported set (iso-directives-unexpected.md, Change 4, The supported set), and `ClientFieldKind` back in `crates/isograph_compiler/src/schema.rs` (removed by iso-directives-unexpected.md change 2).
- A component-creating function the React package supplies to `@isograph/runtime`, as upstream injects `componentFunction` into the environment (`IsographEnvironment.ts:76`, `react/createIsographEnvironment.ts:19`), because the artifact no longer carries `component`'s `createWrapper`. That is component-specific runtime code, though `@isograph/runtime` still does not import React. The helper keeps the runtime's knowledge to "this resolver creates a wrapper".
- A directive names one executor. A plain-function closure (`closure`) needs a second directive or stays a helper.

## Open questions

1. Form selection: does the runtime distinguish the two resolver forms by the value the user's module exports (`typeof resolver`, the helper's object), or does the compiler learn the form from a directive after v1?
2. `TParam` inference: does `component(function PetCard({ data }, props) {..})` as the argument of the two-signature `IdentityWithParam` type `data` from `Pet__PetCard__param` under `tsc` 5.6.3? If not, the helper form needs the user to annotate the param, which is the case for the directive.
3. Under changes 3 and 4: does a closure entry get a second entry holding its data read, computed by settlement whenever a retained root reaches the closure entry and observed by `useClosureData` through `listen` and `snapshot`? Without it, a closure component's render reads, against pico-runtime.md's goal; with it, settlement computes data for components that may never render (pico-runtime.md open decision 3).
4. Identity across garbage collection: upstream keeps the component across a collection and refetch of its record; record-bound storage does not. Is a remount after the record was collected and fetched again acceptable?
5. The store a wrapper reads: `closure`'s wrapper reads the `store` `createWrapper` received, and `component`'s reads the environment in context. Should `read` take the environment, so that `createWrapper` receives it and the component closes over it instead of reading the context?
6. Ordering: closure fields need change 1's tagged entries. Does change 1 land before Step A (network-and-subscriptions.md), so that a React demo with components runs on Step A?
