# parse-arrays: `[ ... ]` list values

Lands after parse-pointers.md. Each contentful chunk is a value. Leftover is `Expectation::Separator(BracketKind::Bracket)`. Constant defaults get the same list form.

## Grammar

```
[ <value>, <value>, ... ]
```

A value is whatever `parse_value` already accepts, including nested lists. A constant default is whatever `parse_constant_value` already accepts, including nested lists, still rejecting `$`.

## Types

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum NonConstantValue {
    Variable(VariableUse),
    #[parent_variant(NonConstant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(NonConstant)]
    Integer(IntegerValue),
    #[parent_variant(NonConstant)]
    Boolean(BooleanValue),
    #[parent_variant(NonConstant)]
    Null(NullValue),
    Object(ObjectLiteral),
}

pub enum NonConstantValueParent<'a> {
    NamedArgument(Box<NamedArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
}

pub enum ConstantValue {
    #[parent_variant(Constant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(Constant)]
    Integer(IntegerValue),
    #[parent_variant(Constant)]
    Boolean(BooleanValue),
    #[parent_variant(Constant)]
    Null(NullValue),
    Object(ConstantObjectLiteral),
}

pub enum ConstantValueParent<'a> {
    VariableDefault(DeclaredVariablePath<'a>),
    ConstantObjectEntry(Box<NamedConstantObjectEntryPath<'a>>),
}
```

After. Origin: those listings. Delta: `List` variants, list-literal types, `List` parent variants.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteral(
    #[resolve_field]
    #[parent_variant(List)]
    pub Vec<WithSpan<Slot<NonConstantValue, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ConstantListLiteral(
    #[resolve_field]
    #[parent_variant(List)]
    pub Vec<WithSpan<Slot<ConstantValue, UnparsedChunkItems>>>,
);

pub enum NonConstantValue {
    Variable(VariableUse),
    #[parent_variant(NonConstant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(NonConstant)]
    Integer(IntegerValue),
    #[parent_variant(NonConstant)]
    Boolean(BooleanValue),
    #[parent_variant(NonConstant)]
    Null(NullValue),
    Object(ObjectLiteral),
    List(ListLiteral),
}

pub enum NonConstantValueParent<'a> {
    NamedArgument(Box<NamedArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
    List(Box<ListLiteralPath<'a>>),
}

pub enum ConstantValue {
    #[parent_variant(Constant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(Constant)]
    Integer(IntegerValue),
    #[parent_variant(Constant)]
    Boolean(BooleanValue),
    #[parent_variant(Constant)]
    Null(NullValue),
    Object(ConstantObjectLiteral),
    List(ConstantListLiteral),
}

pub enum ConstantValueParent<'a> {
    VariableDefault(DeclaredVariablePath<'a>),
    ConstantObjectEntry(Box<NamedConstantObjectEntryPath<'a>>),
    List(Box<ConstantListLiteralPath<'a>>),
}

pub type ListLiteralPath<'a> = PositionResolutionPath<&'a ListLiteral, NonConstantValueParent<'a>>;

pub type ListValueSlotPath<'a> = PositionResolutionPath<
    &'a Slot<NonConstantValue, UnparsedChunkItems>,
    NonConstantValueParent<'a>,
>;

pub type ConstantListLiteralPath<'a> =
    PositionResolutionPath<&'a ConstantListLiteral, ConstantValueParent<'a>>;

pub type ConstantListValueSlotPath<'a> = PositionResolutionPath<
    &'a Slot<ConstantValue, UnparsedChunkItems>,
    ConstantValueParent<'a>,
>;
```

Before leftover / pins: the parse-variables after-state. After: two list pins and leftover variants.

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
        (<DeclaredVariable, UnparsedChunkItems>, VariableDeclarationListPath<'a>),
        (<TypeAnnotation, UnparsedChunkItems>, TypeAnnotationParent<'a>),
        (<NamedConstantObjectEntry, UnparsedChunkItems>, ConstantObjectLiteralPath<'a>),
        (<NonConstantValue, UnparsedChunkItems>, NonConstantValueParent<'a>),
        (<ConstantValue, UnparsedChunkItems>, ConstantValueParent<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    TypeAnnotationSlot(TypeAnnotationSlotPath<'a>),
    NamedConstantObjectEntrySlot(NamedConstantObjectEntrySlotPath<'a>),
    ListValueSlot(ListValueSlotPath<'a>),
    ConstantListValueSlot(ConstantListValueSlotPath<'a>),
}

impl<'a> From<ListValueSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ListValueSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ListValueSlot(path)
    }
}

impl<'a> From<ConstantListValueSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ConstantListValueSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ConstantListValueSlot(path)
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

impl<'a> From<ConstantListValueSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ConstantListValueSlotPath<'a>) -> Self {
        IsographResolutionNode::ConstantListValueSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ListLiteral(ListLiteralPath<'a>),
    ListValueSlot(ListValueSlotPath<'a>),
    ConstantListLiteral(ConstantListLiteralPath<'a>),
    ConstantListValueSlot(ConstantListValueSlotPath<'a>),
```

`parse_value` is `spanning` around `parse_value_item`. `parse_constant_value` is `spanning` around `parse_constant_value_item`. List arms and `parse_each_chunk` call the item functions. List-value brackets record `SemanticToken::Bracket`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(parse_value_item)
}

fn parse_value_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    // existing ladder, then:
    if let Some(group) = cursor.consume_group_if(BracketKind::Bracket, SemanticToken::Bracket) {
        let list = ListLiteral(group.item.children.item.parse_each_chunk(
            cursor,
            Expectation::Separator(BracketKind::Bracket),
            parse_value_item,
        ));
        cursor.record_group_close(group.item, SemanticToken::Bracket);
        return NonConstantValue::List(list).wrap_ok();
    }
    cursor.expected(Expectation::Value).wrap_err()
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(group) =
            cursor.consume_group_if(BracketKind::Bracket, SemanticToken::Bracket)
        {
            let list = ConstantListLiteral(group.item.children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Bracket),
                parse_constant_value_item,
            ));
            cursor.record_group_close(group.item, SemanticToken::Bracket);
            return ConstantValue::List(list).wrap_ok();
        }
```

The list arm sits before the final `expected` in each ladder, after the object arm.

## Tests

Feed list interiors with leftover `Separator(BracketKind::Bracket)` and `parse_value_item`, using the `parsed_items` harness in `arguments.rs`.

```rust
// from crates/isograph_parser/src/arguments.rs
    fn parsed_values(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<NonConstantValue, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    ) {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_value_item,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }

    fn as_value(slot: &Slot<NonConstantValue, UnparsedChunkItems>) -> &NonConstantValue {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a value")
    }

    #[test]
    fn list_values_parse() {
        let text = "1, $x, true";
        let (items, errors, _) = parsed_values(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 3);
        assert!(matches!(as_value(items[0].item.reference()), NonConstantValue::Integer(IntegerValue(1))));
        assert!(matches!(as_value(items[1].item.reference()), NonConstantValue::Variable(_)));
        assert!(matches!(
            as_value(items[2].item.reference()),
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
    }

    #[test]
    fn lists_nest_with_objects() {
        let text = "[1], { a: 2 }";
        let (items, errors, _) = parsed_values(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert!(matches!(as_value(items[0].item.reference()), NonConstantValue::List(_)));
        assert!(matches!(as_value(items[1].item.reference()), NonConstantValue::Object(_)));
    }

    #[test]
    fn empty_and_whitespace_list_interiors_hold_zero_values() {
        for text in ["", "   ", "\n"] {
            let (items, errors, _) = parsed_values(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_trailing_comma_after_a_list_value_is_not_a_parse_error() {
        let text = "1,";
        let (items, errors, _) = parsed_values(text);
        assert_eq!(items.len(), 1);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn leftover_after_a_list_value_keeps_the_item() {
        let text = "1 junk";
        let (items, errors, _) = parsed_values(text);
        assert_eq!(items.len(), 1);
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(BracketKind::Bracket),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_between_list_values_is_chunkings_error() {
        let text = "1,, 2";
        let (items, errors, comma_errors, _) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_value_item,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_paren_group_is_a_list_value() {
        let text = "ids: [1, $x]";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::List(list) => {
                assert_eq!(list.0.len(), 2);
                assert_eq!(
                    as_argument(items[0].item.reference()).value.location,
                    span_of(text, "[1, $x]")
                );
            }
            value => panic!("expected a list, got {value:?}"),
        }
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_constant_list_default_parses_and_rejects_variables() {
        let text = "field Query.Foo($ids: [Int] = [1, 2]) { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert!(matches!(
            declared.default_value.as_ref().map(|value| value.item.reference()),
            Some(ConstantValue::List(_))
        ));

        let with_var = "field Query.Foo($ids: [Int] = [$x]) { bar }";
        let (parse, errors) = parsed(with_var);
        as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::ConstantValue,
                    Found::Token(NonBracketTokenKind::Dollar),
                )
                && error.location == span_of(with_var, "$")
        }));
    }
```

`$` inside the list fails `parse_constant_value_item`, so that list slot is `item: None`. The declaration parses.

## Landing checklist

1. `ListLiteral`, `ConstantListLiteral`, the `List` variants, `parse_value_item` / `parse_constant_value_item`, the `From` impls, the tests. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
