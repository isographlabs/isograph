# parse-arrays: `[ ... ]` list values

Lands after parse-directives.md. Each contentful chunk is a value. Leftover is `Expectation::Separator(BracketKind::Bracket)`. Until this doc, `[` as a value is `Expected(Value, Group(Bracket))`, including after `=` on a variable default.

Upstream `parse_non_constant_value` does not parse lists. The `NonConstantValue::List` variant exists on the isograph type and is unused by that parser. This doc adds the form.

## Grammar

```
[ <value>, <value>, ... ]
```

A value is whatever `parse_non_constant_value` already accepts, including nested lists.

## Types

`NonConstantValue` is a field of `SelectionFieldArgument`, `ObjectEntry`, and `VariableDeclarationOrUsage`. A vanilla `Slot<T, E>` pin's `T::Parent` is the slot path, so `NonConstantValue` cannot be that `T`. The slot item is a wrapper whose only job is to own the list-element parent, the same split as `SelectionFieldArgument` / `ObjectEntry`.

Origin: `NonConstantValueInner::List` in `crates/isograph_lang_types/src/declarations/selection_argument.rs`. Delta: the wrapper and the `Slot`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteral(
    #[resolve_field] pub Vec<WithSpan<Slot<ListLiteralValue, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ListLiteralValueSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteralValue {
    #[resolve_field]
    #[parent_variant(List)]
    pub value: WithSpan<NonConstantValue>,
}

pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringLiteralValueWrapper),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
    List(ListLiteral),
}

```

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum NonConstantValueParent<'a> {
    SelectionFieldArgument(Box<SelectionFieldArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
    VariableDefault(VariableDeclarationOrUsagePath<'a>),
}
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum NonConstantValueParent<'a> {
    SelectionFieldArgument(Box<SelectionFieldArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
    VariableDefault(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListLiteralValuePath<'a>>),
}

pub type ListLiteralPath<'a> = PositionResolutionPath<&'a ListLiteral, NonConstantValueParent<'a>>;

pub type ListLiteralValueSlotPath<'a> = PositionResolutionPath<
    &'a Slot<ListLiteralValue, UnparsedChunkItems>,
    ListLiteralPath<'a>,
>;

pub type ListLiteralValuePath<'a> =
    PositionResolutionPath<&'a ListLiteralValue, ListLiteralValueSlotPath<'a>>;
```

`ListLiteralValue` is not an isograph type. The slot needs a `T` that is not `NonConstantValue`.

```rust
// from crates/isograph_parser/src/chunk.rs
    pins = [
        // existing pins...,
        (<ListLiteralValue, UnparsedChunkItems>, ListLiteralPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
    ListLiteralValueSlot(ListLiteralValueSlotPath<'a>),
```

`From<ListLiteralValueSlotPath>` for `UnparsedChunkItemsParent` and for `IsographResolutionNode`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ListLiteral(ListLiteralPath<'a>),
    ListLiteralValue(ListLiteralValuePath<'a>),
    ListLiteralValueSlot(ListLiteralValueSlotPath<'a>),
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(list) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::Bracket,
            |cursor, children| {
                ListLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Bracket),
                    parse_list_literal_value,
                ))
            },
        ) {
            return NonConstantValue::List(list.item).wrap_ok();
        }
```

`SemanticToken::Bracket` is leftover-fill-in in semantic-tokens.md. A grammar-consumed `[...]` value needs a role. Use `SemanticToken::Bracket` here and amend semantic-tokens.md: leftover-only no longer applies to list-value brackets (type-list brackets stay `GraphQLTypeName`).

```rust
fn parse_list_literal_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<ListLiteralValue, WithSpan<ParseError>> {
    let value = parse_non_constant_value(cursor)?;
    ListLiteralValue { value }.wrap_ok()
}
```

`parse_non_constant_value` stays the spanning ladder. `parse_list_literal_value` calls it; leftover is the slot's, not the value's. Defaults share this function, so list defaults land with list argument values.

## Tests

Feed list interiors with leftover `Separator(BracketKind::Bracket)` and `parse_list_literal_value`, and feed `parse_non_constant_value` on a chunk that is a list.

- `[1, $x, true]` is three values
- `[[1], { a: 2 }]` nests
- `[]` and whitespace-only interiors are empty
- a trailing comma is not a diagnostic
- leftover after a value keeps the item
- a doubled comma is chunking's error
- `id: [1, 2]` as an argument value
- `$ids: [ID!] = [1, $x]` as a default

## Landing checklist

1. `ListLiteral`, `ListLiteralValue`, `NonConstantValue::List`, the pin, the `From` impls, the tests, the semantic-tokens.md sentence. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
