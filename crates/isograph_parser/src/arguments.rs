use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError,
    SemanticToken, Slot, UnparsedChunkItems,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(#[resolve_field] pub Vec<WithSpan<Slot<ObjectEntry, UnparsedChunkItems>>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionFieldArgumentSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionFieldArgument {
    #[resolve_field]
    pub name: WithSpan<FieldArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(SelectionFieldArgument)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntrySlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ValueKeyNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringLiteralValueWrapper),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(#[resolve_field] pub WithSpan<VariableNameWrapper>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringLiteralValueWrapper(common_lang_types::StringLiteralValue);

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
#[resolve_position(
    parent_type = SelectionFieldArgumentPath<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct FieldArgumentNameWrapper(common_lang_types::FieldArgumentName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntryPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ValueKeyNameWrapper(common_lang_types::ValueKeyName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableUsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(common_lang_types::VariableName);

#[derive(Debug)]
pub enum ArgumentListParent<'a> {
    ScalarSelection(crate::ScalarSelectionPath<'a>),
    ObjectSelection(crate::ObjectSelectionPath<'a>),
}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    SelectionFieldArgument(Box<SelectionFieldArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;

pub type ObjectLiteralPath<'a> =
    PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type SelectionFieldArgumentSlotPath<'a> = PositionResolutionPath<
    &'a Slot<SelectionFieldArgument, UnparsedChunkItems>,
    ArgumentListPath<'a>,
>;

pub type ObjectEntrySlotPath<'a> =
    PositionResolutionPath<&'a Slot<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>>;

pub type SelectionFieldArgumentPath<'a> =
    PositionResolutionPath<&'a SelectionFieldArgument, SelectionFieldArgumentSlotPath<'a>>;

pub type ObjectEntryPath<'a> = PositionResolutionPath<&'a ObjectEntry, ObjectEntrySlotPath<'a>>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringLiteralValueWrapperPath<'a> =
    PositionResolutionPath<&'a StringLiteralValueWrapper, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> =
    PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> =
    PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type FieldArgumentNameWrapperPath<'a> =
    PositionResolutionPath<&'a FieldArgumentNameWrapper, SelectionFieldArgumentPath<'a>>;

pub type ValueKeyNameWrapperPath<'a> =
    PositionResolutionPath<&'a ValueKeyNameWrapper, ObjectEntryPath<'a>>;

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableUsePath<'a>>;

impl<'a> From<SelectionFieldArgumentSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: SelectionFieldArgumentSlotPath<'a>) -> Self {
        IsographResolutionNode::SelectionFieldArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        IsographResolutionNode::ObjectEntrySlot(path)
    }
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_name_colon_value<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
) -> Result<(WithSpan<N>, WithSpan<NonConstantValue>), WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_non_constant_value(cursor)?;
    (name.interned(), value).wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::Argument, Expectation::Argument)?;
    SelectionFieldArgument {
        name: name.map(FieldArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_object_entry(cursor: &mut ItemCursor<'_>) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::ObjectKey, Expectation::ObjectEntry)?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn consume_argument_list(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<ArgumentList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            ArgumentList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_argument,
            ))
        },
    )
}

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if cursor
            .consume_token_if(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .is_some()
        {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            return NonConstantValue::Variable(VariableUse(
                name.interned().map(VariableNameWrapper),
            ))
            .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
            return NonConstantValue::String(span.interned().map(StringLiteralValueWrapper).item)
                .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
            let value = match span.token_text().parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64
                        .with_span(span.location)
                        .wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::BooleanOrNull,
        ) {
            return match span.token_text() {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                _ => ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span.location)
                .wrap_err(),
            };
        }
        if let Some(object) = cursor.consume_group_if(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                ObjectLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_object_entry,
                ))
            },
        ) {
            return NonConstantValue::Object(object.item).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        CommaWithoutItem, Found, NonBracketTokenKind, ParseError, SemanticToken, chunk,
        match_brackets, tokenize,
    };

    type ParsedItems<P> = (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    type ParsedPairs = (
        Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    );

    type ParsedArgumentList = (
        Option<WithSpan<ArgumentList>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> ParsedItems<P> {
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
        (items, errors, comma_errors, tokens)
    }

    fn parsed_pairs(text: &str) -> ParsedPairs {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }

    fn parsed_argument_list(text: &str) -> ParsedArgumentList {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let mut stream = tree.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let list = consume_argument_list(stream.cursor());
        (list, errors, tokens)
    }

    fn as_argument(
        slot: &Slot<SelectionFieldArgument, UnparsedChunkItems>,
    ) -> &SelectionFieldArgument {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named argument")
    }

    fn as_entry(slot: &Slot<ObjectEntry, UnparsedChunkItems>) -> &ObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected an object entry")
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
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            FieldArgumentNameWrapper("id".intern().to())
        );
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, "$petId")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "shouted")
        );
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null"#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let values: Vec<&NonConstantValue> = items
            .iter()
            .map(|slot| as_argument(slot.item.reference()).value.item.reference())
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
    }

    #[test]
    fn object_values_use_braces() {
        let text = "input: { id: 4, nested: { on: true } }";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let value = as_argument(items[0].item.reference()).value.reference();
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
            as_entry(object.0[1].item.reference()).name.location,
            span_of(text, "nested")
        );
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_pairs() {
        for text in ["", "   ", "\n"] {
            let (items, errors, _) = parsed_pairs(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_trailing_comma_after_a_pair_is_not_a_parse_error() {
        let text = "id: 1,";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            FieldArgumentNameWrapper("id".intern().to())
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_pair() {
        let text = "a: 99999999999999999999, b: 1";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        as_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_pair_degrades_that_pair_alone() {
        let text = "a 1, b: 2";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
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
        let (items, errors, _) = parsed_pairs(text);
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
    fn a_pair_that_does_not_start_with_a_name_is_an_argument_error() {
        let text = "42: 1";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Argument,
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
                && error.location == span_of(text, "42")
        }));
    }

    #[test]
    fn leftover_after_a_pair_keeps_the_item() {
        let text = "id: $x junk";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
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
        let (items, errors, comma_errors, _) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn consume_argument_list_reads_a_paren_group() {
        let text = "(id: $petId)";
        let (list, errors, tokens) = parsed_argument_list(text);
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(errors, vec![]);
        assert_eq!(list.location, span_of(text, "(id: $petId)"));
        assert_eq!(list.item.0.len(), 1);
        assert_eq!(
            as_argument(list.item.0[0].item.reference()).name.item,
            FieldArgumentNameWrapper("id".intern().to())
        );
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Parenthesis.with_span(span_of(text, "(")),
                SemanticToken::Argument.with_span(span_of(text, "id")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::Variable.with_span(span_of(text, "$")),
                SemanticToken::Variable.with_span(span_of(text, "petId")),
                SemanticToken::Parenthesis.with_span(span_of(text, ")")),
            ],
        );
    }

    #[test]
    fn an_empty_paren_group_is_zero_pairs() {
        let text = "()";
        let (list, errors, _) = parsed_argument_list(text);
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(list.item.0.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_pair_records_argument_colon_and_value_tokens() {
        let text = "id: $petId";
        let (_, errors, tokens) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Argument.with_span(span_of(text, "id")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::Variable.with_span(span_of(text, "$")),
                SemanticToken::Variable.with_span(span_of(text, "petId")),
            ],
        );
    }
}
