# Runtime: loaders, effect sets, and the router

Status: exploration. Not approved for implementation. Nothing in this doc is decided. It records the direction the user wants to take Isograph: every field is either a pure function or a loader, and the runtime, not React, decides which loaders run, from the data the mounted queries currently reach. The doc is a problem discussion, a concrete shape for the parts that are concrete, and the open decisions.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9b81e0b6`). Paths without a directory are under `isograph/libs/isograph-react/src/core`; `react/` and `loadable-hooks/` paths are under `isograph/libs/isograph-react/src`; `disposable-state/` paths are under `isograph/libs/isograph-react-disposable-state/src`; `disposable-types/` paths are under `isograph/libs/isograph-disposable-types/src`; `crates/` paths are under `isograph/crates`. i2 paths are relative to the i2 root.

This doc builds on `refactors/pending/runtime/pico-runtime.md` (held 3b, mark then pull; change 3's retention and `rootCount`; change 4's observers; Values with lifecycles; Other derived node kinds; Follow-ups 10 and 11), `refactors/pending/runtime/network-and-subscriptions.md` (the request cache, `fetchEntrypoint`, the two-phase lifetime, Request entries in the store), and `refactors/pending/runtime/components-in-userland.md` (closure fields). It edits none of them; What this changes in other docs lists the amendments.

## The problem

Upstream starts every resource from a React hook, during render, and ties its lifetime to the component that called the hook.

- A query fetch starts in `useLazyReference` (`react/useLazyReference.ts:50-58`): `getOrCreateCacheForArtifact` returns a `ParentCache`, and `useLazyDisposableState` (`disposable-state/useLazyDisposableState.ts:18-57`) populates it under a temporary retain at render and converts that to a permanent retain at commit (`disposable-state/useCachedResponsivePrecommitValue.ts:55-104`). The fetch is released when the component unmounts (`useLazyDisposableState.ts:31-43`).
- A `@loadable` field's fetch starts in `useClientSideDefer` (`loadable-hooks/useClientSideDefer.ts:42-46`) with the same pair of hooks, or in `useImperativeLoadableField` on a user event (`loadable-hooks/useImperativeLoadableField.ts:41-47`). The parent's reader receives a description, not a running fetch: `LoadableField<TReadFromStore, TResult, TArgs>` is `(args, fetchOptions) => [StableId, Factory<FragmentReference>]` (`reader.ts:171-183`), built by `readLoadablySelectedFieldData` (`read.ts:279-345`). The stable id is `typename:link/fieldName/stableStringifyArgs(variables)` (`read.ts:332-339`), so the identity is (link, field, args), where part of the args come from the component's hook call (`useClientSideDefer(pet.PetCheckinsCard, { limit: 2 })`, `demos/pet-demo/src/components/PetDetailDeferredRoute.tsx:54-57`).
- Pagination holds one disposable per page in `useUpdatableDisposableState` (`loadable-hooks/useSkipLimitPagination.ts:96-99`).

Three consequences follow.

- A resource lives as long as a component, not as long as the data that called for it. When `Pet:1.bestFriend` moves from `Pet:2` to `Pet:3`, a component that deferred-loaded something on the best friend keeps the `Pet:2` resource until React re-renders it and the hook sees a new cache.
- Which resources exist is decided at render, by whichever components React happened to render. A pending render that never commits starts resources that only a timeout releases (`CacheItem.ts:7`, `:195-253`).
- A resource whose inputs did not change is not shared after commit. `CacheItem` leaves its `ParentCache` when its temporary retain count reaches 0, even while permanently retained (`CacheItem.ts:343-350`), so a later render of the same id creates a new item and a new fetch.

pico-runtime.md's Goal moves resolver work out of render: a write marks, and settlement pulls what is observed. This doc moves resources out of render the same way: a write marks, and the runtime pulls each mounted query's effect set and reconciles the loaders it reaches.

## Two kinds of field

Every client field is one of two kinds, and a directive on the field's declaration says which. The directive's name is open decision 1.

- A pure function. Its value is a derived node of pico-runtime.md's graph: lazy, cached, cut off by `valuesEqual`. It reads through the reader walk and acquires nothing. Every client field upstream and i2 have today is this kind. Component fields are pure: a closure field's wrapper is a function of (link, field, variables) (components-in-userland.md, A field whose result is a closure), and the hooks a component calls when React renders it are React's effects, not the runtime's.
- A loader. Its inputs are a pure derived node over its fragment, and its `load(inputs)` returns an `ItemCleanupPair` (`disposable-types/index.ts:2`). The runtime calls `load` once per distinct inputs value per instance, outside every computation, and calls the cleanup when the instance is released or its inputs change.

The split is the one pico-runtime.md's Values with lifecycles states for derived nodes (a computation is pure; a resource is an effect run apart from it), lifted from the runtime's internals to the field definition, so that user code names which fields have effects and the compiler knows it statically.

## Loaders

### The artifact

```ts
// from libs/isograph-runtime/src/artifacts.ts
/**
 * What the default export of a loader field's reader artifact returns. `TInputs` is what `inputs`'
 * resolver returns, and `TValue` is what a reader of the field receives once the instance is loaded.
 */
export type LoaderArtifact<
  TParam extends ResolverParam<TEntityName>,
  TInputs,
  TValue,
  TEntityName extends EntityName,
> = {
  readonly kind: 'Loader';
  readonly selectableName: string;
  /** The loader's inputs: a pure field over the loader's own selections. */
  readonly inputs: ReaderArtifact<TParam, TInputs, TEntityName>;
  /** Acquires what one instance needs for `inputs`. Called once per distinct `inputs` value per instance. */
  load(inputs: TInputs): ItemCleanupPair<LoaderItem<TValue>>;
};

