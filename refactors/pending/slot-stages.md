# Slot stages: `require_complete` to `ArtifactGenerationStage`

Follow-up after the parsing series. Parse builds `IsoLiteralParse`. Artifact generation runs on a tree with no leftover and no extra. Resolve walks the optimistic tree only.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: &Slot<T, UnparsedChunkItems>,
) -> Option<&T> {
    match (slot.item.reference(), slot.extra_tokens.reference()) {
        (Some(item), None) => item.item.reference().wrap_some(),
        _ => None,
    }
}
```

`require_complete` is `None` on leftover or on a form `Err`. Artifact generation calls it on every slot. If any slot is `None`, artifact generation does not run.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: &IsoLiteralParse,
) -> Option<&IsoLiteralItem> {
    if parse.extra_chunks.is_some() {
        return None;
    }
    require_complete(parse.item.item.reference())
}
```

Empty (`parse_iso_literal` returned `None`) is not artifact-ready. A parsed first slot with no extra is `&IsoLiteralItem`.

A type that contains a group maps `require_complete` over its slots. Feature docs that introduce a group write that map: `SelectionSet`, `ArgumentList`, `ObjectLiteral`, `VariableDeclarationList`, `ListTypeAnnotation`. Nested lists convert inner first.

## Stage

A later change can make those group types generic over a `Stage` associated type so the artifact tree cannot represent leftover. This step does not introduce `Stage`. The convert functions return the payload (`&T`) or `None`. The caller that generates artifacts holds the optimistic tree and the converted payloads separately, or rebuilds a narrower tree in a follow-up.

## Resolve

Resolve walks `IsoLiteralParse` only. The converted payloads are not resolved.

## Shipping

Lands after the parsing series. One step: `require_complete` / `require_complete_literal` and the nested converts on every type that contains a group. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
