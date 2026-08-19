# generic-slot: one concrete `ResolvePosition` impl per `Slot<T, E>` pin

Lands after resolve-position-on-unmatched-span.md (refactors/past). GAT predicates on `type Parent` / `type ResolvedNode` are in the derive. `PathParent` (refactors/past/path-parent.md) was for `T::Parent::Parent` on a generic Slot impl. This doc does not use that. Change 1 deletes it.

A generic `impl<T, E> ResolvePosition for Slot<T, E>` does not compile (E0276 on extra GAT bounds; `for<'a> T: ResolvePosition<Parent<'a> = Path<&'a Slot<T, E>, …>>` overflows or implies `'static`). `Slot<IsoLiteralItem, UnparsedChunkItems>` and `Slot<Selection, UnparsedChunkItems>` are different types. Each gets its own concrete impl. `self_type_generics` becomes a list of pins. Each pin is type arguments plus that impl’s `parent_type`. The derive emits one `impl ResolvePosition for Slot<…>` per pin.

This doc’s list has the root pin. A later list appends `(<Selection, UnparsedChunkItems>, SelectionSetPath<'a>)`. `resolved_node` and `on_unmatched_span` stay on the container (every pin uses `IsographResolutionNode` and `from_path`).

`UnparsedChunkItems` is `E` on every pin. One pin: its `parent_type` is `IsoLiteralSlotPath` (live `SlotPath`). Another pin: that parent is an enum of the slot paths (`IsoLiteralSlot(IsoLiteralSlotPath)`, `SelectionSlot(SelectionSlotPath)`, an argument slot, …). `T` is not shared; `IsoLiteralItem`’s parent stays `IsoLiteralSlotPath`, `Selection`’s stays `SelectionSlotPath`.

`item` stays bare `#[resolve_field]`. `extra_tokens` stays bare while leftover’s parent is the one slot path. Once leftover’s parent is the enum, `extra_tokens` cannot stay bare (`Parent` is no longer equal to the slot path) and cannot use `#[parent_variant]` (one field, a different variant per pin). It uses `#[from_container_parent]`: `self.path(parent).to()`, with `From<IsoLiteralSlotPath>` / `From<SelectionSlotPath>` into the leftover parent enum. Live `from_container_parent` is enum-payload only; the struct-field form lands with that second pin.

The `Slot` is in the path for `item`, leftover, and the gap. That is live today.

Live `SlotPath` is renamed `IsoLiteralSlotPath`. `IsographResolutionNode::Slot` is renamed `IsoLiteralSlot`.

Leftover span stays tight to the leftover tokens. The space after `foo` in `entrypoint Query.foo bar` is that gap.

## After

```rust
// from crates/isograph_parser/src/chunk.rs
/// One parse attempt: an item plus leftover tokens in the same chunk.
///
/// `resolve` receives the list parent (`IsoLiteralParsePath` at the root, later
/// `SelectionSetPath` from a second pin).
/// Walk, given that parent:
/// - Position in `item`: bare `#[resolve_field]` passes `self.path(parent)`, a path
///   to this `Slot`. `T::Parent` is that path. `Slot` is a path segment.
/// - Position in `extra_tokens`: the same `self.path(parent)`.
/// - Position in the slot span but in neither field: `on_unmatched_span = from_path`
///   returns `self.path(parent).to()`. Each pin’s `From` builds that pin’s
///   `ResolvedNode` variant (`IsoLiteralSlot`, later `SelectionSlot`).
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
    ]
)]
pub struct Slot<T, E> {
    /// `Some` when the form parsed.
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    /// Unread or failed tokens after the item. Span is tight to those tokens.
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

Hover on `Query` is `entrypoint -> slot -> literal`. Hover on leftover `bar` is the leftover token through `UnparsedChunkItems` through the same `Slot` path. Hover on the space after `foo` is the `Slot`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type IsoLiteralSlotPath<'a> = PositionResolutionPath<
    &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
    IsoLiteralParsePath<'a>,
>;

impl<'a> From<IsoLiteralSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        IsographResolutionNode::IsoLiteralSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
```

`IsoLiteralItem`, `EntrypointDeclaration`, and `UnparsedChunkItems` have `parent_type = IsoLiteralSlotPath<'a>` (today `SlotPath`). Leftover’s parent is that path because this list has one pin.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>, ()),
    ]
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: WithSpan<T>,
    #[resolve_field]
    pub extra_chunks: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl<'a> From<IsoLiteralParsePath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralParsePath<'a>) -> Self {
        IsographResolutionNode::Singleton(path)
    }
}
```

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

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: WithSpan<T>,
    #[resolve_field]
    pub extra_chunks: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type SlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Slot(SlotPath<'a>),
```

Unmatched is `struct_name`. There is no `From` impl.

## Change 1: delete `PathParent`

Origin: `crates/resolve_position/src/lib.rs` after path-parent.md. Delta: the trait, the impl, the test, and the test-module import. No caller remains.

```rust
// from crates/resolve_position/src/lib.rs
pub trait PathParent {
    type Parent;
}

impl<Inner, Parent> PathParent for PositionResolutionPath<Inner, Parent> {
    type Parent = Parent;
}
```

