# generic-slot: one impl for every `Slot<T, E>`

`Slot` is used at the root and in every list. One pinned impl cannot cover `Slot<P, UnparsedChunkItems>` for a later list item `P`. Drop `self_type_generics`. Both fields use `parent_from`. `Slot` is not a path segment and is not a `ResolvedNode` variant.

Origin: the generic-Slot change previously in parse-fields.md. Deltas from that origin: `Slot` states the `ResolvedNode` equality and leftover `From` bounds the generic impl body needs; leftover-gap and leftover parent-chain tests are written out; the generated impl does not write `'static`.

## After

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct Slot<T: ResolvePosition, E: ResolvePosition>
where
    for<'a> T: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> E: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> <E as ResolvePosition>::Parent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    #[resolve_field]
    #[parent_from]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`Slot::Parent` is `T::Parent`. Both fields emit `From::from(parent)`:

- `item` converts `T::Parent` to `T::Parent`. The blanket `From<P> for P` is identity. No bound.
- `extra_tokens` converts `T::Parent` to `E::Parent`. That is `From<T::Parent> for E::Parent`, the leftover wrap. Only `E` needs the bound.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
}

#[resolve_position(parent_type = UnparsedChunkItemsParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralParsePath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: IsoLiteralParsePath<'a>) -> Self {
        UnparsedChunkItemsParent::Literal(parent)
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* fields unchanged */ }

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralParsePath<'a>>;
```

`SlotPath` is deleted. `IsographResolutionNode::Slot` is deleted. `UnparsedChunkItemsPath` moves from `parse_iso_literal.rs` to `chunk.rs`. `chunk.rs` drops its `SlotPath` / `UnparsedChunkItemsPath` imports. `isograph_resolution_node.rs` drops the `Slot` variant and the `SlotPath` import.

A list that stores a `Slot` adds a variant of `UnparsedChunkItemsParent` and a `From`.

## Before

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = IsoLiteralParsePath<'a>,
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <IsoLiteralItem, UnparsedChunkItems>
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* ... */ }

pub type UnparsedChunkItemsPath<'a> = PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type SlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, SlotPath<'a>>;
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Slot(SlotPath<'a>),
```

## Macro: `parent_from` on a struct field

`#[resolve_field]` + `#[parent_from]` on a struct field is accepted. Emission is `From::from(parent)`. A struct that has a `parent_from` field has no container fallback (same as a `transparent` field).

Today `ParentConstruction` has no `FromParent`: enum payloads emit `From::from` directly, and a struct field with `#[parent_from]` is an error. This doc puts `FromParent` back for struct fields.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    Transparent,
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    /// `#[resolve_field]` + `#[parent_from]`: the child's `Parent` is `From` the
    /// container's `Parent`.
    FromParent,
    Transparent,
}
```

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = parent_from {
                parse_parent_from(attr)?;
                return Error::new_spanned(attr, "`#[parent_from]` is an enum-payload attribute")
                    .to_compile_error()
                    .wrap_err();
            }
            match parent_variant {
                Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                None => ParentConstruction::ContainerPath,
            }
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = parent_from {
                parse_parent_from(attr)?;
                ParentConstruction::FromParent
            } else {
                match parent_variant {
                    Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                    None => ParentConstruction::ContainerPath,
                }
            }
```

`Option<WithSpan<T>>` is already a legal field type. `new_parent_expr` gains a `FromParent` arm:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::FromParent => quote!(::std::convert::From::from(parent)),
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

Fallback suppression treats `FromParent` like `Transparent`:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let fallback = if field_infos.iter().any(|info| {
        matches!(
            info.parent_construction,
            ParentConstruction::Transparent | ParentConstruction::FromParent
        )
    }) {
        quote!()
    } else {
        quote! {
            return Self::ResolvedNode::#struct_name(self.path(parent).into());
        }
    };
```

## Leftover span

Form `Ok` and leftover: `extra_tokens.location` starts at `item.location.end`, so the gap after the item is inside leftover.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
                let leftover_span =
                    Span::join(remaining.first().location, remaining.last().location);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
                let leftover_span = Span::new(item.location.end, remaining.last().location.end);
```

A leftover gap is a position in the slot span but in neither field. Take `entrypoint Query.foo bar`:

- `item` span is `entrypoint Query.foo`
- leftover items start at `bar`

If leftover span were tight to `bar`, the space after `foo` would be in the slot span and in neither field. The pinned impl answers that space with `IsographResolutionNode::Slot`. The generic impl has no `Slot` fallback (both fields are `parent_from`), so `resolve` would not return.

Leftover span starts at `item.location.end`, so that space is inside `extra_tokens`. A position on the space answers `UnparsedChunkItems`. A position on `bar` answers the token.

Form `Ok` with no leftover: slot span is the item span. Form `Err`: extra is the whole chunk. Every position the parent passes into `Slot::resolve` is in a field. The generated body has no no-hit arm.

## Generated `Slot`

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition, E: ResolvePosition> ::resolve_position::ResolvePosition for Slot<T, E>
where
    for<'a> T: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> E: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> <E as ResolvePosition>::Parent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    type Parent<'a>
        = <T as ::resolve_position::ResolvePosition>::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = IsographResolutionNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.item.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(parent);
                return item.item.resolve(new_parent, position);
            }
        }
    }
}
```

