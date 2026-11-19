# Runtime: directions beyond Yes, No, and IfNecessary

Status: not actively worked. Nothing in v1 implements any of this. v1 has `check` only (`refactors/pending/runtime/store.md`); the later network layer reads it to decide upstream's `shouldFetch` (`'Yes' | 'No' | 'IfNecessary'`, `isograph/libs/isograph-react/src/core/check.ts:13-16`, decided in `makeNetworkRequest.ts:67-118`).

This doc lists directions. It chooses none. Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`), under `libs/isograph-react/src/core` unless stated.

## Staleness and time-to-live

A policy "fetch if the data is older than N seconds" needs a write time per record or per field. The v1 store holds values only (entity name, id, store key, value), so this adds a second map beside the values or a timestamp inside each stored value. `normalize` stays pure only if the caller passes the time in (`normalize(store, ast, response, variables, root, now)`) and `check` takes `now` too; reading a clock inside either makes them impure and makes tests depend on time. Per-field times cost one number per stored value; per-record times are cheaper and mark every field of a record stale together, including fields another query wrote later. Upstream has no write time.

## Render store data and refetch

"Render what the store has, and fetch anyway" differs from upstream `'Yes'` with `suspendIfInFlight: false` (`read.ts:973-982`). Upstream `'Yes'` always sends the request (`makeNetworkRequest.ts:68-75`); `suspendIfInFlight: false` only stops a read from suspending on the in-flight request, and the read still suspends on `MissingData` (`read.ts:92-122`). A policy that renders stale data while the request is in flight needs `check` (EnoughData) before the request is sent, and a reader that returns data while a request is pending. The cost is a second state for every read (data shown, refresh pending) that the caller has to surface.

## A policy as a function of the check result

`MissingData` names the record that lacks data (`check.ts:34-41`). A caller could fetch only that record with a `Node` refetch of the selections under it, instead of the whole query, when the record has an identity. That needs the normalization AST at that record (a sub-AST, so `check` would return the selection set it was walking, not only the record) and a query per sub-AST, which the compiler does not generate; today only whole entrypoints have query text. It costs one compiled query per position that can be refetched, or a runtime query printer. A `MissingData` record with a derived id (no identity) cannot be refetched by `node(id:)`.

## Partial rendering instead of suspending on the first missing field

Upstream's read stops at the first missing field and throws a promise (`read.ts:92-122`). Rendering what is present and marking each missing field needs a reader result type with a per-field missing state, and `check` that reports every missing position rather than the first. Upstream's docs name this as the future that would make `'No'` useful (`docs-website/docs/conditional-fetching.md:36`). The cost is that every generated reader type gains a missing case, or a missing field is `undefined` in data the resolver receives.

## Per-selectable policies in iso literals

A directive on a selection (`pet(id: $id) @fetch(policy: IfNecessary)`) or on a field declaration would give one query several policies. `check` would then walk the AST per policy region, so the AST has to carry the policy on each node. A policy that differs inside one query does not change what one request fetches; it only changes whether the request is sent, so the useful combinations reduce to "send if any region with `IfNecessary` is missing", which costs AST bytes for a decision the caller makes once.

## Network only, without writing to the store

Fetch and hand the response to the reader without normalizing it. The reader then reads the response by response key, not the store by store key, so this needs a second read path over the response shape. Nothing is retained, and garbage collection is not involved. Upstream has no such mode: every response is normalized (`makeNetworkRequest.ts:233-271`).

## Store only

Never fetch, and read whatever the store has. Upstream's `'No'` is this when the store already has the data (`makeNetworkRequest.ts:77-83`); with missing data the read suspends forever, waiting for a change nothing sends (`read.ts:122`). A store-only mode is useful only with partial rendering (above), or with a caller that writes the store by other means (another query, a subscription, a local write).
