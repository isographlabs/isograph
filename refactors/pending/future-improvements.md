# Future improvements

These are not in the grammar-stage order. They are language-shape and leftover lexer grammar. Do not mix them into type-annotation-null.md, parse-iso-literal-entry.md, or parser-minor-improvements.md.

## Line break and comma are the same chunk separator

These tests pin the language. They are not accidents:

- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`. `a_directive_on_the_next_line_is_its_own_failed_selection`.
- `[Pet\n!]` does not attach the bang to `Pet`. `a_line_break_inside_a_list_type_does_not_attach_bang`.

Spaces do not split chunks. Newlines do.

## `#` comments

comments.md. A comment token is a language-shape change. It is not mixed into leftover-semantic-tokens.md leftover fill-in.

## Commented-out grammar in `token_kind.rs`

Spread, `Pipe`, and `PeriodPeriod` sit as comments in the token enum, plus `TODO don't skip comments and spaces`. `SingleLineComment` is comments.md (that regex is uncommented there, not deleted). The crate rule is that a comment must not describe what was not done.

```rust
// from crates/isograph_parser/src/token_kind.rs
    // #[token("..")]
    // PeriodPeriod,

    // #[token("|")]
    // Pipe,

    // #[token("...")]
    // Spread,
```

Delete those commented-out variants and the TODO on the skip regex. The commented-out `SingleLineComment` regex is comments.md, not this deletion. `token_kind.rs` uses `strum::Display`; there are no handwritten Display arms.

## Per-client index of editor buffer paths

`IsographDb::remove_editor_buffers_of_client` (lsp-open-files.md change 5) runs on every `ClientConnectionEvent::Gone`. It scans every key of `editor_buffer_map`, across every client, to find that client's keys. A per-client index of editor buffer paths, maintained by `set_editor_buffer` and `remove_editor_buffer`, would let disconnect cleanup visit only that client's paths. The index is state `handle` owns outside any memo; no memo reads it.

## `deleteRecord` drops source nodes that still have dependents

pico-runtime.md's `deleteRecord` removes a deleted record's source nodes from the store. A derived node that read one of them while it held `MISSING` keeps depending on the removed node, so a later write of that key goes to a new source node and never marks the reader, which returns `MissingData` from then on. The store's key-to-node map must keep resolving a key to the same node while the node has dependents, as pico's `id_for_key` does: `deleteRecord` writes `MISSING` to a present source and keeps any node with dependents, and the sweep removes it once its dependents are gone. Upstream does not have this bug: its store holds plain values and a subscription re-reads after every write (`libs/isograph-react/src/core/subscribe.ts`).

## Independently toggled pico-js features

runtime/pico-js-feature-flags.md. A Rust generator emits the pico-js engine once per configuration, so each contract check, such as a double release or a read of an unretained node, turns on or off by itself and costs nothing when off.

## Redo the pico-js library swaps on the final API

Once the pico-js API settles (global current node, retain objects with `onChange`, `startBatch`/`endBatch`, scheduled disposal), redo each swap of pico-js into another library's core and rerun that library's own tests and benchmarks: TanStack Store, the TC39 signal polyfill, Jotai, and Preact signals, plus nanostores, which was surveyed but not tried. The earlier swaps used `onDirty`, `pull`, `disposeDerivedNode`, and a per-computed tracking wrapper, which the new API removes. For Preact, measure whether its `Signal` and `Computed` classes can extend pico-js node classes so a Preact signal is the pico-js node. For the signal polyfill, the shim's `liveCount`, dependency snapshots, release cache, per-write version counter, and microtask release queue go away: watching is a retain with `onChange`, and scheduled disposal replaces the cache. Its `State` and `Computed` subclass the pico-js node classes with every field declared in the constructor. What stays is custom `equals` (boxed values) and introspection, which needs pico-js to export functions that walk a node's dependencies and dependents.

## Trace-replay benchmark for pico-js

Once the Isograph runtime runs on pico-js, record every pico-js call a demo app (`demos/github-demo`) makes during scripted sessions (navigate, paginate, edit, refetch) and replay the traces against each engine variant and competitor. Per-scenario results from real traces replace the synthetic app benchmark as the basis for engine decisions; suite totals are not used, because summing tests weights them by their run time.

## Reducers over event streams in pico-js

Consider whether pico-js can handle a reducer over an event stream: a value that folds a sequence of events, `state = reduce(state, event)`, rather than a pure function of the current sources. The questions to answer:

