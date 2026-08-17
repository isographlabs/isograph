# Slot and Singleton in the tree

Lands after parse-entrypoint.md. Entrypoint keeps `IsoLiteralParse` / `IsoLiteralSlot`. This doc deletes those two types and puts `Slot` and `Singleton` in the tree, with `ResolvePosition` on the optimistic monomorphs and then on the generic structs.

`parse_one_item` / `parse_items` / `parse_singleton` already return `Slot` and `Singleton`. After this doc, those values are the tree. There is no `From` into a parallel concrete slot.

This is not resolve-position-generic-slot.md. That doc derives `LevelSlot<T>` / `ParsedSlot<T>` (parsed vs unparsed enum, trailing on the parsed wrapper). The series' `Slot` is `item` plus leftover `extra_tokens`. This doc does not add `LevelSlot`.

## After

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SingletonPath<'a>,
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <
        Option<WithSpan<IsoLiteralItem>>,
        Option<WithSpan<UnparsedChunkItems>>
    >
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: T,
    #[resolve_field]
    pub extra_tokens: E,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <
        WithSpan<Slot<
            <OptimisticStage as Stage>::IsoLiteral,
            <OptimisticStage as Stage>::UnparsedTokens
        >>,
        <OptimisticStage as Stage>::ExtraChunks
    >
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: T,
    #[resolve_field]
    pub extra_chunks: E,
}

pub type IsoLiteralParse = Singleton<
    WithSpan<Slot<
        <OptimisticStage as Stage>::IsoLiteral,
        <OptimisticStage as Stage>::UnparsedTokens,
    >>,
    <OptimisticStage as Stage>::ExtraChunks,
>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;

pub type SlotPath<'a> = PositionResolutionPath<
    &'a Slot<
        <OptimisticStage as Stage>::IsoLiteral,
        <OptimisticStage as Stage>::UnparsedTokens,
    >,
    IsoLiteralParsePath<'a>,
>;

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    push_error: impl FnMut(WithSpan<ParseError>),
) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(WithSpan::new(ParseError::EmptyLiteral, location));
        return WithSpan::new(
            Singleton {
                item: WithSpan::new(
                    Slot {
                        item: None,
                        extra_tokens: None,
                    },
                    location,
                ),
                extra_chunks: None,
            },
            location,
        );
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        Expectation::EndOfDeclaration,
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    WithSpan::new(singleton, location)
}
```

`parse_singleton` already returns `OptimisticSingleton`. That value is the tree. `From<OptimisticSlot> for IsoLiteralSlot` and `From<OptimisticSingleton> for IsoLiteralParse` are deleted. `IsoLiteralSlot` and `IsoLiteralParse` the structs are deleted. The `IsoLiteralParse` name remains only as the alias above.

`IsoLiteralItem`'s `parent_type` is `SlotPath<'a>`. `UnparsedChunkItems`'s `parent_type` is `SlotPath<'a>`. `ExtraChunks`'s `parent_type` is `IsoLiteralParsePath<'a>`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
pub enum IsographResolutionNode<'a> {
    IsoLiteralParse(IsoLiteralParsePath<'a>),
    Slot(SlotPath<'a>),
    IsoLiteralItem(IsoLiteralItemPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
    ExtraChunks(ExtraChunksPath<'a>),
    // chunk-tree variants unchanged
}
```

`IsoLiteralParse(...)` is `SingletonPath` under the alias. The variant name stays `IsoLiteralParse` so resolve tests still match the root. `IsoLiteralSlot(...)` is gone; that leaf is `Slot(...)`.

## Macro: substitute `self_type_generics` into field types first

Today `get_resolve_field_info` classifies `field.ty` as written. `item: T` is a bare path `T`. The classify walk only accepts `WithSpan` / `Option` / `Vec` / `NonEmpty` of those. `self_type_generics` only appears on the impl header: `impl ResolvePosition for Slot<Option<WithSpan<IsoLiteralItem>>, ...>`. It does not rewrite `T` before classify.

