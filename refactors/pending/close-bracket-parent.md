# The close bracket's parent

Requires resolve-position-parent-conversion.md. A close bracket sits in one of two positions — a group's real closing, or a stray item — and its parent enum says which, so genuine-versus-stray reads off the path as a variant instead of a pointer comparison.

## The change

In `matched_brackets.rs`. The parent enum, with the genuine variant named `Bracketed` because the macro names parent variants after the containing struct:

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

/// The conversion the stray delegation passes through.
impl<'a, TContents: TreeContents> From<BracketItemParent<'a, TContents>>
    for CloseBracketParent<'a, TContents>
{
    fn from(parent: BracketItemParent<'a, TContents>) -> Self {
        CloseBracketParent::Stray(parent)
    }
}
```

`CloseBracketPath` reparents onto it, and the derive follows.

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

Both construction sites already emit code that lands in the right variant:

- The closing field, inside `Bracketed`'s derived resolve, emits `<CloseBracket as ResolvePosition>::Parent::Bracketed(self.path(parent).into())`: the variant is `CloseBracketParent::Bracketed`, `self.path(parent)` is the group's `BracketedPath`, and the `.into()` boxes it.
- The stray arm, inside `BracketItem`'s derived resolve, emits `inner.resolve(parent.into(), position)` after resolve-position-parent-conversion.md, and the `From` above wraps the position in `Stray`.

`CloseBracket`'s own derived fallback, `Self::ResolvedNode::CloseBracket(self.path(parent).into())`, builds the reparented `CloseBracketPath` unchanged.

Test updates in the existing suite:

- The stray tests match `CloseBracketParent::Stray` and assert the position inside it: `a_stray_close_is_one_token_inside_the_balanced_brace` and `a_stray_close_does_not_end_a_different_kind` find `Stray(BracketItemParent::Bracketed(group))` with the group balanced; `a_stray_close_at_the_top_level_is_a_leaf_of_the_root` and the crossing test's trailing `}` find `Stray(BracketItemParent::MatchedBrackets(_))`.
- `the_close_pairs_with_the_nearest_open` matches `CloseBracketParent::Bracketed(group)` for the real `}` and asserts the balanced group in its payload, its parent being the forced-shut outer group.
- The `close_bracket` extractor keeps its shape; two helpers beside it unwrap the two variants.

## Landing checklist

1. resolve-position-parent-conversion.md lands first.
2. The change; `cargo test` green.
3. Move this doc to refactors/past.
