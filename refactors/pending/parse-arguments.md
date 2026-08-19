# parse-arguments: argument lists and values

`name : value` is `parse_key_value_pair`. A list of those is `parse_each_chunk`. `( ... )` is an argument list. `{ ... }` is an object value. Lands after generic-slot.md. parse-selection-sets.md attaches the paren list to selections. parse-variables.md reuses `parse_value`.

## Grammar

```
<Identifier> : <value>
```

```
( <pairs> )             argument list
{ <pairs> }             object value
```

```
$ <Identifier>          a variable
"..."                   a string literal (interned source slice, quotes included)
42, -7                  i64
true, false             Boolean::{True, False}
null
{ <pairs> }             object value
```

Tests feed a list interior to `parse_each_chunk`.

## Change 1: `parse_each_chunk`

Already on `ChunkedLevel`. Nested lists pass the parent cursor. Tests that feed a list interior construct a parent stream to hold `text` / `tokens` / `errors`.

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    pub(crate) fn parse_each_chunk<'a, P>(
        &'a self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
}
```

A list trailing comma is not a diagnostic. Length equals chunk count.

## Change 2: `Expectation`

```rust
// from crates/isograph_parser/src/parse_error.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a comma, a line break, or {}", .0.closing())]
    Separator(BracketKind),
    #[error("a value, like $foo, 42, \"bar\", true, false, null, an object literal, or an array literal")]
    Value,
}
```

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
impl BracketKind {
    pub fn closing(self) -> &'static str {
        match self {
            BracketKind::Parenthesis => "')'",
            BracketKind::Brace => "'}'",
            BracketKind::Bracket => "']'",
        }
    }
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
```

A missing pair name is `Expectation::Token(Identifier)`.

## Change 3: `arguments.rs`

```rust
// from crates/isograph_parser/src/arguments.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError,
    Slot, UnparsedChunkItems,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field]
    #[parent_variant(ArgumentList)]
    pub Vec<WithSpan<Slot<KeyValuePair, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(
    #[resolve_field]
    #[parent_variant(ObjectLiteral)]
    pub Vec<WithSpan<Slot<KeyValuePair, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = KeyValuePairParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct KeyValuePair {
    #[resolve_field]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field]
    #[parent_variant(KeyValue)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringValue),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(
    #[resolve_field]
    pub WithSpan<VariableName>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringValue(common_lang_types::StringLiteralValue);

impl From<intern::string_key::StringKey> for StringValue {
    fn from(key: intern::string_key::StringKey) -> Self {
        StringValue(key.to())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = KeyValuePairPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentName(common_lang_types::FieldArgumentName);

impl From<intern::string_key::StringKey> for ArgumentName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ArgumentName(key.to())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableUsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableName(common_lang_types::VariableName);

impl From<intern::string_key::StringKey> for VariableName {
    fn from(key: intern::string_key::StringKey) -> Self {
        VariableName(key.to())
    }
}

#[derive(Debug)]
pub enum ArgumentListParent {}

#[derive(Debug)]
pub enum KeyValuePairParent<'a> {
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    KeyValue(Box<KeyValuePairPath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent>;

pub type ObjectLiteralPath<'a> =
    PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type KeyValuePairPath<'a> =
    PositionResolutionPath<&'a KeyValuePair, KeyValuePairParent<'a>>;

pub type KeyValuePairSlotPath<'a> = PositionResolutionPath<
    &'a Slot<KeyValuePair, UnparsedChunkItems>,
    KeyValuePairParent<'a>,
>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringValuePath<'a> = PositionResolutionPath<&'a StringValue, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> = PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> = PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type ArgumentNamePath<'a> = PositionResolutionPath<&'a ArgumentName, KeyValuePairPath<'a>>;

pub type VariableNamePath<'a> = PositionResolutionPath<&'a VariableName, VariableUsePath<'a>>;
```

`ArgumentListParent` has no variants. parse-selection-sets.md adds `Scalar` and `Object`. A position on `$` answers `VariableUse`.

`NonConstantValueParent::KeyValue` is boxed to break `KeyValuePairPath -> NonConstantValue -> ObjectLiteral -> KeyValuePair`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
}

