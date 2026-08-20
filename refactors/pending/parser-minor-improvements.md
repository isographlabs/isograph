# Parser minor improvements

Not in the grammar-stage order. Do not mix these into type-annotation-null.md or parse-iso-literal-entry.md.

## `Slot<T, E>` is two independent `Option`s

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<T, E> {
    pub item: Option<WithSpan<T>>,
    pub extra: Option<WithSpan<E>>,
}
```

`parse_one_chunk` produces three states: complete, complete-with-leftover, failed (unread remainder in extra; leftover-in-extra.md). `item: None, extra: None` is representable and never built. This is the bool-plus-spare-field case. It should be an enum with those three variants.

`ListTypeAnnotation` repeats the same pair (`inner: Option`, `extra: Option`) instead of being a `Slot<TypeAnnotation, UnparsedChunkItems>`. Empty `[]` fails the whole annotation (and therefore the host declaration). `[42]` succeeds as `List { inner: None, extra: Some(...) }`. Same shape, two recovery policies.

## `parse_singleton` assumes a non-empty level

```rust
// from crates/isograph_parser/src/chunk.rs
        &level.item.0[0],
        level.item.0[0].item.stream(text, tokens, errors),
```

`ChunkedLevel` is a `Vec`. Empty is legal (whitespace-only literals). The two production call sites check `len() == 0` first. The type does not. A `NonEmpty` level, or a different type for "level that has a first chunk," would make the index impossible.

## `EndOfFile` is never emitted

`tokenize` stops at the last real token. End of input at parse time is `Found::EndOfChunk`. Delete `EndOfFile` from `IsographLangTokenKind` and `NonBracketTokenKind`, and the `From` / `Display` / `SplitToken` arms.

## `Expectation::Description` and `Expectation::SelectionSet`

Never passed to `cursor.expected`. Descriptions and selection sets are optional. Delete the two variants. Display tests of `OneOf` use `Keyword` and `Selection` (`Selection` is used in production).

## `Expectation` is not an `Error`

`AstError` is the AST-stage error. `Expectation` is a fragment of `ExpectedFound`. Pipeline `ParseError` wraps `AstError`.

Before:

```rust
// from crates/isograph_parser/src/parse_error.rs
impl std::error::Error for Expectation {}
```

After: that impl is gone. `AstError` stays `thiserror`. `Found` stays `strum::Display`. `Expectation` keeps its handwritten `Display`: `OneOf` is `write_one_of`, and `Separator` uses `kind.closing()`, neither of which is a per-variant strum string.

## Keyword-as-identifier is copy-pasted

`entrypoint` / `field`, `to`, and `true` / `false` / `null` are all "require Identifier, then match the source slice." `consume_to_target` peeks, compares to `"to"`, then `require_token` with `Keyword`. `parse_boolean_or_null` records `BooleanOrNull` before checking the word, so `a: yes` highlights `yes` as boolean/null and then errors. A `consume_keyword` that records only on match would remove the duplication and the bad highlight.

## `parse_each_chunk` tests do not use a shared harness

`consume_selection_set`, `consume_argument_list`, `consume_variable_declaration_list`, object interiors, and list interiors all go through `parse_each_chunk`. That is the right extraction.

`span_of`, `parsed_items`, and the dummy parent-cursor setup are duplicated in `arguments.rs` and `selections.rs` tests. `crates/tests` is an empty crate.
