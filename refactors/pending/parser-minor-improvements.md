# Parser minor improvements

Not in the grammar-stage order. Do not mix these into type-annotation-null.md or parse-iso-literal-entry.md.

## `Slot<T, E>` is two independent `Option`s

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<T, E> {
    pub item: Option<WithSpan<T>>,
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`parse_one_chunk` produces three states: complete, complete-with-leftover, failed (whole chunk cloned into `extra_tokens`). `item: None, extra_tokens: None` is representable and never built. This is the bool-plus-spare-field case. It should be an enum with those three variants.

`ListTypeAnnotation` repeats the same pair (`inner: Option`, `extra_tokens: Option`) instead of being a `Slot<TypeAnnotation, UnparsedChunkItems>`. Empty `[]` fails the whole annotation (and therefore the host declaration). `[42]` succeeds as `List { inner: None, extra_tokens: Some(...) }`. Same shape, two recovery policies.

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
