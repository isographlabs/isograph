# Slot stages: `require_complete` to `ArtifactGenerationStage`

Follow-up after the parsing series. The series parses `IsoLiteralParse` / `IsoLiteralSlot`, the optimistic tree with `Option<WithSpan<_>>` written in. This step introduces `Stage` and converts to `IsoLiteralParse<ArtifactGenerationStage>` when `push_error` was never called and every extra is empty.

Parse builds `IsoLiteralParse`. Artifact generation runs on `IsoLiteralParse<ArtifactGenerationStage>`. Resolve walks the optimistic tree only.

## The slot

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk's parse result. Combinator only; does not impl ResolvePosition
/// until slot-singleton-resolve.md lands.
pub struct Slot<T, E> {
    pub item: T,
    pub extra_tokens: E,
}

pub struct Singleton<T, E> {
    pub item: T,
    pub extra_chunks: E,
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
    type IsoLiteral = Option<WithSpan<IsoLiteralItem>>;
    type UnparsedTokens = Option<WithSpan<UnparsedChunkItems>>;
    type ExtraChunks = Option<WithSpan<ExtraChunks>>;
}

impl Stage for ArtifactGenerationStage {
    type IsoLiteral = IsoLiteralItem;
    type UnparsedTokens = ();
    type ExtraChunks = ();
}
```

`parse_one_item` already returns `Slot<Option<WithSpan<P>>, Option<WithSpan<UnparsedChunkItems>>>`. This step does not change it.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: WithSpan<Slot<Option<WithSpan<T>>, Option<WithSpan<UnparsedChunkItems>>>>,
) -> Option<WithSpan<T>> {
    match (slot.item.item, slot.item.extra_tokens) {
        (Some(item), None) => item.wrap_some(),
        _ => None,
    }
}
```

`require_complete` is `None` on leftover or on a form `Err`. Artifact generation calls it on every slot. If any slot is `None`, artifact generation does not run. The artifact value is `WithSpan<T>`, not a `Slot`.

## Tree types

A type that contains a group is generic over `Stage`. That includes a selection set, an argument list, an object literal, a `[...]` type, and a scalar selection (it may hold an argument list). Feature docs write those types.

`IsoLiteralParse` is a concrete root singleton so resolve has a named type to parent at. It goes away when slot-singleton-resolve.md lands. `IsoLiteralItem` takes a stage parameter when a variant holds a type that contains a group.

## Tree convert

Nested lists have the same function per holder. Each maps `require_complete` over its slots.

`parse_iso_literal` returns `IsoLiteralParse`. The caller that generates artifacts calls `require_complete_literal` after checking that `push_error` was never invoked. Extra root chunks still mean the literal is not artifact-ready; `require_complete_literal` is `None` when `extra` is `Some`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: IsoLiteralParse,
) -> Option<IsoLiteralParse<ArtifactGenerationStage>> {
    if parse.extra_chunks.is_some() {
        return None;
    }
    if parse.item.item.extra_tokens.is_some() {
        return None;
    }
    let location = parse.item.location;
    let item = parse.item.item.item?;
    IsoLiteralParse {
        item: IsoLiteralSlot {
            item: item.item,
            extra_tokens: (),
        }
        .with_span(location),
        extra_chunks: (),
    }
    .wrap_some()
}
```

Empty (`item: None`) is not artifact-ready. A parsed first slot with no extra becomes `WithSpan<IsoLiteralItem>`.

## Resolve

Resolve walks `IsoLiteralParse` only. The artifact tree is not resolved.

## Deleted types

The series' `IsoLiteralParse` / `IsoLiteralSlot` are the optimistic types. This step adds `Stage` and the artifact monomorph.

## Shipping

Lands after the parsing series. One step: `require_complete` / `require_complete_literal` and the nested converts on every `T<S>` that contains a group. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