- How is an event stream represented? One option is a source that holds the latest event. Another is an append-only log source.
- How does a derived node fold events? A computation receives only the current values of the nodes it reads and its `previous` value. Folding with `previous` is correct only if every event triggers exactly one recompute. Lazy evaluation and batching do not guarantee that. Two writes in one batch, or two writes with no read between them, drop an event: the computation runs once and sees only the latest event.
- Does the fold belong outside the graph? One option is a source that the caller updates with `writeSource(state, reduce(current, event))`, so each event is applied exactly once and derived nodes read the result. That moves the reducer out of pico-js and leaves the graph a pure function of its sources.
- What do the contracts and the eager optimization imply? A fold inside the graph depends on how many times a computation runs, and the contracts do not promise a count. The eager optimization changes when a computation runs, so it changes which events a fold-by-`previous` would see.

## Automatic eager derived nodes in pico-js

An experiment outside the core library: a wrapper that makes a derived node eager by itself. A derived node is eager when it holds a retain whose `onChange` does nothing. The flush then recomputes the node right after updates, which saves work when the node is likely to return its previous value and many nodes depend on it.

Per node, the wrapper measures:

- how often a recompute returns the previous value, which is the cutoff rate;
- the number of dependents;
- how many nodes an update reaches past this node;
- optionally the cost of the computation.

The candidate heuristic makes a node eager when its cutoff rate is high and many nodes depend on it, which is the projection pattern: a rarely changing value derived from a frequently written source and read by many effects. The wrapper makes the node lazy again, by releasing the no-op retain, when that stops holding.

Known costs:

- Timing each computation with `performance.now()` costs tens of nanoseconds per run, so the wrapper has to sample the timing instead of timing every run.
- The counters add fields to each node or a side table keyed by node.
- Toggling eagerness churns retains. A node that crosses the threshold back and forth pays a retain and a release each time, so the heuristic needs hysteresis or a minimum number of samples between toggles.

This is a wrapper or an experiment branch. It does not go into the core library. The S16 projection scenario and `cutoffWrite` are the benchmarks that judge it.

Open question: whether the stale-read fix that ships (A2 with A5's skip bit, or a height variant) changes the payoff. Under A2 without A5, an eager node with many dependents costs O(dependents) per update, which may cancel the saving for exactly the nodes the heuristic selects.

## pico-js: skip marking past nodes with callbacks that read only sources

The case is a node with a change callback whose latest run read only sources. A projection of a record's source onto one field is the typical example. A source is always current, so such a node's recompute can never read a stale value. The engine identifies such a node for free with `firstDerivedDependency === undefined`. That needs no declaration and no requirement that the node's dependencies stay the same.

The mechanism:

- Marking stops at such a node.
- The flush refreshes every such queued node first, before any other queued node.
- Marking past one happens only from its own change callback, when its value changed.
- That removes A5's walk past the projection.

The hazard is a source-only node that, during its own refresh, switches branch and links its first derived edge, to a node D that sits past another source-only node not yet refreshed. The engine detects it at that link. Before the read returns, it marks past every source-only queued node not yet refreshed, using A2's beyond marking, so the read of D verifies and refreshes on demand. The node then stops being source-only. Under the contract (no derived reads in batches, `onChange` only reads), nothing else can read past such a node before the flush refreshes it.

This saves A5's one extra marking pass per change of a projection's value. The S16c scenario (the projection changes on every event) and the pure-engine projection microbenchmark judge it. Build it only if A5 is measurably above the base there.

It needs a written correctness proof before it lands, like the one for A5s in the scratchpad proofs.

## pico-js: a build-time transform that hoists dependency declarations

The contract is that every derived read is declared before it happens. A9 (`createDerivedNode` with `mayRead`, `declareDependencies`, and the undeclared-read fallback) is the engine side. This entry removes the declaration work from the people who write computations.

Users write plain `readDerived` calls and no declarations. A Babel or TypeScript transform, not a lint autofix, analyzes each computation and hoists each read's target as early as its inputs allow:

- A target that depends on no value read in the computation goes into the creation-time `mayRead` array. It costs nothing per run.
- A target that depends on data gets a `declareDependencies(callingNode, [targets])` call inserted right after the read that produces that data.

The source stays free of duplicated declarations. Only the built output has them.

A lint rule flags the reads the transform cannot analyze, such as reads through opaque helpers, so the author restructures them or declares them by hand.

The engine keeps the O(1) check on newly linked edges and the fallback marking from A9 as a safety net. A read the transform missed costs a slowdown, not a stale value.

Tokens returned by a declaration, where each token is an index into the declared list, are an alternative that could be cheap. A transform that writes the declarations makes them unnecessary.

Prerequisites:

- A9 lands.
- A9's timing shows that declaring pays off on S16 and S16d.

Isograph's existing Babel plugin and compiler are the place to host the transform.
