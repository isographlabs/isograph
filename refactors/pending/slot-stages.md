# Slot stages: `Option<T>` plus `ExtraTokens`, then `T` plus `()`

Follow-up after the parsing series and resolve-position-generic-slot.md. Replaces `LevelSlot` / `Both` / `Failed` with `Slot<Item, Extra>`. Form types (`EntrypointDeclaration`, `Selection`, …) stay concrete. Every type that stores a slot is generic over a stage.

Parse builds `IsoLiteralParse<Initial>`. Artifact generation runs on `IsoLiteralParse<Artifact>`, produced by `require_complete` when `push_error` was never called and every slot has an item and empty extra.

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
    fn item<'a, T: 'a>(item: &'a Self::Item<T>) -> Self::ItemRef<'a, T>;
    fn extra(extra: &Self::Extra) -> Option<&UnparsedChunkItems>;
}

pub struct Initial;

pub struct Artifact;

/// Remaining unparsed items in the chunk. `items` is `None` when the form consumed the chunk.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ExtraTokens {
    #[resolve_field]
    pub items: Option<WithSpan<UnparsedChunkItems>>,
}

impl Stage for Initial {
    type Item<T> = Option<WithSpan<T>>;
    type Extra = ExtraTokens;
    type ItemRef<'a, T: 'a> = Option<&'a T>;
    fn item<'a, T: 'a>(item: &'a Option<WithSpan<T>>) -> Option<&'a T> {
        item.as_ref().map(|wrapped| wrapped.item.reference())
    }
    fn extra(extra: &ExtraTokens) -> Option<&UnparsedChunkItems> {
        extra.items.as_ref().map(|wrapped| wrapped.item.reference())
    }
}

impl Stage for Artifact {
    type Item<T> = WithSpan<T>;
    type Extra = ();
    type ItemRef<'a, T: 'a> = &'a T;
    fn item<'a, T: 'a>(item: &'a WithSpan<T>) -> &'a T {
        item.item.reference()
    }
    fn extra(_: &()) -> Option<&UnparsedChunkItems> {
        None
    }
}

pub type InitialSlot<T> = Slot<Option<WithSpan<T>>, ExtraTokens>;

pub type ArtifactSlot<T> = Slot<WithSpan<T>, ()>;
```

`Initial` `item()` is `Option<&T>`. `Artifact` `item()` is `&T`. Artifact code does not unwrap.

`None` plus empty `ExtraTokens` is representable and unused. `parse_one_item` never builds it: a form `Err` always clones the source chunk's items into `extra`.

## `item` / `remaining`

```rust
// from crates/isograph_parser/src/chunk.rs
impl<T> Slot<Option<WithSpan<T>>, ExtraTokens> {
    pub fn item(&self) -> Option<&T> {
        Initial::item(&self.item)
    }

    pub fn remaining(&self) -> Option<&UnparsedChunkItems> {
        Initial::extra(&self.extra)
    }
}

impl<T> Slot<WithSpan<T>, ()> {
    pub fn item(&self) -> &T {
        Artifact::item(&self.item)
    }
}
```

## `parse_one_item`

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover_error: impl FnOnce(&mut ItemCursor<'a>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<LevelSlot<P>>
where
    F: FnMut(WithSpan<ParseError>),
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover_error: impl FnOnce(&mut ItemCursor<'a>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<InitialSlot<P>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, |cursor| parse(cursor, push_error));
    match result {
        Ok(item) => {
            if stream.require_end().is_ok() {
                return WithSpan::new(
                    Slot {
                        item: item.wrap_some(),
                        extra: ExtraTokens { items: None },
                    },
                    item.location,
                );
            }
            push_error(leftover_error(stream.cursor()));
            match stream.remaining_contents() {
                Some(remaining) => {
                    let leftover_span =
                        Span::join(remaining.first().location, remaining.last().location);
                    let location = Span::join(item.location, leftover_span);
                    WithSpan::new(
                        Slot {
                            item: item.wrap_some(),
                            extra: ExtraTokens {
                                items: WithSpan::new(
                                    UnparsedChunkItems { items: remaining },
                                    leftover_span,
                                )
                                .wrap_some(),
                            },
                        },
                        location,
                    )
                }
                None => WithSpan::new(
                    Slot {
                        item: item.wrap_some(),
                        extra: ExtraTokens { items: None },
                    },
                    item.location,
                ),
            }
        }
        Err(reason) => {
            push_error(reason);
            let location = chunk.item.contents_span();
            WithSpan::new(
                Slot {
                    item: None,
                    extra: ExtraTokens {
                        items: WithSpan::new(
                            UnparsedChunkItems {
                                items: chunk.item.contents.clone(),
                            },
                            location,
                        )
                        .wrap_some(),
                    },
                },
                location,
            )
        }
    }
}
```

`entrypoint Foo.$ asdf` is form `Err` at `$`. `item` is `None`. `extra.items` is the whole chunk `entrypoint Foo.$ asdf`.

## Convert

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn require_complete<T>(
    slot: WithSpan<InitialSlot<T>>,
) -> Option<WithSpan<ArtifactSlot<T>>> {
    let location = slot.location;
    let Slot { item, extra } = slot.item;
    match (item, extra.items) {
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
    pub first: Option<WithSpan<RootSlot>>,
    pub extra: Option<ExtraChunks>,
}

// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(pub Vec<WithSpan<SelectionSlot>>);
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

`RootSlot`, `BothRoot`, `BothSelection`, `SelectionSlot`, `Both`, `Failed`, and `LevelSlot` are deleted. `FailedParent` is deleted. `UnparsedChunkItems` sits on `ExtraTokens`. `ExtraTokens` has one parent, the slot. `ExtraTokensPath` is `PositionResolutionPath<&'a ExtraTokens, SlotPath<'a>>`.

## Tree convert

Nested lists have the same function per holder (`require_complete_selection_set`, …). Each maps `require_complete` over its slots and maps the item through the matching convert. `ObjectSelection<Initial>` becomes `ObjectSelection<Artifact>` only when the nested `SelectionSet` converts.

`parse_iso_literal` returns `IsoLiteralParse<Initial>`. The caller that generates artifacts calls `require_complete_literal` after checking that `push_error` was never invoked. Extra root chunks (`ExtraChunks`) still mean the literal is not artifact-ready; `require_complete_literal` is `None` when `extra` is `Some`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(
    parse: IsoLiteralParse<Initial>,
) -> Option<IsoLiteralParse<Artifact>> {
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

`Slot<Item, Extra>` derives `ResolvePosition` once `resolve-position-generic-slot.md` can emit a generic struct. `Initial` walks `item` when `Some` and `extra.items` when `Some`. `Artifact` walks `item` only. `()` has no `resolve_field`.

## Deleted types

`LevelSlot`, `Both`, `Failed`, `FailedParent`, `RootSlot`, `BothRoot`, `SelectionSlot`, `BothSelection`, and the other concrete slot copies. `item()` / `remaining()` on those types move to `Slot` plus `Stage`.

## Shipping

Lands after the parsing series and resolve-position-generic-slot.md. One step: `Stage`, `Initial`, `Artifact`, `Slot`, `ExtraTokens`, `parse_one_item` / `parse_items` / `parse_singleton` return `InitialSlot`, `require_complete` / `require_complete_literal` and the nested converts, the `<S>` parameter on every slot-holding type, delete the three-arm slot types. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
