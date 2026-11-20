# Normalization AST: mount and unmount markers

Status: not actively worked. Not part of the first normalization AST change (`refactors/pending/graphql-protocol/normalization-ast.md`), whose node kinds are `Scalar`, `Linked`, and `Conditional` only.

The normalization AST will need to contain markers for things that must mount when a response is normalized and unmount later. This note lists what upstream Isograph does on normalization or around it that is this kind of thing, as candidates. It chooses no design.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`). Runtime paths are under `libs/isograph-react/src`; compiler paths are under `crates`.

## What upstream does today

Upstream's normalization AST has no markers. The normalizer and `check` mount nothing:

- `normalizeDataIntoRecord` (`core/cache.ts:155-220`) writes scalar fields, links, and inline fragments onto records, and adds each changed record to `encounteredIds` (`:211-218`). It starts no request and creates no subscription.
- `check` (`core/check.ts:43-215`) reads records and returns `EnoughData` or `MissingData`. Its only consequence is the caller's choice between retaining without a request and making a request (`core/makeNetworkRequest.ts:84-118`).
- A client pointer is dropped from the printed normalization AST (`artifact_content/src/normalization_ast_text.rs:73`, `MergedServerSelection::ClientObjectSelectable(_) => ""`).

Every mount below happens on a network request, on a read, or in a React hook. The candidates are the things a marker would move onto normalization, or would let normalization and garbage collection know about.

## Candidates

### 1. Retained queries and garbage collection

- Upstream: `makeNetworkRequest` and `retainQueryWithoutMakingNetworkRequest` build a `RetainedQuery { normalizationAst, variables, root }` and call `retainQuery` (`core/makeNetworkRequest.ts:464-493`, `core/garbageCollection.ts:55-63`). Dispose calls `unretainQuery`, which moves the query into `gcBuffer`; overflowing the buffer runs `garbageCollectEnvironment` (`core/makeNetworkRequest.ts:565-577`, `core/garbageCollection.ts:40-53`, `:65-86`). Collection walks each retained query's normalization AST from its root and deletes every record it does not reach (`core/garbageCollection.ts:88-119`, `:149-226`).
- Mounts: one retained query per request, when the request starts.
- Unmounts: on dispose, into the release buffer, then out of it when the buffer overflows.
- Marker: the AST already describes what a retained query keeps. A marker is needed where a nested query's result must stay reachable while the parent is retained (candidates 2 to 5). It would carry the nested query's normalization AST (or a reference to the artifact that holds it), the nested root's record link, and the variables of the nested query.

### 2. `@loadable` fields and their entrypoints

- Compiler: a loadably selected client field contributes only its refetch reader's `id` to the parent query; its own selection set becomes a separate merged selection map and query (`isograph_schema/src/create_merged_selection_set.rs:977-1007`, `was_ever_selected_loadably`). The parent's reader AST holds a `LoadablySelectedField` whose `entrypoint` is either the imported `__entrypoint` or an `EntrypointLoader` with a dynamic `import()` (`artifact_content/src/reader_ast.rs:549-598`). `artifact_content/src/generate_artifacts.rs:227-238` emits the refetch reader artifact with field map `id -> id`.
- Runtime: reading the field returns a function; calling it returns `[stableId, fetcher]` (`core/read.ts:279-451`). The stable id is `<typename>:<id>/<field name>/<sorted args>` (`:331-339`). The fetcher calls `maybeMakeNetworkRequest`, which retains (`:350-357`, `:405-423`). `useClientSideDefer` stores the pair in a `ParentCache` keyed by the stable id and disposes it with the component (`loadable-hooks/useClientSideDefer.ts:42-46`). `useImperativeLoadableField` calls the fetcher on demand and replaces the previous one (`loadable-hooks/useImperativeLoadableField.ts:37-47`). `useSkipLimitPagination` holds one disposable fragment reference per page (`loadable-hooks/useSkipLimitPagination.ts:57-62`, `:96-99`).
- Mounts: the child query's request and retained query, and with `lazyLoadArtifact` the artifact import, when user code calls the field.
- Unmounts: when the hook's owner disposes it, or when a later call replaces it.
- Marker: at the parent record's position, the child entrypoint (or its loader), the field name, the `queryArguments` and how they map from parent variables, and the field map from the parent record (`id`) into the child's variables. The stable id has to be derivable from the marker plus the normalized record.

### 3. Imperatively loaded fields, `__refetch`, and refetch query artifacts

- Compiler: every server object with an `id` field gets a `__refetch` client field whose variant is `ImperativelyLoadedField` (`graphql_network_protocol/src/process_type_system_definition.rs:81-105`, `:195-225`; `ImperativelyLoadedFieldVariant` at `isograph_schema/src/client_selectables.rs:73-85`). Selecting one records a refetch path (`isograph_schema/src/create_merged_selection_set.rs:1009-1019`, `:1055-1131`). Each path becomes `__refetch__<index>.ts`, a `RefetchQuery` artifact with its own query text and normalization AST (`artifact_content/src/imperatively_loaded_fields.rs:113-178`). The entrypoint imports them as `nestedRefetchQueries` with `allowedVariables` (`artifact_content/src/entrypoint_artifact.rs:358-395`).
- Runtime: `readImperativelyLoadedField` reads the refetch reader AST (the `id`), then returns a function that yields `[stableId, fetcher]` with the refetch query at `refetchQueryIndex` and filtered variables (`core/read.ts:1001-1065`).
- Mounts: the refetch request and its retained query, on the user's call.
- Unmounts: on dispose of the returned pair.
- Marker: the refetch query artifact (or its index), `allowedVariables`, the field map, and the inline fragment refinement when the field's parent is abstract (`PathToRefetchFieldInfo.wrap_refetch_field_with_inline_fragment`, `isograph_schema/src/create_merged_selection_set.rs:212-220`).

### 4. Follow-up requests

A follow-up request is a network request that normalizing a response starts, as opposed to a request that a read, a hook, or a user call starts (candidates 2 and 3). This is a separate candidate rather than a widening of candidate 3 because no user code holds its disposable; its lifetime has to come from the parent.

- Upstream: none exist. The stated plan is that an imperatively loaded field not selected `@loadable` "will be fetched as an immediate follow-up request" (`isograph_schema/src/field_loadability.rs:17-21`; the module is marked deprecated at `:1`). Today such a field still loads on the user's call (candidate 3).
- Starts: the normalizer, after it writes the record at the marker's position and the record holds every field the field map reads. A marker under a list starts one follow-up per item.
- Query and variables: the refetch query artifact for that path (`__refetch__<index>.ts`, a `RefetchQuery` with its own query text and normalization AST, `artifact_content/src/imperatively_loaded_fields.rs:113-178`). For `__refetch` the query fetches `node(id: $id)` from `Query`, with field map `id -> id` and top-level argument `id: ID!` (`graphql_network_protocol/src/process_type_system_definition.rs:193-231`). The variables are the field map applied to the just-normalized record, merged with the parent's variables filtered by `allowedVariables`, as `readImperativelyLoadedField` does on a call (`core/read.ts:1056`).
- Unmounts: when the parent's retained query is disposed, when garbage collection deletes the record, or when a later response overwrites the position so it no longer links to the record. Upstream has no request cancellation: `IsographNetworkFunction` is `(operation, variables) => Promise<any>` with no abort signal (`core/IsographEnvironment.ts:103-106`). Dispose (`core/makeNetworkRequest.ts:323-337`) unretains, and a response that arrives after dispose is not normalized (the `UndisposedIncomplete` guard at `:235`).
- Marker: the refetch query artifact reference (import, or index into `nestedRefetchQueries`), `allowedVariables`, the field map from record fields to the follow-up's variables, the field name for the stable id, and the inline fragment refinement when the parent is abstract (as candidate 3).
- `check`: a parent whose own data is present while a follow-up's data is missing must still start that follow-up on a cache hit (`retainQueryWithoutMakingNetworkRequest`), and that cache hit has no response to normalize. `check` would have to walk the marker into the follow-up's normalization AST at the stored record and report, per marker, which follow-ups to start, rather than one `EnoughData` or `MissingData` for the parent.
- Garbage collection: collection walks each retained query from its root (`core/garbageCollection.ts:88-119`). The follow-up is either its own `RetainedQuery` rooted at the root record (where `node(id:)` links to the record), owned and released by the parent, or no retained query at all, with the parent's walk descending through the marker into the follow-up's AST at the record. The second keeps the follow-up's records exactly as long as the parent reaches the record.

### 5. Client pointers

- Compiler: a client object selection records a refetch path through `node(id:)` (`isograph_schema/src/create_merged_selection_set.rs:1135-1233`) and is printed as nothing in the normalization AST (`artifact_content/src/normalization_ast_text.rs:73`).
- Runtime: `readClientPointerData` reads `id`, then returns `[stableId, fetcher]` that calls `maybeMakeNetworkRequest` with the refetch query (`core/read.ts:859-970`).
- Mounts, unmounts, and marker: as candidate 3, with the pointer's target type in place of the field's parent.

### 6. Fragment subscriptions

- Runtime: a component subscribes in an effect and unsubscribes on cleanup (`react/useRerenderOnChange.ts:21-37`, `core/subscribe.ts:15-40`). After normalizing, `makeNetworkRequest` calls `callSubscriptions` with `encounteredIds` (`core/makeNetworkRequest.ts:271`, `:542`), which re-reads each subscription whose recorded records overlap (`core/subscribe.ts:63-109`).
- Mounts: on read (the effect), never on normalization.
- Unmounts: on effect cleanup.
- Marker: none is needed for the subscriptions themselves; normalization already reports changed records. A marker is needed only if a subscription must be created for a record the response introduces (for example, the child of candidate 2 once it mounts on normalization).

### 7. Optimistic store layers

- Runtime: `makeOptimisticUpdate` adds an optimistic store layer and normalizes the optimistic response into it (`core/makeNetworkRequest.ts:495-544`). The real response, an error, or dispose reverts it (`:236-251`, `:309-316`, `:326-333`, `:546-563`). Garbage collection runs only on the base layer (`core/garbageCollection.ts:66-68`).
- Mounts: a store layer per request with `optimisticNetworkResponse`.
- Unmounts: when the request settles or is disposed.
- Marker: probably none; the lifetime is the request's, not a position in the response. Listed because it is the one mount upstream ties directly to normalizing.

### 8. Lazy normalization ASTs

- Compiler and runtime: `@lazyLoad(normalization: true)` emits a `NormalizationAstLoader` (`artifact_content/src/entrypoint_artifact.rs:433-449`), loaded when the request starts (`core/makeNetworkRequest.ts:480-483`). Garbage collection returns without collecting while any retained or buffered query's AST is not loaded (`core/garbageCollection.ts:70-83`).
- Mounts and unmounts: the module load, for the lifetime of the retained query.
- Marker: a marker that names a nested AST (candidates 1 to 5) has to say whether the nested AST is inline or loaded, because collection needs it.

## Open questions

- `refactors/pending/runtime/pico-runtime.md`, Follow-ups, records the user's design notes for candidates 4 and 5: constructors and destructors whose lifetime follows retain and release, a client pointer as a function plus a follow-up query, a pointer whose target is one of the records it read, handles, deduplication, cancellation, and deferral of subscription setup and garbage collection. Those notes are recorded for later and settle none of the questions below.
- Which candidates mount on normalization at all, and which only need to be visible to garbage collection while the mount stays on read or on a user call.
- Whether a marker names a nested artifact by import, by index into a list on the entrypoint (upstream's `nestedRefetchQueries`), or carries the nested AST inline.
- Who owns a mounted thing's lifetime: the parent's retained query, the record it is attached to, or a subscriber. What unmounts it when the parent record is overwritten by a later response that no longer links to it, and when garbage collection deletes the parent record.
- How a marker's stable id is computed so a mount on normalization and a mount on read of the same field and arguments are one mount.
- Whether markers are protocol-neutral core node kinds, like `Conditional`, or protocol data. `__refetch` and `node(id:)` are GraphQL's fetch plan.
- How `refactors/pending/runtime/pico-runtime.md` models a mount; that doc lists client pointers, `@loadable`, and refetch queries as later work (its Later section).
- Follow-up deduplication: two retained queries that normalize the same record, or a refetch that re-normalizes it, should start one follow-up. Upstream's imperative stable id is `<typename>:<link>__<field name>` with no arguments (`core/read.ts:1050`), unlike the loadable stable id (candidate 2). Whether a follow-up already in flight or complete suppresses a new one, and whether a changed field-map input (a different `id`) restarts it.
- Follow-up ordering relative to `callSubscriptions` (`core/makeNetworkRequest.ts:271`): whether the follow-up starts before or after subscribers re-read, and whether a reader of the field sees the in-flight follow-up as pending (suspend) or as missing data.
- Follow-up failure: whether it retries, which reader receives the error, and whether the parent stays complete. Whether a follow-up whose response contains another follow-up marker is bounded, and whether follow-ups from one response (one per list item) are batched into one request.
- Whether `check` must look through markers: a parent whose data is present but whose mounted child query is missing data is `EnoughData` for the parent today.