The derive does not write `'static`. It writes `type ResolvedNode<'a> = IsographResolutionNode<'a> where Self: 'a` and keeps the type's own parameters and where-clause via `split_for_impl`.

`where Self: 'a` on the associated types makes `parent_type = <T as ResolvePosition>::Parent<'a>` legal.

`parent_type = <T as ResolvePosition>::Parent<'a>` makes `Slot`'s parent the same type as `T`'s parent. That parent is the container the slot sits in. `parent_from` on both fields forwards it. `Slot` is not a path segment and is not a `ResolvedNode` variant.

## Tests

### Macro

```rust
// from crates/resolve_position_macros/tests/parent_from_struct.rs
use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    List(ParentPath<'a>),
    Child(ChildPath<'a>),
    Extra(ExtraPath<'a>),
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct List(#[resolve_field] Vec<WithSpan<Slot<Child, Extra>>>);

type ParentPath<'a> = PositionResolutionPath<&'a List, ()>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = TestResolvedNode<'a>
)]
struct Slot<T: ResolvePosition, E: ResolvePosition>
where
    for<'a> T: ResolvePosition<ResolvedNode<'a> = TestResolvedNode<'a>>,
    for<'a> E: ResolvePosition<ResolvedNode<'a> = TestResolvedNode<'a>>,
    for<'a> <E as ResolvePosition>::Parent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    #[resolve_field]
    #[parent_from]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ParentPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ChildPath<'a> = PositionResolutionPath<&'a Child, ParentPath<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ExtraParent<'a>, resolved_node = TestResolvedNode<'a>)]
struct Extra;

type ExtraPath<'a> = PositionResolutionPath<&'a Extra, ExtraParent<'a>>;

#[derive(Debug)]
enum ExtraParent<'a> {
    List(ParentPath<'a>),
}

impl<'a> From<ParentPath<'a>> for ExtraParent<'a> {
    fn from(path: ParentPath<'a>) -> Self {
        ExtraParent::List(path)
    }
}

fn slot_list(slot: Slot<Child, Extra>, span: Span) -> List {
    List(slot.with_span(span).wrap_vec())
}

#[test]
fn an_item_resolves_to_the_child_with_the_list_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.reference()));
        }
        node => panic!("expected the child, got {node:?}"),
    }
}

#[test]
fn leftover_resolves_through_from_the_list_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(4, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(5, 6)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::List(list_path) => {
                assert!(std::ptr::eq(list_path.inner, list.reference()));
            }
        },
        node => panic!("expected leftover, got {node:?}"),
    }
}

#[test]
fn a_gap_after_the_item_resolves_as_leftover() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(4, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::Extra(_) => {}
        node => panic!("expected leftover, got {node:?}"),
    }
}

#[test]
fn a_position_outside_the_slot_resolves_to_the_list() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(10, 11)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
    }
}
```

`TestResolvedNode` has no `Slot` variant. A position that reached the slot is answered by `Child` or `Extra`.

### Parser

Entrypoint tests keep passing. A leftover token still resolves to `NonBracketToken`. `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` still resolves `Query` / `foo` / `entrypoint` / `.` the same way. `token.parent` inside leftover is `ChunkContentItemParent::Unparsed`; that path's parent is `UnparsedChunkItemsParent::Literal`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_gap_after_the_item_resolves_as_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::UnparsedChunkItems(unparsed) => match unparsed.parent {
                UnparsedChunkItemsParent::Literal(_) => {}
            },
            node => panic!("expected leftover, got {node:?}"),
        }
    }

    #[test]
    fn leftover_token_parent_is_the_literal() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => match token.parent {
                ChunkContentItemParent::Unparsed(unparsed) => match unparsed.parent {
                    UnparsedChunkItemsParent::Literal(_) => {}
                },
                parent => panic!("expected an unparsed parent, got {parent:?}"),
            },
            node => panic!("expected the leftover token, got {node:?}"),
        }
    }
```

`leftover_after_an_entrypoint_resolves_to_the_leftover_token` stays.

## Landing checklist

1. Macro: `parent_from` on struct fields, fallback suppression, generic `Slot`, `UnparsedChunkItemsParent`, leftover span, deleted `Slot` / `SlotPath`, entrypoint parent paths, the tests above. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Move this doc to refactors/past.