```rust
// from crates/resolve_position/src/lib.rs
    use crate::{PathParent, PositionResolutionPath, ResolvePosition};
```

```rust
// from crates/resolve_position/src/lib.rs
    #[test]
    fn position_resolution_path_projects_its_parent_type_argument() {
        fn assert_parent<T: PathParent<Parent = U>, U>() {}
        assert_parent::<PositionResolutionPath<&u8, ()>, ()>();
    }
```

After: those three gone. The test-module import is `use crate::{PositionResolutionPath, ResolvePosition};`.

`cargo test -p resolve_position` passes.

## Change 2: `self_type_generics` is a list of pins, or omitted

Origin: `ResolvePositionArgs` and `validate_and_map_generics` in `crates/resolve_position_macros/src/resolve_position_macro.rs` and `map_generics.rs`. Delta: `self_type_generics` is omitted or a list of 2-tuples `[ (<A, B>, ParentTy), ... ]`. `<A, B>` is a parse error. `handle_data_struct` and `handle_data_enum` emit one impl per pin. Container `parent_type` is required when omitted, forbidden when the list is present. A list requires `on_unmatched_span = from_path`. `struct_name` and an omitted unmatched arm are compile errors. Slot and Singleton convert to one-element lists with `from_path` and `From` impls.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
struct ResolvePositionArgs {
    parent_type: Option<syn::Type>,
    resolved_node: syn::Type,
    self_type_generics: Option<SelfTypeGenerics>,
    on_unmatched_span: Option<syn::Ident>,
}

struct SelfTypeGenerics(Vec<SelfTypePin>);

struct SelfTypePin {
    /// One argument per generic parameter of the struct, in declaration order.
    /// `validate_and_map_generics` errors if the counts differ.
    args: syn::AngleBracketedGenericArguments,
    parent_type: syn::Type,
}
```

Omitted is the live generic impl: `parent_type` required, `ty_generics` from `split_for_impl`. Non-generic structs omit it. A generic struct omits it when `#[resolve_field]` does not force a concrete `Parent` equality on a type parameter (the bound that E0276's on `Slot`). `item: Option<WithSpan<T>>` with bare `#[resolve_field]` is that bound; a generic field that is not `#[resolve_field]` is not.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let output = match args.self_type_generics.reference() {
        None => {
            let parent_type = require_parent_type(args)?;
            emit_one_impl(
                /* live path: impl_generics, ty_generics from split_for_impl, parent_type */
            )
        }
        Some(pins) => {
            if let Some(parent_type) = args.parent_type.reference() {
                return Error::new_spanned(
                    parent_type,
                    "`parent_type` is on each pin when `self_type_generics` is present",
                )
                .to_compile_error()
                .to();
            }
            require_from_path_with_pins(args.on_unmatched_span.reference())?;
            let mut impls = Vec::new();
            for pin in pins.0.iter() {
                let generics_map = validate_and_map_generics(
                    input_generics.clone(),
                    pin.args.clone().wrap_some(),
                )?;
                let field_infos = /* same field walk, generics_map */;
                impls.push(emit_one_impl(
                    /* impl_generics empty, ty_generics = pin.args, parent_type = pin.parent_type */
                ));
            }
            quote!(#(#impls)*)
        }
    };
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn require_parent_type(
    args: &ResolvePositionArgs,
) -> Result<syn::Type, proc_macro2::TokenStream> {
    args.parent_type.clone().ok_or_else(|| {
        Error::new_spanned(
            args.resolved_node.reference(),
            "`parent_type` is required when `self_type_generics` is omitted",
        )
        .to_compile_error()
    })
}

fn require_from_path_with_pins(
    on_unmatched_span: Option<&syn::Ident>,
) -> Result<(), proc_macro2::TokenStream> {
    match on_unmatched_span {
        Some(ident) if ident == "from_path" => ().wrap_ok(),
        Some(ident) => Error::new_spanned(
            ident,
            "`on_unmatched_span = from_path` is required when `self_type_generics` is a list",
        )
        .to_compile_error()
        .wrap_err(),
        None => Error::new(
            proc_macro2::Span::call_site(),
            "`on_unmatched_span = from_path` is required when `self_type_generics` is a list",
        )
        .to_compile_error()
        .wrap_err(),
    }
}
```

`handle_data_enum` uses the same split: omitted stays the live generic impl; a list emits one impl per pin (`impl_generics` empty, `ty_generics = pin.args`, `parent_type = pin.parent_type`). Enums have no unmatched arm; `on_unmatched_span` on an enum is still a compile error.

`emit_one_impl` is the current `handle_data_struct` body after `field_infos` is known: predicates, unmatched, the `impl` quote.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
impl syn::parse::Parse for SelfTypeGenerics {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let mut pins = Vec::new();
        while !content.is_empty() {
            let inner;
            syn::parenthesized!(inner in content);
            let args = inner.parse::<syn::AngleBracketedGenericArguments>()?;
            inner.parse::<syn::Token![,]>()?;
            let parent_type = inner.parse::<syn::Type>()?;
            pins.push(SelfTypePin { args, parent_type });
            if content.peek(syn::Token![,]) {
                content.parse::<syn::Token![,]>()?;
            }
        }
        if pins.is_empty() {
            return Error::new(
                input.span(),
                "`self_type_generics` must contain at least one pin",
            )
            .wrap_err();
        }
        SelfTypeGenerics(pins).wrap_ok()
    }
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>, ()),
    ]
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: WithSpan<T>,
    #[resolve_field]
    pub extra_chunks: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl<'a> From<SlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: SlotPath<'a>) -> Self {
        IsographResolutionNode::Slot(path)
    }
}

impl<'a> From<IsoLiteralParsePath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralParsePath<'a>) -> Self {
        IsographResolutionNode::Singleton(path)
    }
}
```

