# resolve_position: parent conversion on delegation

The macro's enum delegation arms call `inner.resolve(parent.into(), position)` instead of `inner.resolve(parent, position)`.

Today a payload's `Parent` type must equal its containing enum's `Parent` type, because the parent is passed through untouched. With the conversion, a payload may have its own parent enum, one total `From` away: the path then records which position the node sits in, and one node type can appear in several positions. The first user is `CloseBracket` (close-bracket-node.md), which sits both as a group's real closing and as a stray item, with `CloseBracketParent` an enum of the two.

This ships alone with no behavior change: `impl<T> From<T> for T` makes the conversion the identity for every existing derive.

## The change

In `resolve_position_macro.rs`, `handle_data_enum`'s arm emission.

Before:

```rust
quote! {
    #enum_name::#variant_name(inner) => inner.resolve(parent, position)
}
```

After:

```rust
quote! {
    #enum_name::#variant_name(inner) => inner.resolve(parent.into(), position)
}
```

The generated code for `BracketItem<BracketsMatched>`, written out, after:

```rust
impl ::resolve_position::ResolvePosition for BracketItem<BracketsMatched> {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            BracketItem::Inner(inner) => inner.resolve(parent.into(), position),
            BracketItem::Bracketed(inner) => inner.resolve(parent.into(), position),
            BracketItem::StrayClose(inner) => inner.resolve(parent.into(), position),
        }
    }
}
```

Every payload here has `Parent = BracketItemParent`, so each `.into()` is the reflexive `From` and the impl is unchanged in behavior. The mixed emission added by resolve-option-like-enums.md uses the same call shape in its delegating arms, and close-bracket-node.md's `From<BracketItemParent> for CloseBracketParent` is the first non-reflexive conversion.
