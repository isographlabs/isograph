# parse-arguments: argument lists and values

Values: variables, strings, integers, booleans, null, object literals. Argument lists are `name: value` chunks. Lands `parse_items` and `ClosingDelimiter`. Lands after generic-slot.md. parse-selection-sets.md attaches the list to selections. parse-variables.md reuses the value grammar for defaults.

## The grammar this doc accepts

Each contentful chunk of an argument list is one argument:

```
<Identifier> : <value>
```

A value is one of:

```
$ <Identifier>          a variable
"..."                   a string literal (interned source slice, quotes included)
42, -7                  an integer literal, converted to i64
true, false             a boolean
null                    null
{ <entries> }           an object literal, each contentful chunk one `<Identifier> : <value>` entry
```

Tests feed a list's interior to `parse_items`. The wrapping paren group lands with the host that consumes it.

## Change 1: `parse_items`

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    pub(crate) fn parse_items<'a, P, F>(
        &'a self,
        text: &'a str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_item(
                    chunk,
                    text,
                    leftover,
                    |cursor, push_error| parse_item(cursor, push_error),
                    push_error,
                )
            })
            .collect()
    }
}
```

A list trailing comma is legal and is not a diagnostic. Length equals chunk count.

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
    #[error("a comma, a line break, or {0}")]
    Separator(ClosingDelimiter),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClosingDelimiter {
    #[error("'}}'")]
    Brace,
    #[error("')'")]
    Parenthesis,
    #[error("']'")]
    Bracket,
}
```

Before, `Separator` is a unit variant (`"a comma or line break"`). This doc gives it the closer. Argument leftover is `Expectation::Separator(ClosingDelimiter::Parenthesis)`. Object-literal leftover is `Expectation::Separator(ClosingDelimiter::Brace)`.

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
```

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
pub struct ArgumentList(#[resolve_field] pub Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedArgument {
    #[resolve_field]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field]
    #[parent_variant(Argument)]
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
pub struct VariableUse {
    #[resolve_field]
    pub name: WithSpan<VariableName>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
// Quotes included. Unquoting is later.
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

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(
    #[resolve_field] pub Vec<WithSpan<Slot<NamedObjectEntry, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectLiteralPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ObjectEntryName>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedArgumentPath<'a>, resolved_node = IsographResolutionNode<'a>)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedObjectEntryPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectEntryName(common_lang_types::ValueKeyName);

impl From<intern::string_key::StringKey> for ObjectEntryName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ObjectEntryName(key.to())
    }
}

#[derive(Debug)]
pub enum ArgumentListParent {}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    Argument(NamedArgumentPath<'a>),
    ObjectEntry(Box<NamedObjectEntryPath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent>;

pub type NamedArgumentPath<'a> = PositionResolutionPath<&'a NamedArgument, ArgumentListPath<'a>>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringValuePath<'a> = PositionResolutionPath<&'a StringValue, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> = PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> = PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type ObjectLiteralPath<'a> = PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type NamedObjectEntryPath<'a> =
    PositionResolutionPath<&'a NamedObjectEntry, ObjectLiteralPath<'a>>;

pub type ArgumentNamePath<'a> = PositionResolutionPath<&'a ArgumentName, NamedArgumentPath<'a>>;

pub type VariableNamePath<'a> = PositionResolutionPath<&'a VariableName, VariableUsePath<'a>>;

pub type ObjectEntryNamePath<'a> =
    PositionResolutionPath<&'a ObjectEntryName, NamedObjectEntryPath<'a>>;
```

`ArgumentListParent` has no variants. parse-selection-sets.md adds `Scalar` and `Object`. A position on `$` answers `VariableUse`. There is no `Dollar` field.

`NonConstantValueParent::ObjectEntry` is boxed to break the cycle `NonConstantValueParent -> NamedObjectEntryPath -> ObjectLiteralPath -> NonConstantValueParent`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
}

impl<'a> From<ArgumentListPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: ArgumentListPath<'a>) -> Self {
        UnparsedChunkItemsParent::ArgumentList(parent)
    }
}

impl<'a> From<ObjectLiteralPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: ObjectLiteralPath<'a>) -> Self {
        UnparsedChunkItemsParent::ObjectLiteral(parent)
    }
}
```

`From<IsoLiteralParsePath>` stays.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum SlotPath<'a> {
    Literal(
        PositionResolutionPath<
            &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
            IsoLiteralParsePath<'a>,
        >,
    ),
    Argument(
        PositionResolutionPath<&'a Slot<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>>,
    ),
    ObjectEntry(
        PositionResolutionPath<&'a Slot<NamedObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>>,
    ),
}
```

`From` impls for the two new variants. A gap in an argument slot answers `IsographResolutionNode::Slot(SlotPath::Argument(_))`.

`parse_value` is the listing in parsing-standards.md.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn consume_argument_list<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Option<WithSpan<ArgumentList>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let group = cursor.consume_group_if(BracketKind::Parenthesis)?;
    ArgumentList(group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Parenthesis),
        parse_argument,
        push_error,
    ))
    .with_span(group.location)
    .wrap_some()
}

fn parse_argument<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<NamedArgument, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Argument))?;
    cursor
        .require_token(NonBracketTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_value(cursor, push_error)?;
    NamedArgument {
        name: cursor
            .token_text(name)
            .intern()
            .to::<ArgumentName>()
            .with_span(name),
        value,
    }
    .wrap_ok()
}