`replace_generics_in_type` already substitutes a type parameter for its `self_type_generics` argument. It is only called on the inner type of an already-recognized wrapper.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    if let syn::Type::Path(syn::TypePath { path, .. }) = field.ty.reference() {
        match parse_resolve_field_type(path, generics_map) {
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let field_ty = replace_generics_in_type(field.ty.clone(), generics_map);
    if let syn::Type::Path(syn::TypePath { path, .. }) = field_ty.reference() {
        match parse_resolve_field_type(path, generics_map) {
```

On `Slot` with `self_type_generics = <Option<WithSpan<IsoLiteralItem>>, Option<WithSpan<UnparsedChunkItems>>>`, `T` becomes `Option<WithSpan<IsoLiteralItem>>` and `E` becomes `Option<WithSpan<UnparsedChunkItems>>`. Classify already walks both.

`S::IsoLiteral` is not a type parameter. Replacing `S` with `OptimisticStage` yields `OptimisticStage::IsoLiteral`, still an associated-type path. Classify still rejects it. This doc does not inline associated types. The tree uses `Slot<T, E>` with `T` / `E` filled at the use site (`S::IsoLiteral` as a type argument, not as a field type).

## Generated `Slot` (root optimistic monomorph)

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition
    for Slot<Option<WithSpan<IsoLiteralItem>>, Option<WithSpan<UnparsedChunkItems>>>
{
    type Parent<'a> = SingletonPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.item.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = <UnparsedChunkItems as ::resolve_position::ResolvePosition>::Parent::Unparsed(
                    self.path(parent).to(),
                );
                return item.item.resolve(new_parent, position);
            }
        }
        return Self::ResolvedNode::Slot(self.path(parent).to());
    }
}
```

`item` uses the container path (`SlotPath`) as `IsoLiteralItem`'s parent. `extra_tokens` uses `parent_variant = Unparsed` on `UnparsedChunkItems`'s contents today; the `UnparsedChunkItems` value's parent is `SlotPath`. The `#[resolve_field]` on `Slot.extra_tokens` is bare: `UnparsedChunkItems::Parent` is `SlotPath`, so the parent is `self.path(parent)` unwrapped. The `parent_variant = Unparsed` stays on `UnparsedChunkItems`'s `NonEmpty` field, not on `Slot.extra_tokens`.

## Generated `Singleton` (root optimistic monomorph)

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition
    for Singleton<
        WithSpan<
            Slot<
                Option<WithSpan<IsoLiteralItem>>,
                Option<WithSpan<UnparsedChunkItems>>,
            >,
        >,
        Option<WithSpan<ExtraChunks>>,
    >
{
    type Parent<'a> = ();
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        if self.item.location.contains(position) {
            let new_parent = self.path(parent);
            return self.item.item.resolve(new_parent, position);
        }
        for item in self.extra_chunks.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return Self::ResolvedNode::IsoLiteralParse(self.path(parent).to());
    }
}
```

The fallback variant is `IsoLiteralParse` even though the type is `Singleton<...>`. The derive today emits `Self::ResolvedNode::#struct_name`. That would be `Singleton`. The attribute gains `resolved_node_variant = IsoLiteralParse` so the root leaf keeps the name the entrypoint tests use.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
    self_type_generics: Option<syn::AngleBracketedGenericArguments>,
    resolved_node_variant: Option<syn::Ident>,
}
```

When `resolved_node_variant` is present, the fallback is `Self::ResolvedNode::#resolved_node_variant(...)`. When it is absent, the fallback is `Self::ResolvedNode::#struct_name(...)` as today. `Slot`'s fallback is `Slot` (the struct name). `Singleton`'s fallback is `IsoLiteralParse`.

## Macro: generic impl for later lists

One `self_type_generics` emits one impl. A selection slot is another `Slot<Option<WithSpan<Selection>>, Option<WithSpan<UnparsedChunkItems>>>`. A second pinned impl cannot come from the same derive.

After the root monomorph works, the derive emits a generic impl when `self_type_generics` is absent and the struct has type parameters. `handle_data_struct` already receives `input_generics`. It uses `split_for_impl` and keeps the where-clause.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
pub struct Slot<T: ResolvePosition, E> {
    #[resolve_field(transparent)]
    pub item: T,
    #[resolve_field]
    pub extra_tokens: E,
}
```

`transparent` is resolve-position-generic-slot.md's field mode: no `WithSpan` check, `T::resolve` with the same parent. This doc lands `transparent` if that doc has not. `Slot` is not a path segment in the generic impl. A position in leftover still walks `extra_tokens`. A position in the slot span but in neither field answers `T`'s parent (the singleton or the list).

The pinned root impl from the previous section is deleted when this generic impl lands. `IsographResolutionNode::Slot` is deleted. `IsoLiteralItem`'s `parent_type` becomes `IsoLiteralParsePath<'a>` (the singleton). `UnparsedChunkItems`'s `parent_type` becomes a parent enum:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    Root(IsoLiteralParsePath<'a>),
}

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;
```

```rust
// from crates/isograph_parser/src/chunk.rs
impl<'a> From<IsoLiteralParsePath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: IsoLiteralParsePath<'a>) -> Self {
        UnparsedChunkItemsParent::Root(parent)
    }
}
```

`Slot.extra_tokens`'s `#[resolve_field]` uses `parent_from` (resolve-position-generic-slot.md) so `UnparsedChunkItems` receives `From::from` of the container parent. Feature docs that put a `Slot` in a list add a variant and a `From`.

