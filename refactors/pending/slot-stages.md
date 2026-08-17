# Slot stages: `require_complete` to `ArtifactGenerationStage`

Follow-up after the parsing series. The series already parses `IsoLiteralParse<OptimisticStage>`. `Stage` is a bag of associated types. This step converts an optimistic tree to `IsoLiteralParse<ArtifactGenerationStage>` when `push_error` was never called and every extra is empty.

Parse builds `IsoLiteralParse<OptimisticStage>`. Artifact generation runs on `IsoLiteralParse<ArtifactGenerationStage>`. Resolve walks the optimistic tree only.

## The slot

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk's parse result. Combinator only; does not impl ResolvePosition
/// until resolve-position-generic-slot.md lands.
pub struct Slot<T, E> {
    pub item: T,
    pub extra: E,
}

pub struct Singleton<T, E> {
    pub item: T,
    pub extra: E,
}

/// Associated types are the entrypoint root's fail-able pieces; feature docs
/// add one when they introduce a new one.
pub trait Stage {
    type IsoLiteral;
    type UnparsedTokens;
    type ExtraChunks;
}

pub struct OptimisticStage;

pub struct ArtifactGenerationStage;

impl Stage for OptimisticStage {
    type IsoLiteral = Option<IsoLiteralItem>;
    type UnparsedTokens = Option<WithSpan<UnparsedChunkItems>>;
    type ExtraChunks = Option<WithSpan<ExtraChunks>>;
}

impl Stage for ArtifactGenerationStage {
    type IsoLiteral = IsoLiteralItem;
    type UnparsedTokens = ();
    type ExtraChunks = ();
}
```

`parse_one_item` already returns `Slot<Option<P>, Option<WithSpan<UnparsedChunkItems>>>`. This step does not change it.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: WithSpan<Slot<Option<T>, Option<WithSpan<UnparsedChunkItems>>>>,
) -> Option<WithSpan<T>> {
    match (slot.item.item, slot.item.extra) {
        (Some(item), None) => WithSpan::new(item, slot.location).wrap_some(),
        _ => None,
    }
}
```

`require_complete` is `None` on leftover or on a form `Err`. Artifact generation calls it on every slot. If any slot is `None`, artifact generation does not run. The artifact value is `WithSpan<T>`, not a `Slot`.

## Tree types

A type that contains a group is generic over `Stage`. That includes a selection set, an argument list, an object literal, a `[...]` type, and a scalar selection (it may hold an argument list). Feature docs write those types.

`IsoLiteralParse` is a concrete root singleton so resolve has a named type to parent at. It goes away when resolve-position-generic-slot.md lands. `IsoLiteralItem` takes a stage parameter when a variant holds a type that contains a group.

## Tree convert

Nested lists have the same function per holder. Each maps `require_complete` over its slots.

`parse_iso_literal` returns `IsoLiteralParse<OptimisticStage>`. The caller that generates artifacts calls `require_complete_literal` after checking that `push_error` was never invoked. Extra root chunks (`S::ExtraChunks`) still mean the literal is not artifact-ready; `require_complete_literal` is `None` when `extra` is `Some`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: IsoLiteralParse<OptimisticStage>,
) -> Option<IsoLiteralParse<ArtifactGenerationStage>> {
    if parse.extra.is_some() {
        return None;
    }
    if parse.item.item.extra.is_some() {
        return None;
    }
    let location = parse.item.location;
    let item = parse.item.item.item.item?;
    IsoLiteralParse {
        item: WithSpan::new(
            IsoLiteralSlot {
                item: WithSpan::new(item, parse.item.item.item.location),
                extra: (),
            },
            location,
        ),
        extra: None,
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

Lands after the parsing series. One step: `require_complete` / `require_complete_literal` and the nested converts on every `T<S>` that contains a group. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
