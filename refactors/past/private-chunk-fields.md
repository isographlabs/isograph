# private-chunk-fields: `Chunk`'s fields go private

A prefactor for the parsing series. parsing-standards.md makes `Chunk`'s methods (`stream`, `contents_span`, `boundary_comma`) the only item access and the only boundary reads a parser has. That boundary should exist before the first parser is written, not land beside it: with the fields private, a parser reading `contents` or `trailing_separator` directly does not compile, so the standards' rule is enforced from the first line of grammar code. The methods themselves still land with their first callers (parse-entrypoint.md and later); this doc only removes the `pub`.

No reader outside chunk.rs exists today (verified by grep over every crate: no `.contents`, `.trailing_separator`, or struct-literal construction of `Chunk` elsewhere). The `ResolvePosition` derive expands at the definition site and the tests are a child module of chunk.rs, so both keep their access.

## The change

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    pub contents: NonEmptyVec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    contents: NonEmptyVec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

`ChunkedLevel`'s vec stays `pub`: the standards give a level no enforcement surface, and the level walkers iterate it plainly.

## Tests

No test changes: the fields' behavior is untouched, and every existing chunk.rs test lives inside the module. The compile itself is the assertion that no outside reader existed.

## Landing checklist

1. The two `pub` removals; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. parsing-standards.md's `Chunk` section states the fields are private as a fact, and its shipping section drops the privatization from the parse-entrypoint.md group.
3. Move this doc to refactors/past.
