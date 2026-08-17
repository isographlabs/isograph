# Slot stages: `Option<T>` plus `ExtraTokens`, then `T` plus `()`

Follow-up after the parsing series and resolve-position-generic-slot.md. The series already parses `OptimisticSlot<T>` (`Slot<Option<WithSpan<T>>, ExtraTokens>`). This step adds `require_complete` to `ArtifactGenerationSlot<T>` (`Slot<WithSpan<T>, ()>`) and makes every slot-holding type generic over `S: Stage`. Form types (`EntrypointDeclaration`, `SelectionName`, …) stay concrete.

Parse builds `IsoLiteralParse<OptimisticStage>`. Artifact generation runs on `IsoLiteralParse<ArtifactGenerationStage>`, produced by `require_complete` when `push_error` was never called and every slot has an item and empty extra.

## The slot

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<Item, Extra> {
    pub item: Item,
    pub extra: Extra,
}

pub trait Stage {
    type Item<T>;
    type Extra;
    type ItemRef<'a, T: 'a>;
    type ExtraRef<'a>;
    fn item<'a, T: 'a>(item: &'a Self::Item<T>) -> Self::ItemRef<'a, T>;
    fn extra<'a>(extra: &'a Self::Extra) -> Self::ExtraRef<'a>;
}

pub struct OptimisticStage;

pub struct ArtifactGenerationStage;

/// Remaining unparsed items in the chunk. `None` when the form consumed the chunk.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ExtraTokens(#[resolve_field] pub Option<WithSpan<UnparsedChunkItems>>);

impl Stage for OptimisticStage {
    type Item<T> = Option<WithSpan<T>>;
    type Extra = ExtraTokens;
    type ItemRef<'a, T: 'a> = Option<&'a T>;
    type ExtraRef<'a> = Option<&'a UnparsedChunkItems>;
    fn item<'a, T: 'a>(item: &'a Option<WithSpan<T>>) -> Option<&'a T> {
        item.as_ref().map(|wrapped| wrapped.item.reference())
    }
    fn extra<'a>(extra: &'a ExtraTokens) -> Option<&'a UnparsedChunkItems> {
        extra.0.as_ref().map(|wrapped| wrapped.item.reference())
    }
}

impl Stage for ArtifactGenerationStage {
    type Item<T> = WithSpan<T>;
    type Extra = ();
    type ItemRef<'a, T: 'a> = &'a T;
    type ExtraRef<'a> = ();
    fn item<'a, T: 'a>(item: &'a WithSpan<T>) -> &'a T {
        item.item.reference()
    }
    fn extra<'a>(_: &'a ()) {}
}

pub type OptimisticSlot<T> = Slot<Option<WithSpan<T>>, ExtraTokens>;

pub type ArtifactGenerationSlot<T> = Slot<WithSpan<T>, ()>;
```

`OptimisticStage` `item()` is `Option<&T>` and `extra()` is `Option<&UnparsedChunkItems>`. `ArtifactGenerationStage` `item()` is `&T` and `extra()` is `()`. Artifact code does not unwrap.

`None` plus empty `ExtraTokens` is representable and unused. `parse_one_item` never builds it: a form `Err` always clones the source chunk's items into `extra`.

## `item` / `remaining`

```rust
// from crates/isograph_parser/src/chunk.rs
impl<T> Slot<Option<WithSpan<T>>, ExtraTokens> {
    pub fn item(&self) -> Option<&T> {
        OptimisticStage::item(&self.item)
    }

    pub fn remaining(&self) -> Option<&UnparsedChunkItems> {
        OptimisticStage::extra(&self.extra)
    }
}

impl<T> Slot<WithSpan<T>, ()> {
    pub fn item(&self) -> &T {
        ArtifactGenerationStage::item(&self.item)
    }

