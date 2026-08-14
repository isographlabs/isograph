# chunk-contents-nonempty: `Chunk::contents` is a `NonEmpty`

A comma no item precedes (`,,`, `{,}`, a leading `,`) is chunking's `CommaWithoutItem`, not a chunk. `absorb_chunk` therefore always starts a chunk from a content item. `Chunk::contents` is a `nonempty::NonEmpty`, so an empty contents vec is unrepresentable.

A trailing boundary, when present, likewise always holds at least one separator token. `ChunkSeparator` is the same `NonEmpty`.

A level can be empty (`{}`). `ChunkedLevel` stays a `Vec`.

## The crate

Workspace dependency, alphabetical:

```toml
# from Cargo.toml, [workspace.dependencies]
nonempty = "0.12.0"
```

```toml
# from crates/isograph_parser/Cargo.toml
nonempty = { workspace = true }
```

`nonempty` 0.12 stores the invariant in the layout: `NonEmpty<T> { pub head: T, pub tail: Vec<T> }`. `new` builds from the first element, `push` appends, `first` and `last` return `&T`, `len` / `get` / `Index<usize>` work as on a vec, `iter()` returns a named `nonempty::Iter<'a, T>` (the type `ChunkStream`'s field holds). The derives the chunk types need (`Clone`, `Debug`, `PartialEq`, `Eq`) come with `T`. No mandatory dependencies.

## Change 1: the derive accepts `NonEmpty`

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        if (last_segment.ident == "Vec" || last_segment.ident == "Option")
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        if (last_segment.ident == "Vec"
            || last_segment.ident == "Option"
            || last_segment.ident == "NonEmpty")
```

The error message's type list gains `NonEmpty<T>`. The emission already iterates via `.iter()`, which `NonEmpty` provides. No behavior change at any existing site.

## Change 2: the field types

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkSeparator(pub Vec<WithSpan<SeparatorToken>>);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
use nonempty::NonEmpty;

pub struct Chunk {
    #[resolve_field]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkSeparator(pub NonEmpty<WithSpan<SeparatorToken>>);
```

## Change 3: construction

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let first_location = peek.commit().location;
    let mut span = first_location;
    let mut contents = vec![WithSpan::new(first, first_location)];
    while let Some(peek) = items.peek() {
        let Some(content_item) = as_content(peek.view(), errors) else {
            break;
        };
        let item = peek.commit();
        span = Span::join(span, item.location);
        contents.push(WithSpan::new(content_item, item.location));
    }

    let mut separators: Vec<WithSpan<SeparatorToken>> = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators
                .iter()
                .any(|token| token.item == SeparatorToken::Comma)
        {
            break;
        }
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }

    let location = separators
        .first()
        .zip(separators.last())
        .map(|(first, last)| Span::join(first.location, last.location));
    let trailing_separator =
        location.map(|location| WithSpan::new(ChunkSeparator(separators), location));
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let first_location = peek.commit().location;
    let mut span = first_location;
    let mut contents = NonEmpty::new(WithSpan::new(first, first_location));
    while let Some(peek) = items.peek() {
        let Some(content_item) = as_content(peek.view(), errors) else {
            break;
        };
        let item = peek.commit();
        span = Span::join(span, item.location);
        contents.push(WithSpan::new(content_item, item.location));
    }

    let mut separators: Option<NonEmpty<WithSpan<SeparatorToken>>> = None;
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators.as_ref().is_some_and(|absorbed| {
                absorbed
                    .iter()
                    .any(|token| token.item == SeparatorToken::Comma)
            })
        {
            break;
        }
        let item = peek.commit();
        let token = WithSpan::new(separator, item.location);
        match &mut separators {
            None => separators = Some(NonEmpty::new(token)),
            Some(absorbed) => absorbed.push(token),
        }
    }

    let trailing_separator = separators.map(|separators| {
        let location = Span::join(separators.first().location, separators.last().location);
        WithSpan::new(ChunkSeparator(separators), location)
    });
```

`contents_span` and `first_item` (parsing-standards.md) call `first` / `last` on `NonEmpty` and need no `Option` unwrap. `Chunk::stream` passes `&self.contents` into `ChunkStream::new`, whose field is `SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>`.

## Tests

Existing comma-without-item tests already cover `{,}`, `,,`, and a leading comma: those inputs produce `CommaWithoutItem` and no empty chunk. `contents.len()` and `contents[index]` keep compiling (`NonEmpty` has both). The test helper that joined contents with `reduce` + `expect` becomes a total join:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
    fn chunk_span(chunk: &Chunk) -> Span {
        let contents = Span::join(
            chunk.contents.first().location,
            chunk.contents.last().location,
        );
        match &chunk.trailing_separator {
            Some(separator) => Span::join(contents, separator.location),
            None => contents,
        }
    }
```

## Landing checklist

1. The workspace and parser dependency lines; the derive accepts `NonEmpty`; the two field types and the construction site; the `chunk_span` helper. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