/** A loader's item: the current value, and a notification when it changes. A static item's `subscribe` never calls `onChange`. */
export type LoaderItem<TValue> = {
  read(): TValue;
  subscribe(onChange: () => void): CleanupFn;
};
```

`LoaderItem` is Relay's `LiveState<T>` (`read(): T`, `subscribe(cb): () => void`; Relay docs, Live Fields), which pico-runtime.md's client roots already name as the shape an external value needs (Other derived node kinds, the `with` blocks sketch). An item that changes over time is not the derived node's value; it is a source (An item that changes over time).

### The reader AST

```ts
// from libs/isograph-runtime/src/artifacts.ts
export type ReaderSelection =
  | ReaderScalarSelection
  | ReaderLinkedSelection
  | ReaderRefinementSelection
  | ReaderResolverSelection
  | ReaderLoaderSelection;

/** A loader field selection: the instance at (link, field, arguments), and its item's value once loaded. */
export type ReaderLoaderSelection = {
  readonly kind: 'Loader';
  readonly alias: string;
  readonly arguments: Arguments | null;
  readonly loaderArtifact: () => LoaderArtifact<ResolverParam, unknown, unknown, EntityName>;
};

/** Whether a selection's subtree contains a `Loader` selection, through nested selections and through the reader ASTs of selected fields. */
export type ReachesLoader = 'ReachesLoader' | 'NoLoader';
```

`ReaderLinkedSelection`, `ReaderRefinementSelection`, and `ReaderResolverSelection` each gain `readonly reachesLoader: ReachesLoader`. The compiler computes it: a selection reaches a loader when its own selections contain a loader selection or a selection that reaches one, and a `Resolver` selection reaches a loader when the selected field's reader AST does. This is a fixed point over the client fields, because a field's reader AST can select fields that select it back only through links, and each field is visited once. Where the flag lives is open decision 4.

### Instance identity

```ts
// from libs/isograph-runtime/src/loaders.ts
declare const loaderInstanceKeyBrand: unique symbol;

/** (link, field, arguments): `resolverDerivedNode(link, resolverStoreKey(selectableName, variables))`'s string, under its own brand. */
export type LoaderInstanceKey = string & { readonly [loaderInstanceKeyBrand]: true };

export function loaderInstanceKey(link: StoreLink, storeKey: StoreKey): LoaderInstanceKey;
```

The key is upstream's stable id (`read.ts:332-339`) with the arguments limited to those the selection passes and the field's defaults (pico-runtime.md, The resolver store key). It does not include arguments supplied at a hook call, because the effect set computes keys from data alone (open decision 2).

### Phases

- Inputs. A pure derived node at `loaderInputsDerivedNode(link, storeKey)`, whose computation reads the loader's `inputs` reader artifact at the link with the field's variables. Lazy, cached, and pulled in any world, like any resolver derived node.
- Construct. The runtime calls `load(inputs)` for an instance when it is first held, and again when its inputs node recomputes to a value `valuesEqual` rejects. An unrelated write that marks the inputs node and leaves its value equal is cut off at the inputs node, so it never reaches `load`. When the inputs change, the order of the old cleanup and the new `load` is open decision 6.
- Retain. Instances are shared by key. Each holder adds one to the instance's count: an effect root whose effect set contains the key, or a temporary retain from an uncommitted render. When the count reaches 0, the runtime calls the cleanup. This is change 3's `rootCount` model (pico-runtime.md, Retention) applied to instances.
- Release. A holder releases when its effect set stops containing the key (a link retargeted, a refinement no longer holds, the inputs node's record was deleted), when its query is unmounted (a route change, a component unmounting), or when a temporary retain expires.

```ts
// from libs/isograph-runtime/src/loaders.ts
/** One loader instance: its key, its artifact, what `load` returned, and who holds it. */
export type LoaderInstance = {
  readonly key: LoaderInstanceKey;
  readonly artifact: LoaderArtifact<ResolverParam, unknown, unknown, EntityName>;
  state: LoaderInstanceState;
  /** Uncommitted renders holding the instance; each expires. */
  temporaryRetainCount: number;
  /** Effect roots whose last reconciled effect set contains `key`. */
  permanentRetainCount: number;
};

/**
 * `Loaded`: `load(inputs)` returned `item` and `cleanup`; `unsubscribe` stops the runtime's subscription to `item`.
 * `LoadThrew`: `load(inputs)` threw `error`. A later inputs value loads again.
 */
export type LoaderInstanceState =
  | {
      readonly kind: 'Loaded';
      readonly inputs: unknown;
      readonly item: LoaderItem<unknown>;
      readonly cleanup: CleanupFn;
      readonly unsubscribe: CleanupFn;
    }
  | { readonly kind: 'LoadThrew'; readonly inputs: unknown; readonly error: unknown };
```

`temporaryRetainCount` and `permanentRetainCount` are network-and-subscriptions.md's `RequestEntry` counts (Request entries in the store), and the instance stays in `environment.loaderInstances` while either is above 0. That is the deviation from `ParentCache`, which forgets an item once its temporary retains are gone (`CacheItem.ts:343-350`). Reason: the reconciler finds a kept instance by key after commit; without that, every write that recomputes an effect set would load the kept instances again (open decision 9).

### An item that changes over time

An item's value is a source, not the inputs node's value and not the effect-set node's value. The runtime mirrors each loaded item:

```ts
// from libs/isograph-runtime/src/sources.ts
/** The current value of the item of the loader instance `key`. */
export function loaderItemSource(key: LoaderInstanceKey): SourceId;
```

- After `load` returns, the runtime calls `item.subscribe(onChange)` and stores the unsubscribe in `Loaded.unsubscribe`. `onChange` queues `markChanged([loaderItemSource(key)])` as a write (open decision 7).
- A reader walk at a `Loader` selection calls `readSource(loaderItemSource(key))` and returns `{ kind: 'Loaded', value: item.read() }` when the instance is `Loaded`, `{ kind: 'NotLoaded' }` when no instance is stored, and `{ kind: 'LoadThrew', error }` otherwise (open decision 2).
- Releasing an instance calls `unsubscribe`, then `cleanup`, then `deleteSource(loaderItemSource(key))`, which marks every reader `Dirty`; they recompute to `NotLoaded` when pulled.

A pure field downstream of a loader reads the item through the walk, so its dependency is recorded and `markChanged` reaches it. This is the client roots' rule (pico-runtime.md, Other derived node kinds): a source is current only if every write to it reports a change, and `subscribe` is the report.

## The effect set

The compiler knows which fields are loaders, so it knows which selection sets reach a loader (`ReachesLoader`). Each mounted query has an effect-set derived node. Its computation walks the entrypoint's reader AST from the query's root, descends only into selections whose `reachesLoader` is `ReachesLoader`, reads each loader's inputs node, and returns the instances it found.

```ts
// from libs/isograph-runtime/src/loaders.ts
/** The loader instances a mounted query reaches, as of its effect-set node's last computation. */
export type EffectSet = ReadonlyMap<LoaderInstanceKey, EffectSetEntry>;

