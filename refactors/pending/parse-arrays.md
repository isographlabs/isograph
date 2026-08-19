# parse-arrays: `[ ... ]` list values

Lands after parse-arguments.md. Each contentful chunk is a value. Leftover is `Expectation::Separator(BracketKind::Bracket)`. parse-variables.md reuses this for constant defaults.

## Grammar

```
[ <value>, <value>, ... ]
```

A value is whatever `parse_value` already accepts, including nested lists.

## Types

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteral(
    #[resolve_field]
    #[parent_variant(List)]
    pub Vec<WithSpan<Slot<NonConstantValue, UnparsedChunkItems>>>,
);

pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringValue),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
    List(ListLiteral),
}

pub enum NonConstantValueParent<'a> {
    KeyValue(Box<KeyValuePairPath<'a>>),
    List(Box<ListLiteralPath<'a>>),
}

pub type ListLiteralPath<'a> = PositionResolutionPath<&'a ListLiteral, NonConstantValueParent<'a>>;

pub type ListValueSlotPath<'a> = PositionResolutionPath<
    &'a Slot<NonConstantValue, UnparsedChunkItems>,
    NonConstantValueParent<'a>,
>;
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    ListLiteral(ListLiteralPath<'a>),
}

impl<'a> From<NonConstantValueParent<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: NonConstantValueParent<'a>) -> Self {
        match parent {
            NonConstantValueParent::KeyValue(path) => path.parent.to(),
            NonConstantValueParent::List(path) => {
                UnparsedChunkItemsParent::ListLiteral(path.dereference())
            }
        }
    }
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
impl<'a> From<ListValueSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ListValueSlotPath<'a>) -> Self {
        IsographResolutionNode::ListValueSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ListLiteral(ListLiteralPath<'a>),
    ListValueSlot(ListValueSlotPath<'a>),
```

`parse_value` is `spanning` around a `parse_value_item` that returns `NonConstantValue`. The array arm and `parse_chunk_item_list` both call `parse_value_item`.

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(group) = cursor.consume_group_if(BracketKind::Bracket) {
            return NonConstantValue::List(ListLiteral(
                group.item.children.item.parse_chunk_item_list(
                    cursor.text(),
                    Expectation::Separator(BracketKind::Bracket),
                    parse_value_item,
                    push_error,
                ),
            ))
            .wrap_ok();
        }
```

## Tests

Feed list interiors with leftover `Separator(BracketKind::Bracket)` and `parse_value_item`.

- `[1, $x, true]` is three values
- `[[1], { a: 2 }]` nests
- `[]` and whitespace-only interiors are empty
- trailing comma is not a diagnostic
- leftover after a value keeps the item
- a doubled comma is chunking's error

## Landing checklist

1. `ListLiteral`, `NonConstantValue::List`, `parse_value_item`, the `From` impls, the tests. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
