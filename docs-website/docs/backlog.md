# Backlog

:::note
See the [open issues](https://github.com/isographlabs/isograph/issues).
:::

## Top mid-term runtime priorities

- preloaded queries w/dispose
- garbage collection & retention
- subscriptions & granular rerendering

## Top mid-term compiler/syntax priorities

- Support for selecting arbitrary mutation fields
- Support for adding/removing fields from mutation field selections
- asFoo typecast linked fields
  - Or syntax: bar: `as Foo { ... }`? This can always be added on afterward after some thought.

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
