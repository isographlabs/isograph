# Backlog

## Simplifying the model

* get rid of @fetchable resolvers, instead have syntax like `` isoFetch`Query.foo` `` (or something else).
  * Can this be an entrypoint by default? (i.e. the generated file here only contain the query text + variables, and no normalization or reader AST)?
* Get rid of @eager. All resolvers are eager, and all resolvers require a function (no identity fn)
* Once fetchable and eager are gone, there will only be regular resolvers (whose data is read along with parent) and components, whose data are read in a separate render. Is there a missing third option, where the data are read when we call a function?
  * Is this only a perf optimization? It can be achieved by not propagating ! too high.

## Top QOL priorities

* Watch mode CLI
* Error printing
* Babel integration for `iso*` literals

## Feature backlog

- garbage collection
- fetch policies
- Unwraps (i.e. `!`) exist in the syntax, but are unused
- Isograph only parses a subset of GraphQL schema syntax
- Resolvers are re-calculated every time. They should be cached in the store.
- The store, etc. should be stored in context.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can they be selected into.
- Stateful resolvers?
- Subscriptions are not supported, and mutations are supported super inflexibly.
- Defer, etc.
- Refetchable queries.
- Pagination.
- Types for variables.
- Inferred types for iso literals via Typescript compiler plugin.
- Lint rules (namely, enforce that the exported const matches the name of the resolver field.)
- Non-batch mode compiler: watch mode and LSP.
- Expose compiler binary as `iso` or something.

## Cleanup backlog

- Typegen code is a mess
- JS code needs structure, etc.
- Separate `ServerFieldId` and `ResolverFieldId` might clean up a bunch of code.
- `HACK__merge_linked_fields` is indicative of the fact that merged linked fields should contain hashmaps of key => merged item, instead of vectors of merged items.
- Objects which do not have IDs should be merged into their parent object in the store.
- Remove the requirement that arguments and parameters have a terminal comma.
- Transform should be removed.

## Known bugs

- Typegen for non-eager non-component resolvers shows the eager type. There needs to be an additional type that is generated and threaded through.
- If a useLazyReference commits then receives new props, it does not make a new network request.
- if mutation primary field has a non-existent type, we panic, this should be an error
- incorrect spans for errors e.g. "Message: The id field on "Pet" must be "ID!"."

## Extended backlog

* Docs
* VSCode extension
* Compiler watch mode
* Fetch policies
* Garbage collection
* Preloaded queries
* Fetch/cache policies
* Granular re-renders
* Traditional refetch fields
* Defer/stream
* Subscriptions
* Interfaces/unions
* Entrypoints
* Field unwrapping syntax
* Pagination
* Refetch on missing data
* Compile to non-GraphQL
* Actually validate variables
* Error reporting (compiler)
* Persisted queries
* Suspense for mutations
* Suspense for refetch
* Strongly typed ID fields
* Custom normalizers
* Lazy normalization ASTs
* Fetching off of typed IDs
* Magic mutation/refetch field customizability
* Stateful resolvers
* Compiler executable for Mac/Windows/Linux
* Unit tests
* E2E tests
* Network request errors
* Proper missing field handlers
* Missing field handlers from schema definitions
* Store held in context
* Imperative store APIs
* Typesafe updaters
* Sample router integration
* Emit new field definitions for GraphiQL and other tools
* Field groups
* Lint rules to enforce e.g. export
* Guide for testing Isograph components
* Support non-globally unique IDs
* Iso lang code mods
* Iso lang syntax highlighting
* Iso lang auto-format
* Separate artifacts for fetching and reading fetchable resolvers
* Incremental compilation
* Saved state
* Support strict mode?
* Load resolvers iff needed
* Object literals as variables
* Server support for JSResource
* Injectable code for @component
* Isograph dev tools
* Experiment with context and JSX for @component
* Vue/Svelte/etc. integration
* Compile compiler to Wasm
* IR explorer
* Code sandbox example
* Non-fetchable fragments (???)
* Topological sort in compiler
* Validate no infinite recursion
* Statically prune inaccessible branches
* Bug fix: Not all files in one folder
* TypeScript errors in emitted artifacts
* Better repr. of nullable types in compiler
* Babel integration for iso literal values
* Typescript integration for type inference of iso literals
* Parallelize artifact gen
