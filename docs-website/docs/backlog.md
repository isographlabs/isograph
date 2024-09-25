# Backlog

:::note
See the [open issues](https://github.com/isographlabs/isograph/issues).
:::

:::warning
This is a disorganized list that is outdated in many places.
:::

## Top mid-term runtime priorities

- change how top-level fields are normalized (i.e. into their own object, without going through ROOT)
- complete work on loadable fields
  - unification
  - filters
  - normalization-time operations
  - parameters
- userland impl of:
  - pagination, etc.
  - live queries
  - useQueryLoader and variants
- extract query text and normalization AST into their own struct
- Load normalization ASTs (etc) when a non-null network response is received. Does this require `as Foo`?
  - Basically, requires normalization time stuff
- `asFoo` fields
  - requires client pointers
- isInFlight
- Typing of entrypoint variables

## Top mid-term compiler/syntax priorities

- Limit exposed fields to specific selections
- Proper selection sets (i.e. rooted at the closest loadable field?)
- Support for selecting arbitrary mutation fields
- Support for adding/removing fields from mutation field selections
- asFoo typecast linked fields
  - Or syntax: bar: `as Foo { ... }`? This can always be added on afterward after some thought.
- connections and pagination
- client links/pointers

## Top cleanup priorities

- validate no unused params and no unused variables
- subscribe to changes in pagination
- Apparently string literals aren't allowed as parameters...
- mutation/query bug for refetch fields... lol
- error parsing config should not panic, but be a diagnostic

## V2 release

- components and "realized" resolvers, as well as ways to invalidate them
  - they could also be lazily calculated
  - may require garbage collection
- cleanup types
- validate no unknown directives left over
- Handle unions etc. correctly
- Special fields (or syntax?) for type casts (i.e. type refinement)

## Feature backlog

- Ability to execute code at normalization time, e.g. for `| normalizationTimeDefer`
- Ability to define filters
- Refetch on missing data
- fetch policies
- Unwraps (i.e. `!`) exist in the syntax, but are unused
  - consider whether it is truly the case that there always is a linear way to unwrap a given field, or whether we should unify this with "execute this on the server" etc.
- Resolvers are re-calculated every time. They should be cached in the store.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can then be selected into.
- Stateful resolvers?
  - This could be thought of as "realized" resolvers, which is to say there is overlap with better DevEx for components
- Subscriptions are not supported
- Types for variables
- typed IDs
- consider resolvers that return functions only read data when called, i.e. do not eagerly read. Consider whether this can be achieved with omitting a !, i.e. foo_resolver! returns TReadFromStore, foo_resolver returns a `ReadDataResult TReadFromStore`
- refetch and mutation fields should return something you can suspend on or whatnot.
- queries in flight as store fields for suspense

## Cleanup backlog

- Typegen code is a mess
  - we should use something that parses JS/ts/flow
- JS code needs structure, etc.
- Objects which do not have IDs should be merged into their parent object in the store.
  - or weak types are scalars
- IsographSchemaObject, etc. should not contain name: `WithLocation<...>`, but instead, be stored `WithLocation T`, and WithLocation should **not** have a span.
- There should be a cleaner separation between GraphQL and Isograph. In particular, we should load the GraphQL schema, but turn it into Isograph concepts, and only deal with Isograph concepts.
- CLI should be in a separate crate from batch-compile; so should watch mode utils, so as to increase the speed of iteration if the dev is running builds.
- CLI command to create missing directories (e.g. project_root).
- do not panic when an unknown token is encountered

## Extended backlog

- Docs
- VSCode extension
- Fetch policies
- Preloaded queries
- Fetch/cache policies
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
- Subscriptions
- Interfaces/unions
- Entrypoints
- Field unwrapping syntax
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
- Topological sort in compiler? (Is this still needed?)
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
- router example and integration
- directive on scalar that affects the JS representation of scalars
- ability to pass a parameter down to the child, e.g. an abstract component can read from its concrete parent an object that implements a given interface. e.g. in order to implement Node, you must implement an id field.
- exposeField errors are pretty bad right now
- date transformations and other types
- client links that return an ID from the original data
  - use generics to enforce this e.g. require a function of `(data: Data<TIDType>) => TIDType`. So you can't return any other value.
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
  - load once: expose `{ kind: 'NotLoaded', load } | { kind: 'Loaded', data }`
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

## Unifying loadable fields

- There are at least three ways to load fields imperatively:
  - `__refetch`
  - exposed fields (i.e. magic mutation fields)
  - @loadable fields
- Exposed fields
  - Exposed fields must be fetched in a follow-up request, since: we need data from the original request (i.e. what we read with the refetch reader) and we need to make a separate request (due to limitations of GraphQL). This is true whether they have different root objects or not.
    - Exposed fields which have an empty refetch reader selection set (not supported?) could be auto-hoisted iff the path from the root to the field contains only non-null fields. But fields with no reader selection set are a perfect use case for `root`, so we should probably not have different behavior here.
    - Fields re-exposed on themselves are a weird edge case, and should probably behave identically to regular ol' exposed fields.
  - However, we can hide this from the user and execute them as an immediate follow up.
- `__refetch` fields can always be merged into the parent, and fetched along with it.
- So, we can make the behavior of fields as follows:
  - regular field: imperative iff selected with @loadable, selected along with parent otherwise
  - exposed: imperative iff selected with @loadable, selected as an immediate follow-up otherwise
  - `__refetch`: imperative iff selected with @loadable, merged into parent otherwise (which is a no-op)
- So, this is a bit awkward for `__refetch` fields. So, it might be better to have special fields `self`, `parent` and `root`, which behave as follows:
  - If selected as a scalar, do **not** have a resolver reader. Instead, refetch all fields selected at that location. Maybe this only makes sense for `self`?
  - If selected as a linked field, that becomes the selection set.
  - If selected non-loadably, fetch as immediate follow-ups (for `root`, or `parent` if the current field is nullable) or are merged into parent (for `self` or `parent` if the current field not nullable).
  - And maybe one should allow users to choose to make the `root` field always be fetched along with the parent (i.e. merged) if the root type is the same, otherwise as a simultaneous request?
- But in any case, that allows us to make `@loadable` the only way to fetch this, and otherwise all fields can have type `MaybeLoaded<OutputType>`, which can then be unwrapped.

## Refetch field artifact cleanup

- We only need to generate queries, not normalization ASTs. The normalization AST should be a reference into the existing normalization AST.
- We should not need to do so for loadable fields, instead generating a query text and normalization AST for each loadable field once (i.e. it's not unique to the entrypoint.)
  - At least until we start specializing for variables, e.g. BlogPost(showDetail: false) might be able to be pruned.
