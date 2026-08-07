# The close bracket's parent

A close bracket sits in one of two positions — a group's real closing, or a stray item — and its parent enum says which, so genuine-versus-stray reads off the path as a variant. The two variants are constructed at type-disjoint sites, so neither flow can produce the other's variant, and the macro is untouched.

## The change

In `matched_brackets.rs`. The parent enum, with the genuine variant named `Bracketed` because the closing field's emission names parent variants after the containing struct:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The two positions a close bracket can sit in.
#[derive(Debug)]
pub enum CloseBracketParent<'a, TContents: TreeContents = BracketsMatched> {
    /// The real closing of this group.
    Bracketed(Box<BracketedPath<'a, TContents>>),
    /// A stray item, at the position it sits in.
    Stray(BracketItemParent<'a, TContents>),
}
```

`CloseBracketPath` reparents onto it, and `CloseBracket`'s derive follows.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a, TContents>>;

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub type CloseBracketPath<'a, TContents = BracketsMatched> =
    PositionResolutionPath<&'a CloseBracket, CloseBracketParent<'a, TContents>>;

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = CloseBracketParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);
```

The closing flow needs nothing else: the closing field's emission inside `Bracketed`'s derived resolve constructs `<CloseBracket as ResolvePosition>::Parent::Bracketed(self.path(parent).into())`, which is `CloseBracketParent::Bracketed` holding the boxed group path, and `CloseBracket`'s derived fallback builds the reparented `CloseBracketPath` through `path`'s existing `From` hook.

The stray flow gets its own type. The stray slot wraps the token, and its resolve — the one hand-written `ResolvePosition` impl in the crate, because the derive can neither name a leaf variant differing from the struct's name nor inject the `Stray` wrapper — parents the token with its position:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// A close bracket sitting where an item sits: no open of its kind was waiting for it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StrayClose(pub CloseBracket);

impl ResolvePosition for StrayClose {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        _position: Span,
    ) -> Self::ResolvedNode<'a> {
        ResolvedBracketNode::CloseBracket(PositionResolutionPath {
            inner: &self.0,
            parent: CloseBracketParent::Stray(parent),
        })
    }
}
```

The slot and its uses follow.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl TreeContents for BracketsMatched {
    type Inner = Inner;
    type StrayClose = CloseBracket;
}
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl TreeContents for BracketsMatched {
    type Inner = Inner;
    type StrayClose = StrayClose;
}
```

The matcher's stray construction site:

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
                items.push(WithSpan::new(
                    BracketItem::StrayClose(CloseBracket(kind)),
                    token.location,
                ));
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
                items.push(WithSpan::new(
                    BracketItem::StrayClose(StrayClose(CloseBracket(kind))),
                    token.location,
                ));
```

The error collector, whose bound becomes `TreeContents<StrayClose = StrayClose>`:

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
            BracketItem::StrayClose(stray) => {
                errors.push(BracketError::UnexpectedClose(WithSpan::new(
                    *stray,
                    item.location,
                )));
            }
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
            BracketItem::StrayClose(stray) => {
                errors.push(BracketError::UnexpectedClose(WithSpan::new(
                    stray.0,
                    item.location,
                )));
            }
```

chunking.md's stage follows the slot: `Chunked`'s `StrayClose` is the same `StrayClose`, its `map` carries the value through unchanged, and its stray test matches `BracketItem::StrayClose(StrayClose(CloseBracket(Parenthesis)))`.

Test updates in the existing suite:

- The stray tests match `CloseBracketParent::Stray` and assert the position inside it: `a_stray_close_is_one_token_inside_the_balanced_brace` and `a_stray_close_does_not_end_a_different_kind` find `Stray(BracketItemParent::Bracketed(group))` with the group balanced; `a_stray_close_at_the_top_level_is_a_leaf_of_the_root` and the crossing test's trailing `}` find `Stray(BracketItemParent::MatchedBrackets(_))`.
- `the_close_pairs_with_the_nearest_open` matches `CloseBracketParent::Bracketed(group)` for the real `}` and asserts the balanced group in its payload, its parent being the forced-shut outer group.
- The `close_bracket` extractor keeps its shape; two helpers beside it unwrap the two parent variants.

## Landing checklist

1. The change; `cargo test` green; the clippy hook passes.
2. chunking.md's quoted slot and stray test updated alongside.
3. Move this doc to refactors/past.
