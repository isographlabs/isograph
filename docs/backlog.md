# Backlog

## Feature backlog

- There are currently no normalization ASTs, which are necessary to:
  - Determine whether to go to network or fulfill a query from store
  - Power garbage collection
  - Making advanced transformations to the returned data
- Unwraps (i.e. `!`) exist in the syntax, but are unused
- Isograph only parses a subset of GraphQL schema syntax
- Resolvers are re-calculated every time. They should be cached in the store.
- The store, etc. should be stored in context.
- Resolvers return opaque objects and cannot be selected into. They should be extended to also allow the return of IDs, which can they be selected into.
- Stateful resolvers?
- Mutations and subscriptions are not supported. In particular, mutations/subscriptions should be made available from within queries.
- ID field is not automatically selected. (What field to select should be inferred from a directive on the schema.)
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

## Priorities for GraphQL conf

- Mutations
- Errors displayed in console
