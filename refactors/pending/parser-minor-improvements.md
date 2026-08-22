# Parser minor improvements

These are not in the grammar-stage order. Do not mix them into type-annotation-null.md, parse-iso-literal-entry.md, or type-annotation-union.md. Each heading is independently shippable.

## `Slot<T, E>` is two independent `Option`s

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<T, E> {
    pub item: Option<WithSpan<T>>,
    pub extra: Option<WithSpan<E>>,
}
```

`parse_one_chunk` produces three states:

- complete: `item: Some`, `extra: None`
- complete with leftover: `item: Some`, `extra: Some`
- failed: `item: None`, `extra: Some` (unread remainder in extra after leftover-in-extra.md)

`item: None, extra: None` is representable and never built. That is a bool-plus-spare-field. `Slot` becomes an enum with those three variants.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum Slot<T, E> {
    Complete(WithSpan<T>),
    CompleteWithLeftover(CompleteWithLeftover<T, E>),
    Failed(WithSpan<E>),
}

pub struct CompleteWithLeftover<T, E> {
    pub item: WithSpan<T>,
    pub extra: WithSpan<E>,
}
```

Call sites that read `slot.item` / `slot.extra` match on the enum. `require_complete` in slot-stages.md is `Slot::Complete`. Resolve pins that currently name `Slot<T, UnparsedChunkItems>` stay on this enum; `Complete` walks `T`, `CompleteWithLeftover` walks `item` and `extra`, `Failed` walks extra.

`ListTypeAnnotation` repeats the same pair (`inner: Option`, `extra: Option`) instead of being a `Slot<TypeAnnotation, UnparsedChunkItems>`. Empty `[]` fails the whole annotation (and therefore the host declaration). `[42]` succeeds as `List { inner: None, extra: Some(...) }`. Same shape, two recovery policies. After this change `ListTypeAnnotation` is `Slot<TypeAnnotation, UnparsedChunkItems>`. Empty `[]` stays a failed annotation. `[42]` is `Slot::Failed`.

## `parse_bracket_interior_type` indexes chunk 0

```rust
// from crates/isograph_parser/src/variables.rs
        &level.item.0[0],
        cursor.stream_chunk(&level.item.0[0].item),
```

`ChunkedLevel` is a `Vec`. Empty is legal: an empty `[]` type. `parse_bracket_interior_type` checks `len() == 0` first. The unpartitioned empty check is `split_first` on the vec, or `consume_line_breaks` then `require_end` for only line breaks.

## `EndOfFile` is never emitted

`tokenize` stops at the last real token. End of input at parse time is `Found::EndOfChunk`. Delete `EndOfFile` from `IsographLangTokenKind` and `NonBracketTokenKind`, and the `From` / `Display` / `SplitToken` arms. leftover-semantic-tokens.md's `leftover_token` maps `EndOfFile` to `None`; that arm goes with the variant.

## `Expectation::Description` and `Expectation::SelectionSet`

These two variants are never passed to `cursor.expected`. Descriptions and selection sets are optional. Delete the two variants and their `Display` arms. The `OneOf` display tests in `parse_error.rs` use `Description` and `SelectionSet`; those tests use `Keyword` and `Selection` instead. `Selection` is used in production.

## `Expectation` is not an `Error`

`AstError` is the AST-stage error. `Expectation` is a fragment of `ExpectedFound`. Pipeline `ParseError` wraps `AstError`.

Before:

```rust
// from crates/isograph_parser/src/parse_error.rs
impl std::error::Error for Expectation {}
```

After: that impl is gone. `AstError` stays `thiserror`. `Found` stays `strum::Display`. `Expectation` keeps its handwritten `Display`: `OneOf` is `write_one_of`, and `Separator` uses `kind.closing()`, neither of which is a per-variant strum string.

## Keyword-as-identifier is copy-pasted

`entrypoint` / `field`, `to`, and `true` / `false` / `null` all require an `Identifier` and then match the source slice.

`consume_to_target` peeks, compares to `"to"`, then `require_token` with `Keyword`. That records only on match.

`parse_boolean_or_null` records `BooleanOrNull` before checking the word, so `a: yes` highlights `yes` as boolean/null and then errors.

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_boolean_or_null(
    cursor: &mut ItemCursor<'_>,
) -> Result<NonConstantValue, WithSpan<AstError>> {
    let span = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            SemanticToken::BooleanOrNull,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    match span.text() {
        "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
        "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
        "null" => NonConstantValue::Null(NullValue).wrap_ok(),
        _ => AstError::expected(
            Expectation::Value,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(span.location)
        .wrap_err(),
    }
}
```

A `consume_keyword` that records only on match removes the duplication and the bad highlight.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_keyword(
        &mut self,
        word: &'static str,
        token: SemanticToken,
    ) -> Option<TokenText<'a>> {
        let text = self.text();
        let peek = self.peek()?;
        let item = peek.view();
        match item.item.reference() {
            ChunkContentItem::NonBracket(found) if found.0 == NonBracketTokenKind::Identifier => {}
            _ => return None,
        }
        let location = item.location;
        if &text[location.as_usize_range()] != word {
            return None;
        }
        peek.commit(token);
        TokenText {
            location,
            text,
        }
        .wrap_some()
    }
```

`parse_boolean_or_null` after:

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_boolean_or_null(
    cursor: &mut ItemCursor<'_>,
) -> Result<NonConstantValue, WithSpan<AstError>> {
    if cursor
        .consume_keyword("true", SemanticToken::BooleanOrNull)
        .is_some()
    {
        return NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok();
    }
    if cursor
        .consume_keyword("false", SemanticToken::BooleanOrNull)
        .is_some()
    {
        return NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok();
    }
    if cursor
        .consume_keyword("null", SemanticToken::BooleanOrNull)
        .is_some()
    {
        return NonConstantValue::Null(NullValue).wrap_ok();
    }
    cursor.expected(Expectation::Value).wrap_err()
}
```

`parse_non_constant_value` only calls this when peek is `Identifier`, so `Expectation::Value` at `yes` is the same error as today. No token is recorded, so leftover fill-in highlights `yes` as Content.

`consume_to_target` after:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn consume_to_target(
    cursor: &mut ItemCursor<'_>,
) -> Result<Option<WithSpan<TypeAnnotation>>, WithSpan<AstError>> {
    if cursor
        .consume_keyword("to", SemanticToken::Keyword)
        .is_none()
    {
        return None.wrap_ok();
    }
    let target_type = parse_type_annotation(cursor)?;
    target_type.wrap_some().wrap_ok()
}
```

leftover-semantic-tokens.md records `Keyword` at `fieldd`; `parse_iso_literal_item` keeps require-then-match so that identifier stays `Keyword`.

## `UnionVariant::Null` is a unit

type-annotation-union.md uses `Null(NullTypeAnnotation)`, a ZST, so the all-delegate `ResolvePosition` derive compiles. The row's location is `None`; resolve is never called.

`Null` is a unit variant. The derive then allows mixed enums: `Named` and `List` continue into the payload, `Null` is unmarked and answers `UnionTypeAnnotation`. Parked design: `refactors/past/resolve-option-like-enums.md`. Depends on type-annotation-union.md.
