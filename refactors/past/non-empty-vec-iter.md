# non-empty-vec-iter: a named iterator type

A prefactor for the parsing series. parsing-standards.md declares `ChunkStream`'s field as `SafePeekable<non_empty_vec::Iter<'a, WithSpan<ChunkContentItem>>>`, but `NonEmptyVec::iter` returns `impl Iterator<Item = &T>`, and a struct field cannot name an `impl` return type. This doc adds the named type. Behavior is unchanged; existing callers (`chunk.rs`, the crate's tests) compile as before.

## The change

Before:

```rust
// from crates/non_empty_vec/src/lib.rs
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        once(&self.first).chain(self.rest.iter())
    }
```

After:

```rust
// from crates/non_empty_vec/src/lib.rs
    pub fn iter(&self) -> Iter<'_, T> {
        Iter(once(&self.first).chain(self.rest.iter()))
    }
```

with the type, wrapping the same chain the method already built:

```rust
// from crates/non_empty_vec/src/lib.rs
/// The concrete iterator `iter` returns: `first`, then `rest` in order. It exists as a
/// named type so a struct field can hold it; an `impl` return type has no name.
pub struct Iter<'a, T>(Chain<Once<&'a T>, slice::Iter<'a, T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
```

The imports change from `std::iter::once` to `std::iter::{Chain, Once, once}` plus `std::slice`.

## Tests

The existing tests already assert the iteration order through `iter()`. One test is added for what the change is for, that the type is nameable:

```rust
// from crates/non_empty_vec/src/lib.rs (test module)
    #[test]
    fn iter_is_a_nameable_type() {
        let mut vec = NonEmptyVec::of(1);
        vec.push(2);
        let mut iter: Iter<'_, i32> = vec.iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), None);
    }
```

## Landing checklist

1. The type, the signature change, and the test; `cargo test -p non_empty_vec` and the clippy pre-commit hook pass.
2. parsing-standards.md's shipping section drops the `non_empty_vec::Iter` assignment from the parse-entrypoint.md group.
3. Move this doc to refactors/past.
