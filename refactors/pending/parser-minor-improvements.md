# Parser minor improvements

Not in the grammar-stage order. Do not mix these into type-annotation-null.md or a combined-parse change.

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