    pub fn remaining(&self) {
        ArtifactGenerationStage::extra(&self.extra)
    }
}
```

`parse_one_item` already returns `OptimisticSlot`. This step does not change it.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: WithSpan<OptimisticSlot<T>>,
) -> Option<WithSpan<ArtifactGenerationSlot<T>>> {
    let location = slot.location;
    let Slot { item, extra } = slot.item;
    match (item, extra.0) {
        (Some(item), None) => WithSpan::new(Slot { item, extra: () }, location).wrap_some(),
        _ => None,
    }
}
```

`require_complete` is `None` on leftover or on a form `Err`. Artifact generation calls it on every slot. If any slot is `None`, artifact generation does not run.

## Tree types

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct IsoLiteralParse {
    pub first: Option<WithSpan<OptimisticSlot<IsoLiteralItem>>>,
    pub extra: Option<ExtraChunks>,
}

// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(pub Vec<WithSpan<OptimisticSlot<Selection>>>);
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct IsoLiteralParse<S: Stage> {
    #[resolve_field]
    pub first: Option<WithSpan<Slot<S::Item<IsoLiteralItem<S>>, S::Extra>>>,
    #[resolve_field]
    pub extra: Option<ExtraChunks>,
}

// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet<S: Stage>(
    #[resolve_field] pub Vec<WithSpan<Slot<S::Item<Selection<S>>, S::Extra>>>,
);

pub enum Selection<S: Stage> {
    Scalar(ScalarSelection<S>),
    Object(ObjectSelection<S>),
}

pub struct ObjectSelection<S: Stage> {
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    pub name: WithSpan<SelectionName>,
    pub arguments: Option<WithSpan<ArgumentList<S>>>,
    pub selection_set: WithSpan<SelectionSet<S>>,
}
```

Every type that contains a slot takes `S`. Form payloads that contain no slot (`EntityName`, `EntrypointKeyword`, `SelectionName`) stay unparameterized. `EntrypointDeclaration` takes `S` only if a later field is a slot.

`IsoLiteralItem<S>`, `ArgumentList<S>`, `ObjectLiteral<S>`, `Singleton<S, T>`, and the other list holders are the same `S` parameter.

`OptimisticSlot<T>` on the tree becomes `Slot<S::Item<T>, S::Extra>`. `IsoLiteralParse` and every list holder take `S`.

## Tree convert

Nested lists have the same function per holder (`require_complete_selection_set`, …). Each maps `require_complete` over its slots and maps the item through the matching convert. `ObjectSelection<OptimisticStage>` becomes `ObjectSelection<ArtifactGenerationStage>` only when the nested `SelectionSet` converts.

`parse_iso_literal` returns `IsoLiteralParse<OptimisticStage>`. The caller that generates artifacts calls `require_complete_literal` after checking that `push_error` was never invoked. Extra root chunks (`ExtraChunks`) still mean the literal is not artifact-ready; `require_complete_literal` is `None` when `extra` is `Some`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: IsoLiteralParse<OptimisticStage>,
) -> Option<IsoLiteralParse<ArtifactGenerationStage>> {
    if parse.extra.is_some() {
        return None;
    }
    let first = match parse.first {
        None => None,
        Some(slot) => require_complete(slot)?.wrap_some(),
    };
    IsoLiteralParse {
        first,
        extra: None,
    }
    .wrap_some()
}
```

## Resolve

`Slot<Item, Extra>` derives `ResolvePosition` once `resolve-position-generic-slot.md` can emit a generic struct. `OptimisticStage` walks `item` when `Some` and `extra.0` when `Some`. `ArtifactGenerationStage` walks `item` only. `()` has no `resolve_field`.

## Deleted types

No new slot enum. `Stage` moves from the series' `OptimisticStage` / `ArtifactGenerationStage` impls onto every slot-holding type. The `From<Singleton<IsoLiteralItem>> for IsoLiteralParse` in the series is gone.

## Shipping

Lands after the parsing series and resolve-position-generic-slot.md. One step: `require_complete` / `require_complete_literal` and the nested converts, the `<S>` parameter on every slot-holding type. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
