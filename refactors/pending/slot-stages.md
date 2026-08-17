# Slot stages: `require_complete` to `ArtifactGenerationStage`

Follow-up after the parsing series. The series already parses `IsoLiteralParse<OptimisticStage>`. `Slot<T, E>` and `Singleton<T, E>` take `T` and `E` from `Stage`. This step converts an optimistic tree to `IsoLiteralParse<ArtifactGenerationStage>` when `push_error` was never called and every extra is empty.

Parse builds `IsoLiteralParse<OptimisticStage>`. Artifact generation runs on `IsoLiteralParse<ArtifactGenerationStage>`. Resolve walks the optimistic tree only.

## The slot

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<T, E> {
    pub item: T,
    pub extra: E,
}

pub struct Singleton<T, E> {
    pub item: T,
    pub extra: E,
}

pub trait Stage {
    type Item<T>;
    type Extra<T>;
    type IsoLiteral;
}

pub struct OptimisticStage;

pub struct ArtifactGenerationStage;

impl Stage for OptimisticStage {
    type Item<T> = Option<WithSpan<T>>;
    type Extra<T> = Option<WithSpan<T>>;
    type IsoLiteral = Option<
        WithSpan<
            Slot<
                <OptimisticStage as Stage>::Item<IsoLiteralItem>,
                <OptimisticStage as Stage>::Extra<UnparsedChunkItems>,
            >,
        >,
    >;
}

impl Stage for ArtifactGenerationStage {
    type Item<T> = WithSpan<T>;
    type Extra<T> = ();
    type IsoLiteral = WithSpan<IsoLiteralItem>;
}
```

`parse_one_item` already returns `Slot<OptimisticStage::Item<P>, OptimisticStage::Extra<UnparsedChunkItems>>`. This step does not change it.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: WithSpan<
        Slot<
            <OptimisticStage as Stage>::Item<T>,
            <OptimisticStage as Stage>::Extra<UnparsedChunkItems>,
        >,
    >,
) -> Option<WithSpan<T>> {
    let location = slot.location;
    let Slot { item, extra } = slot.item;
    match (item, extra) {
        (Some(item), None) => item.wrap_some(),
        _ => None,
    }
}
```

`require_complete` is `None` on leftover or on a form `Err`. Artifact generation calls it on every slot. If any slot is `None`, artifact generation does not run. The artifact value is `WithSpan<T>` (`ArtifactGenerationStage::Item<T>`), not a `Slot`.

## Tree types

Before (the series, lists still pinned to `OptimisticStage`):

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(
    pub Vec<
        WithSpan<
            Slot<
                <OptimisticStage as Stage>::Item<Selection>,
                <OptimisticStage as Stage>::Extra<UnparsedChunkItems>,
            >,
        >,
    >,
);
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet<S: Stage>(
    #[resolve_field] pub Vec<WithSpan<S::Item<Selection<S>>>>,
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

Every type that contains a slot-shaped field takes `S`. Form payloads that contain no slot (`EntityName`, `EntrypointKeyword`, `SelectionName`) stay unparameterized. `IsoLiteralItem` takes `S` when a variant holds a slot. `ArgumentList<S>`, `ObjectLiteral<S>`, and the other list holders are the same `S` parameter.

## Tree convert

Nested lists have the same function per holder (`require_complete_selection_set`, …). Each maps `require_complete` over its slots and maps the item through the matching convert. `ObjectSelection<OptimisticStage>` becomes `ObjectSelection<ArtifactGenerationStage>` only when the nested `SelectionSet` converts.

`parse_iso_literal` returns `IsoLiteralParse<OptimisticStage>`. The caller that generates artifacts calls `require_complete_literal` after checking that `push_error` was never invoked. Extra root chunks (`S::Extra<ExtraChunks>`) still mean the literal is not artifact-ready; `require_complete_literal` is `None` when `extra` is `Some`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: IsoLiteralParse<OptimisticStage>,
) -> Option<IsoLiteralParse<ArtifactGenerationStage>> {
    if parse.extra.is_some() {
        return None;
    }
    let item = match parse.item {
        None => return None,
        Some(slot) => require_complete(slot)?,
    };
    IsoLiteralParse {
        item,
        extra: (),
    }
    .wrap_some()
}
```

Empty (`item: None`) is not artifact-ready. A parsed first slot with no extra becomes `WithSpan<IsoLiteralItem>`.

## Resolve

Resolve walks `IsoLiteralParse<OptimisticStage>` only. The artifact tree is not resolved.

## Deleted types

No new slot enum. The `From<Singleton<...>> for IsoLiteralParse<OptimisticStage>` in the series is gone.

## Shipping

Lands after the parsing series. One step: `require_complete` / `require_complete_literal` and the nested converts, the `<S>` parameter on every remaining slot-holding type. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
