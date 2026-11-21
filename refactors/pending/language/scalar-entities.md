# Scalar entities and the selection shape rule

Status: parked note. Not planned and not under discussion for implementation. This doc records ideas from a discussion so they are not lost. Nothing here is a decision.

## Selection shape rule

A selection without a selection set returns the field's value at its own type. A selection with a selection set returns an object of the selected fields.

- `name` returns the string.
- `best_friend` returns `{}`, or `null` when the field is null. The link never reaches user code.
- `best_friend` and `best_friend {}` are therefore the same selection.
- `name {}` returns `{}`, or `null` when the field is null.
- `best_friends` returns `[{}, null, ...]`, one element per list item.

## Scalars are entities

In the schema, every scalar is an entity. `String` is an entity, so `name { toUpperCase }` selects a client field `toUpperCase` on `String`. The resolver of `toUpperCase` receives the raw string.

## User-defined entities

A source file could declare an entity whose value is a TypeScript type. One candidate syntax is `iso<{ foo: string }>(\`entity FooRef\`)`, which declares the entity `FooRef` with value type `{ foo: string }`. Then `field FooRef.Bar { __self }` receives `{ foo: string }` as `__self`. The syntax is undecided.

## Runtime consequences

These use the terms of `refactors/pending/runtime/pico-runtime.md`.

Every case of the shape rule reads the field's source node. A null to non-null transition, a non-null to null transition, and a link move each mark the readers of that source node.

Under bare `best_friend`, the reader walk creates a fresh `{}` on each read. Two such objects compare equal under the structural rule (`resolverResultsEqual`), so a link move under bare `best_friend` cuts off at the reader and its dependents are not recomputed.

A client field on a scalar needs a node identity. Two options came up:

- (a) A derived node in the record's container, keyed by the path (for example `name/toUpperCase`). The record owns the node, and the node depends on the field's source node. Deleting the record disposes it, and garbage collection treats it like any other derived node the record owns.
- (b) A derived node keyed by the scalar value, shared across records. Every distinct value creates a node, so the set is unbounded, and no record owns the node, so garbage collection has no owner to collect it with.

Option (a) is held in the discussion. It is not decided.

## Query text

GraphQL requires a selection set on a field whose type is an object, interface, or union. Bare `best_friend` in an Isograph literal therefore still requests a field in the generated query (for example `id` or `__typename`), so that the response distinguishes null from non-null and the normalizer can store the link.

## Upstream behavior

Upstream isograph rejects both shapes this rule accepts. The upstream parser classifies a selection by the presence of a selection set: with one it is `SelectionType::Object`, without one it is `SelectionType::Scalar` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:450` to `:476`). Validation then rejects a scalar selection of a server field whose target is an object (`../isograph/crates/isograph_schema/src/validate_selection_sets.rs:140`), a scalar selection of a client pointer (`:157`), an object selection of a server field whose target is a scalar (`:293`), and an object selection of a client field (`:309`). A client field on a scalar parent is rejected with "Invalid parent type. `X` is a scalar." (`../isograph/crates/isograph_schema/src/process_client_field_declaration.rs:72`), and a client pointer to a scalar with "Invalid client pointer target type." (`:135`).

Adopting this rule deviates from upstream. The reason is that a selection's result type follows from the field's type alone, and scalars gain client fields.

## i2 today

i2's parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`crates/isograph_parser/src/selections.rs:89`, called at `:130`) for any selection.

i2's validation rejects both, matching upstream. `target_errors` (`crates/isograph_compiler/src/validation.rs:559`) reports `ValidationError::SelectionSetOnLeaf` for a selection set on a leaf target (`:583`, message "`X` returns a leaf and takes no selection set." at `:40`) and `ValidationError::MissingSelectionSet` for a composite target without one (`:587`, message at `:42`). `parent_error` (`:314`) reports `InvalidEntityError::LeafParent` for a `field` declaration on a leaf parent (`:322`, message at `:75`), and `pointer_target_error` reports `InvalidEntityError::LeafTarget` for a pointer to a leaf (`:343`, message at `:85`).
