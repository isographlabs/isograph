# The close bracket's parent

Requires generic-resolution.md. A close bracket sits in one of two positions, and its parent enum says which, so genuine-versus-stray reads off the path as a variant; at a stage whose stray slot is `Infallible`, the `Stray` payload is uninhabited. The macro is untouched: the leaf emissions construct through `From`, and this doc only retargets what the conversions produce.

## The change

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The two positions a close bracket can sit in. At a stage whose stray slot is
/// `Infallible`, the `Stray` payload is uninhabited and every close is a `Closing`.
#[derive(Debug)]
pub enum CloseBracketParent<'a, TContents: TreeContents = BracketsMatched> {
    /// The group whose closing this is.
    Closing(Box<BracketedPath<'a, TContents>>),
    /// A stray item: the slot value it came from, at the position it sits in.
    Stray(StrayClosePath<'a, TContents>),
}

pub type StrayClosePath<'a, TContents = BracketsMatched> = PositionResolutionPath<
    &'a <TContents as TreeContents>::StrayClose,
    BracketItemParent<'a, TContents>,
>;
```

`CloseBracketPath` reparents onto it.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a, TContents>>;
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, CloseBracketParent<'a, TContents>>;
```

The two conversions the emissions construct through: a closing's parent converts from the group's path, and a stray's whole leaf path converts from the slot-typed path, borrowing the token out of the slot and keeping the slot path as the parent:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<'a, TContents: TreeContents> From<BracketedPath<'a, TContents>>
    for CloseBracketParent<'a, TContents>
{
    fn from(path: BracketedPath<'a, TContents>) -> Self {
        CloseBracketParent::Closing(Box::new(path))
    }
}

impl<'a, TContents: TreeContents> From<StrayClosePath<'a, TContents>>
    for CloseBracketPath<'a, TContents>
{
    fn from(path: StrayClosePath<'a, TContents>) -> Self {
        let token = std::borrow::Borrow::borrow(path.inner);
        CloseBracketPath {
            inner: token,
            parent: CloseBracketParent::Stray(path),
        }
    }
}
```

The borrow is what connects the slot to the token at every stage, so the slot carries the bound.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined.
    type StrayClose: fmt::Debug + PartialEq + Eq;
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined. Every carrier borrows to the token, so a stray answers
    /// the concrete `CloseBracket` leaf at every stage.
    type StrayClose: fmt::Debug + PartialEq + Eq + std::borrow::Borrow<CloseBracket>;
```

std's blanket `Borrow<T> for T` covers every stage whose slot is `CloseBracket`. A stage whose slot is `Infallible` writes the vacuous impl — legal under the orphan rule because `CloseBracket` is local — and its consumers, on stable, write the one vacuous arm the exhaustiveness checker still demands for the uninhabited variant:

```rust
// from a future refined stage
impl std::borrow::Borrow<CloseBracket> for Infallible {
    fn borrow(&self) -> &CloseBracket {
        match *self {}
    }
}

match close.parent {
    CloseBracketParent::Closing(group) => ...,
    CloseBracketParent::Stray(path) => match *path.inner {},
}
```

The `Stray` arm disappears entirely if `never_patterns` lands on stable; until then it is the compiler-checked residue of the impossibility.

The generated leaf emissions are unchanged: the closing field's `own_path.into()` lands in `Closing` through the group-path `From`, and the stray arm's whole-path conversion lands the token in the leaf and the slot-typed path in `Stray`. The `Borrow` bound generic-resolution.md puts on the slot is what the stray conversion borrows through.

Test updates in the existing suite: the stray tests match `CloseBracketParent::Stray` and read the position from its payload's parent; `the_close_pairs_with_the_nearest_open` matches `CloseBracketParent::Closing` and asserts the balanced group in its payload.

## Landing checklist

1. The change; `cargo test` green.
2. Move this doc to refactors/past.