Names are still `Slot` / `SlotPath`.

### Macro test: two pins on one `Slot`

Parser pins share `IsographResolutionNode`. The test does the same.

```rust
// from crates/resolve_position_macros/tests/self_type_generics_pins.rs
use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    ListA(PathA<'a>),
    ListB(PathB<'a>),
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
    ChildA(PositionResolutionPath<&'a ChildA, SlotAPath<'a>>),
    ExtraA(PositionResolutionPath<&'a ExtraA, SlotAPath<'a>>),
    ChildB(PositionResolutionPath<&'a ChildB, SlotBPath<'a>>),
    ExtraB(PositionResolutionPath<&'a ExtraB, SlotBPath<'a>>),
}

impl<'a> From<SlotAPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotAPath<'a>) -> Self {
        TestResolvedNode::SlotA(path)
    }
}

impl<'a> From<SlotBPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotBPath<'a>) -> Self {
        TestResolvedNode::SlotB(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListA(#[resolve_field] Vec<WithSpan<Slot<ChildA, ExtraA>>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListB(#[resolve_field] Vec<WithSpan<Slot<ChildB, ExtraB>>>);

type PathA<'a> = PositionResolutionPath<&'a ListA, ()>;
type PathB<'a> = PositionResolutionPath<&'a ListB, ()>;
type SlotAPath<'a> = PositionResolutionPath<&'a Slot<ChildA, ExtraA>, PathA<'a>>;
type SlotBPath<'a> = PositionResolutionPath<&'a Slot<ChildB, ExtraB>, PathB<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<ChildA, ExtraA>, PathA<'a>),
        (<ChildB, ExtraB>, PathB<'a>),
    ]
)]
struct Slot<T, E> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ExtraA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildB;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ExtraB;

#[test]
fn pin_a_item_and_gap() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: ExtraA.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::ChildA(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected ChildA, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotA(_) => {}
        node => panic!("expected SlotA, got {node:?}"),
    }
}

#[test]
fn pin_b_item_and_gap() {
    let list = ListB(
        Slot {
            item: ChildB.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: ExtraB.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::ChildB(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected ChildB, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotB(_) => {}
        node => panic!("expected SlotB, got {node:?}"),
    }
}
```

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Change 3: `IsoLiteralSlot`

Origin: `Slot` in `chunk.rs` after change 2. Delta: `SlotPath` → `IsoLiteralSlotPath`, `Slot` → `IsoLiteralSlot`, the `From` impl’s variant. `from_path` is already there.

The After listings. Generated root impl (predicates already emitted by the derive):

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for Slot<IsoLiteralItem, UnparsedChunkItems> {
    type Parent<'a>
        = IsoLiteralParsePath<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = IsographResolutionNode<'a>
    where
        Self: 'a,
        IsoLiteralItem: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = IsographResolutionNode<'a>
        >,
        IsoLiteralItem: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
                IsoLiteralParsePath<'a>
            >
        >,
        UnparsedChunkItems: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = IsographResolutionNode<'a>
        >,
        UnparsedChunkItems: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
                IsoLiteralParsePath<'a>
            >
        >,
        IsographResolutionNode<'a>: ::std::convert::From<
            ::resolve_position::PositionResolutionPath<
                &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
                IsoLiteralParsePath<'a>
            >
        >;

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
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return self.path(parent).to();
    }
}
```

### Parser tests

Entrypoint tests keep passing. `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` still goes through `IsoLiteralSlotPath`. Leftover `bar` is still `NonBracketToken`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_gap_after_the_item_resolves_to_the_slot() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::IsoLiteralSlot(path) => {
                assert!(path.inner.item.is_some());
                assert!(path.inner.extra_tokens.is_some());
            }
            node => panic!("expected IsoLiteralSlot, got {node:?}"),
        }
    }
```

`leftover_after_an_entrypoint_resolves_to_the_leftover_token` stays.

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Landing checklist

1. Change 1: delete `PathParent`, the impl, the test. `cargo test -p resolve_position` passes.
2. Change 2: `self_type_generics` is a list or omitted. A list requires `from_path`; `struct_name` is a compile error. Slot and Singleton convert to one-element lists with `from_path` and `From` impls. Two-pin macro test. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
3. Change 3: `IsoLiteralSlot`, `SlotPath` renamed, `From` variant updated, gap test. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
4. Move this doc to refactors/past.
