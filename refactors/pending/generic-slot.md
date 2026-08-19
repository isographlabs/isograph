# generic-slot: one impl for every `Slot<T, E>`

`Slot` is used at the root and in every list. One pinned impl cannot cover `Slot<P, UnparsedChunkItems>` for a later list item `P`. Drop `self_type_generics`. Both fields use `parent_from`. `Slot` is stored in the tree and is not a path segment and not a `ResolvedNode` variant.

`#[resolve_field(transparent)]` does not apply: that attribute is a field with no `WithSpan` check that always delegates. `Slot` has two `Option<WithSpan<_>>` fields. The matching emit is `parent_from` on both, and when every marked field is `parent_from` the no-hit arm is `return parent.to()`, not `ResolvedNode::Slot(...)`.

A position in a field skips `Slot` in the path. A position in the slot span but in neither field, including `{ item: None, extra_tokens: None }`, answers the container: `Singleton` at the root, `SelectionSet` in a selection list. Leftover span stays tight to the leftover tokens. The space after `foo` in `entrypoint Query.foo bar` is that gap.

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
    for<'a> IsographResolutionNode<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    #[resolve_field]
    #[parent_from]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`Slot::Parent` is `T::Parent`. Both fields emit `From::from(parent)`. The no-hit arm is `return parent.to()`. That requires `IsographResolutionNode: From<T::Parent>`.

At the root, `T` is `IsoLiteralItem` and `E` is `UnparsedChunkItems`. After this doc, `IsoLiteralItem::Parent` is `IsoLiteralParsePath`. The generated leftover arm is:

```
let new_parent = From::from(parent);
return extra.item.resolve(new_parent, position);
```

`parent` is `IsoLiteralParsePath`. `UnparsedChunkItems::resolve` wants `UnparsedChunkItems::Parent`, which is `UnparsedChunkItemsParent`. `From::from` is the wrap `UnparsedChunkItemsParent::Literal(parent)`. The compiler requires `UnparsedChunkItemsParent: From<IsoLiteralParsePath>`.

The item arm is the same `From::from(parent)`, but the target is `IsoLiteralItem::Parent`, which is `IsoLiteralParsePath`. That is `From<P> for P`. No impl to write.

The generic leftover bound is that fact for any `T` / `E`: `E::Parent: From<T::Parent>`.

The last bound is the no-hit arm. At the root `T::Parent` is `IsoLiteralParsePath`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
impl<'a> From<IsoLiteralParsePath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralParsePath<'a>) -> Self {
        IsographResolutionNode::Singleton(path)
    }
}
```

A list that stores a `Slot` adds `From<that list's path> for IsographResolutionNode` (the list's own variant) and an `UnparsedChunkItemsParent` variant. `SlotPath` is deleted. `IsographResolutionNode::Slot` is deleted.

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

`UnparsedChunkItemsPath` moves from `parse_iso_literal.rs` to `chunk.rs`. `IsoLiteralItem` and `EntrypointDeclaration` no longer use `SlotPath` as `parent_type`. `SlotPath` is deleted. `isograph_resolution_node.rs` drops `Slot` and the `SlotPath` import. A gap answers `Singleton`; hover does not see a slot.

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

## Macro: `parent_from` on a struct field

`#[resolve_field]` + `#[parent_from]` on a struct field is accepted. Emission is `From::from(parent)`.

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

If every marked field is `parent_from`, the struct is not a leaf: fallback is `return parent.to()`. `transparent` still suppresses fallback (that field always returns). Mixed `parent_from` and ordinary `#[resolve_field]` keeps the struct-name fallback.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let fallback = if field_infos
        .iter()
        .any(|info| matches!(info.field_type, ResolveFieldInfoTypeWrapper::Transparent(_)))
    {
        quote!()
    } else if !field_infos.is_empty()
        && field_infos.iter().all(|info| {
            matches!(info.parent_construction, ParentConstruction::FromParent)
        })
    {
        quote! {
            return parent.to();
        }
    } else {
        quote! {
            return Self::ResolvedNode::#struct_name(self.path(parent).to());
        }
    };
```

## Generated `Slot`

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition, E: ResolvePosition> ::resolve_position::ResolvePosition for Slot<T, E>
where
    for<'a> T: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> E: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> <E as ResolvePosition>::Parent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
    for<'a> IsographResolutionNode<'a>: From<<T as ResolvePosition>::Parent<'a>>,
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
        return parent.to();
    }
}
```

The derive does not write `'static`. It writes `type ResolvedNode<'a> = IsographResolutionNode<'a> where Self: 'a` and keeps the type's own parameters and where-clause via `split_for_impl`.

`where Self: 'a` on the associated types makes `parent_type = <T as ResolvePosition>::Parent<'a>` legal.

Take `entrypoint Query.foo bar`:

- `item` span is `entrypoint Query.foo`
- leftover span is tight to `bar`
- the space after `foo` is in the slot span and in neither field

That space answers `IsographResolutionNode::Singleton`. A position on `bar` answers the token. `{ item: None, extra_tokens: None }` has no field hits, so the same fallback answers `Singleton`.

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

impl<'a> From<ParentPath<'a>> for TestResolvedNode<'a> {
    fn from(path: ParentPath<'a>) -> Self {
        TestResolvedNode::List(path)
    }
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
    for<'a> TestResolvedNode<'a>: From<<T as ResolvePosition>::Parent<'a>>,
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
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::List(list_path) => {
                assert!(std::ptr::eq(list_path.inner, list.reference()));
            }
        },
        node => panic!("expected leftover, got {node:?}"),
    }
}

#[test]
fn a_gap_after_the_item_resolves_to_the_list() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
    }
}

#[test]
fn an_empty_slot_resolves_to_the_list() {
    let list = slot_list(
        Slot {
            item: None,
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
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

### Parser

Entrypoint tests keep passing. A leftover token still resolves to `NonBracketToken`. `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` still resolves `Query` / `foo` / `entrypoint` / `.` the same way: those positions are in `item`, so the path does not go through `Slot`. `token.parent` inside leftover is `ChunkContentItemParent::Unparsed`; that path's parent is `UnparsedChunkItemsParent::Literal`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_gap_after_the_item_resolves_to_the_singleton() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::Singleton(_) => {}
            node => panic!("expected the singleton, got {node:?}"),
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

1. Macro: `parent_from` on struct fields, `FromParent` on `ParentConstruction`, all-`parent_from` fallback is `parent.to()`, generic `Slot`, `From<IsoLiteralParsePath> for IsographResolutionNode`, deleted `Slot` / `SlotPath`, `UnparsedChunkItemsParent`, entrypoint parent paths, the tests above. Leftover span is unchanged. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Move this doc to refactors/past.