`Singleton` in a `[...]` type has a parent other than `()`. The generic `Singleton` impl is the same shape as `Slot`: transparent `item`, `extra_chunks` via `parent_from`, `parent_type = <T as ResolvePosition>::Parent<'a>`. The root pinned impl (`parent_type = ()`, `resolved_node_variant = IsoLiteralParse`) stays until `[...]` lands; parse-variables.md deletes it and uses the generic impl, with `TypeAnnotation` (or the type-annotation path that doc names) as `T::Parent`.

## Lists

A selection set after parse-fields.md:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(
    #[resolve_field]
    pub Vec<WithSpan<Slot<Option<WithSpan<Selection>>, Option<WithSpan<UnparsedChunkItems>>>>>,
);
```

`Stage` grows `type Selection` when that feature wants artifact conversion. The tree field is still the explicit `Slot<...>` arguments, not `S::Selection` as a `#[resolve_field]` type.

`parse_items` already returns `Vec<WithSpan<Slot<...>>>`. After this doc there is no per-list concrete slot struct and no `From`.

## Deleted types

`IsoLiteralSlot` the struct. `IsoLiteralParse` the struct. Both `From` impls into those structs. `IsoLiteralSlotPath`. `IsographResolutionNode::IsoLiteralSlot`.

## Tests

`crates/resolve_position_macros`: a struct `Slot<T, E>` with `self_type_generics = <Option<WithSpan<Child>>, Option<WithSpan<Extra>>>`. `Child` and `Extra` impl `ResolvePosition`. Assert a position on the child's span resolves to the child; a position on the extra span resolves to extra; a position in the slot but in neither field resolves to `Slot`.

`crates/isograph_parser` entrypoint tests: `as_entrypoint` walks `parse.item.item.item` (`Singleton.item` is `WithSpan<Slot>`, `Slot.item` is `Option<WithSpan<IsoLiteralItem>>`). Leftover is `parse.item.item.extra_tokens`. Extra chunks are `parse.extra_chunks`. Resolve on a leftover token is `NonBracketToken`. Resolve on `Query` is `EntityName`. `IsoLiteralParse` / `Slot` leaves match the renamed variants.

## Shipping

1. Macro: substitute `self_type_generics` into field types; `resolved_node_variant`; tests in `resolve_position_macros`. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. `Slot` / `Singleton` derive the root optimistic monomorph. Delete `IsoLiteralSlot`, the `IsoLiteralParse` struct, and both `From`s. Alias `IsoLiteralParse`. Repoint `IsoLiteralItem` / `UnparsedChunkItems` / `ExtraChunks` parents. Entrypoint tests pass.
3. Generic impl + `transparent` + `parent_from` (if not already landed). Delete the pinned `Slot` impl. `IsographResolutionNode::Slot` goes away. `UnparsedChunkItemsParent`. Entrypoint tests pass; leftover parent is `UnparsedChunkItemsParent::Root`.

Lands after parse-entrypoint.md. parse-fields.md and later do not add new concrete slot structs.