impl<'a> From<KeyValuePairParent<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: KeyValuePairParent<'a>) -> Self {
        match parent {
            KeyValuePairParent::ArgumentList(parent) => {
                UnparsedChunkItemsParent::ArgumentList(parent)
            }
            KeyValuePairParent::ObjectLiteral(parent) => {
                UnparsedChunkItemsParent::ObjectLiteral(parent)
            }
        }
    }
}
```

`From<IsoLiteralParsePath>` stays.

```rust
// from crates/isograph_parser/src/arguments.rs
impl<'a> From<KeyValuePairSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: KeyValuePairSlotPath<'a>) -> Self {
        IsographResolutionNode::KeyValuePairSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    KeyValuePairSlot(KeyValuePairSlotPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    KeyValuePair(KeyValuePairPath<'a>),
    ArgumentName(ArgumentNamePath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableName(VariableNamePath<'a>),
    StringValue(StringValuePath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
```

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_key_value_pair(
    cursor: &mut ItemCursor<'_>,
) -> Result<KeyValuePair, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_value(cursor)?;
    KeyValuePair {
        name: cursor
            .token_text(name)
            .intern()
            .to::<ArgumentName>()
            .with_span(name),
        value,
    }
    .wrap_ok()
}

pub(crate) fn consume_argument_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<ArgumentList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis)?;
    ArgumentList(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Parenthesis),
        parse_key_value_pair,
    ))
    .with_span(group.location)
    .wrap_some()
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(dollar) = cursor.consume_token_if(NonBracketTokenKind::Dollar) {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
            return NonConstantValue::Variable(VariableUse(
                cursor
                    .token_text(name)
                    .intern()
                    .to::<VariableName>()
                    .with_span(name),
             )
            .wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::StringLiteral) {
            // Quotes included. Unquoting is later.
            return NonConstantValue::String(
                cursor
                    .token_text(span)
                    .intern()
                    .to::<StringValue>(),
            )
            .wrap_ok();
        }
        // BlockStringLiteral is not consumed.
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64.with_span(span).wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        // No FloatLiteral token. `1.5` does not parse as a value.
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                // Enum values (bare identifiers) are not parsed.
                _ => ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span)
                .wrap_err(),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return NonConstantValue::Object(ObjectLiteral(
                group.item.children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_key_value_pair,
                ),
            ))
            .wrap_ok();
        }
        // `[ ... ]` is parse-arrays.md.
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

`lib.rs` adds `mod arguments;` and `pub use arguments::*;`.

## Tests

```rust
// from crates/isograph_parser/src/arguments.rs
    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let dummy = {
            let (brackets, _) = match_brackets(tokenize("x"), 1);
            chunk(brackets.reference()).0
        };
        let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let items = tree
            .item
            .parse_each_chunk(parent.cursor(), leftover, parse_item);
        (items, errors, comma_errors)
    }

    fn parsed_pairs(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<KeyValuePair, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
    ) {
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_key_value_pair,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
    }

    fn as_pair(slot: &Slot<KeyValuePair, UnparsedChunkItems>) -> &KeyValuePair {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a key-value pair")
    }

    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    #[test]
    fn pairs_parse_as_name_colon_value() {
        let text = "id: $petId, shouted: true";
        let (items, errors) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(as_pair(items[0].item.reference()).name.location, span_of(text, "id"));
        assert_eq!(as_pair(items[0].item.reference()).name.item, "id".intern().to());
        assert_eq!(
            as_pair(items[0].item.reference()).value.location,
            span_of(text, "$petId")
        );
        assert_eq!(
            as_pair(items[1].item.reference()).name.location,
            span_of(text, "shouted")
        );
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null"#;
        let (items, errors) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let values: Vec<&NonConstantValue> = items
            .iter()
            .map(|slot| as_pair(slot.item.reference()).value.item.reference())
            .collect();
        assert!(matches!(values[0], NonConstantValue::Variable(_)));
        assert!(matches!(values[1], NonConstantValue::String(_)));
        assert!(matches!(values[2], NonConstantValue::Integer(IntegerValue(42))));
        assert!(matches!(values[3], NonConstantValue::Integer(IntegerValue(-7))));
        assert!(matches!(
            values[4],
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
        assert!(matches!(
            values[5],
            NonConstantValue::Boolean(BooleanValue(Boolean::False))
        ));
        assert!(matches!(values[6], NonConstantValue::Null(_)));
    }

    #[test]
    fn object_values_use_braces() {
        let text = "input: { id: 4, nested: { on: true } }";
        let (items, errors) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let value = as_pair(items[0].item.reference()).value.reference();
        assert_eq!(
            value.location,
            span_of(text, "{ id: 4, nested: { on: true } }")
        );
        let object = match value.item.reference() {
            NonConstantValue::Object(object) => object,
            value => panic!("expected an object, got {value:?}"),
        };
        assert_eq!(object.0.len(), 2);
        assert_eq!(
            as_pair(object.0[1].item.reference()).name.location,
            span_of(text, "nested")
        );
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_pairs() {
        for text in ["", "   ", "\n"] {
            let (items, errors) = parsed_pairs(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_list_trailing_comma_is_not_a_diagnostic() {
        let text = "id: 1,";
        let (items, errors) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(as_pair(items[0].item.reference()).name.item, "id".intern().to());
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_pair() {
        let text = "a: 99999999999999999999, b: 1";
        let (items, errors) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        as_pair(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_pair_degrades_that_pair_alone() {
        let text = "a 1, b: 2";
        let (items, errors) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert_eq!(as_pair(items[1].item.reference()).name.location, span_of(text, "b"));
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Token(NonBracketTokenKind::Colon),
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
        }));
    }

    #[test]
    fn a_non_value_identifier_is_an_error_at_the_value() {
        let text = "a: yes";
        let (items, errors) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "yes")
        }));
    }

    #[test]
    fn leftover_after_a_pair_keeps_the_item() {
        let text = "id: $x junk";
        let (items, errors) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(as_pair(items[0].item.reference()).name.location, span_of(text, "id"));
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(BracketKind::Parenthesis),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_between_pairs_is_chunkings_error() {
        let text = "a: 1,, b: 2";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_key_value_pair,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(as_pair(items[0].item.reference()).name.location, span_of(text, "a"));
        assert_eq!(as_pair(items[1].item.reference()).name.location, span_of(text, "b"));
        assert_eq!(errors, vec![]);
    }
```

## Landing checklist

1. `parse_each_chunk`, `Separator(BracketKind)`, `IntegerDoesNotFitI64`, `parse_key_value_pair`, `arguments.rs`, the resolution-node variants, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