export type EffectSetEntry = {
  readonly artifact: LoaderArtifact<ResolverParam, unknown, unknown, EntityName>;
  /** The value of the instance's inputs node. */
  readonly inputs: unknown;
};

/** One mounted query: its effect-set node and what the reconciler last acquired for it. */
export type EffectRoot = {
  readonly entrypoint: IsographEntrypoint<ResolverParam, unknown, EntityName>;
  readonly variables: Variables;
  /** `effectSetDerivedNode(entrypoint.normalizationAst.root, resolverStoreKey(entrypoint field, variables))`. */
  readonly effectSetNode: DerivedNodeId;
  /** The effect set the reconciler last acquired holds for. */
  held: EffectSet;
  /** `mountQuery` calls not yet released. */
  mountCount: number;
};
```

The environment gains `loaderInstances: Map<LoaderInstanceKey, LoaderInstance>` and `effectRoots: Map<EffectRootKey, EffectRoot>`, where `EffectRootKey` is network-and-subscriptions.md's `NetworkRequestKey` (`entrypoint.queryText + JSON.stringify(stableCopy(variables))`).

The walk, as a computation over pico-runtime.md's `ComputeContext` (a runtime-defined computation, Follow-ups 10):

```ts
// from libs/isograph-runtime/src/loaders.ts
/** The computation of an effect-set derived node: the loader instances `selections` reach at `link`. */
function effectSetComputation(
  selections: ReaderSelectionSet,
  link: StoreLink,
  variables: Variables,
): Computation<GraphValue>;

/** The entries `selections` reach at `link`, reading each `Linked` value, `Refinement` condition, and loader inputs node through `context`. */
function effectEntries(
  context: ComputeContext<GraphValue>,
  selections: ReaderSelectionSet,
  link: StoreLink,
  variables: Variables,
): Iterable<readonly [LoaderInstanceKey, EffectSetEntry]>;
```

`effectEntries` yields nothing for a `Scalar` selection or a selection whose `reachesLoader` is `NoLoader`. A `Linked` selection reads the link through the walk's hooks, as `read` does, and recurses at each linked record. A `Refinement` reads the record's entity, as `read` does, and recurses when the condition holds. A `Resolver` selection that reaches a loader recurses into the selected field's reader AST at the same link with `childVariables(variables, selection.arguments)`; it does not call the resolver. A `Loader` selection reads the inputs node with `context.readDerivedNode(loaderInputsDerivedNode(link, storeKey), ..)` and yields one entry. Missing data on a path yields nothing for that path.

`GraphValue` is the widened value type Follow-ups 10 names: `ResolverResult<unknown> | EffectSet | ...`, with `valuesEqual` answering unequal for two kinds. Two `EffectSet`s are equal when they have the same keys and each key's `inputs` are `valuesEqual` and its artifact is `===`.

### After a write

Under held 3b (pico-runtime.md, Held: 3b, mark, then pull), a write marks and runs nothing. Then:

1. `settle` pulls the observed derived nodes (change 4).
2. `notify` calls the observers whose value changed.
3. `reconcileEffectRoots` pulls every effect root's effect-set node. The pull brings up to date only the derived nodes on paths to loaders: the links the walk read and the inputs nodes. Every pure field the walk does not read stays marked (`Check` or `Dirty`) until a render or another pull reads it.
4. For each effect root whose effect-set node's value changed, the reconciler diffs `held` against the new value: it acquires each new key and releases each dropped key, and sets `held`.
5. For each instance whose current inputs are not `valuesEqual` to the inputs it was loaded with, the reconciler loads again with the new inputs and releases the old load, in the order open decision 6 decides.
6. `flushTeardowns` runs the releases.
7. The writes queued during 3 to 6 (an item's `onChange`, a `load` that wrote a source) run as the next write, from step 1.

```ts
// from libs/isograph-runtime/src/loaders.ts
/** Pulls each effect root's effect-set node; acquires the instances each one newly reaches, releases the ones it no longer reaches, and reloads each instance whose inputs changed. Each `load` and cleanup runs through `reportCallbackError`. Never throws. */
export function reconcileEffectRoots(environment: IsographEnvironment): void;

/** Mounts the query `entrypoint` with `variables`: creates its effect root or adds to its `mountCount`, pulls its effect-set node, and acquires every instance it reaches. The cleanup removes the mount; the last one releases every instance in `held` and deletes the root. */
export function mountQuery<TParam extends ResolverParam<TEntityName>, TOutput, TEntityName extends EntityName>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
): CleanupFn;
```

Pulling the effect-set nodes after every write is open decision 3's (a) for these nodes alone: pico-runtime.md holds that a retained query nobody observes stays marked (open decision 3, held (b)), and this doc does not change that for pure fields. It is the case pico-runtime.md's Reconciliation already makes for mounted nodes with an effect: "an effect whose inputs changed must be replaced even when no reader reads the node."

### The link move

`PetDetail` on `Pet:1` selects `bestFriend { Presence }`, and `Presence` is a loader whose inputs are `{ id }`.

```text
before  effect set of PetDetailRoute(id: 1): { Pet:2/Presence -> inputs "2" }
        instance Pet:2/Presence: Loaded, permanentRetainCount 1

