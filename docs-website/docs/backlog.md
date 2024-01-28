# Backlog

## Top QOL priorities

- publish isograph.dev (via gh-pages?)
- handle schema keyword in GraphQL
- handle renamed query object (e.g. in Star Wars schema)
- iso overload in files

## V2 release

- components and "realized" resolvers, as well as ways to invalidate them
  - they could also be lazily calculated
  - may require garbage collection
- cleanup types
- error handling
- validate no unknown directives left over
- exposeAs as param
  - use serde for this?
- Handle unions etc. correctly
- Special fields (or syntax?) for type casts (i.e. type refinement)

## Feature backlog

- garbage collection
- granular re-rendering
- fetch policies
- Unwraps (i.e. `!`) exist in the syntax, but are unused
  - consider whether it is truly the case that there always is a linear way to unwrap a given field, or whether we should unify this with "execute this on the server" etc.
- Resolvers are re-calculated every time. They should be cached in the store.
- The store, etc. should be stored in context.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can then be selected into.
- Stateful resolvers?
  - This could be thought of as "realized" resolvers, which is to say there is overlap with better DevEx for components
- Subscriptions are not supported
- Defer, etc.
- Pagination.
- Types for variables
- Inferred types for iso params w/iso overload
- typed IDs
- special fetch fields
- consider resolvers that return functions only read data when called, i.e. do not eagerly read. Consider whether this can be achieved with omitting a !, i.e. foo_resolver! returns TReadFromStore, foo_resolver returns a `ReadDataResult TReadFromStore`

## Cleanup backlog

- Typegen code is a mess
- JS code needs structure, etc.
- `HACK__merge_linked_fields` is indicative of the fact that merged linked fields should contain hashmaps of key => merged item, instead of vectors of merged items.
- Objects which do not have IDs should be merged into their parent object in the store.
  - or weak types are scalars
- Remove the requirement that arguments and parameters have a terminal comma.
- IsographSchemaObject, etc. should not contain name: `WithLocation<...>`, but instead, be stored `WithLocation T`, and WithLocation should **not** have a span.
- There should be a cleaner separation between GraphQL and Isograph. In particular, we should load the GraphQL schema, but turn it into Isograph concepts, and only deal with Isograph concepts.

## Known bugs

- If a useLazyReference commits then receives new props, it does not make a new network request.
- if mutation primary field has a non-existent type, we panic, this should be an error
  - this is because we add the fields before we call Schema::validate_and_construct, where the error would naturally be found.
- incorrect spans for errors e.g. "Message: The id field on "Pet" must be "ID!"."
- error parsing config should not panic, but be a diagnostic

## Extended backlog

- Docs
- VSCode extension
- Fetch policies
- Garbage collection
- Preloaded queries
- Fetch/cache policies
- Granular re-renders
- Traditional refetch fields
- Defer/stream
- Subscriptions
- Interfaces/unions
- Field unwrapping syntax
- Pagination
- Refetch on missing data
- Compile to non-GraphQL
- Actually validate variables
- Persisted queries
- Suspense/response for mutations/refetch
  - this probably means we need mutations to be part of a stateful component
- Strongly typed ID fields
- Custom normalizers
  - Garbage collection for custom normalizers by type
  - other "by type" things?
- Lazy normalization ASTs
- Fetching off of typed IDs
- Stateful resolvers
- Compiler executable for ~Mac~/Windows/Linux
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
- Topological sort in compiler
- Validate no infinite recursion
- Statically prune inaccessible branches
- TypeScript errors in emitted artifacts
- Better repr. of nullable types in compiler
- Babel integration for iso literal values
- Typescript integration for type inference of iso literals
- Parallelize artifact gen
- Rationalize WithSpan vs WithLocation
- Display multiple errors, parse etc. in parallel
  - Multiple errors when parsing the schema (is this possible??)
- Do not look in artifact_directory, if project_root contains artifact_directory
- Do not require that the exported name of an iso is anything in particular.
  - Can we make the transform add an export if none is found?? Probably!
- SWC plugin
- plugin options to point to config