fn parse_object_entry<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<NamedObjectEntry, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::ObjectEntry))?;
    cursor
        .require_token(NonBracketTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_value(cursor, push_error)?;
    NamedObjectEntry {
        name: cursor
            .token_text(name)
            .intern()
            .to::<ObjectEntryName>()
            .with_span(name),
        value,
    }
    .wrap_ok()
}
```

`lib.rs` adds `mod arguments;` and `pub use arguments::*;`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ArgumentList(ArgumentListPath<'a>),
    NamedArgument(NamedArgumentPath<'a>),
    ArgumentName(ArgumentNamePath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableName(VariableNamePath<'a>),
    StringValue(StringValuePath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    NamedObjectEntry(NamedObjectEntryPath<'a>),
    ObjectEntryName(ObjectEntryNamePath<'a>),
```

`ArgumentList` and `ObjectLiteral` expand like a vec of slots. `NamedArgument` mixes a bare name descent with a wrapped value descent (`parent_variant = Argument`). `NamedObjectEntry` is the same with `ObjectEntry`, boxed through `From<T> for Box<T>`. `NonConstantValue` delegates. `VariableUse` has one marked field. `IntegerValue` / `BooleanValue` / `StringValue` / `NullValue` are leaves.

Resolve-from-a-host tests wait for parse-selection-sets.md. This doc asserts parse structure.

## Tests

```rust
// from crates/isograph_parser/src/arguments.rs
    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(
            &mut ItemCursor<'_>,
            &mut Vec<WithSpan<ParseError>>,
        ) -> Result<P, WithSpan<ParseError>>,
    ) -> (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let items = tree
            .item
            .parse_items(text, leftover, parse_item, &mut errors);
        (items, errors, comma_errors)
    }

    fn parsed_arguments(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
    ) {
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(ClosingDelimiter::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
    }

    fn as_named_argument(slot: &Slot<NamedArgument, UnparsedChunkItems>) -> &NamedArgument {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named argument")
    }

    fn as_named_object_entry(slot: &Slot<NamedObjectEntry, UnparsedChunkItems>) -> &NamedObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named object entry")
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
    fn arguments_parse_as_name_colon_value() {
        let text = "id: $petId, shouted: true";
        let (items, errors) = parsed_arguments(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_named_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert_eq!(
            as_named_argument(items[0].item.reference()).name.item,
            "id".intern().to()
        );
        assert_eq!(
            as_named_argument(items[0].item.reference()).value.location,
            span_of(text, "$petId")
        );
        assert_eq!(
            as_named_argument(items[1].item.reference()).name.location,
            span_of(text, "shouted")
        );
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null"#;
        let (items, errors) = parsed_arguments(text);
        assert_eq!(errors, vec![]);
        let values: Vec<&NonConstantValue> = items
            .iter()
            .map(|argument| as_named_argument(argument.item.reference()).value.item.reference())
            .collect();
        assert!(matches!(values[0], NonConstantValue::Variable(_)));
        assert!(matches!(values[1], NonConstantValue::String(_)));
        assert!(matches!(
            values[2],
            NonConstantValue::Integer(IntegerValue(42))
        ));
        assert!(matches!(
            values[3],
            NonConstantValue::Integer(IntegerValue(-7))
        ));
        assert!(matches!(
            values[4],
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
        assert!(matches!(
            values[5],
            NonConstantValue::Boolean(BooleanValue(Boolean::False))
        ));
        assert!(matches!(values[6], NonConstantValue::Null(_)));
        assert_eq!(
            as_named_argument(items[0].item.reference()).value.location,
            span_of(text, "$x")
        );
    }

    #[test]
    fn object_literal_values_nest() {
        let text = "input: { id: 4, nested: { on: true } }";
        let (items, errors) = parsed_arguments(text);
        assert_eq!(errors, vec![]);
        let value = as_named_argument(items[0].item.reference()).value.reference();
        assert_eq!(
            value.location,
            span_of(text, "{ id: 4, nested: { on: true } }")
        );
        let object = match value.item.reference() {
            NonConstantValue::Object(object) => object,
            value => panic!("expected an object literal, got {value:?}"),
        };
        assert_eq!(object.0.len(), 2);
        let nested = as_named_object_entry(object.0[1].item.reference());
        assert_eq!(nested.name.location, span_of(text, "nested"));
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_arguments() {
        for text in ["", "   ", "\n"] {
            let (items, errors) = parsed_arguments(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_list_trailing_comma_is_not_a_diagnostic() {
        let text = "id: 1,";
        let (items, errors) = parsed_arguments(text);
        assert_eq!(items.len(), 1);
        assert_eq!(as_named_argument(items[0].item.reference()).name.item, "id".intern().to());
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_argument() {
        let text = "a: 99999999999999999999, b: 1";
        let (items, errors) = parsed_arguments(text);
        assert!(items[0].item.item.is_none());
        as_named_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_argument_degrades_that_argument_alone() {
        let text = "a 1, b: 2";
        let (items, errors) = parsed_arguments(text);
        assert!(items[0].item.item.is_none());
        assert_eq!(
            as_named_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
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
        let (items, errors) = parsed_arguments(text);
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
    fn leftover_after_an_argument_keeps_the_item() {
        let text = "id: $x junk";
        let (items, errors) = parsed_arguments(text);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_named_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(ClosingDelimiter::Parenthesis),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_between_arguments_is_chunkings_error() {
        let text = "a: 1,, b: 2";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(ClosingDelimiter::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(as_named_argument(items[0].item.reference()).name.location, span_of(text, "a"));
        assert_eq!(as_named_argument(items[1].item.reference()).name.location, span_of(text, "b"));
        assert_eq!(errors, vec![]);
    }
```

## Landing checklist

1. `parse_items`, `ClosingDelimiter`, `IntegerDoesNotFitI64`, `arguments.rs`, the resolution-node variants, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