write   Pet:1.bestFriend = Pet:3
mark    effect-set node Dirty (it read Pet:1.bestFriend); Pet:2/Presence inputs node untouched
settle  observed nodes; PetDetail's own resolver node is pulled only if observed
pull    effect-set node: reads Pet:1.bestFriend -> Pet:3, creates and reads Pet:3/Presence inputs node -> "3"
        new value { Pet:3/Presence -> inputs "3" }, unequal
diff    acquire Pet:3/Presence: not stored, load("3"), permanentRetainCount 1
        release Pet:2/Presence: permanentRetainCount 0, temporaryRetainCount 0 -> pushed to teardowns
flush   Pet:2/Presence: unsubscribe, cleanup, deleteSource(loaderItemSource(Pet:2/Presence))
```

A write to `Pet:2.name` marks nothing the effect-set node read, so the pull finds it `Clean` and the reconciler does nothing. If `Presence` also selected `name`, a write to `Pet:3.name` would mark `Pet:3/Presence`'s inputs node `Dirty` and the effect-set node `Check`; the pull recomputes the inputs node, and only if its value changed does the effect-set node recompute and the instance reload.

### The invariant

A loader instance is held only through an effect root that reaches it, or through a temporary retain from an uncommitted render. An effect root exists only while its query is mounted, and a mounted query is retained (The query fetch as a loader). So an instance's lifetime is contained in its query's retention, plus the temporary retain's window. It is bounded by current data as well as by unmount: an instance the data stops reaching is released by the next write's reconciliation, whether or not any component re-renders.

Imperative loads are outside the invariant (open decision 14).

## The router

The URL is a source: browser state the outside world owns, which arrives as an event (AGENTS.md, Shared state and interior mutability, last paragraph). The mounted entrypoints are a derived node over it, and the app's effects are the union of the mounted entrypoints' effect sets.

```ts
// from libs/isograph-runtime/src/router.ts
/** The current URL, as the router last reported it. */
export function urlSource(): SourceId;

/** A route table: which entrypoints, with which variables, a URL mounts. Pure. */
export type Routes = (url: URL) => ReadonlyArray<MountedEntrypoint>;

