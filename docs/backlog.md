# Backlog

## Simplifying the model

- get rid of @fetchable resolvers, instead have syntax like `` isoFetch`Query.foo` `` (or something else).
  - Can this be an entrypoint by default? (i.e. the generated file here only contain the query text + variables, and no normalization or reader AST)?
  - all non-fetch resolvers now require JS function (enforce this)
- Get rid of @eager. All resolvers are eager, and all resolvers require a function (no identity fn)
- Once fetchable and eager are gone, there will only be regular resolvers (whose data is read along with parent) and components, whose data are read in a separate render. Is there a missing third option, where the data are read when we call a function?
  - Is this only a perf optimization? It can be achieved by not propagating ! too high.
- refactor home route etc. to not have isoFetch and iso in same file.

## Top QOL priorities

- Watch mode CLI
  x Babel integration for `iso*` literals
- eslint enforcement that resolvers are exported
- Expose compiler binary as `iso`.
- multiple errors printed

x Error printing

## Feature backlog

- garbage collection
- fetch policies
- Unwraps (i.e. `!`) exist in the syntax, but are unused
- Isograph only parses a subset of GraphQL schema syntax
- Resolvers are re-calculated every time. They should be cached in the store.
- The store, etc. should be stored in context.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can they be selected into.
- Stateful resolvers?
  - This could be thought of as "realized" resolvers, which is to say there is overlap with better DevEx for components
- Subscriptions are not supported, and mutations are supported super inflexibly.
- Defer, etc.
- Refetchable queries.
- Pagination.
- Types for variables.
- Inferred types for iso literals via Typescript compiler plugin.
- typed IDs

## Cleanup backlog

- Typegen code is a mess
- JS code needs structure, etc.
- Separate `ServerFieldId` and `ResolverFieldId` might clean up a bunch of code.
- `HACK__merge_linked_fields` is indicative of the fact that merged linked fields should contain hashmaps of key => merged item, instead of vectors of merged items.
- Objects which do not have IDs should be merged into their parent object in the store.
- Remove the requirement that arguments and parameters have a terminal comma.
- Transform should be removed.
- IsographSchemaObject, etc. should not contain name: WithLocation<...>, but instead, be stored WithLocation<\_>, and WithLocation should **not** have a span.

## Known bugs

- Typegen for non-eager non-component resolvers shows the eager type. There needs to be an additional type that is generated and threaded through.
- If a useLazyReference commits then receives new props, it does not make a new network request.
- if mutation primary field has a non-existent type, we panic, this should be an error
- incorrect spans for errors e.g. "Message: The id field on "Pet" must be "ID!"."

## Extended backlog

- Docs
- VSCode extension
- Compiler watch mode
- Fetch policies
- Garbage collection
- Preloaded queries
- Fetch/cache policies
- Granular re-renders
- Traditional refetch fields
- Defer/stream
- Subscriptions
- Interfaces/unions
- Entrypoints
- Field unwrapping syntax
- Pagination
- Refetch on missing data
- Compile to non-GraphQL
- Actually validate variables
- Error reporting (compiler)
- Persisted queries
- Suspense for mutations
- Suspense for refetch
- Strongly typed ID fields
- Custom normalizers
- Lazy normalization ASTs
- Fetching off of typed IDs
- Magic mutation/refetch field customizability
- Stateful resolvers
- Compiler executable for Mac/Windows/Linux
- Unit tests
- E2E tests
- Network request errors
- Proper missing field handlers
- Missing field handlers from schema definitions
- Store held in context
- Imperative store APIs
- Typesafe updaters
- Sample router integration
- Emit new field definitions for GraphiQL and other tools
- Field groups
- Lint rules to enforce e.g. export
- Guide for testing Isograph components
- Support non-globally unique IDs
- Iso lang code mods
- Iso lang syntax highlighting
- Iso lang auto-format
- Separate artifacts for fetching and reading fetchable resolvers
- Incremental compilation
- Saved state
- Support strict mode?
- Load resolvers iff needed
- Object literals as variables
- Server support for JSResource
- Injectable code for @component
- Isograph dev tools
- Experiment with context and JSX for @component
- Vue/Svelte/etc. integration
- Compile compiler to Wasm
- IR explorer
- Code sandbox example
- Non-fetchable fragments (???)
- Topological sort in compiler
- Validate no infinite recursion
- Statically prune inaccessible branches
  x Bug fix: Not all files in one folder
- TypeScript errors in emitted artifacts
- Better repr. of nullable types in compiler
- Babel integration for iso literal values
- Typescript integration for type inference of iso literals
- Parallelize artifact gen
- Stuff should be wrapped with WithSource, locations should not be on individual fields
- Display multiple errors, parse etc. in parallel
- Do not look in artifact_directory, if project_root contains artifact_directory

# Plan for isoFetch

- Each isoFetch encountered leads to a fetchable resolver generated
- Non-fetchable resolvers are still generated
- These are separate files. The fetchable artifact contains normalization AST + query text + nested refetch queries; the regular artifact contains the reader + resolver
  - convert can be gotten rid of
- Usage

```js
// homepage.js

// aka import query from '__isograph__/Query/home_component/fetch'
const home_page_query = isoFetch`Query.home_component`;

// home_component.js

// generates artifact in '__isograph__/Query/home_component/reader'
export const home_component = iso`
  Query.home_component @component { ... }
`(HomeComponent);
```
