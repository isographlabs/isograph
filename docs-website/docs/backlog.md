# Backlog

:::note
See the [open issues](https://github.com/isographlabs/isograph/issues).
:::

## Top mid-term runtime priorities

- preloaded queries w/dispose
- garbage collection & retention
- subscriptions & granular rerendering
  - change how top-level fields are normalized (i.e. into their own object, without going through ROOT)
- connections and pagination

## Top mid-term compiler/syntax priorities

- Support for selecting arbitrary mutation fields
- Support for adding/removing fields from mutation field selections
- asFoo typecast linked fields
  - Or syntax: bar: `as Foo { ... }`? This can always be added on afterward after some thought.
- connections and pagination
- client links/pointers

## Top implementation detail priorities

- enable unit tests for react-disposable-state

## Top cleanup priorities

- useLazyReference should refetch when props change

## V2 release

- network error handling
- components and "realized" resolvers, as well as ways to invalidate them
  - they could also be lazily calculated
  - may require garbage collection
- cleanup types
- error handling
- validate no unknown directives left over
- Handle unions etc. correctly
- Special fields (or syntax?) for type casts (i.e. type refinement)

## Feature backlog

- garbage collection
- granular re-rendering
  - Refetch on missing data
- fetch policies
- Unwraps (i.e. `!`) exist in the syntax, but are unused
  - consider whether it is truly the case that there always is a linear way to unwrap a given field, or whether we should unify this with "execute this on the server" etc.
- Resolvers are re-calculated every time. They should be cached in the store.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can then be selected into.
- Stateful resolvers?
  - This could be thought of as "realized" resolvers, which is to say there is overlap with better DevEx for components
- Subscriptions are not supported
- Defer, etc.
- Pagination.
- Types for variables
- typed IDs
- consider resolvers that return functions only read data when called, i.e. do not eagerly read. Consider whether this can be achieved with omitting a !, i.e. foo_resolver! returns TReadFromStore, foo_resolver returns a `ReadDataResult TReadFromStore`
- refetch and mutation fields should return something you can suspend on or whatnot.
- queries in flight as store fields for suspense

## Cleanup backlog

- Typegen code is a mess
- JS code needs structure, etc.
- `HACK__merge_linked_fields` is indicative of the fact that merged linked fields should contain hashmaps of key => merged item, instead of vectors of merged items.
- Objects which do not have IDs should be merged into their parent object in the store.
  - or weak types are scalars
- IsographSchemaObject, etc. should not contain name: `WithLocation<...>`, but instead, be stored `WithLocation T`, and WithLocation should **not** have a span.
- There should be a cleaner separation between GraphQL and Isograph. In particular, we should load the GraphQL schema, but turn it into Isograph concepts, and only deal with Isograph concepts.
- CLI should be separate crate than batch-compile; so should watch mode utils, so as to increase the speed of iteration if the dev is running builds.
- CLI command to create missing directories (e.g. project_root).
- do not panic when an unknown token is encountered

## Known bugs

- If a useLazyReference commits then receives new props, it does not make a new network request.
- error parsing config should not panic, but be a diagnostic

## Extended backlog

- Docs
- VSCode extension
- Fetch policies
- Garbage collection
- Preloaded queries
- Fetch/cache policies
- Granular re-renders
- Ability to select fewer or extra fields on mutation and refetch fields, e.g.

```
# Disallow +/- if = is used
# maybe = is implied
set_foo +{
  additional_field
} -{
  extraneous_field
} ={
  only_field
}

# or its on the field level
set_foo {
  +additional_field
  -extraneous_field
  -* # clear all fields
}
```

- Maybe some way to specify on the directive what fields you want to always select, since it might be annoying to do this on every selection. Though maybe you can go through another client field?
- Ability to pass as arguments such selections?
- Defer/stream
- Subscriptions
- Interfaces/unions
- Entrypoints
- Field unwrapping syntax
- Pagination
- Compile to non-GraphQL
- Actually validate variables
- Typegen for types of component's other props, somehow. Maybe have a second param that is typed.
- Persisted queries
- Strongly typed ID fields
- Custom normalizers
  - Garbage collection for custom normalizers by type
  - other "by type" things?
- Lazy normalization ASTs
- Fetching off of typed IDs
- Stateful resolvers
- Unit tests
- E2E tests
- Network request errors
- Proper missing field handlers
- Missing field handlers from schema definitions/directives
- Store held in context
- Imperative store APIs
- Typesafe updaters
- Sample router integration
- Emit new field definitions for GraphiQL and other tools (!!!)
- Field groups
- Lint rules to enforce `export const` (or compiler support)
- Guide for testing Isograph components
- Support non-globally unique IDs
  - @strong directive
- Iso lang code mods
- Iso lang syntax highlighting
- Iso lang auto-format
- Entrypoints should have separate artifacts for normalization ASTs (and types?)
- Do not hard require reader AST and normalization AST from entrypoints
- Incremental compilation
- Saved state
- Support strict mode?
  - see [this](https://github.com/facebook/relay/blob/c0cc17a07e1f0c01f3e5c564eed50b5a30f4228f/packages/react-relay/relay-hooks/useEntryPointLoader.js#L156-L189)
- Load resolvers iff needed
- Object literals as variables
- Server support for JSResource
- Injectable code for @component
  - Structure the compiler such that the injectable code can live in the React-specific CLI layer
- Isograph dev tools
- Vue/Svelte/etc. integration
- Compile compiler to Wasm
- IR explorer
- Code sandbox example
- Topological sort in compiler
- Validate no infinite recursion
- Statically prune inaccessible branches
- TypeScript errors in emitted artifacts
- Better repr. of nullable types in compiler
- Parallelize artifact gen
- Rationalize WithSpan vs WithLocation
- Can we make the babel transform export the iso literal if no export is found?? Probably!
  - What about unit tests? Should they be able to import these? Maybe? But probably only through Isograph, right?
- SWC plugin
- plugin options to point to config
- Namespaces and components installable from external libraries
- npx way to install Isograph in an existing library
- periodic refetch (live queries)
- router example and integration
- directive on scalar that affects the JS representation of scalars
- ability to pass a parameter down to the child, e.g. an abstract component can read from its concrete parent an object that implements a given interface. e.g. in order to implement Node, you must implement an id field.
- exposeField errors are pretty bad right now
- date transformations and other types
- client links that return an ID from the original data
  - use generics to enforce this e.g. require a function of `(data: Data<TIDType>) => TIDType`. So you can't return shit from anywhere else.
  - opt into non-waterfall behavior for these
  - just for fun, in unit tests, we can make the id type an object instead of a string.

# Types of fields:

- Examples: Mutation fields, lazy, refetch with or w/o variables, regular, pagination

Can vary by:

- Time:
  - Along with parent
  - Deferred
  - Upon normalization
    - this includes: deferred fields when the server doesn't support defer, and waterfalls stemming from a client link
  - Imperatively (this should include on render)
    - Can this include loading arbitrary data structures, e.g. an enum of routes w/params
- Whether we have a selection set generated for you or not
  - The type of the data from a generated selection set should not be exposed to the user.
    - Maybe this implies that they are syntactically part of parent, but exposed as field?
  - Only custom selection sets (or the custom aspects of selection sets) should be exposed
- Whether they syntactically are part of the parent (refetch, mutation fields) or not (fields)
  - i.e. Whether they affect an object or a scalar field? Maybe scalar fields are turned into objects as a result
- Whether they can be disposed (should mutations be auto-disposed? I think not.)
- And some sort of transform that affects the type of the UI exposed to users:
  - mutation: expose a set
  - refetch: expose a set, but read from the latest
  - load once: expose { kind: 'NotLoaded', load } | { kind: 'Loaded', data }
    - is there any benefit to making these load once, instead of useQueryLoader style?
  - useQueryLoader: dispose anything from previous renders
  - pagination: concatenate the items in the set

```
// UnwrappedPromise can be suspended on
type UnwrappedPromise<T> = { kind: 'Success', data: T } | { kind: 'InProgress', promise: Promise }
type Result<T, E> = { kind: 'Ok', value: T } | { kind: 'Error', error: E }
type Disposable<T> = { dispose: () => void, item: T }

// How the mutation field would show up
[mutationName]: {
  startMutation: (config: MutationConfig<TVariables>) => MutationId,
  // All non-disposed items go here? Or do we want tombstones, like... could
  // you conceivably care to know that a disposed item existed?
  // We're also implicitly depending on Maps having insertion-order iteration
  results: Map<MutationId, Disposable<UnwrappedPromise<Result<void>>>>,

  // and some derived getters? e.g.
  isMutationInFlight: boolean,
  // a promise that you can suspend on, which unsuspends only when no
  // mutation is in flight
  inFlightPromise: UnwrappedPromise<Result<void>> | null,
  // a promise you can suspend on, which unsuspends only when the most
  // recent mutation is not in flight
  mostRecentPromise: UnwrappedPromise<Result<void>> | null,
}

type MutationConfig<TVariables> = {
  variables: TVariables,
  onComplete: (data: UnwrappedPromise<Result<void>>) => void,
  // onIncremental? updater? optimistic? fetch policies?
}
```

- appear as new field, modifies parent, implicit selection set, imperatively executed
- can the suspense on mutations be syntactically unwrapped?

### Questions etc

- Where can we store this? Clearly, since we can load during normalization time, it can't be in component state.
  - So, does that mean that we suffer from the situation where multiple identical components share the same loaded queries, pagination state, etc.?
  - Or, are fields that are loaded during normalization different than those loaded post-render?
