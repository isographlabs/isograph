# resolve_position: `resolve_into` and fallbacks

`#[derive(ResolvePosition)]` on an enum currently requires every variant to delegate to a payload with the enum's own `Parent` and `ResolvedNode`. That shuts out mixed enums, where some variants continue into another node's resolution and the rest are inert. Two attribute additions fix it:

- `#[resolve_into]` on a variant: the position continues into the payload, whose parent may differ from the enum's by one total `From` (the call shape from resolve-position-parent-conversion.md).
- `fallback = VariantName` on the enum: every unmarked variant answers that `ResolvedNode` variant, with the parent converted into its payload.

An enum with no marks keeps today's all-delegate emission. The derive's attribute list becomes `attributes(resolve_field, resolve_into, resolve_position)`.

## The change

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
    /// For a mixed enum: the `ResolvedNode` variant an unmarked variant answers with,
    /// holding what the parent converts into.
    fallback: Option<syn::Ident>,
}
```

`handle_data_enum` scans variants for `#[resolve_into]`. With at least one mark, the emission is:

```rust
impl ::resolve_position::ResolvePosition for #enum_name {
    type Parent<'a> = #parent_type;
    type ResolvedNode<'a> = #resolved_node;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span
    ) -> Self::ResolvedNode<'a> {
        match self {
            #(#enum_name::#marked_variant(inner) => inner.resolve(parent.into(), position),)*
            _ => Self::ResolvedNode::#fallback(parent.into()),
        }
    }
}
```

`fallback` is required iff some variant is unmarked, and giving it on an all-delegate enum is an error. A variant with named fields or multiple unnamed fields is an error, as today. A unit variant may only be unmarked (there is no payload to resolve into). An enum whose every variant is marked is the all-delegate emission spelled redundantly, rejected with an error saying to drop the marks.

## The consumer

No current design needs it: raw-items.md's `RawToken` was the candidate consumer until every one of its variants became a delegating node, leaving no fallback. Parked until an enum genuinely mixes continuing and inert variants.

## Landing checklist

1. resolve-position-parent-conversion.md lands first (the delegating call shape).
2. This change; the existing all-delegate derives compile untouched.
3. Move this doc to refactors/past when raw-items.md's Change 2 lands on top.
