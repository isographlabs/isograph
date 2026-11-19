# Components returned by fields

Status: exploration. This is not a plan for v1 and not approved for implementation. It decides nothing; it lays out options and ends with open questions.

The question: what happens when a client field's resolver returns a React component. React compares an element's `type` with `Object.is`; when the type at one position of the tree changes, React unmounts the old subtree (its state is lost and its effects are cleaned up) and mounts the new one. A field that returns a component must therefore return the same function on every read for as long as the component is meant to keep its state. Something has to hold that function. This doc asks where, and whether userland can do it in i2, where v1 has no directives (`refactors/pending/iso-directives-unexpected.md`) and `@isograph/runtime` has no React knowledge (`refactors/pending/runtime/store.md`, `refactors/pending/runtime/reader-ast.md`).

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` paths are under `isograph/libs/isograph-react/src/react`. "Change 1" to "change 4" are the changes of `refactors/pending/runtime/pico-runtime.md`.

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

The rest of this doc asks how much of that userland can do without `@component` and without runtime code that knows about components.

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

The examples below select `id` and build the link by hand (`{ entityName: 'Pet', id: data.id }`). With `link` on `ResolverParam`, each of them reads `link` instead and drops the `id` selection; nothing else in them changes.

The fixture used below is store.md's `Pet` entity (`id`, `name`, `nickname`, `tagline`) and the field `Pet.PetName`.

## Option 1: a resolver that selects only `id` and returns a component

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
- Step A of pico-runtime.md re-reads every subscription after every write with v1's `read`. The nested resolver returns a new function, its parent's data compares unequal (`===` on nested resolver values), and every subscribed parent receives a new card on every write (pico-runtime.md, Step A, Costs). Every write remounts every card.
- Change 2 stores the result in a `ResolverEntry` on `Pet:1` under `PetCard`. Its one dependency is `Entry(Pet:1, id)`. `normalize` keeps an equal value's entry (change 1), so `id`'s `timeUpdated` never moves, verification finds the dependency unchanged, and the stored function is returned on every read. Option 4 covers this.

Without change 2, stability needs a memo the user writes. The only key a resolver has is the link it built by hand, so the memo is a module-level map keyed by a string:

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

A function's identity and the data it closes over change together. A component that must keep its identity across data changes cannot close over the data; it has to read the data at render time from somewhere that changes without changing the function. That is option 2.

## Option 2: a component that reads its own data through a hook, given its link

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

### Fetching the component's data without the field depending on it

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

What is left that the runtime does not give userland:

- `MissingData` still propagates. When `PetCardData` lacks `tagline`, `PetCardFetched`'s resolver is not called and its result is `MissingData`, and so is `PetCard`'s, and so is the parent's (reader-ast.md, Semantics: the first `MissingData` ends the read). The parent cannot render the card at all, where upstream renders it and the card suspends at its own boundary. The parent's result also changes from `MissingData` to `Ok` when the data arrives, which is a new function and a mount, the first one.
- Arguments. When `PetCardData` declares a variable (`$limit: Int = 3`), the store key the hook computes must use the values `PetCardFetched`'s selection passed. The user writes them twice, once in the iso literal and once in the hook call, and nothing checks that they agree. A mismatch is a store key no query wrote and a hook that suspends forever.
- A module cycle. `PetCard.tsx` imports `PetCardData/resolver_reader.ts`, which imports the `PetCardData` resolver from `PetCard.tsx`. ES modules allow it because the reader artifact is read only inside the component body, at render, after both modules finished evaluating. Moving `PetCardData` into its own file removes the cycle.

### Option 2 without a function per record

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

## Option 3: a userland memo per record

Under v1 and Step A, where no resolver result is stored, a memo is the only way a resolver returns the same function twice. The key must be an object whose lifetime is the record's, or the memo leaks (Option 1's string map). Links are not that object: a link value is stored under the store key of the field that points at the record, many links point at one record, and the resolver receives none of them. The record itself is that object: a `StoreRecord` map is created once by `normalize` and dropped when `garbageCollect` deletes it (store.md, `garbageCollect`). userland cannot reach it, so the runtime would expose an opaque handle to it.

### In a `WeakMap`, through a record handle

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

### In the store, through a memo function

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

Under change 2, the store version is a `ResolverEntry` whose dependencies never change. An `id`-only resolver already is that entry. The memo adds nothing for components under change 2, except in one case: a resolver that must depend on changing data and still return the same function (it selects `PetCardData` directly and memoizes), where `memo` makes the recomputed result `===` the stored one, so it backdates. Option 2's constant-returning field reaches the same result with no new API.

## Option 4: change 2 alone

Change 2 makes every resolver result stable until one of its recorded inputs changes, stores it on the record it was read on, and change 3 deletes it with the record or when no retained or observed location reaches it. That is the user's idea for any resolver, and it needs nothing component-specific.

Whether that suffices depends on what the component's resolver depends on:

- A resolver whose dependencies never change (`id` only, or `id` and a field that backdates) returns the same function for the record's lifetime. This is options 1 and 2 under change 2: stable with no memo, and freed with the record.
- A resolver that closes over data re-runs when the data changes and returns a new function. `===` equality (pico-runtime.md, Results, equality) makes every entry that reads it re-run as well. When the parent is itself a component-returning resolver that reads `data.PetCard`, the parent gets a new function too and remounts, and so on up to the first resolver that does not return a component. A change of one pet's `tagline` remounts every component on the path from that card to the root.
- A recomputation the user did not cause gives a new function: `garbageCollect` deletes `Pet:1` and a later `normalize` writes it again; or no retained or observed location reaches the entry, change 3 sweeps it, and a later read recomputes it. A mounted component's entry is reached through its parent's retention, so neither happens while the parent query is retained. Upstream's leaked cache keeps identity across both (What upstream does).

Change 2 therefore solves storage and freeing. It does not solve where the component's data comes from; option 2 and the constant-returning field do.

## Comparison

Option 1, string-keyed memo, v1 or later:

- Ergonomics: one field, plus a hand-written cache per field. The component cannot show changing data.
- Type safety: the props type is the user's annotation; the output type is `ReturnType<typeof resolver>`, so `<data.PetCard selected />` is checked. The cache's key is a string the user builds.
- Garbage collection: never freed; shared across stores.
- React: stable identity. Data changes are not shown.
- Support needed: none.

Option 2 under change 2 (`id`, a constant-returning field, a data field, and `useFragment`):

- Ergonomics: three iso literals per component, a hand-built link, and arguments repeated in the hook call.
- Type safety: `useFragment` checks the artifact's output type, and, typed as reader-ast.md's `read` is (`ReaderArtifact<ResolverParam<TEntityName>, TOutput, TEntityName>` and `NoInfer<Link<TEntityName>>`), that the link's entity is the artifact's parent entity. It does not check that the variables are the ones the selection passed (`Variables` is any JSON map).
- Garbage collection: the component lives in `PetCard`'s resolver entry on `Pet:1` and is deleted with the record or by change 3's sweep.
- React: stable across data changes; the component re-renders alone on its own data; the parent does not re-render. The component suspends at its own boundary only when its data goes missing after the parent rendered; data missing at the parent's read blocks the parent.
- Support needed: changes 2 to 4, and a React package with `useStore`, `useFragment`, and a store provider. No compiler change. Records without an identity selectable and fields on abstract entities get their link from `ResolverParam.link` (reader-ast.md, A resolver's own link).

Option 2 with a module-level component (`PetCardView`):

- Ergonomics: two iso literals, a hand-built link, an import in the parent. The field does not return the component.
- Type safety: as option 2; the link is a typed prop.
- Garbage collection: nothing stored.
- React: stable by module identity, under v1 too. Re-renders as option 2.
- Support needed: the React package's hook. Under v1, the hook has no `listen` and needs Step A or a re-read on every write.

Option 3, a memo per record:

- Ergonomics: one call per resolver. Easy to misuse by closing over `data`.
- Type safety: the `WeakMap` version needs a user-maintained cast; the store version types `memo<T>` per call.
- Garbage collection: freed with the record.
- React: stable; data must still come from option 2's hook.
- Support needed: a public runtime API (`RecordHandle` or `memo`) and, for the store version, a third entry kind. Redundant under change 2.

Option 4, change 2 alone with data in the closure:

- Ergonomics: one field, data in `data`, as any resolver.
- Type safety: as any resolver.
- Garbage collection: freed with the record.
- React: remounts on every change of the data and up the chain of component-returning parents.
- Support needed: change 2.

## A `@component` directive

The most ergonomic shape is upstream's: one iso literal, data in the first parameter, props in the second.

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

In i2 terms the directive could compile to option 2 under change 2, and it would supply what userland writes by hand or cannot reach:

- The split between fetching and depending. The parent's `Resolver` node for a component reads only the component's entry, as upstream's read does (`read.ts:603-612`); the component's selections are still inlined into the query. The runtime computes the component's data in a second entry on the same record, which the mounted component observes. Userland needs `PetCardFetched` and `PetCardData` for this.
- The link. The runtime holds `root` when it reads the component's entry and passes it as the resolver's `link`, as it does for every resolver, so this is no longer a difference from userland.
- Arguments. The data entry's variables are the component field's own, computed once by the runtime (`childVariables`), so the hook cannot be called with other values.
- `MissingData` that does not block the parent. The component's entry does not read the data, so it is `Ok` while the data is missing; the parent renders, and the component suspends at its own boundary. The userland chain propagates `MissingData` through `PetCardFetched`.
- Typing. The output type is a component over the props (upstream's `React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>` admits `key`), and the iso overload types the two parameters (`IdentityWithParamComponent`). Userland annotates the returned function's props by hand, and TypeScript's JSX check admits `key` through `JSX.IntrinsicAttributes`, so the typing gap is the second parameter's inference, not `key`.
- `displayName`. The runtime names the component by field and record (`react/useReadAndSubscribe.ts:127-128`).
- Storage. The component is the component entry's result, freed with the record, where upstream's cache leaks.

What it costs:

- A supported directive: a variant in the parser's supported set (iso-directives-unexpected.md, Change 4, The supported set), and `ClientFieldKind` back in `crates/isograph_compiler/src/schema.rs` (removed by iso-directives-unexpected.md change 2).
- A reader node or artifact kind the runtime dispatches on, and a component-creating function the React package supplies to `@isograph/runtime`, as upstream injects `componentFunction` into the environment (`IsographEnvironment.ts:76`, `react/createIsographEnvironment.ts:19`). That is component-specific runtime code, though `@isograph/runtime` still does not import React.
- Two entries per component per record instead of one.

## Open questions

1. Is option 2's three-literal pattern acceptable as the userland answer, or is the cost to the user the reason to add `@component` after v1?
2. Settled: the runtime gives every resolver its own record's link as `link` on `ResolverParam`, with no `__link` selectable (reader-ast.md, A resolver's own link).
3. Settled: `ReaderArtifact<TParam, TOutput, TEntityName>` carries its parent entity's name, and a link is `Link<TEntityName>`, so a hook typed like `read` rejects a link of another entity (reader-ast.md, The parent entity's name).
4. Should a field be able to select another field for the query without its own result depending on it? That is what `PetCardFetched` emulates. As syntax it is a directive or a new selection form, which v1 does not have.
5. Under v1 and Step A, is a React demo with components in scope? If it is, only option 2's module-level variant or option 3 gives stable identity, since nothing stores resolver results.
6. Identity across garbage collection: upstream keeps the component across a collection and refetch of its record; record-bound storage does not. Is a remount after the record was collected and fetched again acceptable?
7. Should option 3's memo exist at all once change 2 lands, given that an `id`-only resolver is already a memo per record?
