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
