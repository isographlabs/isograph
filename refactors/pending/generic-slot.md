# generic-slot: one concrete `ResolvePosition` impl per `Slot<T, E>` pin

Lands after resolve-position-on-unmatched-span.md (refactors/past). GAT predicates on `type Parent` / `type ResolvedNode` are in the derive. `PathParent` (refactors/past/path-parent.md) was for `T::Parent::Parent` on a generic Slot impl. This doc does not use that. Change 1 deletes it.

A generic `impl<T, E> ResolvePosition for Slot<T, E>` does not compile (E0276 on extra GAT bounds; `for<'a> T: ResolvePosition<Parent<'a> = Path<&'a Slot<T, E>, …>>` overflows or implies `'static`). `Slot<IsoLiteralItem, UnparsedChunkItems>` and `Slot<Selection, UnparsedChunkItems>` are different types. Each gets its own concrete impl. `self_type_generics` becomes a list of pins. Each pin is type arguments plus that impl’s `parent_type`. The derive emits one `impl ResolvePosition for Slot<…>` per pin.

This doc’s list has the root pin. A later list appends `(<Selection, UnparsedChunkItems>, SelectionSetPath<'a>)`. `resolved_node` and `on_unmatched_span` stay on the container (every pin uses `IsographResolutionNode` and `from_path`).

Both fields stay bare `#[resolve_field]`. The `Slot` is in the path for `item`, leftover, and the gap. That is live today.

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

`IsoLiteralItem`, `EntrypointDeclaration`, and `UnparsedChunkItems` keep `parent_type = IsoLiteralSlotPath<'a>` (today `SlotPath`).

`Singleton` keeps the old one-list form: `parent_type = ()` and `self_type_generics = <Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>`.

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

## Change 2: `self_type_generics` is one pin or a list of pins

Origin: `ResolvePositionArgs` and `validate_and_map_generics` in `crates/resolve_position_macros/src/resolve_position_macro.rs` and `map_generics.rs`. Delta: `self_type_generics` parses as either one `<A, B>` (live; uses the container `parent_type`) or `[ (<A, B>, ParentTy), ... ]`. `handle_data_struct` emits one impl per pin. `parent_type` is optional when every pin carries its parent type.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
struct ResolvePositionArgs {
    parent_type: Option<syn::Type>,
    resolved_node: syn::Type,
    self_type_generics: Option<SelfTypeGenerics>,
    on_unmatched_span: Option<syn::Ident>,
}

enum SelfTypeGenerics {
    One(syn::AngleBracketedGenericArguments),
    Pins(Vec<SelfTypePin>),
}

struct SelfTypePin {
    args: syn::AngleBracketedGenericArguments,
    parent_type: syn::Type,
}
```

`One` is live `self_type_generics = <IsoLiteralItem, UnparsedChunkItems>` with container `parent_type`. `Pins` is the list form. An omitted `self_type_generics` is still a generic impl over the struct’s own params (no pin).

If `self_type_generics` is `Pins`, container `parent_type` is absent. If it is `One` or omitted, container `parent_type` is required.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn pins_to_emit(
    args: &ResolvePositionArgs,
) -> Result<Vec<(syn::AngleBracketedGenericArguments, syn::Type)>, proc_macro2::TokenStream> {
    match args.self_type_generics.reference() {
        None => {
            let parent_type = require_parent_type(args)?;
            (syn::parse_quote!(<>), parent_type).wrap_vec().wrap_ok()
        }
        Some(SelfTypeGenerics::One(one)) => {
            let parent_type = require_parent_type(args)?;
            (one.clone(), parent_type).wrap_vec().wrap_ok()
        }
        Some(SelfTypeGenerics::Pins(pins)) => {
            if args.parent_type.is_some() {
                return Error::new_spanned(
                    /* parent_type attr */,
                    "`parent_type` is on each pin when `self_type_generics` is a list",
                )
                .to_compile_error()
                .wrap_err();
            }
            pins.iter()
                .map(|pin| (pin.args.clone(), pin.parent_type.clone()))
                .collect::<Vec<_>>()
                .wrap_ok()
        }
    }
}
```

The `None` branch’s `<>` is wrong for a generic struct (live uses `split_for_impl` and empty map). Keep the live no-pin path as it is: one impl, `parent_type` required, `ty_generics` from `split_for_impl`. Only `One` and `Pins` go through the pin loop.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let output = match pins {
        None => emit_one_impl(
            /* live path: impl_generics, ty_generics from split_for_impl, parent_type required */
        ),
        Some(pins) => {
            let mut impls = Vec::new();
            for (args, parent_type) in pins {
                let generics_map =
                    validate_and_map_generics(input_generics.clone(), args.clone().wrap_some())?;
                let field_infos = /* same field walk, generics_map */;
                impls.push(emit_one_impl(
                    /* impl_generics empty, ty_generics = args, parent_type from the pin */
                ));
            }
            quote!(#(#impls)*)
        }
    };
```

`emit_one_impl` is the current `handle_data_struct` body after `field_infos` is known: predicates, unmatched, the `impl` quote.

Parsing the list: a `syn::Expr::Array` whose elements are tuples `(args, parent_type)`. `args` is `Expr::Path` or a type wrapped as `<A, B>` parsed as `syn::Type::Path` with angle-bracketed args on a dummy, or parse each tuple’s first element as `syn::AngleBracketedGenericArguments` via `syn::parse2`.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn parse_self_type_generics(
    attr: &syn::ParseBuffer,
) -> Result<SelfTypeGenerics, syn::Error> {
    if attr.peek(syn::token::Lt) {
        let one = attr.parse::<syn::AngleBracketedGenericArguments>()?;
        return SelfTypeGenerics::One(one).wrap_ok();
    }
    let content;
    syn::bracketed!(content in attr);
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
    SelfTypeGenerics::Pins(pins).wrap_ok()
}
```

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass. Singleton still uses `One`. Slot is not converted yet.

## Change 3: root pin list, `from_path`, `IsoLiteralSlot`

Origin: `Slot` in `chunk.rs` after change 2. Delta: list form with the root pin, `on_unmatched_span = from_path`, `SlotPath` → `IsoLiteralSlotPath`, `Slot` → `IsoLiteralSlot`, the `From` impl.

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

`cargo test -p resolve_position_macros` passes.

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
2. Change 2: list form of `self_type_generics`, one impl per pin, `One` still uses container `parent_type`. Singleton unchanged. Two-pin macro test. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
3. Change 3: root pin list on `Slot`, `from_path`, `From<IsoLiteralSlotPath>`, `IsoLiteralSlot`, `SlotPath` renamed, gap test. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
4. Move this doc to refactors/past.