export type MountedEntrypoint = {
  readonly entrypoint: IsographEntrypoint<ResolverParam, unknown, EntityName>;
  readonly variables: Variables;
};
```

A route change is a write: the router reports the new URL with `markChanged([urlSource()])`, the mounted-entrypoints node recomputes, and the reconciler mounts the entrypoints that appeared and unmounts the ones that went. Then each new effect root's effect set is pulled and its instances acquired; each unmounted root releases what it held. A route change is mark, pull, diff, the same three steps as any write.

Two caveats are open decisions:

- Mounting follows the committed world. Under concurrent React a navigation inside `startTransition` renders the new route in a pending world while the old one stays on screen. If the URL source changes at navigation, the old route's instances are released before the new route commits, and the screen shows a route whose resources are gone (open decision 11).
- Components outside every route. A modal that retains a query is not in the route table, so no URL-derived node mounts its query (open decision 10).

Upstream has no router. Its demo uses Next.js's router, and each route's page component calls `useLazyReference` (`demos/pet-demo/src/components/routes.tsx:52-55`).

## Loading during render

A render may need an item before any commit mounts a query that reaches it: render-as-you-fetch. The render loads under a temporary retain, and the commit adopts the instance.

```ts
// from libs/isograph-runtime/src/loaders.ts
/** The instance at `key`, loaded with `inputs` when absent, with one more temporary retain that expires after `temporaryRetainTime`. The cleanup removes the temporary retain. */
export function retainLoaderTemporarily(
  environment: IsographEnvironment,
  key: LoaderInstanceKey,
  artifact: LoaderArtifact<ResolverParam, unknown, unknown, EntityName>,
  inputs: unknown,
): ItemCleanupPair<LoaderItem<unknown>>;
```

1. Render: the React package reads the loader field's selection, gets `NotLoaded`, computes the key and pulls the inputs node, and calls `retainLoaderTemporarily`. `load` runs at render, once per key, however many renders retry.
2. Commit: the component's `mountQuery` reconciles its effect root, finds the instance by key in `loaderInstances`, and adds a permanent retain. Then the render's temporary retain is released. The instance's count never reaches 0 in between, so it is not loaded again.
3. No commit: the temporary retain expires, and an instance with no other retain is released.

This is upstream's `useCachedResponsivePrecommitValue` (`disposable-state/useCachedResponsivePrecommitValue.ts:40-116`): `getOrPopulateAndTemporaryRetain` at render (`:112-113`, `ParentCache.ts:55-59`), `permanentRetainIfNotDisposed` at commit (`:78-86`, `CacheItem.ts:114-193`), and, when the temporary retain expired before commit, the cache or the factory again (`:91-98`). The default expiry is 5000 ms (`CacheItem.ts:7`, `:249-252`). The delta: adoption is by key through the effect root, not through the `CacheItem` object the render held, because the commit's retain comes from the effect set, which knows keys.

A pending forked world can start loaders this way: a render in that world pulls the inputs node in that world and takes a temporary retain. It does not mount an effect root; only the committed world has effect roots (Forks).

## The query fetch as a loader

An entrypoint's fetch is a built-in loader at the entrypoint's root.

- Identity: the root link, the entrypoint's field, the variables. This is `NetworkRequestKey` (network-and-subscriptions.md, The request cache) in another form.
- Inputs: the root resolver derived node's kind on its variables, `MissingData` or not, which is pico-runtime.md's Follow-up 11 decision (`IfNecessary` fetches when the root is `MissingData`).
- Load: `fetchEntrypoint` (network-and-subscriptions.md), which returns `ItemCleanupPair<PromiseWrapper<void, unknown>>` and retains the query. The item is static: its value is the `PromiseWrapper`, which react.md's suspension decision reads.
- Every effect set contains its own root's fetch instance first, so an effect root holds its query's retention through the fetch instance's cleanup. That is what makes the invariant's containment hold.

Whether a root that becomes `MissingData` while mounted (after a collection, or a write that removed a link) fetches again is open decision 13.

`@loadable` is the same shape at a selection instead of a root: a fetch of the selected field's own query, keyed by (link, field, args), whose inputs are the refetch reader's data (`read.ts:290-299` reads `refetchReaderAst`, in practice `id`). Whether `@loadable` becomes this built-in loader is open decision 1.

## `eager`

A built-in loader whose item is a hold on its own pure node: while the query is mounted, the node is pulled after every write.

```ts
// from libs/isograph-runtime/src/loaders.ts
// the built-in eager loader's load, for the pure node `id`:
load: () => [staticItem(undefined), observe(environment.graph, id, () => {})],
```

`observe` with a callback that does nothing makes the node observed (pico-runtime.md, change 4), so `settle` pulls it after every write, and `collect` keeps it through `rootCount`. The cleanup removes the observer.

Uses:

- Precompute an expensive resolver before render, so the first render of a newly mounted component reads a `Clean` value instead of the result from before the last write (pico-runtime.md open decision 3's cost under held (b)).
- A value consumed outside React (a document title, an analytics context), current after every write while the query is mounted.

Whether `eager` is syntax, and what it is called, is open decision 12. The name collides with two existing meanings: upstream's `EagerReaderArtifact` is a non-component client field (`reader.ts:25-36`), and i2's `Loadability::Eager` is a selection without `@loadable` (`crates/isograph_parser/src/selections.rs:36-43`).

## Syntax or userland

A feature is syntax when the compiler must know it: it changes generated query text, the operation type, or static analysis such as `ReachesLoader`. It is a userland loader when its behavior is opaque to the compiler.

- Syntax: the query fetch, refetch, `@loadable`, pagination (each generates query text or a refetch artifact); GraphQL subscriptions or `@live` (operation type); client roots (pico-runtime.md, Other derived node kinds; the compiler types their selectables); `eager`, if the effect set must see it to bound its lifetime by data.
- Userland loaders: localStorage write-through, impression logging, a third-party SDK's subscription that writes a source.
- Between the two: polling (a timer; opaque, but its period may want to be part of the instance key), and prefetching a linked route's entrypoint (the route table is the router's, but the entrypoint and its variables are compiler artifacts).

Upstream has no live field and no runtime subscription. `operationKind: 'Query' | 'Mutation' | 'Subscription'` exists only on a persisted operation's extra info (`entrypoint.ts:70-74`); the GraphQL schema parser accepts a subscription root (`crates/graphql_schema_parser/src/parse_schema.rs:437-450`), and nothing in `libs/isograph-react/src` opens one.

## Forks

The goal is a tree of worlds: the committed world and pending optimistic or transition worlds forked from it. Upstream supports a stack, not a tree: each store layer has one `childStoreLayer` (`optimisticProxy.ts:100-160`), and removing a layer re-executes the layers above it (`optimisticProxy.ts:393-493`).

Effects run only in the committed world. Effect roots, the reconciler, and permanent retains exist only there. A pending world may pull any pure node, including inputs nodes and effect-set nodes, for its own render, and may start loaders under temporary retains (Loading during render). It never acquires a permanent retain. When the pending world commits, its mounts become the committed world's effect roots, and the reconciler adopts its temporary retains by key.

pico-runtime.md cites a Follow-ups 12 for forks (Values with lifecycles; The engine (2a), `stateOf`), and its Follow-ups list ends at 11.

## End-user experience

A userland loader. The directive name is a placeholder (`@LOADER`) until open decision 1.

```ts
// from demos/pet-demo/src/Pet/Presence.ts
export const Presence = iso(`
  field Pet.Presence @LOADER {
    id
  }
`)({
  inputs: ({ data }) => data.id,
  load: (petId) => {
    const presence = presenceSdk.watch(petId);
    return [
      {
        read: () => presence.current(),
        subscribe: (onChange) => presence.onChange(onChange),
      },
      () => presence.close(),
    ];
  },
});
```

A pure field that reads it, and a component:

```ts
// from demos/pet-demo/src/Pet/PetBestFriendCard.tsx
export const PetBestFriendCard = iso(`
  field Pet.PetBestFriendCard @component {
    bestFriend {
      PetName
      Presence
    }
  }
`)(function PetBestFriendCard({ data }) {
  const presence = data.bestFriend?.Presence;
  return (
    <div>
      {data.bestFriend?.PetName.name}
      {presence?.kind === 'Loaded' ? (presence.value.online ? ' (online)' : '') : null}
    </div>
  );
});
```

What the user sees:

- Mounting the route that reaches `PetBestFriendCard` on `Pet:1` opens one presence watch for the best friend, whether or not `PetBestFriendCard` has rendered yet.
- Setting `Pet:1.bestFriend` to `Pet:3` closes the `Pet:2` watch and opens the `Pet:3` watch after that write, with no React render in between.
- Two components that render the same best friend's `Presence` share one watch.
- A presence change re-renders only the components whose selections read `Presence` on that record.
- Navigating away closes the watch.

## What this changes in other docs

None of these is edited by this doc.

- pico-runtime.md, change 4: the derived nodes the runtime pulls after a write are the observed ones (`settle`) and the effect-set nodes of mounted queries (`reconcileEffectRoots`). "Mounted derived nodes with an effect" in Held: 3b's list and in the Goal become effect-set nodes of effect roots.
- pico-runtime.md, Values with lifecycles: effects move from derived nodes to loader instances. `DerivedNodeDefinition.effect`, `NodeEffect`, `ObservedEffect`, and `reconcileEffects` are replaced by `LoaderArtifact.load`, `LoaderInstance`, and `reconcileEffectRoots`. An effect is set up when an effect root reaches its instance, not when `observe` mounts its node; a component observing a node no longer starts anything. Reconciliation sets up the new effect before tearing down the old; whether loaders keep that order for an instance whose inputs changed is open decision 6. The rule "a retained query nobody renders mounts nothing" holds with mount meaning `mountQuery`.
- pico-runtime.md, open decision 13: (a) places lifecycles in the engine per derived node; this doc places them per loader instance in the runtime, with the engine unchanged except the widened value type of Follow-ups 10.
- pico-runtime.md, Follow-ups 1, 2, 4, 5, 6: a client pointer's follow-up query is a built-in loader keyed by the pointer's target; deduplication (Follow-ups 5) is sharing by `LoaderInstanceKey`; cancellation (Follow-ups 6) is the loader's cleanup aborting the request.
- pico-runtime.md, Follow-ups 11: the fetch decision becomes the fetch loader's inputs (The query fetch as a loader).
- network-and-subscriptions.md, Fetching: `fetchEntrypoint` is the fetch loader's `load`. `getOrCreateCacheForArtifact` and the request cache of `ParentCache`s are replaced by `loaderInstances`; change 7's `RequestEntry` counts are `LoaderInstance`'s. The two-phase lifetime for requests is Loading during render. `readAndSubscribe`'s own two-phase lifetime for subscriptions is unchanged.
- components-in-userland.md: component fields are pure, unchanged. A component field whose selection set reaches a loader is a `Resolver` selection with `ReachesLoader`, and the effect walk descends into its reader AST although the parent does not read the component's data (A field whose result is a closure). Its open question 3 (a data derived node per closure) does not block this: the effect walk reads only the paths to loaders, not the closure's data node.
- react.md: `useLazyReference`'s commit calls `mountQuery`; a loader field read at render that is `NotLoaded` calls `retainLoaderTemporarily`.

## Open decisions

### 1. The loader directive, and what becomes of `@loadable`

Everything about Isograph's syntax can change, including `@loadable`.

What upstream and i2 have:

- Upstream's `@loadable` is a selection directive: `ScalarSelectionDirectiveSet::Loadable(LoadableDirectiveSet)` (`crates/isograph_lang_types/src/selection_directive_set.rs:9-13`, `:28-32`) with one parameter, `lazyLoadArtifact` (`:39-44`). It is not a property of the field: the same field can be selected normally or loadably (`crates/isograph_schema/src/field_loadability.rs:22-39`, whose header says the model is deprecated and that `@loadable` should become the single source of truth for imperative loading, `:1`, `:17-21`). The reader node is `LoadablySelectedField` with the field's entrypoint and refetch reader AST (`reader.ts:144-159`, generated by `loadably_selected_field_ast_node`, `crates/artifact_content/src/reader_ast.rs:540-620`); the param type is `LoadableField<param, output, Omit<..>>` (`crates/artifact_content/src/generate_updatable_and_parameter_type.rs:619-650`); the merged query does not include the field's selections and generates a separate entrypoint for it (`crates/isograph_schema/src/create_merged_selection_set.rs:977-1010`).
- i2 parses `@loadable` as `SelectionDirectiveName::Loadable` (`crates/isograph_parser/src/directives.rs:33-36`), rejects arguments (`AstError::LoadableArguments`, `:120-124`), exposes `Loadability { Eager, Loadable(Span) }` (`crates/isograph_parser/src/selections.rs:36-43`), and validates only that the parent is fetchable by itself (`LoadableParentNotFetchable`, `crates/isograph_compiler/src/validation.rs:56-67`, `:471-475`). The selection otherwise behaves as without the directive (graphql-protocol/node-entrypoints-and-loadable.md, opening section). i2 accepts no declaration directive: `DeclarationDirectiveName` is empty (`directives.rs:38-41`).

The loader kind is a property of the field declaration. `@loadable` is a property of a selection. The options differ in whether one name serves both and in what the selection-site form means.

- (a) A new declaration directive for the loader kind, `@loadable` unchanged. Parsing: one variant in `DeclarationDirectiveName`, and `ClientFieldKind` back in `crates/isograph_compiler/src/schema.rs` (removed by iso-directives-unexpected.md change 2). Artifacts: the `Loader` reader node and `LoaderArtifact`. Runtime: `ReaderLoaderSelection`. User code: new fields add the directive; `@loadable` selections are as today. Cost: two names for two related concepts; a `@loadable` fetch stays outside the effect set, started by a hook, with component lifetime, so the invariant does not cover it.
- (b) A new declaration directive for the loader kind, and `@loadable` redefined: a `@loadable` selection is an instance of the built-in fetch loader at (link, field, args), in the effect set, started when the query mounts. Parsing: (a)'s, plus `@loadable` accepts the field's arguments in the selection (`PetCheckinsCard(limit: 2) @loadable`), which i2 already accepts as selection arguments; `AstError::LoadableArguments` stays for directive arguments. Artifacts: the `LoadablySelectedField` reader node becomes a `Loader` node whose artifact is the built-in fetch loader over the field's entrypoint; refetch artifacts as upstream. Runtime: the parent receives the item's value (`PromiseWrapper` and the fragment reference) instead of `LoadableField`. User code: `useClientSideDefer(pet.PetCheckinsCard, { limit: 2 })` becomes a read of `pet.PetCheckinsCard` with the arguments in the selection; `useImperativeLoadableField` has no counterpart (open decision 14). Cost: hook-supplied arguments go away.
- (c) One name, `@loadable`, at both positions. On a declaration it makes the field a loader; on a selection it is (b)'s built-in fetch loader. The parser already branches on `DirectivePosition` (`directives.rs:43-47`), so this is one variant in `DeclarationDirectiveName` with the name `loadable`. Changes: (b)'s. Cost: one name with two meanings that position separates, against the rule that a word has one meaning.
- (d) Rename `@loadable` to the loader directive's name at both positions, with (c)'s meanings. Changes: (c)'s, plus every `@loadable` in user code and in i2's parser tests (`crates/isograph_parser/src/format.rs:288-289`, `parse_iso_literal.rs:1172-1215`) and the validation message (`validation.rs:61`). Cost: (c)'s second meaning stays, under a new name; upstream code does not compile unchanged.
- (e) Subsume `@loadable` into the loader kind: no selection-site directive. A field that wants a separate fetch is declared as a loader whose `load` is the built-in fetch (a declaration directive with a parameter, or a generated loader). Changes: `SelectionDirectiveName::Loadable` and `Loadability` are removed; validation of fetchability moves to the declaration. Cost: the same field cannot be selected both normally and loadably, which upstream allows.

Held: none. The name and the choice among (a) to (e) are the user's. The `Eager` variant of `Loadability` is renamed under any option that keeps `Loadability`, because `eager` is open decision 12's name candidate.

### 2. What a reader of a loader field receives

- (a) The item's value as a `LoaderRead<TValue>`: `{ kind: 'Loaded'; value: TValue } | { kind: 'NotLoaded' } | { kind: 'LoadThrew'; error: unknown }`, read through `loaderItemSource(key)`. Pure readers depend on the item through a recorded source. Cost: a pure field's value depends on whether the instance is loaded yet, so the first pull before reconciliation returns `NotLoaded` and the queued write after `load` recomputes it.
- (b) A `LoaderReference<TValue>` carrying only the key, equal across writes; the React package resolves it to the item. Cost: pure fields cannot compute from the item; only components read it.
- (c) Upstream's description: `(args, fetchOptions) => [StableId, Factory]` (`reader.ts:171-183`), with arguments at the call. Cost: identity depends on the caller's arguments, so the effect set cannot compute it from data, and the invariant does not hold.

Held: (a). Deviation from upstream's (c), reason: the effect set computes instance keys from data alone. (a) over (b): a pure field over a live value (Presence feeding a label) is the case client roots exist for.

### 3. Which nodes the runtime pulls after a write

- (a) Observed nodes and effect-set nodes of effect roots. Held (this doc).
- (b) Observed nodes only; effect-set nodes are pulled when a component renders. Cost: an instance the data stopped reaching stays held until a render, which is upstream's behavior and loses the data-bounded lifetime.
- (c) Every retained root (pico-runtime.md open decision 3 (a)). Cost: every retained query's pure fields run on every write.

### 4. Where `ReachesLoader` lives

- (a) A field on `Linked`, `Refinement`, and `Resolver` reader nodes; the effect walk walks the reader AST. Reuses the reader AST and its variable mapping (`childVariables`). Cost: one field per node in every reader artifact.
- (b) A separate effect AST per entrypoint: the merge of every path to a loader, with resolver boundaries inlined and variables substituted, as query generation merges reader ASTs into the normalization AST. Cost: a new compiler merge and artifact; a resolver boundary inlined loses the per-field inputs node location unless the merge keeps it.

Upstream has no counterpart. Held: (a), reason: it reuses the reader AST seam.

### 5. Where effect-set nodes and loader instances live

- (a) Effect-set nodes in `environment.graph` as a runtime-defined derived node kind (Follow-ups 10), owned by the root record's `recordSource`; loader instances in `environment.loaderInstances`, outside the engine. Held: the engine stays store-agnostic (pico-runtime.md open decision 12) and knows nothing of instances.
- (b) Instances in the engine, as pico-runtime.md open decision 13 (a) puts lifecycles. Cost: the engine gains reference counts, load, cleanup, and timers.

### 6. Order of cleanup and load when an instance's inputs change

The instance's key does not change when its inputs change, so the question is only the order of the old load's release and the new `load` for one instance. pico-runtime.md's Reconciliation holds, for derived-node effects, "A new effect is set up before the old one is torn down", and tears the old one down after notification ("A replaced or disposed effect is torn down after every observer of that settlement was notified").

- (a) Cleanup, then load. In step 5 the reconciler calls the old `unsubscribe` and `cleanup`, then `load(newInputs)`, and stores the new `Loaded` state. Never two live resources for one instance, which an exclusive resource needs (one socket per user, one lock, an SDK that allows one watch per id). Costs:
  - A window with no resource. Between the old cleanup and the moment the new resource is live (for a socket, until it reconnects), events the outside world sends are delivered to neither; a loader that must not lose them needs its own replay.
  - What readers see. `cleanup` and `load` run in the same synchronous step, and no pull happens between them, so no reader observes `NotLoaded` from the order alone. Readers see the new item's first `read()`, which for a resource that connects asynchronously is its not-ready value; that is the same gap at the item's level. Readers do observe `NotLoaded` (or `LoadThrew`) when the new `load` throws, since the old item is already gone.
  - It is React's `useEffect` order (the previous effect's cleanup runs before the next effect).
- (b) Load, then cleanup after notify, pico-runtime.md's Reconciliation order. In step 5 the reconciler calls `load(newInputs)`, calls the old `unsubscribe` at once (so the old item's `onChange` no longer marks `loaderItemSource(key)`), stores the new `Loaded` state, queues `markChanged([loaderItemSource(key)])`, and holds the old `cleanup` for the next pass: step 7 runs the queued write, whose `settle` and `notify` move readers to the new item, and that pass's `flushTeardowns` runs the old cleanup. No gap: readers move from the old item's value to the new item's value, and a new `load` that throws leaves the old resource live until the flush. Costs:
  - Two live resources for one instance between `load` and the flush. An exclusive resource fails or evicts the old one, and a server that counts connections counts two.
  - A cleanup held for the next pass is one more list on the environment, and After a write's step 6 flushes only the cleanups held from an earlier pass, so the old item is not released before readers moved off it.
- (c) Per loader, a declaration option. `LoaderArtifact` gains `readonly reloadOrder: ReloadOrder`, with `export type ReloadOrder = 'CleanupThenLoad' | 'LoadThenCleanup'`, written by the compiler from an argument of the loader directive (open decision 1), with a default. It holds up: the right order is a property of the resource, which only the loader's author knows (exclusive resources need (a); a gapless display needs (b)), and both orders use the reconciler's existing steps. Costs: one more directive argument and a default to choose; the built-in loaders each pick one (the fetch loader and `eager` have no exclusive resource, so (b)).
- (d) Hand-off: `load(inputs, previous)` receives the old item and cleanup and decides itself, returning the new pair; the runtime never calls the old cleanup when `load` took it over. Costs: every loader handles the case; the runtime cannot tell whether the old resource was released.

Upstream has no counterpart: a `@loadable` hook given new arguments gets a new `ParentCache` and disposes the old item when the hook's effect for the new cache runs (`disposable-state/useLazyDisposableState.ts:31-43`, keyed on `[parentCache]`), which is (a) at commit.

Held: none. Undecided.

### 7. Writes during reconciliation

A `load` may write a source synchronously (a live item's initial value, a localStorage read), and an item's `onChange` may fire during another load.

- (a) Queue them; after `flushTeardowns`, run the queue as the next write (step 7 of After a write). Held. Cost: a loop that ends only when loads stop writing; a loader that writes on every load of an instance whose inputs it changes does not terminate.
- (b) Forbid synchronous writes: `markChanged` from inside `load` throws. Cost: an initial value shows one write late.
- (c) Re-enter: mark and settle inside the reconciler. Cost: a pull during a diff; the reconciler's `held` maps change under it.

### 8. Effect-set equality

- (a) Same keys, `===` artifacts, `valuesEqual` inputs. Held.
- (b) Same keys only, with inputs changes found by comparing each instance's loaded inputs. Cost: the effect-set node recomputes and is equal while an inputs node changed, so the reconciler must visit every instance on every write to find inputs changes.

### 9. Sharing instances after commit

- (a) Keep every instance in `loaderInstances` while any retain holds it. Held. Deviation from `ParentCache` (`CacheItem.ts:343-350`), reason: the diff must find a kept instance by key. network-and-subscriptions.md change 7 makes the same deviation for requests.
- (b) Upstream's `ParentCache`. Cost: every recomputed effect set reloads every kept instance.

### 10. Effect roots outside the route table

- (a) A registry the router owns: `mountExtraRoot(entrypoint, variables): CleanupFn`, whose roots join the URL-derived ones. Cost: every query-retaining component must reach the router.
- (b) React commit adds a root: `useLazyReference`'s commit calls `mountQuery`, and the router's routes are one more caller. Held: upstream mounts every query from a component (`react/useLazyReference.ts:50-58`).

### 11. Mounting and the committed world

- (a) The router writes the URL source at React's commit of the navigation (a layout effect of the route component), so the pending render uses temporary retains and effect roots change only at commit. Held, until forks exist.
- (b) The URL source is per world (Forks); effect roots follow the committed world's value. Needs forks.
- (c) The URL source changes at navigation. Cost: the committed screen loses its resources before the new route commits.

### 12. `eager`

- (a) A selection directive: the selected field gets the built-in eager loader at (link, field, args), and the effect set reaches it. Held: the effect set must see it for its lifetime to be bounded by data, which a runtime call cannot give.
- (b) A declaration directive: every selection of the field is eager.
- (c) A runtime call, `retainEager(environment, location): CleanupFn`. Cost: lifetime is the caller's, not the data's.
- (d) No `eager`; open decision 3 (d) (a `snapshot` that computes) covers the first use. Cost: render does resolver work (pico-runtime.md Goal).

The name is not chosen: `eager` collides with `EagerReaderArtifact` (`reader.ts:25-36`) and `Loadability::Eager` (`selections.rs:38-39`), and upstream's entrypoint diagnostic uses lazy and eager for `@lazyLoad` (`crates/isograph_schema/src/memoized/entrypoint_access.rs:131-134`).

### 13. A mounted root that becomes `MissingData`

- (a) The fetch loader's inputs are the root's kind, so a root that becomes `MissingData` while mounted reloads, which fetches again. Cost: a collection or a write that removed a link refetches without a render asking.
- (b) The fetch loader's inputs are the variables only; the fetch runs once per mount, as upstream's `useLazyReference` does. Held: upstream's behavior.

### 14. Imperative loads

`useImperativeLoadableField` (`loadable-hooks/useImperativeLoadableField.ts:41-47`), `useImperativeReference` (`react/useImperativeReference.ts`), mutations, and pagination start from a user event, so no data determines them.

- (a) They stay holder-owned, as upstream: the component's hook holds the `ItemCleanupPair`. The invariant covers only instances the effect set reaches. Held: upstream's behavior.
- (b) The event writes a client source (a page count, an `expanded` flag on a client root), and the effect set reaches the instance through it. Cost: every imperative load needs a source and a selection of it; the instance's identity includes that source's value.

### 15. Release timing

- (a) A released instance with no retain is cleaned up in `flushTeardowns` of the write that released it. Held.
- (b) A release buffer, as upstream's `gcBuffer` for queries (`garbageCollection.ts:40-53`, default size 10, `IsographEnvironment.ts:177`), so that an instance released and reached again soon is not reloaded. Follow-ups 7's deferral. v1 removed the buffer for queries (store.md, `garbageCollect`).

### 16. A pending world's inputs

A render in a pending world computes inputs in that world. The committed world's inputs for the same key may differ.

- (a) The key excludes the world. A temporary retain from a pending world loads with that world's inputs; at commit, the reconciler compares the committed inputs and reloads when they differ. Held.
- (b) The key includes the world. Cost: no sharing between a pending render and the committed screen.

### 17. Polling and route prefetching

- (a) Userland loaders: polling is a loader whose `load` starts a timer and whose item mirrors the last response; prefetch is a loader whose `load` calls `fetchEntrypoint` for a linked route. Cost: the period and the target route are opaque to the compiler.
- (b) Syntax: a polling parameter on the fetch loader, and a prefetch selection that names an entrypoint. Cost: directives the compiler must validate.

Not held: both fit the rule in Syntax or userland on different sides.
