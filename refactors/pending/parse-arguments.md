# parse-arguments: argument lists and values

Selections gain argument lists. Values: variables, strings, integers, booleans, null, object literals. parse-variables.md reuses the value grammar for defaults.

## The grammar this doc accepts

```
[<Identifier> :] <Identifier> [<paren group>] [<brace group>]
```

Each contentful chunk of the paren group's interior is one argument:

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

## Changes to parse_error.rs

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    UnsupportedDeclarationType,
    IntegerDoesNotFitI64,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum Expectation {
    Token(NonBracketTokenKind),
    DeclarationKeyword,
    EndOfDeclaration,
    SelectionSet,
    Selection,
    Separator,
    Argument,
    Value,
    ObjectEntry,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
            ParseError::IntegerDoesNotFitI64 => {
                write!(f, "This integer does not fit in a 64-bit signed integer.")
            }
            Expectation::Argument => write!(f, "an argument, like 'id: $id'"),
            Expectation::Value => {
                write!(f, "a value, like $foo, 42, \"bar\", true, false, null, or an object literal")
            }
            Expectation::ObjectEntry => write!(f, "an object entry, like 'id: 4'"),
```

## New module: arguments.rs

```rust
// from crates/isograph_parser/src/arguments.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, Expectation, Found, IsographResolutionNode, NonBracketTokenKind,
    ObjectSelectionPath, ParseError, ScalarSelectionPath, Slot, UnparsedChunkItems,
    UnparsedChunkItemsParent,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(#[resolve_field] pub Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedArgument {
    #[resolve_field]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field(parent_variant = Argument)]
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
    #[resolve_field(parent_variant = ObjectEntry)]
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
pub enum ArgumentListParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    Argument(NamedArgumentPath<'a>),
    ObjectEntry(Box<NamedObjectEntryPath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;

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

A position on `$` answers `VariableUse`. There is no `Dollar` field.

`NonConstantValueParent::ObjectEntry` is boxed to break the cycle `NonConstantValueParent -> NamedObjectEntryPath -> ObjectLiteralPath -> NonConstantValueParent`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
}
```

`From<ArgumentListPath>` and `From<ObjectLiteralPath>` join the existing impls.

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
        Expectation::Separator,
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

## Changes to selections.rs

Before:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct ScalarSelection {
    #[resolve_field(parent_variant = Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Scalar)]
    pub name: WithSpan<SelectionName>,
}

pub struct ObjectSelection {
    #[resolve_field(parent_variant = Object)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Object)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Object)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct ScalarSelection {
    #[resolve_field(parent_variant = Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Scalar)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Scalar)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

pub struct ObjectSelection {
    #[resolve_field(parent_variant = Object)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Object)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Object)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field(parent_variant = Object)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

```rust
// from crates/isograph_parser/src/selections.rs
    let arguments = consume_argument_list(cursor, push_error);
    let selection_set = consume_selection_set(cursor, push_error);
```

The parse-fields.md test `arguments_are_trailing_leftover_until_parse_arguments` is deleted.

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

`ArgumentList` and `ObjectLiteral` expand like `SelectionSet`. `NamedArgument` mixes a bare name descent with a wrapped value descent (`parent_variant = Argument`). `NamedObjectEntry` is the same with `ObjectEntry`, boxed through `From<T> for Box<T>`. `NonConstantValue` delegates. `VariableUse` has one marked field. `IntegerValue` / `BooleanValue` / `StringValue` / `NullValue` are leaves. A position on `$` answers `VariableUse`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn arguments_of(slot: &Slot<Selection, UnparsedChunkItems>) -> &WithSpan<ArgumentList> {
        let arguments = match slot.item.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(Selection::Scalar(scalar)) => scalar.arguments.reference(),
            Some(Selection::Object(object)) => object.arguments.reference(),
            None => panic!("expected a parsed selection"),
        };
        arguments
            .as_ref()
            .expect("the fixture's selection carries arguments")
    }

    fn as_named_argument(slot: &Slot<NamedArgument, UnparsedChunkItems>) -> &NamedArgument {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named argument")
    }

    #[test]
    fn arguments_parse_on_scalar_and_object_selections() {
        let text = "field Query.Foo { pet(id: $petId) { name(shouted: true) } }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let outer_selection = selections(as_field(parse.reference()).selection_set.reference())[0]
            .item
            .reference();
        let outer = arguments_of(outer_selection);
        assert_eq!(outer.location, span_of(text, "(id: $petId)"));
        assert_eq!(
            as_named_argument(outer.item.0[0].item.reference())
                .name
                .location,
            span_of(text, "id")
        );
        assert_eq!(
            as_named_argument(outer.item.0[0].item.reference())
                .name
                .item,
            "id".intern().to()
        );
        let object = as_object(outer_selection);
        let inner_selection = selections(object.selection_set.reference())[0]
            .item
            .reference();
        let inner = arguments_of(inner_selection);
        assert_eq!(inner.location, span_of(text, "(shouted: true)"));
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"field Query.Foo { bar(a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null) }"#;
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let arguments = arguments_of(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        let values: Vec<&NonConstantValue> = arguments
            .item
            .0
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
            as_named_argument(arguments.item.0[0].item.reference())
                .value
                .location,
            span_of(text, "$x")
        );
    }

    #[test]
    fn object_literal_values_nest() {
        let text = "field Query.Foo { bar(input: { id: 4, nested: { on: true } }) }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let arguments = arguments_of(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        let value = as_named_argument(arguments.item.0[0].item.reference()).value.reference();
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

    fn as_named_object_entry(slot: &Slot<NamedObjectEntry, UnparsedChunkItems>) -> &NamedObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named object entry")
    }

    #[test]
    fn empty_argument_lists_parse() {
        let text = "field Query.Foo { bar() }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            arguments_of(
                selections(as_field(parse.reference()).selection_set.reference())[0]
                    .item
                    .reference()
            )
            .item
            .0
            .len(),
            0
        );
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_argument() {
        let text = "field Query.Foo { bar(a: 99999999999999999999, b: 1) }";
        let (parse, errors) = parsed(text);
        let arguments = arguments_of(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        assert!(arguments.item.0[0].item.item.is_none());
        as_named_argument(arguments.item.0[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_argument_degrades_that_argument_alone() {
        let text = "field Query.Foo { bar(a 1, b: 2) }";
        let (parse, errors) = parsed(text);
        let arguments = arguments_of(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        assert!(arguments.item.0[0].item.item.is_none());
        assert_eq!(
            as_named_argument(arguments.item.0[1].item.reference())
                .name
                .location,
            span_of(text, "b")
        );
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    token(NonBracketTokenKind::Colon),
                    Found::Token(IntegerLiteral),
                )
        }));
    }

    #[test]
    fn a_non_value_identifier_is_an_error_at_the_value() {
        let text = "field Query.Foo { bar(a: yes) }";
        let (parse, errors) = parsed(text);
        let arguments = arguments_of(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        assert!(arguments.item.0[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item == expected(Expectation::Value, Found::Token(Identifier))
                && error.location == span_of(text, "yes")
        }));
    }
```

A doubled comma between arguments is chunking's error, same shape as the selection-set test. A leftover token after a complete argument is `extra_tokens` plus `Expected(Separator, ...)`. A position on `$` in `$petId` answers `VariableUse`. A position on `petId` answers `VariableName`.

## Landing checklist

1. arguments.rs, the selections.rs and parse_error.rs changes, the resolution-node variants, the deleted leftover-arguments test, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
