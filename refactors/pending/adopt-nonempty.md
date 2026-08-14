# adopt-nonempty: chunk vecs use the `nonempty` crate

A prefactor for the parsing series, per the AGENTS.md standard to use an existing crate over our own. The `nonempty` crate (0.12.0) provides `NonEmpty<T> { pub head: T, pub tail: Vec<T> }`: non-empty by representation, the same first-plus-rest layout as our `non_empty_vec` crate, with `new`, `push`, `first`, `last`, `len`, `get`, `Index`, the derives the chunk types need (`Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord`), no mandatory dependencies, `no_std`, and a named `nonempty::Iter<'a, T>` returned by `iter()`, which is the type `ChunkStream`'s field holds (parsing-standards.md). This doc swaps the chunk types onto it and deletes `crates/non_empty_vec`.

Method mapping: `NonEmptyVec::of` becomes `NonEmpty::new`; `push`, `first`, `last`, `len`, `get`, and indexing keep their names, so the chunk.rs tests are unchanged.

## Dependencies

The workspace dependency table gains, in alphabetical order:

```toml
# from Cargo.toml, [workspace.dependencies]
nonempty = "0.12.0"
```

The parser's dependency swaps. Before:

```toml
# from crates/isograph_parser/Cargo.toml
non_empty_vec = { path = "../non_empty_vec" }
```

After:

```toml
# from crates/isograph_parser/Cargo.toml
nonempty = { workspace = true }
```

`crates/non_empty_vec` is deleted. The workspace's `members = ["./crates/*"]` glob picks up the removal, and nothing else references the crate.

## chunk.rs

The import. Before:

```rust
// from crates/isograph_parser/src/chunk.rs
use non_empty_vec::NonEmptyVec;
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
use nonempty::NonEmpty;
```

The two field types. Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    contents: NonEmptyVec<WithSpan<ChunkContentItem>>,
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkSeparator(pub NonEmptyVec<WithSpan<SeparatorToken>>);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkSeparator(pub NonEmpty<WithSpan<SeparatorToken>>);
```

The construction sites and the one annotated local. Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut contents = NonEmptyVec::of(WithSpan::new(first, first_location));
```

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators: Option<NonEmptyVec<WithSpan<SeparatorToken>>> = None;
```

```rust
// from crates/isograph_parser/src/chunk.rs
            None => separators = Some(NonEmptyVec::of(token)),
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut contents = NonEmpty::new(WithSpan::new(first, first_location));
```

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators: Option<NonEmpty<WithSpan<SeparatorToken>>> = None;
```

```rust
// from crates/isograph_parser/src/chunk.rs
            None => separators = Some(NonEmpty::new(token)),
```

The `first()`/`last()` reads (`separators.first().location`, `separators.last().location`) are unchanged: both methods exist on `NonEmpty` with the same signatures.

## The resolve_position derive

The macro recognizes container types by name, so `#[resolve_field] contents` requires the name change. Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        // Container types: Vec<T>, Option<T>, or NonEmptyVec<T>
        if (last_segment.ident == "Vec"
            || last_segment.ident == "Option"
            || last_segment.ident == "NonEmptyVec")
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        // Container types: Vec<T>, Option<T>, or NonEmpty<T>
        if (last_segment.ident == "Vec"
            || last_segment.ident == "Option"
            || last_segment.ident == "NonEmpty")
```

And the error message. Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        "Expected WithSpan<T>, WithLocation<T>, WithGenericLocation<T>, GraphQLTypeAnnotation, \
        Vec<T>, Option<T>, or NonEmptyVec<T> where T is a valid resolve field type",
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        "Expected WithSpan<T>, WithLocation<T>, WithGenericLocation<T>, GraphQLTypeAnnotation, \
        Vec<T>, Option<T>, or NonEmpty<T> where T is a valid resolve field type",
```

The generated iteration goes through `iter()`, which `NonEmpty` provides; no other macro change is needed.

## Tests

No test changes: every chunk.rs assertion reads `len`, indexing, `get`, or the fields through in-module access, all of which `NonEmpty` provides under the same names. The compile plus the existing suites are the verification.

## Landing checklist

1. The dependency swap, the chunk.rs and macro renames, and the crate deletion; `cargo test -p isograph_parser -p resolve_position_macros` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. parsing-standards.md and parse-entrypoint.md already assume this world.
